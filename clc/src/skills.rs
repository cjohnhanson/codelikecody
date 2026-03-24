//! Almanac integration for clc — skill aggregation and prime text injection.

use std::fmt::Write;
use std::path::Path;

use almanac::SkillSource;

/// Summary of almanac state for this project.
#[derive(Debug)]
pub struct AlmanacState {
    pub skill_count: usize,
    pub source_count: usize,
    pub index_text: String,
}

/// Detect almanac state for the given project directory and configured sources.
pub fn detect(project_dir: &Path, sources: &[SkillSource]) -> AlmanacState {
    let entries = almanac::skill::index(project_dir, sources);
    let index_text = almanac::skill::format_index_list(&entries);
    AlmanacState {
        skill_count: entries.len(),
        source_count: sources.len(),
        index_text,
    }
}

impl clc_sdk::ClcTool for AlmanacState {
    fn prime(&self, _ctx: &clc_sdk::PrimeContext) -> String {
        if self.skill_count == 0 {
            return String::new();
        }

        let mut out = String::from(
            "## Almanac (skills)\n\n\
             Skills are detailed instructions for specific tasks and contexts. Each\n\
             skill teaches a process, framework, or workflow. Load a skill when you\n\
             need its guidance — don't guess at processes that have a skill.\n\n\
             ### Commands\n\n\
             \x20 almanac list                   List all available skills\n\
             \x20 almanac show <name>            Print the full skill content\n\
             \x20 almanac search <query>         Search skills by keyword\n\n\
             Skills come from two places: built into the binary (always available)\n\
             and configured sources in clc.yml (project-specific). Built-in skills\n\
             are marked [built-in] in the listing.\n\n\
             ### When to load a skill\n\n\
             - Before evaluation or review work (code review, architecture, security, docs, design, QA)\n\
             - Before structured processes (debugging, research, writing, issue scoping)\n\
             - When the user invokes one by name\n\
             - When unsure of the right methodology for a task — search first\n\n\
             ### Available skills\n\n",
        );
        out.push_str(&self.index_text);
        out
    }

    fn status_basic(&self) -> String {
        if self.skill_count == 0 && self.source_count == 0 {
            return String::new();
        }
        format!(
            "almanac: {} skills from {} sources",
            self.skill_count, self.source_count
        )
    }

    fn status_full(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# almanac\n\n{} skills from {} sources\n",
            self.skill_count, self.source_count
        );
        out
    }
}
