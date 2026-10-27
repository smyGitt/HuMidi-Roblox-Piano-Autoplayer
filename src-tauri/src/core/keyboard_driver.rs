use std::error::Error;
use std::fmt;

use enigo::{Direction, Enigo, Keyboard, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Space,
    Ctrl,
    Shift,
    Alt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        platform_from_os_str(std::env::consts::OS)
    }
}

fn platform_from_os_str(os: &str) -> Platform {
    match os {
        "windows" => Platform::Windows,
        "macos" => Platform::MacOs,
        "linux" => Platform::Linux,
        _ => Platform::Other,
    }
}

const MAC_DIGIT_VK: [(char, u32); 10] = [
    ('1', 0x12),
    ('2', 0x13),
    ('3', 0x14),
    ('4', 0x15),
    ('5', 0x17),
    ('6', 0x16),
    ('7', 0x1A),
    ('8', 0x1C),
    ('9', 0x19),
    ('0', 0x1D),
];

fn mac_digit_vk(ch: char) -> Option<u32> {
    MAC_DIGIT_VK
        .iter()
        .find(|&&(c, _)| c == ch)
        .map(|&(_, vk)| vk)
}

pub fn resolve_enigo_key(key: Key, platform: Platform) -> enigo::Key {
    match key {
        Key::Space => enigo::Key::Space,
        Key::Ctrl => enigo::Key::Control,
        Key::Shift => enigo::Key::Shift,
        Key::Alt => enigo::Key::Alt,
        Key::Char(c) => {
            if platform == Platform::MacOs {
                if let Some(vk) = mac_digit_vk(c) {
                    return enigo::Key::Other(vk);
                }
            }
            enigo::Key::Unicode(c)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyActionError(pub String);

impl fmt::Display for KeyActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for KeyActionError {}

pub type KeyActionResult = Result<(), KeyActionError>;

pub trait KeyboardDriver {
    fn press(&mut self, key: Key) -> KeyActionResult;
    fn release(&mut self, key: Key) -> KeyActionResult;
}

impl KeyboardDriver for Box<dyn KeyboardDriver + Send> {
    fn press(&mut self, key: Key) -> KeyActionResult {
        (**self).press(key)
    }

    fn release(&mut self, key: Key) -> KeyActionResult {
        (**self).release(key)
    }
}

pub struct PressedGuard<'a> {
    driver: &'a mut dyn KeyboardDriver,
    modifiers: Vec<Key>,
}

impl<'a> PressedGuard<'a> {
    pub fn press(&mut self, key: Key) -> KeyActionResult {
        self.driver.press(key)
    }

    pub fn release(&mut self, key: Key) -> KeyActionResult {
        self.driver.release(key)
    }
}

impl<'a> Drop for PressedGuard<'a> {
    fn drop(&mut self) {
        for &m in self.modifiers.iter().rev() {
            let _ = self.driver.release(m);
        }
    }
}

pub fn pressed<'a>(driver: &'a mut dyn KeyboardDriver, modifiers: &[Key]) -> PressedGuard<'a> {
    for &m in modifiers {
        let _ = driver.press(m);
    }
    PressedGuard {
        driver,
        modifiers: modifiers.to_vec(),
    }
}

#[derive(Debug)]
pub enum BuildDriverError {
    MacosAccessibilityDenied,
    Enigo(enigo::NewConError),
}

impl fmt::Display for BuildDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildDriverError::MacosAccessibilityDenied => write!(
                f,
                "macOS Accessibility permission is required for key injection.\n\
                 Open System Settings > Privacy & Security > Accessibility, \
                 enable this application, then restart HuMidi."
            ),
            BuildDriverError::Enigo(e) => write!(f, "{e}"),
        }
    }
}

impl Error for BuildDriverError {}

fn check_platform_permission(
    platform: Platform,
    mut permission_check: impl FnMut() -> bool,
) -> Result<(), BuildDriverError> {
    if platform == Platform::MacOs && !permission_check() {
        return Err(BuildDriverError::MacosAccessibilityDenied);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn check_macos_accessibility() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    unsafe { AXIsProcessTrusted() }
}

#[cfg(not(target_os = "macos"))]
fn check_macos_accessibility() -> bool {
    true
}

pub struct EnigoDriver {
    enigo: Enigo,
    platform: Platform,
}

impl EnigoDriver {
    pub fn new() -> Result<Self, BuildDriverError> {
        let platform = Platform::current();
        check_platform_permission(platform, check_macos_accessibility)?;
        let settings = Settings {
            open_prompt_to_get_permissions: false,
            ..Settings::default()
        };
        let enigo = Enigo::new(&settings).map_err(BuildDriverError::Enigo)?;
        Ok(EnigoDriver { enigo, platform })
    }
}

impl KeyboardDriver for EnigoDriver {
    fn press(&mut self, key: Key) -> KeyActionResult {
        let resolved = resolve_enigo_key(key, self.platform);
        self.enigo
            .key(resolved, Direction::Press)
            .map_err(|e| KeyActionError(e.to_string()))
    }

    fn release(&mut self, key: Key) -> KeyActionResult {
        let resolved = resolve_enigo_key(key, self.platform);
        self.enigo
            .key(resolved, Direction::Release)
            .map_err(|e| KeyActionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests;
