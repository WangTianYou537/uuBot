mod auth;
mod codes;
mod config;
mod db;
mod embed;
mod entities;
mod error;
mod routes;
mod services;
mod state;

use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{EntityTrait, PaginatorTrait};
use tracing_subscriber::{EnvFilter, fmt};

use crate::config::Config;
use crate::entities::admins;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let config = Config::from_env()?;
    tracing::info!("connecting to database: {}", redact_url(&config.database_url));

    let conn = db::connect(&config.database_url).await?;
    db::create_tables(&conn).await?;
    tracing::info!("database schema ready");

    seed_admin(&conn, &config).await?;

    let http = reqwest::Client::builder()
        .user_agent("uuBot/0.1")
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let state = AppState::new(conn, config.clone(), http);

    // Resume message monitors for already-bound WeChat accounts.
    services::bot::resume_active_bots(&state).await;

    let router = routes::app(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on http://{}", config.bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}

/// Create the seed admin from config if no admin exists yet.
async fn seed_admin(conn: &sea_orm::DatabaseConnection, config: &Config) -> anyhow::Result<()> {
    let count = admins::Entity::find().count(conn).await?;
    if count > 0 {
        return Ok(());
    }
    let hash = auth::hash_password(&config.admin_password)
        .map_err(|e| anyhow::anyhow!("failed to hash admin password: {e}"))?;
    let model = admins::ActiveModel {
        username: Set(config.admin_username.clone()),
        password_hash: Set(hash),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    admins::Entity::insert(model).exec(conn).await?;
    tracing::info!(
        "seeded admin account '{}' (change the password after first login)",
        config.admin_username
    );
    Ok(())
}

/// Hide credentials when logging a database URL.
fn redact_url(url: &str) -> String {
    match (url.find("://"), url.find('@')) {
        (Some(scheme_end), Some(at)) if at > scheme_end + 3 => {
            format!("{}://***@{}", &url[..scheme_end], &url[at + 1..])
        }
        _ => url.to_string(),
    }
}
