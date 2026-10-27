use std::path::{Path, PathBuf};

use super::config_manager::determine_root_dir;

pub const FALLBACK_THEME_NAME: &str = "Midnight";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ThemeColors {
    pub name: String,
    pub bg_primary: String,
    pub bg_surface: String,
    pub bg_input: String,
    pub text_primary: String,
    pub text_muted: String,
    pub border: String,
    pub accent: String,
    pub accent_play: String,
    pub accent_stop: String,
    pub pedal_color: String,
    pub accent_loaded: String,
    pub knob_color: String,
    #[serde(default)]
    pub viz_note_left: String,
    #[serde(default)]
    pub viz_note_right: String,
    #[serde(default)]
    pub viz_note_other: String,
    #[serde(default)]
    pub viz_progress_line: String,
    #[serde(default)]
    pub viz_measure_line: String,
    #[serde(default)]
    pub viz_key_white: String,
    #[serde(default)]
    pub viz_key_black: String,
    #[serde(default)]
    pub viz_key_pressed: String,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct ThemesFile {
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    custom: Vec<ThemeColors>,
}

pub struct ThemeManager {
    themes_dir: PathBuf,
    themes_file: PathBuf,
}

impl ThemeManager {
    pub fn new() -> Self {
        let themes_dir = determine_root_dir();
        let themes_file = themes_dir.join("themes.json");
        ThemeManager { themes_dir, themes_file }
    }

    pub fn themes_file(&self) -> &Path {
        &self.themes_file
    }

    fn load_raw(&self) -> ThemesFile {
        let Ok(contents) = std::fs::read_to_string(&self.themes_file) else {
            return ThemesFile::default();
        };
        serde_json::from_str(&contents).unwrap_or_default()
    }

    fn save_raw(&self, data: &ThemesFile) {
        let _ = std::fs::create_dir_all(&self.themes_dir);
        if let Ok(serialized) = serde_json::to_string_pretty(data) {
            let _ = std::fs::write(&self.themes_file, serialized);
        }
    }

    pub fn set_themes_dir(&mut self, directory: PathBuf) {
        let new_file = directory.join("themes.json");
        let existing = self.load_raw();
        self.themes_dir = directory;
        self.themes_file = new_file;
        if existing.active.is_some() || !existing.custom.is_empty() {
            self.save_raw(&existing);
        }
    }

    pub fn custom_themes(&self) -> Vec<ThemeColors> {
        self.load_raw().custom
    }

    pub fn save_custom(&self, theme: ThemeColors) {
        let mut data = self.load_raw();
        data.custom.retain(|t| t.name != theme.name);
        data.custom.push(theme);
        self.save_raw(&data);
    }

    pub fn delete_custom(&self, name: &str) {
        let mut data = self.load_raw();
        data.custom.retain(|t| t.name != name);
        if data.active.as_deref() == Some(name) {
            data.active = Some(FALLBACK_THEME_NAME.to_string());
        }
        self.save_raw(&data);
    }

    pub fn get_active_name(&self) -> Option<String> {
        self.load_raw().active
    }

    pub fn set_active_name(&self, name: &str) {
        let mut data = self.load_raw();
        data.active = Some(name.to_string());
        self.save_raw(&data);
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
