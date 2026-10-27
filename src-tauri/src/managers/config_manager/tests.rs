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
