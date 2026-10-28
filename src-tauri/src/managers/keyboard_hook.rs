use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::core::keyboard_driver::Platform;
use crate::core::uinput_driver::VIRTUAL_KEYBOARD_NAME;
use crate::managers::hotkey_manager::RawKey;
use crate::managers::linux_hotkey_keys::raw_key_from_linux_code;

pub const KEYBOARD_HOTKEYS_CONFIG_KEY: &str = "linux_keyboard_hotkeys";

const KEY_ENTER: u16 = 28;
const KEY_A: u16 = 30;
const KEY_RELEASED: i32 = 0;
const KEY_PRESSED: i32 = 1;

pub static ACTIVE_DEVICES: ActiveDevices = ActiveDevices::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookSource {
    Rdev,
    KeyboardDevices,
}

pub fn keyboard_hotkeys_enabled(config: &Value) -> bool {
    config.get(KEYBOARD_HOTKEYS_CONFIG_KEY) != Some(&Value::Bool(false))
}

pub fn active_hook(platform: Platform, config: &Value, device_count: usize) -> HookSource {
    if platform == Platform::Linux && keyboard_hotkeys_enabled(config) && device_count > 0 {
        HookSource::KeyboardDevices
    } else {
        HookSource::Rdev
    }
}

pub fn key_event_to_hook(code: u16, value: i32) -> Option<(RawKey, bool)> {
    let pressed = match value {
        KEY_RELEASED => false,
        KEY_PRESSED => true,
        _ => return None,
    };
    raw_key_from_linux_code(code).map(|raw| (raw, pressed))
}

pub fn is_own_device(name: &str) -> bool {
    name == VIRTUAL_KEYBOARD_NAME
}

pub fn is_keyboard(has_key: impl Fn(u16) -> bool) -> bool {
    has_key(KEY_A) && has_key(KEY_ENTER)
}

pub struct ActiveDevices {
    paths: Mutex<BTreeSet<PathBuf>>,
}

impl ActiveDevices {
    pub const fn new() -> Self {
        ActiveDevices {
            paths: Mutex::new(BTreeSet::new()),
        }
    }

    pub fn claim(&self, path: &Path) -> bool {
        self.paths
            .lock()
            .map(|mut paths| paths.insert(path.to_path_buf()))
            .unwrap_or(false)
    }

    pub fn release(&self, path: &Path) {
        if let Ok(mut paths) = self.paths.lock() {
            paths.remove(path);
        }
    }

    pub fn count(&self) -> usize {
        self.paths.lock().map(|paths| paths.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
