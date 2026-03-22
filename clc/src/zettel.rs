use std::fmt::Write;
use std::path::Path;

use camino::Utf8Path;

use crate::error::Error;

/// Summary of the zettel state for this project.
#[derive(Debug)]
pub struct ZettelState {
    pub has_repo: bool,
    pub note_count: usize,
}

/// Detect zettel state for the given project directory.
pub fn detect(project_dir: &Path) -> Result<ZettelState, Error> {
    let utf8_dir = Utf8Path::new(
        project_dir
            .to_str()
            .ok_or_else(|| Error::NonBlocking("non-UTF8 project directory".into()))?,
    );

    let repo = match zettel::Repo::open(utf8_dir) {
        Ok(r) => r,
        Err(zettel::Error::NotInitialized) => {
            return Ok(ZettelState {
                has_repo: false,
                note_count: 0,
            });
        }
        Err(e) => {
            return Err(Error::NonBlocking(format!("zettel error: {e}")));
        }
    };

    let filter = zettel::ListNotesFilter { tag: None, status: None };
    let note_count = repo.list_notes(&filter).map(|n| n.len()).unwrap_or(0);

    Ok(ZettelState {
        has_repo: true,
        note_count,
    })
}

impl clc_sdk::ClcTool for ZettelState {
    fn prime(&self, _ctx: &clc_sdk::PrimeContext) -> String {
        let mut out = String::new();
        out.push_str("## Zettel (knowledge base)\n\n");

        if !self.has_repo {
            out.push_str("Zettel is not initialized in this project.\n");
            return out;
        }

        out.push_str(
            "Notes are markdown files with YAML frontmatter in `.zettel/`. \
             Each note has a title, tags, and forward links to other notes. \
             Use `[[id]]` syntax in note bodies to reference other notes.\n\n",
        );

        let _ = writeln!(out, "{} notes.", self.note_count);

        out.push_str("\n### Commands\n\n");
        out.push_str("  zettel note create <title> [-t tags] [-l links]  Create a note\n");
        out.push_str("  zettel note list [--tag <tag>]                   List notes\n");
        out.push_str("  zettel note show <id>                            Show a note\n");
        out.push_str("  zettel note edit <id> [--add-tag ...] [...]      Edit a note\n");
        out.push_str("  zettel note delete <id>                          Delete a note\n");
        out.push_str("  zettel backlinks <id>                            Show what links here\n");
        out.push_str("  zettel orphans                                   Find unlinked notes\n");

        out
    }

    fn status_basic(&self) -> String {
        if !self.has_repo {
            return "zettel: not initialized".to_string();
        }
        format!("zettel: {} notes", self.note_count)
    }

    fn status_full(&self) -> String {
        if !self.has_repo {
            return "zettel: not initialized".to_string();
        }
        format!("# zettel\n\n{} notes\n", self.note_count)
    }
}
