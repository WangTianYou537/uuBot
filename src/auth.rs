use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::entities::{admins, users};
use crate::error::AppError;
use crate::state::AppState;

pub const USER_COOKIE_NAME: &str = "user_session";
pub const ADMIN_COOKIE_NAME: &str = "admin_session";
const LEGACY_COOKIE_NAME: &str = "session";
const SESSION_DAYS: i64 = 30;

/// JWT claims. `kind` distinguishes user sessions from admin sessions so a user
/// token can never be replayed against admin routes.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub kind: String,
    pub exp: i64,
}

/// Sign a session token for the given subject and kind ("user" | "admin").
pub fn sign(state: &AppState, sub: i64, kind: &str) -> Result<String, AppError> {
    let exp = (Utc::now() + Duration::days(SESSION_DAYS)).timestamp();
    let claims = Claims {
        sub,
        kind: kind.to_string(),
        exp,
    };
    encode(&Header::new(Algorithm::HS256), &claims, &state.jwt_encoding)
        .map_err(|e| AppError::Internal(format!("failed to sign token: {e}")))
}

/// Build the session cookie holding `token` for a regular user.
pub fn user_session_cookie<'a>(state: &AppState, token: String) -> Cookie<'a> {
    session_cookie_named(state, USER_COOKIE_NAME, token)
}

/// Build the session cookie holding `token` for an admin.
pub fn admin_session_cookie<'a>(state: &AppState, token: String) -> Cookie<'a> {
    session_cookie_named(state, ADMIN_COOKIE_NAME, token)
}

fn session_cookie_named<'a>(state: &AppState, name: &'static str, token: String) -> Cookie<'a> {
    let mut cookie = Cookie::new(name, token);
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(state.config.cookie_secure);
    cookie.set_max_age(time::Duration::days(SESSION_DAYS));
    cookie
}

/// Build a cookie that clears the regular user session.
pub fn clear_user_cookie<'a>(state: &AppState) -> Cookie<'a> {
    clear_cookie_named(state, USER_COOKIE_NAME)
}

/// Build a cookie that clears the admin session.
pub fn clear_admin_cookie<'a>(state: &AppState) -> Cookie<'a> {
    clear_cookie_named(state, ADMIN_COOKIE_NAME)
}

/// Build a cookie that clears the legacy shared session cookie.
pub fn clear_legacy_cookie<'a>(state: &AppState) -> Cookie<'a> {
    clear_cookie_named(state, LEGACY_COOKIE_NAME)
}

fn clear_cookie_named<'a>(state: &AppState, name: &'static str) -> Cookie<'a> {
    let mut cookie = Cookie::new(name, "");
    cookie.set_http_only(true);
    cookie.set_path("/");
    cookie.set_same_site(SameSite::Lax);
    cookie.set_secure(state.config.cookie_secure);
    cookie.set_max_age(time::Duration::seconds(0));
    cookie
}

fn decode_claims(state: &AppState, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    decode::<Claims>(token, &state.jwt_decoding, &validation)
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized("invalid or expired session".into()))
}

fn token_of_kind(parts: &Parts, state: &AppState, kind: &str) -> Result<i64, AppError> {
    let jar = CookieJar::from_headers(&parts.headers);
    let cookie_name = match kind {
        "admin" => ADMIN_COOKIE_NAME,
        _ => USER_COOKIE_NAME,
    };
    let token = jar
        .get(cookie_name)
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::Unauthorized("not authenticated".into()))?;
    let claims = decode_claims(state, &token)?;
    if claims.kind != kind {
        return Err(AppError::Unauthorized("wrong session type".into()));
    }
    Ok(claims.sub)
}

/// Hash a plaintext password using Argon2.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("failed to hash password: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Authenticated regular user, resolved from the session cookie.
pub struct CurrentUser(pub users::Model);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let id = token_of_kind(parts, state, "user")?;
        let user = users::Entity::find_by_id(id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("user no longer exists".into()))?;
        Ok(CurrentUser(user))
    }
}

/// Authenticated admin, resolved from the session cookie.
pub struct CurrentAdmin(pub admins::Model);

impl FromRequestParts<AppState> for CurrentAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let id = token_of_kind(parts, state, "admin")?;
        let admin = admins::Entity::find_by_id(id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("admin no longer exists".into()))?;
        Ok(CurrentAdmin(admin))
    }
}
