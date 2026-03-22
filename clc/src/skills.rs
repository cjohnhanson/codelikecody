//! Thin wrapper over the almanac crate for clc integration.

use std::path::Path;

use almanac::SkillSource;

/// Scan all configured skill sources and format an index for prime text injection.
pub fn format_prime_section(project_dir: &Path, sources: &[SkillSource]) -> String {
    let entries = almanac::skill::index(project_dir, sources);
    almanac::skill::format_index(&entries)
}
