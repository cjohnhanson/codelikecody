//! SSH session management: connect, reverse tunnel, exec commands.
//!
//! Wraps russh to provide a simple interface for workspace communication.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use russh::client;
use russh::keys::key;

use crate::ssh_workspace::SSHTarget;

/// An active SSH connection to a workspace.
pub struct SSHSession {
    session: client::Handle<SessionHandler>,
    #[allow(dead_code)]
    target: SSHTarget,
}

struct SessionHandler;

#[async_trait]
impl client::Handler for SessionHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys — these are ephemeral containers we just created.
        Ok(true)
    }
}

impl SSHSession {
    /// Connect to an SSH target using key-based auth.
    pub async fn connect(
        target: &SSHTarget,
        private_key_path: &Path,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Arc::new(client::Config::default());
        let handler = SessionHandler;

        let mut session =
            client::connect(config, (target.host.as_str(), target.port), handler).await?;

        // Load private key.
        let key_pair = russh::keys::load_secret_key(private_key_path, None)?;

        // Authenticate.
        let auth_result = session
            .authenticate_publickey(&target.user, Arc::new(key_pair))
            .await?;

        if !auth_result {
            return Err("SSH authentication failed".into());
        }

        Ok(Self {
            session,
            target: target.clone(),
        })
    }

    /// Set up a reverse tunnel: remote port on the workspace forwards to
    /// local port on the supervisor.
    pub async fn setup_reverse_tunnel(
        &mut self,
        remote_port: u16,
        _local_port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.session
            .tcpip_forward("127.0.0.1", remote_port.into())
            .await?;
        Ok(())
    }

    /// Execute a command on the remote host and return stdout.
    pub async fn exec(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command.as_bytes().to_vec()).await?;

        let mut stdout = String::new();
        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Data { data }) => {
                    stdout.push_str(&String::from_utf8_lossy(&data));
                }
                Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                    if exit_status != 0 {
                        return Err(format!("command exited with {exit_status}").into());
                    }
                    break;
                }
                Some(russh::ChannelMsg::Eof) => break,
                None => break,
                _ => {}
            }
        }

        Ok(stdout)
    }

    /// Write a file on the remote host.
    pub async fn write_file(
        &mut self,
        path: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let escaped = content.replace('\'', "'\\''");
        self.exec(&format!("cat > {path} << 'CLCEOF'\n{escaped}\nCLCEOF"))
            .await?;
        Ok(())
    }

    /// Start a long-running command (agent process). Returns immediately.
    pub async fn exec_detached(
        &mut self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut channel = self.session.channel_open_session().await?;
        channel
            .exec(true, format!("nohup {command} &").into_bytes())
            .await?;
        Ok(())
    }

    /// Disconnect.
    pub async fn close(self) -> Result<(), Box<dyn std::error::Error>> {
        self.session
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await?;
        Ok(())
    }
}
