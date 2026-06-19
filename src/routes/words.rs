use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::CurrentUser;
use crate::entities::words;
use crate::error::{AppError, AppResult};
use crate::services::{ai, dictionary, settings};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/lookup", get(lookup))
        .route("/ai-translate", axum::routing::post(ai_translate))
        .route("/{id}", get(detail).put(update).delete(delete))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    page: Option<u64>,
    #[serde(default)]
    page_size: Option<u64>,
}

#[derive(Serialize)]
struct ListResp {
    items: Vec<words::Model>,
    total: u64,
    page: u64,
    page_size: u64,
}

/// GET /api/words — paginated list of the user's words, optional search.
async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<ListResp>> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

    let mut cond = Condition::all().add(words::Column::UserId.eq(user.id));
    if let Some(term) = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let like = format!("%{term}%");
        cond = cond.add(
            Condition::any()
                .add(words::Column::Term.like(&like))
                .add(words::Column::Definition.like(&like))
                .add(words::Column::Tags.like(&like))
                .add(words::Column::ContentMarkdown.like(&like)),
        );
    }

    let paginator = words::Entity::find()
        .filter(cond)
        .order_by_desc(words::Column::CreatedAt)
        .paginate(&state.db, page_size);

    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(ListResp {
        items,
        total,
        page,
        page_size,
    }))
}

#[derive(Deserialize)]
struct LookupQuery {
    term: String,
}

/// GET /api/words/lookup — dictionary lookup helper (does not save).
async fn lookup(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
    Query(q): Query<LookupQuery>,
) -> AppResult<Json<dictionary::DictionaryResult>> {
    if q.term.trim().is_empty() {
        return Err(AppError::BadRequest("请输入要查询的单词".into()));
    }
    let cfg: settings::DictionarySettings =
        settings::get(&state.db, settings::KEY_DICTIONARY).await?;
    let res = dictionary::lookup(&state.http, &cfg, &q.term).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct AiTranslateReq {
    term: String,
}

/// POST /api/words/ai-translate — AI translation helper (does not save).
async fn ai_translate(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
    Json(req): Json<AiTranslateReq>,
) -> AppResult<Json<ai::AiTranslationResult>> {
    let term = req.term.trim();
    if term.is_empty() {
        return Err(AppError::BadRequest("请输入要翻译的单词".into()));
    }
    let cfg: settings::AiSettings = settings::get(&state.db, settings::KEY_AI).await?;
    let res = ai::translate(&state.http, &cfg, term).await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct CreateReq {
    term: String,
    #[serde(default)]
    phonetic: String,
    #[serde(default)]
    definition: String,
    #[serde(default)]
    example: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    input_type: String,
    #[serde(default)]
    difficulty: String,
    #[serde(default)]
    content_markdown: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    raw_json: String,
    /// When true and definition is empty, auto-fill from the dictionary.
    #[serde(default)]
    auto_lookup: bool,
}

/// POST /api/words — create a word, optionally auto-filling from the dictionary.
async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(mut req): Json<CreateReq>,
) -> AppResult<Json<words::Model>> {
    let term = req.term.trim().to_string();
    if term.is_empty() {
        return Err(AppError::BadRequest("单词不能为空".into()));
    }

    if req.auto_lookup && req.definition.trim().is_empty() {
        let cfg: settings::DictionarySettings =
            settings::get(&state.db, settings::KEY_DICTIONARY).await?;
        if cfg.enabled {
            // Best-effort: ignore lookup failures, just create with what we have.
            if let Ok(found) = dictionary::lookup(&state.http, &cfg, &term).await {
                if req.phonetic.trim().is_empty() {
                    req.phonetic = found.phonetic;
                }
                if req.definition.trim().is_empty() {
                    req.definition = found.definition;
                }
                if req.example.trim().is_empty() {
                    req.example = found.example;
                }
                if req.note.trim().is_empty() {
                    req.note = found.note;
                }
                if req.tags.trim().is_empty() {
                    req.tags = found.tags;
                }
                if req.content_markdown.trim().is_empty() {
                    req.content_markdown = found.content_markdown;
                }
                if req.raw_json.trim().is_empty() {
                    req.raw_json = found.raw_json;
                }
                if req.source.trim().is_empty() {
                    req.source = "dictionary".into();
                }
            }
        }
    }

    let now = Utc::now();
    let active = words::ActiveModel {
        user_id: Set(user.id),
        term: Set(term),
        phonetic: Set(req.phonetic),
        definition: Set(req.definition),
        example: Set(req.example),
        note: Set(req.note),
        tags: Set(req.tags),
        input_type: Set(req.input_type),
        difficulty: Set(req.difficulty),
        content_markdown: Set(req.content_markdown),
        source: Set(if req.source.trim().is_empty() {
            "manual".into()
        } else {
            req.source
        }),
        raw_json: Set(req.raw_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let model = active.insert(&state.db).await?;
    Ok(Json(model))
}

/// Load a word owned by the user, or 404.
async fn owned(state: &AppState, user_id: i64, id: i64) -> AppResult<words::Model> {
    let word = words::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("单词不存在".into()))?;
    if word.user_id != user_id {
        return Err(AppError::Forbidden("无权访问该单词".into()));
    }
    Ok(word)
}

/// GET /api/words/{id}
async fn detail(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<words::Model>> {
    Ok(Json(owned(&state, user.id, id).await?))
}

#[derive(Deserialize)]
struct UpdateReq {
    term: Option<String>,
    phonetic: Option<String>,
    definition: Option<String>,
    example: Option<String>,
    note: Option<String>,
    tags: Option<String>,
    input_type: Option<String>,
    difficulty: Option<String>,
    content_markdown: Option<String>,
    source: Option<String>,
    raw_json: Option<String>,
}

/// PUT /api/words/{id}
async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateReq>,
) -> AppResult<Json<words::Model>> {
    let word = owned(&state, user.id, id).await?;
    let mut active: words::ActiveModel = word.into();
    if let Some(v) = req.term {
        let v = v.trim().to_string();
        if v.is_empty() {
            return Err(AppError::BadRequest("单词不能为空".into()));
        }
        active.term = Set(v);
    }
    if let Some(v) = req.phonetic {
        active.phonetic = Set(v);
    }
    if let Some(v) = req.definition {
        active.definition = Set(v);
    }
    if let Some(v) = req.example {
        active.example = Set(v);
    }
    if let Some(v) = req.note {
        active.note = Set(v);
    }
    if let Some(v) = req.tags {
        active.tags = Set(v);
    }
    if let Some(v) = req.input_type {
        active.input_type = Set(v);
    }
    if let Some(v) = req.difficulty {
        active.difficulty = Set(v);
    }
    if let Some(v) = req.content_markdown {
        active.content_markdown = Set(v);
    }
    if let Some(v) = req.source {
        active.source = Set(v);
    }
    if let Some(v) = req.raw_json {
        active.raw_json = Set(v);
    }
    active.updated_at = Set(Utc::now());
    let model = active.update(&state.db).await?;
    Ok(Json(model))
}

/// DELETE /api/words/{id}
async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let word = owned(&state, user.id, id).await?;
    words::Entity::delete_by_id(word.id)
        .exec(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}
