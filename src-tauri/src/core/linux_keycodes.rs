use crate::core::keyboard_driver::Key;

const KEY_SPACE: u16 = 57;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_LEFTALT: u16 = 56;

const CHARACTER_KEYS: [(char, u16); 36] = [
    ('1', 2),
    ('2', 3),
    ('3', 4),
    ('4', 5),
    ('5', 6),
    ('6', 7),
    ('7', 8),
    ('8', 9),
    ('9', 10),
    ('0', 11),
    ('q', 16),
    ('w', 17),
    ('e', 18),
    ('r', 19),
    ('t', 20),
    ('y', 21),
    ('u', 22),
    ('i', 23),
    ('o', 24),
    ('p', 25),
    ('a', 30),
    ('s', 31),
    ('d', 32),
    ('f', 33),
    ('g', 34),
    ('h', 35),
    ('j', 36),
    ('k', 37),
    ('l', 38),
    ('z', 44),
    ('x', 45),
    ('c', 46),
    ('v', 47),
    ('b', 48),
    ('n', 49),
    ('m', 50),
];

const OTHER_KEYS: [u16; 4] = [KEY_SPACE, KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_LEFTALT];

fn character_keycode(ch: char) -> Option<u16> {
    if ch == ' ' {
        return Some(KEY_SPACE);
    }
    let lower = ch.to_ascii_lowercase();
    CHARACTER_KEYS
        .iter()
        .find(|&&(c, _)| c == lower)
        .map(|&(_, code)| code)
}

pub fn linux_keycode(key: Key) -> Option<u16> {
    match key {
        Key::Char(ch) => character_keycode(ch),
        Key::Space => Some(KEY_SPACE),
        Key::Ctrl => Some(KEY_LEFTCTRL),
        Key::Shift => Some(KEY_LEFTSHIFT),
        Key::Alt => Some(KEY_LEFTALT),
    }
}

pub fn character_for_linux_code(code: u16) -> Option<char> {
    CHARACTER_KEYS
        .iter()
        .find(|&&(_, key_code)| key_code == code)
        .map(|&(character, _)| character)
}

pub fn all_linux_keycodes() -> Vec<u16> {
    CHARACTER_KEYS
        .iter()
        .map(|&(_, code)| code)
        .chain(OTHER_KEYS)
        .collect()
}

#[cfg(test)]
mod tests;
