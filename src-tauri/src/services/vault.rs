use crate::error::AppError;
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

    /// Full recursive scan of the vault. Files table upsert + FTS refresh.
    /// Implemented in P2 (incremental re-scan by mtime).
    pub fn scan(_vault: &Path) -> Result<Vec<TreeNode>, AppError> {
        Err(AppError::NotImplemented("VaultService::scan"))
    }

    /// Watcher hook — reserved for P2+; not wired in skeleton.
    pub fn sidecar_path(vault: &Path) -> PathBuf {
        vault.join(".ruach").join("index.db")
    }
}
