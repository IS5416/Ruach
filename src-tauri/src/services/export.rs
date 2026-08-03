use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Html,
    Pdf,
}

/// Export documents via the shared render pipeline. Implemented in P8.
pub struct ExportService;

impl ExportService {
    pub fn export(
        _rel_path: &str,
        _format: ExportFormat,
        _dest_dir: Option<&str>,
    ) -> Result<String, AppError> {
        Err(AppError::NotImplemented("ExportService::export"))
    }
}
