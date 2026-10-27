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

const NAMED_KEYS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12", "Space", "Escape",
    "Tab", "Enter", "Backspace", "Delete", "Up", "Down", "Left", "Right",
];

pub const PLAYBACK_HOTKEY_CONFIG_KEY: &str = "hotkey_playback";
pub const SAVE_HOTKEY_CONFIG_KEY: &str = "hotkey_save";

fn parse_combo(text: &str) -> Option<(HashSet<Modifier>, RawKey)> {
    let mut parts: Vec<&str> = text.split('+').collect();
    let key_text = parts.pop()?;
    let mut mods = HashSet::new();
    for part in parts {
        mods.insert(match part {
            "Ctrl" => Modifier::Ctrl,
            "Alt" => Modifier::Alt,
            "Shift" => Modifier::Shift,
            "Cmd" => Modifier::Cmd,
            _ => return None,
        });
    }
    if let Some(name) = NAMED_KEYS.iter().find(|name| **name == key_text) {
        return Some((mods, RawKey::Named(name)));
    }
    let mut chars = key_text.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some((mods, RawKey::Char(c.to_ascii_lowercase())))
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

    pub fn set_playback_hotkey_from_string(&mut self, text: &str) -> bool {
        match parse_combo(text) {
            Some((mods, key)) => {
                self.current_mods = mods;
                self.current_key = key;
                true
            }
            None => false,
        }
    }

    pub fn set_save_hotkey_from_string(&mut self, text: &str) -> bool {
        match parse_combo(text) {
            Some((mods, key)) => {
                self.save_mods = mods;
                self.save_key = key;
                true
            }
            None => false,
        }
    }

    pub fn restore_from_config(&mut self, config: &serde_json::Value) {
        if let Some(text) = config.get(PLAYBACK_HOTKEY_CONFIG_KEY).and_then(|v| v.as_str()) {
            self.set_playback_hotkey_from_string(text);
        }
        if let Some(text) = config.get(SAVE_HOTKEY_CONFIG_KEY).and_then(|v| v.as_str()) {
            self.set_save_hotkey_from_string(text);
        }
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

const DOM_LETTER_PREFIX: &str = "Key";
const DOM_DIGIT_PREFIX: &str = "Digit";

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

pub fn dom_code_to_raw(code: &str) -> Option<RawKey> {
    Some(match code {
        "ControlLeft" => RawKey::ModLeft(Modifier::Ctrl),
        "ControlRight" => RawKey::ModRight(Modifier::Ctrl),
        "ShiftLeft" => RawKey::ModLeft(Modifier::Shift),
        "ShiftRight" => RawKey::ModRight(Modifier::Shift),
        "AltLeft" => RawKey::ModLeft(Modifier::Alt),
        "AltRight" => RawKey::ModRight(Modifier::Alt),
        "MetaLeft" => RawKey::ModLeft(Modifier::Cmd),
        "MetaRight" => RawKey::ModRight(Modifier::Cmd),
        "F1" => RawKey::Named("F1"),
        "F2" => RawKey::Named("F2"),
        "F3" => RawKey::Named("F3"),
        "F4" => RawKey::Named("F4"),
        "F5" => RawKey::Named("F5"),
        "F6" => RawKey::Named("F6"),
        "F7" => RawKey::Named("F7"),
        "F8" => RawKey::Named("F8"),
        "F9" => RawKey::Named("F9"),
        "F10" => RawKey::Named("F10"),
        "F11" => RawKey::Named("F11"),
        "F12" => RawKey::Named("F12"),
        "Space" => RawKey::Named("Space"),
        "Escape" => RawKey::Named("Escape"),
        "Tab" => RawKey::Named("Tab"),
        "Enter" => RawKey::Named("Enter"),
        "Backspace" => RawKey::Named("Backspace"),
        "Delete" => RawKey::Named("Delete"),
        "ArrowUp" => RawKey::Named("Up"),
        "ArrowDown" => RawKey::Named("Down"),
        "ArrowLeft" => RawKey::Named("Left"),
        "ArrowRight" => RawKey::Named("Right"),
        _ => return dom_character_code_to_raw(code),
    })
}

fn dom_character_code_to_raw(code: &str) -> Option<RawKey> {
    if let Some(rest) = code.strip_prefix(DOM_LETTER_PREFIX) {
        return single_char(rest)
            .filter(|c| c.is_ascii_uppercase())
            .map(|c| RawKey::Char(c.to_ascii_lowercase()));
    }
    if let Some(rest) = code.strip_prefix(DOM_DIGIT_PREFIX) {
        return single_char(rest)
            .filter(|c| c.is_ascii_digit())
            .map(RawKey::Char);
    }
    None
}

#[cfg(test)]
mod tests;
