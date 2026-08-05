use crate::error::AppError;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

/// Window sessions: each window owns one document session; `doc:changed`
/// events keep windows consistent. The target document is carried in the
/// window URL (`index.html?doc=<rel_path>`), read on the frontend's mount —
/// no event race, the URL is available before React boots.
pub struct WindowManager;

impl WindowManager {
    pub fn create_window(app: &AppHandle, rel_path: Option<&str>) -> Result<(), AppError> {
        let label = format!(
            "editor-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let url = match rel_path {
            Some(path) => {
                WebviewUrl::App(format!("index.html?doc={}", urlencode(path)).into())
            }
            None => WebviewUrl::App("index.html".into()),
        };
        WebviewWindowBuilder::new(app, &label, url)
            .title("Ruach")
            .inner_size(900.0, 700.0)
            .build()
            .map_err(|e| AppError::Window(e.to_string()))?;
        Ok(())
    }
}

/// Minimal percent-encoding for paths that end up in a URL query.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'%' => out.push_str("%25"),
            b'?' => out.push_str("%3F"),
            b'#' => out.push_str("%23"),
            b'&' => out.push_str("%26"),
            b' ' => out.push_str("%20"),
            b'=' => out.push_str("%3D"),
            0x21..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_query_characters() {
        assert_eq!(urlencode("notes/风的形状.md"), "notes/%E9%A3%8E%E7%9A%84%E5%BD%A2%E7%8A%B6.md");
        assert_eq!(urlencode("a&b?c#d"), "a%26b%3Fc%23d");
    }
}
