-- Migration 001: Create history table for storing translation history
-- This table stores all translations performed by the user for later reference

CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    detected_language TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for faster search queries on source and translated text
CREATE INDEX IF NOT EXISTS idx_history_source_text ON history(source_text);
CREATE INDEX IF NOT EXISTS idx_history_translated_text ON history(translated_text);

-- Index for faster ordering by creation date
CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC);
