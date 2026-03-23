---
title: "CoordinationBackend trait with Postgres implementation via SeaORM"
status: done
priority: 2
assignee:
labels: [clc, architecture]
depends_on: [am9k-agent-trait-extract-claude-specific-code-from-workspace-into-agent-abstraction]
created: 2026-03-22T22:02:06Z
updated: "2026-03-23T00:08:12Z"
---

Replace filesystem communication (stdout.jsonl, pid files, stdin pipes, outbox) with a CoordinationBackend trait. First impl: Postgres via SeaORM.

The backend is the communication bus for the entire system. Message types: PermissionRequest, PermissionGrant, ReviewRequest, ReviewResult, StatusUpdate, AgentOutput, AgentInput.

Every interaction between agents goes through the backend: worker→coordinator permission requests, coordinator→admin escalations, review workflows, status updates. Database makes everything queryable, durable, and observable.

clc-api becomes the API layer over the backend. clc-web becomes the UI. Permission requests that escalate to humans show up in the UI.

Depends on: am9k (Agent trait)

## Scratch Notes

### Implementation (2026-03-22)

**Trait**: `clc-sdk/src/coordination.rs` — async trait via `async-trait` crate.
Object-safe (usable as `dyn CoordinationBackend`). 8 methods covering agent
registration, status lifecycle, message send/recv with cursor, pending
permissions/reviews queries, and agent listing with parent filter.

**MemoryBackend**: In-memory implementation for testing. Lives alongside the
trait in coordination.rs. Enforces duplicate registration errors.

**PostgresBackend**: `clc-sdk/src/coordination_pg.rs`, behind `postgres`
feature flag. SeaORM entities for `coordination_agents` and
`coordination_messages` tables. Auto-creates tables via `create_tables()`.
Messages stored with kind discriminator + JSON payload. Cursor-based recv
uses BIGSERIAL `seq` column with index on `(to_agent, seq)`.

**Contract tests**: 22 test functions in `coordination::contract_tests` module,
reusable by any backend. Both MemoryBackend and PostgresBackend run the full
suite. Pg tests skip gracefully without `DATABASE_URL`.

**Verified**: Ran all 22 contract tests against a real Postgres 17.9 instance.
All pass.

**Not yet done**:
- Wire into clc CLI commands (replacing filesystem reads/writes)
- Wire into clc-api as the API layer
- Migration system (currently uses CREATE TABLE IF NOT EXISTS)
- Connection pooling / configuration
- Message retention / cleanup
