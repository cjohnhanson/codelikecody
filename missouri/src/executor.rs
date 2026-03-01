use std::process::Command;
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use tempfile::TempDir;

use crate::compare::{self, ComparisonResult, OutputDiff};
use crate::error;
use crate::graph::{Assertion, SandboxConfig, StateGraph, StateId, Transition};
use crate::paths::TestPath;

/// Sandbox configuration for transition execution.
#[derive(Debug, Clone)]
pub enum Sandbox {
    /// No sandbox — env_clear + manual PATH construction.
    None,
    /// Nix shell sandbox: commands run inside `nix shell nixpkgs#pkg1 ... --command`.
    Nix {
        /// Absolute path to the `nix` binary.
        nix_bin: Utf8PathBuf,
        /// Package names to provide via nixpkgs.
        packages: Vec<String>,
    },
}

/// Detect and prepare sandbox from project-level config.
///
/// Reads `graph.sandbox_config` to determine the sandbox mode:
/// - `SandboxConfig::None` → `Sandbox::None`
/// - `SandboxConfig::Packages(pkgs)` → `Sandbox::Nix` (or `Sandbox::None` if preinstalled)
///
/// When `MISSOURI_SANDBOX=preinstalled` is set, packages config resolves to
/// `Sandbox::None` — tools are assumed to already be on PATH (e.g., inside a
/// nix derivation where packages are `nativeCheckInputs`).
pub fn detect_sandbox(graph: &StateGraph) -> error::Result<Sandbox> {
    // Check for preinstalled override
    if std::env::var("MISSOURI_SANDBOX").ok().as_deref() == Some("preinstalled") {
        return Ok(Sandbox::None);
    }

    match &graph.sandbox_config {
        SandboxConfig::None => Ok(Sandbox::None),
        SandboxConfig::Packages(packages) => {
            let nix_bin = which_nix().ok_or_else(|| error::Error::NixNotFound {
                root: graph.root.clone(),
            })?;
            Ok(Sandbox::Nix {
                nix_bin,
                packages: packages.clone(),
            })
        }
    }
}

/// Resolve the absolute path to `nix` from the current process's PATH.
fn which_nix() -> Option<Utf8PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(':') {
        let candidate = Utf8PathBuf::from(dir).join("nix");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    std::option::Option::None
}

/// Build the PATH env var: state bin/ → project bin/ → base path.
fn build_path_env(
    state_bin: Option<&Utf8Path>,
    project_bin: Option<&Utf8Path>,
    base_path: &str,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(sb) = state_bin {
        parts.push(sb.as_str());
    }
    if let Some(pb) = project_bin {
        parts.push(pb.as_str());
    }
    parts.push(base_path);
    parts.join(":")
}

/// How assertions interact with the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode {
    /// Run transitions + filesystem comparison + output assertions + state assertions.
    Full,
    /// Run only state assertions (no transitions, no filesystem comparison).
    CheckOnly,
    /// Run transitions + filesystem comparison, skip all assertions.
    NoCheck,
}

/// Result of running a single state assertion.
#[derive(Debug)]
pub struct AssertionResult {
    pub name: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout_diff: Option<(String, String)>,
    pub stderr_diff: Option<(String, String)>,
    pub error: Option<String>,
}

/// Result of executing a single transition.
#[derive(Debug)]
pub struct StepResult {
    pub transition_name: String,
    pub source_name: String,
    pub target_name: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub comparison: Option<ComparisonResult>,
    pub output_diffs: Vec<OutputDiff>,
    pub assertion_results: Vec<AssertionResult>,
    pub passed: bool,
    pub duration: Duration,
}

/// Result of executing a full test path.
#[derive(Debug)]
pub struct PathResult {
    pub path_display: String,
    pub steps: Vec<StepResult>,
    pub passed: bool,
    pub duration: Duration,
}

/// Configuration for recording transition output.
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Base output directory for recordings (e.g. `<root>/<config_dir>/runs/<run_id>/`).
    pub output_dir: Utf8PathBuf,
    /// The run ID.
    pub run_id: String,
}

/// Options for test execution.
pub struct RunOptions {
    pub keep_temp: bool,
    pub verbose: bool,
    pub sandbox: Sandbox,
    pub check_mode: CheckMode,
    /// If set, record transition output to .cast files.
    pub recording: Option<RecordingConfig>,
}

/// Result of running a single setup command.
#[derive(Debug)]
pub struct SetupResult {
    pub name: String,
    pub passed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Run setup commands before test paths. Returns results and whether all passed.
pub fn run_setup_phase(graph: &StateGraph, opts: &RunOptions) -> Vec<SetupResult> {
    let base_path = std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into());
    let path_env = build_path_env(None, graph.project_bin.as_deref(), &base_path);

    graph
        .setup
        .iter()
        .scan(true, |still_passing, cmd| {
            if !*still_passing || crate::signal::is_interrupted() {
                return None; // stop after first failure or interruption
            }
            let result = run_single_setup(
                cmd,
                &graph.project_root,
                &path_env,
                &graph.project_env,
                &opts.sandbox,
            );
            if !result.passed {
                *still_passing = false;
            }
            Some(result)
        })
        .collect()
}

/// Run a single setup command.
fn run_single_setup(
    cmd: &crate::graph::SetupCommand,
    work_dir: &Utf8Path,
    path_env: &str,
    project_env: &std::collections::BTreeMap<String, String>,
    sandbox: &Sandbox,
) -> SetupResult {
    let output = if cmd.shell {
        match sandbox {
            Sandbox::None => crate::signal::run_tracked(
                Command::new("sh")
                    .arg("-c")
                    .arg(&cmd.command)
                    .current_dir(work_dir.as_std_path())
                    .env_clear()
                    .envs(project_env.iter())
                    .env("PATH", path_env),
            ),
            Sandbox::Nix { nix_bin, packages } => {
                let mut args: Vec<String> = vec!["shell".into()];
                args.push("--extra-experimental-features".into());
                args.push("nix-command flakes".into());
                for pkg in packages {
                    args.push(format!("nixpkgs#{pkg}"));
                }
                args.extend([
                    "--command".into(),
                    "sh".into(),
                    "-c".into(),
                    cmd.command.clone(),
                ]);
                crate::signal::run_tracked(
                    Command::new(nix_bin.as_str())
                        .args(&args)
                        .current_dir(work_dir.as_std_path())
                        .env_clear()
                        .envs(project_env.iter())
                        .env("PATH", path_env),
                )
            }
        }
    } else {
        let parts: Vec<&str> = cmd.command.split_whitespace().collect();
        if parts.is_empty() {
            return SetupResult {
                name: cmd.name.clone(),
                passed: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "empty command".into(),
            };
        }
        match sandbox {
            Sandbox::None => crate::signal::run_tracked(
                Command::new(parts[0])
                    .args(&parts[1..])
                    .current_dir(work_dir.as_std_path())
                    .env_clear()
                    .envs(project_env.iter())
                    .env("PATH", path_env),
            ),
            Sandbox::Nix { nix_bin, packages } => {
                let mut args: Vec<String> = vec!["shell".into()];
                args.push("--extra-experimental-features".into());
                args.push("nix-command flakes".into());
                for pkg in packages {
                    args.push(format!("nixpkgs#{pkg}"));
                }
                args.push("--command".into());
                for p in parts {
                    args.push(p.to_string());
                }
                crate::signal::run_tracked(
                    Command::new(nix_bin.as_str())
                        .args(&args)
                        .current_dir(work_dir.as_std_path())
                        .env_clear()
                        .envs(project_env.iter())
                        .env("PATH", path_env),
                )
            }
        }
    };

    match output {
        Ok(o) => {
            let exit_code = o.status.code();
            SetupResult {
                name: cmd.name.clone(),
                passed: o.status.success(),
                exit_code,
                stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
            }
        }
        Err(e) => SetupResult {
            name: cmd.name.clone(),
            passed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("failed to execute command: {e}"),
        },
    }
}

/// Progress events emitted during test execution.
pub enum ProgressEvent<'a> {
    /// A path is about to start executing.
    PathStarted {
        index: usize,
        total: usize,
        display: &'a str,
    },
    /// A path finished executing.
    PathFinished { index: usize, passed: bool },
    /// Execution was interrupted by a signal.
    Interrupted,
}

/// Execute all test paths in parallel and return results.
pub fn run_all_paths(
    graph: &StateGraph,
    paths: &[TestPath],
    opts: &RunOptions,
    on_progress: Option<&(dyn Fn(ProgressEvent) + Sync)>,
) -> Vec<PathResult> {
    let total = paths.len();

    let results: Vec<PathResult> = paths
        .par_iter()
        .enumerate()
        .map(|(path_idx, path)| {
            if crate::signal::is_interrupted() {
                return PathResult {
                    path_display: path.display(graph),
                    steps: Vec::new(),
                    passed: false,
                    duration: Duration::ZERO,
                };
            }

            let display = path.display(graph);
            if let Some(cb) = on_progress {
                cb(ProgressEvent::PathStarted {
                    index: path_idx,
                    total,
                    display: &display,
                });
            }
            let result = run_path(graph, path, opts, path_idx);
            if let Some(cb) = on_progress {
                cb(ProgressEvent::PathFinished {
                    index: path_idx,
                    passed: result.passed,
                });
            }
            result
        })
        .collect();

    if crate::signal::is_interrupted()
        && let Some(cb) = on_progress
    {
        cb(ProgressEvent::Interrupted);
    }

    results
}

/// Execute a single test path.
fn run_path(graph: &StateGraph, path: &TestPath, opts: &RunOptions, path_idx: usize) -> PathResult {
    let path_display = path.display(graph);
    let start = Instant::now();

    let mut result = match opts.check_mode {
        CheckMode::CheckOnly => run_path_check_only(graph, path, path_display, opts),
        CheckMode::Full | CheckMode::NoCheck => {
            run_path_transitions(graph, path, path_display, opts, path_idx)
        }
    };
    result.duration = start.elapsed();
    result
}

/// CheckOnly mode: iterate states in path order, run assertions on each.
fn run_path_check_only(
    graph: &StateGraph,
    path: &TestPath,
    path_display: String,
    opts: &RunOptions,
) -> PathResult {
    let mut steps = Vec::new();
    let mut passed = true;

    // Collect states in path order (source of first, then targets)
    let mut state_ids: Vec<StateId> = Vec::new();
    if let Some(&first_ti) = path.steps.first() {
        state_ids.push(graph.transitions[first_ti].source);
    }
    for &ti in &path.steps {
        state_ids.push(graph.transitions[ti].target);
    }

    for (i, &state_id) in state_ids.iter().enumerate() {
        if crate::signal::is_interrupted() {
            passed = false;
            break;
        }
        let state = &graph.states[state_id.0];
        let assertions = graph.assertions_for(state_id);
        if assertions.is_empty() {
            continue;
        }

        let step_start = Instant::now();

        // Copy state to temp dir to run assertions
        let (temp_dir, work_dir) = match copy_state_to_temp(state_id, graph) {
            Ok(pair) => pair,
            Err(e) => {
                steps.push(StepResult {
                    transition_name: format!("assertions on {}", state.name),
                    source_name: state.name.clone(),
                    target_name: state.name.clone(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e,
                    comparison: None,
                    output_diffs: Vec::new(),
                    assertion_results: Vec::new(),
                    passed: false,
                    duration: step_start.elapsed(),
                });
                passed = false;
                break;
            }
        };

        let assertion_results =
            run_assertions(&assertions, &work_dir, &state.env, graph, &opts.sandbox);
        let assertions_passed = assertion_results.iter().all(|a| a.passed);
        if !assertions_passed {
            passed = false;
        }

        // Determine a label — use transition name if available, else state name
        let label = if i > 0 {
            let ti = path.steps[i - 1];
            graph.transitions[ti].name.clone()
        } else {
            format!("(root) {}", state.name)
        };

        steps.push(StepResult {
            transition_name: label,
            source_name: state.name.clone(),
            target_name: state.name.clone(),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            comparison: None,
            output_diffs: Vec::new(),
            assertion_results,
            passed: assertions_passed,
            duration: step_start.elapsed(),
        });

        if !opts.keep_temp {
            drop(temp_dir);
        }

        if !assertions_passed {
            break;
        }
    }

    PathResult {
        path_display,
        steps,
        passed,
        duration: Duration::ZERO,
    }
}

/// Full and NoCheck modes: execute transitions, compare filesystem, optionally run assertions.
fn run_path_transitions(
    graph: &StateGraph,
    path: &TestPath,
    path_display: String,
    opts: &RunOptions,
    path_idx: usize,
) -> PathResult {
    let mut steps = Vec::new();
    let mut passed = true;
    let run_assertions_flag = opts.check_mode == CheckMode::Full;

    // For chained paths (A → B → C), the output of one transition
    // becomes the input for the next. Start with the first state.
    let mut current_dir: Option<(TempDir, Utf8PathBuf)> = None;

    for (step_idx, &transition_idx) in path.steps.iter().enumerate() {
        if crate::signal::is_interrupted() {
            passed = false;
            break;
        }
        let transition = &graph.transitions[transition_idx];
        let source = &graph.states[transition.source.0];
        let target = &graph.states[transition.target.0];

        // Determine the working directory for this step.
        // First step: copy the source state to a temp dir.
        // Subsequent steps: use the temp dir from the previous step.
        let (temp_dir, work_dir) = if step_idx == 0 {
            match copy_state_to_temp(source.id, graph) {
                Ok(pair) => pair,
                Err(e) => {
                    steps.push(StepResult {
                        transition_name: transition.name.clone(),
                        source_name: source.name.clone(),
                        target_name: target.name.clone(),
                        exit_code: None,
                        stdout: String::new(),
                        stderr: e,
                        comparison: None,
                        output_diffs: Vec::new(),
                        assertion_results: Vec::new(),
                        passed: false,
                        duration: Duration::ZERO,
                    });
                    passed = false;
                    break;
                }
            }
        } else {
            match current_dir.take() {
                Some(pair) => pair,
                None => match copy_state_to_temp(source.id, graph) {
                    Ok(pair) => pair,
                    Err(e) => {
                        steps.push(StepResult {
                            transition_name: transition.name.clone(),
                            source_name: source.name.clone(),
                            target_name: target.name.clone(),
                            exit_code: None,
                            stdout: String::new(),
                            stderr: e,
                            comparison: None,
                            output_diffs: Vec::new(),
                            assertion_results: Vec::new(),
                            passed: false,
                            duration: Duration::ZERO,
                        });
                        passed = false;
                        break;
                    }
                },
            }
        };

        // In Full mode, run source state assertions on the first step
        let mut source_assertion_results = Vec::new();
        if run_assertions_flag && step_idx == 0 {
            let source_assertions = graph.assertions_for(source.id);
            if !source_assertions.is_empty() {
                source_assertion_results = run_assertions(
                    &source_assertions,
                    &work_dir,
                    &source.env,
                    graph,
                    &opts.sandbox,
                );
            }
        }

        // Build recording path if recording is enabled
        let recording_path = opts.recording.as_ref().map(|rc| {
            let path_dir = rc.output_dir.join(format!("path-{path_idx}"));
            path_dir.join(format!("step-{step_idx}.cast"))
        });

        // Execute the transition command in the sandboxed env
        let step_result = execute_transition(
            transition,
            &work_dir,
            &source.env,
            target,
            graph,
            &opts.sandbox,
            run_assertions_flag,
            recording_path.as_ref(),
        );

        // Merge source assertions into the step result (first step only)
        let mut step_result = step_result;
        if !source_assertion_results.is_empty() {
            let source_failed = source_assertion_results.iter().any(|a| !a.passed);
            step_result
                .assertion_results
                .splice(0..0, source_assertion_results);
            if source_failed {
                step_result.passed = false;
            }
        }

        let step_passed = step_result.passed;
        if !step_passed {
            passed = false;
        }

        // If this step passed and there are more steps, carry the temp dir forward
        if step_passed && step_idx + 1 < path.steps.len() {
            current_dir = Some((temp_dir, work_dir));
        } else if !opts.keep_temp {
            drop(temp_dir); // cleanup
        }

        steps.push(step_result);

        if !step_passed {
            break; // stop on first failure
        }
    }

    PathResult {
        path_display,
        steps,
        passed,
        duration: Duration::ZERO,
    }
}

/// Copy a state's files (excluding .missouri/) to a temp directory.
fn copy_state_to_temp(
    state_id: StateId,
    graph: &StateGraph,
) -> std::result::Result<(TempDir, Utf8PathBuf), String> {
    let state = &graph.states[state_id.0];
    let temp_dir = TempDir::new().map_err(|e| format!("failed to create temp dir: {e}"))?;
    let temp_path = Utf8PathBuf::try_from(temp_dir.path().to_owned())
        .map_err(|e| format!("temp dir path not UTF-8: {e}"))?;

    copy_dir_recursive(&state.path, &temp_path, &graph.config_dir)
        .map_err(|e| format!("failed to copy state to temp dir: {e}"))?;

    Ok((temp_dir, temp_path))
}

/// Recursively copy directory contents, skipping the config directory.
fn copy_dir_recursive(src: &Utf8Path, dst: &Utf8Path, config_dir: &str) -> std::io::Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str == config_dir {
            continue;
        }

        let src_path = Utf8PathBuf::try_from(entry.path())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let dst_path = dst.join(name_str.as_ref());

        let ft = entry.file_type()?;
        if ft.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path, config_dir)?;
        } else if ft.is_symlink() {
            let target = std::fs::read_link(&src_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dst_path)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_file(&target, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Run assertion commands against a state in a working directory.
fn run_assertions(
    assertions: &[&Assertion],
    work_dir: &Utf8Path,
    state_env: &std::collections::BTreeMap<String, String>,
    graph: &StateGraph,
    sandbox: &Sandbox,
) -> Vec<AssertionResult> {
    assertions
        .iter()
        .map(|assertion| run_single_assertion(assertion, work_dir, state_env, graph, sandbox))
        .collect()
}

/// Run a single assertion command and compare output.
fn run_single_assertion(
    assertion: &Assertion,
    work_dir: &Utf8Path,
    state_env: &std::collections::BTreeMap<String, String>,
    graph: &StateGraph,
    sandbox: &Sandbox,
) -> AssertionResult {
    let state = &graph.states[assertion.state.0];
    let bin_dir = state.path.join(&graph.config_dir).join("bin");
    let bin_dir_opt = if bin_dir.exists() {
        Some(bin_dir.as_path())
    } else {
        None
    };
    let system_path =
        std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into());
    let base_path = state_env
        .get("PATH")
        .map(|s| s.as_str())
        .unwrap_or(&system_path);
    let path_env = build_path_env(bin_dir_opt, graph.project_bin.as_deref(), base_path);

    let output = match sandbox {
        Sandbox::None => build_assertion_command_bare(assertion, work_dir, state_env, &path_env),
        Sandbox::Nix { nix_bin, packages } => build_assertion_command_nix(
            assertion, work_dir, state_env, &path_env, nix_bin, packages,
        ),
    };

    let output = match output {
        Some(result) => result,
        None => {
            return AssertionResult {
                name: assertion.name.clone(),
                passed: false,
                exit_code: None,
                stdout_diff: None,
                stderr_diff: None,
                error: Some("empty command".into()),
            };
        }
    };

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return AssertionResult {
                name: assertion.name.clone(),
                passed: false,
                exit_code: None,
                stdout_diff: None,
                stderr_diff: None,
                error: Some(format!("failed to execute command: {e}")),
            };
        }
    };

    let exit_code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Exit code check: should_fail inverts the expectation
    if assertion.should_fail {
        if output.status.success() {
            return AssertionResult {
                name: assertion.name.clone(),
                passed: false,
                exit_code,
                stdout_diff: None,
                stderr_diff: None,
                error: Some("expected command to fail, but it exited 0".into()),
            };
        }
        // Command failed as expected — pass (no stdout/stderr comparison for should_fail)
        return AssertionResult {
            name: assertion.name.clone(),
            passed: true,
            exit_code,
            stdout_diff: None,
            stderr_diff: None,
            error: None,
        };
    }

    if !output.status.success() {
        return AssertionResult {
            name: assertion.name.clone(),
            passed: false,
            exit_code,
            stdout_diff: None,
            stderr_diff: None,
            error: Some(format!(
                "command exited with {}",
                exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into())
            )),
        };
    }

    // Compare stdout/stderr if expected values are specified
    let stdout_diff = assertion.expected_stdout.as_ref().and_then(|expected| {
        if *expected != stdout {
            Some((expected.clone(), stdout.clone()))
        } else {
            None
        }
    });

    let stderr_diff = assertion.expected_stderr.as_ref().and_then(|expected| {
        if *expected != stderr {
            Some((expected.clone(), stderr.clone()))
        } else {
            None
        }
    });

    let passed = stdout_diff.is_none() && stderr_diff.is_none();

    AssertionResult {
        name: assertion.name.clone(),
        passed,
        exit_code,
        stdout_diff,
        stderr_diff,
        error: None,
    }
}

/// Build a bare assertion command (no sandbox).
fn build_assertion_command_bare(
    assertion: &Assertion,
    work_dir: &Utf8Path,
    state_env: &std::collections::BTreeMap<String, String>,
    path_env: &str,
) -> Option<std::io::Result<std::process::Output>> {
    if assertion.shell {
        Some(crate::signal::run_tracked(
            Command::new("sh")
                .arg("-c")
                .arg(&assertion.command)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(state_env.iter())
                .env("PATH", path_env),
        ))
    } else {
        let parts: Vec<&str> = assertion.command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        Some(crate::signal::run_tracked(
            Command::new(parts[0])
                .args(&parts[1..])
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(state_env.iter())
                .env("PATH", path_env),
        ))
    }
}

/// Build an assertion command wrapped in `nix shell`.
fn build_assertion_command_nix(
    assertion: &Assertion,
    work_dir: &Utf8Path,
    state_env: &std::collections::BTreeMap<String, String>,
    path_env: &str,
    nix_bin: &Utf8Path,
    packages: &[String],
) -> Option<std::io::Result<std::process::Output>> {
    let mut args: Vec<String> = vec!["shell".into()];
    args.push("--extra-experimental-features".into());
    args.push("nix-command flakes".into());
    for pkg in packages {
        args.push(format!("nixpkgs#{pkg}"));
    }
    args.push("--command".into());

    if assertion.shell {
        args.push("sh".into());
        args.push("-c".into());
        args.push(assertion.command.clone());
        Some(crate::signal::run_tracked(
            Command::new(nix_bin.as_str())
                .args(&args)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(state_env.iter())
                .env("PATH", path_env),
        ))
    } else {
        let parts: Vec<&str> = assertion.command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        for p in parts {
            args.push(p.to_string());
        }
        Some(crate::signal::run_tracked(
            Command::new(nix_bin.as_str())
                .args(&args)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(state_env.iter())
                .env("PATH", path_env),
        ))
    }
}

/// Execute a single transition command and compare the result.
fn execute_transition(
    transition: &Transition,
    work_dir: &Utf8Path,
    source_env: &std::collections::BTreeMap<String, String>,
    target: &crate::graph::State,
    graph: &StateGraph,
    sandbox: &Sandbox,
    run_assertions_flag: bool,
    recording_path: Option<&Utf8PathBuf>,
) -> StepResult {
    let step_start = Instant::now();
    let source_name = graph.states[transition.source.0].name.clone();
    let target_name = target.name.clone();

    // Build PATH: source state's config bin/ → project bin/ → base PATH
    let source_state = &graph.states[transition.source.0];
    let bin_dir = source_state.path.join(&graph.config_dir).join("bin");
    let bin_dir_opt = if bin_dir.exists() {
        Some(bin_dir.as_path())
    } else {
        None
    };
    let system_path =
        std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".into());
    let base_path = source_env
        .get("PATH")
        .map(|s| s.as_str())
        .unwrap_or(&system_path);
    let path_env = build_path_env(bin_dir_opt, graph.project_bin.as_deref(), base_path);

    // Run the command — using recorder if recording is enabled, otherwise normal execution.
    let output = if let Some(cast_path) = recording_path {
        Some(crate::recorder::record_command(
            &transition.command,
            transition.shell,
            work_dir,
            source_env,
            &path_env,
            cast_path,
            sandbox,
        ))
    } else {
        match sandbox {
            Sandbox::None => build_command_bare(transition, work_dir, source_env, &path_env),
            Sandbox::Nix { nix_bin, packages } => build_command_nix(
                transition, work_dir, source_env, &path_env, nix_bin, packages,
            ),
        }
    };

    // Handle empty command (non-shell mode)
    let output = match output {
        Some(result) => result,
        None => {
            return StepResult {
                transition_name: transition.name.clone(),
                source_name,
                target_name,
                exit_code: None,
                stdout: String::new(),
                stderr: "empty command".into(),
                comparison: None,
                output_diffs: Vec::new(),
                assertion_results: Vec::new(),
                passed: false,
                duration: step_start.elapsed(),
            };
        }
    };

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            return StepResult {
                transition_name: transition.name.clone(),
                source_name,
                target_name,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("failed to execute command: {e}"),
                comparison: None,
                output_diffs: Vec::new(),
                assertion_results: Vec::new(),
                passed: false,
                duration: step_start.elapsed(),
            };
        }
    };

    let exit_code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return StepResult {
            transition_name: transition.name.clone(),
            source_name,
            target_name,
            exit_code,
            stdout,
            stderr,
            comparison: None,
            output_diffs: Vec::new(),
            assertion_results: Vec::new(),
            passed: false,
            duration: step_start.elapsed(),
        };
    }

    // Compare transition stdout/stderr if expected values are specified
    let mut output_diffs = Vec::new();
    if let Some(expected) = &transition.expected_stdout
        && *expected != stdout
    {
        output_diffs.push(OutputDiff::StdoutMismatch {
            expected: expected.clone(),
            actual: stdout.clone(),
        });
    }
    if let Some(expected) = &transition.expected_stderr
        && *expected != stderr
    {
        output_diffs.push(OutputDiff::StderrMismatch {
            expected: expected.clone(),
            actual: stderr.clone(),
        });
    }

    // Build bin dirs for comparator PATH: state bin/ + project bin/
    let source_state = &graph.states[transition.source.0];
    let bin_dir = source_state.path.join(&graph.config_dir).join("bin");
    let mut comparator_bin_dirs: Vec<&Utf8Path> = Vec::new();
    if bin_dir.exists() {
        comparator_bin_dirs.push(bin_dir.as_path());
    }
    if let Some(ref pb) = graph.project_bin {
        comparator_bin_dirs.push(pb.as_path());
    }

    // Compare the result against the expected target state
    let comparison = compare::compare_trees(
        work_dir,
        &target.path,
        &transition.file_comparators,
        &comparator_bin_dirs,
        source_env,
        &graph.config_dir,
        &graph.ignore,
        sandbox,
    );

    // Compare env vars only when the target state or transition defines env expectations.
    let env_diffs = if !target.env.is_empty() || !transition.env_comparators.is_empty() {
        compare::compare_env(
            source_env,
            &target.env,
            &transition.env_comparators,
            &comparator_bin_dirs,
            source_env,
            sandbox,
        )
    } else {
        Vec::new()
    };

    let mut comparison = comparison;
    comparison.env_diffs = env_diffs;
    comparison.passed = comparison.passed && comparison.env_diffs.is_empty();

    // Run target state assertions in Full mode
    let assertion_results = if run_assertions_flag {
        let target_assertions = graph.assertions_for(transition.target);
        if !target_assertions.is_empty() {
            run_assertions(&target_assertions, work_dir, &target.env, graph, sandbox)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let assertions_passed = assertion_results.iter().all(|a| a.passed);
    let passed = comparison.passed && output_diffs.is_empty() && assertions_passed;

    StepResult {
        transition_name: transition.name.clone(),
        source_name,
        target_name,
        exit_code,
        stdout,
        stderr,
        comparison: Some(comparison),
        output_diffs,
        assertion_results,
        passed,
        duration: step_start.elapsed(),
    }
}

/// Build a command without any sandbox wrapping (env_clear + manual PATH).
fn build_command_bare(
    transition: &Transition,
    work_dir: &Utf8Path,
    source_env: &std::collections::BTreeMap<String, String>,
    path_env: &str,
) -> Option<std::io::Result<std::process::Output>> {
    if transition.shell {
        Some(crate::signal::run_tracked(
            Command::new("sh")
                .arg("-c")
                .arg(&transition.command)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(source_env.iter())
                .env("PATH", path_env),
        ))
    } else {
        let parts: Vec<&str> = transition.command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        Some(crate::signal::run_tracked(
            Command::new(parts[0])
                .args(&parts[1..])
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(source_env.iter())
                .env("PATH", path_env),
        ))
    }
}

/// Build a command wrapped in `nix shell` (env_clear + state vars + PATH).
///
/// Shell mode: `nix shell nixpkgs#pkg1 ... --command sh -c "<command>"`
/// Non-shell:  `nix shell nixpkgs#pkg1 ... --command <cmd> <args...>`
fn build_command_nix(
    transition: &Transition,
    work_dir: &Utf8Path,
    source_env: &std::collections::BTreeMap<String, String>,
    path_env: &str,
    nix_bin: &Utf8Path,
    packages: &[String],
) -> Option<std::io::Result<std::process::Output>> {
    let mut args: Vec<String> = vec!["shell".into()];
    args.push("--extra-experimental-features".into());
    args.push("nix-command flakes".into());
    for pkg in packages {
        args.push(format!("nixpkgs#{pkg}"));
    }
    args.push("--command".into());

    if transition.shell {
        args.push("sh".into());
        args.push("-c".into());
        args.push(transition.command.clone());
        Some(crate::signal::run_tracked(
            Command::new(nix_bin.as_str())
                .args(&args)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(source_env.iter())
                .env("PATH", path_env),
        ))
    } else {
        let parts: Vec<&str> = transition.command.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }
        for p in parts {
            args.push(p.to_string());
        }
        Some(crate::signal::run_tracked(
            Command::new(nix_bin.as_str())
                .args(&args)
                .current_dir(work_dir.as_std_path())
                .env_clear()
                .envs(source_env.iter())
                .env("PATH", path_env),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8Path;
    use std::fs;

    fn make_state(tmp: &Utf8Path, name: &str, yaml: &str) {
        let state_dir = tmp.join(name);
        let missouri_dir = state_dir.join(".missouri");
        fs::create_dir_all(&missouri_dir).unwrap();
        fs::write(missouri_dir.join("missouri.yml"), yaml).unwrap();
    }

    #[test]
    fn detect_sandbox_none_when_no_config() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(tmp.path()).unwrap();

        make_state(
            root,
            "a",
            r#"
transitions:
  - command: "echo"
    target: "../b"
"#,
        );
        make_state(root, "b", "{}");

        let graph = StateGraph::discover(root, ".missouri").unwrap();
        assert!(matches!(graph.sandbox_config, SandboxConfig::None));
        let sandbox = detect_sandbox(&graph).unwrap();
        assert!(matches!(sandbox, Sandbox::None));
    }

    #[test]
    fn detect_sandbox_packages_resolves_to_nix() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(tmp.path()).unwrap();

        // Create project-level config with packages
        let root_missouri = root.join(".missouri");
        fs::create_dir_all(&root_missouri).unwrap();
        fs::write(
            root_missouri.join("missouri.yml"),
            "packages:\n  - python3\n  - uv\n",
        )
        .unwrap();

        make_state(
            root,
            "a",
            r#"
transitions:
  - command: "echo"
    target: "../b"
"#,
        );
        make_state(root, "b", "{}");

        let graph = StateGraph::discover(root, ".missouri").unwrap();
        assert!(matches!(graph.sandbox_config, SandboxConfig::Packages(_)));

        // nix must be on PATH for this test to produce Sandbox::Nix
        if which_nix().is_none() {
            eprintln!("skipping detect_sandbox_packages_resolves_to_nix: nix not on PATH");
            return;
        }

        // Clear MISSOURI_SANDBOX in case it's set (e.g., inside nix build)
        // SAFETY: test is single-threaded for this env var manipulation.
        let saved = std::env::var("MISSOURI_SANDBOX").ok();
        unsafe { std::env::remove_var("MISSOURI_SANDBOX") };

        let sandbox = detect_sandbox(&graph).unwrap();

        // Restore if it was set
        if let Some(val) = saved {
            unsafe { std::env::set_var("MISSOURI_SANDBOX", val) };
        }

        match sandbox {
            Sandbox::Nix { nix_bin, packages } => {
                assert!(nix_bin.as_str().contains("nix"));
                assert_eq!(packages, vec!["python3", "uv"]);
            }
            _ => panic!("expected Sandbox::Nix, got {sandbox:?}"),
        }
    }

    #[test]
    fn detect_sandbox_preinstalled_via_env_var() {
        let tmp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(tmp.path()).unwrap();

        // Create project-level config with packages
        let root_missouri = root.join(".missouri");
        fs::create_dir_all(&root_missouri).unwrap();
        fs::write(
            root_missouri.join("missouri.yml"),
            "packages:\n  - python3\n  - uv\n",
        )
        .unwrap();

        make_state(
            root,
            "a",
            r#"
transitions:
  - command: "echo"
    target: "../b"
"#,
        );
        make_state(root, "b", "{}");

        let graph = StateGraph::discover(root, ".missouri").unwrap();
        assert!(matches!(graph.sandbox_config, SandboxConfig::Packages(_)));

        // With MISSOURI_SANDBOX=preinstalled, packages config should resolve
        // to Sandbox::None (tools assumed already on PATH).
        // SAFETY: test is single-threaded for this env var manipulation.
        unsafe { std::env::set_var("MISSOURI_SANDBOX", "preinstalled") };
        let sandbox = detect_sandbox(&graph).unwrap();
        unsafe { std::env::remove_var("MISSOURI_SANDBOX") };
        assert!(
            matches!(sandbox, Sandbox::None),
            "expected Sandbox::None when MISSOURI_SANDBOX=preinstalled, got {sandbox:?}"
        );
    }

    #[test]
    fn which_nix_finds_binary() {
        let result = which_nix();
        if result.is_none() {
            eprintln!("skipping which_nix_finds_binary: nix not on PATH");
            return;
        }
        assert!(result.unwrap().as_str().ends_with("nix"));
    }
}
