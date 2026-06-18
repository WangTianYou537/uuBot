use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::CurrentUser;
use crate::entities::{bot_conversations, bot_messages, wx_bindings};
use crate::error::{AppError, AppResult};
use crate::services::{bot, settings};
use crate::state::AppState;

const BOT_SECRET_HEADER: &str = "x-uubot-bot-secret";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/webhook", post(webhook))
        .route("/bindings", get(list_bindings).post(create_binding))
        .route("/bindings/{id}", axum::routing::delete(revoke_binding))
        .route("/bindings/{id}/qrcode", post(qrcode))
        .route("/conversations", get(list_conversations))
        .route("/conversations/{id}/messages", get(list_messages))
}

#[derive(Deserialize)]
struct BindingReq {
    #[serde(default)]
    display_name: String,
}

#[derive(Serialize)]
struct BindingListResp {
    items: Vec<wx_bindings::Model>,
    total: u64,
    max_bindings: u32,
}

async fn list_bindings(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<BindingListResp>> {
    let cfg: settings::BotSettings = settings::get(&state.db, settings::KEY_BOT).await?;
    let items = wx_bindings::Entity::find()
        .filter(wx_bindings::Column::UserId.eq(user.id))
        .filter(wx_bindings::Column::Status.ne("revoked"))
        .order_by_desc(wx_bindings::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let total = items.len() as u64;
    Ok(Json(BindingListResp {
        items,
        total,
        max_bindings: cfg.max_bindings_per_user,
    }))
}

async fn create_binding(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<BindingReq>,
) -> AppResult<Json<wx_bindings::Model>> {
    let binding = bot::create_binding(&state, user.id, req.display_name).await?;
    Ok(Json(binding))
}

async fn revoke_binding(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    bot::revoke_binding(&state, user.id, id).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct QrQuery {
    #[serde(default)]
    force: bool,
}

/// POST /api/bot/bindings/{id}/qrcode — fetch (or refresh with ?force=true) the
/// WeChat login QR for a pending binding.
async fn qrcode(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<QrQuery>,
) -> AppResult<Json<bot::QrInfo>> {
    let info = bot::request_qr(&state, user.id, id, q.force).await?;
    Ok(Json(info))
}

async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<bot::WxInboundMessage>,
) -> AppResult<Json<bot::BotReply>> {
    let cfg: settings::BotSettings = settings::get(&state.db, settings::KEY_BOT).await?;
    if !cfg.enabled {
        return Err(AppError::BadRequest("wx-bot 尚未启用".into()));
    }
    if cfg.webhook_secret.trim().is_empty() {
        return Err(AppError::BadRequest("请先在后台配置 wx-bot Webhook Secret".into()));
    }
    let got = headers
        .get(BOT_SECRET_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if got != cfg.webhook_secret.trim() {
        return Err(AppError::Unauthorized("wx-bot secret 不正确".into()));
    }
    let reply = bot::handle_inbound(&state, req).await?;
    Ok(Json(reply))
}

#[derive(Serialize)]
struct ConversationListResp {
    items: Vec<bot_conversations::Model>,
    total: u64,
}

async fn list_conversations(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ConversationListResp>> {
    let items = bot_conversations::Entity::find()
        .filter(bot_conversations::Column::UserId.eq(user.id))
        .order_by_desc(bot_conversations::Column::UpdatedAt)
        .all(&state.db)
        .await?;
    let total = items.len() as u64;
    Ok(Json(ConversationListResp { items, total }))
}

#[derive(Deserialize)]
struct MessagesQuery {
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    page_size: Option<u64>,
}

#[derive(Serialize)]
struct MessageListResp {
    items: Vec<bot_messages::Model>,
    total: u64,
    page: u64,
    page_size: u64,
}

async fn list_messages(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(q): Query<MessagesQuery>,
) -> AppResult<Json<MessageListResp>> {
    let conversation = bot_conversations::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("会话不存在".into()))?;
    if conversation.user_id != user.id {
        return Err(AppError::Forbidden("无权访问该会话".into()));
    }

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(50).clamp(1, 100);
    let paginator = bot_messages::Entity::find()
        .filter(bot_messages::Column::ConversationId.eq(id))
        .order_by_asc(bot_messages::Column::CreatedAt)
        .paginate(&state.db, page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(MessageListResp {
        items,
        total,
        page,
        page_size,
    }))
}
