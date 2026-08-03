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
    /// Full implementation lands in P1 (autosave, session buffer).
    pub fn save(
        _vault: &Path,
        _rel_path: &str,
        _content: &str,
        _expected_mtime: Option<i64>,
    ) -> Result<(), AppError> {
        Err(AppError::NotImplemented("DocumentService::save"))
    }
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
}
