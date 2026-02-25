use std::fmt::Write;
use std::path::Path;

use camino::Utf8PathBuf;

use crate::error::Error;

/// Summary of a missouri test run.
#[derive(Debug)]
pub struct TestSummary {
    pub passed: usize,
    pub total: usize,
    pub all_green: bool,
}

/// Discover and run the missouri test suite at `tests/missouri/` in the project.
/// Returns `None` if no missouri project exists at that path.
pub fn run_tests(project_dir: &Path) -> Result<Option<TestSummary>, Error> {
    let test_dir = project_dir.join("tests").join("missouri");
    if !test_dir.is_dir() {
        return Ok(None);
    }

    let test_dir = Utf8PathBuf::try_from(test_dir)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 test directory path: {e}")))?;

    let config_dir = ".missouri";

    let graph = missouri::graph::StateGraph::discover(&test_dir, config_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to discover missouri graph: {e}")))?;

    let paths = missouri::paths::enumerate_paths(&graph);
    if paths.is_empty() {
        return Ok(Some(TestSummary {
            passed: 0,
            total: 0,
            all_green: true,
        }));
    }

    let sandbox = missouri::executor::detect_sandbox(&graph)
        .map_err(|e| Error::NonBlocking(format!("failed to detect sandbox: {e}")))?;

    let opts = missouri::executor::RunOptions {
        keep_temp: false,
        verbose: false,
        sandbox,
        check_mode: missouri::executor::CheckMode::Full,
        recording: None,
    };

    // Run setup if any.
    if !graph.setup.is_empty() {
        let setup_results = missouri::executor::run_setup_phase(&graph, &opts);
        let setup_failed = setup_results.iter().any(|r| !r.passed);
        if setup_failed {
            return Ok(Some(TestSummary {
                passed: 0,
                total: 1,
                all_green: false,
            }));
        }
    }

    let results = missouri::executor::run_all_paths(&graph, &paths, &opts, None);

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    Ok(Some(TestSummary {
        passed,
        total,
        all_green: passed == total,
    }))
}

/// Missouri state for trait implementation (without running tests).
#[derive(Debug)]
pub struct MissouriState {
    pub has_tests: bool,
    pub path_count: usize,
    pub state_count: usize,
}

/// Detect missouri state without running tests.
#[allow(dead_code)]
pub fn detect(project_dir: &Path) -> Result<MissouriState, Error> {
    let test_dir = project_dir.join("tests").join("missouri");
    if !test_dir.is_dir() {
        return Ok(MissouriState {
            has_tests: false,
            path_count: 0,
            state_count: 0,
        });
    }

    let test_dir = Utf8PathBuf::try_from(test_dir)
        .map_err(|e| Error::NonBlocking(format!("non-UTF8 test directory path: {e}")))?;

    let config_dir = ".missouri";

    let graph = missouri::graph::StateGraph::discover(&test_dir, config_dir)
        .map_err(|e| Error::NonBlocking(format!("failed to discover missouri graph: {e}")))?;

    let paths = missouri::paths::enumerate_paths(&graph);

    Ok(MissouriState {
        has_tests: true,
        path_count: paths.len(),
        state_count: graph.states.len(),
    })
}

/// Append rich missouri test authoring detail — directory structure,
/// missouri.yml format, assertions. Used in `tests-unwritten` phase
/// when the agent needs enough context to actually write tests.
fn append_authoring_detail(out: &mut String) {
    out.push_str("## Missouri Test Authoring\n\n");

    out.push_str(
        "Missouri tests are directed graphs of filesystem states. Each state is a \
         directory containing the files that should exist at that point, plus a \
         `.missouri/missouri.yml` config that defines transitions and assertions.\n\n",
    );

    out.push_str("### Directory Layout\n\n");
    out.push_str(
        "```\n\
         tests/missouri/\n\
         ├── missouri.yml            # project-level config (env, setup)\n\
         ├── state-a/\n\
         │   ├── .missouri/\n\
         │   │   └── missouri.yml    # transitions + assertions\n\
         │   └── <expected files>    # files that should exist in this state\n\
         └── state-b/\n\
             ├── .missouri/\n\
             │   └── missouri.yml\n\
             └── <expected files>\n\
         ```\n\n",
    );

    out.push_str("### State Config (`.missouri/missouri.yml`)\n\n");
    out.push_str(
        "```yaml\n\
         transitions:\n\
         \x20 - name: \"describe the transition\"\n\
         \x20   command: \"the-command-to-run\"\n\
         \x20   target: ../next-state\n\
         \x20   stdout: \"expected stdout\\n\"   # optional\n\
         \x20   stderr: \"\"                     # optional\n\n\
         assertions:\n\
         \x20 - name: \"check something\"\n\
         \x20   command: \"test -f output.txt\"\n\
         \x20   should_fail: false              # optional, default false\n\
         ```\n\n",
    );

    out.push_str("### Key Concepts\n\n");
    out.push_str(
        "- **States**: Directories with `.missouri/missouri.yml`. Files in the directory \
         represent expected filesystem state.\n\
         - **Transitions**: Commands that move from one state to another. Missouri runs \
         the command, then diffs actual files against the target state directory.\n\
         - **Assertions**: Side-effect-free checks run within a state. No state change.\n\
         - **Paths**: Sequences of transitions from root states (no inbound edges) through \
         the graph. Missouri discovers and runs all paths.\n\
         - **Automatic diffing**: Files and environment variables are compared between \
         states automatically. Unexpected changes cause failures.\n\
         - **Ignore patterns**: `.missouri/ignore` (gitignore syntax) excludes files from \
         comparison.\n\n",
    );

    out.push_str("### Project Config (`missouri.yml` at test root)\n\n");
    out.push_str(
        "```yaml\n\
         env:\n\
         \x20 APP_ENV: test\n\
         setup:\n\
         \x20 - command: \"cargo build\"\n\
         ```\n\n",
    );
}

impl clc_sdk::ClcTool for MissouriState {
    fn prime(&self, ctx: &clc_sdk::PrimeContext) -> String {
        let mut out = String::new();
        out.push_str("# Missouri Testing\n\n");

        if !self.has_tests {
            out.push_str(
                "Missouri is a filesystem state graph testing tool.\n\
                 No test suite exists yet in this project.\n\n",
            );

            // In tests_unwritten, provide authoring detail even when no tests exist.
            if ctx.phase.as_deref() == Some("tests-unwritten") {
                append_authoring_detail(&mut out);
                out.push_str("\n## What to Do\n\n");
                out.push_str(
                    "Write tests that define the expected behavior for this issue.\n\
                     Do not write implementation code yet. Define the state graph first.\n\
                     Advance phase: `clc status set tests-written`\n",
                );
            } else {
                out.push_str("Tests belong in `tests/missouri/` as filesystem state graphs.\n");
            }

            return out;
        }

        out.push_str("This project uses missouri for end-to-end testing.\n");
        let _ = write!(
            out,
            "{} test paths across {} states.\n\n",
            self.path_count, self.state_count
        );

        match ctx.phase.as_deref() {
            Some("tests-unwritten") => {
                append_authoring_detail(&mut out);
                out.push_str("\n## What to Do\n\n");
                out.push_str(
                    "Write tests that define the expected behavior for this issue.\n\
                     Do not write implementation code yet. Define the state graph first.\n\
                     Advance phase: `clc status set tests-written`\n",
                );
            }
            Some("tests-written") => {
                out.push_str("## What to Do\n\n");
                out.push_str(
                    "Tests are written. Run them to confirm they fail as expected.\n\
                     Advance phase: `clc status set red`\n",
                );
            }
            Some("red") => {
                out.push_str("## What to Do\n\n");
                out.push_str(
                    "Tests exist and should be failing. Begin implementation to make them pass.\n\
                     Advance phase: `clc status set implementing`\n",
                );
            }
            Some("implementing") => {
                out.push_str("Run `clc status` to check test results.\n");
                out.push_str(
                    "Fix failing tests before advancing. When all paths pass:\n\
                     `clc status set green`\n",
                );
            }
            Some("green") => {
                out.push_str("All test paths are passing.\n");
            }
            _ => {
                out.push_str("Run `missouri run` or `clc status` to check test results.\n");
            }
        }

        out
    }

    fn status_basic(&self) -> String {
        if !self.has_tests {
            return "missouri: no tests".to_string();
        }
        format!(
            "missouri: {} paths, {} states",
            self.path_count, self.state_count
        )
    }

    fn status_full(&self) -> String {
        if !self.has_tests {
            return "missouri: no test suite found".to_string();
        }
        format!(
            "# missouri\n\n{} test paths across {} states\n",
            self.path_count, self.state_count
        )
    }
}

impl clc_sdk::ClcTool for TestSummary {
    fn prime(&self, _ctx: &clc_sdk::PrimeContext) -> String {
        if self.all_green {
            format!(
                "# Missouri Testing\n\n\
                 All {} test paths passing.\n",
                self.total
            )
        } else {
            format!(
                "# Missouri Testing\n\n\
                 {}/{} test paths passing — {} failing.\n\
                 Fix failing paths before advancing phase.\n",
                self.passed,
                self.total,
                self.total - self.passed
            )
        }
    }

    fn status_basic(&self) -> String {
        format!("missouri: {}/{} passing", self.passed, self.total)
    }

    fn status_full(&self) -> String {
        if self.all_green {
            format!("# missouri\n\nAll {} test paths passing\n", self.total)
        } else {
            format!(
                "# missouri\n\n{}/{} passing — {} failing\n",
                self.passed,
                self.total,
                self.total - self.passed
            )
        }
    }
}
