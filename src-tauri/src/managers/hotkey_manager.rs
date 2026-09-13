use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Shift,
    Ctrl,
    Alt,
    Cmd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RawKey {
    ModLeft(Modifier),
    ModRight(Modifier),
    Mod(Modifier),
    Char(char),
    Named(&'static str),
}

pub fn normalize(key: RawKey) -> RawKey {
    match key {
        RawKey::ModLeft(m) | RawKey::ModRight(m) => RawKey::Mod(m),
        other => other,
    }
}

fn as_canonical_modifier(key: RawKey) -> Option<Modifier> {
    match key {
        RawKey::Mod(m) => Some(m),
        _ => None,
    }
}

fn format_combo(mods: &HashSet<Modifier>, k: RawKey) -> String {
    let mut parts = Vec::new();
    for (m, label) in [
        (Modifier::Ctrl, "Ctrl"),
        (Modifier::Alt, "Alt"),
        (Modifier::Shift, "Shift"),
        (Modifier::Cmd, "Cmd"),
    ] {
        if mods.contains(&m) {
            parts.push(label.to_string());
        }
    }
    match k {
        RawKey::Char(c) => parts.push(c.to_uppercase().to_string()),
        RawKey::Named(name) => parts.push(name.to_string()),
        RawKey::Mod(m) | RawKey::ModLeft(m) | RawKey::ModRight(m) => {
            parts.push(format!("{m:?}").to_uppercase())
        }
    }
    parts.join("+")
}

#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyEvent {
    BoundUpdated(String),
    BoundSaveUpdated(String),
    ToggleRequested,
    SaveRequested,
}

pub struct HotkeyManager {
    current_mods: HashSet<Modifier>,
    current_key: RawKey,
    save_mods: HashSet<Modifier>,
    save_key: RawKey,
    held_mods: HashSet<Modifier>,
    listening_for_bind: bool,
    listening_for_save_bind: bool,
}

impl HotkeyManager {
    pub fn new() -> Self {
        HotkeyManager {
            current_mods: HashSet::new(),
            current_key: RawKey::Named("F6"),
            save_mods: HashSet::from([Modifier::Ctrl]),
            save_key: RawKey::Char('s'),
            held_mods: HashSet::new(),
            listening_for_bind: false,
            listening_for_save_bind: false,
        }
    }

    pub fn format_hotkey_string(&self) -> String {
        format_combo(&self.current_mods, self.current_key)
    }

    pub fn format_save_hotkey_string(&self) -> String {
        format_combo(&self.save_mods, self.save_key)
    }

    pub fn is_listening_for_bind(&self) -> bool {
        self.listening_for_bind
    }

    pub fn is_listening_for_save_bind(&self) -> bool {
        self.listening_for_save_bind
    }

    pub fn held_modifiers(&self) -> &HashSet<Modifier> {
        &self.held_mods
    }

    pub fn on_release(&mut self, key: RawKey) {
        let canon = normalize(key);
        if let Some(m) = as_canonical_modifier(canon) {
            self.held_mods.remove(&m);
        }
    }

    pub fn on_press(&mut self, key: RawKey) -> Vec<HotkeyEvent> {
        let canon = normalize(key);
        let mut events = Vec::new();

        if let Some(m) = as_canonical_modifier(canon) {
            self.held_mods.insert(m);
        }

        if self.listening_for_bind {
            if as_canonical_modifier(canon).is_some() {
                return events;
            }
            self.current_mods = self.held_mods.clone();
            self.current_key = key;
            self.listening_for_bind = false;
            events.push(HotkeyEvent::BoundUpdated(self.format_hotkey_string()));
        } else if self.listening_for_save_bind {
            if as_canonical_modifier(canon).is_some() {
                return events;
            }
            self.save_mods = self.held_mods.clone();
            self.save_key = key;
            self.listening_for_save_bind = false;
            events.push(HotkeyEvent::BoundSaveUpdated(self.format_save_hotkey_string()));
        } else {
            if normalize(key) == normalize(self.current_key) && self.held_mods == self.current_mods {
                events.push(HotkeyEvent::ToggleRequested);
            }
            if normalize(key) == normalize(self.save_key) && self.held_mods == self.save_mods {
                events.push(HotkeyEvent::SaveRequested);
            }
        }

        events
    }

    pub fn start_binding(&mut self) {
        self.listening_for_bind = true;
    }

    pub fn start_save_binding(&mut self) {
        self.listening_for_save_bind = true;
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn rdev_key_to_raw(key: rdev::Key) -> Option<RawKey> {
    use rdev::Key as RK;
    Some(match key {
        RK::ControlLeft => RawKey::ModLeft(Modifier::Ctrl),
        RK::ControlRight => RawKey::ModRight(Modifier::Ctrl),
        RK::ShiftLeft => RawKey::ModLeft(Modifier::Shift),
        RK::ShiftRight => RawKey::ModRight(Modifier::Shift),
        RK::Alt => RawKey::ModLeft(Modifier::Alt),
        RK::AltGr => RawKey::ModRight(Modifier::Alt),
        RK::MetaLeft => RawKey::ModLeft(Modifier::Cmd),
        RK::MetaRight => RawKey::ModRight(Modifier::Cmd),
        RK::F1 => RawKey::Named("F1"),
        RK::F2 => RawKey::Named("F2"),
        RK::F3 => RawKey::Named("F3"),
        RK::F4 => RawKey::Named("F4"),
        RK::F5 => RawKey::Named("F5"),
        RK::F6 => RawKey::Named("F6"),
        RK::F7 => RawKey::Named("F7"),
        RK::F8 => RawKey::Named("F8"),
        RK::F9 => RawKey::Named("F9"),
        RK::F10 => RawKey::Named("F10"),
        RK::F11 => RawKey::Named("F11"),
        RK::F12 => RawKey::Named("F12"),
        RK::Space => RawKey::Named("Space"),
        RK::Escape => RawKey::Named("Escape"),
        RK::Tab => RawKey::Named("Tab"),
        RK::Return => RawKey::Named("Enter"),
        RK::Backspace => RawKey::Named("Backspace"),
        RK::Delete => RawKey::Named("Delete"),
        RK::UpArrow => RawKey::Named("Up"),
        RK::DownArrow => RawKey::Named("Down"),
        RK::LeftArrow => RawKey::Named("Left"),
        RK::RightArrow => RawKey::Named("Right"),
        RK::KeyA => RawKey::Char('a'),
        RK::KeyB => RawKey::Char('b'),
        RK::KeyC => RawKey::Char('c'),
        RK::KeyD => RawKey::Char('d'),
        RK::KeyE => RawKey::Char('e'),
        RK::KeyF => RawKey::Char('f'),
        RK::KeyG => RawKey::Char('g'),
        RK::KeyH => RawKey::Char('h'),
        RK::KeyI => RawKey::Char('i'),
        RK::KeyJ => RawKey::Char('j'),
        RK::KeyK => RawKey::Char('k'),
        RK::KeyL => RawKey::Char('l'),
        RK::KeyM => RawKey::Char('m'),
        RK::KeyN => RawKey::Char('n'),
        RK::KeyO => RawKey::Char('o'),
        RK::KeyP => RawKey::Char('p'),
        RK::KeyQ => RawKey::Char('q'),
        RK::KeyR => RawKey::Char('r'),
        RK::KeyS => RawKey::Char('s'),
        RK::KeyT => RawKey::Char('t'),
        RK::KeyU => RawKey::Char('u'),
        RK::KeyV => RawKey::Char('v'),
        RK::KeyW => RawKey::Char('w'),
        RK::KeyX => RawKey::Char('x'),
        RK::KeyY => RawKey::Char('y'),
        RK::KeyZ => RawKey::Char('z'),
        RK::Num0 => RawKey::Char('0'),
        RK::Num1 => RawKey::Char('1'),
        RK::Num2 => RawKey::Char('2'),
        RK::Num3 => RawKey::Char('3'),
        RK::Num4 => RawKey::Char('4'),
        RK::Num5 => RawKey::Char('5'),
        RK::Num6 => RawKey::Char('6'),
        RK::Num7 => RawKey::Char('7'),
        RK::Num8 => RawKey::Char('8'),
        RK::Num9 => RawKey::Char('9'),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
