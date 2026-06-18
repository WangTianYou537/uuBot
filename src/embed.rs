use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// The compiled frontend bundle, embedded into the binary at build time.
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

/// Serve embedded static files, falling back to `index.html` for SPA routes.
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response();
    }

    // SPA fallback: serve index.html so client-side routing works on refresh.
    match Assets::get("index.html") {
        Some(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
