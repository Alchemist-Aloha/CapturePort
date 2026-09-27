use rusqlite::{Connection, DatabaseName};
use std::path::Path;

use crate::CatalogError;

pub(crate) const CURRENT_SCHEMA_VERSION: i64 = 2;

pub(crate) fn migrate(
    connection: &Connection,
    file_path: Option<&Path>,
) -> Result<(), CatalogError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
    )?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(CatalogError::Request(format!(
            "database schema {version} is newer than supported schema {CURRENT_SCHEMA_VERSION}"
        )));
    }
    if version < 1 {
        let tx = connection.unchecked_transaction()?;
        tx.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS sources (
                id INTEGER PRIMARY KEY,
                stable_id TEXT,
                source_type TEXT NOT NULL,
                manufacturer TEXT,
                model TEXT,
                serial TEXT,
                alias TEXT,
                volume_uuid TEXT,
                first_seen_at TEXT NOT NULL,
                last_seen_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS media (
                id INTEGER PRIMARY KEY,
                source_id INTEGER NOT NULL REFERENCES sources(id),
                source_path TEXT NOT NULL,
                source_filename TEXT NOT NULL,
                source_size INTEGER NOT NULL,
                capture_time TEXT,
                media_type TEXT NOT NULL,
                quick_fingerprint TEXT,
                content_hash TEXT,
                first_seen_at TEXT NOT NULL,
                UNIQUE(source_id, source_path, source_size, capture_time)
            );
            CREATE TABLE IF NOT EXISTS import_sessions (
                id INTEGER PRIMARY KEY,
                source_id INTEGER NOT NULL REFERENCES sources(id),
                preset_id INTEGER,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                error TEXT
            );
            CREATE TABLE IF NOT EXISTS imports (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL REFERENCES import_sessions(id),
                media_id INTEGER NOT NULL REFERENCES media(id),
                destination_path TEXT NOT NULL,
                destination_size INTEGER,
                copy_completed_at TEXT,
                verification_method TEXT,
                verified_at TEXT,
                status TEXT NOT NULL,
                error TEXT,
                UNIQUE(session_id, media_id)
            );
            CREATE TABLE IF NOT EXISTS presets (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                configuration_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );
            INSERT INTO schema_version(version)
                SELECT 1 WHERE NOT EXISTS (SELECT 1 FROM schema_version);
            CREATE INDEX IF NOT EXISTS idx_media_source ON media(source_id);
            CREATE INDEX IF NOT EXISTS idx_media_path ON media(source_id, source_path);
            CREATE INDEX IF NOT EXISTS idx_media_filename_size ON media(source_id, source_filename, source_size);
            CREATE INDEX IF NOT EXISTS idx_media_capture_time ON media(capture_time);
            CREATE INDEX IF NOT EXISTS idx_media_quick_fingerprint ON media(quick_fingerprint);
            CREATE INDEX IF NOT EXISTS idx_media_content_hash ON media(content_hash);
            CREATE INDEX IF NOT EXISTS idx_imports_session ON imports(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_source_started ON import_sessions(source_id, started_at DESC);
            PRAGMA user_version = 1;
        "#)?;
        tx.commit()?;
    }
    if version < 2 {
        // Preserve a recoverable copy before rebuilding the table. SQLite's
        // online backup includes WAL state and is safer than copying files.
        if version >= 1 {
            if let Some(path) = file_path {
                let backup_path = path.with_extension("sqlite.pre-v2.bak");
                if !backup_path.exists() {
                    connection.backup(DatabaseName::Main, &backup_path, None)?;
                }
            }
        }
        let tx = connection.unchecked_transaction()?;
        tx.execute_batch(r#"
            ALTER TABLE imports RENAME TO imports_v1;
            CREATE TABLE imports (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL REFERENCES import_sessions(id),
                media_id INTEGER NOT NULL REFERENCES media(id),
                destination_path TEXT NOT NULL,
                destination_size INTEGER,
                copy_completed_at TEXT,
                verification_method TEXT,
                verified_at TEXT,
                status TEXT NOT NULL,
                error TEXT,
                UNIQUE(session_id, media_id, destination_path)
            );
            INSERT INTO imports(id,session_id,media_id,destination_path,destination_size,copy_completed_at,verification_method,verified_at,status,error)
                SELECT id,session_id,media_id,destination_path,destination_size,copy_completed_at,verification_method,verified_at,status,error
                FROM imports_v1;
            DROP TABLE imports_v1;
            CREATE INDEX IF NOT EXISTS idx_imports_session ON imports(session_id);
            UPDATE schema_version SET version = 2;
            PRAGMA user_version = 2;
        "#)?;
        tx.commit()?;
    }
    Ok(())
}
