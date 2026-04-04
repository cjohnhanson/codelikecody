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

struct SessionHandler {
    local_port: u16,
}

#[async_trait]
impl client::Handler for SessionHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<russh::client::Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let local_port = self.local_port;
        eprintln!("reverse tunnel: forwarding to localhost:{local_port}");
        tokio::spawn(async move {
            match tokio::net::TcpStream::connect(format!("127.0.0.1:{local_port}")).await {
                Ok(mut tcp) => {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                }
                Err(e) => {
                    eprintln!("reverse tunnel: failed to connect to localhost:{local_port}: {e}");
                }
            }
        });
        Ok(())
    }
}

impl SSHSession {
    /// Connect to an SSH target using key-based auth.
    /// Connect to an SSH target. `local_port` is the host port that
    /// reverse-tunneled connections are forwarded to (0 to disable).
    pub async fn connect(
        target: &SSHTarget,
        private_key_path: &Path,
        local_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Arc::new(client::Config::default());
        let handler = SessionHandler { local_port };

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
        let bound_port = self.session
            .tcpip_forward("127.0.0.1", remote_port.into())
            .await?;
        eprintln!("ssh: tcpip_forward requested port {remote_port}, server bound port {bound_port}");
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

    /// Write a text file on the remote host via clc.
    pub async fn write_file(
        &mut self,
        path: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.exec_with_stdin(
            &format!("clc workspace write-file {path}"),
            content.as_bytes(),
        )
        .await?;
        Ok(())
    }

    /// Run a command on the remote host, piping binary data to its stdin.
    /// Returns stdout as bytes.
    pub async fn exec_with_stdin(
        &mut self,
        command: &str,
        stdin_data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut channel = self.session.channel_open_session().await?;
        channel
            .exec(true, command.as_bytes().to_vec())
            .await?;

        // Write data to the command's stdin.
        channel.data(stdin_data).await?;
        channel.eof().await?;

        // Read stdout.
        let mut stdout = Vec::new();
        loop {
            match channel.wait().await {
                Some(russh::ChannelMsg::Data { data }) => {
                    stdout.extend_from_slice(&data);
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

    /// Start a long-running command. Returns immediately.
    #[allow(dead_code)]
    /// Uses setsid to detach from the SSH session so the process survives
    /// channel close.
    pub async fn exec_detached(
        &mut self,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channel = self.session.channel_open_session().await?;
        let detached_cmd = format!(
            "setsid sh -c '{command} > /tmp/agent-stdout.log 2>/tmp/agent-stderr.log' &"
        );
        channel
            .exec(true, detached_cmd.into_bytes())
            .await?;
        // Wait briefly for the process to start, then close channel.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }

    /// Disconnect.
    #[allow(dead_code)]
    pub async fn close(self) -> Result<(), Box<dyn std::error::Error>> {
        self.session
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await?;
        Ok(())
    }
}
