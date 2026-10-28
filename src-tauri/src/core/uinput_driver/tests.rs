use super::*;
use crate::core::keyboard_driver::KeyActionError;
use std::cell::Cell;

const KEY_G: u16 = 34;
const KEY_LEFTSHIFT: u16 = 42;

type Sent = Arc<Mutex<Vec<(u16, bool)>>>;

struct FakeSink {
    sent: Sent,
    fail: bool,
}

impl KeySink for FakeSink {
    fn send(&mut self, code: u16, pressed: bool) -> io::Result<()> {
        if self.fail {
            return Err(io::Error::other("device gone"));
        }
        self.sent.lock().unwrap().push((code, pressed));
        Ok(())
    }
}

fn fake_sink() -> (FakeSink, Sent) {
    let sent: Sent = Arc::new(Mutex::new(Vec::new()));
    (
        FakeSink {
            sent: Arc::clone(&sent),
            fail: false,
        },
        sent,
    )
}

mod test_uinput_driver {
    use super::*;

    #[test]
    fn test_a_press_and_a_release_send_the_key_down_then_up() {
        let (sink, sent) = fake_sink();
        let mut driver = UinputDriver::new(SharedSink::new(sink));
        driver.press(Key::Char('g')).unwrap();
        driver.release(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true), (KEY_G, false)]);
    }

    #[test]
    fn test_modifiers_are_sent_as_their_own_keys() {
        let (sink, sent) = fake_sink();
        let mut driver = UinputDriver::new(SharedSink::new(sink));
        driver.press(Key::Shift).unwrap();
        driver.release(Key::Shift).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_LEFTSHIFT, true), (KEY_LEFTSHIFT, false)]);
    }

    #[test]
    fn test_a_key_with_no_linux_code_is_an_error_and_sends_nothing() {
        let (sink, sent) = fake_sink();
        let mut driver = UinputDriver::new(SharedSink::new(sink));
        let press = driver.press(Key::Char('!')).unwrap_err();
        let release = driver.release(Key::Char('!')).unwrap_err();
        assert!(press.0.contains("'!'"), "{press}");
        assert!(release.0.contains("'!'"), "{release}");
        assert!(sent.lock().unwrap().is_empty());
    }

    #[test]
    fn test_a_device_failure_is_reported_for_that_key() {
        let mut driver = UinputDriver::new(SharedSink::new(FakeSink {
            sent: Arc::new(Mutex::new(Vec::new())),
            fail: true,
        }));
        assert_eq!(driver.press(Key::Char('g')), Err(KeyActionError("device gone".to_string())));
        assert_eq!(driver.release(Key::Char('g')), Err(KeyActionError("device gone".to_string())));
    }

    #[test]
    fn test_two_drivers_on_one_shared_device_send_to_the_same_device() {
        let (sink, sent) = fake_sink();
        let shared = SharedSink::new(sink);
        UinputDriver::new(shared.clone()).press(Key::Char('g')).unwrap();
        UinputDriver::new(shared).release(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true), (KEY_G, false)]);
    }

    #[test]
    fn test_a_poisoned_device_lock_is_an_error_not_a_panic() {
        let (sink, _sent) = fake_sink();
        let shared = SharedSink::new(sink);
        let poisoner = shared.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.0.lock().unwrap();
            panic!("poison the device lock");
        })
        .join();
        assert!(UinputDriver::new(shared).press(Key::Char('g')).is_err());
    }
}

mod test_shared_sink_or_open {
    use super::*;

    #[test]
    fn test_the_first_call_opens_the_device_waits_for_it_and_keeps_it() {
        let slot = Mutex::new(None);
        let (sink, sent) = fake_sink();
        let settled = Cell::new(0);
        let shared = shared_sink_or_open(&slot, || Ok(sink), || settled.set(settled.get() + 1)).unwrap();
        assert_eq!(settled.get(), 1);
        assert!(slot.lock().unwrap().is_some());
        UinputDriver::new(shared).press(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true)]);
    }

    #[test]
    fn test_a_later_call_reuses_the_device_without_opening_or_waiting_again() {
        let slot = Mutex::new(None);
        let (sink, sent) = fake_sink();
        shared_sink_or_open(&slot, || Ok(sink), || {}).unwrap();
        let opened_again = Cell::new(false);
        let settled_again = Cell::new(false);
        let shared = shared_sink_or_open(
            &slot,
            || {
                opened_again.set(true);
                Ok(fake_sink().0)
            },
            || settled_again.set(true),
        )
        .unwrap();
        assert!(!opened_again.get());
        assert!(!settled_again.get());
        UinputDriver::new(shared).press(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true)]);
    }

    #[test]
    fn test_a_device_that_cannot_be_opened_gives_none_without_waiting_and_is_retried_later() {
        let slot = Mutex::new(None);
        let settled = Cell::new(false);
        let result = shared_sink_or_open::<FakeSink>(
            &slot,
            || Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            || settled.set(true),
        );
        assert!(result.is_none());
        assert!(!settled.get());
        assert!(slot.lock().unwrap().is_none());
        assert!(shared_sink_or_open(&slot, || Ok(fake_sink().0), || {}).is_some());
    }

    #[test]
    fn test_a_poisoned_slot_gives_none() {
        let slot = Mutex::new(None::<SharedSink>);
        std::thread::scope(|scope| {
            let _ = scope
                .spawn(|| {
                    let _guard = slot.lock().unwrap();
                    panic!("poison the slot");
                })
                .join();
        });
        assert!(shared_sink_or_open(&slot, || Ok(fake_sink().0), || {}).is_none());
    }
}

mod test_driver_or_fallback {
    use super::*;

    struct FallbackDriver {
        sent: Sent,
    }

    impl KeyboardDriver for FallbackDriver {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            self.sent.lock().unwrap().push((0, true));
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            self.sent.lock().unwrap().push((0, false));
            Ok(())
        }
    }

    #[test]
    fn test_a_device_is_used_and_the_fallback_is_never_built() {
        let (sink, sent) = fake_sink();
        let built = Cell::new(false);
        let mut driver = driver_or_fallback(Some(SharedSink::new(sink)), || {
            built.set(true);
            Ok::<_, String>(FallbackDriver {
                sent: Arc::new(Mutex::new(Vec::new())),
            })
        })
        .unwrap();
        driver.press(Key::Char('g')).unwrap();
        assert!(!built.get());
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true)]);
    }

    #[test]
    fn test_without_a_device_the_fallback_is_used() {
        let sent: Sent = Arc::new(Mutex::new(Vec::new()));
        let fallback_sent = Arc::clone(&sent);
        let mut driver =
            driver_or_fallback(None, || Ok::<_, String>(FallbackDriver { sent: fallback_sent })).unwrap();
        driver.press(Key::Char('g')).unwrap();
        driver.release(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(0, true), (0, false)]);
    }

    #[test]
    fn test_without_a_device_a_fallback_error_is_passed_on() {
        let result = driver_or_fallback(None, || Err::<FallbackDriver, _>("no display".to_string()));
        assert_eq!(result.err(), Some("no display".to_string()));
    }
}

mod test_driver_failure_message {
    use super::*;

    #[test]
    fn test_on_linux_it_names_the_device_and_the_command_that_grants_access() {
        let message = driver_failure_message(Platform::Linux, "no display");
        assert!(message.contains("/dev/uinput"), "{message}");
        assert!(message.contains("sudo modprobe uinput"), "{message}");
        assert!(message.contains("sudo chmod 0666 /dev/uinput"), "{message}");
        assert!(message.ends_with("no display"), "{message}");
    }

    #[test]
    fn test_on_other_platforms_the_message_is_unchanged() {
        for platform in [Platform::Windows, Platform::MacOs, Platform::Other] {
            assert_eq!(driver_failure_message(platform, "no display"), "no display");
        }
    }
}

mod test_driver_or_explained_fallback {
    use super::*;

    struct Quiet;

    impl KeyboardDriver for Quiet {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
    }

    #[test]
    fn test_a_fallback_error_on_linux_comes_back_explained() {
        let result =
            driver_or_explained_fallback(Platform::Linux, false, None, || Err::<Quiet, _>("no display".to_string()));
        let message = result.err().unwrap();
        assert!(message.contains("sudo chmod 0666 /dev/uinput"), "{message}");
        assert!(message.ends_with("no display"), "{message}");
    }

    #[test]
    fn test_a_fallback_error_elsewhere_comes_back_as_it_was() {
        let result =
            driver_or_explained_fallback(Platform::Windows, false, None, || Err::<Quiet, _>("no display".to_string()));
        assert_eq!(result.err(), Some("no display".to_string()));
    }

    #[test]
    fn test_a_working_fallback_and_a_device_both_give_a_driver() {
        assert!(driver_or_explained_fallback(Platform::Linux, false, None, || Ok::<_, String>(Quiet)).is_ok());
        let (sink, _sent) = fake_sink();
        assert!(driver_or_explained_fallback(Platform::Linux, false, Some(SharedSink::new(sink)), || {
            Err::<Quiet, _>("unused".to_string())
        })
        .is_ok());
    }
}

mod test_wayland_without_a_device {
    use super::*;
    use std::cell::Cell;

    struct Quiet;

    impl KeyboardDriver for Quiet {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
    }

    #[test]
    fn test_on_wayland_without_a_device_there_is_an_error_and_the_fallback_is_never_built() {
        let built = Cell::new(false);
        let result = driver_or_explained_fallback(Platform::Linux, true, None, || {
            built.set(true);
            Ok::<_, String>(Quiet)
        });
        let message = result.err().unwrap();
        assert!(!built.get());
        assert!(message.contains("/dev/uinput"), "{message}");
        assert!(message.contains("sudo chmod 0666 /dev/uinput"), "{message}");
        assert!(message.contains("Wayland"), "{message}");
    }

    #[test]
    fn test_on_wayland_a_device_is_still_used() {
        let (sink, sent) = fake_sink();
        let mut driver = driver_or_explained_fallback(Platform::Linux, true, Some(SharedSink::new(sink)), || {
            Err::<Quiet, _>("unused".to_string())
        })
        .unwrap();
        driver.press(Key::Char('g')).unwrap();
        assert_eq!(*sent.lock().unwrap(), vec![(KEY_G, true)]);
    }

    #[test]
    fn test_without_wayland_the_fallback_still_works_when_there_is_no_device() {
        assert!(driver_or_explained_fallback(Platform::Linux, false, None, || Ok::<_, String>(Quiet)).is_ok());
    }

    #[test]
    fn test_the_wayland_rule_applies_only_on_linux() {
        assert!(driver_or_explained_fallback(Platform::Windows, true, None, || Ok::<_, String>(Quiet)).is_ok());
        assert!(driver_or_explained_fallback(Platform::MacOs, true, None, || Ok::<_, String>(Quiet)).is_ok());
    }
}

mod test_is_wayland_session {
    use super::*;

    fn session(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name| owned.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    #[test]
    fn test_a_wayland_session_type_is_wayland_whatever_its_case() {
        assert!(is_wayland_session(session(&[("XDG_SESSION_TYPE", "wayland")])));
        assert!(is_wayland_session(session(&[("XDG_SESSION_TYPE", "Wayland")])));
    }

    #[test]
    fn test_an_x11_session_is_not_wayland_even_with_a_wayland_display_set() {
        assert!(!is_wayland_session(session(&[("XDG_SESSION_TYPE", "x11"), ("WAYLAND_DISPLAY", "wayland-0")])));
    }

    #[test]
    fn test_any_other_session_type_is_not_wayland() {
        assert!(!is_wayland_session(session(&[("XDG_SESSION_TYPE", "tty"), ("WAYLAND_DISPLAY", "wayland-0")])));
    }

    #[test]
    fn test_without_a_session_type_a_wayland_display_decides() {
        assert!(is_wayland_session(session(&[("WAYLAND_DISPLAY", "wayland-0")])));
        assert!(!is_wayland_session(session(&[("WAYLAND_DISPLAY", "")])));
    }

    #[test]
    fn test_an_empty_session_type_counts_as_missing() {
        assert!(is_wayland_session(session(&[("XDG_SESSION_TYPE", ""), ("WAYLAND_DISPLAY", "wayland-0")])));
    }

    #[test]
    fn test_with_neither_it_is_not_wayland() {
        assert!(!is_wayland_session(session(&[])));
    }
}


mod test_virtual_keyboard_name {
    use super::*;

    #[test]
    fn test_the_name_is_the_one_the_ci_rule_and_the_hotkey_reader_look_for() {
        assert_eq!(VIRTUAL_KEYBOARD_NAME, "HuMidi virtual keyboard");
    }
}
