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
