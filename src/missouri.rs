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

    let results = missouri::executor::run_all_paths(&graph, &paths, &opts);

    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    Ok(Some(TestSummary {
        passed,
        total,
        all_green: passed == total,
    }))
}
