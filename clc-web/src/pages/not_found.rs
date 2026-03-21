use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-32">
            <p class="font-display text-7xl font-900 text-surface-raised dark:text-zinc-800 mb-3">"404"</p>
            <p class="text-sm text-text-muted dark:text-zinc-500 mb-6">"Nothing here."</p>
            <A href="/" attr:class="text-[12px] font-mono text-accent dark:text-blue-400 hover:text-accent-hover dark:hover:text-blue-300 transition-colors">
                "← Back"
            </A>
        </div>
    }
}
