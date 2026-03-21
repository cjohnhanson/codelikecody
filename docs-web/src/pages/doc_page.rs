use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::content;

#[component]
pub fn DocPage() -> impl IntoView {
    let params = use_params_map();

    let page_html = move || {
        let params = params.read();
        // Build slug from path segments
        let slug = match (params.get("section"), params.get("page")) {
            (Some(section), Some(page)) => format!("{section}/{page}"),
            _ => String::new(),
        };

        let slug_ref = if slug.is_empty() { "" } else { slug.as_str() };

        match content::find_page(slug_ref) {
            Some(page) => {
                let html = page.render_html();
                view! {
                    <article>
                        <div class="doc-content" inner_html=html />
                    </article>
                }
                .into_any()
            }
            None => view! {
                <div class="py-20 text-center">
                    <h1 class="font-display text-4xl font-700 mb-4">"404"</h1>
                    <p class="text-text-secondary">"Page not found."</p>
                </div>
            }
            .into_any(),
        }
    };

    view! {
        {page_html}
    }
}

/// Landing page — renders the root doc (empty slug).
#[component]
pub fn LandingPage() -> impl IntoView {
    let page = content::find_page("");

    match page {
        Some(page) => {
            let html = page.render_html();
            view! {
                <article>
                    <div class="doc-content" inner_html=html />
                </article>
            }
            .into_any()
        }
        None => view! {
            <p>"No landing page found."</p>
        }
        .into_any(),
    }
}
