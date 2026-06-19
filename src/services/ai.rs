use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::services::settings::{AiProvider, AiSettings};

/// Word fields returned by AI translation. Shape intentionally mirrors the
/// editable word form so the frontend can prefill without saving.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiTranslationResult {
    #[serde(default)]
    pub phonetic: String,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub example: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub input_type: String,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub content_markdown: String,
    #[serde(default)]
    pub raw_json: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessageResp {
    content: Vec<ClaudeContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResp {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiResp {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: String,
}

/// Call the configured LLM provider and ask for a structured translation of `term`.
pub async fn translate(
    http: &reqwest::Client,
    cfg: &AiSettings,
    term: &str,
) -> AppResult<AiTranslationResult> {
    validate_config(cfg)?;
    let prompt = build_prompt(term);
    match cfg.provider {
        AiProvider::Claude => translate_claude(http, cfg, &prompt).await,
        AiProvider::OpenaiCompatible => translate_openai_compatible(http, cfg, &prompt).await,
        AiProvider::Gemini => translate_gemini(http, cfg, &prompt).await,
    }
}

fn validate_config(cfg: &AiSettings) -> AppResult<()> {
    if !cfg.enabled {
        return Err(AppError::BadRequest("AI 翻译尚未启用".into()));
    }
    if cfg.api_endpoint.trim().is_empty() || cfg.api_key.trim().is_empty() {
        return Err(AppError::BadRequest("请先在后台配置 AI Endpoint 和 API Key".into()));
    }
    if cfg.provider != AiProvider::Gemini && cfg.model.trim().is_empty() {
        return Err(AppError::BadRequest("请先在后台配置 AI 模型名称".into()));
    }
    Ok(())
}

fn build_prompt(term: &str) -> String {
    format!(
        "请分析用户输入：\n\n{term}\n\n\
         你必须只返回 JSON 对象，不要在 JSON 外添加任何解释。字段要求：\n\
         - input_type: word / phrase / sentence / multiple / unknown。\n\
         - phonetic: 美式 IPA 音标；不适用则空字符串。\n\
         - definition: 中文核心释义摘要，多个义项每行一条，适合列表快速浏览。\n\
         - example: 一个最有代表性的英文例句，并附中文翻译，可用换行。\n\
         - note: 简短学习提示、易混点或用法提醒。\n\
         - tags: 2 到 5 个中文标签，用英文逗号分隔。\n\
         - difficulty: 初级 / 中级 / 高级 / 学术 / 专业。\n\
         - content_markdown: 按系统提示词要求输出完整 Markdown 学习型讲解。\n\
         - raw_json: 如无额外结构化信息，返回空字符串。"
    )
}

fn result_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "phonetic": { "type": "string" },
            "definition": { "type": "string" },
            "example": { "type": "string" },
            "note": { "type": "string" },
            "tags": { "type": "string" },
            "input_type": { "type": "string" },
            "difficulty": { "type": "string" },
            "content_markdown": { "type": "string" },
            "raw_json": { "type": "string" }
        },
        "required": ["phonetic", "definition", "example", "note", "tags", "input_type", "difficulty", "content_markdown", "raw_json"],
        "additionalProperties": false
    })
}

async fn translate_claude(
    http: &reqwest::Client,
    cfg: &AiSettings,
    prompt: &str,
) -> AppResult<AiTranslationResult> {
    let body = json!({
        "model": cfg.model.trim(),
        "max_tokens": 6000,
        "system": cfg.system_prompt,
        "messages": [{ "role": "user", "content": prompt }],
        "output_config": {
            "format": {
                "type": "json_schema",
                "schema": result_schema()
            }
        }
    });

    let text = send_json(
        http
            .post(cfg.api_endpoint.trim())
            .header("x-api-key", cfg.api_key.trim())
            .header("anthropic-version", "2023-06-01")
            .json(&body),
    )
    .await?;

    let msg: ClaudeMessageResp = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("解析 Claude 响应失败: {e}")))?;

    if msg.stop_reason.as_deref() == Some("refusal") {
        return Err(AppError::BadRequest("AI 拒绝了本次翻译请求".into()));
    }

    let json_text = msg
        .content
        .into_iter()
        .find_map(|block| match block {
            ClaudeContentBlock::Text { text } => Some(text),
            ClaudeContentBlock::Other => None,
        })
        .ok_or_else(|| AppError::Internal("Claude 响应中没有文本结果".into()))?;

    parse_translation_json(&json_text)
}

async fn translate_openai_compatible(
    http: &reqwest::Client,
    cfg: &AiSettings,
    prompt: &str,
) -> AppResult<AiTranslationResult> {
    let body = json!({
        "model": cfg.model.trim(),
        "messages": [
            { "role": "system", "content": cfg.system_prompt },
            { "role": "user", "content": prompt }
        ],
        "response_format": { "type": "json_object" }
    });

    let text = send_json(
        http
            .post(cfg.api_endpoint.trim())
            .bearer_auth(cfg.api_key.trim())
            .json(&body),
    )
    .await?;

    let msg: OpenAiChatResp = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("解析 OpenAI 兼容响应失败: {e}")))?;
    let json_text = msg
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .ok_or_else(|| AppError::Internal("OpenAI 兼容响应中没有文本结果".into()))?;

    parse_translation_json(&json_text)
}

async fn translate_gemini(
    http: &reqwest::Client,
    cfg: &AiSettings,
    prompt: &str,
) -> AppResult<AiTranslationResult> {
    let endpoint = gemini_endpoint(cfg)?;
    let body = json!({
        "system_instruction": {
            "parts": [{ "text": cfg.system_prompt }]
        },
        "contents": [{
            "role": "user",
            "parts": [{ "text": prompt }]
        }],
        "generationConfig": {
            "response_mime_type": "application/json",
            "response_schema": result_schema()
        }
    });

    let text = send_json(http.post(endpoint).json(&body)).await?;
    let msg: GeminiResp = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("解析 Gemini 响应失败: {e}")))?;
    let json_text = msg
        .candidates
        .into_iter()
        .flat_map(|candidate| candidate.content.parts)
        .map(|part| part.text)
        .collect::<String>();

    if json_text.trim().is_empty() {
        return Err(AppError::Internal("Gemini 响应中没有文本结果".into()));
    }

    parse_translation_json(&json_text)
}

fn gemini_endpoint(cfg: &AiSettings) -> AppResult<reqwest::Url> {
    let mut url = reqwest::Url::parse(cfg.api_endpoint.trim())
        .map_err(|e| AppError::BadRequest(format!("Gemini Endpoint 无效: {e}")))?;
    if !url.query_pairs().any(|(key, _)| key == "key") {
        url.query_pairs_mut().append_pair("key", cfg.api_key.trim());
    }
    Ok(url)
}

async fn send_json(builder: reqwest::RequestBuilder) -> AppResult<String> {
    let resp = builder
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("AI 翻译请求失败: {e}")))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("读取 AI 响应失败: {e}")))?;

    if !status.is_success() {
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(extract_error_message)
            .unwrap_or_else(|| format!("AI 服务返回 {status}"));
        return Err(AppError::BadRequest(message));
    }

    Ok(text)
}

fn extract_error_message(value: serde_json::Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/error/status"))
        .or_else(|| value.pointer("/error"))
        .and_then(|message| match message {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(_) => message
                .get("message")
                .and_then(|m| m.as_str())
                .map(str::to_string),
            _ => None,
        })
}

fn parse_translation_json(text: &str) -> AppResult<AiTranslationResult> {
    serde_json::from_str::<AiTranslationResult>(text.trim()).map_err(|e| {
        let preview = text.trim().chars().take(50).collect::<String>();
        AppError::Internal(format!("解析 AI 翻译 JSON 失败: {e}; 原文前50字: {preview}"))
    })
}
