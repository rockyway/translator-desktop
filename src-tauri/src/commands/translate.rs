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
    pub metadata: Option<TranslationMetadata>,
}

/// Alternative translation for a word
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativeTranslation {
    pub word: String,
}

/// Definition entry with gloss and optional example
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionEntry {
    pub gloss: String,
    pub example: Option<String>,
}

/// Definition group with part of speech
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub part_of_speech: String,
    pub entries: Vec<DefinitionEntry>,
}

/// Example sentence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub text: String,
}

/// Synonym entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Synonym {
    pub word: String,
}

/// Related word entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWord {
    pub word: String,
}

/// Translation metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMetadata {
    pub examples: Vec<Example>,
    pub definitions: Vec<Definition>,
    pub alternatives: Vec<AlternativeTranslation>,
    pub synonyms: Vec<Synonym>,
    pub related_words: Vec<RelatedWord>,
    pub transliteration: Option<String>,
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
    // dt=t: translation, dt=bd: dictionary, dt=ex: examples, dt=md: definitions
    // dt=ss: synonyms, dt=rw: related words, dt=rm: transliteration
    let url = format!(
        "https://translate.google.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&dt=bd&dt=ex&dt=md&dt=ss&dt=rw&dt=rm&dj=1&q={}",
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

    // Parse metadata (examples, definitions, alternatives, synonyms, related words, transliteration)
    let examples = parse_examples(&json);
    let definitions = parse_definitions(&json);
    let alternatives = parse_alternatives(&json);
    let synonyms = parse_synonyms(&json);
    let related_words = parse_related_words(&json);
    let transliteration = parse_transliteration(&json);

    let metadata = if examples.is_empty() && definitions.is_empty() && alternatives.is_empty()
        && synonyms.is_empty() && related_words.is_empty() && transliteration.is_none() {
        None
    } else {
        Some(TranslationMetadata {
            examples,
            definitions,
            alternatives,
            synonyms,
            related_words,
            transliteration,
        })
    };

    Ok(TranslateResult {
        translated_text,
        detected_language,
        metadata,
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

/// Parse examples from Google Translate API response
fn parse_examples(json: &serde_json::Value) -> Vec<Example> {
    json.get("examples")
        .and_then(|e| e.get("example"))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|ex| {
                    ex.get("text")
                        .and_then(|t| t.as_str())
                        .map(|text| Example {
                            text: text.replace("<b>", "").replace("</b>", ""),
                        })
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default()
}

/// Parse definitions from Google Translate API response
fn parse_definitions(json: &serde_json::Value) -> Vec<Definition> {
    json.get("definitions")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|def| {
                    let pos = def.get("pos")
                        .and_then(|p| p.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let entries: Vec<DefinitionEntry> = def.get("entry")
                        .and_then(|e| e.as_array())
                        .map(|entries| {
                            entries.iter()
                                .filter_map(|entry| {
                                    entry.get("gloss")
                                        .and_then(|g| g.as_str())
                                        .map(|gloss| DefinitionEntry {
                                            gloss: gloss.to_string(),
                                            example: entry.get("example")
                                                .and_then(|e| e.as_str())
                                                .map(|s| s.to_string()),
                                        })
                                })
                                .take(3)
                                .collect()
                        })
                        .unwrap_or_default();

                    if entries.is_empty() { None } else { Some(Definition { part_of_speech: pos, entries }) }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse alternative translations from Google Translate API response
fn parse_alternatives(json: &serde_json::Value) -> Vec<AlternativeTranslation> {
    json.get("alternative_translations")
        .and_then(|at| at.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("alternative"))
        .and_then(|alt| alt.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    a.get("word_postproc")
                        .and_then(|w| w.as_str())
                        .map(|word| AlternativeTranslation { word: word.to_string() })
                })
                .skip(1)  // Skip first (same as main translation)
                .take(5)
                .collect()
        })
        .unwrap_or_default()
}

/// Parse synonyms from Google Translate API response
fn parse_synonyms(json: &serde_json::Value) -> Vec<Synonym> {
    json.get("synsets")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .flat_map(|synset| {
                    synset.get("entry")
                        .and_then(|e| e.as_array())
                        .map(|entries| {
                            entries.iter()
                                .flat_map(|entry| {
                                    entry.get("synonym")
                                        .and_then(|syns| syns.as_array())
                                        .map(|syns| {
                                            syns.iter()
                                                .filter_map(|s| s.as_str().map(|w| Synonym { word: w.to_string() }))
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default()
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .take(10)
                .collect()
        })
        .unwrap_or_default()
}

/// Parse related words from Google Translate API response
fn parse_related_words(json: &serde_json::Value) -> Vec<RelatedWord> {
    json.get("related_words")
        .and_then(|rw| rw.get("word"))
        .and_then(|w| w.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|w| w.as_str().map(|word| RelatedWord { word: word.to_string() }))
                .take(10)
                .collect()
        })
        .unwrap_or_default()
}

/// Parse transliteration from Google Translate API response
fn parse_transliteration(json: &serde_json::Value) -> Option<String> {
    // Try sentences first (for source text romanization)
    json.get("sentences")
        .and_then(|s| s.as_array())
        .and_then(|arr| {
            arr.iter()
                .find_map(|sentence| {
                    sentence.get("src_translit")
                        .or_else(|| sentence.get("translit"))
                        .and_then(|t| t.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
        })
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
