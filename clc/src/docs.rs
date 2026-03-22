//! Bundled documentation, baked in at compile time.
//! clc bundles its own ecosystem-level docs plus surfaces missouri and tisket docs.

/// A single documentation page.
pub struct DocPage {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub raw: &'static str,
}

impl DocPage {
    /// Return the markdown content with the metadata comment stripped.
    pub fn content(&self) -> &str {
        let md = self.raw;
        if let Some(start) = md.find("<!-- metadata") {
            if let Some(end) = md[start..].find("-->") {
                return md[start + end + 3..].trim_start_matches('\n');
            }
        }
        md
    }
}

/// clc's own documentation pages (ecosystem-level).
static CLC_PAGES: &[DocPage] = &[
    DocPage {
        slug: "what-is-codelikecody",
        title: "What is codelikecody?",
        description: "Philosophy, tools, and how they fit together",
        raw: include_str!("../docs/what-is-codelikecody.md"),
    },
    DocPage {
        slug: "getting-started",
        title: "Getting Started",
        description: "Set up clc on a project and complete your first task",
        raw: include_str!("../docs/getting-started.md"),
    },
    DocPage {
        slug: "clc/phase-system",
        title: "The Phase System",
        description: "How clc enforces test-driven development through ordered phases",
        raw: include_str!("../docs/phase-system.md"),
    },
    DocPage {
        slug: "clc/orchestration",
        title: "Multi-Agent Orchestration",
        description: "How to dispatch, monitor, and land multiple coding agents",
        raw: include_str!("../docs/orchestration.md"),
    },
    DocPage {
        slug: "clc/cli-reference",
        title: "clc CLI Reference",
        description: "Complete command reference for the clc workflow engine",
        raw: include_str!("../docs/cli-reference.md"),
    },
];

/// Print a listing of all docs to stdout, grouped by tool.
pub fn list() {
    println!("clc");
    print!("{}", format_list(CLC_PAGES));
    println!();
    println!("missouri");
    print!("{}", missouri::docs::format_list(missouri::docs::PAGES));
    println!();
    println!("tisket");
    print!("{}", tisket::docs::format_list(tisket::docs::PAGES));
    println!();
    println!("almanac");
    print!("{}", almanac::docs::format_list(almanac::docs::PAGES));
}

/// Find a doc by flexible identifier across all tools.
/// Searches clc docs first, then missouri, then tisket, then almanac.
pub fn find(identifier: &str) -> Option<&'static str> {
    if let Some(page) = find_in(CLC_PAGES, identifier) {
        return Some(page.content());
    }
    if let Some(page) = missouri::docs::find(identifier) {
        return Some(page.content());
    }
    if let Some(page) = tisket::docs::find(identifier) {
        return Some(page.content());
    }
    if let Some(page) = almanac::docs::find(identifier) {
        return Some(page.content());
    }
    None
}

/// Print a doc by identifier. Returns false if not found.
pub fn show(identifier: &str) -> bool {
    if let Some(content) = find(identifier) {
        print!("{content}");
        true
    } else {
        false
    }
}

/// Search docs for a query string across all tools.
pub fn search(query: &str) {
    let mut any = false;
    let clc_matches = find_matching(CLC_PAGES, query);
    if !clc_matches.is_empty() {
        println!("clc");
        print!("{}", format_list_from_refs(&clc_matches));
        any = true;
    }
    let mo_matches = missouri::docs::find_matching(missouri::docs::PAGES, query);
    if !mo_matches.is_empty() {
        println!("missouri");
        print!("{}", missouri::docs::format_list_from_refs(&mo_matches));
        any = true;
    }
    let ti_matches = tisket::docs::find_matching(tisket::docs::PAGES, query);
    if !ti_matches.is_empty() {
        println!("tisket");
        print!("{}", tisket::docs::format_list_from_refs(&ti_matches));
        any = true;
    }
    let al_matches = almanac::docs::find_matching(almanac::docs::PAGES, query);
    if !al_matches.is_empty() {
        println!("almanac");
        print!("{}", almanac::docs::format_list_from_refs(&al_matches));
        any = true;
    }
    if !any {
        eprintln!("no docs matching '{query}'");
    }
}

/// Find a doc page in a given slice by flexible identifier.
fn find_in<'a>(pages: &'a [DocPage], identifier: &str) -> Option<&'a DocPage> {
    // 1. Exact slug match
    if let Some(page) = pages.iter().find(|p| p.slug == identifier) {
        return Some(page);
    }
    // 2. Case-insensitive slug match
    let lower = identifier.to_lowercase();
    if let Some(page) = pages.iter().find(|p| p.slug.to_lowercase() == lower) {
        return Some(page);
    }
    // 3. Case-insensitive title match
    if let Some(page) = pages.iter().find(|p| p.title.to_lowercase() == lower) {
        return Some(page);
    }
    // 4. Unique slug prefix
    let prefix_matches: Vec<_> = pages
        .iter()
        .filter(|p| p.slug.starts_with(identifier) || p.slug.starts_with(&lower))
        .collect();
    if prefix_matches.len() == 1 {
        return Some(prefix_matches[0]);
    }
    None
}

/// Format a listing of doc pages showing slug (what to type) and description.
fn format_list(pages: &[DocPage]) -> String {
    let mut out = String::new();
    for page in pages {
        out.push_str(&format!("  {:<25} {}\n", page.slug, page.description));
    }
    out
}

/// Format a listing from a vec of page references.
fn format_list_from_refs(pages: &[&DocPage]) -> String {
    let mut out = String::new();
    for page in pages {
        out.push_str(&format!("  {:<25} {}\n", page.slug, page.description));
    }
    out
}

/// Find all docs matching a query string. Returns matching pages.
fn find_matching<'a>(pages: &'a [DocPage], query: &str) -> Vec<&'a DocPage> {
    let q = query.to_lowercase();
    pages
        .iter()
        .filter(|page| {
            page.slug.to_lowercase().contains(&q)
                || page.title.to_lowercase().contains(&q)
                || page.description.to_lowercase().contains(&q)
                || page.content().to_lowercase().contains(&q)
        })
        .collect()
}
