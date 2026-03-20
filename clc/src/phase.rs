use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::error::Error;

const STATE_FILENAME: &str = "state";

/// Ordered workflow phases. Forward one step, backwards to any earlier phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    TestsUnwritten,
    TestsWritten,
    Red,
    Implementing,
    Green,
    ReviewRequested,
    InReview,
    Reviewed,
    Done,
}

impl Phase {
    const ALL: &[Self] = &[
        Self::TestsUnwritten,
        Self::TestsWritten,
        Self::Red,
        Self::Implementing,
        Self::Green,
        Self::ReviewRequested,
        Self::InReview,
        Self::Reviewed,
        Self::Done,
    ];

    fn ordinal(self) -> usize {
        Self::ALL.iter().position(|&p| p == self).unwrap()
    }

    /// Return the only valid next phase, if one exists.
    pub fn next(self) -> Option<Self> {
        let idx = self.ordinal();
        Self::ALL.get(idx + 1).copied()
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::TestsUnwritten => "tests-unwritten",
            Self::TestsWritten => "tests-written",
            Self::Red => "red",
            Self::Implementing => "implementing",
            Self::Green => "green",
            Self::ReviewRequested => "review-requested",
            Self::InReview => "in-review",
            Self::Reviewed => "reviewed",
            Self::Done => "done",
        };
        f.write_str(s)
    }
}

impl FromStr for Phase {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tests-unwritten" => Ok(Self::TestsUnwritten),
            "tests-written" => Ok(Self::TestsWritten),
            "red" => Ok(Self::Red),
            "implementing" => Ok(Self::Implementing),
            "green" => Ok(Self::Green),
            "review-requested" => Ok(Self::ReviewRequested),
            "in-review" => Ok(Self::InReview),
            "reviewed" => Ok(Self::Reviewed),
            "done" => Ok(Self::Done),
            _ => Err(Error::NonBlocking(format!("unknown phase: {s}"))),
        }
    }
}

/// Load the current phase from `.clc/state`, if it exists.
pub fn load(project_dir: &Path) -> Result<Option<Phase>, Error> {
    let state = load_state(project_dir)?;
    Ok(state.map(|s| s.phase))
}

/// Load the current attempts count from `.clc/state`.
pub fn load_attempts(project_dir: &Path) -> Result<u32, Error> {
    let state = load_state(project_dir)?;
    Ok(state.map_or(0, |s| s.attempts))
}

struct State {
    phase: Phase,
    attempts: u32,
}

fn load_state(project_dir: &Path) -> Result<Option<State>, Error> {
    let state_path = project_dir.join(".clc").join(STATE_FILENAME);

    if !state_path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&state_path).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to read state {}: {e}",
            state_path.display()
        ))
    })?;

    let phase_str = contents
        .lines()
        .find_map(|line| line.strip_prefix("phase:").map(str::trim))
        .ok_or_else(|| {
            Error::NonBlocking(format!(
                "state file {} missing phase field",
                state_path.display()
            ))
        })?;

    let attempts = contents
        .lines()
        .find_map(|line| line.strip_prefix("attempts:").map(str::trim))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let phase = phase_str.parse()?;
    Ok(Some(State { phase, attempts }))
}

/// Initialize the phase state for a freshly created worktree.
/// Unlike `set`, this bypasses sequential-transition validation — it is
/// only for use during `clc pickup` when the worktree has no prior state.
pub fn init_phase(project_dir: &Path, target: &str) -> Result<(), Error> {
    let target_phase: Phase = target.parse()?;
    write_state(project_dir, target_phase, 0)
}

/// Validate and perform a phase transition, writing the new state file.
/// Forward transitions are gated by `required_attempts`.
pub fn set(project_dir: &Path, target: &str, required_attempts: u32) -> Result<(), Error> {
    let target_phase: Phase = target.parse()?;
    let current_state = load_state(project_dir)?;

    let is_forward = match &current_state {
        None => {
            if target_phase != Phase::TestsUnwritten {
                return Err(Error::NonBlocking(format!(
                    "cannot set phase to '{target}': no current phase, must start with 'tests-unwritten'"
                )));
            }
            true
        }
        Some(state) => {
            let current_ord = state.phase.ordinal();
            let target_ord = target_phase.ordinal();

            if target_ord == current_ord {
                return Err(Error::NonBlocking(format!(
                    "already at phase '{}'",
                    state.phase
                )));
            }

            if target_ord > current_ord + 1 {
                let expected_next = state.phase.next().expect("checked above");
                return Err(Error::NonBlocking(format!(
                    "cannot skip from '{}' to '{target}': next forward phase is '{expected_next}'",
                    state.phase
                )));
            }

            target_ord > current_ord
        }
    };

    // Attempt gating: only applies to forward transitions from an existing phase.
    if is_forward && required_attempts > 1 && current_state.is_some() {
        let current_attempts = current_state.as_ref().map_or(0, |s| s.attempts);
        let next_attempt = current_attempts + 1;

        if next_attempt < required_attempts {
            // Not enough attempts yet — increment and reject.
            let current_phase = current_state
                .as_ref()
                .map_or(Phase::TestsUnwritten, |s| s.phase);
            write_state(project_dir, current_phase, next_attempt)?;
            return Err(Error::NonBlocking(format!(
                "attempt {next_attempt}/{required_attempts} to advance to '{target}': \
                 reconsider before trying again"
            )));
        }
    }

    // Transition succeeds — write new phase with attempts reset.
    write_state(project_dir, target_phase, 0)
}

fn write_state(project_dir: &Path, phase: Phase, attempts: u32) -> Result<(), Error> {
    use std::fmt::Write;

    let clc_dir = project_dir.join(".clc");
    std::fs::create_dir_all(&clc_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to create .clc dir: {e}")))?;
    let state_path = clc_dir.join(STATE_FILENAME);

    // Preserve non-phase/non-attempts lines (e.g., "untracked: true").
    let existing = if state_path.exists() {
        std::fs::read_to_string(&state_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut content = String::new();
    let _ = writeln!(content, "phase: {phase}");
    if attempts > 0 {
        let _ = writeln!(content, "attempts: {attempts}");
    }

    // Carry forward lines that aren't phase or attempts.
    for line in existing.lines() {
        if !line.starts_with("phase:") && !line.starts_with("attempts:") && !line.is_empty() {
            content.push_str(line);
            content.push('\n');
        }
    }

    std::fs::write(&state_path, content).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to write state {}: {e}",
            state_path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Parsing new review phases ---

    #[test]
    fn parse_review_requested() {
        let phase: Phase = "review-requested".parse().unwrap();
        assert_eq!(phase, Phase::ReviewRequested);
    }

    #[test]
    fn parse_in_review() {
        let phase: Phase = "in-review".parse().unwrap();
        assert_eq!(phase, Phase::InReview);
    }

    #[test]
    fn parse_reviewed() {
        let phase: Phase = "reviewed".parse().unwrap();
        assert_eq!(phase, Phase::Reviewed);
    }

    // --- Display for new review phases ---

    #[test]
    fn display_review_requested() {
        assert_eq!(Phase::ReviewRequested.to_string(), "review-requested");
    }

    #[test]
    fn display_in_review() {
        assert_eq!(Phase::InReview.to_string(), "in-review");
    }

    #[test]
    fn display_reviewed() {
        assert_eq!(Phase::Reviewed.to_string(), "reviewed");
    }

    // --- Ordering: new phases sit between green and done ---

    #[test]
    fn review_requested_follows_green() {
        assert_eq!(Phase::Green.next(), Some(Phase::ReviewRequested));
    }

    #[test]
    fn in_review_follows_review_requested() {
        assert_eq!(Phase::ReviewRequested.next(), Some(Phase::InReview));
    }

    #[test]
    fn reviewed_follows_in_review() {
        assert_eq!(Phase::InReview.next(), Some(Phase::Reviewed));
    }

    #[test]
    fn done_follows_reviewed() {
        assert_eq!(Phase::Reviewed.next(), Some(Phase::Done));
    }

    #[test]
    fn done_has_no_next() {
        assert_eq!(Phase::Done.next(), None);
    }

    #[test]
    fn ordinal_ordering_is_correct() {
        assert!(Phase::Green.ordinal() < Phase::ReviewRequested.ordinal());
        assert!(Phase::ReviewRequested.ordinal() < Phase::InReview.ordinal());
        assert!(Phase::InReview.ordinal() < Phase::Reviewed.ordinal());
        assert!(Phase::Reviewed.ordinal() < Phase::Done.ordinal());
    }

    // --- Phase transitions via set() ---

    #[test]
    fn set_green_to_review_requested_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: green\n").unwrap();

        set(dir.path(), "review-requested", 1).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, Some(Phase::ReviewRequested));
    }

    #[test]
    fn set_review_requested_to_in_review_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: review-requested\n").unwrap();

        set(dir.path(), "in-review", 1).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, Some(Phase::InReview));
    }

    #[test]
    fn set_in_review_to_reviewed_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: in-review\n").unwrap();

        set(dir.path(), "reviewed", 1).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, Some(Phase::Reviewed));
    }

    #[test]
    fn set_reviewed_to_done_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: reviewed\n").unwrap();

        set(dir.path(), "done", 1).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, Some(Phase::Done));
    }

    #[test]
    fn set_green_to_done_rejects_skipping() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: green\n").unwrap();

        let result = set(dir.path(), "done", 1);
        assert!(result.is_err());
    }

    #[test]
    fn set_green_to_in_review_rejects_skipping() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: green\n").unwrap();

        let result = set(dir.path(), "in-review", 1);
        assert!(result.is_err());
    }

    #[test]
    fn backward_reviewed_to_implementing_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let clc_dir = dir.path().join(".clc");
        std::fs::create_dir_all(&clc_dir).unwrap();
        std::fs::write(clc_dir.join("state"), "phase: reviewed\n").unwrap();

        set(dir.path(), "implementing", 1).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, Some(Phase::Implementing));
    }

    // --- Roundtrip: all phases ---

    #[test]
    fn all_phases_roundtrip_through_display_and_parse() {
        for &phase in Phase::ALL {
            let s = phase.to_string();
            let parsed: Phase = s.parse().unwrap();
            assert_eq!(parsed, phase, "roundtrip failed for {s}");
        }
    }
}
