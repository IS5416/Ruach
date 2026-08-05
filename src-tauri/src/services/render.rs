use crate::error::AppError;

/// Single markdown engine: comrak (GFM). Renders preview HTML and export
/// HTML through the same pipeline. Raw HTML is dropped by default (safe).
pub struct RenderService;

impl RenderService {
    pub fn render_markdown(content: &str) -> Result<String, AppError> {
        let mut options = comrak::Options::default();
        // GFM extensions.
        options.extension.autolink = true;
        options.extension.table = true;
        options.extension.strikethrough = true;
        options.extension.tasklist = true;
        options.extension.superscript = true;
        options.extension.footnotes = true;
        options.extension.description_lists = true;
        options.extension.header_ids = Some(String::new());
        options.extension.tagfilter = true;
        // Smart punctuation (curly quotes, ellipsis).
        options.parse.smart = true;
        // Security: raw HTML never reaches the preview.
        options.render.unsafe_ = false;

        let mut arena = comrak::Arena::new();
        let root = comrak::parse_document(&mut arena, content, &options);
        let mut buf: Vec<u8> = Vec::new();
        comrak::format_html(root, &options, &mut buf)
            .map_err(|e| AppError::Other(format!("render failed: {e}")))?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(content: &str) -> String {
        RenderService::render_markdown(content).expect("render")
    }

    #[test]
    fn renders_gfm_table() {
        let html = render("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn renders_strikethrough() {
        let html = render("~~删除~~");
        assert!(html.contains("<del>删除</del>"));
    }

    #[test]
    fn drops_raw_html() {
        let html = render("正文\n\n<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>");
        assert!(!html.contains("<script>"));
        assert!(!html.contains("onerror"));
    }

    #[test]
    fn renders_code_block() {
        let html = render("```rust\nfn main() {}\n```\n");
        assert!(html.contains("<pre>"));
        assert!(html.contains("fn main() {}"));
    }

    #[test]
    fn renders_tasklist() {
        let html = render("- [x] done\n- [ ] todo\n");
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked"));
    }

    #[test]
    fn renders_heading_id() {
        let html = render("# 风的形状\n");
        assert!(html.contains("<h1"));
        assert!(html.contains("id="));
    }
}
