use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use evdev::{Device, EventType, KeyCode};

use crate::managers::hotkey_manager::RawKey;
use crate::managers::keyboard_hook::{is_keyboard, is_own_device, key_event_to_hook, ACTIVE_DEVICES};

pub const RESCAN_INTERVAL: Duration = Duration::from_secs(5);

pub type KeySink = Arc<dyn Fn(RawKey, bool) + Send + Sync>;

pub fn scan_once(on_key: &KeySink) -> usize {
    let mut started = 0;
    for (path, device) in evdev::enumerate() {
        if is_own_device(device.name().unwrap_or_default()) {
            continue;
        }
        let supported = device.supported_keys();
        let has_key = |code: u16| supported.is_some_and(|keys| keys.contains(KeyCode(code)));
        if !is_keyboard(has_key) {
            continue;
        }
        if !ACTIVE_DEVICES.claim(&path) {
            continue;
        }
        let on_key = Arc::clone(on_key);
        std::thread::spawn(move || read_device(path, device, on_key));
        started += 1;
    }
    started
}

fn read_device(path: PathBuf, mut device: Device, on_key: KeySink) {
    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    if event.event_type() != EventType::KEY {
                        continue;
                    }
                    if let Some((raw, pressed)) = key_event_to_hook(event.code(), event.value()) {
                        on_key(raw, pressed);
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    ACTIVE_DEVICES.release(&path);
}

pub fn spawn(on_key: KeySink) {
    std::thread::spawn(move || loop {
        scan_once(&on_key);
        std::thread::sleep(RESCAN_INTERVAL);
    });
}

#[cfg(test)]
mod tests;
