use crate::error::AppError;
use crate::services::attachment::{AttachmentData, AttachmentResult, AttachmentService};
use crate::services::config::AppSettings;
use crate::services::db::Database;
use crate::services::document::{DocOpenResult, DocumentService, SessionDraft, SessionInfo};
use crate::services::export::{ExportFormat, ExportService};
use crate::services::index::IndexService;
use crate::services::render::RenderService;
use crate::services::search::{SearchHit, SearchService};
use crate::services::snapshot::SnapshotService;
use crate::services::vault::{TreeNode, VaultService};
use crate::services::window::WindowManager;
use crate::state::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

fn with_vault<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(
        &std::path::Path,
        &rusqlite::Connection,
    ) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let vault = state
        .vault
        .lock()
        .expect("vault mutex poisoned")
        .clone()
        .ok_or_else(|| AppError::Vault("no vault open".to_string()))?;
    with_db(state, |conn| f(&vault, conn))
}

/// Borrow the sidecar connection for the duration of `f`; the guard is
/// bound to a named local so the borrow outlives the call.
fn with_db<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let guard = state.db.lock().expect("db mutex poisoned");
    let db = guard
        .as_ref()
        .ok_or_else(|| AppError::Vault("no vault open".to_string()))?;
    let conn = db.conn();
    f(&conn)
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
pub fn doc_open(
    app: AppHandle,
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<DocOpenResult, AppError> {
    let res = with_vault(&state, |vault, conn| {
        // Validate before indexing — index_file reads the file, so an
        // invalid rel_path must never touch disk (no FTS read side-channel).
        DocumentService::validate_rel_path(&rel_path)?;
        // Lazy index on open: tags/links/FTS stay fresh without a watcher.
        let _ = IndexService::index_file(conn, vault, &rel_path);
        DocumentService::open(vault, &rel_path)
    })?;
    let _ = app.emit("doc:opened", &res.meta.rel_path);
    Ok(res)
}

#[tauri::command]
pub fn doc_save(
    app: AppHandle,
    state: State<'_, AppState>,
    rel_path: String,
    content: String,
    expected_mtime: Option<i64>,
) -> Result<i64, AppError> {
    let res = with_vault(&state, |vault, conn| {
        let mtime = DocumentService::save(conn, vault, &rel_path, &content, expected_mtime)?;
        // Keep tags/links/FTS fresh right after save (lazy indexing).
        let _ = IndexService::index_file_content(conn, &rel_path, &content);
        Ok(mtime)
    });
    if res.is_ok() {
        let _ = app.emit("doc:changed", &rel_path);
    }
    res
}

#[tauri::command]
pub fn session_flush(
    state: State<'_, AppState>,
    doc_key: String,
    content: String,
    cursor: Option<i64>,
) -> Result<(), AppError> {
    with_db(&state, |conn| {
        DocumentService::session_flush(conn, &doc_key, &content, cursor)
    })
}

#[tauri::command]
pub fn session_list(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, AppError> {
    with_db(&state, |conn| DocumentService::session_list(conn))
}

#[tauri::command]
pub fn session_restore(
    state: State<'_, AppState>,
    doc_key: String,
) -> Result<SessionDraft, AppError> {
    with_db(&state, |conn| DocumentService::session_restore(conn, &doc_key))
}

#[tauri::command]
pub fn session_discard(
    state: State<'_, AppState>,
    doc_key: String,
) -> Result<(), AppError> {
    with_db(&state, |conn| DocumentService::session_discard(conn, &doc_key))
}

#[tauri::command]
pub fn vault_scan(state: State<'_, AppState>) -> Result<Vec<TreeNode>, AppError> {
    with_vault(&state, |vault, conn| VaultService::scan(conn, vault))
}

#[tauri::command]
pub fn index_file(state: State<'_, AppState>, rel_path: String) -> Result<(), AppError> {
    with_vault(&state, |vault, conn| {
        DocumentService::validate_rel_path(&rel_path)?;
        IndexService::index_file(conn, vault, &rel_path)
    })
}

#[tauri::command]
pub fn index_reindex(state: State<'_, AppState>) -> Result<u32, AppError> {
    with_vault(&state, |vault, conn| IndexService::reindex(conn, vault))
}

#[tauri::command]
pub fn search_query(state: State<'_, AppState>, q: String) -> Result<Vec<SearchHit>, AppError> {
    with_db(&state, |conn| SearchService::query(conn, &q))
}

#[tauri::command]
pub fn attach_paste(
    state: State<'_, AppState>,
    data_url: String,
    orig_name: Option<String>,
) -> Result<AttachmentResult, AppError> {
    let (bytes, mime) = crate::services::attachment::decode_data_url(&data_url)?;
    with_vault(&state, |vault, _| {
        AttachmentService::save_paste(vault, &bytes, &mime, orig_name.as_deref())
    })
}

#[tauri::command]
pub fn attach_read(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<AttachmentData, AppError> {
    with_vault(&state, |vault, _| AttachmentService::read(vault, &rel_path))
}

#[tauri::command]
pub fn render_markdown(content: String) -> Result<String, AppError> {
    RenderService::render_markdown(&content)
}

#[tauri::command]
pub fn export_document(
    state: State<'_, AppState>,
    rel_path: String,
    format: ExportFormat,
    dest_dir: Option<String>,
) -> Result<String, AppError> {
    with_vault(&state, |vault, _| {
        ExportService::export(vault, &rel_path, format, dest_dir.as_deref())
    })
}

#[tauri::command]
pub fn snapshot_create(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<i64, AppError> {
    with_db(&state, |conn| SnapshotService::create(conn, &rel_path))
}

#[tauri::command]
pub fn snapshot_restore(
    state: State<'_, AppState>,
    rel_path: String,
    snapshot_at: i64,
) -> Result<String, AppError> {
    with_db(&state, |conn| SnapshotService::restore(conn, &rel_path, snapshot_at))
}

#[tauri::command]
pub fn snapshot_list(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<Vec<i64>, AppError> {
    with_db(&state, |conn| SnapshotService::list(conn, &rel_path))
}

#[tauri::command]
pub fn window_create(
    app: AppHandle,
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
