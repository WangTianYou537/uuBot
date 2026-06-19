use chrono::Utc;
use qrcode::QrCode;
use qrcode::render::svg;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::collections::HashMap;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;
use wx_bot_sdk::auth::accounts::DEFAULT_BASE_URL;
use wx_bot_sdk::auth::login_qr::{start_weixin_login_with_qr, wait_for_weixin_login};
use wx_bot_sdk::bot::{StartOptions, WeixinBot, handler};

use crate::entities::{bot_conversations, bot_messages, words, wx_bindings};
use crate::error::{AppError, AppResult};
use crate::services::{ai, settings};
use crate::state::AppState;

/// In-flight QR logins and the wx-bot monitors already running, so a binding
/// never spawns two waiters and an account never starts two monitors. The stored
/// `WeixinBot` shares its stop channel (an `Arc`) with the running task, so
/// `stop()` on the stored handle actually halts the monitor.
#[derive(Default)]
pub struct BotRuntime {
    /// `binding_code`s with a background login waiter currently polling.
    login_waiters: AsyncMutex<HashSet<String>>,
    /// Running message monitors keyed by `external_user_id` (bot account id).
    monitors: AsyncMutex<HashMap<String, WeixinBot>>,
}

const STATUS_PENDING: &str = "pending";
const STATUS_ACTIVE: &str = "active";
const STATUS_REVOKED: &str = "revoked";
const DIRECTION_INBOUND: &str = "inbound";
const DIRECTION_OUTBOUND: &str = "outbound";
const MESSAGE_OK: &str = "ok";
const MESSAGE_ERROR: &str = "error";
const MESSAGE_IGNORED: &str = "ignored";

#[derive(Debug, Clone, Deserialize)]
pub struct WxInboundMessage {
    pub external_user_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub conversation_id: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct BotReply {
    pub reply: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    Trans { term: String },
    Add { term: Option<String> },
    List { n: u64 },
    Clear,
}

impl BotCommand {
    fn name(&self) -> &'static str {
        match self {
            BotCommand::Trans { .. } => "trans",
            BotCommand::Add { .. } => "add",
            BotCommand::List { .. } => "list",
            BotCommand::Clear => "clear",
        }
    }
}

pub fn parse_command(input: &str) -> AppResult<Option<BotCommand>> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Ok(None);
    }

    let without_slash = trimmed.trim_start_matches('/');
    let (name, rest) = without_slash
        .split_once(char::is_whitespace)
        .map(|(name, rest)| (name.trim(), rest.trim()))
        .unwrap_or((without_slash.trim(), ""));

    match name {
        "trans" => {
            let term = strip_optional_parens(rest).trim().to_string();
            if term.is_empty() {
                return Err(AppError::BadRequest("用法：/trans example".into()));
            }
            Ok(Some(BotCommand::Trans { term }))
        }
        "add" => {
            let term = strip_optional_parens(rest).trim().to_string();
            Ok(Some(BotCommand::Add {
                term: (!term.is_empty()).then_some(term),
            }))
        }
        "list" => {
            let parsed = parse_args(strip_optional_parens(rest));
            let n = match parsed.params.get("n") {
                Some(raw) => raw
                    .parse::<u64>()
                    .map_err(|_| AppError::BadRequest("-n 需要是数字".into()))?,
                None => 10,
            }
            .clamp(1, 50);
            Ok(Some(BotCommand::List { n }))
        }
        "clear" => Ok(Some(BotCommand::Clear)),
        "" => Err(AppError::BadRequest("请输入指令".into())),
        other => Err(AppError::BadRequest(format!("未知指令：/{other}"))),
    }
}

fn strip_optional_parens(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

struct ParsedArgs {
    params: HashMap<String, String>,
}

fn parse_args(input: &str) -> ParsedArgs {
    let mut params = HashMap::new();
    let parts: Vec<&str> = input.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if let Some(name) = part.strip_prefix('-') {
            if !name.is_empty() && i + 1 < parts.len() && !parts[i + 1].starts_with('-') {
                params.insert(name.to_string(), parts[i + 1].to_string());
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    ParsedArgs { params }
}

pub async fn create_binding(
    state: &AppState,
    user_id: i64,
    display_name: String,
) -> AppResult<wx_bindings::Model> {
    let cfg: settings::BotSettings = settings::get(&state.db, settings::KEY_BOT).await?;
    ensure_binding_limit(state, user_id, cfg.max_bindings_per_user).await?;

    let now = Utc::now();
    let active = wx_bindings::ActiveModel {
        user_id: Set(user_id),
        external_user_id: Set(None),
        binding_code: Set(Uuid::new_v4().to_string()),
        display_name: Set(display_name),
        avatar: Set(String::new()),
        status: Set(STATUS_PENDING.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    Ok(active.insert(&state.db).await?)
}

#[derive(Debug, Serialize)]
pub struct QrInfo {
    /// Raw content encoded in the QR (a WeChat login URL).
    pub content: String,
    /// Pre-rendered SVG so the frontend needs no QR library.
    pub svg: String,
}

/// Request (or, with `force`, refresh) the WeChat login QR for a pending binding.
/// Spawns a single background waiter per binding that activates it once scanned.
pub async fn request_qr(
    state: &AppState,
    user_id: i64,
    binding_id: i64,
    force: bool,
) -> AppResult<QrInfo> {
    let binding = wx_bindings::Entity::find_by_id(binding_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("绑定不存在".into()))?;
    if binding.user_id != user_id {
        return Err(AppError::Forbidden("无权操作该绑定".into()));
    }
    if binding.status == STATUS_ACTIVE {
        return Err(AppError::BadRequest("该微信已绑定".into()));
    }

    let started = start_weixin_login_with_qr(DEFAULT_BASE_URL, Some(&binding.binding_code), None, force)
        .await
        .map_err(|e| AppError::Internal(format!("获取微信二维码失败: {e}")))?;
    let content = started
        .qrcode_url
        .ok_or_else(|| AppError::Internal("微信未返回二维码".into()))?;

    spawn_login_waiter(state, binding.id, binding.binding_code.clone()).await;
    Ok(QrInfo {
        svg: render_qr_svg(&content),
        content,
    })
}

fn render_qr_svg(content: &str) -> String {
    match QrCode::new(content.as_bytes()) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(220, 220)
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}

/// Start exactly one background waiter for `binding_code`. The waiter long-polls
/// WeChat; on a confirmed scan it marks the binding active and starts its monitor.
async fn spawn_login_waiter(state: &AppState, binding_id: i64, binding_code: String) {
    {
        let mut waiters = state.bot.login_waiters.lock().await;
        if !waiters.insert(binding_code.clone()) {
            return; // already waiting
        }
    }
    let state = state.clone();
    tokio::spawn(async move {
        let result = wait_for_weixin_login(&binding_code, DEFAULT_BASE_URL, None, None).await;
        state.bot.login_waiters.lock().await.remove(&binding_code);

        match result {
            Ok(waited) if waited.connected => {
                if let Some(account_id) = waited.account_id {
                    if let Err(e) = activate_binding(&state, binding_id, &account_id).await {
                        tracing::error!("failed to activate wx binding {binding_id}: {e}");
                    } else {
                        start_monitor(&state, account_id).await;
                    }
                }
            }
            Ok(waited) => {
                tracing::info!("wx binding {binding_id} not connected: {}", waited.message);
            }
            Err(e) => {
                tracing::error!("wx login wait failed for binding {binding_id}: {e}");
            }
        }
    });
}

async fn activate_binding(state: &AppState, binding_id: i64, account_id: &str) -> AppResult<()> {
    let binding = wx_bindings::Entity::find_by_id(binding_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("绑定不存在".into()))?;
    let mut active: wx_bindings::ActiveModel = binding.into();
    active.external_user_id = Set(Some(account_id.to_string()));
    active.status = Set(STATUS_ACTIVE.to_string());
    active.updated_at = Set(Utc::now());
    active.update(&state.db).await?;
    Ok(())
}

pub async fn ensure_binding_limit(
    state: &AppState,
    user_id: i64,
    max_bindings: u32,
) -> AppResult<()> {
    let count = wx_bindings::Entity::find()
        .filter(wx_bindings::Column::UserId.eq(user_id))
        .filter(wx_bindings::Column::Status.ne(STATUS_REVOKED))
        .count(&state.db)
        .await?;
    if count >= u64::from(max_bindings) {
        return Err(AppError::BadRequest(format!(
            "每个用户最多绑定 {max_bindings} 个微信"
        )));
    }
    Ok(())
}

pub async fn revoke_binding(state: &AppState, user_id: i64, id: i64) -> AppResult<()> {
    let binding = wx_bindings::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("绑定不存在".into()))?;
    if binding.user_id != user_id {
        return Err(AppError::Forbidden("无权操作该绑定".into()));
    }
    let account_id = binding.external_user_id.clone();
    let mut active: wx_bindings::ActiveModel = binding.into();
    active.status = Set(STATUS_REVOKED.to_string());
    active.updated_at = Set(Utc::now());
    active.update(&state.db).await?;

    // Stop the message monitor for this account, if any was running.
    if let Some(account_id) = account_id {
        if let Err(e) = stop_monitor(state, &account_id).await {
            tracing::warn!("failed to stop wx monitor for {account_id}: {e}");
        }
    }
    Ok(())
}

/// On startup, resume a message monitor for every already-active binding.
pub async fn resume_active_bots(state: &AppState) {
    let bindings = match wx_bindings::Entity::find()
        .filter(wx_bindings::Column::Status.eq(STATUS_ACTIVE))
        .all(&state.db)
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("failed to load active wx bindings: {e}");
            return;
        }
    };
    for binding in bindings {
        if let Some(account_id) = binding.external_user_id {
            start_monitor(state, account_id).await;
        }
    }
}

/// Start a long-polling message monitor for `account_id` (idempotent). The
/// monitor dispatches every inbound WeChat message through [`handle_inbound`].
pub async fn start_monitor(state: &AppState, account_id: String) {
    let bot = {
        let mut monitors = state.bot.monitors.lock().await;
        if monitors.contains_key(&account_id) {
            return; // already running
        }
        let bot = match WeixinBot::from_account(&account_id) {
            Ok(bot) => bot,
            Err(e) => {
                tracing::error!("cannot start wx monitor for {account_id}: {e}");
                return;
            }
        };
        monitors.insert(account_id.clone(), bot.clone());
        bot
    };

    let handler_state = state.clone();
    let task_state = state.clone();
    let bot_account = account_id.clone();
    tokio::spawn(async move {
        let on_message = handler(move |ctx| {
            let state = handler_state.clone();
            let account_id = bot_account.clone();
            async move {
                if ctx.message_type != "text" {
                    return Ok(Some("目前只支持文字指令，请发送 /trans /add /list。".to_string()));
                }
                let inbound = WxInboundMessage {
                    external_user_id: account_id,
                    display_name: ctx.from.clone(),
                    avatar: String::new(),
                    conversation_id: ctx.from.clone(),
                    text: ctx.body.clone(),
                };
                let reply = match handle_inbound(&state, inbound).await {
                    Ok(r) => r.reply,
                    Err(e) => e.to_string(),
                };
                Ok(Some(reply))
            }
        });

        let result = bot
            .start(StartOptions {
                on_message,
                long_poll_timeout_ms: None,
            })
            .await;
        if let Err(e) = result {
            tracing::error!("wx monitor for {account_id} stopped: {e}");
        }
        task_state.bot.monitors.lock().await.remove(&account_id);
    });
}

async fn stop_monitor(state: &AppState, account_id: &str) -> AppResult<()> {
    let bot = state.bot.monitors.lock().await.remove(account_id);
    if let Some(bot) = bot {
        bot.stop()
            .await
            .map_err(|e| AppError::Internal(format!("停止微信监听失败: {e}")))?;
    }
    Ok(())
}

pub async fn handle_inbound(state: &AppState, inbound: WxInboundMessage) -> AppResult<BotReply> {
    let binding = wx_bindings::Entity::find()
        .filter(wx_bindings::Column::ExternalUserId.eq(Some(inbound.external_user_id.trim().to_string())))
        .filter(wx_bindings::Column::Status.eq(STATUS_ACTIVE))
        .one(&state.db)
        .await?;

    let Some(binding) = binding else {
        return Ok(BotReply {
            reply: "这个微信还没有绑定 uuBot 账号，请先在网页端申请绑定。".into(),
        });
    };

    refresh_binding_profile(state, &binding, &inbound).await?;
    let conversation = find_or_create_conversation(state, &binding, &inbound).await?;

    let parsed = parse_command(&inbound.text);
    let command_name = parsed
        .as_ref()
        .ok()
        .and_then(|cmd| cmd.as_ref().map(BotCommand::name))
        .unwrap_or("");
    record_message(
        state,
        conversation.id,
        DIRECTION_INBOUND,
        &inbound.text,
        command_name,
        if parsed.as_ref().ok().and_then(|cmd| cmd.as_ref()).is_some() {
            MESSAGE_OK
        } else if parsed.is_err() {
            MESSAGE_ERROR
        } else {
            MESSAGE_IGNORED
        },
        json!({}),
    )
    .await?;

    let reply = match parsed {
        Ok(Some(command)) => execute_command(state, conversation.clone(), command).await?,
        Ok(None) => "请发送以 / 开头的指令，例如 /trans example".into(),
        Err(e) => e.to_string(),
    };

    if command_name != "clear" {
        record_message(
            state,
            conversation.id,
            DIRECTION_OUTBOUND,
            &reply,
            command_name,
            MESSAGE_OK,
            json!({}),
        )
        .await?;
    }

    Ok(BotReply { reply })
}

async fn refresh_binding_profile(
    state: &AppState,
    binding: &wx_bindings::Model,
    inbound: &WxInboundMessage,
) -> AppResult<()> {
    if inbound.display_name.trim().is_empty() && inbound.avatar.trim().is_empty() {
        return Ok(());
    }
    let mut active: wx_bindings::ActiveModel = binding.clone().into();
    if !inbound.display_name.trim().is_empty() {
        active.display_name = Set(inbound.display_name.trim().to_string());
    }
    if !inbound.avatar.trim().is_empty() {
        active.avatar = Set(inbound.avatar.trim().to_string());
    }
    active.updated_at = Set(Utc::now());
    active.update(&state.db).await?;
    Ok(())
}

async fn find_or_create_conversation(
    state: &AppState,
    binding: &wx_bindings::Model,
    inbound: &WxInboundMessage,
) -> AppResult<bot_conversations::Model> {
    let external_id = if inbound.conversation_id.trim().is_empty() {
        inbound.external_user_id.trim()
    } else {
        inbound.conversation_id.trim()
    };

    if let Some(found) = bot_conversations::Entity::find()
        .filter(bot_conversations::Column::BindingId.eq(binding.id))
        .filter(bot_conversations::Column::ExternalConversationId.eq(external_id))
        .one(&state.db)
        .await?
    {
        return Ok(found);
    }

    let now = Utc::now();
    let active = bot_conversations::ActiveModel {
        user_id: Set(binding.user_id),
        binding_id: Set(binding.id),
        external_conversation_id: Set(external_id.to_string()),
        last_translated_term: Set(String::new()),
        last_translation_json: Set(String::new()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    Ok(active.insert(&state.db).await?)
}

async fn execute_command(
    state: &AppState,
    conversation: bot_conversations::Model,
    command: BotCommand,
) -> AppResult<String> {
    match command {
        BotCommand::Trans { term } => {
            let cfg: settings::AiSettings = settings::get(&state.db, settings::KEY_AI).await?;
            let translated = ai::translate(&state.http, &cfg, &term).await?;
            let json = serde_json::to_string(&translated)
                .map_err(|e| AppError::Internal(format!("保存翻译结果失败: {e}")))?;
            let mut active: bot_conversations::ActiveModel = conversation.clone().into();
            active.last_translated_term = Set(term.clone());
            active.last_translation_json = Set(json);
            active.updated_at = Set(Utc::now());
            active.update(&state.db).await?;
            Ok(format_translation(&term, &translated))
        }
        BotCommand::Add { term } => {
            let (term, translated) = match term {
                Some(term) => {
                    let cfg: settings::AiSettings = settings::get(&state.db, settings::KEY_AI).await?;
                    let translated = ai::translate(&state.http, &cfg, &term).await?;
                    (term, translated)
                }
                None => {
                    if conversation.last_translated_term.trim().is_empty()
                        || conversation.last_translation_json.trim().is_empty()
                    {
                        return Err(AppError::BadRequest(
                            "没有可添加的上一次翻译，请先使用 /trans 单词".into(),
                        ));
                    }
                    let translated = serde_json::from_str::<ai::AiTranslationResult>(
                        &conversation.last_translation_json,
                    )
                    .map_err(|e| AppError::Internal(format!("读取上一次翻译失败: {e}")))?;
                    (conversation.last_translated_term.clone(), translated)
                }
            };
            let word = insert_word_from_translation(state, conversation.user_id, &term, translated).await?;
            Ok(format!("已加入词库：{}", word.term))
        }
        BotCommand::List { n } => {
            let items = words::Entity::find()
                .filter(words::Column::UserId.eq(conversation.user_id))
                .order_by_desc(words::Column::CreatedAt)
                .limit(n)
                .all(&state.db)
                .await?;
            if items.is_empty() {
                return Ok("词库还是空的。".into());
            }
            let lines = items
                .into_iter()
                .enumerate()
                .map(|(i, word)| format_list_word(i + 1, &word.term, &word.definition))
                .collect::<Vec<_>>()
                .join("\n\n");
            Ok(lines)
        }
        BotCommand::Clear => {
            clear_conversation_history(state, conversation.id).await?;
            let mut active: bot_conversations::ActiveModel = conversation.into();
            active.last_translated_term = Set(String::new());
            active.last_translation_json = Set(String::new());
            active.updated_at = Set(Utc::now());
            active.update(&state.db).await?;
            Ok("已清除当前会话的上一次翻译记录和网页端聊天记录。".into())
        }
    }
}

async fn clear_conversation_history(state: &AppState, conversation_id: i64) -> AppResult<()> {
    bot_messages::Entity::delete_many()
        .filter(bot_messages::Column::ConversationId.eq(conversation_id))
        .exec(&state.db)
        .await?;
    Ok(())
}

async fn insert_word_from_translation(
    state: &AppState,
    user_id: i64,
    term: &str,
    translated: ai::AiTranslationResult,
) -> AppResult<words::Model> {
    let now = Utc::now();
    let raw_json = if translated.raw_json.trim().is_empty() {
        serde_json::to_string(&translated).unwrap_or_default()
    } else {
        translated.raw_json.clone()
    };
    let active = words::ActiveModel {
        user_id: Set(user_id),
        term: Set(term.trim().to_string()),
        phonetic: Set(translated.phonetic),
        definition: Set(translated.definition),
        example: Set(translated.example),
        note: Set(translated.note),
        tags: Set(translated.tags),
        input_type: Set(translated.input_type),
        difficulty: Set(translated.difficulty),
        content_markdown: Set(translated.content_markdown),
        source: Set("bot".into()),
        raw_json: Set(raw_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    Ok(active.insert(&state.db).await?)
}

async fn record_message(
    state: &AppState,
    conversation_id: i64,
    direction: &str,
    content: &str,
    command: &str,
    status: &str,
    metadata: serde_json::Value,
) -> AppResult<bot_messages::Model> {
    let active = bot_messages::ActiveModel {
        conversation_id: Set(conversation_id),
        direction: Set(direction.to_string()),
        content: Set(content.to_string()),
        command: Set(command.to_string()),
        status: Set(status.to_string()),
        metadata_json: Set(metadata.to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    Ok(active.insert(&state.db).await?)
}

fn format_translation(term: &str, translated: &ai::AiTranslationResult) -> String {
    if !translated.content_markdown.trim().is_empty() {
        return translated.content_markdown.trim().to_string();
    }
    let mut parts = vec![format!("{} {}", term, translated.phonetic).trim().to_string()];
    if !translated.definition.trim().is_empty() {
        parts.push(translated.definition.trim().to_string());
    }
    if !translated.example.trim().is_empty() {
        parts.push(format!("例句：{}", translated.example.trim()));
    }
    if !translated.note.trim().is_empty() {
        parts.push(format!("提示：{}", translated.note.trim()));
    }
    if !translated.tags.trim().is_empty() {
        parts.push(format!("标签：{}", translated.tags.trim()));
    }
    parts.join("\n")
}

fn format_list_word(index: usize, term: &str, definition: &str) -> String {
    let definition = definition.trim();
    if definition.is_empty() {
        return format!("{index}. {term} — 暂无释义");
    }

    let lines = definition
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    match lines.as_slice() {
        [] => format!("{index}. {term} — 暂无释义"),
        [one] => format!("{index}. {term} — {one}"),
        _ => format!("{index}. {term}\n  - {}", lines.join("\n  - ")),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_command, BotCommand};

    #[test]
    fn parses_commands() {
        assert_eq!(
            parse_command("/trans example").unwrap(),
            Some(BotCommand::Trans { term: "example".into() })
        );
        assert_eq!(
            parse_command("/add (example)").unwrap(),
            Some(BotCommand::Add { term: Some("example".into()) })
        );
        assert_eq!(parse_command("/add").unwrap(), Some(BotCommand::Add { term: None }));
        assert_eq!(parse_command("/list (-n 10)").unwrap(), Some(BotCommand::List { n: 10 }));
        assert_eq!(parse_command("/list").unwrap(), Some(BotCommand::List { n: 10 }));
        assert_eq!(parse_command("/clear").unwrap(), Some(BotCommand::Clear));
        assert!(parse_command("/list -n bad").is_err());
        assert_eq!(parse_command("hello").unwrap(), None);
    }
}
