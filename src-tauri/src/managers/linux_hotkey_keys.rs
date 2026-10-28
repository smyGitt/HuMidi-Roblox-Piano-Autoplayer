use crate::core::linux_keycodes::character_for_linux_code;
use crate::managers::hotkey_manager::{Modifier, RawKey};

const NAMED_KEYS: [(u16, RawKey); 30] = [
    (1, RawKey::Named("Escape")),
    (14, RawKey::Named("Backspace")),
    (15, RawKey::Named("Tab")),
    (28, RawKey::Named("Enter")),
    (29, RawKey::ModLeft(Modifier::Ctrl)),
    (42, RawKey::ModLeft(Modifier::Shift)),
    (54, RawKey::ModRight(Modifier::Shift)),
    (56, RawKey::ModLeft(Modifier::Alt)),
    (57, RawKey::Named("Space")),
    (59, RawKey::Named("F1")),
    (60, RawKey::Named("F2")),
    (61, RawKey::Named("F3")),
    (62, RawKey::Named("F4")),
    (63, RawKey::Named("F5")),
    (64, RawKey::Named("F6")),
    (65, RawKey::Named("F7")),
    (66, RawKey::Named("F8")),
    (67, RawKey::Named("F9")),
    (68, RawKey::Named("F10")),
    (87, RawKey::Named("F11")),
    (88, RawKey::Named("F12")),
    (97, RawKey::ModRight(Modifier::Ctrl)),
    (100, RawKey::ModRight(Modifier::Alt)),
    (103, RawKey::Named("Up")),
    (105, RawKey::Named("Left")),
    (106, RawKey::Named("Right")),
    (108, RawKey::Named("Down")),
    (111, RawKey::Named("Delete")),
    (125, RawKey::ModLeft(Modifier::Cmd)),
    (126, RawKey::ModRight(Modifier::Cmd)),
];

pub fn raw_key_from_linux_code(code: u16) -> Option<RawKey> {
    if let Some(character) = character_for_linux_code(code) {
        return Some(RawKey::Char(character));
    }
    NAMED_KEYS.iter().find(|&&(named, _)| named == code).map(|&(_, key)| key)
}

#[cfg(test)]
mod tests;
