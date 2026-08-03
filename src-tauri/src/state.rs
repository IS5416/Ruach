use crate::services::config::ConfigService;
use crate::services::db::Database;
use std::path::PathBuf;
use std::sync::Mutex;

/// Application-wide state, managed by Tauri.
pub struct AppState {
    /// Sidecar DB for the currently open vault; `None` until a vault opens.
    pub db: Mutex<Option<Database>>,
    /// Vault root; `None` until a vault opens.
    pub vault: Mutex<Option<PathBuf>>,
    /// Application-level settings (per device).
    pub config: ConfigService,
}
