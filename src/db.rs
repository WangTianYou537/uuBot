use sea_orm::sea_query::TableCreateStatement;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema, Statement};

use crate::entities::{
    admins, bot_conversations, bot_messages, email_codes, settings, users, words, wx_bindings,
};

/// Connect to the database described by `database_url` (sqlite/mysql/postgres).
pub async fn connect(database_url: &str) -> anyhow::Result<DatabaseConnection> {
    let db = Database::connect(database_url).await?;
    Ok(db)
}

/// Create all tables if they do not already exist. Uses SeaORM's schema builder
/// so the same code works across every supported backend.
pub async fn create_tables(db: &DatabaseConnection) -> anyhow::Result<()> {
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);

    create(db, backend, schema.create_table_from_entity(users::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(words::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(admins::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(settings::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(email_codes::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(wx_bindings::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(bot_conversations::Entity)).await?;
    create(db, backend, schema.create_table_from_entity(bot_messages::Entity)).await?;
    add_word_columns(db, backend).await?;

    Ok(())
}

async fn create(
    db: &DatabaseConnection,
    backend: DbBackend,
    mut stmt: TableCreateStatement,
) -> anyhow::Result<()> {
    stmt.if_not_exists();
    db.execute(backend.build(&stmt)).await?;
    Ok(())
}

async fn add_word_columns(db: &DatabaseConnection, backend: DbBackend) -> anyhow::Result<()> {
    for (name, ty) in [
        ("input_type", "VARCHAR(32) NOT NULL DEFAULT ''"),
        ("difficulty", "VARCHAR(32) NOT NULL DEFAULT ''"),
        ("content_markdown", "TEXT NOT NULL DEFAULT ''"),
        ("source", "VARCHAR(32) NOT NULL DEFAULT 'manual'"),
        ("raw_json", "TEXT NOT NULL DEFAULT ''"),
    ] {
        let sql = format!("ALTER TABLE words ADD COLUMN {name} {ty}");
        if let Err(e) = db.execute(Statement::from_string(backend, sql)).await {
            let msg = e.to_string().to_ascii_lowercase();
            if !msg.contains("duplicate")
                && !msg.contains("already exists")
                && !msg.contains("duplicate column")
            {
                return Err(e.into());
            }
        }
    }
    Ok(())
}
