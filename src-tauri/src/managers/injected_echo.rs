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
mod tests {
    use super::*;
    use crate::core::keyboard_driver::KeyActionError;

    const BEFORE_EXPIRY: Duration = Duration::from_millis(100);
    const AFTER_EXPIRY: Duration = Duration::from_millis(500);

    mod test_raw_key_for {
        use super::*;

        #[test]
        fn test_chars_are_lowercased() {
            assert_eq!(raw_key_for(Key::Char('S')), RawKey::Char('s'));
            assert_eq!(raw_key_for(Key::Char('7')), RawKey::Char('7'));
        }

        #[test]
        fn test_space_and_modifiers() {
            assert_eq!(raw_key_for(Key::Space), RawKey::Named("Space"));
            assert_eq!(raw_key_for(Key::Ctrl), RawKey::Mod(Modifier::Ctrl));
            assert_eq!(raw_key_for(Key::Shift), RawKey::Mod(Modifier::Shift));
            assert_eq!(raw_key_for(Key::Alt), RawKey::Mod(Modifier::Alt));
        }
    }

    mod test_echoes {
        use super::*;

        #[test]
        fn test_an_expected_press_is_consumed_once() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            assert!(echoes.consume_at(RawKey::Char('s'), true, now));
            assert!(!echoes.consume_at(RawKey::Char('s'), true, now));
        }

        #[test]
        fn test_nothing_expected_means_nothing_consumed() {
            let echoes = InjectedEchoes::new();
            assert!(!echoes.consume_at(RawKey::Named("F6"), true, Instant::now()));
        }

        #[test]
        fn test_press_and_release_are_tracked_separately() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            assert!(!echoes.consume_at(RawKey::Char('s'), false, now));
            assert!(echoes.consume_at(RawKey::Char('s'), true, now));
        }

        #[test]
        fn test_a_different_key_is_not_consumed() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            assert!(!echoes.consume_at(RawKey::Char('d'), true, now));
        }

        #[test]
        fn test_left_and_right_modifiers_match_the_generic_modifier() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Ctrl, true, now);
            echoes.expect_at(Key::Shift, false, now);
            assert!(echoes.consume_at(RawKey::ModLeft(Modifier::Ctrl), true, now));
            assert!(echoes.consume_at(RawKey::ModRight(Modifier::Shift), false, now));
        }

        #[test]
        fn test_two_expected_presses_are_consumed_twice() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            echoes.expect_at(Key::Char('s'), true, now);
            assert!(echoes.consume_at(RawKey::Char('s'), true, now));
            assert!(echoes.consume_at(RawKey::Char('s'), true, now));
            assert!(!echoes.consume_at(RawKey::Char('s'), true, now));
        }

        #[test]
        fn test_an_echo_still_counts_before_it_expires() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Space, true, now);
            assert!(echoes.consume_at(RawKey::Named("Space"), true, now + BEFORE_EXPIRY));
        }

        #[test]
        fn test_an_echo_that_never_arrived_expires_and_does_not_swallow_a_real_press() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            assert!(!echoes.consume_at(RawKey::Char('s'), true, now + AFTER_EXPIRY));
        }

        #[test]
        fn test_expired_entries_are_dropped_when_a_new_one_is_added() {
            let echoes = InjectedEchoes::new();
            let now = Instant::now();
            echoes.expect_at(Key::Char('s'), true, now);
            echoes.expect_at(Key::Char('d'), true, now + AFTER_EXPIRY);
            assert_eq!(echoes.pending.lock().unwrap().len(), 1);
        }
    }

    mod test_echo_tracking_driver {
        use super::*;

        #[derive(Default)]
        struct FakeDriver {
            presses: Vec<Key>,
            releases: Vec<Key>,
            fail: bool,
        }

        impl KeyboardDriver for FakeDriver {
            fn press(&mut self, key: Key) -> KeyActionResult {
                self.presses.push(key);
                if self.fail {
                    Err(KeyActionError("failed".to_string()))
                } else {
                    Ok(())
                }
            }
            fn release(&mut self, key: Key) -> KeyActionResult {
                self.releases.push(key);
                Ok(())
            }
        }

        #[test]
        fn test_presses_and_releases_reach_the_inner_driver_and_are_registered() {
            let echoes = Arc::new(InjectedEchoes::new());
            let mut driver = EchoTrackingDriver::new(FakeDriver::default(), Arc::clone(&echoes));
            driver.press(Key::Ctrl).unwrap();
            driver.press(Key::Char('s')).unwrap();
            driver.release(Key::Char('s')).unwrap();
            driver.release(Key::Ctrl).unwrap();
            assert_eq!(driver.inner.presses, vec![Key::Ctrl, Key::Char('s')]);
            assert_eq!(driver.inner.releases, vec![Key::Char('s'), Key::Ctrl]);
            assert!(echoes.consume(RawKey::ModLeft(Modifier::Ctrl), true));
            assert!(echoes.consume(RawKey::Char('s'), true));
            assert!(echoes.consume(RawKey::Char('s'), false));
            assert!(echoes.consume(RawKey::ModRight(Modifier::Ctrl), false));
        }

        #[test]
        fn test_the_inner_error_is_passed_through() {
            let echoes = Arc::new(InjectedEchoes::new());
            let inner = FakeDriver {
                fail: true,
                ..FakeDriver::default()
            };
            let mut driver = EchoTrackingDriver::new(inner, echoes);
            assert_eq!(
                driver.press(Key::Char('s')),
                Err(KeyActionError("failed".to_string()))
            );
        }

        #[test]
        fn test_a_real_key_the_driver_never_sent_is_not_consumed() {
            let echoes = Arc::new(InjectedEchoes::new());
            let mut driver = EchoTrackingDriver::new(FakeDriver::default(), Arc::clone(&echoes));
            driver.press(Key::Char('s')).unwrap();
            assert!(!echoes.consume(RawKey::Named("F6"), true));
        }
    }
}
