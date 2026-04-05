//! Review gate commands: request, approve, request-changes.
//!
//! Workers call `clc review request <type>` to request a typed review.
//! Reviewer agents call `clc review approve` or `clc review request-changes`
//! to render a verdict. Verdicts are stored in the coordination database.

use std::path::Path;

use crate::coordination::Coordination;
use crate::error::Error;

fn open_coordination(cwd: &Path) -> Result<Coordination, Error> {
    Coordination::open(cwd)
        .map_err(|e| Error::NonBlocking(format!("coordination: {e}")))
}

fn recv_messages(
    coord: &Coordination,
    agent_id: &str,
) -> Result<Vec<clc_sdk::coordination::Message>, Error> {
    let (msgs, _) = coord
        .recv(agent_id, &clc_sdk::coordination::Cursor::default())
        .map_err(|e| Error::NonBlocking(format!("coordination recv: {e}")))?;
    Ok(msgs)
}

/// Request a review of the given type. Sends a ReviewRequest message to the
/// coordination database.
pub fn request(cwd: &Path, review_type: &str) -> Result<(), Error> {
    let branch = crate::git::current_branch(cwd).unwrap_or_default();
    let coord = open_coordination(cwd)?;

    let msg = clc_sdk::coordination::Message {
        id: format!(
            "review-req-{review_type}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        from: branch.clone(),
        to: "coordinator".into(),
        kind: clc_sdk::coordination::MessageKind::ReviewRequest {
            review_type: review_type.to_string(),
            branch: branch.clone(),
            summary: format!("Review type '{review_type}' requested by worker '{branch}'"),
        },
        timestamp: std::time::SystemTime::now(),
    };
    coord
        .send(msg)
        .map_err(|e| Error::NonBlocking(format!("coordination send: {e}")))?;

    eprintln!("Review requested: type={review_type}, branch={branch}");
    Ok(())
}

/// Approve the current review. Must be called from a reviewer session
/// (CLC_REVIEW_TYPE env var must be set).
pub fn approve(cwd: &Path, comments: &str) -> Result<(), Error> {
    let review_type = std::env::var("CLC_REVIEW_TYPE").map_err(|_| {
        Error::NonBlocking(
            "CLC_REVIEW_TYPE not set — approve can only be called from a reviewer session".into(),
        )
    })?;

    let agent_id = crate::git::current_branch(cwd).unwrap_or_default();
    let reviewer_id = std::env::var("CLC_REVIEWER_ID").unwrap_or_else(|_| {
        format!("{agent_id}-reviewer-{review_type}")
    });

    let coord = open_coordination(cwd)?;

    let msg = clc_sdk::coordination::Message {
        id: format!(
            "review-result-{review_type}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        from: reviewer_id,
        to: agent_id.clone(),
        kind: clc_sdk::coordination::MessageKind::ReviewResult {
            request_id: String::new(), // TODO: link to the specific request
            review_type: review_type.clone(),
            verdict: clc_sdk::coordination::ReviewVerdict::Approved,
            comments: comments.to_string(),
        },
        timestamp: std::time::SystemTime::now(),
    };
    coord
        .send(msg)
        .map_err(|e| Error::NonBlocking(format!("coordination send: {e}")))?;

    eprintln!("Review approved: type={review_type}, worker={agent_id}");
    Ok(())
}

/// Request changes on the current review. Must be called from a reviewer session.
pub fn request_changes(cwd: &Path, comments: &str) -> Result<(), Error> {
    let review_type = std::env::var("CLC_REVIEW_TYPE").map_err(|_| {
        Error::NonBlocking(
            "CLC_REVIEW_TYPE not set — request-changes can only be called from a reviewer session"
                .into(),
        )
    })?;

    let agent_id = crate::git::current_branch(cwd).unwrap_or_default();
    let reviewer_id = std::env::var("CLC_REVIEWER_ID").unwrap_or_else(|_| {
        format!("{agent_id}-reviewer-{review_type}")
    });

    let coord = open_coordination(cwd)?;

    let msg = clc_sdk::coordination::Message {
        id: format!(
            "review-result-{review_type}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        from: reviewer_id,
        to: agent_id.clone(),
        kind: clc_sdk::coordination::MessageKind::ReviewResult {
            request_id: String::new(),
            review_type: review_type.clone(),
            verdict: clc_sdk::coordination::ReviewVerdict::ChangesRequested,
            comments: comments.to_string(),
        },
        timestamp: std::time::SystemTime::now(),
    };
    coord
        .send(msg)
        .map_err(|e| Error::NonBlocking(format!("coordination send: {e}")))?;

    eprintln!("Changes requested: type={review_type}, worker={agent_id}");
    Ok(())
}

/// Check if all required review types for a transition have passing verdicts.
/// Returns Ok(()) if all reviews pass, or Err with details about what's missing.
pub fn check_review_requirements(
    cwd: &Path,
    worker_id: &str,
    required_reviews: &[String],
) -> Result<(), Error> {
    if required_reviews.is_empty() {
        return Ok(());
    }

    let coord = open_coordination(cwd)?;

    // Read all messages addressed to the worker.
    let msgs = recv_messages(&coord, worker_id)?;

    // For each required review type, check if there's an Approved verdict.
    let mut missing = Vec::new();
    for review_type in required_reviews {
        let has_approval = msgs.iter().any(|m| {
            matches!(
                &m.kind,
                clc_sdk::coordination::MessageKind::ReviewResult {
                    review_type: rt,
                    verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                    ..
                } if rt == review_type
            )
        });
        if !has_approval {
            missing.push(review_type.as_str());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::NonBlocking(format!(
            "transition blocked — required reviews not yet approved: {}",
            missing.join(", ")
        )))
    }
}

/// Check if a reviewer session has pending review requests for a given worker.
/// Returns the review types that have been requested but not yet resolved.
pub fn pending_review_types(
    cwd: &Path,
    worker_id: &str,
) -> Result<Vec<String>, Error> {
    let coord = open_coordination(cwd)?;

    // Get all messages from the worker to the coordinator.
    let (msgs, _) = coord
        .recv("coordinator", &clc_sdk::coordination::Cursor::default())
        .map_err(|e| Error::NonBlocking(format!("coordination recv: {e}")))?;

    // Find review requests from this worker.
    let requested: Vec<String> = msgs
        .iter()
        .filter(|m| m.from == worker_id)
        .filter_map(|m| match &m.kind {
            clc_sdk::coordination::MessageKind::ReviewRequest { review_type, .. } => {
                Some(review_type.clone())
            }
            _ => None,
        })
        .collect();

    if requested.is_empty() {
        return Ok(vec![]);
    }

    // Check which ones have been approved (only Approved verdicts count).
    let worker_msgs = recv_messages(&coord, worker_id)?;
    let approved: Vec<String> = worker_msgs
        .iter()
        .filter_map(|m| match &m.kind {
            clc_sdk::coordination::MessageKind::ReviewResult {
                review_type,
                verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                ..
            } => Some(review_type.clone()),
            _ => None,
        })
        .collect();

    // Return types that are requested but not yet approved.
    Ok(requested
        .into_iter()
        .filter(|rt| !approved.contains(rt))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clc_sdk::coordination::CoordinationBackend;

    #[test]
    fn check_review_requirements_passes_when_no_reviews_required() {
        // No coordination DB needed — empty requirements pass immediately.
        let dir = tempfile::tempdir().unwrap();
        // This will fail to open coordination (no DB) but that's fine —
        // the function short-circuits on empty requirements.
        let result = check_review_requirements(dir.path(), "worker", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn check_review_requirements_blocks_when_missing() {
        // Use local SQLite mode — create a temp dir with coordination DB.
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();

        // Open coordination (creates SQLite DB).
        let coord = Coordination::open(dir.path()).unwrap();

        // Register agent.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // Access the inner DB for registration — coordination.rs wraps this.
            let db = clc_sdk::coordination_db::DbBackend::connect(
                &format!("sqlite://{}?mode=rwc", clc_dir.join("coordination.db").display()),
            )
            .await
            .unwrap();
            db.create_tables().await.unwrap();
            db.register_agent("test-worker", None).await.unwrap();
            db.register_agent("test-reviewer", None).await.unwrap();
        });

        // No approval yet — should fail.
        let result = check_review_requirements(dir.path(), "test-worker", &["code".into()]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("code"));

        // Send an approval.
        coord
            .send(clc_sdk::coordination::Message {
                id: "rev-result-1".into(),
                from: "test-reviewer".into(),
                to: "test-worker".into(),
                kind: clc_sdk::coordination::MessageKind::ReviewResult {
                    request_id: "".into(),
                    review_type: "code".into(),
                    verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                    comments: "lgtm".into(),
                },
                timestamp: std::time::SystemTime::now(),
            })
            .unwrap();

        // Now should pass.
        let result = check_review_requirements(dir.path(), "test-worker", &["code".into()]);
        assert!(result.is_ok());
    }

    #[test]
    fn check_review_requirements_changes_requested_does_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();

        let coord = Coordination::open(dir.path()).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let db = clc_sdk::coordination_db::DbBackend::connect(
                &format!("sqlite://{}?mode=rwc", clc_dir.join("coordination.db").display()),
            )
            .await
            .unwrap();
            db.create_tables().await.unwrap();
            db.register_agent("test-worker", None).await.unwrap();
        });

        // Send a ChangesRequested verdict — should NOT satisfy the gate.
        coord
            .send(clc_sdk::coordination::Message {
                id: "rev-changes-1".into(),
                from: "test-reviewer".into(),
                to: "test-worker".into(),
                kind: clc_sdk::coordination::MessageKind::ReviewResult {
                    request_id: "".into(),
                    review_type: "code".into(),
                    verdict: clc_sdk::coordination::ReviewVerdict::ChangesRequested,
                    comments: "needs work".into(),
                },
                timestamp: std::time::SystemTime::now(),
            })
            .unwrap();

        let result = check_review_requirements(dir.path(), "test-worker", &["code".into()]);
        assert!(result.is_err(), "ChangesRequested should not satisfy the review gate");
        assert!(
            result.unwrap_err().to_string().contains("code"),
            "error should mention the missing review type"
        );
    }

    #[test]
    fn check_review_requirements_partial_approval_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();

        let coord = Coordination::open(dir.path()).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let db = clc_sdk::coordination_db::DbBackend::connect(
                &format!("sqlite://{}?mode=rwc", clc_dir.join("coordination.db").display()),
            )
            .await
            .unwrap();
            db.create_tables().await.unwrap();
            db.register_agent("test-worker", None).await.unwrap();
        });

        // Approve "code" but not "security".
        coord
            .send(clc_sdk::coordination::Message {
                id: "rev-code-1".into(),
                from: "code-reviewer".into(),
                to: "test-worker".into(),
                kind: clc_sdk::coordination::MessageKind::ReviewResult {
                    request_id: "".into(),
                    review_type: "code".into(),
                    verdict: clc_sdk::coordination::ReviewVerdict::Approved,
                    comments: "lgtm".into(),
                },
                timestamp: std::time::SystemTime::now(),
            })
            .unwrap();

        // Both "code" and "security" required — only "code" approved.
        let result = check_review_requirements(
            dir.path(),
            "test-worker",
            &["code".into(), "security".into()],
        );
        assert!(result.is_err(), "partial approval should not satisfy the gate");
        assert!(
            result.unwrap_err().to_string().contains("security"),
            "error should mention the missing reviewer"
        );
    }
}
