use leptos::prelude::*;

use crate::api;
use crate::components::issue_card::IssueCard;
use crate::types::Issue;

const INITIAL_SHOW: usize = 6;

fn status_border(status: &str) -> &'static str {
    match status {
        "todo" => "border-l-status-todo",
        "in_progress" => "border-l-status-in-progress",
        "blocked" => "border-l-status-blocked",
        "discovery" => "border-l-status-discovery",
        "paused" => "border-l-status-paused",
        _ => "border-l-zinc-400",
    }
}

fn status_accent(status: &str) -> &'static str {
    match status {
        "todo" => "bg-status-todo",
        "in_progress" => "bg-status-in-progress",
        "blocked" => "bg-status-blocked",
        "discovery" => "bg-status-discovery",
        "paused" => "bg-status-paused",
        _ => "bg-zinc-400",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "todo" => "Todo",
        "in_progress" => "In Progress",
        "blocked" => "Blocked",
        "discovery" => "Discovery",
        "paused" => "Paused",
        _ => "Other",
    }
}

fn group_by_status(issues: &[Issue]) -> Vec<(&'static str, Vec<Issue>)> {
    // Active work first, then ready, then intake/blocked/paused
    let columns = ["in_progress", "todo", "blocked", "discovery", "paused"];
    columns
        .iter()
        .map(|&status| {
            let matching: Vec<Issue> = issues
                .iter()
                .filter(|i| i.frontmatter.status == status)
                .cloned()
                .collect();
            (status, matching)
        })
        .filter(|(_, issues)| !issues.is_empty())
        .collect()
}

#[component]
fn StatusColumn(status: &'static str, issues: Vec<Issue>) -> impl IntoView {
    let count = issues.len();
    let border = status_border(status);
    let _accent = status_accent(status);
    let label = status_label(status);
    let needs_expand = count > INITIAL_SHOW;

    let (expanded, set_expanded) = signal(false);

    view! {
        <section class=format!("border-l-[3px] {border} dark:opacity-90 pl-4 flex flex-col")>
            <div class="flex items-center gap-2 mb-3 shrink-0">
                <h2 class="font-display text-[15px] font-600 text-text dark:text-zinc-200">{label}</h2>
                <span class="inline-flex items-center justify-center min-w-[22px] h-[22px] px-1.5 text-[11px] font-mono font-500 text-text-secondary dark:text-zinc-400 bg-surface-raised dark:bg-zinc-800 rounded">
                    {count}
                </span>
            </div>
            <div class=move || {
                let base = "flex flex-col gap-2 overflow-y-auto";
                if expanded.get() && needs_expand {
                    format!("{base} max-h-[calc(100vh-180px)]")
                } else {
                    base.to_string()
                }
            }>
                {move || {
                    let limit = if expanded.get() || !needs_expand { count } else { INITIAL_SHOW };
                    issues[..limit]
                        .iter()
                        .map(|issue| view! { <IssueCard issue=issue.clone() /> })
                        .collect_view()
                }}
            </div>
            {needs_expand.then(|| {
                let remaining = count - INITIAL_SHOW;
                view! {
                    <button
                        class="mt-2 shrink-0 w-full py-1.5 text-[11px] font-mono text-text-muted dark:text-zinc-500 hover:text-accent dark:hover:text-blue-400 bg-surface-raised/50 dark:bg-zinc-800/50 hover:bg-surface-raised dark:hover:bg-zinc-800 rounded border border-transparent hover:border-border-subtle dark:hover:border-zinc-700 transition-all cursor-pointer"
                        on:click=move |_| set_expanded.update(|v| *v = !*v)
                    >
                        {move || {
                            if expanded.get() {
                                "Collapse".to_string()
                            } else {
                                format!("Show {remaining} more")
                            }
                        }}
                    </button>
                }
            })}
        </section>
    }
}

#[component]
pub fn Board() -> impl IntoView {
    let issues = LocalResource::new(move || async move {
        api::list_issues(None, None, false).await.unwrap_or_default()
    });

    view! {
        <div class="max-w-screen-2xl mx-auto px-8 py-8">
            <div class="mb-6">
                <h1 class="font-display text-[26px] font-700 text-text dark:text-zinc-50 tracking-tight">"Issues"</h1>
            </div>
            <Suspense fallback=move || {
                view! {
                    <p class="font-mono text-sm text-text-muted animate-pulse">"Loading..."</p>
                }
            }>
                {move || {
                    issues.get().map(|data| {
                        let total = data.len();
                        let columns = group_by_status(&data);
                        view! {
                            <p class="text-[11px] font-mono text-text-muted mb-6">
                                {total} " open"
                            </p>
                            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8 items-start">
                                {columns
                                    .into_iter()
                                    .map(|(status, issues)| {
                                        view! { <StatusColumn status=status issues=issues /> }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
