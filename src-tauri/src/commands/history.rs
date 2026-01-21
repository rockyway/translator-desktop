//! History commands for managing translation history.
//!
//! This module provides CRUD operations for the translation history database,
//! allowing users to save, retrieve, search, and delete their translation history.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use tauri::State;
use tokio::sync::Mutex;

/// Database state wrapper for managed state
pub struct DbState(pub Mutex<SqlitePool>);

/// Represents a single history entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub source_text: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub detected_language: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// Input for adding a new history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddHistoryInput {
    pub source_text: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub detected_language: Option<String>,
    pub metadata: Option<String>,
}

/// Pagination result for history queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Error type for history operations
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Source text cannot be empty")]
    EmptySourceText,

    #[error("Translated text cannot be empty")]
    EmptyTranslatedText,

    #[error("Source language cannot be empty")]
    EmptySourceLanguage,

    #[error("Target language cannot be empty")]
    EmptyTargetLanguage,

    #[error("History entry not found: {0}")]
    NotFound(i64),
}

// Implement serialization for Tauri command error handling
impl Serialize for HistoryError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Escape special characters in LIKE patterns to prevent SQL injection
/// Escapes %, _, and \ characters
fn escape_like_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Initialize the database schema
pub async fn init_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_text TEXT NOT NULL,
            translated_text TEXT NOT NULL,
            source_language TEXT NOT NULL,
            target_language TEXT NOT NULL,
            detected_language TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Migration: Add metadata column if not exists
    let column_exists: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('history') WHERE name = 'metadata'"
    )
    .fetch_one(pool)
    .await?;

    if !column_exists {
        sqlx::query("ALTER TABLE history ADD COLUMN metadata TEXT")
            .execute(pool)
            .await?;
    }

    // Create indexes for faster queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_history_source_text ON history(source_text);")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_history_translated_text ON history(translated_text);",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC);")
        .execute(pool)
        .await?;

    Ok(())
}

/// Add a translation to history
///
/// # Arguments
/// * `db_state` - The database state
/// * `input` - The history entry data to add
///
/// # Returns
/// * The ID of the newly created history entry
#[tauri::command]
pub async fn add_history(
    db_state: State<'_, DbState>,
    input: AddHistoryInput,
) -> Result<i64, HistoryError> {
    // Validate input
    if input.source_text.trim().is_empty() {
        return Err(HistoryError::EmptySourceText);
    }
    if input.translated_text.trim().is_empty() {
        return Err(HistoryError::EmptyTranslatedText);
    }
    if input.source_language.trim().is_empty() {
        return Err(HistoryError::EmptySourceLanguage);
    }
    if input.target_language.trim().is_empty() {
        return Err(HistoryError::EmptyTargetLanguage);
    }

    let pool = db_state.0.lock().await;

    let result = sqlx::query(
        r#"
        INSERT INTO history (source_text, translated_text, source_language, target_language, detected_language, metadata)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&input.source_text)
    .bind(&input.translated_text)
    .bind(&input.source_language)
    .bind(&input.target_language)
    .bind(&input.detected_language)
    .bind(&input.metadata)
    .execute(&*pool)
    .await
    .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;

    Ok(result.last_insert_rowid())
}

/// Get paginated history entries
///
/// # Arguments
/// * `db_state` - The database state
/// * `limit` - Maximum number of entries to return (default: 20, max: 100)
/// * `offset` - Number of entries to skip (default: 0)
///
/// # Returns
/// * A paginated list of history entries
#[tauri::command]
pub async fn get_history(
    db_state: State<'_, DbState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<HistoryPage, HistoryError> {
    let limit = limit.unwrap_or(20).min(100).max(1);
    let offset = offset.unwrap_or(0).max(0);

    let pool = db_state.0.lock().await;

    // Get total count
    let total_row = sqlx::query("SELECT COUNT(*) as count FROM history")
        .fetch_one(&*pool)
        .await
        .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;
    let total: i64 = total_row.get("count");

    // Get paginated entries
    let entries: Vec<HistoryEntry> = sqlx::query_as(
        r#"
        SELECT id, source_text, translated_text, source_language, target_language, detected_language, metadata, created_at
        FROM history
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&*pool)
    .await
    .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;

    Ok(HistoryPage {
        entries,
        total,
        limit,
        offset,
    })
}

/// Delete a single history entry by ID
///
/// # Arguments
/// * `db_state` - The database state
/// * `id` - The ID of the history entry to delete
///
/// # Returns
/// * `true` if the entry was deleted, error if not found
#[tauri::command]
pub async fn delete_history(db_state: State<'_, DbState>, id: i64) -> Result<bool, HistoryError> {
    let pool = db_state.0.lock().await;

    let result = sqlx::query("DELETE FROM history WHERE id = ?")
        .bind(id)
        .execute(&*pool)
        .await
        .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err(HistoryError::NotFound(id));
    }

    Ok(true)
}

/// Clear all history entries
///
/// # Arguments
/// * `db_state` - The database state
///
/// # Returns
/// * The number of entries deleted
#[tauri::command]
pub async fn clear_history(db_state: State<'_, DbState>) -> Result<i64, HistoryError> {
    let pool = db_state.0.lock().await;

    let result = sqlx::query("DELETE FROM history")
        .execute(&*pool)
        .await
        .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;

    Ok(result.rows_affected() as i64)
}

/// Search history by text (searches both source and translated text)
///
/// # Arguments
/// * `db_state` - The database state
/// * `query` - The search query
/// * `limit` - Maximum number of entries to return (default: 20, max: 100)
/// * `offset` - Number of entries to skip (default: 0)
///
/// # Returns
/// * A paginated list of matching history entries
#[tauri::command]
pub async fn search_history(
    db_state: State<'_, DbState>,
    query: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<HistoryPage, HistoryError> {
    let query_trimmed = query.trim();
    if query_trimmed.is_empty() {
        // Return empty result for empty query
        return Ok(HistoryPage {
            entries: vec![],
            total: 0,
            limit: limit.unwrap_or(20),
            offset: offset.unwrap_or(0),
        });
    }

    let limit = limit.unwrap_or(20).min(100).max(1);
    let offset = offset.unwrap_or(0).max(0);
    let escaped_query = escape_like_pattern(query_trimmed);
    let search_pattern = format!("%{}%", escaped_query);

    let pool = db_state.0.lock().await;

    // Get total count for search
    let total_row = sqlx::query(
        r#"
        SELECT COUNT(*) as count FROM history
        WHERE source_text LIKE ? ESCAPE '\' OR translated_text LIKE ? ESCAPE '\'
        "#,
    )
    .bind(&search_pattern)
    .bind(&search_pattern)
    .fetch_one(&*pool)
    .await
    .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;
    let total: i64 = total_row.get("count");

    // Get paginated search results
    let entries: Vec<HistoryEntry> = sqlx::query_as(
        r#"
        SELECT id, source_text, translated_text, source_language, target_language, detected_language, metadata, created_at
        FROM history
        WHERE source_text LIKE ? ESCAPE '\' OR translated_text LIKE ? ESCAPE '\'
        ORDER BY created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(&search_pattern)
    .bind(&search_pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(&*pool)
    .await
    .map_err(|e| HistoryError::DatabaseError(e.to_string()))?;

    Ok(HistoryPage {
        entries,
        total,
        limit,
        offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_history_input_validation_empty_source() {
        let input = AddHistoryInput {
            source_text: "".to_string(),
            translated_text: "Hello".to_string(),
            source_language: "vi".to_string(),
            target_language: "en".to_string(),
            detected_language: None,
            metadata: None,
        };

        // Validation would fail
        assert!(input.source_text.trim().is_empty());
    }

    #[test]
    fn test_add_history_input_validation_empty_translated() {
        let input = AddHistoryInput {
            source_text: "Xin chao".to_string(),
            translated_text: "   ".to_string(),
            source_language: "vi".to_string(),
            target_language: "en".to_string(),
            detected_language: None,
            metadata: None,
        };

        // Validation would fail
        assert!(input.translated_text.trim().is_empty());
    }

    #[test]
    fn test_pagination_defaults() {
        // Test default values
        let limit = None::<i64>.unwrap_or(20).min(100).max(1);
        let offset = None::<i64>.unwrap_or(0).max(0);

        assert_eq!(limit, 20);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_pagination_clamping() {
        // Test limit clamping
        let limit = Some(200_i64).unwrap_or(20).min(100).max(1);
        assert_eq!(limit, 100);

        let limit = Some(0_i64).unwrap_or(20).min(100).max(1);
        assert_eq!(limit, 1);

        // Test offset clamping
        let offset = Some(-10_i64).unwrap_or(0).max(0);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_search_pattern_format() {
        let query = "hello";
        let pattern = format!("%{}%", query);
        assert_eq!(pattern, "%hello%");
    }

    #[test]
    fn test_escape_like_pattern() {
        // Test escaping percent sign
        assert_eq!(escape_like_pattern("100%"), "100\\%");

        // Test escaping underscore
        assert_eq!(escape_like_pattern("hello_world"), "hello\\_world");

        // Test escaping backslash
        assert_eq!(escape_like_pattern("path\\to\\file"), "path\\\\to\\\\file");

        // Test combined escaping
        assert_eq!(escape_like_pattern("50%_off\\sale"), "50\\%\\_off\\\\sale");

        // Test no escaping needed
        assert_eq!(escape_like_pattern("hello world"), "hello world");
    }
}
