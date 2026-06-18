use axum::Json;
use axum::extract::State;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{self, CurrentUser};
use crate::codes;
use crate::entities::users;
use crate::error::{AppError, AppResult};
use crate::services::{email, settings};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(me))
        .route("/email/send-code", post(email_send_code))
        .route("/email/bind", post(email_bind))
        .route("/password", post(set_password))
}

/// GET /api/me — current user profile.
async fn me(CurrentUser(user): CurrentUser) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(json!({
        "id": user.id,
        "nickname": user.nickname,
        "avatar": user.avatar,
        "email": user.email,
        "email_verified": user.email_verified,
        "has_password": user.password_hash.is_some(),
        "created_at": user.created_at,
    })))
}

#[derive(Deserialize)]
struct SendCodeReq {
    email: String,
}

/// POST /api/me/email/send-code — send a binding code to a candidate email.
async fn email_send_code(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
    Json(req): Json<SendCodeReq>,
) -> AppResult<Json<serde_json::Value>> {
    let email_addr = req.email.trim().to_lowercase();
    if !email_addr.contains('@') {
        return Err(AppError::BadRequest("邮箱格式不正确".into()));
    }
    // Reject if the email is already bound to a different account.
    if let Some(other) = users::Entity::find()
        .filter(users::Column::Email.eq(&email_addr))
        .one(&state.db)
        .await?
    {
        if other.id != _user.id {
            return Err(AppError::Conflict("该邮箱已被其他账号绑定".into()));
        }
    }

    let smtp: settings::SmtpSettings = settings::get(&state.db, settings::KEY_SMTP).await?;
    let code = codes::create_code(&state.db, &email_addr, "bind").await?;
    email::send_code_email(&smtp, &email_addr, &code, "绑定邮箱").await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct BindReq {
    email: String,
    code: String,
    #[serde(default)]
    password: Option<String>,
}

/// POST /api/me/email/bind — verify the code and bind the email (optionally set password).
async fn email_bind(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<BindReq>,
) -> AppResult<Json<serde_json::Value>> {
    let email_addr = req.email.trim().to_lowercase();
    if let Some(other) = users::Entity::find()
        .filter(users::Column::Email.eq(&email_addr))
        .one(&state.db)
        .await?
    {
        if other.id != user.id {
            return Err(AppError::Conflict("该邮箱已被其他账号绑定".into()));
        }
    }

    codes::verify_code(&state.db, &email_addr, "bind", &req.code).await?;

    let mut active: users::ActiveModel = user.into();
    active.email = Set(Some(email_addr));
    active.email_verified = Set(true);
    if let Some(pw) = req.password.as_deref() {
        if !pw.is_empty() {
            if pw.len() < 6 {
                return Err(AppError::BadRequest("密码至少 6 位".into()));
            }
            active.password_hash = Set(Some(auth::hash_password(pw)?));
        }
    }
    active.updated_at = Set(Utc::now());
    users::Entity::update(active).exec(&state.db).await?;

    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct PasswordReq {
    password: String,
}

/// POST /api/me/password — set or change the account password (requires bound email).
async fn set_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<PasswordReq>,
) -> AppResult<Json<serde_json::Value>> {
    if user.email.is_none() {
        return Err(AppError::BadRequest("请先绑定邮箱再设置密码".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少 6 位".into()));
    }
    let mut active: users::ActiveModel = user.into();
    active.password_hash = Set(Some(auth::hash_password(&req.password)?));
    active.updated_at = Set(Utc::now());
    users::Entity::update(active).exec(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}
