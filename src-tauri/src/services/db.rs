use crate::error::AppError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub const SCHEMA_VERSION: i64 = 2;

/// v2: snapshot history (interface reserved — SnapshotService P8+).
pub const SCHEMA_V2_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS snapshots (
  rel_path    TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  snapshot_at INTEGER NOT NULL,
  content     TEXT NOT NULL,
  PRIMARY KEY (rel_path, snapshot_at)
);
"#;

/// Schema for the vault sidecar database. All derived data keyed by
/// vault-relative path so the vault can move/sync externally.
pub const SCHEMA_SQL: &str = include_str!("db_schema.sql");

/// Owns the single SQLite connection for a vault. `Connection` is `Send`
/// but not `Sync`, so it lives behind a `Mutex`; all access happens on the
/// Tauri thread pool (commands are sync).
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (creating if needed) a vault sidecar database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// In-memory database for tests.
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // Tolerate poisoning: the connection state itself is harmless, and
        // a panicking command must not brick every later one.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn init_schema(conn: &Connection) -> Result<(), AppError> {
    // ON DELETE CASCADE between derived tables and files requires this.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        // A vault created by a newer app build — the frontend can map this
        // to a friendly "vault is from a newer version" hint.
        return Err(AppError::SchemaVersion {
            found: version,
            max: SCHEMA_VERSION,
        });
    }
    if version == 0 {
        conn.execute_batch(SCHEMA_SQL)?;
    }
    if version < 2 {
        conn.execute_batch(SCHEMA_V2_SQL)?;
    }
    conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_db_has_all_tables() {
        let db = Database::open_in_memory().expect("open");
        let conn = db.conn();
        for table in [
            "files", "tags", "links", "attachments", "recent", "sessions", "docs_fts",
        ] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |r| r.get(0),
                )
                .expect("query");
            assert_eq!(count, 1, "table {table} missing");
        }
    }

    #[test]
    fn schema_version_set() {
        let db = Database::open_in_memory().expect("open");
        let version: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .expect("query");
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn fts_trigram_tokenizer_available() {
        // Fails at open time if bundled SQLite lacks trigram.
        let db = Database::open_in_memory().expect("open");
        let ok: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'docs_fts'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(ok, 1);
    }
}
