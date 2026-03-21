use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <div class="py-20 text-center">
            <h1 class="font-display text-6xl font-700 mb-4 text-text-muted">"404"</h1>
            <p class="text-text-secondary text-lg">"Page not found."</p>
        </div>
    }
}
