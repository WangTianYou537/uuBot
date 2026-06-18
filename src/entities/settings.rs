use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Key-value store for runtime-configurable settings (SMTP, OAuth provider, dictionary).
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    #[sea_orm(column_type = "Text")]
    pub value: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
