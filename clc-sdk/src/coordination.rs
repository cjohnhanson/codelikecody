//! CoordinationBackend trait: the storage and messaging layer between agents.
//!
//! Replaces filesystem-based communication (stdout.jsonl, pid files, stdin
//! pipes, outbox) with a structured storage backend. The first implementation
//! uses Postgres via SeaORM.
//!
//! The coordinator, workers, and admin interact through this backend.
//! CLI commands (`clc worker check`, `clc permissions`, etc.) read from
//! and write to the backend instead of the filesystem.

use std::time::SystemTime;

/// Unique identifier for an agent (worker, coordinator, admin).
pub type AgentId = String;

/// Unique identifier for a message or request.
pub type MessageId = String;

/// A message sent between agents or from an agent to the system.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub from: AgentId,
    pub to: AgentId,
    pub kind: MessageKind,
    pub timestamp: SystemTime,
}

/// The type and payload of a message.
#[derive(Debug, Clone)]
pub enum MessageKind {
    /// A text message from one agent to another (replaces stdin pipe writes).
    Text(String),

    /// Agent output line (replaces stdout.jsonl).
    Output(String),

    /// Agent requests permission to use a tool.
    PermissionRequest {
        tool_name: String,
        reason: String,
    },

    /// Permission granted (from coordinator or admin).
    PermissionGrant {
        request_id: MessageId,
        scope: String,
    },

    /// Permission denied.
    PermissionDenied {
        request_id: MessageId,
        reason: String,
    },

    /// Request for code review.
    ReviewRequest {
        branch: String,
        summary: String,
    },

    /// Review result.
    ReviewResult {
        request_id: MessageId,
        verdict: ReviewVerdict,
        comments: String,
    },

    /// Status update from an agent.
    StatusUpdate {
        phase: String,
        detail: String,
    },
}

/// Review verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewVerdict {
    Approved,
    ChangesRequested,
    Rejected,
}

/// Agent status as tracked by the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent has been created but not started.
    Pending,
    /// Agent is running.
    Running,
    /// Agent completed successfully.
    Completed,
    /// Agent failed.
    Failed,
    /// Agent was stopped by the coordinator or admin.
    Stopped,
}

/// Opaque cursor for incremental message retrieval.
#[derive(Debug, Clone, Default)]
pub struct Cursor(pub i64);

/// Error from coordination operations.
#[derive(Debug)]
pub enum CoordinationError {
    /// Storage operation failed.
    Storage(String),
    /// Agent not found.
    NotFound(String),
    /// Invalid operation for current state.
    InvalidState(String),
}

impl std::fmt::Display for CoordinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(msg) => write!(f, "coordination storage: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::InvalidState(msg) => write!(f, "invalid state: {msg}"),
        }
    }
}

impl std::error::Error for CoordinationError {}

/// The coordination layer between agents.
///
/// All inter-agent communication goes through this trait. CLI commands
/// read from and write to the backend. The coordinator monitors workers
/// by polling messages. Permission requests flow from workers through
/// the coordinator to the admin (if escalated).
///
/// Methods are async because the first production implementation
/// (Postgres via SeaORM) requires async I/O.
#[async_trait::async_trait]
pub trait CoordinationBackend: Send + Sync {
    /// Register a new agent. Returns `InvalidState` if the agent already exists.
    async fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError>;

    /// Update an agent's status. Returns `NotFound` if the agent doesn't exist.
    async fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError>;

    /// Get an agent's current status. Returns `NotFound` if the agent doesn't exist.
    async fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError>;

    /// Send a message. Returns the message ID.
    async fn send(
        &self,
        msg: Message,
    ) -> Result<MessageId, CoordinationError>;

    /// Receive messages for an agent since a cursor.
    /// Returns messages and a new cursor for the next call.
    async fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError>;

    /// Get pending permission requests for an agent (as the grantor).
    async fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError>;

    /// Get pending review requests for an agent (as the reviewer).
    async fn pending_reviews(
        &self,
        reviewer_id: &str,
    ) -> Result<Vec<Message>, CoordinationError>;

    /// List all agents with their statuses, optionally filtered by parent.
    async fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError>;
}

/// In-memory backend for testing. Not for production use.
#[derive(Default)]
pub struct MemoryBackend {
    agents: std::sync::Mutex<Vec<(AgentId, Option<AgentId>, AgentStatus)>>,
    messages: std::sync::Mutex<Vec<Message>>,
}

#[async_trait::async_trait]
impl CoordinationBackend for MemoryBackend {
    async fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError> {
        let mut agents = self.agents.lock().unwrap();
        if agents.iter().any(|(aid, _, _)| aid == id) {
            return Err(CoordinationError::InvalidState(format!(
                "agent {id} already registered"
            )));
        }
        agents.push((
            id.to_string(),
            parent_id.map(str::to_string),
            AgentStatus::Pending,
        ));
        Ok(())
    }

    async fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError> {
        let mut agents = self.agents.lock().unwrap();
        for (id, _, s) in agents.iter_mut() {
            if id == agent_id {
                *s = status;
                return Ok(());
            }
        }
        Err(CoordinationError::NotFound(agent_id.to_string()))
    }

    async fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError> {
        let agents = self.agents.lock().unwrap();
        agents
            .iter()
            .find(|(id, _, _)| id == agent_id)
            .map(|(_, _, s)| s.clone())
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))
    }

    async fn send(
        &self,
        msg: Message,
    ) -> Result<MessageId, CoordinationError> {
        let id = msg.id.clone();
        self.messages.lock().unwrap().push(msg);
        Ok(id)
    }

    async fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError> {
        let messages = self.messages.lock().unwrap();
        let msgs: Vec<_> = messages
            .iter()
            .enumerate()
            .filter(|(i, m)| *i as i64 >= cursor.0 && m.to == agent_id)
            .map(|(_, m)| m.clone())
            .collect();
        let new_cursor = Cursor(messages.len() as i64);
        Ok((msgs, new_cursor))
    }

    async fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        let messages = self.messages.lock().unwrap();
        Ok(messages
            .iter()
            .filter(|m| {
                m.to == grantor_id
                    && matches!(m.kind, MessageKind::PermissionRequest { .. })
            })
            .cloned()
            .collect())
    }

    async fn pending_reviews(
        &self,
        reviewer_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        let messages = self.messages.lock().unwrap();
        Ok(messages
            .iter()
            .filter(|m| {
                m.to == reviewer_id
                    && matches!(m.kind, MessageKind::ReviewRequest { .. })
            })
            .cloned()
            .collect())
    }

    async fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError> {
        let agents = self.agents.lock().unwrap();
        Ok(agents
            .iter()
            .filter(|(_, p, _)| match (parent_id, p) {
                (None, _) => true,
                (Some(pid), Some(p)) => pid == p,
                (Some(_), None) => false,
            })
            .map(|(id, _, s)| (id.clone(), s.clone()))
            .collect())
    }
}

/// Contract tests that any `CoordinationBackend` implementation must pass.
/// Reusable across MemoryBackend, PostgresBackend, etc.
#[cfg(test)]
pub mod contract_tests {
    use super::*;

    fn msg(id: &str, from: &str, to: &str, kind: MessageKind) -> Message {
        Message {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            kind,
            timestamp: SystemTime::now(),
        }
    }

    /// New agents start in Pending status.
    pub async fn register_starts_pending(b: &dyn CoordinationBackend) {
        b.register_agent("w1", Some("coord")).await.unwrap();
        assert_eq!(b.get_status("w1").await.unwrap(), AgentStatus::Pending);
    }

    /// Duplicate registration returns an error.
    pub async fn register_duplicate_errors(b: &dyn CoordinationBackend) {
        b.register_agent("dup", None).await.unwrap();
        let err = b.register_agent("dup", None).await;
        assert!(err.is_err());
    }

    /// Status can be updated and retrieved.
    pub async fn set_and_get_status(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();
        b.set_status("w1", AgentStatus::Running).await.unwrap();
        assert_eq!(b.get_status("w1").await.unwrap(), AgentStatus::Running);
    }

    /// Status transitions through the full lifecycle.
    pub async fn status_lifecycle(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();
        for status in [
            AgentStatus::Running,
            AgentStatus::Completed,
        ] {
            b.set_status("w1", status.clone()).await.unwrap();
            assert_eq!(b.get_status("w1").await.unwrap(), status);
        }
    }

    /// Getting status of a non-existent agent returns NotFound.
    pub async fn status_not_found(b: &dyn CoordinationBackend) {
        assert!(b.get_status("ghost").await.is_err());
    }

    /// Setting status of a non-existent agent returns NotFound.
    pub async fn set_status_not_found(b: &dyn CoordinationBackend) {
        assert!(
            b.set_status("ghost", AgentStatus::Running).await.is_err()
        );
    }

    /// Messages are delivered to the addressed agent.
    pub async fn send_and_recv(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();

        b.send(msg("m1", "coord", "w1", MessageKind::Text("hello".into())))
            .await
            .unwrap();

        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0].kind, MessageKind::Text(t) if t == "hello"));
    }

    /// Messages addressed to other agents are not returned.
    pub async fn recv_filters_by_recipient(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();
        b.register_agent("w2", None).await.unwrap();

        b.send(msg("m1", "coord", "w1", MessageKind::Text("for w1".into())))
            .await
            .unwrap();
        b.send(msg("m2", "coord", "w2", MessageKind::Text("for w2".into())))
            .await
            .unwrap();

        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0].kind, MessageKind::Text(t) if t == "for w1"));
    }

    /// Cursor advances past already-received messages.
    pub async fn cursor_tracks_position(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();

        b.send(msg("m1", "coord", "w1", MessageKind::Text("first".into())))
            .await
            .unwrap();

        let (msgs, cursor) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert_eq!(msgs.len(), 1);

        b.send(msg("m2", "coord", "w1", MessageKind::Text("second".into())))
            .await
            .unwrap();

        let (msgs, _) = b.recv("w1", &cursor).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0].kind, MessageKind::Text(t) if t == "second"));
    }

    /// Empty recv returns empty vec and valid cursor.
    pub async fn recv_empty(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();
        let (msgs, cursor) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert!(msgs.is_empty());
        // Cursor should be usable for subsequent calls.
        let (msgs2, _) = b.recv("w1", &cursor).await.unwrap();
        assert!(msgs2.is_empty());
    }

    /// Permission requests show up in pending_permissions for the grantor.
    pub async fn permission_request_flow(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();

        b.send(msg(
            "req-1",
            "w1",
            "coord",
            MessageKind::PermissionRequest {
                tool_name: "Bash(git push)".into(),
                reason: "push branch".into(),
            },
        ))
        .await
        .unwrap();

        let pending = b.pending_permissions("coord").await.unwrap();
        assert_eq!(pending.len(), 1);

        // Grant the permission.
        b.send(msg(
            "grant-1",
            "coord",
            "w1",
            MessageKind::PermissionGrant {
                request_id: "req-1".into(),
                scope: "Bash(git push *)".into(),
            },
        ))
        .await
        .unwrap();

        // Worker receives the grant.
        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert!(msgs
            .iter()
            .any(|m| matches!(&m.kind, MessageKind::PermissionGrant { .. })));
    }

    /// Permission denial flow.
    pub async fn permission_denied_flow(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();

        b.send(msg(
            "req-1",
            "w1",
            "coord",
            MessageKind::PermissionRequest {
                tool_name: "Bash(rm -rf /)".into(),
                reason: "cleanup".into(),
            },
        ))
        .await
        .unwrap();

        b.send(msg(
            "deny-1",
            "coord",
            "w1",
            MessageKind::PermissionDenied {
                request_id: "req-1".into(),
                reason: "absolutely not".into(),
            },
        ))
        .await
        .unwrap();

        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert!(msgs
            .iter()
            .any(|m| matches!(&m.kind, MessageKind::PermissionDenied { .. })));
    }

    /// Review requests show up in pending_reviews for the reviewer.
    pub async fn review_request_flow(b: &dyn CoordinationBackend) {
        b.register_agent("reviewer", None).await.unwrap();
        b.register_agent("w1", None).await.unwrap();

        b.send(msg(
            "rev-1",
            "w1",
            "reviewer",
            MessageKind::ReviewRequest {
                branch: "feat/thing".into(),
                summary: "added thing".into(),
            },
        ))
        .await
        .unwrap();

        let pending = b.pending_reviews("reviewer").await.unwrap();
        assert_eq!(pending.len(), 1);

        b.send(msg(
            "result-1",
            "reviewer",
            "w1",
            MessageKind::ReviewResult {
                request_id: "rev-1".into(),
                verdict: ReviewVerdict::Approved,
                comments: "lgtm".into(),
            },
        ))
        .await
        .unwrap();

        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert!(msgs.iter().any(|m| matches!(
            &m.kind,
            MessageKind::ReviewResult { verdict, .. } if *verdict == ReviewVerdict::Approved
        )));
    }

    /// Non-permission messages don't appear in pending_permissions.
    pub async fn pending_permissions_filters_kind(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();

        b.send(msg(
            "m1",
            "w1",
            "coord",
            MessageKind::Text("just a message".into()),
        ))
        .await
        .unwrap();

        let pending = b.pending_permissions("coord").await.unwrap();
        assert!(pending.is_empty());
    }

    /// Non-review messages don't appear in pending_reviews.
    pub async fn pending_reviews_filters_kind(b: &dyn CoordinationBackend) {
        b.register_agent("reviewer", None).await.unwrap();

        b.send(msg(
            "m1",
            "w1",
            "reviewer",
            MessageKind::Text("just a message".into()),
        ))
        .await
        .unwrap();

        let pending = b.pending_reviews("reviewer").await.unwrap();
        assert!(pending.is_empty());
    }

    /// List agents returns all agents when no parent filter.
    pub async fn list_agents_all(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();
        b.register_agent("w2", Some("coord")).await.unwrap();
        b.register_agent("w3", Some("other")).await.unwrap();

        let all = b.list_agents(None).await.unwrap();
        assert_eq!(all.len(), 4);
    }

    /// List agents filters by parent.
    pub async fn list_agents_by_parent(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();
        b.register_agent("w2", Some("coord")).await.unwrap();
        b.register_agent("w3", Some("other")).await.unwrap();

        let coord_workers = b.list_agents(Some("coord")).await.unwrap();
        assert_eq!(coord_workers.len(), 2);

        let other_workers = b.list_agents(Some("other")).await.unwrap();
        assert_eq!(other_workers.len(), 1);
    }

    /// Agents with no parent don't appear in parent-filtered queries.
    pub async fn list_agents_parentless_excluded(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        let filtered = b.list_agents(Some("coord")).await.unwrap();
        assert!(filtered.is_empty());
    }

    /// Status update messages are delivered correctly.
    pub async fn status_update_message(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();

        b.send(msg(
            "s1",
            "w1",
            "coord",
            MessageKind::StatusUpdate {
                phase: "implementing".into(),
                detail: "writing tests".into(),
            },
        ))
        .await
        .unwrap();

        let (msgs, _) = b.recv("coord", &Cursor::default()).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(
            &msgs[0].kind,
            MessageKind::StatusUpdate { phase, .. } if phase == "implementing"
        ));
    }

    /// Output messages are delivered correctly.
    pub async fn output_message(b: &dyn CoordinationBackend) {
        b.register_agent("coord", None).await.unwrap();
        b.register_agent("w1", Some("coord")).await.unwrap();

        b.send(msg(
            "o1",
            "w1",
            "coord",
            MessageKind::Output("line of output".into()),
        ))
        .await
        .unwrap();

        let (msgs, _) = b.recv("coord", &Cursor::default()).await.unwrap();
        assert!(matches!(&msgs[0].kind, MessageKind::Output(s) if s == "line of output"));
    }

    /// Multiple messages arrive in order.
    pub async fn message_ordering(b: &dyn CoordinationBackend) {
        b.register_agent("w1", None).await.unwrap();

        for i in 0..5 {
            b.send(msg(
                &format!("m{i}"),
                "coord",
                "w1",
                MessageKind::Text(format!("msg-{i}")),
            ))
            .await
            .unwrap();
        }

        let (msgs, _) = b.recv("w1", &Cursor::default()).await.unwrap();
        assert_eq!(msgs.len(), 5);
        for (i, m) in msgs.iter().enumerate() {
            assert!(matches!(&m.kind, MessageKind::Text(t) if *t == format!("msg-{i}")));
        }
    }

    /// send returns the message ID from the message.
    pub async fn send_returns_id(b: &dyn CoordinationBackend) {
        let returned_id = b
            .send(msg("my-id", "a", "b", MessageKind::Text("x".into())))
            .await
            .unwrap();
        assert_eq!(returned_id, "my-id");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::contract_tests;

    fn backend() -> MemoryBackend {
        MemoryBackend::default()
    }

    #[tokio::test]
    async fn register_starts_pending() {
        contract_tests::register_starts_pending(&backend()).await;
    }

    #[tokio::test]
    async fn register_duplicate_errors() {
        contract_tests::register_duplicate_errors(&backend()).await;
    }

    #[tokio::test]
    async fn set_and_get_status() {
        contract_tests::set_and_get_status(&backend()).await;
    }

    #[tokio::test]
    async fn status_lifecycle() {
        contract_tests::status_lifecycle(&backend()).await;
    }

    #[tokio::test]
    async fn status_not_found() {
        contract_tests::status_not_found(&backend()).await;
    }

    #[tokio::test]
    async fn set_status_not_found() {
        contract_tests::set_status_not_found(&backend()).await;
    }

    #[tokio::test]
    async fn send_and_recv() {
        contract_tests::send_and_recv(&backend()).await;
    }

    #[tokio::test]
    async fn recv_filters_by_recipient() {
        contract_tests::recv_filters_by_recipient(&backend()).await;
    }

    #[tokio::test]
    async fn cursor_tracks_position() {
        contract_tests::cursor_tracks_position(&backend()).await;
    }

    #[tokio::test]
    async fn recv_empty() {
        contract_tests::recv_empty(&backend()).await;
    }

    #[tokio::test]
    async fn permission_request_flow() {
        contract_tests::permission_request_flow(&backend()).await;
    }

    #[tokio::test]
    async fn permission_denied_flow() {
        contract_tests::permission_denied_flow(&backend()).await;
    }

    #[tokio::test]
    async fn review_request_flow() {
        contract_tests::review_request_flow(&backend()).await;
    }

    #[tokio::test]
    async fn pending_permissions_filters_kind() {
        contract_tests::pending_permissions_filters_kind(&backend()).await;
    }

    #[tokio::test]
    async fn pending_reviews_filters_kind() {
        contract_tests::pending_reviews_filters_kind(&backend()).await;
    }

    #[tokio::test]
    async fn list_agents_all() {
        contract_tests::list_agents_all(&backend()).await;
    }

    #[tokio::test]
    async fn list_agents_by_parent() {
        contract_tests::list_agents_by_parent(&backend()).await;
    }

    #[tokio::test]
    async fn list_agents_parentless_excluded() {
        contract_tests::list_agents_parentless_excluded(&backend()).await;
    }

    #[tokio::test]
    async fn status_update_message() {
        contract_tests::status_update_message(&backend()).await;
    }

    #[tokio::test]
    async fn output_message() {
        contract_tests::output_message(&backend()).await;
    }

    #[tokio::test]
    async fn message_ordering() {
        contract_tests::message_ordering(&backend()).await;
    }

    #[tokio::test]
    async fn send_returns_id() {
        contract_tests::send_returns_id(&backend()).await;
    }
}
