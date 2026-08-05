use crate::error::AppError;
use rusqlite::Connection;

/// Version snapshot history — table exists in the schema (v2), interface
/// defined, implementation deferred. Snapshots let users roll a document
/// back to an earlier saved state.
pub struct SnapshotService;

impl SnapshotService {
    /// Store a snapshot of the current disk content; returns snapshot_at.
    pub fn create(_conn: &Connection, _rel_path: &str) -> Result<i64, AppError> {
        Err(AppError::NotImplemented("SnapshotService::create"))
    }

    /// Return the content of a snapshot.
    pub fn restore(_conn: &Connection, _rel_path: &str, _snapshot_at: i64) -> Result<String, AppError> {
        Err(AppError::NotImplemented("SnapshotService::restore"))
    }

    /// List snapshot timestamps for a document, newest first.
    pub fn list(_conn: &Connection, _rel_path: &str) -> Result<Vec<i64>, AppError> {
        Err(AppError::NotImplemented("SnapshotService::list"))
    }
}
