use super::*;
use crate::core::linux_keycodes::all_linux_keycodes;
use crate::managers::hotkey_manager::{dom_code_to_raw, Modifier};

const NAMED_AND_MODIFIER_KEYS: [(u16, &str); 32] = [
    (1, "Escape"),
    (14, "Backspace"),
    (15, "Tab"),
    (28, "Enter"),
    (29, "ControlLeft"),
    (42, "ShiftLeft"),
    (54, "ShiftRight"),
    (56, "AltLeft"),
    (57, "Space"),
    (59, "F1"),
    (60, "F2"),
    (61, "F3"),
    (62, "F4"),
    (63, "F5"),
    (64, "F6"),
    (65, "F7"),
    (66, "F8"),
    (67, "F9"),
    (68, "F10"),
    (87, "F11"),
    (88, "F12"),
    (97, "ControlRight"),
    (100, "AltRight"),
    (103, "ArrowUp"),
    (105, "ArrowLeft"),
    (106, "ArrowRight"),
    (108, "ArrowDown"),
    (111, "Delete"),
    (125, "MetaLeft"),
    (126, "MetaRight"),
    (30, "KeyA"),
    (2, "Digit1"),
];

#[test]
fn test_every_named_and_modifier_key_is_the_same_key_the_window_forwards() {
    for (code, dom_code) in NAMED_AND_MODIFIER_KEYS {
        assert_eq!(raw_key_from_linux_code(code), dom_code_to_raw(dom_code), "{code} {dom_code}");
        assert!(raw_key_from_linux_code(code).is_some(), "{code} {dom_code}");
    }
}

#[test]
fn test_the_default_hotkeys_map_to_what_the_settings_page_shows() {
    assert_eq!(raw_key_from_linux_code(64), Some(RawKey::Named("F6")));
    assert_eq!(raw_key_from_linux_code(29), Some(RawKey::ModLeft(Modifier::Ctrl)));
    assert_eq!(raw_key_from_linux_code(31), Some(RawKey::Char('s')));
}

#[test]
fn test_every_letter_and_digit_key_is_its_character() {
    for code in all_linux_keycodes() {
        let Some(RawKey::Char(ch)) = raw_key_from_linux_code(code) else {
            continue;
        };
        assert!(ch.is_ascii_lowercase() || ch.is_ascii_digit(), "{code} {ch}");
    }
    let characters = all_linux_keycodes()
        .into_iter()
        .filter(|&code| matches!(raw_key_from_linux_code(code), Some(RawKey::Char(_))))
        .count();
    assert_eq!(characters, 36);
}

#[test]
fn test_keys_the_hotkeys_do_not_use_are_ignored() {
    for code in [0, 58, 69, 99, 119, 200, u16::MAX] {
        assert_eq!(raw_key_from_linux_code(code), None, "{code}");
    }
}
