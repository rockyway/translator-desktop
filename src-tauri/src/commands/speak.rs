//! TTS (Text-to-Speech) command for Google Translate TTS API integration.
//!
//! Uses the Google Translate TTS endpoint to generate audio for text,
//! similar to the web app's server implementation.

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::Serialize;

/// Maximum character limit for TTS (same as web app)
const MAX_TEXT_LENGTH: usize = 200;

/// Error type for speak operations
#[derive(Debug, thiserror::Error)]
pub enum SpeakError {
    #[error("Text cannot be empty")]
    EmptyText,

    #[error("Text exceeds {0} character limit for TTS")]
    TextTooLong(usize),

    #[error("Language code is required")]
    MissingLanguageCode,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Google TTS API error: {0}")]
    ApiError(String),
}

// Implement serialization for Tauri command error handling
impl Serialize for SpeakError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Generate TTS audio using Google Translate TTS API
///
/// # Arguments
/// * `text` - The text to convert to speech (max 200 characters)
/// * `language_code` - Language code for TTS (e.g., "en", "vi", "ja")
///
/// # Returns
/// * Base64-encoded MP3 audio data as a String
#[tauri::command]
pub async fn speak(text: String, language_code: String) -> Result<String, SpeakError> {
    // Validate text
    let text = text.trim();
    if text.is_empty() {
        return Err(SpeakError::EmptyText);
    }

    // Check character limit (same as web app)
    if text.len() > MAX_TEXT_LENGTH {
        return Err(SpeakError::TextTooLong(MAX_TEXT_LENGTH));
    }

    // Validate language code
    let language_code = language_code.trim();
    if language_code.is_empty() {
        return Err(SpeakError::MissingLanguageCode);
    }

    // Build the Google TTS URL
    // Uses the same endpoint as google-translate-api-x library
    let url = format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&client=tw-ob&tl={}&q={}",
        urlencoding::encode(language_code),
        urlencoding::encode(text)
    );

    // Make the request
    let client = Client::new();
    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(SpeakError::ApiError(format!(
            "HTTP {} - {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown error")
        )));
    }

    // Get audio bytes and encode to base64
    let audio_bytes = response.bytes().await?;

    if audio_bytes.is_empty() {
        return Err(SpeakError::ApiError("Empty audio response".to_string()));
    }

    let base64_audio = STANDARD.encode(&audio_bytes);

    Ok(base64_audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speak_empty_text() {
        let result = speak("".to_string(), "en".to_string()).await;
        assert!(matches!(result, Err(SpeakError::EmptyText)));
    }

    #[tokio::test]
    async fn test_speak_whitespace_only() {
        let result = speak("   ".to_string(), "en".to_string()).await;
        assert!(matches!(result, Err(SpeakError::EmptyText)));
    }

    #[tokio::test]
    async fn test_speak_text_too_long() {
        let long_text = "a".repeat(201);
        let result = speak(long_text, "en".to_string()).await;
        assert!(matches!(result, Err(SpeakError::TextTooLong(200))));
    }

    #[tokio::test]
    async fn test_speak_missing_language_code() {
        let result = speak("Hello".to_string(), "".to_string()).await;
        assert!(matches!(result, Err(SpeakError::MissingLanguageCode)));
    }

    #[tokio::test]
    async fn test_speak_whitespace_language_code() {
        let result = speak("Hello".to_string(), "   ".to_string()).await;
        assert!(matches!(result, Err(SpeakError::MissingLanguageCode)));
    }

    #[test]
    fn test_max_text_length_constant() {
        assert_eq!(MAX_TEXT_LENGTH, 200);
    }
}
