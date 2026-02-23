use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::error::Error;

const STATE_FILENAME: &str = "state";

/// Ordered workflow phases. The sequence is fixed and forward-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    TestsUnwritten,
    TestsWritten,
    Red,
    Implementing,
    Green,
}

impl Phase {
    const ALL: &[Self] = &[
        Self::TestsUnwritten,
        Self::TestsWritten,
        Self::Red,
        Self::Implementing,
        Self::Green,
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
            _ => Err(Error::NonBlocking(format!("unknown phase: {s}"))),
        }
    }
}

/// Load the current phase from `.clc/state`, if it exists.
pub fn load(project_dir: &Path) -> Result<Option<Phase>, Error> {
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

    // Parse "phase: <name>\n"
    let phase_str = contents
        .lines()
        .find_map(|line| line.strip_prefix("phase:").map(str::trim))
        .ok_or_else(|| {
            Error::NonBlocking(format!(
                "state file {} missing phase field",
                state_path.display()
            ))
        })?;

    phase_str.parse().map(Some)
}

/// Validate and perform a phase transition, writing the new state file.
pub fn set(project_dir: &Path, target: &str) -> Result<(), Error> {
    let target_phase: Phase = target.parse()?;
    let current = load(project_dir)?;

    match current {
        None => {
            // No state file — only the first phase is valid.
            if target_phase != Phase::TestsUnwritten {
                return Err(Error::NonBlocking(format!(
                    "cannot set phase to '{target}': no current phase, must start with 'tests-unwritten'"
                )));
            }
        }
        Some(current_phase) => {
            let expected_next = current_phase.next().ok_or_else(|| {
                Error::NonBlocking(format!(
                    "cannot advance from '{current_phase}': already at terminal phase"
                ))
            })?;

            if target_phase != expected_next {
                return Err(Error::NonBlocking(format!(
                    "cannot transition from '{current_phase}' to '{target}': next valid phase is '{expected_next}'"
                )));
            }
        }
    }

    // Write the state file.
    let state_path = project_dir.join(".clc").join(STATE_FILENAME);
    let content = format!("phase: {target_phase}\n");
    std::fs::write(&state_path, content).map_err(|e| {
        Error::NonBlocking(format!(
            "failed to write state {}: {e}",
            state_path.display()
        ))
    })?;

    Ok(())
}
