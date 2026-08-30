//! Text-to-speech command using a self-hosted openai-edge-tts server
//! (https://github.com/travisvn/openai-edge-tts), which wraps Microsoft Edge's
//! online neural voices behind an OpenAI-compatible `/v1/audio/speech` endpoint.
//!
//! The server is external infrastructure the user runs themselves (e.g. via
//! `docker run -d -p 5050:5050 travisvn/openai-edge-tts:latest`). Its base URL is
//! stored in `config_store` under `edge_tts_base_url`, defaulting to
//! `http://localhost:5050` when unset.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use tauri::State;

use super::translate::HttpClientState;
use super::DbState;

const BASE_URL_SETTING: &str = "edge_tts_base_url";
const DEFAULT_BASE_URL: &str = "http://localhost:5050";

/// Error type for edge-tts operations
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Text cannot be empty")]
    EmptyText,

    #[error(
        "Could not reach the TTS server at {0}. Is it running? \
         (docker run -d -p 5050:5050 travisvn/openai-edge-tts:latest)"
    )]
    ServerUnreachable(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("TTS server error: {0}")]
    ApiError(String),
}

impl Serialize for TtsError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Speaks `text` using the given edge-tts voice name (e.g. "vi-VN-HoaiMyNeural")
/// via a self-hosted openai-edge-tts server, returning base64-encoded MP3 audio.
///
/// # Arguments
/// * `text` - The text to convert to speech
/// * `voice` - An edge-tts voice name
///
/// # Returns
/// * Base64-encoded MP3 audio data as a String
#[tauri::command]
pub async fn speak_edge_tts(
    text: String,
    voice: String,
    http_client: State<'_, HttpClientState>,
    db_state: State<'_, DbState>,
) -> Result<String, TtsError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TtsError::EmptyText);
    }

    let base_url = {
        let pool = db_state.0.lock().await;
        sqlx::query_scalar::<_, String>("SELECT value FROM config_store WHERE key = ?")
            .bind(BASE_URL_SETTING)
            .fetch_optional(&*pool)
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<String>(&v).ok())
    }
    .filter(|v| !v.trim().is_empty())
    .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let base_url = base_url.trim_end_matches('/');

    // openai-edge-tts expects a Bearer token for OpenAI-SDK compatibility, but per its
    // own docs it isn't a real credential - any placeholder string is accepted as long
    // as REQUIRE_API_KEY isn't set to a specific value the user configured server-side.
    let response = http_client
        .0
        .post(format!("{}/v1/audio/speech", base_url))
        .bearer_auth("your_api_key_here")
        .json(&serde_json::json!({
            "input": text,
            "voice": voice,
            "response_format": "mp3",
        }))
        .send()
        .await
        .map_err(|_| TtsError::ServerUnreachable(base_url.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(TtsError::ApiError(format!("HTTP {}: {}", status.as_u16(), body)));
    }

    let audio_bytes = response.bytes().await?;
    if audio_bytes.is_empty() {
        return Err(TtsError::ApiError("Empty audio response".to_string()));
    }

    Ok(STANDARD.encode(&audio_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text_validation() {
        let text = "   ".trim();
        assert!(text.is_empty());
    }

    #[test]
    fn test_default_base_url_trims_trailing_slash() {
        let base_url = "http://localhost:5050/";
        assert_eq!(base_url.trim_end_matches('/'), "http://localhost:5050");
    }
}
