use leptos::prelude::*;
use leptos_router::components::A;

use crate::content;

#[component]
pub fn Header() -> impl IntoView {
    let toggle_dark = move |_| {
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.document_element() {
                let _ = el.class_list().toggle("dark");
            }
        }
    };

    view! {
        <header class="header-bar md:ml-60">
            <div class="px-8 py-3 flex items-baseline justify-between pl-14 md:pl-8">
                <span class="font-mono text-[11px] text-text-muted">"docs"</span>
                <button
                    on:click=toggle_dark
                    aria-label="Toggle dark mode"
                    class="px-2 py-1 text-[11px] font-mono text-text-muted hover:text-text hover:bg-surface-raised rounded transition-all cursor-pointer"
                >
                    "◐"
                </button>
            </div>
        </header>
    }
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);
    let toggle_menu = move |_| set_menu_open.update(|v| *v = !*v);

    view! {
        // Mobile hamburger
        <button
            class="fixed top-3 left-3 z-50 p-2 rounded bg-surface-raised border border-border md:hidden"
            on:click=toggle_menu
        >
            <span class="font-mono text-sm">"☰"</span>
        </button>

        <nav class={move || {
            let base = "sidebar md:translate-x-0";
            if menu_open.get() {
                format!("{base} translate-x-0")
            } else {
                format!("{base} -translate-x-full")
            }
        }}>
            <div class="px-5 py-5">
                <A href="/" attr:class="block mb-1 no-underline">
                    <span class="sidebar-brand">"codelikecody"</span>
                </A>

                // Top-level pages
                <div class="mt-4">
                    {content::top_level_pages().into_iter().map(|page| {
                        view! {
                            <A
                                href={format!("/{}", page.slug)}
                                attr:class="sidebar-link"
                            >
                                {page.title}
                            </A>
                        }
                    }).collect_view()}
                </div>

                // Sections
                {["clc", "missouri", "tisket"].into_iter().map(|section| {
                    let pages = content::section_pages(section);
                    view! {
                        <div>
                            <p class="sidebar-section-label">{section}</p>
                            {pages.into_iter().map(|page| {
                                view! {
                                    <A
                                        href={format!("/{}", page.slug)}
                                        attr:class="sidebar-link"
                                    >
                                        {page.title}
                                    </A>
                                }
                            }).collect_view()}
                        </div>
                    }
                }).collect_view()}
            </div>
        </nav>
    }
}
