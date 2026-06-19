use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{AppError, AppResult};
use crate::services::settings::DictionarySettings;

/// Result of a dictionary lookup. Fields may be empty if not found.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DictionaryResult {
    pub phonetic: String,
    pub definition: String,
    pub example: String,
    pub note: String,
    pub tags: String,
    pub content_markdown: String,
    pub raw_json: String,
}

/// Look up a term using the configured DeeplX endpoint.
pub async fn lookup(
    http: &reqwest::Client,
    cfg: &DictionarySettings,
    term: &str,
) -> AppResult<DictionaryResult> {
    if !cfg.enabled {
        return Err(AppError::BadRequest("词典查询未启用".into()));
    }
    if cfg.api_endpoint.trim().is_empty() {
        return Err(AppError::BadRequest("请先配置 DeeplX Endpoint".into()));
    }

    let body = json!({
        "text": term.trim(),
        "source_lang": cfg.source_lang.trim(),
        "target_lang": cfg.target_lang.trim(),
    });

    let resp = http
        .post(cfg.api_endpoint.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("DeeplX 请求失败: {e}")))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("读取 DeeplX 响应失败: {e}")))?;

    if !status.is_success() {
        return Err(AppError::BadRequest(format!("DeeplX 返回 {status}")));
    }

    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("DeeplX 返回解析失败: {e}")))?;

    Ok(parse_deeplx(term.trim(), &value))
}

fn parse_deeplx(term: &str, value: &serde_json::Value) -> DictionaryResult {
    let translated = value
        .get("data")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("translation").and_then(|v| v.as_str()))
        .or_else(|| value.get("result").and_then(|v| v.as_str()))
        .unwrap_or("")
        .trim()
        .to_string();

    let alternatives = value
        .get("alternatives")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut markdown = format!("## DeeplX 翻译\n\n**原文**：{term}\n\n**译文**：{translated}");
    if !alternatives.is_empty() {
        markdown.push_str("\n\n**候选译法**：\n");
        for alt in &alternatives {
            markdown.push_str(&format!("- {alt}\n"));
        }
    }

    DictionaryResult {
        phonetic: String::new(),
        definition: translated,
        example: String::new(),
        note: if alternatives.is_empty() {
            String::new()
        } else {
            format!("候选译法：{}", alternatives.join("；"))
        },
        tags: "DeeplX, 翻译".into(),
        content_markdown: markdown,
        raw_json: value.to_string(),
    }
}
