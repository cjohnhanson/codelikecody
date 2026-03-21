use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::components::nav::{Header, Sidebar};
use crate::pages::doc_page::{DocPage, LandingPage};
use crate::pages::not_found::NotFound;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Sidebar />
            <Header />

            <main class="md:ml-60 px-8 py-8 max-w-3xl">
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=LandingPage />
                    <Route path=path!("/:page") view=DocPage />
                    <Route path=path!("/:section/:page") view=DocPage />
                </Routes>
            </main>
        </Router>
    }
}
