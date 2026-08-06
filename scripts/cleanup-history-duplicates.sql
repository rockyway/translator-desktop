-- Cleanup consecutive duplicate source_text entries from history
-- Run this with: sqlite3 <path-to-db>/translator.db < cleanup-history-duplicates.sql
--
-- Database location (Windows): %APPDATA%\com.translator.desktop\translator.db

-- Show duplicates before deletion (for verification)
SELECT 'Consecutive duplicates found:' as info;
SELECT h2.id, h2.source_text, h2.created_at
FROM history h1
INNER JOIN history h2 ON h2.id = (
    SELECT MIN(h3.id)
    FROM history h3
    WHERE h3.id > h1.id
)
WHERE h1.source_text = h2.source_text
ORDER BY h2.id;

-- Count duplicates
SELECT 'Total duplicates to delete: ' || COUNT(*) as info
FROM history h1
INNER JOIN history h2 ON h2.id = (
    SELECT MIN(h3.id)
    FROM history h3
    WHERE h3.id > h1.id
)
WHERE h1.source_text = h2.source_text;

-- Delete consecutive duplicates (keeps the first occurrence in each series)
DELETE FROM history
WHERE id IN (
    SELECT h2.id
    FROM history h1
    INNER JOIN history h2 ON h2.id = (
        SELECT MIN(h3.id)
        FROM history h3
        WHERE h3.id > h1.id
    )
    WHERE h1.source_text = h2.source_text
);

SELECT 'Cleanup complete. Rows deleted: ' || changes() as result;
