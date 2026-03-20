use std::fmt::Write;
use std::path::Path;

use camino::Utf8Path;

use crate::error::Error;

/// Summary of the tisket state for this project.
#[derive(Debug)]
pub struct TisketState {
    pub has_repo: bool,
    pub current_issue: Option<CurrentIssue>,
    pub open_count: usize,
}

/// An issue that appears to be the active one for this branch.
#[derive(Debug)]
pub struct CurrentIssue {
    pub id: String,
    pub title: String,
    pub status: tisket::issue::Status,
    pub body: String,
    pub scratch: String,
}

/// Detect tisket state for the given project directory.
/// Looks for `tisket.yml`, opens the repo, and tries to find
/// an issue matching the current branch name.
pub fn detect(project_dir: &Path, branch: Option<&str>) -> Result<TisketState, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo = match tisket::Repo::open(utf8_dir) {
        Ok(r) => r,
        Err(tisket::Error::NotInitialized) => {
            return Ok(TisketState {
                has_repo: false,
                current_issue: None,
                open_count: 0,
            });
        }
        Err(e) => {
            return Err(Error::NonBlocking(format!("tisket error: {e}")));
        }
    };

    let open_count = repo
        .list_issues(None, None, None, false, &[])
        .map(|issues| issues.len())
        .unwrap_or(0);

    let current_issue = branch.and_then(|name| find_issue_for_branch(&repo, name));

    Ok(TisketState {
        has_repo: true,
        current_issue,
        open_count,
    })
}

/// Try to find an issue whose ID matches the branch name.
fn find_issue_for_branch(repo: &tisket::Repo, branch: &str) -> Option<CurrentIssue> {
    let issue = repo.find_issue(branch).ok()?;
    if issue.closed {
        return None;
    }
    Some(CurrentIssue {
        id: issue.id,
        title: issue.frontmatter.title,
        status: issue.frontmatter.status,
        body: issue.body,
        scratch: issue.scratch,
    })
}

impl clc_sdk::ClcTool for TisketState {
    fn prime(&self, ctx: &clc_sdk::PrimeContext) -> String {
        use std::fmt::Write;

        let mut out = String::new();
        out.push_str("## Tisket (issue tracker)\n\n");

        if !self.has_repo {
            out.push_str(
                "Tisket is not initialized in this project.\n\
                 Issues live as markdown files in `.tisket/`. Each has a status, priority,\n\
                 and optional dependencies. Tisket is where tasks come from.\n",
            );
            return out;
        }

        out.push_str(
            "Issues live as markdown files in `.tisket/`. Each has a status, priority,\n\
             and optional dependencies. Tisket is where tasks come from —\n\
             don't invent work, pick up what's there.\n\n",
        );

        if let Some(issue) = &self.current_issue {
            let _ = write!(out, "### Active: {} ({})\n\n", issue.id, issue.status);
            let _ = writeln!(out, "**{}**\n", issue.title);

            if !issue.body.is_empty() {
                out.push_str(&issue.body);
                out.push_str("\n\n");
            }

            if !issue.scratch.is_empty() {
                out.push_str("### Scratch Notes\n\n");
                out.push_str(&issue.scratch);
                out.push('\n');
            }

            // Phase-adapted directives when an issue is active.
            match ctx.phase.as_deref() {
                Some("tests-unwritten" | "tests-written" | "red") => {
                    out.push_str(
                        "\nThis issue defines the work. Review the requirements above before \
                         implementing.\n",
                    );
                }
                Some("implementing") => {
                    out.push_str(
                        "\nAll work in this session relates to this issue.\n\
                         Update the scratch notes with progress as work proceeds.\n",
                    );
                }
                Some("green") => {
                    out.push_str(
                        "\nUpdate the scratch notes with a summary of what was done.\n\
                         Run `clc done` to finalize.\n",
                    );
                }
                _ => {}
            }
        } else {
            let _ = writeln!(
                out,
                "{} open issues. No active issue on this branch.",
                self.open_count
            );
            out.push_str(
                "\nBrowse with `tisket issue list` or `tisket issue show <id>`.\n\
                 Pick up work with `clc pickup <id>`.\n",
            );
        }

        out
    }

    fn status_basic(&self) -> String {
        if !self.has_repo {
            return "tisket: not initialized".to_string();
        }
        self.current_issue.as_ref().map_or_else(
            || format!("tisket: no active issue — {} open", self.open_count),
            |issue| {
                format!(
                    "tisket: {} ({}) — {} open",
                    issue.id, issue.status, self.open_count
                )
            },
        )
    }

    fn status_full(&self) -> String {
        if !self.has_repo {
            return "tisket: not initialized".to_string();
        }
        let mut out = String::new();
        if let Some(issue) = &self.current_issue {
            let _ = write!(out, "# tisket: {} ({})\n\n", issue.id, issue.status);
            let _ = write!(out, "**{}**\n\n", issue.title);
            if !issue.body.is_empty() {
                out.push_str(&issue.body);
                out.push('\n');
            }
            if !issue.scratch.is_empty() {
                out.push_str("\n## Scratch\n\n");
                out.push_str(&issue.scratch);
                out.push('\n');
            }
        } else {
            let _ = writeln!(out, "tisket: no active issue — {} open", self.open_count);
        }
        out
    }
}
