use leptos::prelude::*;
use leptos_router::components::A;

use crate::types::Issue;

#[component]
pub fn IssueCard(issue: Issue) -> impl IntoView {
    let labels = issue
        .frontmatter
        .labels
        .iter()
        .map(|l| {
            view! { <span class="label">{l.clone()}</span> }
        })
        .collect_view();

    let priority = issue
        .frontmatter
        .priority
        .as_ref()
        .map(|p| {
            view! { <span class="priority">"P" {p.clone()}</span> }
        });

    view! {
        <A href=format!("/issues/{}", issue.id)>
            <article class="issue-card">
                <h3>{issue.frontmatter.title.clone()}</h3>
                <div class="meta">
                    {priority}
                    <span class="id">{issue.id.clone()}</span>
                </div>
                <div class="labels">{labels}</div>
            </article>
        </A>
    }
}
