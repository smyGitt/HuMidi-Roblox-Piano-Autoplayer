use std::io;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

use crate::core::linux_keycodes::all_linux_keycodes;
use crate::core::uinput_driver::KeySink;

pub const DEVICE_NAME: &str = crate::core::uinput_driver::VIRTUAL_KEYBOARD_NAME;

const KEY_DOWN: i32 = 1;
const KEY_UP: i32 = 0;

pub struct UinputDevice(VirtualDevice);

pub fn open() -> io::Result<UinputDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in all_linux_keycodes() {
        keys.insert(KeyCode(code));
    }
    let device = VirtualDevice::builder()?
        .name(DEVICE_NAME)
        .with_keys(&keys)?
        .build()?;
    Ok(UinputDevice(device))
}

impl KeySink for UinputDevice {
    fn send(&mut self, code: u16, pressed: bool) -> io::Result<()> {
        let value = if pressed { KEY_DOWN } else { KEY_UP };
        self.0.emit(&[InputEvent::new(EventType::KEY.0, code, value)])
    }
}

#[cfg(test)]
mod tests;
