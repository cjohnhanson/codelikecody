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

impl clc_sdk::ClcTool for MissouriState {
    fn prime(&self) -> String {
        String::new()
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
    fn prime(&self) -> String {
        String::new()
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
