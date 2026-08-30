//! Translation command using the official Google Cloud Translation API (v2 Basic).
//!
//! Requires a user-supplied API key stored in `config_store` under the key
//! `google_translate_api_key` (see `commands::settings::{get_setting, set_setting}`).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tauri::State;

use super::DbState;

/// Shared HTTP client state for reuse across requests
/// Creating a client is expensive (TLS setup, connection pooling)
/// so we reuse it for all translation requests
pub struct HttpClientState(pub Arc<Client>);

impl Default for HttpClientState {
    fn default() -> Self {
        Self(Arc::new(
            Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                // Bounds how long a slow/hung request (e.g. a cold connection to an
                // external API on the very first call of a session) can block a caller.
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to create HTTP client"),
        ))
    }
}

/// Maximum allowed text length for translation (5000 characters)
const MAX_TEXT_LENGTH: usize = 5000;

/// Config store key holding the Google Cloud Translation API key
const API_KEY_SETTING: &str = "google_translate_api_key";

/// Result of a translation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResult {
    pub translated_text: String,
    pub detected_language: Option<String>,
}

/// Error type for translation operations
#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("Text cannot be empty")]
    EmptyText,

    #[error("Text exceeds maximum length of {0} characters")]
    TextTooLong(usize),

    #[error("Target language is required")]
    MissingTargetLanguage,

    #[error("Google Cloud Translation API key not configured. Add one in Settings.")]
    MissingApiKey,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse translation response")]
    ParseError,

    #[error("Google Translate API error: {0}")]
    ApiError(String),
}

// Implement serialization for Tauri command error handling
impl Serialize for TranslateError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Response shape from Google Cloud Translation API v2
#[derive(Debug, Deserialize)]
struct GoogleTranslateResponse {
    data: GoogleTranslateData,
}

#[derive(Debug, Deserialize)]
struct GoogleTranslateData {
    translations: Vec<GoogleTranslation>,
}

#[derive(Debug, Deserialize)]
struct GoogleTranslation {
    #[serde(rename = "translatedText")]
    translated_text: String,
    #[serde(rename = "detectedSourceLanguage")]
    detected_source_language: Option<String>,
}

/// Error response shape from Google Cloud Translation API v2
#[derive(Debug, Deserialize)]
struct GoogleErrorResponse {
    error: GoogleErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GoogleErrorDetail {
    message: String,
}

/// Translate text using the official Google Cloud Translation API (v2 Basic)
///
/// # Arguments
/// * `text` - The text to translate
/// * `from` - Source language code (use "auto" for auto-detection)
/// * `to` - Target language code
/// * `http_client` - Shared HTTP client state
/// * `db_state` - Database state, used to look up the stored API key
///
/// # Returns
/// * `TranslateResult` containing the translated text and detected language
#[tauri::command]
pub async fn translate(
    text: String,
    from: String,
    to: String,
    http_client: State<'_, HttpClientState>,
    db_state: State<'_, DbState>,
) -> Result<TranslateResult, TranslateError> {
    // Validate input
    let text = text.trim();
    if text.is_empty() {
        return Err(TranslateError::EmptyText);
    }

    if text.len() > MAX_TEXT_LENGTH {
        return Err(TranslateError::TextTooLong(MAX_TEXT_LENGTH));
    }

    let to = to.trim();
    if to.is_empty() {
        return Err(TranslateError::MissingTargetLanguage);
    }

    // Use "auto" for empty source language
    let from = if from.trim().is_empty() {
        "auto"
    } else {
        from.trim()
    };

    let api_key = {
        let pool = db_state.0.lock().await;
        sqlx::query_scalar::<_, String>("SELECT value FROM config_store WHERE key = ?")
            .bind(API_KEY_SETTING)
            .fetch_optional(&*pool)
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<String>(&v).ok())
    }
    .ok_or(TranslateError::MissingApiKey)?;

    if api_key.trim().is_empty() {
        return Err(TranslateError::MissingApiKey);
    }

    // Build the request form body. Omit `source` for auto-detect - Google's API
    // auto-detects the source language when it's left out.
    let mut params = vec![("q", text), ("target", to), ("format", "text")];
    if from != "auto" {
        params.push(("source", from));
    }

    let response = http_client
        .0
        .post("https://translation.googleapis.com/language/translate/v2")
        .query(&[("key", api_key.as_str())])
        .form(&params)
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        let message = serde_json::from_str::<GoogleErrorResponse>(&body)
            .map(|e| e.error.message)
            .unwrap_or_else(|_| format!("HTTP {}", status.as_u16()));
        return Err(TranslateError::ApiError(message));
    }

    let parsed: GoogleTranslateResponse =
        serde_json::from_str(&body).map_err(|_| TranslateError::ParseError)?;

    let translation = parsed
        .data
        .translations
        .into_iter()
        .next()
        .ok_or(TranslateError::ParseError)?;

    Ok(TranslateResult {
        translated_text: translation.translated_text,
        detected_language: translation.detected_source_language,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_translate_response() {
        let json = r#"{"data":{"translations":[{"translatedText":"Xin chào","detectedSourceLanguage":"en"}]}}"#;
        let parsed: GoogleTranslateResponse = serde_json::from_str(json).unwrap();
        let translation = parsed.data.translations.into_iter().next().unwrap();
        assert_eq!(translation.translated_text, "Xin chào");
        assert_eq!(translation.detected_source_language, Some("en".to_string()));
    }

    #[test]
    fn test_parse_translate_response_no_detected_language() {
        let json = r#"{"data":{"translations":[{"translatedText":"Hola"}]}}"#;
        let parsed: GoogleTranslateResponse = serde_json::from_str(json).unwrap();
        let translation = parsed.data.translations.into_iter().next().unwrap();
        assert_eq!(translation.translated_text, "Hola");
        assert_eq!(translation.detected_source_language, None);
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{"error":{"code":429,"message":"Quota exceeded","status":"RESOURCE_EXHAUSTED"}}"#;
        let parsed: GoogleErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.error.message, "Quota exceeded");
    }
}
