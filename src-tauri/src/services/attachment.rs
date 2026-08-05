use crate::error::AppError;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AttachmentResult {
    /// Vault-relative path, e.g. `attachments/1754188800123-ab12.png`.
    pub rel_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AttachmentData {
    pub mime: String,
    /// base64 payload — inlined into the preview iframe as a data URL.
    pub base64: String,
}

/// MIME → file extension mapping for pasted images.
pub fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        _ => "bin",
    }
}

/// Decode a `data:<mime>;base64,<payload>` URL into bytes + mime.
pub fn decode_data_url(data_url: &str) -> Result<(Vec<u8>, String), AppError> {
    let rest = data_url
        .strip_prefix("data:")
        .ok_or_else(|| AppError::Parse("not a data url".to_string()))?;
    let (meta, payload) = rest
        .split_once(',')
        .ok_or_else(|| AppError::Parse("data url missing comma".to_string()))?;
    let mime = meta
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .to_string();
    if !meta.contains(";base64") {
        return Err(AppError::Parse("only base64 data urls supported".to_string()));
    }
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| AppError::Parse(format!("invalid base64: {e}")))?;
    Ok((bytes, mime))
}

/// Pasted images / dropped files land in `<vault>/attachments/`, referenced
/// from documents by relative path. Names are `{millis}-{content-hash4}.{ext}`
/// — unique per paste without a random crate.
pub struct AttachmentService;

impl AttachmentService {
    pub fn save_paste(
        vault: &Path,
        bytes: &[u8],
        mime: &str,
        orig_name: Option<&str>,
    ) -> Result<AttachmentResult, AppError> {
        let dir = vault.join("attachments");
        std::fs::create_dir_all(&dir)?;

        let ext = if let Some(name) = orig_name {
            // Trust the original extension when it looks like an image ext.
            Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .filter(|e| matches!(*e, "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp"))
                .map(|e| if e == "jpeg" { "jpg" } else { e })
                .unwrap_or_else(|| mime_to_ext(mime))
        } else {
            mime_to_ext(mime)
        };

        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let hash = content_hash4(bytes);
        let name = format!("{millis}-{hash}.{ext}");
        let abs = dir.join(&name);
        std::fs::write(&abs, bytes)?;

        Ok(AttachmentResult {
            rel_path: format!("attachments/{name}"),
        })
    }

    /// Read an attachment back as base64 for inline preview.
    pub fn read(vault: &Path, rel_path: &str) -> Result<AttachmentData, AppError> {
        let abs = vault.join(rel_path);
        let bytes = std::fs::read(&abs)
            .map_err(|_| AppError::NotFound(rel_path.to_string()))?;
        let mime = Path::new(rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| match e {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "bmp" => "image/bmp",
                _ => "application/octet-stream",
            })
            .unwrap_or("application/octet-stream")
            .to_string();
        use base64::Engine;
        Ok(AttachmentData {
            mime,
            base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }
}

/// Cheap stable digest: FNV-1a over the bytes, last 4 hex chars.
fn content_hash4(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:04x}", (hash & 0xffff) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_ext_mapping() {
        assert_eq!(mime_to_ext("image/png"), "png");
        assert_eq!(mime_to_ext("image/jpeg"), "jpg");
        assert_eq!(mime_to_ext("image/webp"), "webp");
        assert_eq!(mime_to_ext("application/pdf"), "bin");
    }

    #[test]
    fn decodes_data_url() {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode("PNGDATA");
        let (bytes, mime) = decode_data_url(&format!("data:image/png;base64,{b64}")).unwrap();
        assert_eq!(bytes, b"PNGDATA");
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn rejects_bad_data_urls() {
        assert!(decode_data_url("not-a-url").is_err());
        assert!(decode_data_url("data:image/png,plain").is_err());
    }

    #[test]
    fn save_paste_writes_unique_files() {
        let dir = std::env::temp_dir().join(format!(
            "ruach-attach-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let a = AttachmentService::save_paste(&dir, b"AAAA", "image/png", None).unwrap();
        let b = AttachmentService::save_paste(&dir, b"BBBB", "image/png", None).unwrap();
        assert_ne!(a.rel_path, b.rel_path);
        assert!(a.rel_path.ends_with(".png"));
        assert!(b.rel_path.starts_with("attachments/"));

        let abs_a = dir.join(&a.rel_path);
        assert!(abs_a.exists());
        assert_eq!(std::fs::read(&abs_a).unwrap(), b"AAAA");

        // Original extension wins when plausible.
        let c = AttachmentService::save_paste(&dir, b"CCCC", "image/png", Some("shot.webp")).unwrap();
        assert!(c.rel_path.ends_with(".webp"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
