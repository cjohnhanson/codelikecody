use leptos::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <h1>"404"</h1>
        <p>"Page not found."</p>
    }
}
