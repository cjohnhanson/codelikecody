//! Coordination layer: sync wrapper around the async CoordinationBackend.
//!
//! Manages the SQLite database lifecycle and provides blocking methods
//! for the synchronous CLI code.

use std::path::Path;
use std::sync::Arc;

use clc_sdk::coordination::{
    AgentId, AgentStatus, CoordinationBackend, CoordinationError, Cursor,
    Message, MessageId,
};
use clc_sdk::coordination_db::DbBackend;

/// Sync coordination handle. Wraps the async backend with a tokio runtime.
pub struct Coordination {
    backend: Arc<DbBackend>,
    rt: tokio::runtime::Runtime,
}

impl Coordination {
    /// Open or create the coordination database at `.clc/coordination.db`
    /// inside `project_dir`.
    pub fn open(project_dir: &Path) -> Result<Self, CoordinationError> {
        let db_path = project_dir.join(".clc").join("coordination.db");

        // Ensure parent directory exists.
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoordinationError::Storage(format!("create .clc dir: {e}"))
            })?;
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| CoordinationError::Storage(format!("tokio runtime: {e}")))?;

        let backend = rt.block_on(async {
            let db = DbBackend::connect(&url).await?;
            db.create_tables().await?;
            Ok::<_, CoordinationError>(db)
        })?;

        Ok(Self {
            backend: Arc::new(backend),
            rt,
        })
    }

    pub fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError> {
        self.rt
            .block_on(self.backend.register_agent(id, parent_id))
    }

    pub fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError> {
        self.rt
            .block_on(self.backend.set_status(agent_id, status))
    }

    pub fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError> {
        self.rt.block_on(self.backend.get_status(agent_id))
    }

    pub fn send(&self, msg: Message) -> Result<MessageId, CoordinationError> {
        self.rt.block_on(self.backend.send(msg))
    }

    pub fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError> {
        self.rt.block_on(self.backend.recv(agent_id, cursor))
    }

    pub fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        self.rt
            .block_on(self.backend.pending_permissions(grantor_id))
    }

    #[allow(dead_code)] // Will be wired when review flows are implemented.
    pub fn pending_reviews(
        &self,
        reviewer_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        self.rt
            .block_on(self.backend.pending_reviews(reviewer_id))
    }

    pub fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError> {
        self.rt.block_on(self.backend.list_agents(parent_id))
    }
}
