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

        // 6. Create git pack on the host and pipe it to the workspace
        //    via clc workspace receive. Pack created by gix, served as JSON
        //    with base64-encoded pack data + refs.
        let branch_name = self.config.workspace_config.tisket_id.clone();
        let pack_data = crate::git_pack::create_pack(
            &self.config.workspace_config.project_dir,
            &branch_name,
        )
        .map_err(|e| WorkspaceError::Process(format!("create pack: {e}")))?;

        // Serialize as JSON envelope for clc workspace receive.
        let b64 = base64_encode(&pack_data.pack);
        let refs: Vec<serde_json::Value> = pack_data
            .refs
            .iter()
            .map(|(oid, name)| serde_json::json!([oid, name]))
            .collect();
        let envelope = serde_json::json!({
            "pack": b64,
            "refs": refs,
            "branch": branch_name,
        });
        let envelope_bytes = serde_json::to_vec(&envelope)
            .map_err(|e| WorkspaceError::Process(format!("serialize: {e}")))?;

        // Pipe to clc workspace receive on the container.
        self.rt.block_on(async {
            session
                .exec_with_stdin(
                    &format!("clc workspace receive --stdin --branch {branch_name} --dir /project"),
                    &envelope_bytes,
                )
                .await
                .map_err(|e| WorkspaceError::Process(format!("receive pack: {e}")))
        })?;

        // 7. Initialize workspace and start agent — all via clc commands.
        //    clc workspace init: creates .clc/worker/ with pipes, files, hooks.
        //    clc workspace start: builds agent command via Agent trait, wires stdio,
        //    spawns process, writes PID, sends initial prompt. No shell involved.
        self.rt.block_on(async {
            session
                .exec("cd /project && clc workspace init")
                .await
                .map_err(|e| WorkspaceError::Process(format!("workspace init: {e}")))
        })?;

        let mut start_args = vec![
            "clc".to_string(),
            "workspace".to_string(),
            "start".to_string(),
            "--branch".to_string(),
            branch_name.clone(),
            "--model".to_string(),
            self.config.workspace_config.agent_config.model.clone(),
        ];

        let api_url = format!("http://localhost:{tunnel_port}");
        start_args.push("--api-url".to_string());
        start_args.push(api_url);

        if let Some(ref token) = self.config.oauth_token {
            start_args.push("--oauth-token".to_string());
            start_args.push(token.clone());
        }

        let start_cmd = start_args.join(" ");
        let pid_output = self.rt.block_on(async {
            session
                .exec(&format!("cd /project && {start_cmd}"))
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
