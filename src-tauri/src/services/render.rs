use crate::error::AppError;

/// Single markdown engine: comrak (GFM). Renders preview HTML and export
/// HTML through the same pipeline. Raw HTML is dropped by default (safe).
/// Implemented in P3.
pub struct RenderService;

impl RenderService {
    pub fn render_markdown(_content: &str) -> Result<String, AppError> {
        Err(AppError::NotImplemented("RenderService::render_markdown"))
    }
}
