use std::path::Path;

use camino::Utf8Path;

use crate::error::Error;

/// Summary of the tisket state for this project.
#[derive(Debug)]
pub struct TisketState {
    pub has_repo: bool,
    pub current_issue: Option<CurrentIssue>,
}

/// An issue that appears to be the active one for this branch.
#[derive(Debug)]
pub struct CurrentIssue {
    pub id: String,
    pub title: String,
    pub status: String,
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
            });
        }
        Err(e) => {
            return Err(Error::NonBlocking(format!("tisket error: {e}")));
        }
    };

    let current_issue = branch.and_then(|name| find_issue_for_branch(&repo, name));

    Ok(TisketState {
        has_repo: true,
        current_issue,
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
    })
}
