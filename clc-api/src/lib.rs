use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

pub mod error;
pub mod handlers;
pub mod types;

pub struct AppState {
    pub root: camino::Utf8PathBuf,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/projects", get(handlers::list_projects))
        .route(
            "/api/issues",
            get(handlers::list_issues).post(handlers::create_issue),
        )
        .route(
            "/api/issues/{id}",
            get(handlers::get_issue).patch(handlers::edit_issue),
        )
        .route("/api/issues/{id}/close", post(handlers::close_issue))
        .route("/api/issues/{id}/reopen", post(handlers::reopen_issue))
        .route("/api/search", get(handlers::search))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}
