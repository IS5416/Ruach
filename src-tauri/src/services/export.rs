use crate::error::AppError;
use crate::services::render::RenderService;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Html,
    Pdf,
}

/// Self-contained document shell. Typography mirrors the app's warm-paper
/// default; keeps exports readable outside the app.
const EXPORT_CSS: &str = r#"
body { margin: 0; background: #f6f1e7; color: #3d3629;
       font-family: Georgia, "Songti SC", "STSong", serif;
       font-size: 16px; line-height: 1.9; -webkit-font-smoothing: antialiased; }
.markdown-body { max-width: 46em; margin: 0 auto; padding: 48px 28px; }
h1, h2, h3 { font-weight: 600; line-height: 1.4; }
h1 { font-size: 1.7em; } h2 { font-size: 1.35em; border-bottom: 1px solid #d9cbb0; padding-bottom: .25em; }
p, ul, ol, blockquote, pre, table { margin: .9em 0; }
a { color: #8a6d3b; text-decoration: none; }
blockquote { margin-left: 0; padding-left: 1.2em; border-left: 2px solid #d9cbb0; color: #6b6150; }
code { font-family: Consolas, "Cascadia Code", monospace; font-size: .88em; background: #efe8d6; padding: .15em .4em; }
pre { background: #efe8d6; border: 1px solid #d9cbb0; border-radius: 6px; padding: 1em; overflow-x: auto; }
pre code { background: transparent; padding: 0; }
table { border-collapse: collapse; font-size: .92em; }
th, td { border: 1px solid #d9cbb0; padding: .4em .8em; }
th { background: #efe8d6; }
img { max-width: 100%; }
"#;

/// Export documents via the shared render pipeline. HTML writes a
/// self-contained file; PDF goes through the print pipeline on the
/// frontend (this service keeps the interface for it).
pub struct ExportService;

impl ExportService {
    pub fn export(
        vault: &Path,
        rel_path: &str,
        format: ExportFormat,
        dest_dir: Option<&str>,
    ) -> Result<String, AppError> {
        let content = std::fs::read_to_string(vault.join(rel_path))
            .map_err(|_| AppError::NotFound(rel_path.to_string()))?;
        let body = RenderService::render_markdown(&content)?;
        let stem = Path::new(rel_path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "export".to_string());

        let dest = match (format, dest_dir) {
            (ExportFormat::Html, Some(dir)) => PathBuf::from(dir).join(format!("{stem}.html")),
            (ExportFormat::Html, None) => vault.join("export").join(format!("{stem}.html")),
            (ExportFormat::Pdf, _) => {
                return Err(AppError::NotImplemented(
                    "ExportService::export pdf (use print pipeline)",
                ));
            }
        };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let doc = format!(
            "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n<div class=\"markdown-body\">{}</div>\n</body>\n</html>\n",
            html_escape(&stem),
            EXPORT_CSS,
            body
        );
        std::fs::write(&dest, doc)?;
        Ok(dest.to_string_lossy().into_owned())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db::SCHEMA_SQL;
    use rusqlite::Connection;

    #[test]
    fn export_html_writes_self_contained_file() {
        let dir = std::env::temp_dir().join(format!(
            "ruach-export-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("风的形状.md"), "# 风的形状\n\n正文\n").unwrap();

        // Export needs a live RenderService; the db is unused here.
        let _conn = Connection::open_in_memory().unwrap();
        let out = ExportService::export(&dir, "风的形状.md", ExportFormat::Html, None).unwrap();
        let raw = std::fs::read_to_string(&out).unwrap();
        assert!(raw.contains("<h1"));
        assert!(raw.contains("风的形状"));
        assert!(raw.contains("<style>"));
        assert!(raw.starts_with("<!doctype html>"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_table_migrates() {
        // Fresh v1 schema + v2 migration must both leave snapshots present.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute_batch(crate::services::db::SCHEMA_V2_SQL).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'snapshots'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
