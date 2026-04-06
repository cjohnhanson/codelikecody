//! Coordination layer: sync interface for coordination operations.
//!
//! Detects `CLC_API_URL` env var. If set, operations go through HTTP
//! to the supervisor API. If not, opens SQLite directly (local mode).

use std::path::Path;
use std::sync::Arc;

use clc_sdk::coordination::{
    AgentId, AgentStatus, CoordinationBackend, CoordinationError, Cursor,
    Message, MessageId,
};
use clc_sdk::coordination_db::DbBackend;

use crate::coordination_client::ApiClient;

enum Backend {
    Db {
        backend: Arc<DbBackend>,
        rt: tokio::runtime::Runtime,
    },
    Api(ApiClient),
}

/// Sync coordination handle. Routes to SQLite or HTTP based on env.
pub struct Coordination {
    inner: Backend,
}

impl Coordination {
    /// Open coordination. If `CLC_API_URL` is set, use the HTTP API.
    /// Otherwise, open SQLite at `.clc/coordination.db`.
    pub fn open(project_dir: &Path) -> Result<Self, CoordinationError> {
        if let Ok(api_url) = std::env::var("CLC_API_URL") {
            let client = ApiClient::new(&api_url)?;
            return Ok(Self {
                inner: Backend::Api(client),
            });
        }

        let db_path = project_dir.join(".clc").join("coordination.db");

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
            inner: Backend::Db {
                backend: Arc::new(backend),
                rt,
            },
        })
    }

    pub fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.register_agent(id, parent_id)),
            Backend::Api(client) => client.register_agent(id, parent_id),
        }
    }

    /// Register an agent and return its bearer token for API authentication.
    /// In DB mode, generates and stores the token locally.
    /// In API mode, the API generates and returns the token.
    pub fn register_agent_with_token(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<String, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                rt.block_on(backend.register_agent(id, parent_id))?;
                let token = crate::config::generate_token();
                rt.block_on(backend.set_token(id, &token))?;
                Ok(token)
            }
            Backend::Api(client) => client.register_agent_with_token(id, parent_id),
        }
    }

    pub fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.set_status(agent_id, status)),
            Backend::Api(client) => client.set_status(agent_id, status),
        }
    }

    pub fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.get_status(agent_id)),
            Backend::Api(client) => client.get_status(agent_id),
        }
    }

    pub fn send(&self, msg: Message) -> Result<MessageId, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.send(msg)),
            Backend::Api(client) => client.send(msg),
        }
    }

    pub fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.recv(agent_id, cursor)),
            Backend::Api(client) => client.recv(agent_id, cursor),
        }
    }

    pub fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                rt.block_on(backend.pending_permissions(grantor_id))
            }
            Backend::Api(client) => client.pending_permissions(grantor_id),
        }
    }

    #[allow(dead_code)]
    pub fn pending_reviews(
        &self,
        reviewer_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                rt.block_on(backend.pending_reviews(reviewer_id))
            }
            Backend::Api(_) => Ok(Vec::new()), // Not yet implemented in API.
        }
    }

    #[allow(dead_code)]
    pub fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.list_agents(parent_id)),
            Backend::Api(client) => client.list_agents(parent_id),
        }
    }

    pub fn get_phase(
        &self,
        agent_id: &str,
    ) -> Result<Option<(String, i32, Option<String>)>, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.get_phase(agent_id)),
            Backend::Api(_) => {
                // Supervisor always uses DB backend directly.
                Err(CoordinationError::Storage("get_phase not available via API".into()))
            }
        }
    }

    /// Set phase directly via DB (supervisor use only — bypasses API validation
    /// because the supervisor IS the validation authority).
    pub fn set_phase_via_db(
        &self,
        agent_id: &str,
        phase: &str,
        workflow: Option<&str>,
    ) -> Result<(), CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                rt.block_on(backend.set_phase(agent_id, phase, 0, workflow))
            }
            Backend::Api(_) => {
                Err(CoordinationError::Storage("set_phase_via_db not available via API".into()))
            }
        }
    }

    pub fn grant_permission(
        &self,
        agent_id: &str,
        tool_pattern: &str,
        granted_by: &str,
        reason: &str,
    ) -> Result<(), CoordinationError> {
        // Use agent_id as session_id — one session per agent for now.
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(
                backend.grant_permission(agent_id, agent_id, tool_pattern, granted_by, reason),
            ),
            Backend::Api(_) => {
                // API clients don't grant directly — coordinator uses DB.
                Ok(())
            }
        }
    }

    #[allow(dead_code)]
    pub fn check_permission(
        &self,
        agent_id: &str,
        tool_name: &str,
    ) -> Result<bool, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                rt.block_on(backend.check_permission(agent_id, tool_name))
            }
            Backend::Api(_) => Ok(false),
        }
    }

    pub fn set_pid(
        &self,
        agent_id: &str,
        pid: Option<i32>,
    ) -> Result<(), CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.set_pid(agent_id, pid)),
            Backend::Api(_) => {
                // PID tracking goes through PATCH /agents/:id {pid}
                // For now, skip for API clients — pid is a local concern.
                Ok(())
            }
        }
    }

    /// Get the registration timestamp for an agent as a `SystemTime`.
    pub fn get_agent_created_at(
        &self,
        agent_id: &str,
    ) -> Result<std::time::SystemTime, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => {
                let dt = rt.block_on(backend.get_agent_created_at(agent_id))?;
                let secs = dt.timestamp();
                let nanos = dt.timestamp_subsec_nanos();
                Ok(std::time::UNIX_EPOCH
                    + std::time::Duration::new(secs as u64, nanos))
            }
            Backend::Api(_) => {
                Err(CoordinationError::Storage(
                    "get_agent_created_at not available via API".into(),
                ))
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_pid(
        &self,
        agent_id: &str,
    ) -> Result<Option<i32>, CoordinationError> {
        match &self.inner {
            Backend::Db { backend, rt } => rt.block_on(backend.get_pid(agent_id)),
            Backend::Api(_) => Ok(None),
        }
    }
}

