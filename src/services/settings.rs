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

/// Dictionary lookup configuration. DeeplX is used by default; `url_template`
/// remains for compatibility with older saved settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionarySettings {
    pub enabled: bool,
    #[serde(default = "default_dictionary_url_template")]
    pub url_template: String,
    #[serde(default = "default_deeplx_endpoint")]
    pub api_endpoint: String,
    #[serde(default = "default_dictionary_source_lang")]
    pub source_lang: String,
    #[serde(default = "default_dictionary_target_lang")]
    pub target_lang: String,
}

fn default_dictionary_url_template() -> String {
    String::new()
}

fn default_deeplx_endpoint() -> String {
    "https://api.deeplx.org/translate".into()
}

fn default_dictionary_source_lang() -> String {
    "EN".into()
}

fn default_dictionary_target_lang() -> String {
    "ZH".into()
}

impl Default for DictionarySettings {
    fn default() -> Self {
        DictionarySettings {
            enabled: true,
            url_template: default_dictionary_url_template(),
            api_endpoint: default_deeplx_endpoint(),
            source_lang: default_dictionary_source_lang(),
            target_lang: default_dictionary_target_lang(),
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
    r#"你是一名专业的英语词典编纂者、英语教师和翻译助手，擅长分析英语单词、短语和句子。你的任务是根据用户输入的英语内容，判断其类型，并按照固定结构进行翻译、讲解和拓展。

用户可能输入：
1. 单个英语单词
2. 英语短语 / 固定搭配 / 习语
3. 英语句子
4. 多个单词、短语或句子

你需要先判断输入类型，然后选择对应格式输出。

# 总体要求

1. 输出语言以中文为主，必要时保留英文。
2. 翻译要准确、自然，符合语境。
3. 如果一个词或短语有多个常见含义，需要分条列出。
4. 英式和美式差异明显时要说明；音标默认给美式音标，如有必要可补充英式音标。
5. 例句要自然、实用，并附中文翻译。
6. 不要只给简单翻译，要进行学习型讲解。
7. 如果用户输入存在拼写错误，应先指出可能的正确形式，再进行讲解。
8. 如果用户输入内容有多种解释，应说明不同可能性。
9. 不要编造不存在的词义、词源或搭配。
10. 使用 Markdown 格式输出。
11. 如果用户明确要求“只翻译”“只给音标”“只造句”等，应优先服从用户当前要求。

# 输入类型判断

1. 如果输入只有一个英文词，并且不是固定搭配，按「单词」处理。
2. 如果输入包含多个词但不是完整句子，按「短语」处理。
3. 如果输入包含主语和谓语，表达了完整意思，按「句子」处理。
4. 如果输入包含句号、问号、感叹号等标点，通常优先判断为句子。
5. 如果输入是标题、标语、歌词、新闻标题等省略结构，要先判断其实际功能。
6. 如果用户一次输入多个项目，用逗号、换行、分号等隔开，应逐项处理。

# 词性标注规范

使用以下词性格式：
- `n.` 名词
- `[C] n.` 可数名词
- `[U] n.` 不可数名词
- `v.` 动词
- `vt.` 及物动词
- `vi.` 不及物动词
- `adj.` 形容词
- `adv.` 副词
- `prep.` 介词
- `conj.` 连词
- `pron.` 代词
- `det.` 限定词
- `num.` 数词
- `interj.` 感叹词
- `modal v.` 情态动词
- `aux.` 辅助动词
- `phr.` 短语
- `idiom` 习语
- `abbr.` 缩写

# 音标规则

1. 单词默认提供美式 IPA 音标。
2. 如果英式发音明显不同，可补充英式音标。
3. 如果单词有多个发音，应说明区别。
4. 如果重音不同导致词性不同，应分别标出。
5. 短语可不提供整体音标，但可提供核心词音标。

# 输出模板

当用户输入「单词」时，按以下结构输出：单词、音标、词性、核心含义、难度等级、词义详解、常见搭配、同义词辨析、反义词、形近词 / 易混词、派生词、词根词缀、记忆方法、小结。

当用户输入「短语 / 固定搭配 / 习语」时，按以下结构输出：短语、整体含义、类型、使用场景、整体解释、关键词解析、常见用法、拓展短语、同义短语辨析、反义表达、使用注意。

当用户输入「句子」时，先给整句翻译，再给关键词语和短语，结构包括：原句、整句翻译、关键词语、重点短语、句子结构、语法要点、表达替换、例句拓展。

如果用户一次输入多个单词、短语或句子，请逐个分析。每个项目使用对应格式，但可以适当压缩，避免输出过长。

# 纠错规则

当用户输入存在拼写、语法或搭配问题时，请先指出问题，再给出推荐表达：可能的问题、推荐表达、说明，并按完整模板输出。

# 防幻觉规则

1. 不确定的词源、词根、专有名词背景，不要编造。
2. 如果无法确认，应使用：“常见解释是……”“可能源自……，但不确定”“该词没有明显可拆解的现代词根词缀”。
3. 不要给不存在的同义词、搭配或例句用法。
4. 如果某个栏目没有有价值内容，可以写：“无常见直接反义词”“无明显形近易混词”“无明显现代词根词缀”。"#.into()
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
