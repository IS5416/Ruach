use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    pub rel_path: String,
    pub title: String,
    /// 1 = title match, 0 = body match. Frontend sorts by this.
    pub score: u8,
}

/// Full-text search over the sidecar FTS5 index (trigram tokenizer —
/// CJK substring friendly). Implemented in P5.
pub struct SearchService;

impl SearchService {
    pub fn query(_q: &str) -> Result<Vec<SearchHit>, AppError> {
        Err(AppError::NotImplemented("SearchService::query"))
    }
}
