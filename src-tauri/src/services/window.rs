use crate::error::AppError;
use tauri::AppHandle;

/// Window sessions: each window owns one document session; `doc:changed`
/// events keep windows consistent. Implemented in P6.
pub struct WindowManager;

impl WindowManager {
    pub fn create_window(_app: &AppHandle, _rel_path: Option<&str>) -> Result<(), AppError> {
        Err(AppError::NotImplemented("WindowManager::create_window"))
    }
}
