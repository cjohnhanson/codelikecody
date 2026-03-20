use leptos::prelude::*;

use crate::api;
use crate::components::issue_card::IssueCard;
use crate::types::Issue;

fn group_by_status(issues: &[Issue]) -> Vec<(&'static str, Vec<Issue>)> {
    let columns = ["todo", "in_progress", "blocked", "discovery", "paused"];
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
pub fn Board() -> impl IntoView {
    let issues = LocalResource::new(move || async move {
        api::list_issues(None, None, false).await.unwrap_or_default()
    });

    view! {
        <h1>"Board"</h1>
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || {
                issues.get().map(|data| {
                    let columns = group_by_status(&data);
                    view! {
                        <div class="board">
                            {columns
                                .into_iter()
                                .map(|(status, issues)| {
                                    view! {
                                        <section class="column">
                                            <h2>{status}</h2>
                                            {issues
                                                .into_iter()
                                                .map(|issue| view! { <IssueCard issue=issue /> })
                                                .collect_view()}
                                        </section>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })
            }}
        </Suspense>
    }
}
