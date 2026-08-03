use crate::error::AppError;
use crate::services::attachment::{AttachmentResult, AttachmentService};
use crate::services::config::AppSettings;
use crate::services::document::{DocOpenResult, DocumentService};
use crate::services::export::{ExportFormat, ExportService};
use crate::services::index::IndexService;
use crate::services::search::{SearchHit, SearchService};
use crate::services::vault::{TreeNode, VaultService};
use crate::services::{db::Database, window::WindowManager};
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

fn with_vault<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&std::path::Path, &Mutex<Option<Database>>) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let vault = state
        .vault
        .lock()
        .expect("vault mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Vault("no vault open".to_string()))?;
    f(&vault, &state.db)
}

#[tauri::command]
pub fn vault_open(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    let vault = PathBuf::from(&path);
    VaultService::validate(&vault)?;
    let db = Database::open(VaultService::sidecar_path(&vault))?;
    *state.db.lock().expect("db mutex poisoned") = Some(db);
    *state.vault.lock().expect("vault mutex poisoned") = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn doc_open(state: State<'_, AppState>, rel_path: String) -> Result<DocOpenResult, AppError> {
    with_vault(&state, |vault, _| DocumentService::open(vault, &rel_path))
}

#[tauri::command]
pub fn doc_save(
    state: State<'_, AppState>,
    rel_path: String,
    content: String,
    expected_mtime: Option<i64>,
) -> Result<(), AppError> {
    with_vault(&state, |vault, _| {
        DocumentService::save(vault, &rel_path, &content, expected_mtime)
    })
}

#[tauri::command]
pub fn vault_scan(state: State<'_, AppState>) -> Result<Vec<TreeNode>, AppError> {
    with_vault(&state, |vault, _| VaultService::scan(vault))
}

#[tauri::command]
pub fn index_file(state: State<'_, AppState>, rel_path: String) -> Result<(), AppError> {
    with_vault(&state, |vault, _| IndexService::index_file(vault, &rel_path))
}

#[tauri::command]
pub fn index_reindex(state: State<'_, AppState>) -> Result<u32, AppError> {
    with_vault(&state, |vault, _| IndexService::reindex(vault))
}

#[tauri::command]
pub fn search_query(_state: State<'_, AppState>, q: String) -> Result<Vec<SearchHit>, AppError> {
    SearchService::query(&q)
}

#[tauri::command]
pub fn attach_paste(
    _state: State<'_, AppState>,
    data_url: String,
) -> Result<AttachmentResult, AppError> {
    // P4: decode base64 data URL and delegate to AttachmentService.
    let bytes = data_url.into_bytes();
    AttachmentService::save_paste(&bytes, None)
}

#[tauri::command]
pub fn render_markdown(_content: String) -> Result<String, AppError> {
    Err(AppError::NotImplemented("render_markdown"))
}

#[tauri::command]
pub fn export_document(
    _state: State<'_, AppState>,
    rel_path: String,
    format: ExportFormat,
    dest_dir: Option<String>,
) -> Result<String, AppError> {
    ExportService::export(&rel_path, format, dest_dir.as_deref())
}

#[tauri::command]
pub fn window_create(
    app: tauri::AppHandle,
    rel_path: Option<String>,
) -> Result<(), AppError> {
    WindowManager::create_window(&app, rel_path.as_deref())
}

#[tauri::command]
pub fn config_load(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    state.config.load()
}

#[tauri::command]
pub fn config_save(state: State<'_, AppState>, settings: AppSettings) -> Result<(), AppError> {
    state.config.save(&settings)
}
