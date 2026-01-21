//! Translation command for Google Translate API integration.
//!
//! Uses the unofficial Google Translate API endpoint to translate text
//! between languages, similar to the web app's server implementation.

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Maximum allowed text length for translation (5000 characters)
const MAX_TEXT_LENGTH: usize = 5000;

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

/// Translate text using Google Translate API
///
/// # Arguments
/// * `text` - The text to translate
/// * `from` - Source language code (use "auto" for auto-detection)
/// * `to` - Target language code
///
/// # Returns
/// * `TranslateResult` containing the translated text and detected language
#[tauri::command]
pub async fn translate(
    text: String,
    from: String,
    to: String,
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

    // Build the Google Translate API URL
    // This uses the unofficial gtx client endpoint
    let url = format!(
        "https://translate.google.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&dt=bd&dj=1&q={}",
        urlencoding::encode(from),
        urlencoding::encode(to),
        urlencoding::encode(text)
    );

    // Make the request
    let client = Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(TranslateError::ApiError(format!(
            "HTTP {} - {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown error")
        )));
    }

    let json: serde_json::Value = response.json().await?;

    // Parse the response
    // The response format with dj=1 is a JSON object with "sentences" array
    // Each sentence has "trans" (translated) and "orig" (original) fields
    let translated_text = parse_translated_text(&json)?;
    let detected_language = parse_detected_language(&json);

    Ok(TranslateResult {
        translated_text,
        detected_language,
    })
}

/// Parse translated text from Google Translate API response
fn parse_translated_text(json: &serde_json::Value) -> Result<String, TranslateError> {
    // With dj=1, response is a JSON object with "sentences" array
    if let Some(sentences) = json.get("sentences").and_then(|s| s.as_array()) {
        let translated: String = sentences
            .iter()
            .filter_map(|s| s.get("trans").and_then(|t| t.as_str()))
            .collect();

        if !translated.is_empty() {
            return Ok(translated);
        }
    }

    Err(TranslateError::ParseError)
}

/// Parse detected source language from Google Translate API response
fn parse_detected_language(json: &serde_json::Value) -> Option<String> {
    // The detected language is in "src" field when using dj=1
    json.get("src")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_translated_text() {
        let json = serde_json::json!({
            "sentences": [
                {"trans": "Hello", "orig": "Bonjour"},
                {"trans": " world", "orig": " monde"}
            ],
            "src": "fr"
        });

        let result = parse_translated_text(&json).unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_parse_detected_language() {
        let json = serde_json::json!({
            "sentences": [{"trans": "Hello", "orig": "Bonjour"}],
            "src": "fr"
        });

        let result = parse_detected_language(&json);
        assert_eq!(result, Some("fr".to_string()));
    }

    #[test]
    fn test_parse_missing_language() {
        let json = serde_json::json!({
            "sentences": [{"trans": "Hello", "orig": "Hello"}]
        });

        let result = parse_detected_language(&json);
        assert_eq!(result, None);
    }
}
