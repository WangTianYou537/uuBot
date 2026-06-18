use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One-time verification codes sent over email (purpose: "bind" or "login").
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_codes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub email: String,
    pub code: String,
    pub purpose: String,
    pub expires_at: ChronoDateTimeUtc,
    pub consumed: bool,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
