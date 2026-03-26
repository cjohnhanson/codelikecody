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
    pub ca: Arc<EphemeralCA>,
    pub api_port: u16,
    pub oauth_token: Option<String>,
    /// Custom start command. When set, overrides the default `clc workspace start`.
    /// Used for coordinators in Docker which run `clc coordinator-run` instead.
    pub start_command: Option<Vec<String>>,
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
        eprintln!("ssh workspace: creating environment...");
        let target = self.env.create()?;
        eprintln!("ssh workspace: environment created ({}:{})", target.host, target.port);

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

        eprintln!("ssh workspace: connecting via SSH...");
        let mut session = self.rt.block_on(async {
            // Wait for sshd to be ready.
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            SSHSession::connect(&target, &ssh_key_path)
                .await
                .map_err(|e| WorkspaceError::Process(format!("SSH connect: {e}")))
        })?;
        eprintln!("ssh workspace: SSH connected");

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

        eprintln!("ssh workspace: certs deployed");

        // 5. Set up reverse tunnel: workspace's localhost:tunnel_port → supervisor's API.
        let tunnel_port = self.tunnel_port;
        let api_port = self.config.api_port;
        self.rt.block_on(async {
            session
                .setup_reverse_tunnel(tunnel_port, api_port)
                .await
                .map_err(|e| WorkspaceError::Process(format!("reverse tunnel: {e}")))
        })?;

        eprintln!("ssh workspace: reverse tunnel established");

        // 6. Create git pack on the host and pipe it to the workspace
        //    via clc workspace receive. Pack created by gix, served as JSON
        //    with base64-encoded pack data + refs.
        // Coordinators (custom start_command) run on main; workers run on their branch.
        let branch_name = if self.config.start_command.is_some() {
            self.config.workspace_config.main_branch.clone()
        } else {
            self.config.workspace_config.tisket_id.clone()
        };
        let pack_data = crate::git_pack::create_pack(
            &self.config.workspace_config.project_dir,
            &branch_name,
        )
        .map_err(|e| WorkspaceError::Process(format!("create pack: {e}")))?;
        eprintln!("ssh workspace: git pack created ({} bytes)", pack_data.pack.len());

        // Transfer pack and refs as files via SSH, then unpack.
        let refs_json = serde_json::to_string(&pack_data.refs
            .iter()
            .map(|(oid, name)| serde_json::json!([oid, name]))
            .collect::<Vec<_>>())
            .unwrap_or_default();

        eprintln!("ssh workspace: transferring pack via SSH...");
        self.rt.block_on(async {
            session
                .exec_with_stdin(
                    "clc workspace write-file /tmp/repo.pack",
                    &pack_data.pack,
                )
                .await
                .map_err(|e| WorkspaceError::Process(format!("write pack: {e}")))?;
            session
                .exec_with_stdin(
                    "clc workspace write-file /tmp/repo.refs",
                    refs_json.as_bytes(),
                )
                .await
                .map_err(|e| WorkspaceError::Process(format!("write refs: {e}")))?;
            Ok::<_, WorkspaceError>(())
        })?;
        eprintln!("ssh workspace: pack transferred");

        self.rt.block_on(async {
            session
                .exec(&format!(
                    "clc workspace receive --pack-file /tmp/repo.pack --refs-file /tmp/repo.refs --branch {branch_name} --dir /project"
                ))
                .await
                .map_err(|e| WorkspaceError::Process(format!("receive pack: {e}")))
        })?;
        eprintln!("ssh workspace: pack unpacked");

        // 7. Initialize workspace and start agent — all via clc commands.
        //    clc workspace init: creates .clc/worker/ with pipes, files, hooks.
        //    clc workspace start: builds agent command via Agent trait, wires stdio,
        //    spawns process, writes PID, sends initial prompt. No shell involved.
        let init_result = self.rt.block_on(async {
            session
                .exec("cd /project && clc workspace init 2>&1")
                .await
        });
        match init_result {
            Ok(output) => {
                if !output.is_empty() {
                    eprintln!("ssh workspace: init output: {output}");
                }
            }
            Err(e) => {
                // Try to get more info.
                let debug = self.rt.block_on(async {
                    session.exec("ls -la /project/ 2>&1 && ls -la /project/.git/ 2>&1").await
                });
                if let Ok(ls) = debug {
                    eprintln!("ssh workspace: /project contents: {ls}");
                }
                return Err(WorkspaceError::Process(format!("workspace init: {e}")));
            }
        }

        let start_args = if let Some(ref custom) = self.config.start_command {
            // Custom start command (e.g. coordinator-run).
            // API URL and token are passed as env vars via workspace init,
            // not as CLI args — the custom command controls its own flags.
            custom.clone()
        } else {
            // Default: start a worker agent via clc workspace start.
            let mut args = vec![
                "clc".to_string(),
                "workspace".to_string(),
                "start".to_string(),
                "--branch".to_string(),
                branch_name.clone(),
                "--model".to_string(),
                self.config.workspace_config.agent_config.model.clone(),
            ];

            let api_url = format!("https://localhost:{tunnel_port}");
            args.push("--api-url".to_string());
            args.push(api_url);

            if let Some(ref token) = self.config.oauth_token {
                args.push("--oauth-token".to_string());
                args.push(token.clone());
            }

            args
        };

        let start_cmd = start_args.join(" ");
        // coordinator-run is long-running; background it so exec returns.
        // clc workspace start already daemonizes. Both keep the SSH session
        // alive via the reverse tunnel held by the supervisor.
        let exec_cmd = if self.config.start_command.is_some() {
            // Write a launcher script with inline PEM certs as env vars.
            // Avoids file I/O issues from SSH write_file.
            let script = format!(
                "#!/bin/sh\ncd /project\nexport CLC_API_URL=https://localhost:{tunnel_port}\nexport CLC_API_CERT='{cert_pem}'\nexport CLC_API_KEY='{key_pem}'\nexport CLC_API_CA='{ca_pem}'\nexec {start_cmd} > /tmp/agent.log 2>&1\n",
                tunnel_port = tunnel_port,
                cert_pem = cert.cert_pem,
                key_pem = cert.key_pem,
                ca_pem = self.config.ca.ca_cert_pem,
                start_cmd = start_cmd,
            );
            self.rt.block_on(async {
                session
                    .write_file("/tmp/start.sh", &script)
                    .await
                    .map_err(|e| WorkspaceError::Process(format!("write launcher: {e}")))
            })?;
            self.rt.block_on(async {
                session
                    .exec("chmod +x /tmp/start.sh")
                    .await
                    .map_err(|e| WorkspaceError::Process(format!("chmod: {e}")))
            })?;
            "nohup /tmp/start.sh &\nsleep 1\ncat /proc/$(pgrep -f coordinator-run | head -1)/status 2>/dev/null | head -1 || echo 'PID unknown'".to_string()
        } else {
            format!("cd /project && {start_cmd}")
        };
        let pid_output = self.rt.block_on(async {
            session
                .exec(&exec_cmd)
                .await
                .map_err(|e| WorkspaceError::Process(format!("workspace start: {e}")))
        })?;

        eprintln!(
            "ssh workspace: agent started ({})",
            pid_output.trim()
        );

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

/// Base64 encode binary data.
pub fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    let mut buf = Vec::new();
    let mut encoder = base64_writer(&mut buf);
    encoder.write_all(data).unwrap();
    drop(encoder);
    String::from_utf8(buf).unwrap_or_default()
}

fn base64_writer(w: &mut Vec<u8>) -> impl std::io::Write + '_ {
    // Simple base64 encoder — no external crate needed.
    struct B64Writer<'a>(&'a mut Vec<u8>, Vec<u8>);

    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    impl<'a> std::io::Write for B64Writer<'a> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.1.extend_from_slice(buf);
            while self.1.len() >= 3 {
                let chunk: [u8; 3] = [self.1[0], self.1[1], self.1[2]];
                self.0.push(CHARS[((chunk[0] >> 2) & 0x3F) as usize]);
                self.0.push(CHARS[(((chunk[0] & 0x03) << 4) | (chunk[1] >> 4)) as usize]);
                self.0.push(CHARS[(((chunk[1] & 0x0F) << 2) | (chunk[2] >> 6)) as usize]);
                self.0.push(CHARS[(chunk[2] & 0x3F) as usize]);
                self.1.drain(..3);
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            let len = self.1.len();
            if len == 1 {
                self.0.push(CHARS[((self.1[0] >> 2) & 0x3F) as usize]);
                self.0.push(CHARS[((self.1[0] & 0x03) << 4) as usize]);
                self.0.push(b'=');
                self.0.push(b'=');
            } else if len == 2 {
                self.0.push(CHARS[((self.1[0] >> 2) & 0x3F) as usize]);
                self.0.push(CHARS[(((self.1[0] & 0x03) << 4) | (self.1[1] >> 4)) as usize]);
                self.0.push(CHARS[((self.1[1] & 0x0F) << 2) as usize]);
                self.0.push(b'=');
            }
            self.1.clear();
            Ok(())
        }
    }

    impl<'a> Drop for B64Writer<'a> {
        fn drop(&mut self) {
            let _ = std::io::Write::flush(self);
        }
    }

    B64Writer(w, Vec::new())
}

/// Docker environment: creates a container with sshd, returns SSH target.
pub struct DockerEnvironment {
    image: String,
    container_id: Option<String>,
    #[allow(dead_code)]
    project_dir: PathBuf,
    #[allow(dead_code)]
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
        use bollard::container::{Config, CreateContainerOptions};
        use bollard::models::HostConfig;

        let docker = bollard::Docker::connect_with_local_defaults()
            .map_err(|e| WorkspaceError::Process(format!("docker: {e}")))?;

        let container_id = self.rt.block_on(async {
            // Only mount SSH public key for authentication. No project code —
            // the repo is pushed via gix over SSH after the container starts.
            let mut binds: Vec<String> = Vec::new();

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
