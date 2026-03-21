use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use pulldown_cmark::{Options, Parser};

use crate::api;

fn render_markdown(input: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(input, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

fn status_pill(status: &str) -> (&'static str, &'static str) {
    match status {
        "todo" => ("Todo", "bg-blue-500/10 text-blue-700 dark:bg-blue-500/20 dark:text-blue-400"),
        "in_progress" => ("In Progress", "bg-amber-500/10 text-amber-700 dark:bg-amber-500/20 dark:text-amber-400"),
        "blocked" => ("Blocked", "bg-red-500/10 text-red-700 dark:bg-red-500/20 dark:text-red-400"),
        "discovery" => ("Discovery", "bg-violet-500/10 text-violet-700 dark:bg-violet-500/20 dark:text-violet-400"),
        "paused" => ("Paused", "bg-zinc-500/10 text-zinc-600 dark:bg-zinc-500/20 dark:text-zinc-400"),
        "done" => ("Done", "bg-green-500/10 text-green-700 dark:bg-green-500/20 dark:text-green-400"),
        _ => ("Unknown", "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400"),
    }
}

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
        <div class="max-w-2xl mx-auto px-8 py-8">
            <Suspense fallback=move || {
                view! { <p class="font-mono text-sm text-text-muted animate-pulse">"Loading..."</p> }
            }>
                {move || {
                    issue.get().map(|data| match data {
                        Some(issue) => {
                            let (status_text, status_cls) = status_pill(&issue.frontmatter.status);

                            let labels = issue
                                .frontmatter
                                .labels
                                .iter()
                                .map(|l| {
                                    view! {
                                        <span class="inline-block px-1.5 py-0.5 text-[10px] font-mono font-500 text-text-muted dark:text-zinc-500 bg-surface-raised dark:bg-dark-raised">
                                            {l.clone()}
                                        </span>
                                    }
                                })
                                .collect_view();

                            let deps_list = issue.frontmatter.depends_on.clone();
                            let deps = deps_list
                                .into_iter()
                                .map(|d| {
                                    view! {
                                        <li>
                                            <A href=format!("/issues/{d}") attr:class="font-mono text-sm text-accent dark:text-blue-400 hover:text-accent-hover dark:hover:text-blue-300 underline decoration-accent/30 dark:decoration-blue-400/30">
                                                {d.clone()}
                                            </A>
                                        </li>
                                    }
                                })
                                .collect_view();

                            view! {
                                <A href="/" attr:class="inline-block mb-6 text-[12px] font-mono text-text-muted dark:text-zinc-500 hover:text-text dark:hover:text-zinc-300 transition-colors">
                                    "← Back"
                                </A>

                                <article>
                                    <div class="flex items-center gap-3 mb-3">
                                        <span class=format!("px-2.5 py-1 text-[11px] font-mono font-500 rounded-full {status_cls}")>
                                            {status_text}
                                        </span>
                                        {issue.frontmatter.priority.as_ref().map(|p| {
                                            let dot_color = match p.as_str() {
                                                "1" => "bg-red-500",
                                                "2" => "bg-amber-500",
                                                "3" => "bg-blue-400",
                                                _ => "bg-zinc-400",
                                            };
                                            let p = p.clone();
                                            view! {
                                                <span class="inline-flex items-center gap-1.5">
                                                    <span class=format!("w-2 h-2 rounded-full {dot_color}")></span>
                                                    <span class="text-[12px] font-mono text-text-muted dark:text-zinc-500">"P"{p}</span>
                                                </span>
                                            }
                                        })}
                                    </div>

                                    <h1 class="font-display text-[26px] font-700 leading-tight text-text dark:text-zinc-50 mb-1.5 tracking-tight">
                                        {issue.frontmatter.title.clone()}
                                    </h1>
                                    <p class="font-mono text-[10px] text-text-muted/50 dark:text-zinc-700 mb-8">
                                        {issue.project.clone()}" / "{issue.id.clone()}
                                    </p>

                                    {(issue.frontmatter.assignee.is_some() || issue.frontmatter.due_date.is_some()).then(|| {
                                        let assignee = issue.frontmatter.assignee.clone();
                                        let due = issue.frontmatter.due_date.clone();
                                        view! {
                                            <div class="flex items-center gap-6 mb-8 py-3 border-y border-border dark:border-dark-border text-sm">
                                                {assignee.map(|a| view! {
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-[10px] font-mono text-text-muted dark:text-zinc-600 uppercase tracking-widest">"Assignee"</span>
                                                        <span class="text-text dark:text-zinc-300">{a}</span>
                                                    </div>
                                                })}
                                                {due.map(|d| view! {
                                                    <div class="flex items-center gap-2">
                                                        <span class="text-[10px] font-mono text-text-muted dark:text-zinc-600 uppercase tracking-widest">"Due"</span>
                                                        <span class="text-text dark:text-zinc-300">{d}</span>
                                                    </div>
                                                })}
                                            </div>
                                        }
                                    })}

                                    {(!issue.frontmatter.labels.is_empty()).then(|| {
                                        view! {
                                            <div class="flex flex-wrap gap-1.5 mb-8">{labels}</div>
                                        }
                                    })}

                                    {(!issue.frontmatter.depends_on.is_empty()).then(|| {
                                        view! {
                                            <section class="mb-8">
                                                <h2 class="font-display text-base font-500 text-text dark:text-zinc-100 mb-3">"Dependencies"</h2>
                                                <ul class="space-y-1.5 pl-1">{deps}</ul>
                                            </section>
                                        }
                                    })}

                                    {(!issue.body.is_empty()).then(|| {
                                        let html = render_markdown(&issue.body);
                                        view! {
                                            <section>
                                                <h2 class="text-[10px] font-mono text-text-muted dark:text-zinc-600 uppercase tracking-widest mb-4">"Description"</h2>
                                                <div
                                                    class="prose prose-sm dark:prose-invert max-w-none text-text-secondary dark:text-zinc-400"
                                                    inner_html=html
                                                />
                                            </section>
                                        }
                                    })}
                                </article>
                            }
                                .into_any()
                        }
                        None => {
                            view! {
                                <div class="py-24 text-center">
                                    <p class="font-display text-5xl font-900 text-surface-raised dark:text-zinc-800 mb-3">"404"</p>
                                    <p class="text-sm text-text-muted dark:text-zinc-500 mb-6">"This issue doesn't exist."</p>
                                    <A href="/" attr:class="text-[12px] font-mono text-accent dark:text-blue-400 hover:text-accent-hover dark:hover:text-blue-300">"← Back"</A>
                                </div>
                            }
                                .into_any()
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
