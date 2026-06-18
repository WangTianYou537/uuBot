use axum::Json;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth;
use crate::codes;
use crate::entities::users;
use crate::error::{AppError, AppResult};
use crate::services::{email, mapay, settings};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/wechat/login", get(wechat_login))
        .route("/wechat/callback", get(wechat_callback))
        .route("/email/send-code", post(email_send_code))
        .route("/email/login", post(email_login))
        .route("/logout", post(logout))
}

#[derive(Serialize)]
struct LoginUrlResp {
    url: String,
    qrcode: String,
}

/// GET /api/auth/wechat/login — get the WeChat login redirect URL + QR code.
async fn wechat_login(State(state): State<AppState>) -> AppResult<Json<LoginUrlResp>> {
    let oauth: settings::OAuthSettings =
        settings::get(&state.db, settings::KEY_OAUTH).await?;
    let redirect_uri = format!("{}/api/auth/wechat/callback", state.config.public_base_url);
    let r = mapay::start_login(&state.http, &oauth, "wx", &redirect_uri).await?;
    Ok(Json(LoginUrlResp {
        url: r.url,
        qrcode: r.qrcode,
    }))
}

#[derive(Deserialize)]
struct CallbackQuery {
    #[serde(default)]
    code: String,
    #[serde(default, rename = "type")]
    _type: String,
}

/// GET /api/auth/wechat/callback — provider redirects here with `code`.
/// Upserts the user, sets the session cookie, and redirects to the SPA.
async fn wechat_callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
    jar: CookieJar,
) -> AppResult<impl IntoResponse> {
    if q.code.is_empty() {
        return Err(AppError::BadRequest("缺少授权 code".into()));
    }
    let oauth: settings::OAuthSettings =
        settings::get(&state.db, settings::KEY_OAUTH).await?;
    let identity = mapay::exchange_code(&state.http, &oauth, "wx", &q.code).await?;

    // Upsert by social_uid.
    let existing = users::Entity::find()
        .filter(users::Column::SocialUid.eq(&identity.social_uid))
        .one(&state.db)
        .await?;

    let now = Utc::now();
    let user_id = match existing {
        Some(u) => {
            // Refresh profile fields.
            let mut active: users::ActiveModel = u.clone().into();
            active.nickname = Set(identity.nickname);
            active.avatar = Set(identity.avatar);
            active.updated_at = Set(now);
            users::Entity::update(active).exec(&state.db).await?;
            u.id
        }
        None => {
            let active = users::ActiveModel {
                social_uid: Set(identity.social_uid),
                nickname: Set(identity.nickname),
                avatar: Set(identity.avatar),
                email: Set(None),
                password_hash: Set(None),
                email_verified: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };
            let res = users::Entity::insert(active).exec(&state.db).await?;
            res.last_insert_id
        }
    };

    let token = auth::sign(&state, user_id, "user")?;
    let jar = jar.add(auth::session_cookie(&state, token));
    Ok((jar, Redirect::to("/app")))
}

#[derive(Deserialize)]
struct SendCodeReq {
    email: String,
}

/// POST /api/auth/email/send-code — send a login code to a bound email.
async fn email_send_code(
    State(state): State<AppState>,
    Json(req): Json<SendCodeReq>,
) -> AppResult<Json<serde_json::Value>> {
    let email_addr = req.email.trim().to_lowercase();
    // Only send if some user has this email bound (avoid leaking existence loudly,
    // but we still must not create accounts via email).
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email_addr))
        .one(&state.db)
        .await?;
    if user.is_none() {
        return Err(AppError::BadRequest("该邮箱未绑定任何账号".into()));
    }

    let smtp: settings::SmtpSettings = settings::get(&state.db, settings::KEY_SMTP).await?;
    let code = codes::create_code(&state.db, &email_addr, "login").await?;
    email::send_code_email(&smtp, &email_addr, &code, "登录").await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct EmailLoginReq {
    email: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

/// POST /api/auth/email/login — log in with email using code OR password.
async fn email_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<EmailLoginReq>,
) -> AppResult<impl IntoResponse> {
    let email_addr = req.email.trim().to_lowercase();
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email_addr))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("邮箱或凭证不正确".into()))?;

    match (req.code.as_deref(), req.password.as_deref()) {
        (Some(code), _) if !code.is_empty() => {
            codes::verify_code(&state.db, &email_addr, "login", code).await?;
        }
        (_, Some(password)) if !password.is_empty() => {
            let hash = user
                .password_hash
                .as_deref()
                .ok_or_else(|| AppError::BadRequest("该账号未设置密码,请使用验证码登录".into()))?;
            if !auth::verify_password(password, hash) {
                return Err(AppError::Unauthorized("邮箱或密码不正确".into()));
            }
        }
        _ => return Err(AppError::BadRequest("请提供验证码或密码".into())),
    }

    let token = auth::sign(&state, user.id, "user")?;
    let jar = jar.add(auth::session_cookie(&state, token));
    Ok((jar, Json(json!({ "ok": true }))))
}

/// POST /api/auth/logout — clear the session cookie.
async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let jar = jar.add(auth::clear_cookie(&state));
    (jar, Json(json!({ "ok": true })))
}
