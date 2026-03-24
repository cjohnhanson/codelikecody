//! SSH workspace: runs an agent over an SSH connection with a reverse
//! tunnel back to the supervisor API.
//!
//! The Environment trait creates the SSH target (Docker container, remote
//! host, etc). SSHWorkspace connects via russh, sets up the reverse
//! tunnel, deploys the workspace cert, and starts the agent.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use claude_code::protocol::OutputMessage;
use clc_sdk::workspace::{
    PermissionDenial, Workspace, WorkspaceConfig, WorkspaceError, WorkspaceStatus,
};

use crate::ssh_session::SSHSession;
use crate::tls::{EphemeralCA, WorkspaceCert};

/// SSH connection target returned by an Environment.
#[derive(Debug, Clone)]
pub struct SSHTarget {
    pub host: String,
    pub port: u16,
    pub user: String,
}

/// Environment lifecycle — creates and destroys the thing SSH connects to.
pub trait Environment: Send {
    fn create(&mut self) -> Result<SSHTarget, WorkspaceError>;
    fn destroy(&mut self) -> Result<(), WorkspaceError>;
}

/// SSH workspace configuration.
pub struct SSHWorkspaceConfig {
    pub workspace_config: WorkspaceConfig,
    pub agent: Box<dyn clc_sdk::agent::Agent>,
    pub ca: Arc<EphemeralCA>,
    pub api_port: u16,
    pub oauth_token: Option<String>,
}

/// Workspace that communicates over SSH with a reverse tunnel to the supervisor API.
pub struct SSHWorkspace {
    config: SSHWorkspaceConfig,
    env: Box<dyn Environment>,
    target: Option<SSHTarget>,
    session: Option<SSHSession>,
    tunnel_port: u16,
    status: WorkspaceStatus,
    denials: Vec<PermissionDenial>,
    cert: Option<WorkspaceCert>,
    project_dir: PathBuf,
    rt: tokio::runtime::Runtime,
}

impl SSHWorkspace {
    pub fn new(
        config: SSHWorkspaceConfig,
        env: Box<dyn Environment>,
        tunnel_port: u16,
    ) -> Result<Self, WorkspaceError> {
        let project_dir = config.workspace_config.project_dir.clone();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| WorkspaceError::Process(format!("tokio: {e}")))?;

        Ok(Self {
            config,
            env,
            target: None,
            session: None,
            tunnel_port,
            status: WorkspaceStatus::NotStarted,
            denials: Vec::new(),
            cert: None,
            project_dir,
            rt,
        })
    }
}

impl Workspace for SSHWorkspace {
    fn start(&mut self) -> Result<(), WorkspaceError> {
        if self.status != WorkspaceStatus::NotStarted {
            return Err(WorkspaceError::Process("workspace already started".into()));
        }

        // 1. Create the environment (Docker container, etc).
        let target = self.env.create()?;

        // 2. Sign a workspace cert.
        let agent_id = self.config.workspace_config.tisket_id.clone();
        let cert = self
            .config
            .ca
            .sign_workspace_cert(&agent_id, "worker")
            .map_err(|e| WorkspaceError::Process(format!("cert signing: {e}")))?;

        // 3. Connect via SSH.
        let ssh_key_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".ssh")
            .join("id_ed25519");

        let mut session = self.rt.block_on(async {
            // Wait for sshd to be ready.
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            SSHSession::connect(&target, &ssh_key_path)
                .await
                .map_err(|e| WorkspaceError::Process(format!("SSH connect: {e}")))
        })?;

        // 4. Deploy workspace cert and CA cert.
        self.rt.block_on(async {
            session
                .write_file("/tmp/workspace-cert.pem", &cert.cert_pem)
                .await
                .map_err(|e| WorkspaceError::Process(format!("deploy cert: {e}")))?;
            session
                .write_file("/tmp/workspace-key.pem", &cert.key_pem)
                .await
                .map_err(|e| WorkspaceError::Process(format!("deploy key: {e}")))?;
            session
                .write_file("/tmp/ca-cert.pem", &self.config.ca.ca_cert_pem)
                .await
                .map_err(|e| WorkspaceError::Process(format!("deploy CA: {e}")))?;
            Ok::<_, WorkspaceError>(())
        })?;

        // 5. Set up reverse tunnel: workspace's localhost:tunnel_port → supervisor's API.
        let tunnel_port = self.tunnel_port;
        let api_port = self.config.api_port;
        self.rt.block_on(async {
            session
                .setup_reverse_tunnel(tunnel_port, api_port)
                .await
                .map_err(|e| WorkspaceError::Process(format!("reverse tunnel: {e}")))
        })?;

        // 6. Build and start the agent command.
        let cmd = self
            .config
            .agent
            .build_start_command(
                &self.config.workspace_config.agent_config,
                Path::new("/project"),
            )
            .map_err(|e| WorkspaceError::Process(format!("{e}")))?;

        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();

        // Build env var exports.
        let mut env_parts = vec![
            format!("CLC_API_URL=http://localhost:{tunnel_port}"),
            format!("CLC_API_CERT=/tmp/workspace-cert.pem"),
            format!("CLC_API_KEY=/tmp/workspace-key.pem"),
            format!("CLC_API_CA=/tmp/ca-cert.pem"),
        ];

        if let Some(ref token) = self.config.oauth_token {
            env_parts.push(format!("CLAUDE_CODE_OAUTH_TOKEN={token}"));
        }

        let env_exports = env_parts
            .iter()
            .map(|e| format!("export {e}"))
            .collect::<Vec<_>>()
            .join("; ");

        // Write initial prompt to a file.
        let initial_prompt = &self.config.workspace_config.agent_config.initial_prompt;
        if !initial_prompt.is_empty() {
            let input = claude_code::protocol::InputMessage::user(initial_prompt);
            let json = serde_json::to_string(&input).unwrap_or_default();
            self.rt.block_on(async {
                session
                    .write_file("/tmp/initial-prompt.jsonl", &format!("{json}\n"))
                    .await
                    .map_err(|e| WorkspaceError::Process(format!("write prompt: {e}")))
            })?;
        }

        // Quote args for shell.
        let quoted_args: Vec<String> = std::iter::once(program)
            .chain(args)
            .map(|a| format!("'{}'", a.replace('\'', "'\\''")))
            .collect();
        let agent_cmd = quoted_args.join(" ");

        let full_cmd = if initial_prompt.is_empty() {
            format!("{env_exports}; cd /project; {agent_cmd}")
        } else {
            format!(
                "{env_exports}; cd /project; {agent_cmd} < /tmp/initial-prompt.jsonl"
            )
        };

        self.rt.block_on(async {
            session
                .exec_detached(&full_cmd)
                .await
                .map_err(|e| WorkspaceError::Process(format!("start agent: {e}")))
        })?;

        self.target = Some(target);
        self.cert = Some(cert);
        self.session = Some(session);
        self.status = WorkspaceStatus::Running;

        Ok(())
    }

    fn send_message(&mut self, _msg: &str) -> Result<(), WorkspaceError> {
        // Messages go through the supervisor API via the reverse tunnel.
        // The workspace's clc commands handle this — no direct send needed.
        Ok(())
    }

    fn recv_output(&mut self) -> Result<Vec<OutputMessage>, WorkspaceError> {
        // Output goes through the supervisor API.
        // The coordinator reads it via the API, not through the workspace.
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
        &self.config.workspace_config.tisket_id
    }

    fn stop(&mut self) -> Result<(), WorkspaceError> {
        // Destroy the environment (stops container, etc).
        if let Err(e) = self.env.destroy() {
            eprintln!("workspace destroy error: {e}");
        }
        self.status = WorkspaceStatus::Failed;
        Ok(())
    }
}

/// Docker environment: creates a container with sshd, returns SSH target.
pub struct DockerEnvironment {
    image: String,
    container_id: Option<String>,
    project_dir: PathBuf,
    worktree_dir: PathBuf,
    rt: tokio::runtime::Runtime,
}

impl DockerEnvironment {
    pub fn new(
        image: &str,
        project_dir: &Path,
        tisket_id: &str,
    ) -> Result<Self, WorkspaceError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| WorkspaceError::Process(format!("tokio: {e}")))?;

        let worktree_dir = project_dir.join(".worktrees").join(tisket_id);

        Ok(Self {
            image: image.to_string(),
            container_id: None,
            project_dir: project_dir.to_path_buf(),
            worktree_dir,
            rt,
        })
    }
}

impl Environment for DockerEnvironment {
    fn create(&mut self) -> Result<SSHTarget, WorkspaceError> {
        use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
        use bollard::models::HostConfig;

        let docker = bollard::Docker::connect_with_local_defaults()
            .map_err(|e| WorkspaceError::Process(format!("docker: {e}")))?;

        let container_id = self.rt.block_on(async {
            // Mount worktree, project .git, and SSH public key.
            let worktree_mount = format!("{}:/project", self.worktree_dir.display());
            let git_mount = format!("{}/.git:/project-git:ro", self.project_dir.display());
            let mut binds = vec![worktree_mount, git_mount];

            // Mount user's SSH public key for authentication.
            let pub_key_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".ssh")
                .join("id_ed25519.pub");
            if pub_key_path.exists() {
                binds.push(format!(
                    "{}:/root/.ssh/authorized_keys:ro",
                    pub_key_path.display()
                ));
            }

            let host_config = HostConfig {
                binds: Some(binds),
                // Expose SSH port — pick a random high port.
                publish_all_ports: Some(true),
                ..Default::default()
            };

            let config = Config {
                image: Some(self.image.as_str()),
                // Start sshd in the foreground.
                cmd: Some(vec!["/usr/sbin/sshd", "-D", "-e"]),
                exposed_ports: Some(
                    std::collections::HashMap::from([("22/tcp", Default::default())]),
                ),
                host_config: Some(host_config),
                ..Default::default()
            };

            let container = docker
                .create_container(
                    Some(CreateContainerOptions::<&str> {
                        name: "",
                        platform: None,
                    }),
                    config,
                )
                .await
                .map_err(|e| WorkspaceError::Process(format!("create container: {e}")))?;

            docker
                .start_container::<String>(&container.id, None)
                .await
                .map_err(|e| WorkspaceError::Process(format!("start container: {e}")))?;

            // Get the mapped SSH port.
            let info = docker
                .inspect_container(&container.id, None)
                .await
                .map_err(|e| WorkspaceError::Process(format!("inspect: {e}")))?;

            let ssh_port = info
                .network_settings
                .and_then(|ns| ns.ports)
                .and_then(|ports| ports.get("22/tcp").cloned())
                .and_then(|bindings| bindings)
                .and_then(|bindings| bindings.first().cloned())
                .and_then(|binding| binding.host_port)
                .and_then(|p| p.parse::<u16>().ok())
                .ok_or_else(|| {
                    WorkspaceError::Process("could not determine SSH port".to_string())
                })?;

            Ok::<(String, u16), WorkspaceError>((container.id, ssh_port))
        })?;

        let (cid, ssh_port) = container_id;
        self.container_id = Some(cid);

        Ok(SSHTarget {
            host: "127.0.0.1".to_string(),
            port: ssh_port,
            user: "root".to_string(),
        })
    }

    fn destroy(&mut self) -> Result<(), WorkspaceError> {
        if let Some(ref cid) = self.container_id {
            let docker = bollard::Docker::connect_with_local_defaults()
                .map_err(|e| WorkspaceError::Process(format!("docker: {e}")))?;
            let cid = cid.clone();
            self.rt.block_on(async {
                let _ = docker.stop_container(&cid, None).await;
                let _ = docker
                    .remove_container(
                        &cid,
                        Some(bollard::container::RemoveContainerOptions {
                            force: true,
                            ..Default::default()
                        }),
                    )
                    .await;
            });
        }
        Ok(())
    }
}
