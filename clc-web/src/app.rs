use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::nav::Nav;
use crate::pages::board::Board;
use crate::pages::issue_detail::IssueDetail;
use crate::pages::not_found::NotFound;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="min-h-screen bg-surface dark:bg-dark-bg transition-colors duration-200">
                <a href="#main" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-3 focus:py-1.5 focus:text-sm focus:bg-accent focus:text-white focus:rounded">"Skip to content"</a>
                <Nav />
                <main id="main" role="main">
                    <Routes fallback=NotFound>
                        <Route path=path!("/") view=Board />
                        <Route path=path!("/issues/:id") view=IssueDetail />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
