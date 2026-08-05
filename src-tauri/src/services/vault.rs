use crate::error::AppError;
use crate::services::document::DocumentService;
use crate::services::index::IndexService;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct TreeNode {
    pub rel_path: String,
    pub name: String,
    pub is_dir: bool,
}

/// Vault = a directory of markdown documents plus its `.ruach/` sidecar.
pub struct VaultService;

impl VaultService {
    /// Validate a vault root: must exist and be a directory.
    pub fn validate(path: &Path) -> Result<(), AppError> {
        let meta = std::fs::metadata(path)
            .map_err(|_| AppError::Vault(format!("vault not found: {}", path.display())))?;
        if !meta.is_dir() {
            return Err(AppError::Vault(format!("not a directory: {}", path.display())));
        }
        Ok(())
    }

    /// Recursive scan: walk `.md` files (skipping dot-dirs like `.ruach/`),
    /// upsert `files` rows (incremental by mtime+size), re-index content
    /// of changed files, and prune rows for files deleted from disk.
    /// Returns the flat tree for the frontend.
    pub fn scan(conn: &Connection, vault: &Path) -> Result<Vec<TreeNode>, AppError> {
        let now = unix_now();
        let mut nodes = Vec::new();
        let mut seen = std::collections::HashSet::new();
        scan_dir(conn, vault, vault, "", &mut nodes, now, &mut seen)?;
        // Reconcile: drop files rows (and cascaded tags/links/FTS) for
        // files that no longer exist on disk.
        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("SELECT rel_path FROM files")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for rel in rows.collect::<Result<Vec<_>, _>>()? {
                if !seen.contains(&rel) {
                    tx.execute("DELETE FROM files WHERE rel_path = ?1", [&rel])?;
                }
            }
        }
        tx.commit()?;
        Ok(nodes)
    }

    /// Watcher hook — reserved for P2+; not wired in skeleton.
    pub fn sidecar_path(vault: &Path) -> PathBuf {
        vault.join(".ruach").join("index.db")
    }
}

fn scan_dir(
    conn: &Connection,
    vault: &Path,
    dir: &Path,
    prefix: &str,
    nodes: &mut Vec<TreeNode>,
    now: i64,
    seen: &mut std::collections::HashSet<String>,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue; // .ruach, .obsidian, hidden files — not part of the tree
        }
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let ftype = entry.file_type()?;
        if ftype.is_dir() {
            nodes.push(TreeNode {
                rel_path: rel.clone(),
                name,
                is_dir: true,
            });
            scan_dir(conn, vault, &entry.path(), &rel, nodes, now, seen)?;
        } else if name.ends_with(".md") {
            let meta = entry.metadata()?;
            let mtime = mtime_secs(&meta);
            let size = meta.len();
            upsert_file(conn, vault, &rel, &name, mtime, size, now)?;
            seen.insert(rel.clone());
            nodes.push(TreeNode {
                rel_path: rel,
                name,
                is_dir: false,
            });
        }
    }
    Ok(())
}

fn upsert_file(
    conn: &Connection,
    vault: &Path,
    rel_path: &str,
    name: &str,
    mtime: i64,
    size: u64,
    now: i64,
) -> Result<(), AppError> {
    let unchanged: Option<i64> = conn
        .query_row(
            "SELECT mtime FROM files WHERE rel_path = ?1 AND mtime = ?2 AND size = ?3",
            rusqlite::params![rel_path, mtime, size as i64],
            |r| r.get(0),
        )
        .ok();

    if unchanged.is_some() {
        return Ok(()); // incremental scan: nothing changed
    }

    let content = std::fs::read_to_string(vault.join(rel_path))
        .map_err(|_| AppError::NotFound(rel_path.to_string()))?;
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let title = DocumentService::title_from(&content, &stem);

    conn.execute(
        "INSERT INTO files (rel_path, title, mtime, size, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(rel_path) DO UPDATE SET
           title = excluded.title, mtime = excluded.mtime,
           size = excluded.size, updated_at = excluded.updated_at",
        rusqlite::params![rel_path, title, mtime, size as i64, now],
    )?;

    IndexService::index_file_content(conn, rel_path, &content)
}

fn mtime_secs(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db::SCHEMA_SQL;

    fn temp_vault(name: &str) -> (PathBuf, Connection) {
        let dir = std::env::temp_dir().join(format!(
            "ruach-vault-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("notes/sub")).unwrap();
        std::fs::create_dir_all(dir.join(".ruach")).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        (dir, conn)
    }

    #[test]
    fn scan_lists_md_files_and_dirs() {
        let (dir, conn) = temp_vault("scan");
        std::fs::write(dir.join("notes/a.md"), "# A\n").unwrap();
        std::fs::write(dir.join("notes/sub/b.md"), "正文\n").unwrap();
        // A non-md file must not appear as a node.
        std::fs::write(dir.join("notes/img.png"), "x").unwrap();

        let nodes = VaultService::scan(&conn, &dir).unwrap();
        let paths: Vec<&str> = nodes.iter().map(|n| n.rel_path.as_str()).collect();
        assert!(paths.contains(&"notes/a.md".to_string().as_str()));
        assert!(paths.contains(&"notes/sub/b.md".to_string().as_str()));
        assert!(paths.contains(&"notes".to_string().as_str()));
        assert!(!paths.contains(&"notes/img.png".to_string().as_str()));
        assert!(!nodes.iter().any(|n| n.rel_path.starts_with(".ruach")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_indexes_changed_files() {
        let (dir, conn) = temp_vault("index");
        std::fs::write(dir.join("a.md"), "# 风的形状\n\n#tag\n\n[[链接目标]]\n").unwrap();
        VaultService::scan(&conn, &dir).unwrap();

        let tags: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags, 1);
        let links: i64 = conn
            .query_row("SELECT COUNT(*) FROM links", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 1);
        // FTS row present and searchable (trigram needs >= 3 chars).
        let hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM docs_fts WHERE docs_fts MATCH ?1",
                ["\"风的形状\""],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hits, 1);

        // Second scan is a no-op: tags stay 1 (no dupes).
        VaultService::scan(&conn, &dir).unwrap();
        let tags2: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags2, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_prunes_deleted_files() {
        let (dir, conn) = temp_vault("prune");
        std::fs::write(dir.join("a.md"), "# A\n").unwrap();
        VaultService::scan(&conn, &dir).unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);

        // File removed from disk → next scan drops its rows.
        std::fs::remove_file(dir.join("a.md")).unwrap();
        VaultService::scan(&conn, &dir).unwrap();
        let n2: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n2, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
