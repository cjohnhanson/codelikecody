//! Docker workspace: runs an agent inside a Docker container.
//!
//! The project directory is mounted into the container. The agent process
//! (claude) runs inside the container with full isolation. Communication
//! happens through the coordination DB (SQLite file mounted into the
//! container, or Postgres over the network).

use std::path::{Path, PathBuf};

use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::exec::CreateExecOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use claude_code::protocol::OutputMessage;
use clc_sdk::workspace::{
    PermissionDenial, Workspace, WorkspaceConfig, WorkspaceError, WorkspaceStatus,
};

/// Default image for worker containers. Must have claude-code and clc installed.
const DEFAULT_IMAGE: &str = "clc-worker:latest";

pub struct DockerWorkspace {
    config: WorkspaceConfig,
    agent: Box<dyn clc_sdk::agent::Agent>,
    image: String,
    container_id: Option<String>,
    docker: Docker,
    rt: tokio::runtime::Runtime,
    status: WorkspaceStatus,
    denials: Vec<PermissionDenial>,
    project_dir: PathBuf,
}

impl DockerWorkspace {
    pub fn new(
        config: WorkspaceConfig,
        agent: Box<dyn clc_sdk::agent::Agent>,
        image: Option<String>,
    ) -> Result<Self, WorkspaceError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| WorkspaceError::Process(format!("tokio runtime: {e}")))?;

        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| WorkspaceError::Process(format!("docker connect: {e}")))?;

        let project_dir = config.project_dir.clone();

        Ok(Self {
            config,
            agent,
            image: image.unwrap_or_else(|| DEFAULT_IMAGE.to_string()),
            container_id: None,
            docker,
            rt,
            status: WorkspaceStatus::NotStarted,
            denials: Vec::new(),
            project_dir,
        })
    }
}

impl Workspace for DockerWorkspace {
    fn start(&mut self) -> Result<(), WorkspaceError> {
        if self.status != WorkspaceStatus::NotStarted {
            return Err(WorkspaceError::Process("workspace already started".into()));
        }

        // Build the agent command to run inside the container.
        let cmd = self
            .agent
            .build_start_command(&self.config.agent_config, Path::new("/project"))
            .map_err(|e| WorkspaceError::Process(format!("{e}")))?;

        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        let mut full_cmd = vec![shell_quote(&program)];
        full_cmd.extend(args.iter().map(|a| shell_quote(a)));

        // Mount the worktree (where the worker actually works) as /project.
        // Also mount the project root as /project-root for access to .tisket/, tisket.yml, etc.
        let worktree_dir = self
            .project_dir
            .join(".worktrees")
            .join(&self.config.tisket_id);
        // Mount the worktree as /project (working dir).
        // Mount the project root's .git/ so git worktree references resolve.
        // Mount tisket config so tisket commands work.
        let worktree_mount = format!("{}:/project", worktree_dir.display());
        let git_mount = format!("{}/.git:/project-git:ro", self.project_dir.display());
        let tisket_mount = format!("{}/tisket.yml:/project/tisket.yml:ro", self.project_dir.display());
        let tisket_dir_mount = format!("{}/.tisket:/project/.tisket", self.project_dir.display());

        // Fix the worktree's .git file to point to the container path.
        let git_file = worktree_dir.join(".git");
        if git_file.is_file() {
            let content = std::fs::read_to_string(&git_file).unwrap_or_default();
            if content.contains("gitdir:") {
                // Rewrite to container-relative path.
                let wt_name = &self.config.tisket_id;
                let container_gitdir = format!("gitdir: /project-git/worktrees/{wt_name}\n");
                let _ = std::fs::write(&git_file, container_gitdir);
            }
        }

        let mut binds = vec![worktree_mount, git_mount, tisket_mount, tisket_dir_mount];

        // Agent-specific secrets: mount token files and inject env vars.
        // The Agent trait doesn't expose this yet, so we check for known
        // agent secret paths. This should move to an Agent::secrets() method.
        let mut secret_env: Vec<(String, String)> = Vec::new();
        if self.agent.name() == "claude-code" {
            let token_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".claude")
                .join("token");
            if token_path.exists() {
                binds.push(format!(
                    "{}:/run/secrets/anthropic-token:ro",
                    token_path.display()
                ));
                if let Ok(token) = std::fs::read_to_string(&token_path) {
                    secret_env.push((
                        "CLAUDE_CODE_OAUTH_TOKEN".to_string(),
                        token.trim().to_string(),
                    ));
                }
            }
        }

        let host_config = HostConfig {
            binds: Some(binds),
            ..Default::default()
        };

        // Build env vars from the agent command + secrets.
        let mut env_vec: Vec<String> = cmd
            .get_envs()
            .filter_map(|(k, v)| {
                v.map(|val| {
                    format!(
                        "{}={}",
                        k.to_string_lossy(),
                        val.to_string_lossy()
                    )
                })
            })
            .collect();
        for (k, v) in &secret_env {
            env_vec.push(format!("{k}={v}"));
        }

        // Write initial prompt to a temp file, mount it, and pipe it to stdin.
        let initial_prompt = &self.config.agent_config.initial_prompt;
        let prompt_json = if initial_prompt.is_empty() {
            String::new()
        } else {
            let input = claude_code::protocol::InputMessage::user(initial_prompt);
            serde_json::to_string(&input).unwrap_or_default()
        };

        // Write the initial prompt to a file in the worktree for Claude to read via stdin redirect.
        if !prompt_json.is_empty() {
            let prompt_path = worktree_dir.join(".clc").join("initial-prompt.jsonl");
            if let Some(parent) = prompt_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&prompt_path, format!("{prompt_json}\n"));
        }

        // Build the shell command: redirect prompt file to stdin if it exists.
        let agent_cmd = full_cmd.join(" ");
        let shell_cmd = format!(
            "if [ -f /project/.clc/initial-prompt.jsonl ]; then \
               {agent_cmd} < /project/.clc/initial-prompt.jsonl; \
             else \
               {agent_cmd}; \
             fi"
        );

        let container_config = Config {
            image: Some(self.image.as_str()),
            cmd: Some(vec!["sh", "-c", &shell_cmd]),
            working_dir: Some("/project"),
            env: Some(env_vec.iter().map(String::as_str).collect()),
            host_config: Some(host_config),
            ..Default::default()
        };

        let container_id = self.rt.block_on(async {
            let container = self
                .docker
                .create_container(
                    Some(CreateContainerOptions::<&str> {
                        name: "",
                        platform: None,
                    }),
                    container_config,
                )
                .await
                .map_err(|e| WorkspaceError::Process(format!("create container: {e}")))?;

            self.docker
                .start_container::<String>(&container.id, None)
                .await
                .map_err(|e| WorkspaceError::Process(format!("start container: {e}")))?;

            Ok::<_, WorkspaceError>(container.id)
        })?;

        self.container_id = Some(container_id);
        self.status = WorkspaceStatus::Running;
        Ok(())
    }

    fn send_message(&mut self, msg: &str) -> Result<(), WorkspaceError> {
        let cid = self
            .container_id
            .as_ref()
            .ok_or_else(|| WorkspaceError::Communication("not started".into()))?
            .clone();

        let input = claude_code::protocol::InputMessage::user(msg);
        let json = serde_json::to_string(&input)
            .map_err(|e| WorkspaceError::Communication(format!("serialize: {e}")))?;

        self.rt.block_on(async {
            let exec = self
                .docker
                .create_exec(
                    &cid,
                    CreateExecOptions {
                        cmd: Some(vec![
                            "sh",
                            "-c",
                            &format!("echo '{}' > /proc/1/fd/0", json.replace('\'', "'\\''")),
                        ]),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| WorkspaceError::Communication(format!("exec: {e}")))?;

            self.docker
                .start_exec(&exec.id, None)
                .await
                .map_err(|e| WorkspaceError::Communication(format!("start exec: {e}")))?;

            Ok::<_, WorkspaceError>(())
        })
    }

    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError> {
        // In Docker workspace, output goes through the coordination DB,
        // not stdout files. Return empty — the coordinator reads from DB.
        //
        // Check if container is still running.
        if let Some(ref cid) = self.container_id {
            let running = self.rt.block_on(async {
                match self.docker.inspect_container(cid, None).await {
                    Ok(info) => info
                        .state
                        .and_then(|s| s.running)
                        .unwrap_or(false),
                    Err(_) => false,
                }
            });

            if !running && self.status == WorkspaceStatus::Running {
                self.status = WorkspaceStatus::Completed;
            }
        }

        Ok(Vec::new())
    }

    fn status(&self) -> WorkspaceStatus {
        self.status
    }

    fn permission_denials(&self) -> &[PermissionDenial] {
        &self.denials
    }

    fn working_dir(&self) -> &Path {
        &self.project_dir
    }

    fn tisket_id(&self) -> &str {
        &self.config.tisket_id
    }

    fn stop(&mut self) -> Result<(), WorkspaceError> {
        if let Some(ref cid) = self.container_id {
            let cid = cid.clone();
            let _ = self.rt.block_on(async {
                let _ = self.docker.stop_container(&cid, None).await;
                let _ = self
                    .docker
                    .remove_container(
                        &cid,
                        Some(RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            });
            self.status = WorkspaceStatus::Failed;
        }
        Ok(())
    }
}

/// Single-quote a string for safe shell embedding.
fn shell_quote(s: &str) -> String {
    // Replace single quotes with '\'' (end quote, escaped quote, restart quote).
    format!("'{}'", s.replace('\'', "'\\''"))
}
