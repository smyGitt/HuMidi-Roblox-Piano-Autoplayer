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
        viz_note_left: "#5588ff".to_string(),
        viz_note_right: "#33cc33".to_string(),
        viz_note_other: "#999999".to_string(),
        viz_progress_line: "#cc3333".to_string(),
        viz_measure_line: "#444444".to_string(),
        viz_key_white: "#ffffff".to_string(),
        viz_key_black: "#222222".to_string(),
        viz_key_pressed: "#33cc33".to_string(),
        builtin: false,
    }
}

mod test_theme_colors_format {
    use super::*;

    #[test]
    fn test_theme_saved_before_visualizer_colors_still_loads_with_them_empty() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = r##"{"active":"Sunset","custom":[{"name":"Sunset","bg_primary":"#111111","bg_surface":"#222222","bg_input":"#333333","text_primary":"#eeeeee","text_muted":"#999999","border":"#444444","accent":"#5588ff","accent_play":"#33cc33","accent_stop":"#cc3333","pedal_color":"#ffaa00","accent_loaded":"#ccaa00","knob_color":"#ffffff"}]}"##;
        std::fs::write(dir.path().join("themes.json"), legacy).unwrap();
        let customs = manager_in(dir.path()).custom_themes();
        assert_eq!(customs.len(), 1);
        assert_eq!(customs[0].accent, "#5588ff");
        assert_eq!(customs[0].viz_note_left, "");
        assert_eq!(customs[0].viz_key_pressed, "");
    }

    #[test]
    fn test_visualizer_colors_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let manager = manager_in(dir.path());
        manager.save_custom(sample_theme("Sunset"));
        let reloaded = manager_in(dir.path()).custom_themes();
        assert_eq!(reloaded[0], sample_theme("Sunset"));
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
