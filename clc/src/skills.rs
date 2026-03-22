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
    let index_text = almanac::skill::format_index(&entries);
    AlmanacState {
        skill_count: entries.len(),
        source_count: sources.len(),
        index_text,
    }
}

impl clc_sdk::ClcTool for AlmanacState {
    fn prime(&self, _ctx: &clc_sdk::PrimeContext) -> String {
        self.index_text.clone()
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
