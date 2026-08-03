use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    WarmPaper,
    ColdStone,
    NightInk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontPreset {
    Serif,
    SansSerif,
}

/// Application-level settings (per device). Lives in the app config dir,
/// NOT in the vault sidecar — the sidecar only holds vault-derived data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: ThemeKind,
    pub font_preset: FontPreset,
    pub line_height: f32,
    pub page_width: u32,
    pub show_file_tree: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeKind::WarmPaper,
            font_preset: FontPreset::Serif,
            line_height: 1.8,
            page_width: 720,
            show_file_tree: true,
        }
    }
}

pub struct ConfigService {
    path: PathBuf,
}

impl ConfigService {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<AppSettings, AppError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str(&raw).map_err(|e| AppError::Parse(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), AppError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let raw = serde_json::to_string_pretty(settings)?;
        fs::write(&self.path, raw)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "ruach-config-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let service = ConfigService::new(dir.join("settings.json"));

        let settings = AppSettings {
            theme: ThemeKind::NightInk,
            font_preset: FontPreset::SansSerif,
            ..AppSettings::default()
        };
        service.save(&settings).expect("save");

        let loaded = service.load().expect("load");
        assert_eq!(loaded.theme, ThemeKind::NightInk);
        assert_eq!(loaded.font_preset, FontPreset::SansSerif);
        assert_eq!(loaded.line_height, 1.8);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let service = ConfigService::new(
            std::env::temp_dir().join("ruach-config-missing-any-key.json"),
        );
        let loaded = service.load().expect("load");
        assert_eq!(loaded.theme, ThemeKind::WarmPaper);
    }
}
