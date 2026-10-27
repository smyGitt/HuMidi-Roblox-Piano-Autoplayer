use super::*;

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Press(kb::Key),
    Release(kb::Key),
}

struct RecordingDriver {
    log: Vec<Action>,
}

impl RecordingDriver {
    fn new() -> Self {
        RecordingDriver { log: vec![] }
    }
}

impl KeyboardDriver for RecordingDriver {
    fn press(&mut self, key: kb::Key) -> kb::KeyActionResult {
        self.log.push(Action::Press(key));
        Ok(())
    }
    fn release(&mut self, key: kb::Key) -> kb::KeyActionResult {
        self.log.push(Action::Release(key));
        Ok(())
    }
}

fn config() -> PlaybackConfig {
    PlaybackConfig::default()
}

fn press(time: f64, priority: i32, key_char: &str, pitch: i32, velocity: i32) -> KeyEvent {
    let mut e = KeyEvent::new(time, priority, "press", key_char);
    e.pitch = Some(pitch);
    e.velocity = Some(velocity);
    e
}

fn release(time: f64, priority: i32, key_char: &str, pitch: i32) -> KeyEvent {
    let mut e = KeyEvent::new(time, priority, "release", key_char);
    e.pitch = Some(pitch);
    e
}

fn pedal(time: f64, down: bool) -> KeyEvent {
    KeyEvent::new(time, if down { 1 } else { 0 }, "pedal", if down { "down" } else { "up" })
}

fn key_for_pitch(pitch: i32) -> char {
    KeyMapper::new(false).get_key_for_pitch(pitch).unwrap()
}

mod test_load_compiled_events {
    use super::*;

    #[test]
    fn test_populates_key_states_from_events() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let events = vec![press(0.0, 2, "a", 60, 64), release(0.5, 4, "a", 60)];
        engine.load_compiled_events(events, 1.0);
        assert!(engine.key_states.contains_key("a"));
    }

    #[test]
    fn test_deduplicates_key_states_across_multiple_events_for_same_key() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let events = vec![
            press(0.0, 2, "a", 60, 64),
            release(0.5, 4, "a", 60),
            press(1.0, 2, "a", 62, 64),
        ];
        engine.load_compiled_events(events, 2.0);
        assert_eq!(engine.key_states.len(), 1);
    }

    #[test]
    fn test_sets_total_duration() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![], 12.5);
        assert_eq!(engine.total_duration, 12.5);
    }
}

mod test_stop {
    use super::*;

    #[test]
    fn test_first_call_returns_status_message() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let events = engine.stop();
        assert_eq!(events, vec![PlayerEvent::Status("Stopping playback...".to_string())]);
        assert!(engine.is_stopped());
    }

    #[test]
    fn test_second_call_is_a_no_op() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.stop();
        let events = engine.stop();
        assert!(events.is_empty());
    }

    #[test]
    fn test_stop_clears_pause_flag() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.toggle_pause(0.0);
        assert!(engine.is_paused());
        engine.stop();
        assert!(!engine.is_paused());
    }
}

mod test_toggle_pause {
    use super::*;

    #[test]
    fn test_pausing_sets_paused_flag() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let events = engine.toggle_pause(10.0);
        assert!(engine.is_paused());
        assert_eq!(events, vec![PlayerEvent::Status("Paused.".to_string())]);
    }

    #[test]
    fn test_resuming_accumulates_paused_time() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.toggle_pause(10.0);
        let events = engine.toggle_pause(12.5);
        assert!(!engine.is_paused());
        assert_eq!(events, vec![PlayerEvent::Status("Resuming...".to_string())]);
        assert!((engine.total_paused_time - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_multiple_pause_resume_cycles_accumulate() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.toggle_pause(0.0);
        engine.toggle_pause(1.0);
        engine.toggle_pause(5.0);
        engine.toggle_pause(6.5);
        assert!((engine.total_paused_time - 2.5).abs() < 1e-9);
    }
}

mod test_seek {
    use super::*;

    #[test]
    fn test_seek_parks_target_without_mutating_index_immediately() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.seek(0.5);
        assert_eq!(engine.event_index(), 0);
    }

    #[test]
    fn test_tick_applies_pending_seek_and_updates_index() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(
            vec![
                press(0.0, 2, "a", 60, 64),
                release(0.5, 4, "a", 60),
                press(1.0, 2, "a", 62, 64),
                release(1.5, 4, "a", 62),
            ],
            2.0,
        );
        engine.start(0.0);
        engine.seek(0.9);
        let outcome = engine.tick(&mut driver, 0.9);
        assert_eq!(engine.event_index(), 2);
        assert!(outcome
            .events
            .iter()
            .any(|e| matches!(e, PlayerEvent::Progress(t) if (t - 0.9).abs() < 1e-9)));
    }

    #[test]
    fn test_seek_while_paused_resets_total_paused_time() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.start(0.0);
        engine.toggle_pause(1.0);
        engine.seek(0.5);
        engine.tick(&mut driver, 1.0);
        assert_eq!(engine.total_paused_time, 0.0);
    }
}

mod test_current_time {
    use super::*;

    #[test]
    fn test_while_playing_it_is_the_time_since_start() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.start(10.0);
        assert!((engine.current_time(12.5) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_while_paused_it_stays_at_the_moment_of_the_pause() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.start(0.0);
        engine.toggle_pause(3.0);
        assert!((engine.current_time(3.0) - 3.0).abs() < 1e-9);
        assert!((engine.current_time(7.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_after_a_resume_the_paused_stretch_is_left_out() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.start(0.0);
        engine.toggle_pause(3.0);
        engine.toggle_pause(5.0);
        assert!((engine.current_time(6.0) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_a_seek_while_paused_holds_at_the_target() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.start(0.0);
        engine.toggle_pause(1.0);
        engine.seek(0.5);
        engine.tick(&mut driver, 1.0);
        assert!((engine.current_time(4.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_a_seek_while_playing_moves_to_the_target_and_runs_on() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 3.0);
        engine.start(0.0);
        engine.seek(2.0);
        engine.tick(&mut driver, 1.0);
        assert!((engine.current_time(1.5) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_after_the_automatic_pause_at_the_end_it_holds() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 2.0);
        assert!(engine.is_paused());
        assert!((engine.current_time(9.0) - 2.0).abs() < 1e-9);
    }
}

mod test_tick_batching {
    use super::*;

    #[test]
    fn test_due_events_are_batched_and_executed() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let key_char = key_for_pitch(60);
        engine.load_compiled_events(vec![press(0.0, 2, &key_char.to_string(), 60, 64)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.1);
        assert_eq!(engine.event_index(), 1);
        assert!(driver.log.contains(&Action::Press(kb::Key::Char(key_char))));
    }

    #[test]
    fn test_future_events_are_not_executed_early() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(5.0, 2, "a", 60, 64)], 6.0);
        engine.start(0.0);
        let outcome = engine.tick(&mut driver, 0.1);
        assert_eq!(engine.event_index(), 0);
        assert!(driver.log.is_empty());
        assert!(outcome.suggested_sleep > 0.0);
    }

    #[test]
    fn test_batch_is_sorted_by_priority_before_execution() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(
            vec![
                release(0.0, 4, "a", 60),
                pedal(0.0, true),
                press(0.0, 2, "b", 62, 64),
            ],
            1.0,
        );
        engine.start(0.0);
        engine.tick(&mut driver, 0.1);
        assert_eq!(engine.event_index(), 3);
    }

    #[test]
    fn test_all_due_events_at_same_batch_consumed_in_one_tick() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(
            vec![
                press(0.0, 2, "a", 60, 64),
                press(0.01, 2, "b", 62, 64),
                press(0.02, 2, "c", 64, 64),
            ],
            1.0,
        );
        engine.start(0.0);
        engine.tick(&mut driver, 0.5);
        assert_eq!(engine.event_index(), 3);
    }
}

mod test_key_net_reference_counting {
    use super::*;

    #[test]
    fn test_overlapping_presses_on_same_key_do_not_release_until_net_zero() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(
            vec![
                press(0.0, 2, "a", 60, 64),
                press(0.0, 2, "a", 61, 64),
                release(0.1, 4, "a", 60),
                release(0.2, 4, "a", 61),
            ],
            1.0,
        );
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.tick(&mut driver, 0.15);
        assert!(
            !driver.log.contains(&Action::Release(kb::Key::Char('a'))),
            "key should still be net-held after only one of two overlapping presses released"
        );
        engine.tick(&mut driver, 0.25);
        assert!(driver.log.contains(&Action::Release(kb::Key::Char('a'))));
    }

    #[test]
    fn test_physically_down_key_gets_restruck_not_double_pressed_silently() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let key_char = key_for_pitch(60).to_string();
        engine.load_compiled_events(
            vec![
                press(0.0, 2, &key_char, 60, 64),
                press(0.1, 2, &key_char, 60, 64),
            ],
            1.0,
        );
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.tick(&mut driver, 0.15);
        let expected_char = key_for_pitch(60);
        let press_count = driver
            .log
            .iter()
            .filter(|a| **a == Action::Press(kb::Key::Char(expected_char)))
            .count();
        let release_count = driver
            .log
            .iter()
            .filter(|a| **a == Action::Release(kb::Key::Char(expected_char)))
            .count();
        assert_eq!(press_count, 2, "re-strike presses twice");
        assert_eq!(release_count, 1, "re-strike releases once, mid-restrike");
    }
}

mod test_pedal_handling {
    use super::*;

    #[test]
    fn test_pedal_down_presses_space() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.1);
        assert!(driver.log.contains(&Action::Press(kb::Key::Space)));
    }

    #[test]
    fn test_pedal_up_releases_space() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true), pedal(0.1, false)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.tick(&mut driver, 0.15);
        assert!(driver.log.contains(&Action::Release(kb::Key::Space)));
    }

    #[test]
    fn test_redundant_pedal_down_is_a_no_op_string() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let mut driver = RecordingDriver::new();
        let ev = pedal(0.0, true);
        engine.pedal_is_down = false;
        let first = engine.handle_pedal_event(&mut driver, &ev);
        let second = engine.handle_pedal_event(&mut driver, &ev);
        assert_eq!(first, "Pressed Space");
        assert_eq!(second, "Already down (no-op)");
    }

    #[test]
    fn test_stopped_engine_pedal_event_is_no_op() {
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let mut driver = RecordingDriver::new();
        engine.stop();
        let result = engine.handle_pedal_event(&mut driver, &pedal(0.0, true));
        assert_eq!(result, "Stopped (no-op)");
    }
}

mod test_auto_pause {
    use super::*;

    #[test]
    fn test_auto_pauses_after_events_exhausted_and_duration_passed() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 0.2);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        let outcome = engine.tick(&mut driver, 0.5);
        assert!(engine.is_paused());
        assert!(outcome.events.contains(&PlayerEvent::AutoPaused));
    }

    #[test]
    fn test_does_not_auto_pause_before_duration_grace_period() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        let outcome = engine.tick(&mut driver, 1.05);
        assert!(!engine.is_paused());
        assert!(!outcome.events.contains(&PlayerEvent::AutoPaused));
    }

    #[test]
    fn test_auto_pause_shuts_down_active_keys() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 0.1);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.tick(&mut driver, 0.5);
        assert!(driver.log.contains(&Action::Release(kb::Key::Char('a'))));
    }
}

mod test_sections {
    use super::*;

    #[test]
    fn test_section_boundary_crossed_emits_section_event() {
        let mut driver = RecordingDriver::new();
        let sections = vec![MusicalSection::new(0.0, 1.0, vec![])];
        let mut engine = PlaybackEngine::new(config(), sections);
        engine.load_compiled_events(vec![], 2.0);
        engine.start(0.0);
        let outcome = engine.tick(&mut driver, 0.1);
        assert!(outcome.events.contains(&PlayerEvent::Section(0)));
    }

    #[test]
    fn test_section_not_yet_reached_does_not_emit() {
        let mut driver = RecordingDriver::new();
        let sections = vec![MusicalSection::new(5.0, 6.0, vec![])];
        let mut engine = PlaybackEngine::new(config(), sections);
        engine.load_compiled_events(vec![], 10.0);
        engine.start(0.0);
        let outcome = engine.tick(&mut driver, 0.1);
        assert!(!outcome.events.iter().any(|e| matches!(e, PlayerEvent::Section(_))));
    }
}

mod test_shutdown {
    use super::*;

    #[test]
    fn test_releases_all_active_keys() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.shutdown(&mut driver);
        assert!(driver.log.contains(&Action::Release(kb::Key::Char('a'))));
    }

    #[test]
    fn test_releases_pedal_if_down() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.shutdown(&mut driver);
        assert!(driver.log.contains(&Action::Release(kb::Key::Space)));
        assert!(!engine.pedal_is_down);
    }

    #[test]
    fn test_release_pedal_releases_space_even_when_the_engine_thinks_it_is_up() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        assert!(!engine.pedal_is_down);
        engine.release_pedal(&mut driver);
        assert_eq!(driver.log, vec![Action::Release(kb::Key::Space)]);
        assert!(!engine.pedal_is_down);
    }

    #[test]
    fn test_release_pedal_clears_a_pedal_the_engine_thinks_is_down() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        assert!(engine.pedal_is_down);
        engine.release_pedal(&mut driver);
        assert!(!engine.pedal_is_down);
    }

    #[test]
    fn test_pedal_down_after_release_pedal_presses_space_again() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true)], 1.0);
        engine.release_pedal(&mut driver);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        assert_eq!(
            driver.log,
            vec![Action::Release(kb::Key::Space), Action::Press(kb::Key::Space)]
        );
    }

    #[test]
    fn test_releases_all_modifiers_unconditionally() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.shutdown(&mut driver);
        assert!(driver.log.contains(&Action::Release(kb::Key::Shift)));
        assert!(driver.log.contains(&Action::Release(kb::Key::Ctrl)));
        assert!(driver.log.contains(&Action::Release(kb::Key::Alt)));
    }

    #[test]
    fn test_shutdown_on_inactive_keys_does_not_call_release() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.shutdown(&mut driver);
        assert!(!driver.log.contains(&Action::Release(kb::Key::Char('a'))));
    }
}

mod test_pause_resume_physical_state {
    use super::*;

    #[test]
    fn test_pausing_releases_held_keys() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![press(0.0, 2, "a", 60, 64)], 1.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.toggle_pause(0.1);
        engine.tick(&mut driver, 0.15);
        assert!(driver.log.contains(&Action::Release(kb::Key::Char('a'))));
    }

    #[test]
    fn test_resuming_represses_held_keys() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        let key_char = key_for_pitch(60).to_string();
        engine.load_compiled_events(
            vec![press(0.0, 2, &key_char, 60, 64), release(5.0, 4, &key_char, 60)],
            6.0,
        );
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.toggle_pause(0.1);
        engine.tick(&mut driver, 0.15);
        driver.log.clear();
        engine.toggle_pause(1.0);
        engine.tick(&mut driver, 1.0);
        let expected_char = key_for_pitch(60);
        let press_count = driver
            .log
            .iter()
            .filter(|a| **a == Action::Press(kb::Key::Char(expected_char)))
            .count();
        assert_eq!(press_count, 1, "resume should re-press the still-held key");
    }

    #[test]
    fn test_resuming_restores_pedal_if_it_was_down() {
        let mut driver = RecordingDriver::new();
        let mut engine = PlaybackEngine::new(config(), vec![]);
        engine.load_compiled_events(vec![pedal(0.0, true), pedal(5.0, false)], 6.0);
        engine.start(0.0);
        engine.tick(&mut driver, 0.05);
        engine.toggle_pause(0.1);
        engine.tick(&mut driver, 0.15);
        driver.log.clear();
        engine.toggle_pause(1.0);
        engine.tick(&mut driver, 1.0);
        assert!(driver.log.contains(&Action::Press(kb::Key::Space)));
    }
}

mod test_get_press_info_from_event {
    use super::*;

    #[test]
    fn test_no_pitch_returns_empty_modifiers_and_key_char() {
        let engine = PlaybackEngine::new(config(), vec![]);
        let ev = KeyEvent::new(0.0, 1, "pedal", "down");
        let (modifiers, key) = engine.get_press_info_from_event(&ev);
        assert!(modifiers.is_empty());
        assert_eq!(key, "down");
    }

    #[test]
    fn test_black_key_pitch_carries_shift_modifier() {
        let engine = PlaybackEngine::new(config(), vec![]);
        let ev = press(0.0, 2, "1", 37, 64);
        let (modifiers, _) = engine.get_press_info_from_event(&ev);
        assert!(modifiers.contains(&kb::Key::Shift));
    }
}

mod test_symbol_to_base {
    use super::*;

    #[test]
    fn test_symbol_maps_to_digit() {
        assert_eq!(symbol_to_base("!"), '1');
        assert_eq!(symbol_to_base(")"), '0');
    }

    #[test]
    fn test_non_symbol_passes_through_unchanged() {
        assert_eq!(symbol_to_base("a"), 'a');
    }
}
