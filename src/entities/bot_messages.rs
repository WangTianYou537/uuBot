use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bot_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(indexed)]
    pub conversation_id: i64,
    /// inbound | outbound
    pub direction: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub command: String,
    /// ok | error | ignored
    pub status: String,
    #[sea_orm(column_type = "Text")]
    pub metadata_json: String,
    pub created_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bot_conversations::Entity",
        from = "Column::ConversationId",
        to = "super::bot_conversations::Column::Id",
        on_delete = "Cascade"
    )]
    BotConversation,
}

impl Related<super::bot_conversations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BotConversation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
