use std::path::{Path, PathBuf};

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

pub(crate) fn determine_root_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".humidi")
        .join("config.json")
}

pub struct ConfigManager {
    pub save_dir: PathBuf,
    pub midi_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let root_dir = determine_root_dir();
        let save_dir = root_dir.join("saves");
        let _ = std::fs::create_dir_all(&save_dir);

        let config_dir = dirs::home_dir().unwrap_or_default().join(".humidi");
        let _ = std::fs::create_dir_all(&config_dir);
        let config_path = config_dir.join("config.json");

        ConfigManager {
            save_dir,
            midi_dir: PathBuf::new(),
            config_dir,
            config_path,
        }
    }

    pub fn load(&mut self) -> serde_json::Value {
        if !self.config_path.exists() {
            return empty_object();
        }
        let Ok(contents) = std::fs::read_to_string(&self.config_path) else {
            return empty_object();
        };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return empty_object();
        };

        if let Some(save_dir) = parsed.get("save_dir").and_then(|v| v.as_str()) {
            if Path::new(save_dir).exists() {
                self.save_dir = PathBuf::from(save_dir);
            }
        }
        if let Some(midi_dir) = parsed.get("midi_dir").and_then(|v| v.as_str()) {
            if Path::new(midi_dir).exists() {
                self.midi_dir = PathBuf::from(midi_dir);
            }
        }

        parsed
    }

    pub fn save(&self, config_data: &serde_json::Value) {
        let mut data = config_data.clone();
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert(
                "save_dir".to_string(),
                serde_json::Value::String(self.save_dir.to_string_lossy().to_string()),
            );
            map.insert(
                "midi_dir".to_string(),
                serde_json::Value::String(self.midi_dir.to_string_lossy().to_string()),
            );
        }
        if let Ok(serialized) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&self.config_path, serialized);
        }
    }

    pub fn set_save_dir(&mut self, new_dir: PathBuf) {
        self.save_dir = new_dir;
    }

    pub fn set_midi_dir(&mut self, new_dir: PathBuf) {
        self.midi_dir = new_dir;
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manager_in(dir: &std::path::Path) -> ConfigManager {
        ConfigManager {
            save_dir: dir.join("saves"),
            midi_dir: PathBuf::new(),
            config_dir: dir.to_path_buf(),
            config_path: dir.join("config.json"),
        }
    }

    mod test_load {
        use super::*;

        #[test]
        fn test_missing_file_returns_empty() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            assert_eq!(mgr.load(), empty_object());
        }

        #[test]
        fn test_corrupted_json_returns_empty() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            std::fs::write(&mgr.config_path, "not valid json { }}}").unwrap();
            assert_eq!(mgr.load(), empty_object());
        }

        #[test]
        fn test_valid_file_loaded() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            std::fs::write(&mgr.config_path, json!({"tempo": 110}).to_string()).unwrap();
            assert_eq!(mgr.load()["tempo"], 110);
        }
    }

    mod test_save {
        use super::*;

        #[test]
        fn test_save_creates_file() {
            let dir = tempfile::tempdir().unwrap();
            let mgr = manager_in(dir.path());
            mgr.save(&json!({"tempo": 95}));
            assert!(mgr.config_path.exists());
        }

        #[test]
        fn test_save_injects_save_dir() {
            let dir = tempfile::tempdir().unwrap();
            let mgr = manager_in(dir.path());
            mgr.save(&json!({}));
            let data: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&mgr.config_path).unwrap()).unwrap();
            assert!(data.get("save_dir").is_some());
        }

        #[test]
        fn test_round_trip() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            mgr.save(&json!({"tempo": 110, "pedal_style": "harmonic"}));
            let loaded = mgr.load();
            assert_eq!(loaded["tempo"], 110);
            assert_eq!(loaded["pedal_style"], "harmonic");
        }

        #[test]
        fn test_save_does_not_panic_on_non_object_input() {
            let dir = tempfile::tempdir().unwrap();
            let mgr = manager_in(dir.path());
            mgr.save(&json!([1, 2, 3]));
        }
    }

    mod test_set_save_dir {
        use super::*;

        #[test]
        fn test_updates_in_memory() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let new_dir = dir.path().join("newsaves");
            mgr.set_save_dir(new_dir.clone());
            assert_eq!(mgr.save_dir, new_dir);
        }

        #[test]
        fn test_does_not_persist_without_save() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            mgr.set_save_dir(dir.path().join("newsaves"));
            assert_eq!(mgr.load(), empty_object());
        }

        #[test]
        fn test_load_updates_save_dir_when_path_exists() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let real_dir = dir.path().join("mysaves");
            std::fs::create_dir(&real_dir).unwrap();
            mgr.set_save_dir(real_dir.clone());
            mgr.save(&json!({}));
            mgr.save_dir = PathBuf::from("old");
            mgr.load();
            assert_eq!(mgr.save_dir, real_dir);
        }

        #[test]
        fn test_load_ignores_nonexistent_save_dir() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let ghost = dir.path().join("ghost");
            mgr.save(&json!({"save_dir": ghost.to_string_lossy()}));
            let old = mgr.save_dir.clone();
            mgr.load();
            assert_eq!(mgr.save_dir, old);
        }
    }

    mod test_set_midi_dir {
        use super::*;

        #[test]
        fn test_updates_in_memory() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let new_dir = dir.path().join("midi");
            mgr.set_midi_dir(new_dir.clone());
            assert_eq!(mgr.midi_dir, new_dir);
        }

        #[test]
        fn test_load_ignores_nonexistent_midi_dir() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let ghost = dir.path().join("ghost_midi");
            mgr.save(&json!({"midi_dir": ghost.to_string_lossy()}));
            let old = mgr.midi_dir.clone();
            mgr.load();
            assert_eq!(mgr.midi_dir, old);
        }

        #[test]
        fn test_load_updates_midi_dir_when_path_exists() {
            let dir = tempfile::tempdir().unwrap();
            let mut mgr = manager_in(dir.path());
            let real_dir = dir.path().join("realmidi");
            std::fs::create_dir(&real_dir).unwrap();
            mgr.set_midi_dir(real_dir.clone());
            mgr.save(&json!({}));
            mgr.midi_dir = PathBuf::new();
            mgr.load();
            assert_eq!(mgr.midi_dir, real_dir);
        }
    }

    mod test_config_path {
        use super::*;

        #[test]
        fn test_ends_with_dot_humidi_config_json() {
            let p = config_path();
            assert!(p.ends_with(".humidi/config.json") || p.ends_with(".humidi\\config.json"));
        }
    }
}
