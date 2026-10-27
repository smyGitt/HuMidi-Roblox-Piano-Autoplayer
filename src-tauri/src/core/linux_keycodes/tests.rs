use super::*;

const KEY_1: u16 = 2;
const KEY_0: u16 = 11;
const KEY_Q: u16 = 16;
const KEY_P: u16 = 25;
const KEY_A: u16 = 30;
const KEY_L: u16 = 38;
const KEY_Z: u16 = 44;
const KEY_M: u16 = 50;
const KEY_SPACE: u16 = 57;
const KEY_LEFTCTRL: u16 = 29;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_LEFTALT: u16 = 56;

const EXPECTED: [(char, u16); 36] = [
    ('1', KEY_1),
    ('2', 3),
    ('3', 4),
    ('4', 5),
    ('5', 6),
    ('6', 7),
    ('7', 8),
    ('8', 9),
    ('9', 10),
    ('0', KEY_0),
    ('q', KEY_Q),
    ('w', 17),
    ('e', 18),
    ('r', 19),
    ('t', 20),
    ('y', 21),
    ('u', 22),
    ('i', 23),
    ('o', 24),
    ('p', KEY_P),
    ('a', KEY_A),
    ('s', 31),
    ('d', 32),
    ('f', 33),
    ('g', 34),
    ('h', 35),
    ('j', 36),
    ('k', 37),
    ('l', KEY_L),
    ('z', KEY_Z),
    ('x', 45),
    ('c', 46),
    ('v', 47),
    ('b', 48),
    ('n', 49),
    ('m', KEY_M),
];

mod test_linux_keycode {
    use super::*;

    #[test]
    fn test_every_letter_and_digit_maps_to_its_linux_key() {
        for (ch, code) in EXPECTED {
            assert_eq!(linux_keycode(Key::Char(ch)), Some(code), "{ch}");
        }
    }

    #[test]
    fn test_an_uppercase_letter_is_the_same_physical_key() {
        for (ch, code) in EXPECTED {
            assert_eq!(linux_keycode(Key::Char(ch.to_ascii_uppercase())), Some(code), "{ch}");
        }
    }

    #[test]
    fn test_space_and_the_modifiers_map_to_their_left_keys() {
        assert_eq!(linux_keycode(Key::Space), Some(KEY_SPACE));
        assert_eq!(linux_keycode(Key::Char(' ')), Some(KEY_SPACE));
        assert_eq!(linux_keycode(Key::Ctrl), Some(KEY_LEFTCTRL));
        assert_eq!(linux_keycode(Key::Shift), Some(KEY_LEFTSHIFT));
        assert_eq!(linux_keycode(Key::Alt), Some(KEY_LEFTALT));
    }

    #[test]
    fn test_a_character_no_piano_key_uses_has_no_key() {
        for ch in ['!', '-', '.', '\n', '\u{e9}', '\u{663}', '\0'] {
            assert_eq!(linux_keycode(Key::Char(ch)), None, "{ch:?}");
        }
    }
}

mod test_all_linux_keycodes {
    use super::*;

    #[test]
    fn test_it_lists_every_key_the_driver_can_send_once() {
        let mut expected: Vec<u16> = EXPECTED.iter().map(|&(_, code)| code).collect();
        expected.extend([KEY_SPACE, KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_LEFTALT]);
        expected.sort_unstable();
        let mut listed = all_linux_keycodes();
        listed.sort_unstable();
        assert_eq!(listed, expected);
    }
}
