use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AttachmentResult {
    /// Vault-relative path, e.g. `attachments/1754188800-a1b2.png`.
    pub rel_path: String,
}

/// Pasted images / dropped files land in `<vault>/attachments/`, referenced
/// from documents by relative path. Implemented in P4.
pub struct AttachmentService;

impl AttachmentService {
    /// Save pasted image data (data URL or raw bytes) into the attachment
    /// dir with a dedup name (timestamp + short random + ext).
    pub fn save_paste(_data: &[u8], _orig_name: Option<&str>) -> Result<AttachmentResult, AppError> {
        Err(AppError::NotImplemented("AttachmentService::save_paste"))
    }
}
