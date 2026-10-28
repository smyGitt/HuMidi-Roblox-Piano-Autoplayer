use super::*;
use crate::managers::hotkey_manager::Modifier;
use serde_json::json;
use std::path::PathBuf;

const KEY_A: u16 = 30;
const KEY_S: u16 = 31;
const KEY_ENTER: u16 = 28;
const KEY_F6: u16 = 64;
const KEY_LEFTCTRL: u16 = 29;
const UNKNOWN_KEY: u16 = 999;
const RELEASED: i32 = 0;
const PRESSED: i32 = 1;
const REPEATED: i32 = 2;
const NOT_A_KEY_STATE: i32 = 3;

fn device(name: &str) -> PathBuf {
    PathBuf::from(format!("/dev/input/{name}"))
}

mod test_keyboard_hotkeys_enabled {
    use super::*;

    #[test]
    fn test_it_is_on_unless_the_setting_is_off() {
        assert!(keyboard_hotkeys_enabled(&json!({})));
        assert!(keyboard_hotkeys_enabled(&json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: true })));
        assert!(!keyboard_hotkeys_enabled(&json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: false })));
    }

    #[test]
    fn test_a_setting_of_the_wrong_type_does_not_turn_it_off() {
        assert!(keyboard_hotkeys_enabled(&json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: "no" })));
        assert!(keyboard_hotkeys_enabled(&json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: 0 })));
        assert!(keyboard_hotkeys_enabled(&json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: null })));
    }

    #[test]
    fn test_other_settings_do_not_matter() {
        assert!(keyboard_hotkeys_enabled(&json!({ "opacity": 50 })));
    }

    #[test]
    fn test_the_config_key_is_the_name_the_settings_page_saves() {
        assert_eq!(KEYBOARD_HOTKEYS_CONFIG_KEY, "linux_keyboard_hotkeys");
    }
}

mod test_active_hook {
    use super::*;

    #[test]
    fn test_on_linux_with_the_setting_on_and_a_keyboard_open_the_devices_are_used() {
        assert_eq!(active_hook(Platform::Linux, &json!({}), 1), HookSource::KeyboardDevices);
        assert_eq!(active_hook(Platform::Linux, &json!({}), 4), HookSource::KeyboardDevices);
    }

    #[test]
    fn test_with_no_keyboard_open_the_old_hook_is_used() {
        assert_eq!(active_hook(Platform::Linux, &json!({}), 0), HookSource::Rdev);
    }

    #[test]
    fn test_with_the_setting_off_the_old_hook_is_used_even_with_keyboards_open() {
        let config = json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: false });
        assert_eq!(active_hook(Platform::Linux, &config, 3), HookSource::Rdev);
    }

    #[test]
    fn test_other_platforms_always_use_the_old_hook() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Other] {
            assert_eq!(active_hook(platform, &json!({}), 3), HookSource::Rdev, "{platform:?}");
        }
    }
}

mod test_key_event_to_hook {
    use super::*;

    #[test]
    fn test_a_press_and_a_release_of_a_letter() {
        assert_eq!(key_event_to_hook(KEY_A, PRESSED), Some((RawKey::Char('a'), true)));
        assert_eq!(key_event_to_hook(KEY_A, RELEASED), Some((RawKey::Char('a'), false)));
    }

    #[test]
    fn test_the_default_hotkey_and_a_modifier() {
        assert_eq!(key_event_to_hook(KEY_F6, PRESSED), Some((RawKey::Named("F6"), true)));
        assert_eq!(key_event_to_hook(KEY_LEFTCTRL, PRESSED), Some((RawKey::ModLeft(Modifier::Ctrl), true)));
        assert_eq!(key_event_to_hook(KEY_S, PRESSED), Some((RawKey::Char('s'), true)));
    }

    #[test]
    fn test_a_held_key_repeating_is_ignored() {
        assert_eq!(key_event_to_hook(KEY_A, REPEATED), None);
        assert_eq!(key_event_to_hook(KEY_F6, REPEATED), None);
    }

    #[test]
    fn test_a_key_the_hotkeys_do_not_use_is_ignored() {
        assert_eq!(key_event_to_hook(UNKNOWN_KEY, PRESSED), None);
        assert_eq!(key_event_to_hook(UNKNOWN_KEY, RELEASED), None);
    }

    #[test]
    fn test_a_value_that_is_not_a_key_state_is_ignored() {
        assert_eq!(key_event_to_hook(KEY_A, NOT_A_KEY_STATE), None);
        assert_eq!(key_event_to_hook(KEY_A, -1), None);
    }
}

mod test_is_own_device {
    use super::*;

    #[test]
    fn test_only_the_exact_name_of_the_virtual_keyboard_counts() {
        assert!(is_own_device("HuMidi virtual keyboard"));
        assert!(!is_own_device("HuMidi virtual keyboard 2"));
        assert!(!is_own_device("HuMidi hook test keyboard"));
        assert!(!is_own_device("AT Translated Set 2 keyboard"));
        assert!(!is_own_device(""));
    }
}

mod test_is_keyboard {
    use super::*;

    #[test]
    fn test_a_device_with_letters_and_enter_is_a_keyboard() {
        assert!(is_keyboard(|code| code == KEY_A || code == KEY_ENTER));
    }

    #[test]
    fn test_a_device_missing_either_is_not() {
        assert!(!is_keyboard(|code| code == KEY_A));
        assert!(!is_keyboard(|code| code == KEY_ENTER));
        assert!(!is_keyboard(|_| false));
    }
}

mod test_active_devices {
    use super::*;

    #[test]
    fn test_a_new_device_can_be_claimed_once() {
        let devices = ActiveDevices::new();
        assert!(devices.claim(&device("event3")));
        assert!(!devices.claim(&device("event3")));
        assert_eq!(devices.count(), 1);
    }

    #[test]
    fn test_devices_are_counted_separately() {
        let devices = ActiveDevices::new();
        assert!(devices.claim(&device("event3")));
        assert!(devices.claim(&device("event4")));
        assert_eq!(devices.count(), 2);
    }

    #[test]
    fn test_a_released_device_can_be_claimed_again() {
        let devices = ActiveDevices::new();
        devices.claim(&device("event3"));
        devices.release(&device("event3"));
        assert_eq!(devices.count(), 0);
        assert!(devices.claim(&device("event3")));
    }

    #[test]
    fn test_releasing_a_device_that_was_never_claimed_changes_nothing() {
        let devices = ActiveDevices::new();
        devices.claim(&device("event3"));
        devices.release(&device("event9"));
        assert_eq!(devices.count(), 1);
    }

    #[test]
    fn test_none_are_active_to_begin_with() {
        assert_eq!(ActiveDevices::new().count(), 0);
    }

    #[test]
    fn test_a_poisoned_lock_neither_claims_nor_panics() {
        let devices = ActiveDevices::new();
        std::thread::scope(|scope| {
            let _ = scope
                .spawn(|| {
                    let _guard = devices.paths.lock().unwrap();
                    panic!("poison the lock");
                })
                .join();
        });
        assert!(!devices.claim(&device("event3")));
        devices.release(&device("event3"));
        assert_eq!(devices.count(), 0);
    }
}
