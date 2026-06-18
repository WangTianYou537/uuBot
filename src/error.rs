use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Unified application error type. Converts into a JSON `{ "error": "..." }`
/// response with an appropriate status code.
#[derive(Debug)]
pub enum AppError {
    /// 400 — invalid input from the client.
    BadRequest(String),
    /// 401 — not authenticated.
    Unauthorized(String),
    /// 403 — authenticated but not allowed.
    Forbidden(String),
    /// 404 — resource not found.
    NotFound(String),
    /// 409 — conflict (e.g. unique constraint).
    Conflict(String),
    /// 500 — unexpected internal error.
    Internal(String),
}

impl AppError {
    fn parts(&self) -> (StatusCode, &str) {
        match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (status, msg) = self.parts();
        write!(f, "{status}: {msg}")
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = self.parts();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("internal error: {msg}");
        }
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

// Convenient conversions so handlers can use `?`.
impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::Internal(format!("database error: {e}"))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
