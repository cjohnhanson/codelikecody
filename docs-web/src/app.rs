use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::components::nav::Sidebar;
use crate::pages::doc_page::{DocPage, LandingPage};
use crate::pages::not_found::NotFound;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Sidebar />

            // Main content area — offset by sidebar width on desktop
            <main class="md:ml-64 min-h-screen p-8 max-w-4xl">
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=LandingPage />
                    <Route path=path!("/:page") view=DocPage />
                    <Route path=path!("/:section/:page") view=DocPage />
                </Routes>
            </main>
        </Router>
    }
}
