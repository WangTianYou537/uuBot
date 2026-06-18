use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique, indexed)]
    pub social_uid: String,
    pub nickname: String,
    #[sea_orm(column_type = "Text")]
    pub avatar: String,
    #[sea_orm(unique, nullable, indexed)]
    pub email: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub email_verified: bool,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::words::Entity")]
    Words,
}

impl Related<super::words::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Words.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
