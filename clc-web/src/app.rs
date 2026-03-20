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
            <Nav />
            <main>
                <Routes fallback=NotFound>
                    <Route path=path!("/") view=Board />
                    <Route path=path!("/issues/:id") view=IssueDetail />
                </Routes>
            </main>
        </Router>
    }
}
