use std::fmt::Write;
use std::path::Path;

use camino::Utf8Path;

use crate::error::Error;

/// Summary of belmont state for this project.
#[derive(Debug)]
pub struct BelmontState {
    pub initialized: bool,
    pub secret_count: usize,
    pub available_count: usize,
    pub missing: Vec<String>,
    pub secret_names: Vec<String>,
}

/// Detect belmont state for the given project directory.
pub fn detect(project_dir: &Path) -> Result<BelmontState, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let config = match belmont::BelmontConfig::load(utf8_dir) {
        Ok(c) => c,
        Err(belmont::Error::NotInitialized) => {
            return Ok(BelmontState {
                initialized: false,
                secret_count: 0,
                available_count: 0,
                missing: vec![],
                secret_names: vec![],
            });
        }
        Err(e) => {
            return Err(Error::NonBlocking(format!("belmont error: {e}")));
        }
    };

    let registry = belmont::SecretRegistry::resolve(&config);
    let secret_names: Vec<String> = registry.names().iter().map(|s| (*s).to_string()).collect();
    let missing: Vec<String> = registry.missing().iter().map(|s| (*s).to_string()).collect();
    let available_count = secret_names.len() - missing.len();

    Ok(BelmontState {
        initialized: true,
        secret_count: secret_names.len(),
        available_count,
        missing,
        secret_names,
    })
}

impl clc_sdk::ClcTool for BelmontState {
    fn prime(&self, _ctx: &clc_sdk::PrimeContext) -> String {
        let mut out = String::new();
        out.push_str("## Belmont (secrets)\n\n");

        if !self.initialized {
            out.push_str("Belmont is not initialized in this project.\n");
            return out;
        }

        if self.secret_count == 0 {
            out.push_str("Belmont is initialized but no secrets are declared.\n");
            return out;
        }

        out.push_str("Available secrets:\n");
        for name in &self.secret_names {
            if self.missing.contains(name) {
                let _ = writeln!(out, "  - `belmont://{name}` (MISSING)");
            } else {
                let _ = writeln!(out, "  - `belmont://{name}`");
            }
        }
        out.push('\n');

        let _ = writeln!(
            out,
            "{}/{} secrets available.",
            self.available_count, self.secret_count
        );

        if !self.missing.is_empty() {
            out.push_str("\nRun `belmont check` to see resolution errors.\n");
        }

        out.push_str(
            "\n### Rules\n\n\
             - **Never ask for secret values.** Secrets are managed by belmont, not by humans \
             pasting values into chat.\n\
             - **Never read files that contain secret values.** If a command writes secrets to \
             a file, do not read that file.\n\
             - **Always use `belmont run -- <command>`** for any command that needs secrets. \
             Belmont injects values as environment variables and scrubs them from output.\n\
             - In shell commands, reference secrets as `$SECRET_NAME` (belmont injects via env).\n\
             - In scrubbed output, secret values appear as `belmont://SECRET_NAME`.\n",
        );

        out
    }

    fn status_basic(&self) -> String {
        if !self.initialized {
            return String::new();
        }
        if self.secret_count == 0 {
            return "belmont: no secrets declared".to_string();
        }
        format!(
            "belmont: {}/{} secrets available",
            self.available_count, self.secret_count
        )
    }

    fn status_full(&self) -> String {
        if !self.initialized {
            return "belmont: not initialized".to_string();
        }
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# belmont\n\n{}/{} secrets available\n",
            self.available_count, self.secret_count
        );
        for name in &self.secret_names {
            if self.missing.contains(name) {
                let _ = writeln!(out, "  MISSING  {name}");
            } else {
                let _ = writeln!(out, "  ok       {name}");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use clc_sdk::ClcTool;

    use super::*;

    #[test]
    fn not_initialized_state() {
        let state = BelmontState {
            initialized: false,
            secret_count: 0,
            available_count: 0,
            missing: vec![],
            secret_names: vec![],
        };
        assert!(state.prime(&clc_sdk::PrimeContext { phase: None }).contains("not initialized"));
        assert!(state.status_basic().is_empty());
    }

    #[test]
    fn prime_lists_secrets() {
        let state = BelmontState {
            initialized: true,
            secret_count: 2,
            available_count: 2,
            missing: vec![],
            secret_names: vec!["DB_URL".to_string(), "API_KEY".to_string()],
        };
        let prime = state.prime(&clc_sdk::PrimeContext { phase: None });
        assert!(prime.contains("belmont://DB_URL"));
        assert!(prime.contains("belmont://API_KEY"));
        assert!(prime.contains("2/2 secrets available"));
    }

    #[test]
    fn prime_marks_missing() {
        let state = BelmontState {
            initialized: true,
            secret_count: 2,
            available_count: 1,
            missing: vec!["GONE".to_string()],
            secret_names: vec!["OK".to_string(), "GONE".to_string()],
        };
        let prime = state.prime(&clc_sdk::PrimeContext { phase: None });
        assert!(prime.contains("MISSING"));
        assert!(prime.contains("1/2 secrets available"));
    }

    #[test]
    fn status_basic_format() {
        let state = BelmontState {
            initialized: true,
            secret_count: 3,
            available_count: 3,
            missing: vec![],
            secret_names: vec!["A".into(), "B".into(), "C".into()],
        };
        assert_eq!(state.status_basic(), "belmont: 3/3 secrets available");
    }

    #[test]
    fn prime_includes_rules() {
        let state = BelmontState {
            initialized: true,
            secret_count: 1,
            available_count: 1,
            missing: vec![],
            secret_names: vec!["TOKEN".to_string()],
        };
        let prime = state.prime(&clc_sdk::PrimeContext { phase: None });
        assert!(prime.contains("belmont run"));
        assert!(prime.contains("Never ask for secret values"));
    }

    #[test]
    fn status_full_shows_each_secret() {
        let state = BelmontState {
            initialized: true,
            secret_count: 2,
            available_count: 1,
            missing: vec!["GONE".to_string()],
            secret_names: vec!["OK".to_string(), "GONE".to_string()],
        };
        let full = state.status_full();
        assert!(full.contains("ok       OK"));
        assert!(full.contains("MISSING  GONE"));
    }
}
