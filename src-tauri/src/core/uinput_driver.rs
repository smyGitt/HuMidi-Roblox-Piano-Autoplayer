use std::io;
use std::sync::{Arc, Mutex};

use crate::core::keyboard_driver::{Key, KeyActionError, KeyActionResult, KeyboardDriver, Platform};
use crate::core::linux_keycodes::linux_keycode;

pub trait KeySink: Send {
    fn send(&mut self, code: u16, pressed: bool) -> io::Result<()>;
}

#[derive(Clone)]
pub struct SharedSink(Arc<Mutex<Box<dyn KeySink>>>);

impl SharedSink {
    pub fn new(sink: impl KeySink + 'static) -> Self {
        SharedSink(Arc::new(Mutex::new(Box::new(sink))))
    }

    fn send(&self, code: u16, pressed: bool) -> KeyActionResult {
        let mut sink = self
            .0
            .lock()
            .map_err(|e| KeyActionError(e.to_string()))?;
        sink.send(code, pressed).map_err(|e| KeyActionError(e.to_string()))
    }
}

pub struct UinputDriver {
    sink: SharedSink,
}

impl UinputDriver {
    pub fn new(sink: SharedSink) -> Self {
        UinputDriver { sink }
    }

    fn send(&self, key: Key, pressed: bool) -> KeyActionResult {
        let code = linux_keycode(key)
            .ok_or_else(|| KeyActionError(format!("No Linux key code for {key:?}")))?;
        self.sink.send(code, pressed)
    }
}

impl KeyboardDriver for UinputDriver {
    fn press(&mut self, key: Key) -> KeyActionResult {
        self.send(key, true)
    }

    fn release(&mut self, key: Key) -> KeyActionResult {
        self.send(key, false)
    }
}

pub fn shared_sink_or_open<S: KeySink + 'static>(
    slot: &Mutex<Option<SharedSink>>,
    open: impl FnOnce() -> io::Result<S>,
    settle: impl FnOnce(),
) -> Option<SharedSink> {
    let mut slot = slot.lock().ok()?;
    if let Some(shared) = slot.as_ref() {
        return Some(shared.clone());
    }
    let shared = SharedSink::new(open().ok()?);
    settle();
    *slot = Some(shared.clone());
    Some(shared)
}

pub fn driver_failure_message(platform: Platform, detail: &str) -> String {
    if platform != Platform::Linux {
        return detail.to_string();
    }
    format!(
        "HuMidi cannot send key presses because it has no access to /dev/uinput.          Run these two commands in a terminal, then press Play again:          sudo modprobe uinput && sudo chmod 0666 /dev/uinput. Details: {detail}"
    )
}

pub const VIRTUAL_KEYBOARD_NAME: &str = "HuMidi virtual keyboard";

const SESSION_TYPE_VARIABLE: &str = "XDG_SESSION_TYPE";
const DISPLAY_VARIABLE: &str = "WAYLAND_DISPLAY";
const WAYLAND_SESSION: &str = "wayland";
const WAYLAND_DETAIL: &str = "Wayland does not let an app type into other windows any other way.";

pub fn is_wayland_session(variable: impl Fn(&str) -> Option<String>) -> bool {
    match variable(SESSION_TYPE_VARIABLE).filter(|kind| !kind.is_empty()) {
        Some(kind) => kind.eq_ignore_ascii_case(WAYLAND_SESSION),
        None => variable(DISPLAY_VARIABLE).is_some_and(|display| !display.is_empty()),
    }
}

pub fn driver_or_explained_fallback<F, E>(
    platform: Platform,
    wayland: bool,
    sink: Option<SharedSink>,
    fallback: impl FnOnce() -> Result<F, E>,
) -> Result<Box<dyn KeyboardDriver + Send>, String>
where
    F: KeyboardDriver + Send + 'static,
    E: std::fmt::Display,
{
    if sink.is_none() && wayland && platform == Platform::Linux {
        return Err(driver_failure_message(platform, WAYLAND_DETAIL));
    }
    driver_or_fallback(sink, fallback).map_err(|e| driver_failure_message(platform, &e.to_string()))
}

pub fn driver_or_fallback<F, E>(
    sink: Option<SharedSink>,
    fallback: impl FnOnce() -> Result<F, E>,
) -> Result<Box<dyn KeyboardDriver + Send>, E>
where
    F: KeyboardDriver + Send + 'static,
{
    match sink {
        Some(shared) => Ok(Box::new(UinputDriver::new(shared))),
        None => fallback().map(|driver| Box::new(driver) as Box<dyn KeyboardDriver + Send>),
    }
}

#[cfg(test)]
mod tests;
