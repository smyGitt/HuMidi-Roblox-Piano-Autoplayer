use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::core::keyboard_driver::{Key, KeyActionResult, KeyboardDriver};
use crate::managers::hotkey_manager::{normalize, Modifier, RawKey};

const ECHO_MAX_AGE: Duration = Duration::from_millis(250);

struct Echo {
    key: RawKey,
    is_press: bool,
    at: Instant,
}

#[derive(Default)]
pub struct InjectedEchoes {
    pending: Mutex<Vec<Echo>>,
}

pub fn raw_key_for(key: Key) -> RawKey {
    match key {
        Key::Char(c) => RawKey::Char(c.to_ascii_lowercase()),
        Key::Space => RawKey::Named("Space"),
        Key::Ctrl => RawKey::Mod(Modifier::Ctrl),
        Key::Shift => RawKey::Mod(Modifier::Shift),
        Key::Alt => RawKey::Mod(Modifier::Alt),
    }
}

fn is_fresh(echo: &Echo, now: Instant) -> bool {
    now.checked_duration_since(echo.at)
        .map_or(true, |age| age <= ECHO_MAX_AGE)
}

impl InjectedEchoes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn expect(&self, key: Key, is_press: bool) {
        self.expect_at(key, is_press, Instant::now());
    }

    pub fn consume(&self, raw: RawKey, is_press: bool) -> bool {
        self.consume_at(raw, is_press, Instant::now())
    }

    fn expect_at(&self, key: Key, is_press: bool, now: Instant) {
        let Ok(mut pending) = self.pending.lock() else {
            return;
        };
        pending.retain(|e| is_fresh(e, now));
        pending.push(Echo {
            key: raw_key_for(key),
            is_press,
            at: now,
        });
    }

    fn consume_at(&self, raw: RawKey, is_press: bool, now: Instant) -> bool {
        let Ok(mut pending) = self.pending.lock() else {
            return false;
        };
        pending.retain(|e| is_fresh(e, now));
        let wanted = normalize(raw);
        match pending
            .iter()
            .position(|e| e.key == wanted && e.is_press == is_press)
        {
            Some(index) => {
                pending.remove(index);
                true
            }
            None => false,
        }
    }
}

pub struct EchoTrackingDriver<D: KeyboardDriver> {
    inner: D,
    echoes: Arc<InjectedEchoes>,
}

impl<D: KeyboardDriver> EchoTrackingDriver<D> {
    pub fn new(inner: D, echoes: Arc<InjectedEchoes>) -> Self {
        EchoTrackingDriver { inner, echoes }
    }
}

impl<D: KeyboardDriver> KeyboardDriver for EchoTrackingDriver<D> {
    fn press(&mut self, key: Key) -> KeyActionResult {
        self.echoes.expect(key, true);
        self.inner.press(key)
    }

    fn release(&mut self, key: Key) -> KeyActionResult {
        self.echoes.expect(key, false);
        self.inner.release(key)
    }
}

#[cfg(test)]
mod tests;
