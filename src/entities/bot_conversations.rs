use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bot_conversations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub user_id: i64,
    #[sea_orm(indexed)]
    pub binding_id: i64,
    pub external_conversation_id: String,
    pub last_translated_term: String,
    #[sea_orm(column_type = "Text")]
    pub last_translation_json: String,
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
    #[sea_orm(
        belongs_to = "super::wx_bindings::Entity",
        from = "Column::BindingId",
        to = "super::wx_bindings::Column::Id",
        on_delete = "Cascade"
    )]
    WxBinding,
    #[sea_orm(has_many = "super::bot_messages::Entity")]
    BotMessages,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::wx_bindings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WxBinding.def()
    }
}

impl Related<super::bot_messages::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BotMessages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
