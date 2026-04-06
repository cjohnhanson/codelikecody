//! Database-backed [`CoordinationBackend`] via SeaORM.
//!
//! Works with both SQLite and Postgres. Enabled by the `sqlite` or
//! `postgres` feature flags respectively.

use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement,
};

use crate::coordination::{
    AgentId, AgentStatus, CoordinationBackend, CoordinationError, Cursor,
    Message, MessageId, MessageKind, ReviewVerdict,
};

/// SeaORM entity for the `coordination_agents` table.
mod agent_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "coordination_agents")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub parent_id: Option<String>,
        pub status: String,
        pub pid: Option<i32>,
        pub token: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for the `coordination_messages` table.
mod message_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "coordination_messages")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        /// Auto-incrementing sequence for cursor-based retrieval.
        /// Database assigns the value on insert (BIGSERIAL/AUTOINCREMENT).
        #[sea_orm(unique, default_value = "0")]
        pub seq: i64,
        pub from_agent: String,
        pub to_agent: String,
        /// Message kind discriminator (text, output, permission_request, etc.).
        pub kind: String,
        /// JSON payload for the message kind's fields.
        pub payload: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for agent sessions.
mod session_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_sessions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub agent_id: String,
        pub claude_session_id: Option<String>,
        pub phase: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for permission grants.
mod grant_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "permission_grants")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub session_id: String,
        pub agent_id: String,
        pub tool_pattern: String,
        pub granted_by: String,
        pub reason: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for phase state.
mod phase_entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "phase_state")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub agent_id: String,
        pub phase: String,
        pub attempts: i32,
        pub workflow: Option<String>,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Database-backed coordination backend via SeaORM.
///
/// Works with any SeaORM-supported database (SQLite, Postgres).
/// The database type is detected from the connection and appropriate
/// DDL is used for table creation.
pub struct DbBackend {
    db: DatabaseConnection,
}

impl DbBackend {
    /// Create a new backend from an existing database connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Connect to a database by URL and create a backend.
    ///
    /// SQLite: `sqlite:///path/to/db.sqlite?mode=rwc`
    /// Postgres: `postgres://user@host/dbname`
    pub async fn connect(url: &str) -> Result<Self, CoordinationError> {
        let db = sea_orm::Database::connect(url)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        Ok(Self { db })
    }

    /// Create the tables if they don't exist. Call once at startup.
    pub async fn create_tables(&self) -> Result<(), CoordinationError> {
        let backend = self.db.get_database_backend();
        let statements: &[&str] = match backend {
            sea_orm::DatabaseBackend::Postgres => &POSTGRES_DDL,
            sea_orm::DatabaseBackend::Sqlite => &SQLITE_DDL,
            sea_orm::DatabaseBackend::MySql => {
                return Err(CoordinationError::Storage(
                    "MySQL not supported".to_string(),
                ));
            }
        };

        for sql in statements {
            self.db
                .execute(Statement::from_string(backend, (*sql).to_string()))
                .await
                .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Access the underlying database connection.
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

const POSTGRES_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS coordination_agents (
        id TEXT PRIMARY KEY,
        parent_id TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        pid INTEGER,
        created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )",
    "CREATE TABLE IF NOT EXISTS coordination_messages (
        id TEXT PRIMARY KEY,
        seq BIGSERIAL UNIQUE NOT NULL,
        from_agent TEXT NOT NULL,
        to_agent TEXT NOT NULL,
        kind TEXT NOT NULL,
        payload TEXT NOT NULL DEFAULT '{}',
        created_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )",
    "CREATE INDEX IF NOT EXISTS idx_messages_to_seq
        ON coordination_messages (to_agent, seq)",
    "CREATE INDEX IF NOT EXISTS idx_messages_kind_to
        ON coordination_messages (kind, to_agent)",
    "CREATE TABLE IF NOT EXISTS agent_sessions (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        claude_session_id TEXT,
        phase TEXT NOT NULL DEFAULT 'tests-unwritten',
        created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )",
    "CREATE TABLE IF NOT EXISTS permission_grants (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        tool_pattern TEXT NOT NULL,
        granted_by TEXT NOT NULL,
        reason TEXT NOT NULL DEFAULT '',
        created_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )",
    "CREATE INDEX IF NOT EXISTS idx_grants_session
        ON permission_grants (session_id, agent_id)",
    "CREATE TABLE IF NOT EXISTS phase_state (
        agent_id TEXT PRIMARY KEY,
        phase TEXT NOT NULL DEFAULT 'tests-unwritten',
        attempts INTEGER NOT NULL DEFAULT 0,
        workflow TEXT,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
    )",
];

const SQLITE_DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS coordination_agents (
        id TEXT PRIMARY KEY,
        parent_id TEXT,
        status TEXT NOT NULL DEFAULT 'pending',
        pid INTEGER,
        token TEXT,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    "CREATE TABLE IF NOT EXISTS coordination_messages (
        id TEXT PRIMARY KEY,
        seq INTEGER,
        from_agent TEXT NOT NULL,
        to_agent TEXT NOT NULL,
        kind TEXT NOT NULL,
        payload TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_messages_to_seq
        ON coordination_messages (to_agent, seq)",
    "CREATE INDEX IF NOT EXISTS idx_messages_kind_to
        ON coordination_messages (kind, to_agent)",
    "CREATE TABLE IF NOT EXISTS agent_sessions (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        claude_session_id TEXT,
        phase TEXT NOT NULL DEFAULT 'tests-unwritten',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    "CREATE TABLE IF NOT EXISTS permission_grants (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        agent_id TEXT NOT NULL,
        tool_pattern TEXT NOT NULL,
        granted_by TEXT NOT NULL,
        reason TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
    "CREATE INDEX IF NOT EXISTS idx_grants_session
        ON permission_grants (session_id, agent_id)",
    "CREATE TABLE IF NOT EXISTS phase_state (
        agent_id TEXT PRIMARY KEY,
        phase TEXT NOT NULL DEFAULT 'tests-unwritten',
        attempts INTEGER NOT NULL DEFAULT 0,
        workflow TEXT,
        updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    )",
];

fn status_to_str(status: &AgentStatus) -> &'static str {
    match status {
        AgentStatus::Pending => "pending",
        AgentStatus::Running => "running",
        AgentStatus::Completed => "completed",
        AgentStatus::Failed => "failed",
        AgentStatus::Stopped => "stopped",
    }
}

fn str_to_status(s: &str) -> Result<AgentStatus, CoordinationError> {
    match s {
        "pending" => Ok(AgentStatus::Pending),
        "running" => Ok(AgentStatus::Running),
        "completed" => Ok(AgentStatus::Completed),
        "failed" => Ok(AgentStatus::Failed),
        "stopped" => Ok(AgentStatus::Stopped),
        other => Err(CoordinationError::Storage(format!(
            "unknown agent status: {other}"
        ))),
    }
}

fn kind_to_str(kind: &MessageKind) -> &'static str {
    match kind {
        MessageKind::Text(_) => "text",
        MessageKind::Output(_) => "output",
        MessageKind::PermissionRequest { .. } => "permission_request",
        MessageKind::PermissionGrant { .. } => "permission_grant",
        MessageKind::PermissionDenied { .. } => "permission_denied",
        MessageKind::ReviewRequest { .. } => "review_request",
        MessageKind::ReviewResult { .. } => "review_result",
        MessageKind::StatusUpdate { .. } => "status_update",
    }
}

fn kind_to_payload(kind: &MessageKind) -> String {
    match kind {
        MessageKind::Text(t) => serde_json::json!({ "text": t }).to_string(),
        MessageKind::Output(t) => serde_json::json!({ "output": t }).to_string(),
        MessageKind::PermissionRequest { tool_name, reason } => {
            serde_json::json!({ "tool_name": tool_name, "reason": reason }).to_string()
        }
        MessageKind::PermissionGrant { request_id, scope } => {
            serde_json::json!({ "request_id": request_id, "scope": scope }).to_string()
        }
        MessageKind::PermissionDenied { request_id, reason } => {
            serde_json::json!({ "request_id": request_id, "reason": reason }).to_string()
        }
        MessageKind::ReviewRequest { review_type, branch, summary } => {
            serde_json::json!({ "review_type": review_type, "branch": branch, "summary": summary }).to_string()
        }
        MessageKind::ReviewResult {
            request_id,
            review_type,
            verdict,
            comments,
            diff_hash,
        } => {
            let v = match verdict {
                ReviewVerdict::Approved => "approved",
                ReviewVerdict::ChangesRequested => "changes_requested",
                ReviewVerdict::Rejected => "rejected",
            };
            let mut obj = serde_json::json!({ "request_id": request_id, "review_type": review_type, "verdict": v, "comments": comments });
            if let Some(h) = diff_hash {
                obj["diff_hash"] = serde_json::Value::String(h.clone());
            }
            obj.to_string()
        }
        MessageKind::StatusUpdate { phase, detail } => {
            serde_json::json!({ "phase": phase, "detail": detail }).to_string()
        }
    }
}

fn payload_to_kind(
    kind_str: &str,
    payload: &str,
) -> Result<MessageKind, CoordinationError> {
    let v: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| CoordinationError::Storage(format!("bad payload JSON: {e}")))?;

    match kind_str {
        "text" => Ok(MessageKind::Text(
            v["text"].as_str().unwrap_or_default().to_string(),
        )),
        "output" => Ok(MessageKind::Output(
            v["output"].as_str().unwrap_or_default().to_string(),
        )),
        "permission_request" => Ok(MessageKind::PermissionRequest {
            tool_name: v["tool_name"].as_str().unwrap_or_default().to_string(),
            reason: v["reason"].as_str().unwrap_or_default().to_string(),
        }),
        "permission_grant" => Ok(MessageKind::PermissionGrant {
            request_id: v["request_id"].as_str().unwrap_or_default().to_string(),
            scope: v["scope"].as_str().unwrap_or_default().to_string(),
        }),
        "permission_denied" => Ok(MessageKind::PermissionDenied {
            request_id: v["request_id"].as_str().unwrap_or_default().to_string(),
            reason: v["reason"].as_str().unwrap_or_default().to_string(),
        }),
        "review_request" => Ok(MessageKind::ReviewRequest {
            review_type: v["review_type"].as_str().unwrap_or_default().to_string(),
            branch: v["branch"].as_str().unwrap_or_default().to_string(),
            summary: v["summary"].as_str().unwrap_or_default().to_string(),
        }),
        "review_result" => {
            let verdict = match v["verdict"].as_str().unwrap_or_default() {
                "approved" => ReviewVerdict::Approved,
                "changes_requested" => ReviewVerdict::ChangesRequested,
                "rejected" => ReviewVerdict::Rejected,
                other => {
                    return Err(CoordinationError::Storage(format!(
                        "unknown verdict: {other}"
                    )));
                }
            };
            Ok(MessageKind::ReviewResult {
                request_id: v["request_id"].as_str().unwrap_or_default().to_string(),
                review_type: v["review_type"].as_str().unwrap_or_default().to_string(),
                verdict,
                comments: v["comments"].as_str().unwrap_or_default().to_string(),
                diff_hash: v["diff_hash"].as_str().map(String::from),
            })
        }
        "status_update" => Ok(MessageKind::StatusUpdate {
            phase: v["phase"].as_str().unwrap_or_default().to_string(),
            detail: v["detail"].as_str().unwrap_or_default().to_string(),
        }),
        other => Err(CoordinationError::Storage(format!(
            "unknown message kind: {other}"
        ))),
    }
}

fn model_to_message(
    model: &message_entity::Model,
) -> Result<Message, CoordinationError> {
    let kind = payload_to_kind(&model.kind, &model.payload)?;
    Ok(Message {
        id: model.id.clone(),
        from: model.from_agent.clone(),
        to: model.to_agent.clone(),
        kind,
        timestamp: model.created_at.into(),
    })
}

#[async_trait::async_trait]
impl CoordinationBackend for DbBackend {
    async fn register_agent(
        &self,
        id: &str,
        parent_id: Option<&str>,
    ) -> Result<(), CoordinationError> {
        let existing = agent_entity::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        if existing.is_some() {
            return Err(CoordinationError::InvalidState(format!(
                "agent {id} already registered"
            )));
        }

        let now = chrono::Utc::now();
        let model = agent_entity::ActiveModel {
            id: Set(id.to_string()),
            parent_id: Set(parent_id.map(str::to_string)),
            status: Set("pending".to_string()),
            pid: Set(None),
            token: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        model
            .insert(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn set_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> Result<(), CoordinationError> {
        let existing = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        let model = existing
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        let mut active: agent_entity::ActiveModel = model.into();
        active.status = Set(status_to_str(&status).to_string());
        active.updated_at = Set(chrono::Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_status(
        &self,
        agent_id: &str,
    ) -> Result<AgentStatus, CoordinationError> {
        let model = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        str_to_status(&model.status)
    }

    async fn send(
        &self,
        msg: Message,
    ) -> Result<MessageId, CoordinationError> {
        let id = msg.id.clone();

        // SQLite lacks BIGSERIAL; compute next seq manually.
        // Postgres uses BIGSERIAL (NotSet lets the DB assign it).
        let seq = if self.db.get_database_backend() == sea_orm::DatabaseBackend::Sqlite {
            let row = self.db
                .query_one(Statement::from_string(
                    sea_orm::DatabaseBackend::Sqlite,
                    "SELECT COALESCE(MAX(seq), 0) + 1 AS next_seq FROM coordination_messages".to_string(),
                ))
                .await
                .map_err(|e| CoordinationError::Storage(e.to_string()))?;
            let next: i64 = row
                .map(|r| r.try_get_by_index::<i64>(0).unwrap_or(1))
                .unwrap_or(1);
            Set(next)
        } else {
            sea_orm::ActiveValue::NotSet
        };

        let model = message_entity::ActiveModel {
            id: Set(msg.id),
            seq,
            from_agent: Set(msg.from),
            to_agent: Set(msg.to),
            kind: Set(kind_to_str(&msg.kind).to_string()),
            payload: Set(kind_to_payload(&msg.kind)),
            created_at: Set(chrono::Utc::now()),
        };

        model
            .insert(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(id)
    }

    async fn recv(
        &self,
        agent_id: &str,
        cursor: &Cursor,
    ) -> Result<(Vec<Message>, Cursor), CoordinationError> {
        let models = message_entity::Entity::find()
            .filter(
                Condition::all()
                    .add(message_entity::Column::ToAgent.eq(agent_id))
                    .add(message_entity::Column::Seq.gt(cursor.0)),
            )
            .order_by_asc(message_entity::Column::Seq)
            .all(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        let new_cursor = models
            .last()
            .map_or(Cursor(cursor.0), |m| Cursor(m.seq));

        let msgs: Result<Vec<_>, _> =
            models.iter().map(model_to_message).collect();

        Ok((msgs?, new_cursor))
    }

    async fn pending_permissions(
        &self,
        grantor_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        let models = message_entity::Entity::find()
            .filter(
                Condition::all()
                    .add(message_entity::Column::ToAgent.eq(grantor_id))
                    .add(message_entity::Column::Kind.eq("permission_request")),
            )
            .order_by_asc(message_entity::Column::Seq)
            .all(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        models.iter().map(model_to_message).collect()
    }

    async fn pending_reviews(
        &self,
        reviewer_id: &str,
    ) -> Result<Vec<Message>, CoordinationError> {
        let models = message_entity::Entity::find()
            .filter(
                Condition::all()
                    .add(message_entity::Column::ToAgent.eq(reviewer_id))
                    .add(message_entity::Column::Kind.eq("review_request")),
            )
            .order_by_asc(message_entity::Column::Seq)
            .all(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        models.iter().map(model_to_message).collect()
    }

    async fn list_agents(
        &self,
        parent_id: Option<&str>,
    ) -> Result<Vec<(AgentId, AgentStatus)>, CoordinationError> {
        let query = match parent_id {
            None => agent_entity::Entity::find(),
            Some(pid) => agent_entity::Entity::find()
                .filter(agent_entity::Column::ParentId.eq(pid.to_string())),
        };

        let models = query
            .all(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        models
            .iter()
            .map(|m| {
                let status = str_to_status(&m.status)?;
                Ok((m.id.clone(), status))
            })
            .collect()
    }
}

impl DbBackend {
    /// Store a process ID for an agent. Not part of the trait — specific to
    /// process-based agent implementations.
    pub async fn set_pid(
        &self,
        agent_id: &str,
        pid: Option<i32>,
    ) -> Result<(), CoordinationError> {
        let existing = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        let model = existing
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        let mut active: agent_entity::ActiveModel = model.into();
        active.pid = Set(pid);
        active.updated_at = Set(chrono::Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Get the stored PID for an agent.
    pub async fn get_pid(
        &self,
        agent_id: &str,
    ) -> Result<Option<i32>, CoordinationError> {
        let model = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        Ok(model.pid)
    }

    pub async fn get_parent_id(
        &self,
        agent_id: &str,
    ) -> Result<Option<String>, CoordinationError> {
        let model = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        Ok(model.parent_id)
    }

    /// Store a bearer token for an agent.
    pub async fn set_token(
        &self,
        agent_id: &str,
        token: &str,
    ) -> Result<(), CoordinationError> {
        let existing = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        let model = existing
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        let mut active: agent_entity::ActiveModel = model.into();
        active.token = Set(Some(token.to_string()));
        active.updated_at = Set(chrono::Utc::now());

        active
            .update(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Look up the agent_id associated with a bearer token.
    /// Returns None if no agent has that token.
    pub async fn get_agent_id_by_token(
        &self,
        token: &str,
    ) -> Result<Option<String>, CoordinationError> {
        let model = agent_entity::Entity::find()
            .filter(agent_entity::Column::Token.eq(token))
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(model.map(|m| m.id))
    }

    /// Create an agent session.
    pub async fn create_session(
        &self,
        session_id: &str,
        agent_id: &str,
        initial_phase: &str,
    ) -> Result<(), CoordinationError> {
        let now = chrono::Utc::now();
        let model = session_entity::ActiveModel {
            id: Set(session_id.to_string()),
            agent_id: Set(agent_id.to_string()),
            claude_session_id: Set(None),
            phase: Set(initial_phase.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get the phase for an agent. Returns None if no phase_state entry exists.
    pub async fn get_phase(
        &self,
        agent_id: &str,
    ) -> Result<Option<(String, i32, Option<String>)>, CoordinationError> {
        let model = phase_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        Ok(model.map(|m| (m.phase, m.attempts, m.workflow)))
    }

    /// Set the phase and workflow name for an agent.
    pub async fn set_phase(
        &self,
        agent_id: &str,
        phase: &str,
        attempts: i32,
        workflow: Option<&str>,
    ) -> Result<(), CoordinationError> {
        let now = chrono::Utc::now();

        let existing = phase_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        if let Some(model) = existing {
            let mut active: phase_entity::ActiveModel = model.into();
            active.phase = Set(phase.to_string());
            active.attempts = Set(attempts);
            if let Some(wf) = workflow {
                active.workflow = Set(Some(wf.to_string()));
            }
            active.updated_at = Set(now);
            active
                .update(&self.db)
                .await
                .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        } else {
            let model = phase_entity::ActiveModel {
                agent_id: Set(agent_id.to_string()),
                phase: Set(phase.to_string()),
                attempts: Set(attempts),
                workflow: Set(workflow.map(str::to_string)),
                updated_at: Set(now),
            };
            model
                .insert(&self.db)
                .await
                .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Store a permission grant for an agent session.
    pub async fn grant_permission(
        &self,
        session_id: &str,
        agent_id: &str,
        tool_pattern: &str,
        granted_by: &str,
        reason: &str,
    ) -> Result<(), CoordinationError> {
        let now = chrono::Utc::now();
        let id = format!(
            "grant-{}",
            now.timestamp_nanos_opt().unwrap_or(0)
        );
        let model = grant_entity::ActiveModel {
            id: Set(id),
            session_id: Set(session_id.to_string()),
            agent_id: Set(agent_id.to_string()),
            tool_pattern: Set(tool_pattern.to_string()),
            granted_by: Set(granted_by.to_string()),
            reason: Set(reason.to_string()),
            created_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get the `created_at` timestamp for an agent.
    pub async fn get_agent_created_at(
        &self,
        agent_id: &str,
    ) -> Result<chrono::DateTime<chrono::Utc>, CoordinationError> {
        let model = agent_entity::Entity::find_by_id(agent_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?
            .ok_or_else(|| CoordinationError::NotFound(agent_id.to_string()))?;

        Ok(model.created_at)
    }

    /// Check if an agent has a permission grant matching the tool pattern.
    pub async fn check_permission(
        &self,
        agent_id: &str,
        tool_name: &str,
    ) -> Result<bool, CoordinationError> {
        let grants = grant_entity::Entity::find()
            .filter(
                Condition::all()
                    .add(grant_entity::Column::AgentId.eq(agent_id)),
            )
            .all(&self.db)
            .await
            .map_err(|e| CoordinationError::Storage(e.to_string()))?;

        // Check if any granted pattern matches the requested tool.
        Ok(grants.iter().any(|g| tool_matches_pattern(tool_name, &g.tool_pattern)))
    }
}

/// Check if a tool name matches a grant pattern.
/// Patterns support trailing wildcards: "Bash(cargo *)" matches "Bash(cargo test)".
fn tool_matches_pattern(tool_name: &str, pattern: &str) -> bool {
    if pattern == tool_name {
        return true;
    }

    // Handle patterns like "Bash(cargo *)" — wildcard inside parens.
    // Extract the tool prefix and the argument glob.
    if let Some(paren) = pattern.find('(') {
        let pat_tool = &pattern[..paren];
        let inner = &pattern[paren + 1..pattern.len().saturating_sub(1)]; // strip parens

        if let Some(tool_paren) = tool_name.find('(') {
            let name_tool = &tool_name[..tool_paren];
            if pat_tool != name_tool {
                return false;
            }
            let name_inner = &tool_name[tool_paren + 1..tool_name.len().saturating_sub(1)];

            // "cargo *" matches "cargo test", "cargo", "cargo build --release"
            if let Some(prefix) = inner.strip_suffix('*') {
                let prefix = prefix.trim_end();
                return name_inner.starts_with(prefix) || name_inner == prefix;
            }
            return inner == name_inner;
        }
        return false;
    }

    // Simple trailing wildcard (no parens).
    if let Some(prefix) = pattern.strip_suffix('*') {
        return tool_name.starts_with(prefix);
    }

    false
}

/// Convenience alias for the Postgres-backed variant.
pub type PostgresBackend = DbBackend;

/// Convenience alias for the SQLite-backed variant.
pub type SqliteBackend = DbBackend;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::contract_tests;

    async fn sqlite_backend() -> DbBackend {
        let backend = DbBackend::connect("sqlite::memory:")
            .await
            .expect("connect to in-memory SQLite");
        backend
            .create_tables()
            .await
            .expect("create tables in SQLite");
        backend
    }

    macro_rules! db_contract_test {
        ($name:ident) => {
            #[tokio::test]
            async fn $name() {
                let backend = sqlite_backend().await;
                contract_tests::$name(&backend).await;
            }
        };
    }

    db_contract_test!(register_starts_pending);
    db_contract_test!(register_duplicate_errors);
    db_contract_test!(set_and_get_status);
    db_contract_test!(status_lifecycle);
    db_contract_test!(status_not_found);
    db_contract_test!(set_status_not_found);
    db_contract_test!(send_and_recv);
    db_contract_test!(recv_filters_by_recipient);
    db_contract_test!(cursor_tracks_position);
    db_contract_test!(recv_empty);
    db_contract_test!(permission_request_flow);
    db_contract_test!(permission_denied_flow);
    db_contract_test!(review_request_flow);
    db_contract_test!(pending_permissions_filters_kind);
    db_contract_test!(pending_reviews_filters_kind);
    db_contract_test!(list_agents_all);
    db_contract_test!(list_agents_by_parent);
    db_contract_test!(list_agents_parentless_excluded);
    db_contract_test!(status_update_message);
    db_contract_test!(output_message);
    db_contract_test!(message_ordering);
    db_contract_test!(send_returns_id);

    #[tokio::test]
    async fn get_agent_created_at_returns_registration_time() {
        let backend = sqlite_backend().await;
        let before = chrono::Utc::now();
        backend.register_agent("w1", None).await.unwrap();
        let after = chrono::Utc::now();

        let created_at = backend.get_agent_created_at("w1").await.unwrap();
        assert!(created_at >= before);
        assert!(created_at <= after);
    }

    #[tokio::test]
    async fn get_agent_created_at_not_found() {
        let backend = sqlite_backend().await;
        let err = backend.get_agent_created_at("ghost").await;
        assert!(err.is_err());
    }

    /// Postgres contract tests — requires DATABASE_URL env var.
    mod postgres {
        use super::*;

        async fn pg_backend() -> Option<DbBackend> {
            let url = std::env::var("DATABASE_URL").ok()?;
            let backend = DbBackend::connect(&url).await.ok()?;
            backend.create_tables().await.ok()?;

            let db_backend = backend.db.get_database_backend();
            let _ = backend
                .db
                .execute(Statement::from_string(
                    db_backend,
                    "DELETE FROM coordination_messages; DELETE FROM coordination_agents;"
                        .to_string(),
                ))
                .await;

            Some(backend)
        }

        macro_rules! pg_contract_test {
            ($name:ident) => {
                #[tokio::test]
                async fn $name() {
                    let Some(backend) = pg_backend().await else {
                        eprintln!(
                            "Skipping {}: no DATABASE_URL set",
                            stringify!($name)
                        );
                        return;
                    };
                    contract_tests::$name(&backend).await;
                }
            };
        }

        pg_contract_test!(register_starts_pending);
        pg_contract_test!(register_duplicate_errors);
        pg_contract_test!(set_and_get_status);
        pg_contract_test!(status_lifecycle);
        pg_contract_test!(status_not_found);
        pg_contract_test!(set_status_not_found);
        pg_contract_test!(send_and_recv);
        pg_contract_test!(recv_filters_by_recipient);
        pg_contract_test!(cursor_tracks_position);
        pg_contract_test!(recv_empty);
        pg_contract_test!(permission_request_flow);
        pg_contract_test!(permission_denied_flow);
        pg_contract_test!(review_request_flow);
        pg_contract_test!(pending_permissions_filters_kind);
        pg_contract_test!(pending_reviews_filters_kind);
        pg_contract_test!(list_agents_all);
        pg_contract_test!(list_agents_by_parent);
        pg_contract_test!(list_agents_parentless_excluded);
        pg_contract_test!(status_update_message);
        pg_contract_test!(output_message);
        pg_contract_test!(message_ordering);
        pg_contract_test!(send_returns_id);
    }
}
