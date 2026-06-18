use axum::Json;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::CookieJar;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{self, CurrentAdmin};
use crate::entities::{admins, bot_messages, users, words, wx_bindings};
use crate::error::{AppError, AppResult};
use crate::services::{email, settings};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/users", get(list_users))
        .route("/stats", get(stats))
        .route("/test-smtp", post(test_smtp))
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

/// POST /api/admin/login
async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginReq>,
) -> AppResult<impl IntoResponse> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Username.eq(&req.username))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("用户名或密码不正确".into()))?;

    if !auth::verify_password(&req.password, &admin.password_hash) {
        return Err(AppError::Unauthorized("用户名或密码不正确".into()));
    }

    let token = auth::sign(&state, admin.id, "admin")?;
    let jar = jar.add(auth::session_cookie(&state, token));
    Ok((jar, Json(json!({ "ok": true }))))
}

/// POST /api/admin/logout
async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let jar = jar.add(auth::clear_cookie(&state));
    (jar, Json(json!({ "ok": true })))
}

/// GET /api/admin/me
async fn me(CurrentAdmin(admin): CurrentAdmin) -> Json<serde_json::Value> {
    Json(json!({ "id": admin.id, "username": admin.username }))
}

#[derive(Serialize)]
struct SettingsResp {
    smtp: settings::SmtpSettings,
    oauth: settings::OAuthSettings,
    dictionary: settings::DictionarySettings,
    ai: settings::AiSettings,
    bot: settings::BotSettings,
}

/// GET /api/admin/settings
async fn get_settings(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<SettingsResp>> {
    Ok(Json(SettingsResp {
        smtp: settings::get(&state.db, settings::KEY_SMTP).await?,
        oauth: settings::get(&state.db, settings::KEY_OAUTH).await?,
        dictionary: settings::get(&state.db, settings::KEY_DICTIONARY).await?,
        ai: settings::get(&state.db, settings::KEY_AI).await?,
        bot: settings::get(&state.db, settings::KEY_BOT).await?,
    }))
}

#[derive(Deserialize)]
struct SettingsUpdate {
    smtp: Option<settings::SmtpSettings>,
    oauth: Option<settings::OAuthSettings>,
    dictionary: Option<settings::DictionarySettings>,
    ai: Option<settings::AiSettings>,
    bot: Option<settings::BotSettings>,
}

/// PUT /api/admin/settings
async fn put_settings(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(req): Json<SettingsUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(smtp) = req.smtp {
        settings::set(&state.db, settings::KEY_SMTP, &smtp).await?;
    }
    if let Some(oauth) = req.oauth {
        settings::set(&state.db, settings::KEY_OAUTH, &oauth).await?;
    }
    if let Some(dict) = req.dictionary {
        settings::set(&state.db, settings::KEY_DICTIONARY, &dict).await?;
    }
    if let Some(ai) = req.ai {
        settings::set(&state.db, settings::KEY_AI, &ai).await?;
    }
    if let Some(bot) = req.bot {
        settings::set(&state.db, settings::KEY_BOT, &bot).await?;
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct UsersQuery {
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    page_size: Option<u64>,
}

/// GET /api/admin/users
async fn list_users(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(q): Query<UsersQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

    let paginator = users::Entity::find()
        .order_by_desc(users::Column::CreatedAt)
        .paginate(&state.db, page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    })))
}

/// GET /api/admin/stats
async fn stats(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<serde_json::Value>> {
    let user_count = users::Entity::find().count(&state.db).await?;
    let word_count = words::Entity::find().count(&state.db).await?;
    let bot_binding_count = wx_bindings::Entity::find().count(&state.db).await?;
    let bot_message_count = bot_messages::Entity::find().count(&state.db).await?;
    Ok(Json(json!({
        "users": user_count,
        "words": word_count,
        "bot_bindings": bot_binding_count,
        "bot_messages": bot_message_count,
    })))
}

#[derive(Deserialize)]
struct TestSmtpReq {
    to: String,
}

/// POST /api/admin/test-smtp — send a test email using the saved SMTP config.
async fn test_smtp(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(req): Json<TestSmtpReq>,
) -> AppResult<Json<serde_json::Value>> {
    let smtp: settings::SmtpSettings = settings::get(&state.db, settings::KEY_SMTP).await?;
    email::send_email(
        &smtp,
        req.to.trim(),
        "【uuBot】SMTP 测试邮件",
        "这是一封来自 uuBot 的测试邮件,如果你收到了它,说明 SMTP 配置正确。".into(),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}
