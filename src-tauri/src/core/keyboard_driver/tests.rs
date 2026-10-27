use super::*;

mod test_platform_from_os_str {
    use super::*;

    #[test]
    fn test_windows() {
        assert_eq!(platform_from_os_str("windows"), Platform::Windows);
    }

    #[test]
    fn test_macos() {
        assert_eq!(platform_from_os_str("macos"), Platform::MacOs);
    }

    #[test]
    fn test_linux() {
        assert_eq!(platform_from_os_str("linux"), Platform::Linux);
    }

    #[test]
    fn test_freebsd_is_other() {
        assert_eq!(platform_from_os_str("freebsd"), Platform::Other);
    }

    #[test]
    fn test_unknown_is_other() {
        assert_eq!(platform_from_os_str("nonexistent-os"), Platform::Other);
    }

    #[test]
    fn test_current_matches_std_env_consts_os() {
        assert_eq!(Platform::current(), platform_from_os_str(std::env::consts::OS));
    }
}

mod test_mac_digit_vk {
    use super::*;

    #[test]
    fn test_all_ten_digits_transcribed_byte_for_byte() {
        let expected: [(char, u32); 10] = [
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
        for (ch, vk) in expected {
            assert_eq!(mac_digit_vk(ch), Some(vk), "digit {ch} mismatched");
        }
    }

    #[test]
    fn test_letters_are_not_digits() {
        for ch in "abcxyzABCXYZ".chars() {
            assert_eq!(mac_digit_vk(ch), None, "{ch} should not be a digit vk");
        }
    }

    #[test]
    fn test_symbols_are_not_digits() {
        for ch in "!@#$%^&*()".chars() {
            assert_eq!(mac_digit_vk(ch), None, "{ch} should not be a digit vk");
        }
    }

    #[test]
    fn test_non_ascii_is_not_a_digit() {
        assert_eq!(mac_digit_vk('あ'), None);
    }

    #[test]
    fn test_space_is_not_a_digit() {
        assert_eq!(mac_digit_vk(' '), None);
    }
}

mod test_resolve_enigo_key {
    use super::*;

    const ALL_PLATFORMS: [Platform; 4] =
        [Platform::Windows, Platform::MacOs, Platform::Linux, Platform::Other];

    #[test]
    fn test_space_maps_to_space_on_every_platform() {
        for platform in ALL_PLATFORMS {
            assert_eq!(resolve_enigo_key(Key::Space, platform), enigo::Key::Space);
        }
    }

    #[test]
    fn test_ctrl_maps_to_control_on_every_platform() {
        for platform in ALL_PLATFORMS {
            assert_eq!(resolve_enigo_key(Key::Ctrl, platform), enigo::Key::Control);
        }
    }

    #[test]
    fn test_shift_maps_to_shift_on_every_platform() {
        for platform in ALL_PLATFORMS {
            assert_eq!(resolve_enigo_key(Key::Shift, platform), enigo::Key::Shift);
        }
    }

    #[test]
    fn test_alt_maps_to_alt_on_every_platform() {
        for platform in ALL_PLATFORMS {
            assert_eq!(resolve_enigo_key(Key::Alt, platform), enigo::Key::Alt);
        }
    }

    #[test]
    fn test_digit_chars_are_coerced_to_number_row_vk_on_macos_only() {
        for ch in "1234567890".chars() {
            let expected_vk = mac_digit_vk(ch).unwrap();
            assert_eq!(
                resolve_enigo_key(Key::Char(ch), Platform::MacOs),
                enigo::Key::Other(expected_vk)
            );
            for platform in [Platform::Windows, Platform::Linux, Platform::Other] {
                assert_eq!(
                    resolve_enigo_key(Key::Char(ch), platform),
                    enigo::Key::Unicode(ch),
                    "digit {ch} should not be coerced on {platform:?}"
                );
            }
        }
    }

    #[test]
    fn test_letter_chars_are_never_coerced_even_on_macos() {
        for ch in "abcqwertyuiopasdfghjklzxcvbnm".chars() {
            for platform in ALL_PLATFORMS {
                assert_eq!(
                    resolve_enigo_key(Key::Char(ch), platform),
                    enigo::Key::Unicode(ch)
                );
            }
        }
    }

    #[test]
    fn test_symbol_chars_are_never_coerced() {
        for ch in "!@#$%^&*()".chars() {
            for platform in ALL_PLATFORMS {
                assert_eq!(
                    resolve_enigo_key(Key::Char(ch), platform),
                    enigo::Key::Unicode(ch)
                );
            }
        }
    }

    #[test]
    fn test_linux_never_coerces_digits() {
        for ch in "1234567890".chars() {
            assert_eq!(
                resolve_enigo_key(Key::Char(ch), Platform::Linux),
                enigo::Key::Unicode(ch)
            );
        }
    }
}

mod test_check_platform_permission {
    use super::*;

    #[test]
    fn test_macos_with_permission_ok() {
        assert!(check_platform_permission(Platform::MacOs, || true).is_ok());
    }

    #[test]
    fn test_macos_without_permission_err() {
        let result = check_platform_permission(Platform::MacOs, || false);
        assert!(matches!(
            result,
            Err(BuildDriverError::MacosAccessibilityDenied)
        ));
    }

    #[test]
    fn test_windows_ignores_permission_check() {
        assert!(check_platform_permission(Platform::Windows, || false).is_ok());
        assert!(check_platform_permission(Platform::Windows, || true).is_ok());
    }

    #[test]
    fn test_linux_ignores_permission_check() {
        assert!(check_platform_permission(Platform::Linux, || false).is_ok());
    }

    #[test]
    fn test_other_ignores_permission_check() {
        assert!(check_platform_permission(Platform::Other, || false).is_ok());
    }

    #[test]
    fn test_permission_check_not_called_on_non_macos() {
        let mut called = false;
        let _ = check_platform_permission(Platform::Windows, || {
            called = true;
            false
        });
        assert!(!called);
    }
}

mod test_pressed_guard {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Action {
        Press(Key),
        Release(Key),
    }

    struct RecordingDriver {
        log: Vec<Action>,
    }

    impl KeyboardDriver for RecordingDriver {
        fn press(&mut self, key: Key) -> KeyActionResult {
            self.log.push(Action::Press(key));
            Ok(())
        }
        fn release(&mut self, key: Key) -> KeyActionResult {
            self.log.push(Action::Release(key));
            Ok(())
        }
    }

    #[test]
    fn test_modifiers_pressed_before_guard_returns() {
        let mut driver = RecordingDriver { log: vec![] };
        let guard = pressed(&mut driver, &[Key::Ctrl, Key::Shift]);
        drop(guard);
        assert_eq!(driver.log[0], Action::Press(Key::Ctrl));
        assert_eq!(driver.log[1], Action::Press(Key::Shift));
    }

    #[test]
    fn test_modifiers_released_in_reverse_order_on_drop() {
        let mut driver = RecordingDriver { log: vec![] };
        let guard = pressed(&mut driver, &[Key::Ctrl, Key::Shift]);
        drop(guard);
        assert_eq!(driver.log[2], Action::Release(Key::Shift));
        assert_eq!(driver.log[3], Action::Release(Key::Ctrl));
    }

    #[test]
    fn test_base_key_pressed_and_released_between_modifier_press_and_release() {
        let mut driver = RecordingDriver { log: vec![] };
        {
            let mut guard = pressed(&mut driver, &[Key::Ctrl]);
            let _ = guard.press(Key::Char('1'));
            let _ = guard.release(Key::Char('1'));
        }
        assert_eq!(
            driver.log,
            vec![
                Action::Press(Key::Ctrl),
                Action::Press(Key::Char('1')),
                Action::Release(Key::Char('1')),
                Action::Release(Key::Ctrl),
            ]
        );
    }

    #[test]
    fn test_empty_modifiers_presses_and_releases_nothing() {
        let mut driver = RecordingDriver { log: vec![] };
        let guard = pressed(&mut driver, &[]);
        drop(guard);
        assert!(driver.log.is_empty());
    }

    #[test]
    fn test_single_modifier_round_trips() {
        let mut driver = RecordingDriver { log: vec![] };
        let guard = pressed(&mut driver, &[Key::Alt]);
        drop(guard);
        assert_eq!(
            driver.log,
            vec![Action::Press(Key::Alt), Action::Release(Key::Alt)]
        );
    }

    struct FailingDriver;

    impl KeyboardDriver for FailingDriver {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Err(KeyActionError("press failed".to_string()))
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Err(KeyActionError("release failed".to_string()))
        }
    }

    #[test]
    fn test_guard_base_key_press_failure_is_reported_to_caller() {
        let mut driver = FailingDriver;
        let mut guard = pressed(&mut driver, &[Key::Ctrl]);
        let result = guard.press(Key::Char('1'));
        assert!(result.is_err());
    }

    #[test]
    fn test_guard_drop_does_not_panic_when_release_fails() {
        let mut driver = FailingDriver;
        let guard = pressed(&mut driver, &[Key::Ctrl, Key::Shift]);
        drop(guard);
    }

    #[test]
    fn test_pressed_does_not_propagate_modifier_press_failure() {
        let mut driver = FailingDriver;
        let _guard = pressed(&mut driver, &[Key::Ctrl]);
    }
}

mod test_boxed_driver {
    use super::*;

    struct FailingDriver;

    impl KeyboardDriver for FailingDriver {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Err(KeyActionError("press failed".to_string()))
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Err(KeyActionError("release failed".to_string()))
        }
    }

    fn presses_and_releases(driver: &mut impl KeyboardDriver) -> (KeyActionResult, KeyActionResult) {
        (driver.press(Key::Shift), driver.release(Key::Char('g')))
    }

    #[test]
    fn test_a_boxed_driver_forwards_presses_and_releases() {
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        struct Shared(std::sync::Arc<std::sync::Mutex<Vec<(Key, bool)>>>);
        impl KeyboardDriver for Shared {
            fn press(&mut self, key: Key) -> KeyActionResult {
                self.0.lock().unwrap().push((key, true));
                Ok(())
            }
            fn release(&mut self, key: Key) -> KeyActionResult {
                self.0.lock().unwrap().push((key, false));
                Ok(())
            }
        }
        let mut boxed: Box<dyn KeyboardDriver + Send> = Box::new(Shared(std::sync::Arc::clone(&log)));
        assert_eq!(presses_and_releases(&mut boxed), (Ok(()), Ok(())));
        assert_eq!(*log.lock().unwrap(), vec![(Key::Shift, true), (Key::Char('g'), false)]);
    }

    #[test]
    fn test_a_boxed_driver_passes_errors_on() {
        let mut boxed: Box<dyn KeyboardDriver + Send> = Box::new(FailingDriver);
        assert_eq!(
            presses_and_releases(&mut boxed),
            (
                Err(KeyActionError("press failed".to_string())),
                Err(KeyActionError("release failed".to_string()))
            )
        );
    }
}

mod test_enigo_features {
    #[test]
    fn test_the_libei_backend_is_not_enabled_because_its_portal_failure_aborts_the_app() {
        let manifest = include_str!("../../../Cargo.toml");
        let enigo_line = manifest.lines().find(|l| l.starts_with("enigo")).unwrap();
        assert!(!enigo_line.contains("libei"), "{enigo_line}");
    }
}
