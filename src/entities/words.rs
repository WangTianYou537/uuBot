use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "words")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub term: String,
    pub phonetic: String,
    #[sea_orm(column_type = "Text")]
    pub definition: String,
    #[sea_orm(column_type = "Text")]
    pub example: String,
    #[sea_orm(column_type = "Text")]
    pub note: String,
    /// Comma-separated tags.
    pub tags: String,
    #[serde(default)]
    pub input_type: String,
    #[serde(default)]
    pub difficulty: String,
    #[sea_orm(column_type = "Text")]
    #[serde(default)]
    pub content_markdown: String,
    #[serde(default)]
    pub source: String,
    #[sea_orm(column_type = "Text")]
    #[serde(default)]
    pub raw_json: String,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
