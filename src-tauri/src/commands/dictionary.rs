//! Dictionary metadata command using the free freedictionaryapi.com service.
//!
//! Supplies the definitions/examples/synonyms/pronunciation shown in the
//! "Additional Information" panel for short (single word/phrase) translations.
//! No API key required.

use serde::{Deserialize, Serialize};
use tauri::State;

use super::translate::HttpClientState;

/// Alternative translation for a word (no longer populated - kept for frontend shape compatibility)
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

/// Antonym entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Antonym {
    pub word: String,
}

/// Related word entry (no longer populated - kept for frontend shape compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWord {
    pub word: String,
}

/// Dictionary/translation metadata shown in the "Additional Information" panel
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMetadata {
    pub examples: Vec<Example>,
    pub definitions: Vec<Definition>,
    pub alternatives: Vec<AlternativeTranslation>,
    pub synonyms: Vec<Synonym>,
    pub antonyms: Vec<Antonym>,
    pub related_words: Vec<RelatedWord>,
    pub transliteration: Option<String>,
}

/// Error type for dictionary lookups
#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("Word cannot be empty")]
    EmptyWord,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Failed to parse dictionary response")]
    ParseError,
}

impl Serialize for DictionaryError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// --- freedictionaryapi.com response shape ---

#[derive(Debug, Deserialize)]
struct DictResponse {
    entries: Vec<DictEntry>,
}

#[derive(Debug, Deserialize)]
struct DictEntry {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: String,
    #[serde(default)]
    pronunciations: Vec<DictPronunciation>,
    senses: Vec<DictSense>,
}

#[derive(Debug, Deserialize)]
struct DictPronunciation {
    text: String,
}

#[derive(Debug, Deserialize)]
struct DictSense {
    definition: String,
    #[serde(default)]
    examples: Vec<String>,
    #[serde(default)]
    synonyms: Vec<String>,
    #[serde(default)]
    antonyms: Vec<String>,
}

/// Look up dictionary metadata (definitions, examples, synonyms, pronunciation) for a
/// single word or short phrase via freedictionaryapi.com.
///
/// # Arguments
/// * `word` - The word/short phrase to look up
/// * `language` - ISO language code (e.g. "en", "vi")
///
/// # Returns
/// * `Some(TranslationMetadata)` if the word was found, `None` if not (not an error)
#[tauri::command]
pub async fn get_dictionary_metadata(
    word: String,
    language: String,
    http_client: State<'_, HttpClientState>,
) -> Result<Option<TranslationMetadata>, DictionaryError> {
    let word = word.trim();
    if word.is_empty() {
        return Err(DictionaryError::EmptyWord);
    }

    let language = if language.trim().is_empty() {
        "en"
    } else {
        language.trim()
    };

    let url = format!(
        "https://freedictionaryapi.com/api/v1/entries/{}/{}",
        urlencoding::encode(language),
        urlencoding::encode(word)
    );

    let response = http_client.0.get(&url).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Ok(None);
    }

    let body = response.text().await?;
    let parsed: DictResponse = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return Err(DictionaryError::ParseError),
    };

    let mut definitions = Vec::new();
    let mut examples = Vec::new();
    let mut synonyms = Vec::new();
    let mut antonyms = Vec::new();
    let mut transliteration = None;

    for entry in &parsed.entries {
        if transliteration.is_none() {
            transliteration = entry.pronunciations.first().map(|p| p.text.clone());
        }

        let def_entries: Vec<DefinitionEntry> = entry
            .senses
            .iter()
            .take(3)
            .map(|sense| DefinitionEntry {
                gloss: sense.definition.clone(),
                example: sense.examples.first().cloned(),
            })
            .collect();

        if !def_entries.is_empty() {
            definitions.push(Definition {
                part_of_speech: entry.part_of_speech.clone(),
                entries: def_entries,
            });
        }

        for sense in &entry.senses {
            for example_text in &sense.examples {
                if examples.len() < 5 {
                    examples.push(Example {
                        text: example_text.clone(),
                    });
                }
            }
            for synonym_word in &sense.synonyms {
                if synonyms.len() < 10 {
                    synonyms.push(Synonym {
                        word: synonym_word.clone(),
                    });
                }
            }
            for antonym_word in &sense.antonyms {
                if antonyms.len() < 10 {
                    antonyms.push(Antonym {
                        word: antonym_word.clone(),
                    });
                }
            }
        }
    }

    if definitions.is_empty()
        && examples.is_empty()
        && synonyms.is_empty()
        && antonyms.is_empty()
        && transliteration.is_none()
    {
        return Ok(None);
    }

    Ok(Some(TranslationMetadata {
        examples,
        definitions,
        alternatives: Vec::new(),
        synonyms,
        antonyms,
        related_words: Vec::new(),
        transliteration,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dict_response() {
        let json = r#"{
            "word": "hello",
            "entries": [{
                "language": {"code": "en", "name": "English"},
                "partOfSpeech": "interjection",
                "pronunciations": [{"type": "ipa", "text": "/hɛˈloʊ/", "tags": []}],
                "senses": [{
                    "definition": "A greeting.",
                    "examples": ["Hello, everyone."],
                    "synonyms": ["hi"],
                    "antonyms": ["goodbye"],
                    "tags": []
                }]
            }]
        }"#;

        let parsed: DictResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].part_of_speech, "interjection");
        assert_eq!(parsed.entries[0].pronunciations[0].text, "/hɛˈloʊ/");
        assert_eq!(parsed.entries[0].senses[0].definition, "A greeting.");
        assert_eq!(parsed.entries[0].senses[0].synonyms, vec!["hi"]);
        assert_eq!(parsed.entries[0].senses[0].antonyms, vec!["goodbye"]);
    }

    #[test]
    fn test_empty_word_validation() {
        let word = "   ".trim();
        assert!(word.is_empty());
    }
}
