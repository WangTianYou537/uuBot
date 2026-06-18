use chrono::{Duration, Utc};
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::entities::email_codes;
use crate::error::AppError;
use crate::services::email::generate_code;

const CODE_TTL_MINUTES: i64 = 10;

/// Create and persist a fresh verification code for `email`/`purpose`.
/// Returns the generated code so the caller can email it.
pub async fn create_code(
    db: &DatabaseConnection,
    email: &str,
    purpose: &str,
) -> Result<String, AppError> {
    let code = generate_code();
    let now = Utc::now();
    let model = email_codes::ActiveModel {
        email: Set(email.to_string()),
        code: Set(code.clone()),
        purpose: Set(purpose.to_string()),
        expires_at: Set(now + Duration::minutes(CODE_TTL_MINUTES)),
        consumed: Set(false),
        created_at: Set(now),
        ..Default::default()
    };
    email_codes::Entity::insert(model).exec(db).await?;
    Ok(code)
}

/// Verify a submitted code for `email`/`purpose`. On success marks it consumed.
pub async fn verify_code(
    db: &DatabaseConnection,
    email: &str,
    purpose: &str,
    submitted: &str,
) -> Result<(), AppError> {
    let now = Utc::now();
    let candidate = email_codes::Entity::find()
        .filter(email_codes::Column::Email.eq(email))
        .filter(email_codes::Column::Purpose.eq(purpose))
        .filter(email_codes::Column::Consumed.eq(false))
        .filter(email_codes::Column::ExpiresAt.gt(now))
        .order_by_desc(email_codes::Column::Id)
        .one(db)
        .await?;

    let Some(row) = candidate else {
        return Err(AppError::BadRequest("验证码无效或已过期".into()));
    };
    if row.code != submitted {
        return Err(AppError::BadRequest("验证码不正确".into()));
    }

    let mut active: email_codes::ActiveModel = row.into();
    active.consumed = Set(true);
    email_codes::Entity::update(active).exec(db).await?;
    Ok(())
}
