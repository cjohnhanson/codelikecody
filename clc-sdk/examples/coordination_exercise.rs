//! Integration test binary that exercises the CoordinationBackend trait
//! through a realistic multi-agent workflow. Used by missouri tests.
//!
//! Runs the full coordination lifecycle and writes results to files
//! for missouri state comparison.

use clc_sdk::coordination::{
    AgentStatus, CoordinationBackend, Cursor, MemoryBackend, Message,
    MessageKind, ReviewVerdict,
};
use std::io::Write;
use std::time::SystemTime;

fn msg(id: &str, from: &str, to: &str, kind: MessageKind) -> Message {
    Message {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        kind,
        timestamp: SystemTime::now(),
    }
}

#[tokio::main]
async fn main() {
    let backend = MemoryBackend::default();
    let mut results = Vec::new();

    // Phase 1: Register agents
    backend
        .register_agent("coordinator", None)
        .await
        .unwrap();
    backend
        .register_agent("worker-1", Some("coordinator"))
        .await
        .unwrap();
    backend
        .register_agent("worker-2", Some("coordinator"))
        .await
        .unwrap();

    let agents = backend.list_agents(None).await.unwrap();
    results.push(format!("registered: {}", agents.len()));

    // Verify initial status
    let status = backend.get_status("worker-1").await.unwrap();
    results.push(format!("worker-1 initial: {status:?}"));

    // Duplicate registration fails
    let dup = backend.register_agent("worker-1", None).await;
    results.push(format!("duplicate registration: {}", if dup.is_err() { "rejected" } else { "ERROR: accepted" }));

    // Phase 2: Status lifecycle
    backend
        .set_status("worker-1", AgentStatus::Running)
        .await
        .unwrap();
    let status = backend.get_status("worker-1").await.unwrap();
    results.push(format!("worker-1 running: {status:?}"));

    // Phase 3: Permission request flow
    backend
        .send(msg(
            "perm-req-1",
            "worker-1",
            "coordinator",
            MessageKind::PermissionRequest {
                tool_name: "Bash(git push)".into(),
                reason: "push feature branch".into(),
            },
        ))
        .await
        .unwrap();

    let pending = backend.pending_permissions("coordinator").await.unwrap();
    results.push(format!("pending permissions: {}", pending.len()));

    // Grant the permission
    backend
        .send(msg(
            "perm-grant-1",
            "coordinator",
            "worker-1",
            MessageKind::PermissionGrant {
                request_id: "perm-req-1".into(),
                scope: "Bash(git push *)".into(),
            },
        ))
        .await
        .unwrap();

    // Worker receives the grant
    let (msgs, cursor) = backend.recv("worker-1", &Cursor::default()).await.unwrap();
    let has_grant = msgs
        .iter()
        .any(|m| matches!(&m.kind, MessageKind::PermissionGrant { .. }));
    results.push(format!("worker-1 received grant: {has_grant}"));

    // Phase 4: Worker sends status update
    backend
        .send(msg(
            "status-1",
            "worker-1",
            "coordinator",
            MessageKind::StatusUpdate {
                phase: "implementing".into(),
                detail: "writing tests".into(),
            },
        ))
        .await
        .unwrap();

    // Phase 5: Worker output
    backend
        .send(msg(
            "out-1",
            "worker-1",
            "coordinator",
            MessageKind::Output("test suite passing".into()),
        ))
        .await
        .unwrap();

    // Coordinator receives status + output
    let (coord_msgs, _) = backend
        .recv("coordinator", &Cursor::default())
        .await
        .unwrap();
    let has_status = coord_msgs
        .iter()
        .any(|m| matches!(&m.kind, MessageKind::StatusUpdate { .. }));
    let has_output = coord_msgs
        .iter()
        .any(|m| matches!(&m.kind, MessageKind::Output(_)));
    results.push(format!("coordinator received status: {has_status}"));
    results.push(format!("coordinator received output: {has_output}"));

    // Phase 6: Review request flow
    backend
        .send(msg(
            "review-req-1",
            "worker-1",
            "coordinator",
            MessageKind::ReviewRequest {
                branch: "feat/thing".into(),
                summary: "added the thing".into(),
            },
        ))
        .await
        .unwrap();

    let reviews = backend.pending_reviews("coordinator").await.unwrap();
    results.push(format!("pending reviews: {}", reviews.len()));

    backend
        .send(msg(
            "review-result-1",
            "coordinator",
            "worker-1",
            MessageKind::ReviewResult {
                request_id: "review-req-1".into(),
                verdict: ReviewVerdict::Approved,
                comments: "lgtm".into(),
            },
        ))
        .await
        .unwrap();

    // Worker receives review result (using cursor from earlier recv)
    let (new_msgs, _) = backend.recv("worker-1", &cursor).await.unwrap();
    let has_approval = new_msgs.iter().any(|m| {
        matches!(
            &m.kind,
            MessageKind::ReviewResult { verdict, .. } if *verdict == ReviewVerdict::Approved
        )
    });
    results.push(format!("worker-1 received approval: {has_approval}"));

    // Phase 7: Worker completes
    backend
        .set_status("worker-1", AgentStatus::Completed)
        .await
        .unwrap();

    // Phase 8: Permission denial for worker-2
    backend
        .set_status("worker-2", AgentStatus::Running)
        .await
        .unwrap();
    backend
        .send(msg(
            "perm-req-2",
            "worker-2",
            "coordinator",
            MessageKind::PermissionRequest {
                tool_name: "Bash(rm -rf /)".into(),
                reason: "cleanup".into(),
            },
        ))
        .await
        .unwrap();
    backend
        .send(msg(
            "perm-deny-1",
            "coordinator",
            "worker-2",
            MessageKind::PermissionDenied {
                request_id: "perm-req-2".into(),
                reason: "not allowed".into(),
            },
        ))
        .await
        .unwrap();

    let (w2_msgs, _) = backend
        .recv("worker-2", &Cursor::default())
        .await
        .unwrap();
    let has_denial = w2_msgs
        .iter()
        .any(|m| matches!(&m.kind, MessageKind::PermissionDenied { .. }));
    results.push(format!("worker-2 received denial: {has_denial}"));

    // Phase 9: List agents with parent filter
    let coord_children = backend
        .list_agents(Some("coordinator"))
        .await
        .unwrap();
    results.push(format!("coordinator children: {}", coord_children.len()));

    // Final status check
    let final_statuses: Vec<_> = backend
        .list_agents(None)
        .await
        .unwrap()
        .iter()
        .map(|(id, s)| format!("{id}={s:?}"))
        .collect();
    results.push(format!("final: {}", final_statuses.join(", ")));

    // Phase 10: Not-found errors
    let ghost_status = backend.get_status("ghost").await;
    results.push(format!(
        "ghost agent: {}",
        if ghost_status.is_err() { "not found" } else { "ERROR: found" }
    ));

    let ghost_set = backend
        .set_status("ghost", AgentStatus::Failed)
        .await;
    results.push(format!(
        "set ghost status: {}",
        if ghost_set.is_err() { "not found" } else { "ERROR: accepted" }
    ));

    // Write results
    let output = results.join("\n") + "\n";
    print!("{output}");

    let mut f = std::fs::File::create("coordination-results.txt").unwrap();
    f.write_all(output.as_bytes()).unwrap();
}
