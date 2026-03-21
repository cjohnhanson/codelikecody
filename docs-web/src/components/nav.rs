use leptos::prelude::*;
use leptos_router::components::A;

use crate::content;

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

        // Sidebar
        <nav class={move || {
            let base = "fixed top-0 left-0 h-full w-64 bg-surface-card border-r border-border overflow-y-auto z-40 transition-transform duration-200 md:translate-x-0";
            if menu_open.get() {
                format!("{base} translate-x-0")
            } else {
                format!("{base} -translate-x-full")
            }
        }}>
            <div class="p-6">
                // Title
                <A href="/" attr:class="block mb-6 no-underline">
                    <span class="font-display text-xl font-700 text-text">"codelikecody"</span>
                </A>

                // Top-level pages
                <ul class="list-none pl-0 mb-6 space-y-1">
                    {content::top_level_pages().into_iter().map(|page| {
                        view! {
                            <li>
                                <A
                                    href={format!("/{}", page.slug)}
                                    attr:class="block py-1 font-mono text-sm text-text-secondary hover:text-accent no-underline"
                                >
                                    {page.title}
                                </A>
                            </li>
                        }
                    }).collect_view()}
                </ul>

                // Sections
                {["clc", "missouri", "tisket"].into_iter().map(|section| {
                    let pages = content::section_pages(section);
                    view! {
                        <div class="mb-6">
                            <h3 class="font-mono text-xs font-500 uppercase tracking-wider text-text-muted mb-2">
                                {section}
                            </h3>
                            <ul class="list-none pl-0 space-y-1">
                                {pages.into_iter().map(|page| {
                                    view! {
                                        <li>
                                            <A
                                                href={format!("/{}", page.slug)}
                                                attr:class="block py-1 font-mono text-sm text-text-secondary hover:text-accent no-underline"
                                            >
                                                {page.title}
                                            </A>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                        </div>
                    }
                }).collect_view()}
            </div>
        </nav>
    }
}
