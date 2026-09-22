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
mod tests {
    use super::*;

    fn manager_in(dir: &std::path::Path) -> ThemeManager {
        ThemeManager {
            themes_dir: dir.to_path_buf(),
            themes_file: dir.join("themes.json"),
        }
    }

    fn sample_theme(name: &str) -> ThemeColors {
        ThemeColors {
            name: name.to_string(),
            bg_primary: "#111111".to_string(),
            bg_surface: "#222222".to_string(),
            bg_input: "#333333".to_string(),
            text_primary: "#eeeeee".to_string(),
            text_muted: "#999999".to_string(),
            border: "#444444".to_string(),
            accent: "#5588ff".to_string(),
            accent_play: "#33cc33".to_string(),
            accent_stop: "#cc3333".to_string(),
            pedal_color: "#ffaa00".to_string(),
            accent_loaded: "#ccaa00".to_string(),
            knob_color: "#ffffff".to_string(),
            builtin: false,
        }
    }

    mod test_get_active_name {
        use super::*;

        #[test]
        fn test_missing_file_returns_none() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            assert_eq!(manager.get_active_name(), None);
        }

        #[test]
        fn test_corrupted_file_returns_none() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("themes.json"), "not json").unwrap();
            let manager = manager_in(dir.path());
            assert_eq!(manager.get_active_name(), None);
        }

        #[test]
        fn test_returns_stored_active_name() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.set_active_name("Light");
            assert_eq!(manager.get_active_name(), Some("Light".to_string()));
        }
    }

    mod test_set_active_name {
        use super::*;

        #[test]
        fn test_persists_across_new_manager_instances() {
            let dir = tempfile::tempdir().unwrap();
            manager_in(dir.path()).set_active_name("Hatsune Miku");
            let reloaded = manager_in(dir.path());
            assert_eq!(reloaded.get_active_name(), Some("Hatsune Miku".to_string()));
        }

        #[test]
        fn test_overwrites_previous_active_name() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.set_active_name("Light");
            manager.set_active_name("Midnight");
            assert_eq!(manager.get_active_name(), Some("Midnight".to_string()));
        }

        #[test]
        fn test_preserves_existing_custom_themes() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.set_active_name("Sunset");
            assert_eq!(manager.custom_themes().len(), 1);
            assert_eq!(manager.get_active_name(), Some("Sunset".to_string()));
        }
    }

    mod test_save_custom {
        use super::*;

        #[test]
        fn test_adds_a_new_custom_theme() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            let customs = manager.custom_themes();
            assert_eq!(customs.len(), 1);
            assert_eq!(customs[0].name, "Sunset");
        }

        #[test]
        fn test_replaces_existing_theme_with_same_name_instead_of_duplicating() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            let mut updated = sample_theme("Sunset");
            updated.accent = "#000000".to_string();
            manager.save_custom(updated);
            let customs = manager.custom_themes();
            assert_eq!(customs.len(), 1);
            assert_eq!(customs[0].accent, "#000000");
        }

        #[test]
        fn test_multiple_distinct_custom_themes_all_persist() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.save_custom(sample_theme("Ocean"));
            let mut names: Vec<String> = manager.custom_themes().iter().map(|t| t.name.clone()).collect();
            names.sort();
            assert_eq!(names, vec!["Ocean".to_string(), "Sunset".to_string()]);
        }
    }

    mod test_delete_custom {
        use super::*;

        #[test]
        fn test_removes_the_named_theme() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.delete_custom("Sunset");
            assert!(manager.custom_themes().is_empty());
        }

        #[test]
        fn test_deleting_nonexistent_theme_is_a_no_op() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.delete_custom("DoesNotExist");
            assert_eq!(manager.custom_themes().len(), 1);
        }

        #[test]
        fn test_deleting_the_active_theme_resets_active_to_fallback() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.set_active_name("Sunset");
            manager.delete_custom("Sunset");
            assert_eq!(manager.get_active_name(), Some(FALLBACK_THEME_NAME.to_string()));
        }

        #[test]
        fn test_deleting_a_non_active_theme_leaves_active_name_untouched() {
            let dir = tempfile::tempdir().unwrap();
            let manager = manager_in(dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.save_custom(sample_theme("Ocean"));
            manager.set_active_name("Sunset");
            manager.delete_custom("Ocean");
            assert_eq!(manager.get_active_name(), Some("Sunset".to_string()));
        }
    }

    mod test_set_themes_dir {
        use super::*;

        #[test]
        fn test_migrates_existing_data_to_new_directory() {
            let old_dir = tempfile::tempdir().unwrap();
            let new_dir = tempfile::tempdir().unwrap();
            let mut manager = manager_in(old_dir.path());
            manager.save_custom(sample_theme("Sunset"));
            manager.set_active_name("Sunset");

            manager.set_themes_dir(new_dir.path().to_path_buf());

            assert_eq!(manager.themes_file(), new_dir.path().join("themes.json"));
            assert_eq!(manager.get_active_name(), Some("Sunset".to_string()));
            assert_eq!(manager.custom_themes().len(), 1);
        }

        #[test]
        fn test_empty_source_does_not_create_a_file_at_the_new_location() {
            let old_dir = tempfile::tempdir().unwrap();
            let new_dir = tempfile::tempdir().unwrap();
            let mut manager = manager_in(old_dir.path());
            manager.set_themes_dir(new_dir.path().to_path_buf());
            assert!(!new_dir.path().join("themes.json").exists());
        }
    }
}
