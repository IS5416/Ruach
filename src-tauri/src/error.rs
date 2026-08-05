use serde::ser::{Serialize, SerializeMap, Serializer};

/// Unified application error. Every Tauri command returns `Result<T, AppError>`;
/// the error serializes to `{ "code": "...", "message": "..." }` for the frontend.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("document not found: {0}")]
    NotFound(String),
    #[error("file changed on disk; refusing to overwrite")]
    FileChanged,
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("invalid vault: {0}")]
    Vault(String),
    #[error("window error: {0}")]
    Window(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("vault database version {found} is newer than app version {max}")]
    SchemaVersion { found: i64, max: i64 },
    #[error("parse error: {0}")]
    Parse(String),
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Db(_) => "db",
            Self::Json(_) => "json",
            Self::NotFound(_) => "not_found",
            Self::FileChanged => "file_changed",
            Self::InvalidPath(_) => "invalid_path",
            Self::Vault(_) => "vault",
            Self::Window(_) => "window",
            Self::SchemaVersion { .. } => "schema_version",
            Self::Parse(_) => "parse",
            Self::NotImplemented(_) => "not_implemented",
            Self::Other(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}
