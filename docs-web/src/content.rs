//! Markdown content baked into the binary at compile time.

use pulldown_cmark::{Options, Parser, html};

/// A documentation page with its metadata and raw markdown.
pub struct DocPage {
    pub slug: &'static str,
    pub title: &'static str,
    pub raw: &'static str,
    pub section: Option<&'static str>,
}

impl DocPage {
    /// Get markdown content with the metadata comment stripped.
    fn markdown(&self) -> &str {
        let md = self.raw;
        if let Some(start) = md.find("<!-- metadata") {
            if let Some(end) = md[start..].find("-->") {
                return md[start + end + 3..].trim_start_matches('\n');
            }
        }
        md
    }

    /// Render markdown to HTML.
    pub fn render_html(&self) -> String {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS;
        let parser = Parser::new_ext(self.markdown(), options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }
}

/// All documentation pages, baked in from docs/ at compile time.
pub static PAGES: &[DocPage] = &[
    DocPage {
        slug: "",
        title: "codelikecody",
        raw: include_str!("../../docs/index.md"),
        section: None,
    },
    DocPage {
        slug: "what-is-codelikecody",
        title: "What is codelikecody?",
        raw: include_str!("../../docs/what-is-codelikecody.md"),
        section: None,
    },
    DocPage {
        slug: "getting-started",
        title: "Getting Started",
        raw: include_str!("../../docs/getting-started.md"),
        section: None,
    },
    DocPage {
        slug: "clc/phase-system",
        title: "The Phase System",
        raw: include_str!("../../docs/clc/phase-system.md"),
        section: Some("clc"),
    },
    DocPage {
        slug: "clc/orchestration",
        title: "Multi-Agent Orchestration",
        raw: include_str!("../../docs/clc/orchestration.md"),
        section: Some("clc"),
    },
    DocPage {
        slug: "clc/cli-reference",
        title: "clc CLI Reference",
        raw: include_str!("../../docs/clc/cli-reference.md"),
        section: Some("clc"),
    },
    DocPage {
        slug: "missouri/getting-started",
        title: "Getting Started with Missouri",
        raw: include_str!("../../docs/missouri/getting-started.md"),
        section: Some("missouri"),
    },
    DocPage {
        slug: "missouri/writing-tests",
        title: "Writing Tests",
        raw: include_str!("../../docs/missouri/writing-tests.md"),
        section: Some("missouri"),
    },
    DocPage {
        slug: "missouri/cli-reference",
        title: "missouri CLI Reference",
        raw: include_str!("../../docs/missouri/cli-reference.md"),
        section: Some("missouri"),
    },
    DocPage {
        slug: "tisket/workflow",
        title: "Workflow",
        raw: include_str!("../../docs/tisket/workflow.md"),
        section: Some("tisket"),
    },
    DocPage {
        slug: "tisket/cli-reference",
        title: "tisket CLI Reference",
        raw: include_str!("../../docs/tisket/cli-reference.md"),
        section: Some("tisket"),
    },
];

/// Find a page by its slug.
pub fn find_page(slug: &str) -> Option<&'static DocPage> {
    PAGES.iter().find(|p| p.slug == slug)
}

/// Get all pages in a section.
pub fn section_pages(section: &str) -> Vec<&'static DocPage> {
    PAGES.iter().filter(|p| p.section == Some(section)).collect()
}

/// Get top-level pages (no section).
pub fn top_level_pages() -> Vec<&'static DocPage> {
    PAGES
        .iter()
        .filter(|p| p.section.is_none() && !p.slug.is_empty())
        .collect()
}
