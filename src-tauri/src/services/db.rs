use crate::error::AppError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub const SCHEMA_VERSION: i64 = 1;

/// Schema for the vault sidecar database. All derived data keyed by
/// vault-relative path so the vault can move/sync externally.
const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS files (
  rel_path   TEXT PRIMARY KEY,
  title      TEXT NOT NULL,
  mtime      INTEGER NOT NULL,
  size       INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
  rel_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  tag      TEXT NOT NULL,
  PRIMARY KEY (rel_path, tag)
);

CREATE TABLE IF NOT EXISTS links (
  src_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  dst_path TEXT NOT NULL,
  label    TEXT,
  PRIMARY KEY (src_path, dst_path)
);

CREATE TABLE IF NOT EXISTS attachments (
  rel_path  TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  name      TEXT NOT NULL,
  orig_name TEXT,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (rel_path, name)
);

CREATE TABLE IF NOT EXISTS recent (
  rel_path  TEXT PRIMARY KEY REFERENCES files(rel_path) ON DELETE CASCADE,
  opened_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  doc_key    TEXT PRIMARY KEY,
  content    TEXT NOT NULL,
  cursor     INTEGER,
  updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
  rel_path UNINDEXED, title, body, tokenize='trigram'
);
"#;

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
        self.conn.lock().expect("db mutex poisoned")
    }
}

fn init_schema(conn: &Connection) -> Result<(), AppError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(AppError::Db(rusqlite::Error::InvalidColumnName(
            format!("database version {version} newer than app version {SCHEMA_VERSION}"),
        )));
    }
    if version == 0 {
        conn.execute_batch(SCHEMA_SQL)?;
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION}"))?;
    }
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
