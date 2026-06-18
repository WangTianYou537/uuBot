pub mod admin;
pub mod auth;
pub mod bot;
pub mod me;
pub mod words;

use axum::Router;
use tower_http::trace::TraceLayer;

use crate::embed::static_handler;
use crate::state::AppState;

/// Build the full application router: `/api/*` handlers plus the embedded SPA.
pub fn app(state: AppState) -> Router {
    let api = Router::new()
        .nest("/auth", auth::router())
        .nest("/bot", bot::router())
        .nest("/me", me::router())
        .nest("/words", words::router())
        .nest("/admin", admin::router());

    Router::new()
        .nest("/api", api)
        .fallback(static_handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
