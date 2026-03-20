use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;

#[component]
pub fn IssueDetail() -> impl IntoView {
    let params = use_params_map();

    let issue = LocalResource::new(move || {
        let id = params.read().get("id");
        async move {
            match id {
                Some(id) => api::get_issue(&id).await.ok(),
                None => None,
            }
        }
    });

    view! {
        <Suspense fallback=move || view! { <p>"Loading..."</p> }>
            {move || {
                issue.get().map(|data| match data {
                    Some(issue) => {
                        let labels = issue
                            .frontmatter
                            .labels
                            .iter()
                            .map(|l| view! { <span class="label">{l.clone()}</span> })
                            .collect_view();

                        let deps = issue
                            .frontmatter
                            .depends_on
                            .iter()
                            .map(|d| view! { <li>{d.clone()}</li> })
                            .collect_view();

                        view! {
                            <article class="issue-detail">
                                <h1>{issue.frontmatter.title.clone()}</h1>
                                <dl>
                                    <dt>"Status"</dt>
                                    <dd>{issue.frontmatter.status.clone()}</dd>
                                    <dt>"Project"</dt>
                                    <dd>{issue.project.clone()}</dd>
                                    <dt>"Priority"</dt>
                                    <dd>{issue.frontmatter.priority.clone().unwrap_or_default()}</dd>
                                    <dt>"Assignee"</dt>
                                    <dd>{issue.frontmatter.assignee.clone().unwrap_or_default()}</dd>
                                </dl>
                                <div class="labels">{labels}</div>
                                {(!issue.frontmatter.depends_on.is_empty()).then(|| {
                                    view! {
                                        <section>
                                            <h2>"Dependencies"</h2>
                                            <ul>{deps}</ul>
                                        </section>
                                    }
                                })}
                                {(!issue.body.is_empty()).then(|| {
                                    view! {
                                        <section>
                                            <h2>"Description"</h2>
                                            <pre>{issue.body.clone()}</pre>
                                        </section>
                                    }
                                })}
                            </article>
                        }
                            .into_any()
                    }
                    None => view! { <p>"Issue not found."</p> }.into_any(),
                })
            }}
        </Suspense>
    }
}
