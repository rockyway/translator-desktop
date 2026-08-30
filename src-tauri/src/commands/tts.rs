//! Text-to-speech command using the official Google Cloud Text-to-Speech API.
//!
//! Uses Chirp 3: HD voices (docs.cloud.google.com/text-to-speech/docs/chirp3-hd) for
//! every supported language except Malay and Traditional Chinese, which Chirp 3 HD
//! doesn't cover yet - those always use a fixed Standard-tier voice regardless of the
//! user's voice preference. Swapping the tier for a given language (e.g. to WaveNet or
//! Neural2) only means changing that language's entry in `resolve_locale` below;
//! nothing else in this file is tier-specific.
//!
//! The Chirp 3 HD voice *personality* (Kore, Puck, Aoede, ...) is user-configurable:
//! a global default (`google_tts_default_voice`, config_store) applies to every
//! language, optionally overridden per-language via `google_tts_voice_overrides`
//! (a JSON object of language code -> voice name), both set from Settings.
//!
//! Authenticates with an API key stored in `config_store` under `google_tts_api_key`.
//! If that's unset, falls back to the shared `google_translate_api_key` (see
//! `commands::translate`) so a single Google Cloud API key can drive both features.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::State;

use super::translate::HttpClientState;
use super::DbState;

/// Config store key holding a Text-to-Speech-specific API key (optional override).
const TTS_API_KEY_SETTING: &str = "google_tts_api_key";

/// Config store key holding the Google Cloud Translation API key, reused as the
/// default TTS key when no TTS-specific key is configured.
const TRANSLATE_API_KEY_SETTING: &str = "google_translate_api_key";

/// Config store key holding the default Chirp 3 HD voice name, used for any language
/// without its own entry in `VOICE_OVERRIDES_SETTING`.
const DEFAULT_VOICE_SETTING: &str = "google_tts_default_voice";

/// Config store key holding a JSON object of `{ languageCode: voiceName }` overrides.
const VOICE_OVERRIDES_SETTING: &str = "google_tts_voice_overrides";

/// Voice used when nothing is configured yet.
const FALLBACK_VOICE: &str = "Kore";

/// All Chirp 3 HD voice personality names (docs.cloud.google.com/text-to-speech/docs/chirp3-hd).
/// Used to reject a stale/invalid stored voice name rather than send it to the API.
const CHIRP3_HD_VOICES: &[&str] = &[
    "Achernar",
    "Achird",
    "Algenib",
    "Algieba",
    "Alnilam",
    "Aoede",
    "Autonoe",
    "Callirrhoe",
    "Charon",
    "Despina",
    "Enceladus",
    "Erinome",
    "Fenrir",
    "Gacrux",
    "Iapetus",
    "Kore",
    "Laomedeia",
    "Leda",
    "Orus",
    "Pulcherrima",
    "Puck",
    "Rasalgethi",
    "Sadachbia",
    "Sadaltager",
    "Schedar",
    "Sulafat",
    "Umbriel",
    "Vindemiatrix",
    "Zephyr",
    "Zubenelgenubi",
];

/// Error type for Google Cloud Text-to-Speech operations
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("Text cannot be empty")]
    EmptyText,

    #[error("Text-to-speech is not yet supported for language \"{0}\"")]
    UnsupportedLanguage(String),

    #[error("Google Cloud Text-to-Speech API key not configured. Add one in Settings.")]
    MissingApiKey,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse text-to-speech response")]
    ParseError,

    #[error("Google Cloud Text-to-Speech API error: {0}")]
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

#[derive(Debug, Deserialize)]
struct GoogleTtsResponse {
    #[serde(rename = "audioContent")]
    audio_content: String,
}

#[derive(Debug, Deserialize)]
struct GoogleErrorResponse {
    error: GoogleErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GoogleErrorDetail {
    message: String,
}

/// Whether a locale has real Chirp 3 HD voices, or must fall back to a fixed
/// Standard-tier voice because Chirp 3 HD doesn't cover it (yet).
enum VoiceAvailability {
    Chirp3Hd,
    FixedStandard(&'static str),
}

/// Maps this app's language codes to a Google Cloud TTS locale.
fn resolve_locale(language_code: &str) -> Option<(&'static str, VoiceAvailability)> {
    use VoiceAvailability::{Chirp3Hd, FixedStandard};
    match language_code.to_lowercase().as_str() {
        "ar" => Some(("ar-XA", Chirp3Hd)),
        "bg" => Some(("bg-BG", Chirp3Hd)),
        "zh-cn" => Some(("cmn-CN", Chirp3Hd)),
        // Chirp 3 HD doesn't cover Traditional Chinese (Taiwan) yet.
        "zh-tw" => Some(("cmn-TW", FixedStandard("cmn-TW-Standard-A"))),
        "cs" => Some(("cs-CZ", Chirp3Hd)),
        "da" => Some(("da-DK", Chirp3Hd)),
        "nl" => Some(("nl-NL", Chirp3Hd)),
        "en" => Some(("en-US", Chirp3Hd)),
        "fi" => Some(("fi-FI", Chirp3Hd)),
        "fr" => Some(("fr-FR", Chirp3Hd)),
        "de" => Some(("de-DE", Chirp3Hd)),
        "el" => Some(("el-GR", Chirp3Hd)),
        "he" => Some(("he-IL", Chirp3Hd)),
        "hi" => Some(("hi-IN", Chirp3Hd)),
        "hu" => Some(("hu-HU", Chirp3Hd)),
        "id" => Some(("id-ID", Chirp3Hd)),
        "it" => Some(("it-IT", Chirp3Hd)),
        "ja" => Some(("ja-JP", Chirp3Hd)),
        "ko" => Some(("ko-KR", Chirp3Hd)),
        // Chirp 3 HD doesn't cover Malay yet.
        "ms" => Some(("ms-MY", FixedStandard("ms-MY-Standard-A"))),
        "no" => Some(("nb-NO", Chirp3Hd)),
        "pl" => Some(("pl-PL", Chirp3Hd)),
        "pt" => Some(("pt-BR", Chirp3Hd)),
        "ro" => Some(("ro-RO", Chirp3Hd)),
        "ru" => Some(("ru-RU", Chirp3Hd)),
        "es" => Some(("es-ES", Chirp3Hd)),
        "sv" => Some(("sv-SE", Chirp3Hd)),
        "th" => Some(("th-TH", Chirp3Hd)),
        "tr" => Some(("tr-TR", Chirp3Hd)),
        "uk" => Some(("uk-UA", Chirp3Hd)),
        "vi" => Some(("vi-VN", Chirp3Hd)),
        _ => None,
    }
}

async fn get_config_value(pool: &SqlitePool, key: &str) -> Option<String> {
    get_config_json::<String>(pool, key).await
}

async fn get_config_json<T: DeserializeOwned>(pool: &SqlitePool, key: &str) -> Option<T> {
    sqlx::query_scalar::<_, String>("SELECT value FROM config_store WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_str::<T>(&v).ok())
}

/// Resolves the Chirp 3 HD voice name to use for `language_code`: the per-language
/// override if one is set, else the configured default, else `FALLBACK_VOICE`. A
/// stored value that isn't a real Chirp 3 HD voice name (e.g. a stale setting from a
/// removed voice) is treated as unset rather than sent to the API.
async fn resolve_voice_name(pool: &SqlitePool, language_code: &str) -> String {
    let normalized = language_code.to_lowercase();

    let overrides = get_config_json::<HashMap<String, String>>(pool, VOICE_OVERRIDES_SETTING)
        .await
        .unwrap_or_default();

    let candidate = match overrides
        .into_iter()
        .find(|(code, _)| code.to_lowercase() == normalized)
    {
        Some((_, voice)) => Some(voice),
        None => get_config_value(pool, DEFAULT_VOICE_SETTING).await,
    };

    candidate
        .filter(|voice| CHIRP3_HD_VOICES.contains(&voice.as_str()))
        .unwrap_or_else(|| FALLBACK_VOICE.to_string())
}

/// Speaks `text` in the given app language code using the Google Cloud Text-to-Speech
/// API, returning base64-encoded MP3 audio.
///
/// # Arguments
/// * `text` - The text to convert to speech
/// * `language_code` - This app's language code (e.g. "en", "vi", "zh-CN")
///
/// # Returns
/// * Base64-encoded MP3 audio data as a String
#[tauri::command]
pub async fn speak_google_tts(
    text: String,
    language_code: String,
    http_client: State<'_, HttpClientState>,
    db_state: State<'_, DbState>,
) -> Result<String, TtsError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TtsError::EmptyText);
    }

    let (locale, availability) = resolve_locale(&language_code)
        .ok_or_else(|| TtsError::UnsupportedLanguage(language_code.clone()))?;

    let (voice_name, api_key) = {
        let pool = db_state.0.lock().await;

        let voice_name = match availability {
            VoiceAvailability::FixedStandard(name) => name.to_string(),
            VoiceAvailability::Chirp3Hd => {
                let voice = resolve_voice_name(&pool, &language_code).await;
                format!("{locale}-Chirp3-HD-{voice}")
            }
        };

        let api_key = match get_config_value(&pool, TTS_API_KEY_SETTING).await {
            Some(key) if !key.trim().is_empty() => Some(key),
            _ => get_config_value(&pool, TRANSLATE_API_KEY_SETTING).await,
        };

        (voice_name, api_key)
    };

    let api_key = api_key
        .filter(|key| !key.trim().is_empty())
        .ok_or(TtsError::MissingApiKey)?;

    let response = http_client
        .0
        .post("https://texttospeech.googleapis.com/v1/text:synthesize")
        .query(&[("key", api_key.as_str())])
        .json(&serde_json::json!({
            "input": { "text": text },
            "voice": { "languageCode": locale, "name": voice_name },
            "audioConfig": { "audioEncoding": "MP3" },
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        let message = serde_json::from_str::<GoogleErrorResponse>(&body)
            .map(|e| e.error.message)
            .unwrap_or_else(|_| format!("HTTP {}", status.as_u16()));
        return Err(TtsError::ApiError(message));
    }

    let parsed: GoogleTtsResponse =
        serde_json::from_str(&body).map_err(|_| TtsError::ParseError)?;

    if parsed.audio_content.is_empty() {
        return Err(TtsError::ApiError("Empty audio response".to_string()));
    }

    // Validate it's real base64 before handing it to the frontend's atob().
    STANDARD
        .decode(&parsed.audio_content)
        .map_err(|_| TtsError::ParseError)?;

    Ok(parsed.audio_content)
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
    fn test_resolve_locale_chirp3_hd() {
        let (locale, availability) = resolve_locale("vi").unwrap();
        assert_eq!(locale, "vi-VN");
        assert!(matches!(availability, VoiceAvailability::Chirp3Hd));

        let (locale, availability) = resolve_locale("EN").unwrap();
        assert_eq!(locale, "en-US");
        assert!(matches!(availability, VoiceAvailability::Chirp3Hd));

        let (locale, _) = resolve_locale("zh-CN").unwrap();
        assert_eq!(locale, "cmn-CN");
    }

    #[test]
    fn test_resolve_locale_fixed_standard_fallback() {
        let (locale, availability) = resolve_locale("ms").unwrap();
        assert_eq!(locale, "ms-MY");
        assert!(matches!(availability, VoiceAvailability::FixedStandard("ms-MY-Standard-A")));

        let (locale, availability) = resolve_locale("zh-tw").unwrap();
        assert_eq!(locale, "cmn-TW");
        assert!(matches!(availability, VoiceAvailability::FixedStandard("cmn-TW-Standard-A")));
    }

    #[test]
    fn test_resolve_locale_unknown_language() {
        assert!(resolve_locale("xx").is_none());
    }

    #[test]
    fn test_chirp3_voice_list_has_no_duplicates() {
        let mut sorted = CHIRP3_HD_VOICES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), CHIRP3_HD_VOICES.len());
    }

    #[test]
    fn test_fallback_voice_is_valid() {
        assert!(CHIRP3_HD_VOICES.contains(&FALLBACK_VOICE));
    }

    #[test]
    fn test_parse_tts_response() {
        let json = r#"{"audioContent":"SGVsbG8="}"#;
        let parsed: GoogleTtsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.audio_content, "SGVsbG8=");
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{"error":{"code":400,"message":"Invalid voice","status":"INVALID_ARGUMENT"}}"#;
        let parsed: GoogleErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.error.message, "Invalid voice");
    }
}
