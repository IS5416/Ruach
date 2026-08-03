use crate::error::AppError;
use std::path::Path;

/// Extracted knowledge markers: `#tag` on its own line and `[[target]]` links.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DocumentMarkers {
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

/// Derives tags/links/FTS rows for files. Lazy indexing: index a file on
/// open, re-scan changed files on startup, full `reindex` as fallback.
pub struct IndexService;

impl IndexService {
    pub fn extract_markers(content: &str) -> DocumentMarkers {
        let mut markers = DocumentMarkers::default();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(_tag) = trimmed.strip_prefix("# ") {
                // Heading line, not a tag. Skip.
                continue;
            }
            // Whole-line tags: `#tag` / `#tag/tag2` / comma-separated `#a, #b`
            if trimmed.starts_with('#') && !trimmed.starts_with("##") {
                for part in trimmed.trim_start_matches('#').split(',') {
                    let tag = part.trim().trim_matches('#').trim();
                    if !tag.is_empty() && !tag.contains(char::is_whitespace) {
                        markers.tags.push(tag.to_string());
                    }
                }
            }
        }
        for start in content.match_indices("[[").map(|(i, _)| i) {
            let rest = &content[start + 2..];
            let end = rest.find("]]");
            if let Some(end) = end {
                let target = rest[..end].trim();
                if !target.is_empty() {
                    markers.links.push(target.to_string());
                }
            }
        }
        markers
    }

    /// Index one file into the sidecar (files/tags/links + FTS row).
    /// Implemented in P2.
    pub fn index_file(_vault: &Path, _rel_path: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("IndexService::index_file"))
    }

    /// Full rebuild of the index.
    pub fn reindex(_vault: &Path) -> Result<u32, AppError> {
        Err(AppError::NotImplemented("IndexService::reindex"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_tags_and_links() {
        let content = "# 标题\n\n正文\n\n#tag, #daily/2026-08-03\n\n参考 [[另一个笔记]] 和 [[未创建]]\n";
        let m = IndexService::extract_markers(content);
        assert!(m.tags.contains(&"tag".to_string()));
        assert!(m.tags.contains(&"daily/2026-08-03".to_string()));
        assert_eq!(m.links, vec!["另一个笔记", "未创建"]);
    }

    #[test]
    fn heading_is_not_tag() {
        let m = IndexService::extract_markers("# 标题\n");
        assert!(m.tags.is_empty());
    }
}
