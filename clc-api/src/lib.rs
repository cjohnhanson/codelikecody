use std::sync::Arc;

use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

pub mod error;
pub mod handlers;
pub mod types;

pub struct AppState {
    pub root: camino::Utf8PathBuf,
}

fn api_routes() -> Router<Arc<AppState>> {
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
}

pub fn router(state: AppState) -> Router {
    api_routes()
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}

pub fn router_with_static(state: AppState, static_dir: &str) -> Router {
    // Serve static files for known extensions (JS, WASM, CSS, etc.)
    // For everything else, serve index.html so the client-side router handles it.
    let index_html: &'static str = Box::leak(
        std::fs::read_to_string(std::path::PathBuf::from(static_dir).join("index.html"))
            .unwrap_or_else(|e| panic!("failed to read {static_dir}/index.html: {e}"))
            .into_boxed_str(),
    );

    let static_dir_owned = static_dir.to_string();
    let spa_fallback = move |uri: axum::http::Uri| {
        let static_dir = static_dir_owned.clone();
        let path = uri.path().trim_start_matches('/').to_string();
        async move {
            let file_path = std::path::PathBuf::from(&static_dir).join(&path);
            if file_path.exists() && file_path.is_file() {
                let content = tokio::fs::read(&file_path).await.unwrap_or_default();
                let mime = if path.ends_with(".js") {
                    "application/javascript"
                } else if path.ends_with(".wasm") {
                    "application/wasm"
                } else if path.ends_with(".css") {
                    "text/css"
                } else {
                    "application/octet-stream"
                };
                ([(axum::http::header::CONTENT_TYPE, mime)], content).into_response()
            } else {
                axum::response::Html(index_html).into_response()
            }
        }
    };

    api_routes()
        .fallback(spa_fallback)
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state))
}
