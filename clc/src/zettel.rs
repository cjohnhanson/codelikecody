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
            "Zettel is a zettelkasten-style knowledge base. Notes are atomic — one idea \
             per note, written in the author's own words. Notes link to each other via \
             frontmatter `links:` or `[[id]]` references in the body.\n\n",
        );

        out.push_str(
            "Notes have a status: `draft` or `permanent`. Drafts are captured ideas \
             that haven't been processed yet. Permanent notes have been reviewed, \
             reformulated, and linked by the human. Only the human promotes notes \
             to permanent — never do this autonomously.\n\n",
        );

        let _ = writeln!(out, "{} notes.", self.note_count);

        out.push_str(
            "\n### How to use zettel\n\n\
             **Reading for context:** Before starting work on a topic, search the \
             knowledge base. Use `zettel search`, `zettel read --tag`, or \
             `zettel context <id>` to load relevant knowledge.\n\n\
             **Creating notes:** Only when the human asks. Create as `draft` (the default). \
             Include enough context that the note is useful later — what prompted it, \
             what it connects to.\n\n\
             **Never** create notes autonomously, promote drafts to permanent, \
             or delete notes without being asked.\n\n",
        );

        out.push_str("### Commands\n\n");
        out.push_str(
            "  zettel note list [--tag T] [--status S] [--where K:V]  List/filter notes\n",
        );
        out.push_str(
            "  zettel note show <id> [--field F]                      Show a note\n",
        );
        out.push_str(
            "  zettel note create <title> [-t tags] [-s status]       Create a note\n",
        );
        out.push_str(
            "  zettel note edit <id> [--status ...] [--add-tag ...]   Edit a note\n",
        );
        out.push_str(
            "  zettel search <pattern>                                Regex search\n",
        );
        out.push_str(
            "  zettel read [--tag T] [--status S]                     Dump full content\n",
        );
        out.push_str(
            "  zettel context <id> [-d depth]                         Note + neighborhood\n",
        );
        out.push_str(
            "  zettel backlinks <id>                                  What links here\n",
        );
        out.push_str(
            "  zettel orphans                                         Unlinked notes\n",
        );
        out.push_str(
            "  zettel stats                                           Knowledge base health\n",
        );

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
