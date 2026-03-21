use leptos::prelude::*;
use leptos_router::components::A;

fn is_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("theme").ok().flatten())
        .is_some_and(|v| v == "dark")
}

fn set_dark(dark: bool) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = doc.document_element() {
            if dark {
                let _ = el.class_list().add_1("dark");
            } else {
                let _ = el.class_list().remove_1("dark");
            }
        }
    }
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
    {
        if dark {
            let _ = storage.set_item("theme", "dark");
        } else {
            let _ = storage.remove_item("theme");
        }
    }
}

#[component]
pub fn Nav() -> impl IntoView {
    // Restore theme on mount
    if is_dark() {
        set_dark(true);
    }

    let toggle_dark = move |_| {
        let currently_dark = is_dark();
        set_dark(!currently_dark);
    };

    view! {
        <header class="border-b border-border dark:border-dark-border bg-surface dark:bg-dark-card">
            <nav role="navigation" aria-label="Main" class="max-w-screen-2xl mx-auto px-8 py-3.5 flex items-baseline justify-between">
                <A href="/" attr:class="group">
                    <span class="font-display text-xl font-700 tracking-tight text-text dark:text-zinc-100">"tisket"</span>
                </A>
                <button
                    on:click=toggle_dark
                    aria-label="Toggle dark mode"
                    class="px-2 py-1 text-[11px] font-mono text-text-muted dark:text-zinc-500 hover:text-text dark:hover:text-zinc-300 hover:bg-surface-raised dark:hover:bg-dark-raised rounded transition-all cursor-pointer focus:outline-2 focus:outline-accent dark:focus:outline-blue-400 focus:outline-offset-2"
                >
                    "◐"
                </button>
            </nav>
        </header>
    }
}
