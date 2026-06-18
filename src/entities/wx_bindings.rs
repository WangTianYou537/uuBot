use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "wx_bindings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub user_id: i64,
    /// External WeChat identity from the wx-bot provider. Empty while pending.
    #[sea_orm(unique, nullable, indexed)]
    pub external_user_id: Option<String>,
    #[sea_orm(unique, indexed)]
    pub binding_code: String,
    pub display_name: String,
    #[sea_orm(column_type = "Text")]
    pub avatar: String,
    /// pending | active | revoked
    pub status: String,
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
    #[sea_orm(has_many = "super::bot_conversations::Entity")]
    BotConversations,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::bot_conversations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BotConversations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
