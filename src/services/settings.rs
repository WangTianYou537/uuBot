use sea_orm::{ActiveValue::Set, EntityTrait};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::entities::settings;
use crate::error::AppError;

pub const KEY_SMTP: &str = "smtp";
pub const KEY_OAUTH: &str = "oauth";
pub const KEY_DICTIONARY: &str = "dictionary";
pub const KEY_AI: &str = "ai";
pub const KEY_BOT: &str = "bot";

/// SMTP / outbound email configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    /// Use implicit TLS (port 465). When false, STARTTLS is used.
    pub use_implicit_tls: bool,
}

impl Default for SmtpSettings {
    fn default() -> Self {
        SmtpSettings {
            enabled: false,
            host: String::new(),
            port: 587,
            username: String::new(),
            password: String::new(),
            from_email: String::new(),
            from_name: "uuBot".into(),
            use_implicit_tls: false,
        }
    }
}

/// Aggregate-login (mapay.cn) provider configuration.
/// Maps the doc's `appid`/`appkey` to clientId/secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSettings {
    pub base_url: String,
    pub appid: String,
    pub appkey: String,
}

impl Default for OAuthSettings {
    fn default() -> Self {
        OAuthSettings {
            base_url: "https://login.mapay.cn/connect.php".into(),
            appid: String::new(),
            appkey: String::new(),
        }
    }
}

/// Dictionary lookup configuration. `url_template` must contain `{word}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionarySettings {
    pub enabled: bool,
    pub url_template: String,
}

impl Default for DictionarySettings {
    fn default() -> Self {
        DictionarySettings {
            enabled: true,
            url_template: "https://api.dictionaryapi.dev/api/v2/entries/en/{word}".into(),
        }
    }
}

/// AI translation provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    OpenaiCompatible,
    Claude,
    Gemini,
}

fn default_ai_provider() -> AiProvider {
    AiProvider::Claude
}

fn default_ai_endpoint() -> String {
    "https://api.anthropic.com/v1/messages".into()
}

fn default_ai_model() -> String {
    "claude-opus-4-8".into()
}

fn default_ai_system_prompt() -> String {
    "你是 uuBot 的词典助手。根据用户给出的单词或短语，返回准确、简洁、适合中文使用者记忆的词条信息。只返回符合 JSON schema 的内容，不要添加额外解释。".into()
}

/// AI translation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ai_provider")]
    pub provider: AiProvider,
    #[serde(default = "default_ai_endpoint")]
    pub api_endpoint: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default = "default_ai_system_prompt")]
    pub system_prompt: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            enabled: false,
            provider: default_ai_provider(),
            api_endpoint: default_ai_endpoint(),
            api_key: String::new(),
            model: default_ai_model(),
            system_prompt: default_ai_system_prompt(),
        }
    }
}

/// WeChat bot integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_bot_max_bindings")]
    pub max_bindings_per_user: u32,
    #[serde(default)]
    pub webhook_secret: String,
}

fn default_bot_max_bindings() -> u32 {
    3
}

impl Default for BotSettings {
    fn default() -> Self {
        BotSettings {
            enabled: false,
            max_bindings_per_user: default_bot_max_bindings(),
            webhook_secret: String::new(),
        }
    }
}

/// Fetch a typed settings value, falling back to `Default` when unset or invalid.
pub async fn get<T>(db: &sea_orm::DatabaseConnection, key: &str) -> Result<T, AppError>
where
    T: DeserializeOwned + Default,
{
    let row = settings::Entity::find_by_id(key.to_string())
        .one(db)
        .await?;
    match row {
        Some(m) => Ok(serde_json::from_str(&m.value).unwrap_or_default()),
        None => Ok(T::default()),
    }
}

/// Upsert a typed settings value.
pub async fn set<T>(db: &sea_orm::DatabaseConnection, key: &str, value: &T) -> Result<(), AppError>
where
    T: Serialize,
{
    let json = serde_json::to_string(value)
        .map_err(|e| AppError::Internal(format!("failed to serialize settings: {e}")))?;
    let model = settings::ActiveModel {
        key: Set(key.to_string()),
        value: Set(json),
    };
    // Upsert on the primary key.
    settings::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(settings::Column::Key)
                .update_column(settings::Column::Value)
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}
