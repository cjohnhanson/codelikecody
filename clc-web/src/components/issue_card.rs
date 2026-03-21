use leptos::prelude::*;
use leptos_router::components::A;

use crate::types::Issue;

fn priority_dot(p: &str) -> &'static str {
    match p {
        "1" => "bg-red-500",
        "2" => "bg-amber-500",
        "3" => "bg-blue-400",
        _ => "bg-zinc-300 dark:bg-zinc-600",
    }
}

fn relative_age(created: &Option<String>) -> String {
    // Simple relative time from ISO date string
    let Some(s) = created else { return String::new() };
    let clean = s.trim_matches('"');
    // Just show the date portion for now
    if let Some(date) = clean.split('T').next() {
        date.to_string()
    } else {
        String::new()
    }
}

#[component]
pub fn IssueCard(issue: Issue) -> impl IntoView {
    let labels = issue
        .frontmatter
        .labels
        .iter()
        .take(3) // max 3 labels on card
        .map(|l| {
            view! {
                <span class="px-1.5 py-px text-[10px] font-mono font-500 text-text-muted dark:text-zinc-500 bg-surface-raised dark:bg-dark-raised">
                    {l.clone()}
                </span>
            }
        })
        .collect_view();

    let extra_labels = issue.frontmatter.labels.len().saturating_sub(3);

    let priority = issue.frontmatter.priority.as_ref().map(|p| {
        let dot = priority_dot(p);
        let label = format!("P{p}");
        view! {
            <span class="inline-flex items-center gap-1 shrink-0" aria-label=format!("Priority {p}")>
                <span class=format!("w-1.5 h-1.5 rounded-full {dot}")></span>
                <span class="text-[10px] font-mono font-500 text-text-muted dark:text-zinc-500">{label}</span>
            </span>
        }
    });

    let assignee_initial = issue.frontmatter.assignee.as_ref().map(|a| {
        let initial = a.chars().next().unwrap_or('?').to_uppercase().to_string();
        view! {
            <span class="inline-flex items-center justify-center w-4 h-4 rounded-full bg-accent/10 dark:bg-blue-500/20 text-[9px] font-mono font-500 text-accent dark:text-blue-400" title=a.clone()>
                {initial}
            </span>
        }
    });

    let _date = relative_age(&issue.frontmatter.created);

    view! {
        <A href=format!("/issues/{}", issue.id) attr:class="block group">
            <article class="py-2 px-3 bg-surface-card dark:bg-dark-card border border-border-subtle dark:border-dark-border-subtle rounded hover:border-border dark:hover:border-dark-border transition-all duration-100">
                <h3 class="text-[13px] font-600 leading-snug text-text dark:text-zinc-200 group-hover:text-accent dark:group-hover:text-blue-400 transition-colors line-clamp-2">
                    {issue.frontmatter.title.clone()}
                </h3>
                <div class="mt-1.5 flex items-center gap-1.5">
                    {priority}
                    {assignee_initial}
                    <div class="flex items-center gap-1 ml-auto">
                        {labels}
                        {(extra_labels > 0).then(|| {
                            view! {
                                <span class="text-[9px] font-mono text-text-muted dark:text-zinc-600">
                                    "+"{extra_labels}
                                </span>
                            }
                        })}
                    </div>
                </div>
            </article>
        </A>
    }
}
