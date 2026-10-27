use super::*;

mod test_hotkey_persistence {
    use super::*;

    #[test]
    fn test_every_formatted_hotkey_parses_back_to_the_same_combo() {
        let mut mgr = HotkeyManager::new();
        for text in ["F6", "Ctrl+S", "Ctrl+Shift+K", "Alt+F12", "Ctrl+Alt+Shift+Cmd+Space", "Shift+7", "Up"] {
            assert!(mgr.set_playback_hotkey_from_string(text), "{text}");
            assert_eq!(mgr.format_hotkey_string(), text);
        }
    }

    #[test]
    fn test_restored_hotkey_actually_triggers_the_toggle() {
        let mut mgr = HotkeyManager::new();
        assert!(mgr.set_playback_hotkey_from_string("Ctrl+K"));
        assert!(mgr.on_press(RawKey::ModLeft(Modifier::Ctrl)).is_empty());
        let events = mgr.on_press(RawKey::Char('k'));
        assert_eq!(events, vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_unparseable_text_is_rejected_and_leaves_the_binding_alone() {
        let mut mgr = HotkeyManager::new();
        assert!(!mgr.set_playback_hotkey_from_string("Hyper+F6"));
        assert!(!mgr.set_playback_hotkey_from_string("Ctrl+NotAKey"));
        assert!(!mgr.set_playback_hotkey_from_string(""));
        assert_eq!(mgr.format_hotkey_string(), "F6");
    }

    #[test]
    fn test_restore_from_config_sets_both_hotkeys_and_ignores_bad_values() {
        let mut mgr = HotkeyManager::new();
        mgr.restore_from_config(&serde_json::json!({ "hotkey_playback": "F9", "hotkey_save": "Alt+W" }));
        assert_eq!(mgr.format_hotkey_string(), "F9");
        assert_eq!(mgr.format_save_hotkey_string(), "Alt+W");
        mgr.restore_from_config(&serde_json::json!({ "hotkey_playback": 5, "hotkey_save": "Bogus+W" }));
        assert_eq!(mgr.format_hotkey_string(), "F9");
        assert_eq!(mgr.format_save_hotkey_string(), "Alt+W");
    }

    #[test]
    fn test_restore_from_an_empty_config_keeps_the_defaults() {
        let mut mgr = HotkeyManager::new();
        mgr.restore_from_config(&serde_json::json!({}));
        assert_eq!(mgr.format_hotkey_string(), "F6");
        assert_eq!(mgr.format_save_hotkey_string(), "Ctrl+S");
    }
}

mod test_default_hotkey {
    use super::*;

    #[test]
    fn test_default_toggle_is_f6_no_modifiers() {
        let mgr = HotkeyManager::new();
        assert_eq!(mgr.format_hotkey_string(), "F6");
    }

    #[test]
    fn test_default_save_hotkey_is_ctrl_s() {
        let mgr = HotkeyManager::new();
        assert_eq!(mgr.format_save_hotkey_string(), "Ctrl+S");
    }

    #[test]
    fn test_plain_f6_press_emits_toggle_requested() {
        let mut mgr = HotkeyManager::new();
        let events = mgr.on_press(RawKey::Named("F6"));
        assert_eq!(events, vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_ctrl_s_emits_save_requested() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::ModLeft(Modifier::Ctrl));
        let events = mgr.on_press(RawKey::Char('s'));
        assert_eq!(events, vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_s_without_ctrl_does_not_save() {
        let mut mgr = HotkeyManager::new();
        let events = mgr.on_press(RawKey::Char('s'));
        assert!(events.is_empty());
    }

    #[test]
    fn test_f6_with_extra_modifier_does_not_toggle() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::ModLeft(Modifier::Shift));
        let events = mgr.on_press(RawKey::Named("F6"));
        assert!(events.is_empty());
    }

    #[test]
    fn test_release_clears_held_modifier() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::ModLeft(Modifier::Ctrl));
        assert!(mgr.held_modifiers().contains(&Modifier::Ctrl));
        mgr.on_release(RawKey::ModLeft(Modifier::Ctrl));
        assert!(!mgr.held_modifiers().contains(&Modifier::Ctrl));
    }

    #[test]
    fn test_release_of_unheld_modifier_is_a_no_op() {
        let mut mgr = HotkeyManager::new();
        mgr.on_release(RawKey::ModLeft(Modifier::Alt));
        assert!(mgr.held_modifiers().is_empty());
    }

    #[test]
    fn test_non_modifier_press_does_not_affect_held_mods() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::Char('x'));
        assert!(mgr.held_modifiers().is_empty());
    }
}

mod test_modifier_normalization {
    use super::*;

    #[test]
    fn test_left_and_right_ctrl_are_equivalent_for_matching() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::ModRight(Modifier::Ctrl));
        let events = mgr.on_press(RawKey::Char('s'));
        assert_eq!(events, vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_left_and_right_variants_share_held_state() {
        let mut mgr = HotkeyManager::new();
        mgr.on_press(RawKey::ModLeft(Modifier::Ctrl));
        mgr.on_release(RawKey::ModRight(Modifier::Ctrl));
        assert!(!mgr.held_modifiers().contains(&Modifier::Ctrl));
    }
}

mod test_rebinding {
    use super::*;

    #[test]
    fn test_start_binding_captures_next_non_modifier_key() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        assert!(mgr.is_listening_for_bind());

        let events = mgr.on_press(RawKey::ModLeft(Modifier::Alt));
        assert!(events.is_empty());
        assert!(mgr.is_listening_for_bind());

        let events = mgr.on_press(RawKey::Char('p'));
        assert!(!mgr.is_listening_for_bind());
        assert_eq!(events, vec![HotkeyEvent::BoundUpdated("Alt+P".to_string())]);
        assert_eq!(mgr.format_hotkey_string(), "Alt+P");
    }

    #[test]
    fn test_rebound_hotkey_no_longer_responds_to_old_combo() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        mgr.on_press(RawKey::ModLeft(Modifier::Alt));
        mgr.on_press(RawKey::Char('p'));

        mgr.on_release(RawKey::ModLeft(Modifier::Alt));
        let events = mgr.on_press(RawKey::Named("F6"));
        assert!(events.is_empty());

        mgr.on_press(RawKey::ModLeft(Modifier::Alt));
        let events = mgr.on_press(RawKey::Char('p'));
        assert_eq!(events, vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_start_save_binding_captures_next_non_modifier_key() {
        let mut mgr = HotkeyManager::new();
        mgr.start_save_binding();
        mgr.on_press(RawKey::ModLeft(Modifier::Shift));
        let events = mgr.on_press(RawKey::Char('k'));
        assert_eq!(
            events,
            vec![HotkeyEvent::BoundSaveUpdated("Shift+K".to_string())]
        );
        assert_eq!(mgr.format_save_hotkey_string(), "Shift+K");
    }

    #[test]
    fn test_binding_mode_does_not_also_check_toggle_or_save_match() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        let events = mgr.on_press(RawKey::Named("F6"));
        assert_eq!(
            events,
            vec![HotkeyEvent::BoundUpdated("F6".to_string())],
            "capturing the same key as the existing default must only bind, not also fire ToggleRequested"
        );
    }

    #[test]
    fn test_toggle_binding_mode_takes_priority_when_both_flags_set() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        mgr.start_save_binding();
        let events = mgr.on_press(RawKey::Char('z'));
        assert_eq!(
            events,
            vec![HotkeyEvent::BoundUpdated("Z".to_string())],
            "if/elif priority means toggle-bind mode (checked first) wins when both flags are set"
        );
        assert!(
            mgr.is_listening_for_save_bind(),
            "save-bind mode is left untouched by the elif branch, since it was never reached"
        );
    }
}

mod test_rdev_key_to_raw {
    use super::*;

    #[test]
    fn test_control_left_maps_to_mod_left_ctrl() {
        assert_eq!(
            rdev_key_to_raw(rdev::Key::ControlLeft),
            Some(RawKey::ModLeft(Modifier::Ctrl))
        );
    }

    #[test]
    fn test_control_right_maps_to_mod_right_ctrl() {
        assert_eq!(
            rdev_key_to_raw(rdev::Key::ControlRight),
            Some(RawKey::ModRight(Modifier::Ctrl))
        );
    }

    #[test]
    fn test_shift_variants_map_to_shift_modifier() {
        assert_eq!(
            rdev_key_to_raw(rdev::Key::ShiftLeft),
            Some(RawKey::ModLeft(Modifier::Shift))
        );
        assert_eq!(
            rdev_key_to_raw(rdev::Key::ShiftRight),
            Some(RawKey::ModRight(Modifier::Shift))
        );
    }

    #[test]
    fn test_alt_and_altgr_map_to_alt_modifier() {
        assert_eq!(
            rdev_key_to_raw(rdev::Key::Alt),
            Some(RawKey::ModLeft(Modifier::Alt))
        );
        assert_eq!(
            rdev_key_to_raw(rdev::Key::AltGr),
            Some(RawKey::ModRight(Modifier::Alt))
        );
    }

    #[test]
    fn test_meta_variants_map_to_cmd_modifier() {
        assert_eq!(
            rdev_key_to_raw(rdev::Key::MetaLeft),
            Some(RawKey::ModLeft(Modifier::Cmd))
        );
        assert_eq!(
            rdev_key_to_raw(rdev::Key::MetaRight),
            Some(RawKey::ModRight(Modifier::Cmd))
        );
    }

    #[test]
    fn test_f6_maps_to_named_f6_matching_default_toggle_binding() {
        assert_eq!(rdev_key_to_raw(rdev::Key::F6), Some(RawKey::Named("F6")));
    }

    #[test]
    fn test_key_s_maps_to_char_s_matching_default_save_binding() {
        assert_eq!(rdev_key_to_raw(rdev::Key::KeyS), Some(RawKey::Char('s')));
    }

    #[test]
    fn test_all_letters_map_to_lowercase_chars() {
        let pairs = [
            (rdev::Key::KeyA, 'a'),
            (rdev::Key::KeyM, 'm'),
            (rdev::Key::KeyZ, 'z'),
        ];
        for (key, expected) in pairs {
            assert_eq!(rdev_key_to_raw(key), Some(RawKey::Char(expected)));
        }
    }

    #[test]
    fn test_digits_map_to_char_digits() {
        assert_eq!(rdev_key_to_raw(rdev::Key::Num0), Some(RawKey::Char('0')));
        assert_eq!(rdev_key_to_raw(rdev::Key::Num9), Some(RawKey::Char('9')));
    }

    #[test]
    fn test_unmapped_key_returns_none() {
        assert_eq!(rdev_key_to_raw(rdev::Key::Unknown(0)), None);
    }

    #[test]
    fn test_mapped_key_round_trips_through_manager_default_toggle() {
        let mut mgr = HotkeyManager::new();
        let raw = rdev_key_to_raw(rdev::Key::F6).unwrap();
        assert_eq!(mgr.on_press(raw), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_mapped_ctrl_s_round_trips_through_manager_default_save() {
        let mut mgr = HotkeyManager::new();
        let ctrl = rdev_key_to_raw(rdev::Key::ControlLeft).unwrap();
        let s = rdev_key_to_raw(rdev::Key::KeyS).unwrap();
        mgr.on_press(ctrl);
        assert_eq!(mgr.on_press(s), vec![HotkeyEvent::SaveRequested]);
    }
}

mod test_dom_code_to_raw {
    use super::*;

    const FUNCTION_KEY_COUNT: u32 = 12;

    const SAME_KEY_AS_THE_GLOBAL_HOOK: &[(&str, rdev::Key)] = &[
        ("ControlLeft", rdev::Key::ControlLeft),
        ("ControlRight", rdev::Key::ControlRight),
        ("ShiftLeft", rdev::Key::ShiftLeft),
        ("ShiftRight", rdev::Key::ShiftRight),
        ("AltLeft", rdev::Key::Alt),
        ("AltRight", rdev::Key::AltGr),
        ("MetaLeft", rdev::Key::MetaLeft),
        ("MetaRight", rdev::Key::MetaRight),
        ("Space", rdev::Key::Space),
        ("Escape", rdev::Key::Escape),
        ("Tab", rdev::Key::Tab),
        ("Enter", rdev::Key::Return),
        ("Backspace", rdev::Key::Backspace),
        ("Delete", rdev::Key::Delete),
        ("ArrowUp", rdev::Key::UpArrow),
        ("ArrowDown", rdev::Key::DownArrow),
        ("ArrowLeft", rdev::Key::LeftArrow),
        ("ArrowRight", rdev::Key::RightArrow),
        ("F1", rdev::Key::F1),
        ("F2", rdev::Key::F2),
        ("F3", rdev::Key::F3),
        ("F4", rdev::Key::F4),
        ("F5", rdev::Key::F5),
        ("F6", rdev::Key::F6),
        ("F7", rdev::Key::F7),
        ("F8", rdev::Key::F8),
        ("F9", rdev::Key::F9),
        ("F10", rdev::Key::F10),
        ("F11", rdev::Key::F11),
        ("F12", rdev::Key::F12),
        ("KeyA", rdev::Key::KeyA),
        ("KeyB", rdev::Key::KeyB),
        ("KeyC", rdev::Key::KeyC),
        ("KeyD", rdev::Key::KeyD),
        ("KeyE", rdev::Key::KeyE),
        ("KeyF", rdev::Key::KeyF),
        ("KeyG", rdev::Key::KeyG),
        ("KeyH", rdev::Key::KeyH),
        ("KeyI", rdev::Key::KeyI),
        ("KeyJ", rdev::Key::KeyJ),
        ("KeyK", rdev::Key::KeyK),
        ("KeyL", rdev::Key::KeyL),
        ("KeyM", rdev::Key::KeyM),
        ("KeyN", rdev::Key::KeyN),
        ("KeyO", rdev::Key::KeyO),
        ("KeyP", rdev::Key::KeyP),
        ("KeyQ", rdev::Key::KeyQ),
        ("KeyR", rdev::Key::KeyR),
        ("KeyS", rdev::Key::KeyS),
        ("KeyT", rdev::Key::KeyT),
        ("KeyU", rdev::Key::KeyU),
        ("KeyV", rdev::Key::KeyV),
        ("KeyW", rdev::Key::KeyW),
        ("KeyX", rdev::Key::KeyX),
        ("KeyY", rdev::Key::KeyY),
        ("KeyZ", rdev::Key::KeyZ),
        ("Digit0", rdev::Key::Num0),
        ("Digit1", rdev::Key::Num1),
        ("Digit2", rdev::Key::Num2),
        ("Digit3", rdev::Key::Num3),
        ("Digit4", rdev::Key::Num4),
        ("Digit5", rdev::Key::Num5),
        ("Digit6", rdev::Key::Num6),
        ("Digit7", rdev::Key::Num7),
        ("Digit8", rdev::Key::Num8),
        ("Digit9", rdev::Key::Num9),
    ];

    const NOT_MAPPED: &[&str] = &[
        "",
        " ",
        "KeyA ",
        " KeyA",
        "keya",
        "KEYA",
        "Key",
        "KeyAB",
        "Keya",
        "Key1",
        "Key-",
        "Key\u{c0}",
        "Digit",
        "Digit10",
        "DigitA",
        "Digita",
        "Digit-",
        "Digit\u{663}",
        "F0",
        "F13",
        "NumpadEnter",
        "Numpad5",
        "Insert",
        "Home",
        "End",
        "PageUp",
        "CapsLock",
        "ContextMenu",
        "Unidentified",
        "garbage",
    ];

    #[test]
    fn test_every_code_maps_to_exactly_what_the_global_hook_maps_the_same_key_to() {
        for (code, key) in SAME_KEY_AS_THE_GLOBAL_HOOK {
            let mapped = dom_code_to_raw(code);
            assert!(mapped.is_some(), "{code} is not mapped");
            assert_eq!(mapped, rdev_key_to_raw(*key), "{code}");
        }
    }

    #[test]
    fn test_every_letter_code_maps_to_its_lowercase_char() {
        for letter in 'a'..='z' {
            let code = format!("Key{}", letter.to_ascii_uppercase());
            assert_eq!(dom_code_to_raw(&code), Some(RawKey::Char(letter)), "{code}");
        }
    }

    #[test]
    fn test_every_digit_code_maps_to_its_char() {
        for digit in '0'..='9' {
            let code = format!("Digit{digit}");
            assert_eq!(dom_code_to_raw(&code), Some(RawKey::Char(digit)), "{code}");
        }
    }

    #[test]
    fn test_every_function_key_code_maps_to_the_named_key_the_binding_text_uses() {
        for number in 1..=FUNCTION_KEY_COUNT {
            let code = format!("F{number}");
            let mapped = dom_code_to_raw(&code).unwrap();
            assert_eq!(format_combo(&HashSet::new(), mapped), code);
        }
    }

    #[test]
    fn test_codes_the_global_hook_does_not_map_are_not_mapped_either() {
        for code in NOT_MAPPED {
            assert_eq!(dom_code_to_raw(code), None, "{code:?}");
        }
    }

    #[test]
    fn test_pressing_g_while_listening_binds_g() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        let events = mgr.on_press(dom_code_to_raw("KeyG").unwrap());
        assert_eq!(events, vec![HotkeyEvent::BoundUpdated("G".to_string())]);
    }

    #[test]
    fn test_pressing_g_while_listening_for_save_binds_g_for_save() {
        let mut mgr = HotkeyManager::new();
        mgr.start_save_binding();
        let events = mgr.on_press(dom_code_to_raw("KeyG").unwrap());
        assert_eq!(events, vec![HotkeyEvent::BoundSaveUpdated("G".to_string())]);
    }

    #[test]
    fn test_modifier_codes_then_a_key_bind_the_combo() {
        let mut mgr = HotkeyManager::new();
        mgr.start_binding();
        assert!(mgr.on_press(dom_code_to_raw("ControlRight").unwrap()).is_empty());
        assert!(mgr.on_press(dom_code_to_raw("ShiftLeft").unwrap()).is_empty());
        let events = mgr.on_press(dom_code_to_raw("KeyK").unwrap());
        assert_eq!(events, vec![HotkeyEvent::BoundUpdated("Ctrl+Shift+K".to_string())]);
    }

    #[test]
    fn test_releasing_a_modifier_code_clears_it_from_the_held_set() {
        let mut mgr = HotkeyManager::new();
        let alt = dom_code_to_raw("AltRight").unwrap();
        mgr.on_press(alt);
        assert!(mgr.held_modifiers().contains(&Modifier::Alt));
        mgr.on_release(alt);
        assert!(mgr.held_modifiers().is_empty());
    }

    #[test]
    fn test_the_default_toggle_and_save_keys_work_from_their_codes() {
        let mut mgr = HotkeyManager::new();
        assert_eq!(mgr.on_press(dom_code_to_raw("F6").unwrap()), vec![HotkeyEvent::ToggleRequested]);
        mgr.on_press(dom_code_to_raw("ControlLeft").unwrap());
        assert_eq!(mgr.on_press(dom_code_to_raw("KeyS").unwrap()), vec![HotkeyEvent::SaveRequested]);
    }
}
