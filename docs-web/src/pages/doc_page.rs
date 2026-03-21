use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use web_sys::wasm_bindgen::JsCast;

use crate::content;

/// Intercept clicks on internal links within rendered markdown content
/// and route them through the SPA router instead of triggering full
/// page navigation.
fn setup_link_interceptor(node_ref: NodeRef<leptos::html::Div>) {
    let navigate = use_navigate();

    Effect::new(move |_| {
        let Some(el) = node_ref.get() else { return };
        let navigate = navigate.clone();

        let closure =
            wasm_bindgen::closure::Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
                let Some(target) = ev.target() else { return };
                // Walk up from the click target to find an <a> element
                let mut node: Option<web_sys::Element> = target.dyn_into::<web_sys::Element>().ok();
                while let Some(el) = node {
                    if el.tag_name() == "A" {
                        if let Some(href) = el.get_attribute("href") {
                            // Only intercept internal links (starting with /)
                            if href.starts_with('/') {
                                ev.prevent_default();
                                navigate(&href, Default::default());
                                return;
                            }
                        }
                        return;
                    }
                    node = el.parent_element();
                }
            });

        let _ = el.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget(); // Leak the closure — it lives as long as the element
    });
}

/// Render a doc page by slug from the route params.
#[component]
pub fn DocPage() -> impl IntoView {
    let params = use_params_map();
    let content_ref = NodeRef::<leptos::html::Div>::new();
    setup_link_interceptor(content_ref);

    let page_html = move || {
        let params = params.read();
        let slug = match (params.get("section"), params.get("page")) {
            (Some(section), Some(page)) => format!("{section}/{page}"),
            (None, Some(page)) => page.to_string(),
            (Some(section), None) => section.to_string(),
            (None, None) => String::new(),
        };

        let slug_ref = if slug.is_empty() { "" } else { slug.as_str() };

        match content::find_page(slug_ref) {
            Some(page) => {
                let html = page.render_html();
                view! {
                    <article>
                        <div class="doc-content" node_ref=content_ref inner_html=html />
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
    let content_ref = NodeRef::<leptos::html::Div>::new();
    setup_link_interceptor(content_ref);

    match page {
        Some(page) => {
            let html = page.render_html();
            view! {
                <article>
                    <div class="doc-content" node_ref=content_ref inner_html=html />
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
