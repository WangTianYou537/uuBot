use std::sync::Arc;

use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::services::bot::BotRuntime;

/// Shared application state passed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub jwt_encoding: Arc<EncodingKey>,
    pub jwt_decoding: Arc<DecodingKey>,
    /// Tracks in-flight QR logins and running wx-bot monitors.
    pub bot: Arc<BotRuntime>,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config, http: reqwest::Client) -> Self {
        let jwt_encoding = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let jwt_decoding = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        AppState {
            db,
            config: Arc::new(config),
            http,
            jwt_encoding: Arc::new(jwt_encoding),
            jwt_decoding: Arc::new(jwt_decoding),
            bot: Arc::new(BotRuntime::default()),
        }
    }
}
