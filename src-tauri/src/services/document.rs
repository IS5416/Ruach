use crate::error::AppError;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentMeta {
    pub rel_path: String,
    pub title: String,
    pub mtime: i64,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocOpenResult {
    pub content: String,
    pub meta: DocumentMeta,
}

pub struct DocumentService;

impl DocumentService {
    /// Reject absolute paths and `..` traversal — every rel_path entering
    /// the service must pass this gate.
    pub fn validate_rel_path(rel_path: &str) -> Result<(), AppError> {
        let p = Path::new(rel_path);
        if p.is_absolute()
            || rel_path.contains("..")
            || rel_path.contains('\\')
            || rel_path.contains("//")
        {
            return Err(AppError::InvalidPath(rel_path.to_string()));
        }
        if rel_path.is_empty() || rel_path.starts_with('/') {
            return Err(AppError::InvalidPath(rel_path.to_string()));
        }
        Ok(())
    }

    /// First `# ` heading line, or the file stem if none.
    pub fn title_from(content: &str, fallback_stem: &str) -> String {
        content
            .lines()
            .find_map(|l| l.strip_prefix("# ").map(str::trim).filter(|t| !t.is_empty()))
            .unwrap_or(fallback_stem)
            .to_string()
    }

    /// Read a document from the vault. Indexing is lazy (IndexService),
    /// done on open by the caller.
    pub fn open(vault: &Path, rel_path: &str) -> Result<DocOpenResult, AppError> {
        Self::validate_rel_path(rel_path)?;
        let abs = vault.join(rel_path);
        let content = fs::read_to_string(&abs).map_err(|_| AppError::NotFound(rel_path.to_string()))?;
        let meta = fs::metadata(&abs).map_err(|_| AppError::NotFound(rel_path.to_string()))?;
        let stem = Path::new(rel_path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let title = Self::title_from(&content, &stem);
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(DocOpenResult {
            content,
            meta: DocumentMeta {
                rel_path: rel_path.to_string(),
                title,
                mtime,
                size: meta.len(),
            },
        })
    }

    /// Save with mtime conflict detection and write-temp-then-rename.
    /// Returns the new file mtime so callers can update their baseline
    /// (avoids false conflicts on the next autosave). On success the
    /// session buffer row for this doc is removed — its content now
    /// lives on disk.
    pub fn save(
        conn: &rusqlite::Connection,
        vault: &Path,
        rel_path: &str,
        content: &str,
        expected_mtime: Option<i64>,
    ) -> Result<i64, AppError> {
        Self::validate_rel_path(rel_path)?;
        let abs = vault.join(rel_path);

        match fs::metadata(&abs) {
            Ok(meta) => {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                if let Some(expected) = expected_mtime {
                    if mtime != expected {
                        return Err(AppError::FileChanged);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {} // new file
            Err(e) => return Err(e.into()),
        }

        let dir = abs.parent().ok_or_else(|| AppError::InvalidPath(rel_path.to_string()))?;
        fs::create_dir_all(dir)?;
        let file_name = abs
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::InvalidPath(rel_path.to_string()))?;
        let tmp = dir.join(format!(".{file_name}.ruach-tmp"));
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &abs)?;

        conn.execute("DELETE FROM sessions WHERE doc_key = ?1", [rel_path])?;

        let mtime = fs::metadata(&abs)?
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(mtime)
    }

    /// Flush an editing session (dirty doc or untitled draft) to the
    /// recovery buffer. `doc_key` is a rel_path or `:untitled:<ts>`.
    pub fn session_flush(
        conn: &rusqlite::Connection,
        doc_key: &str,
        content: &str,
        cursor: Option<i64>,
    ) -> Result<(), AppError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO sessions (doc_key, content, cursor, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(doc_key) DO UPDATE SET
               content = excluded.content,
               cursor = excluded.cursor,
               updated_at = excluded.updated_at",
            rusqlite::params![doc_key, content, cursor, now],
        )?;
        Ok(())
    }

    pub fn session_list(conn: &rusqlite::Connection) -> Result<Vec<SessionInfo>, AppError> {
        let mut stmt = conn.prepare(
            "SELECT doc_key, updated_at, substr(content, 1, 80) FROM sessions
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionInfo {
                doc_key: r.get(0)?,
                updated_at: r.get(1)?,
                preview: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn session_restore(
        conn: &rusqlite::Connection,
        doc_key: &str,
    ) -> Result<SessionDraft, AppError> {
        conn.query_row(
            "SELECT content, cursor FROM sessions WHERE doc_key = ?1",
            [doc_key],
            |r| {
                Ok(SessionDraft {
                    content: r.get(0)?,
                    cursor: r.get(1)?,
                })
            },
        )
        .map_err(|_| AppError::NotFound(doc_key.to_string()))
    }

    pub fn session_discard(conn: &rusqlite::Connection, doc_key: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM sessions WHERE doc_key = ?1", [doc_key])?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub doc_key: String,
    pub updated_at: i64,
    pub preview: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionDraft {
    pub content: String,
    pub cursor: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_traversal_and_absolute() {
        assert!(DocumentService::validate_rel_path("../escape.md").is_err());
        assert!(DocumentService::validate_rel_path("/etc/passwd").is_err());
        assert!(DocumentService::validate_rel_path("C:/windows").is_err());
        assert!(DocumentService::validate_rel_path("").is_err());
        assert!(DocumentService::validate_rel_path("notes/a.md").is_ok());
    }

    #[test]
    fn title_from_first_heading() {
        assert_eq!(
            DocumentService::title_from("# 风的形状\n\n正文", "a"),
            "风的形状"
        );
        assert_eq!(DocumentService::title_from("无标题", "a"), "a");
    }

    #[test]
    fn open_reads_file() {
        let dir = std::env::temp_dir().join(format!(
            "ruach-doc-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.md"), "# Hello\n").unwrap();

        let res = DocumentService::open(&dir, "a.md").expect("open");
        assert_eq!(res.content, "# Hello\n");
        assert_eq!(res.meta.title, "Hello");

        assert!(DocumentService::open(&dir, "missing.md").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    fn temp_vault(name: &str) -> (std::path::PathBuf, rusqlite::Connection) {
        let dir = std::env::temp_dir().join(format!(
            "ruach-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("db_schema.sql")).unwrap();
        (dir, conn)
    }

    #[test]
    fn save_creates_and_updates_file() {
        let (dir, conn) = temp_vault("save-create");
        DocumentService::save(&conn, &dir, "notes/a.md", "# A\n", None).expect("save");
        assert_eq!(fs::read_to_string(dir.join("notes/a.md")).unwrap(), "# A\n");

        // Update with the mtime read back from disk.
        let meta = fs::metadata(dir.join("notes/a.md")).unwrap();
        let mtime = meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        DocumentService::save(&conn, &dir, "notes/a.md", "# A v2\n", Some(mtime)).expect("save v2");
        assert_eq!(fs::read_to_string(dir.join("notes/a.md")).unwrap(), "# A v2\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_rejects_stale_mtime() {
        let (dir, conn) = temp_vault("save-conflict");
        DocumentService::save(&conn, &dir, "a.md", "# A\n", None).expect("save");
        // Stale expected mtime (0) vs actual disk mtime.
        let err = DocumentService::save(&conn, &dir, "a.md", "# A v2\n", Some(0)).unwrap_err();
        assert!(matches!(err, AppError::FileChanged));
        // Disk unchanged.
        assert_eq!(fs::read_to_string(dir.join("a.md")).unwrap(), "# A\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_clears_session_after_success() {
        let (dir, conn) = temp_vault("save-session");
        DocumentService::session_flush(&conn, "a.md", "草稿内容", Some(3)).expect("flush");
        DocumentService::save(&conn, &dir, "a.md", "落盘", None).expect("save");
        assert!(DocumentService::session_list(&conn).unwrap().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn session_flush_restore_discard_roundtrip() {
        let (_dir, conn) = temp_vault("session");
        DocumentService::session_flush(&conn, ":untitled:123", "草稿", Some(2)).expect("flush");

        let list = DocumentService::session_list(&conn).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].doc_key, ":untitled:123");

        let draft = DocumentService::session_restore(&conn, ":untitled:123").expect("restore");
        assert_eq!(draft.content, "草稿");
        assert_eq!(draft.cursor, Some(2));

        DocumentService::session_discard(&conn, ":untitled:123").expect("discard");
        assert!(DocumentService::session_restore(&conn, ":untitled:123").is_err());
    }
}
