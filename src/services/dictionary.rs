use serde::Serialize;

use crate::error::AppError;
use crate::services::settings::DictionarySettings;

/// Result of a dictionary lookup. Fields may be empty if not found.
#[derive(Debug, Default, Serialize)]
pub struct DictionaryResult {
    pub phonetic: String,
    pub definition: String,
    pub example: String,
}

/// Look up a term using the configured dictionary endpoint.
///
/// The default endpoint (dictionaryapi.dev) returns an array of entries; we
/// parse it loosely so a change in shape degrades gracefully rather than erroring.
pub async fn lookup(
    http: &reqwest::Client,
    cfg: &DictionarySettings,
    term: &str,
) -> Result<DictionaryResult, AppError> {
    if !cfg.enabled {
        return Err(AppError::BadRequest("词典查询未启用".into()));
    }
    let url = cfg
        .url_template
        .replace("{word}", &urlencoding(term.trim()));

    let resp = http
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("词典请求失败: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::NotFound("未找到该单词的释义".into()));
    }

    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("词典返回解析失败: {e}")))?;

    Ok(parse_dictionaryapi(&value))
}

/// Minimal percent-encoding for path segments (avoids pulling another dep).
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn parse_dictionaryapi(value: &serde_json::Value) -> DictionaryResult {
    let mut result = DictionaryResult::default();
    let entry = value.as_array().and_then(|a| a.first());
    let Some(entry) = entry else {
        return result;
    };

    // Phonetic: top-level "phonetic", else first non-empty in "phonetics".
    if let Some(p) = entry.get("phonetic").and_then(|v| v.as_str()) {
        result.phonetic = p.to_string();
    }
    if result.phonetic.is_empty() {
        if let Some(arr) = entry.get("phonetics").and_then(|v| v.as_array()) {
            for ph in arr {
                if let Some(t) = ph.get("text").and_then(|v| v.as_str()) {
                    if !t.is_empty() {
                        result.phonetic = t.to_string();
                        break;
                    }
                }
            }
        }
    }

    // Collect up to a few definitions across meanings, grabbing the first example.
    let mut defs: Vec<String> = Vec::new();
    if let Some(meanings) = entry.get("meanings").and_then(|v| v.as_array()) {
        for meaning in meanings {
            let pos = meaning
                .get("partOfSpeech")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(definitions) = meaning.get("definitions").and_then(|v| v.as_array()) {
                for d in definitions {
                    if let Some(text) = d.get("definition").and_then(|v| v.as_str()) {
                        if pos.is_empty() {
                            defs.push(text.to_string());
                        } else {
                            defs.push(format!("({pos}) {text}"));
                        }
                    }
                    if result.example.is_empty() {
                        if let Some(ex) = d.get("example").and_then(|v| v.as_str()) {
                            result.example = ex.to_string();
                        }
                    }
                    if defs.len() >= 3 {
                        break;
                    }
                }
            }
            if defs.len() >= 3 {
                break;
            }
        }
    }
    result.definition = defs.join("\n");
    result
}
