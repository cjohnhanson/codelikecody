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

/// Print a listing of all docs to stdout.
pub fn list() {
    println!("clc");
    for page in CLC_PAGES {
        println!("  {:<25} {}", page.title, page.description);
    }
    println!();
    println!("missouri");
    for page in missouri::docs::PAGES {
        println!("  {:<25} {}", page.title, page.description);
    }
    println!();
    println!("tisket");
    for page in tisket::docs::PAGES {
        println!("  {:<25} {}", page.title, page.description);
    }
    println!();
    println!("almanac");
    for page in almanac::docs::PAGES {
        println!("  {:<25} {}", page.title, page.description);
    }
}

/// Print a doc by slug. Searches clc docs first, then missouri, then tisket.
/// Returns false if not found.
pub fn show(slug: &str) -> bool {
    for page in CLC_PAGES {
        if page.slug == slug {
            print!("{}", page.content());
            return true;
        }
    }
    if missouri::docs::show(slug) {
        return true;
    }
    if tisket::docs::show(slug) {
        return true;
    }
    almanac::docs::show(slug)
}

/// Search docs for a query string across all tools.
pub fn search(query: &str) {
    let q = query.to_lowercase();
    for page in CLC_PAGES {
        if page.title.to_lowercase().contains(&q)
            || page.description.to_lowercase().contains(&q)
            || page.content().to_lowercase().contains(&q)
        {
            println!("{:<25} {}", page.title, page.description);
        }
    }
    missouri::docs::search(query);
    tisket::docs::search(query);
    almanac::docs::search(query);
}
