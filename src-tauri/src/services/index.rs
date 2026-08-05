use crate::error::AppError;
use crate::services::document::DocumentService;
use rusqlite::Connection;
use std::path::Path;

/// Extracted knowledge markers: `#tag` on its own line and `[[target]]` links.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DocumentMarkers {
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

/// Derives tags/links/FTS rows for files. Lazy indexing: index a file on
/// open, re-index changed files during vault scan, full `reindex` fallback.
pub struct IndexService;

impl IndexService {
    pub fn extract_markers(content: &str) -> DocumentMarkers {
        let mut markers = DocumentMarkers::default();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(_tag) = trimmed.strip_prefix("# ") {
                // Heading line, not a tag. Skip.
                continue;
            }
            // Whole-line tags: `#tag` / `#tag/tag2` / comma-separated `#a, #b`
            if trimmed.starts_with('#') && !trimmed.starts_with("##") {
                for part in trimmed.trim_start_matches('#').split(',') {
                    let tag = part.trim().trim_matches('#').trim();
                    if !tag.is_empty() && !tag.contains(char::is_whitespace) {
                        markers.tags.push(tag.to_string());
                    }
                }
            }
        }
        for start in content.match_indices("[[").map(|(i, _)| i) {
            let rest = &content[start + 2..];
            let end = rest.find("]]");
            if let Some(end) = end {
                let target = rest[..end].trim();
                if !target.is_empty() {
                    markers.links.push(target.to_string());
                }
            }
        }
        markers
    }

    /// Index one file from disk (read + write sidecar rows).
    pub fn index_file(conn: &Connection, vault: &Path, rel_path: &str) -> Result<(), AppError> {
        let content = std::fs::read_to_string(vault.join(rel_path))
            .map_err(|_| AppError::NotFound(rel_path.to_string()))?;
        Self::index_file_content(conn, rel_path, &content)
    }

    /// Write derived rows for one document: tags, links, FTS. Replaces any
    /// existing rows (idempotent). Title is used for the FTS title column.
    pub fn index_file_content(
        conn: &Connection,
        rel_path: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let markers = Self::extract_markers(content);
        let stem = Path::new(rel_path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let title = DocumentService::title_from(content, &stem);

        conn.execute("DELETE FROM tags WHERE rel_path = ?1", [rel_path])?;
        conn.execute("DELETE FROM links WHERE src_path = ?1", [rel_path])?;
        conn.execute("DELETE FROM docs_fts WHERE rel_path = ?1", [rel_path])?;

        for tag in &markers.tags {
            conn.execute(
                "INSERT OR IGNORE INTO tags (rel_path, tag) VALUES (?1, ?2)",
                rusqlite::params![rel_path, tag],
            )?;
        }
        for link in &markers.links {
            conn.execute(
                "INSERT OR IGNORE INTO links (src_path, dst_path) VALUES (?1, ?2)",
                rusqlite::params![rel_path, link],
            )?;
        }
        conn.execute(
            "INSERT INTO docs_fts (rel_path, title, body) VALUES (?1, ?2, ?3)",
            rusqlite::params![rel_path, title, content],
        )?;
        Ok(())
    }

    /// Full rebuild: wipe derived rows and re-index every `.md` file.
    /// Returns the number of files indexed.
    pub fn reindex(conn: &Connection, vault: &Path) -> Result<u32, AppError> {
        conn.execute("DELETE FROM tags", [])?;
        conn.execute("DELETE FROM links", [])?;
        conn.execute("DELETE FROM docs_fts", [])?;
        conn.execute("DELETE FROM files", [])?;

        let mut count = 0u32;
        Self::reindex_dir(conn, vault, vault, &mut count)?;
        Ok(count)
    }

    fn reindex_dir(
        conn: &Connection,
        vault: &Path,
        dir: &Path,
        count: &mut u32,
    ) -> Result<(), AppError> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type()?.is_dir() {
                Self::reindex_dir(conn, vault, &entry.path(), count)?;
            } else if name.ends_with(".md") {
                let rel = entry
                    .path()
                    .strip_prefix(vault)
                    .map_err(|_| AppError::Vault("path outside vault".to_string()))?
                    .to_string_lossy()
                    .into_owned()
                    .replace('\\', "/");
                let content = std::fs::read_to_string(entry.path())
                    .map_err(|_| AppError::NotFound(rel.clone()))?;
                let stem = entry
                    .path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let title = DocumentService::title_from(&content, &stem);
                let mtime = entry
                    .metadata()?
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                conn.execute(
                    "INSERT INTO files (rel_path, title, mtime, size, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    rusqlite::params![
                        rel,
                        title,
                        mtime,
                        entry.metadata()?.len() as i64,
                        now
                    ],
                )?;
                Self::index_file_content(conn, &rel, &content)?;
                *count += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db::SCHEMA_SQL;

    fn mem_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn
    }

    fn seed_file(conn: &Connection, rel_path: &str) {
        conn.execute(
            "INSERT INTO files (rel_path, title, mtime, size, created_at, updated_at)
             VALUES (?1, '', 0, 0, 0, 0)",
            [rel_path],
        )
        .unwrap();
    }

    #[test]
    fn extracts_tags_and_links() {
        let content = "# 标题\n\n正文\n\n#tag, #daily/2026-08-03\n\n参考 [[另一个笔记]] 和 [[未创建]]\n";
        let m = IndexService::extract_markers(content);
        assert!(m.tags.contains(&"tag".to_string()));
        assert!(m.tags.contains(&"daily/2026-08-03".to_string()));
        assert_eq!(m.links, vec!["另一个笔记", "未创建"]);
    }

    #[test]
    fn heading_is_not_tag() {
        let m = IndexService::extract_markers("# 标题\n");
        assert!(m.tags.is_empty());
    }

    #[test]
    fn index_content_writes_derived_rows() {
        let conn = mem_db();
        seed_file(&conn, "a.md");
        IndexService::index_file_content(&conn, "a.md", "# 风的形状\n\n#tag\n\n[[b]]\n").unwrap();

        let tags: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags, 1);
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 1);
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM docs_fts WHERE docs_fts MATCH ?1",
                ["\"风的形状\""],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);

        // Re-index replaces, not duplicates.
        IndexService::index_file_content(&conn, "a.md", "无标记\n").unwrap();
        let tags2: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags2, 0);
    }

    #[test]
    fn reindex_rebuilds_all() {
        let dir = std::env::temp_dir().join(format!(
            "ruach-reindex-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), "# A\n#tag\n").unwrap();
        std::fs::write(dir.join("b.md"), "# B\n").unwrap();

        let conn = mem_db();
        let count = IndexService::reindex(&conn, &dir).unwrap();
        assert_eq!(count, 2);

        let files: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(files, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
