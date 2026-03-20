use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Nav() -> impl IntoView {
    view! {
        <header>
            <nav>
                <A href="/">
                    <strong>"tisket"</strong>
                </A>
            </nav>
        </header>
    }
}
