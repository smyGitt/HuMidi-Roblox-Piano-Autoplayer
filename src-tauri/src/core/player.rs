use std::collections::{HashMap, HashSet};

use crate::core::config::PlaybackConfig;
use crate::core::keyboard_driver::{self as kb, KeyboardDriver};
use crate::core::midi::{KeyMapper, Modifier};
use crate::core::models::{KeyEvent, KeyState, MusicalSection};

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    Status(String),
    Progress(f64),
    Finished,
    Visualizer(Vec<i32>),
    Pedal(bool),
    AutoPaused,
    Error(String),
    Section(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickOutcome {
    pub events: Vec<PlayerEvent>,
    pub suggested_sleep: f64,
}

fn symbol_to_base(key_char: &str) -> char {
    let ch = key_char.chars().next().unwrap_or(' ');
    for &(symbol, base) in KeyMapper::SYMBOL_MAP.iter() {
        if symbol == ch {
            return base;
        }
    }
    ch
}

pub struct PlaybackEngine {
    config: PlaybackConfig,
    sections: Vec<MusicalSection>,
    mapper: KeyMapper,

    compiled_events: Vec<KeyEvent>,
    event_index: usize,

    stopped: bool,
    paused: bool,
    was_paused: bool,
    pending_seek: Option<f64>,

    key_states: HashMap<String, KeyState>,
    active_pitches: HashSet<i32>,
    pedal_is_down: bool,

    key_net: HashMap<String, i32>,
    key_last_press: HashMap<String, KeyEvent>,
    pedal_net_down: bool,

    start_time: f64,
    total_paused_time: f64,
    last_pause_timestamp: f64,
    total_duration: f64,

    last_progress_emit_time: f64,
    progress_update_interval: f64,

    current_section_idx: i32,
}

impl PlaybackEngine {
    pub fn new(config: PlaybackConfig, sections: Vec<MusicalSection>) -> Self {
        let mapper = KeyMapper::new(config.use_88_key_layout);
        PlaybackEngine {
            config,
            sections,
            mapper,
            compiled_events: Vec::new(),
            event_index: 0,
            stopped: false,
            paused: false,
            was_paused: false,
            pending_seek: None,
            key_states: HashMap::new(),
            active_pitches: HashSet::new(),
            pedal_is_down: false,
            key_net: HashMap::new(),
            key_last_press: HashMap::new(),
            pedal_net_down: false,
            start_time: 0.0,
            total_paused_time: 0.0,
            last_pause_timestamp: 0.0,
            total_duration: 0.0,
            last_progress_emit_time: 0.0,
            progress_update_interval: 1.0 / 60.0,
            current_section_idx: -1,
        }
    }

    pub fn load_compiled_events(&mut self, events: Vec<KeyEvent>, total_duration: f64) {
        self.key_states.clear();
        for ev in &events {
            self.key_states
                .entry(ev.key_char.clone())
                .or_insert_with(|| KeyState::new(&ev.key_char));
        }
        self.compiled_events = events;
        self.total_duration = total_duration;
    }

    pub fn should_run_countdown(&self) -> bool {
        self.config.countdown
    }

    pub fn countdown_messages() -> Vec<String> {
        vec![
            "Get ready...".to_string(),
            "3...".to_string(),
            "2...".to_string(),
            "1...".to_string(),
        ]
    }

    pub fn start(&mut self, now: f64) {
        self.start_time = now;
        self.total_paused_time = 0.0;
        self.event_index = 0;
        self.last_progress_emit_time = now;
        self.current_section_idx = -1;
        self.was_paused = false;
        self.key_net.clear();
        self.key_last_press.clear();
        self.pedal_net_down = false;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn event_index(&self) -> usize {
        self.event_index
    }

    pub fn compiled_event_count(&self) -> usize {
        self.compiled_events.len()
    }

    pub fn stop(&mut self) -> Vec<PlayerEvent> {
        if self.stopped {
            return vec![];
        }
        self.stopped = true;
        self.paused = false;
        vec![PlayerEvent::Status("Stopping playback...".to_string())]
    }

    pub fn toggle_pause(&mut self, now: f64) -> Vec<PlayerEvent> {
        if self.paused {
            let pause_duration = now - self.last_pause_timestamp;
            self.total_paused_time += pause_duration;
            self.paused = false;
            vec![PlayerEvent::Status("Resuming...".to_string())]
        } else {
            self.last_pause_timestamp = now;
            self.paused = true;
            vec![PlayerEvent::Status("Paused.".to_string())]
        }
    }

    pub fn seek(&mut self, target_time: f64) {
        self.pending_seek = Some(target_time);
    }

    fn playback_time(&self, now: f64) -> f64 {
        (now - self.start_time) - self.total_paused_time
    }

    fn consume_pending_seek(&mut self, driver: &mut dyn KeyboardDriver, now: f64) -> Vec<PlayerEvent> {
        if let Some(target) = self.pending_seek.take() {
            self.apply_seek(driver, target, now)
        } else {
            vec![]
        }
    }

    fn apply_seek(&mut self, driver: &mut dyn KeyboardDriver, target_time: f64, now: f64) -> Vec<PlayerEvent> {
        let mut events = self.shutdown(driver);
        let times: Vec<f64> = self.compiled_events.iter().map(|e| e.time).collect();
        let new_idx = times.partition_point(|&t| t < target_time);
        self.event_index = new_idx;
        self.key_net.clear();
        self.key_last_press.clear();
        self.pedal_net_down = false;

        if self.paused {
            self.total_paused_time = 0.0;
            self.start_time = now - target_time;
            self.last_pause_timestamp = now;
        } else {
            self.start_time = now - target_time - self.total_paused_time;
        }
        self.last_progress_emit_time = now;
        events.push(PlayerEvent::Progress(target_time));
        events
    }

    pub fn tick(&mut self, driver: &mut dyn KeyboardDriver, now: f64) -> TickOutcome {
        let mut events = Vec::new();
        if self.stopped {
            return TickOutcome {
                events,
                suggested_sleep: 0.0,
            };
        }

        events.extend(self.consume_pending_seek(driver, now));

        if self.paused {
            if !self.was_paused {
                events.extend(self.shutdown(driver));
                self.was_paused = true;
            }
            return TickOutcome {
                events,
                suggested_sleep: 0.05,
            };
        }

        if self.was_paused {
            events.extend(self.sync_active_keys_at_resume(driver));
            self.was_paused = false;
        }

        let playback_time = self.playback_time(now);

        let next_sec_idx = self.current_section_idx + 1;
        if !self.sections.is_empty() && next_sec_idx >= 0 && (next_sec_idx as usize) < self.sections.len() {
            let sec = &self.sections[next_sec_idx as usize];
            if playback_time >= sec.start_time {
                self.current_section_idx = next_sec_idx;
                events.push(PlayerEvent::Section(next_sec_idx as usize));
            }
        }

        if self.event_index >= self.compiled_events.len() {
            if playback_time > self.total_duration + 0.1 {
                if !self.paused {
                    self.last_pause_timestamp = now;
                    self.paused = true;
                    events.extend(self.shutdown(driver));
                    events.push(PlayerEvent::AutoPaused);
                    events.push(PlayerEvent::Status("Playback finished. Paused.".to_string()));
                }
                return TickOutcome {
                    events,
                    suggested_sleep: 0.1,
                };
            }
            return TickOutcome {
                events,
                suggested_sleep: 0.001,
            };
        }

        let next_event_time = self.compiled_events[self.event_index].time;
        let suggested_sleep;

        if next_event_time <= playback_time {
            let mut batch = Vec::new();
            while self.event_index < self.compiled_events.len() {
                let e = &self.compiled_events[self.event_index];
                if e.time <= playback_time {
                    batch.push(e.clone());
                    self.event_index += 1;
                } else {
                    break;
                }
            }
            batch.sort_by_key(|e| e.priority);
            events.extend(self.execute_chord_event(driver, &batch, playback_time));
            suggested_sleep = 0.0;
        } else {
            let sleep_time = (next_event_time - playback_time - 0.001).min(self.progress_update_interval);
            suggested_sleep = sleep_time.max(0.0005);
        }

        if now - self.last_progress_emit_time >= self.progress_update_interval {
            events.push(PlayerEvent::Progress(playback_time));
            self.last_progress_emit_time = now;
        }

        TickOutcome {
            events,
            suggested_sleep,
        }
    }

    fn get_press_info_from_event(&self, event: &KeyEvent) -> (Vec<kb::Key>, String) {
        let Some(pitch) = event.pitch else {
            return (vec![], event.key_char.clone());
        };
        let Some(key_data) = self.mapper.get_key_data(pitch) else {
            return (vec![], event.key_char.clone());
        };
        let mut modifiers: Vec<kb::Key> = key_data
            .modifiers
            .iter()
            .map(|m| match m {
                Modifier::Ctrl => kb::Key::Ctrl,
                Modifier::Shift => kb::Key::Shift,
            })
            .collect();
        if self.config.use_velocity_accent && event.velocity.is_some() {
            modifiers.push(kb::Key::Alt);
        }
        (modifiers, key_data.key.to_string())
    }

    fn execute_chord_event(
        &mut self,
        driver: &mut dyn KeyboardDriver,
        batch: &[KeyEvent],
        _playback_time: f64,
    ) -> Vec<PlayerEvent> {
        let mut events = Vec::new();
        if self.stopped {
            return events;
        }

        let press_events: Vec<&KeyEvent> = batch.iter().filter(|e| e.action == "press").collect();
        let release_events: Vec<&KeyEvent> = batch.iter().filter(|e| e.action == "release").collect();
        let pedal_events: Vec<&KeyEvent> = batch.iter().filter(|e| e.action == "pedal").collect();

        let mut state_changed = false;

        for event in &pedal_events {
            let _physical = self.handle_pedal_event(driver, event);
            let new_pedal_state = event.key_char == "down";
            self.pedal_net_down = new_pedal_state;
            if new_pedal_state != self.pedal_net_down {
                events.push(PlayerEvent::Pedal(self.pedal_net_down));
            }
        }
        if !pedal_events.is_empty() {
            events.push(PlayerEvent::Pedal(self.pedal_net_down));
        }

        for event in &release_events {
            let key_char = event.key_char.clone();
            let net = {
                let count = self.key_net.entry(key_char.clone()).or_insert(0);
                *count -= 1;
                *count
            };
            if let Some(pitch) = event.pitch {
                self.active_pitches.remove(&pitch);
                state_changed = true;
            }
            if !self.key_states.contains_key(&key_char) {
                continue;
            }
            if net <= 0 {
                let base_key = symbol_to_base(&key_char);
                if let Some(state) = self.key_states.get_mut(&key_char) {
                    state.release();
                }
                let _ = driver.release(kb::Key::Char(base_key));
            }
        }

        for event in &press_events {
            let key_char = event.key_char.clone();
            *self.key_net.entry(key_char.clone()).or_insert(0) += 1;
            self.key_last_press.insert(key_char.clone(), (*event).clone());
            if let Some(pitch) = event.pitch {
                self.active_pitches.insert(pitch);
                state_changed = true;
            }
            if !self.key_states.contains_key(&key_char) || event.pitch.is_none() {
                continue;
            }

            let (modifiers, base_key) = self.get_press_info_from_event(event);
            let base_key_char = base_key.chars().next().unwrap_or(' ');
            let was_physically_down = self.key_states[&key_char].is_physically_down();
            if let Some(state) = self.key_states.get_mut(&key_char) {
                state.press();
            }

            let mut guard = kb::pressed(driver, &modifiers);
            if was_physically_down {
                let _ = guard.release(kb::Key::Char(base_key_char));
                std::thread::sleep(std::time::Duration::from_millis(1));
                let _ = guard.press(kb::Key::Char(base_key_char));
            } else {
                let _ = guard.press(kb::Key::Char(base_key_char));
            }
        }

        if state_changed {
            events.push(PlayerEvent::Visualizer(
                self.active_pitches.iter().copied().collect(),
            ));
        }

        events
    }

    fn handle_pedal_event(&mut self, driver: &mut dyn KeyboardDriver, event: &KeyEvent) -> String {
        if self.stopped {
            return "Stopped (no-op)".to_string();
        }
        match event.key_char.as_str() {
            "down" => {
                if self.pedal_is_down {
                    return "Already down (no-op)".to_string();
                }
                self.pedal_is_down = true;
                match driver.press(kb::Key::Space) {
                    Ok(()) => "Pressed Space".to_string(),
                    Err(e) => format!("FAILED: {e}"),
                }
            }
            "up" => {
                if !self.pedal_is_down {
                    return "Already up (no-op)".to_string();
                }
                self.pedal_is_down = false;
                match driver.release(kb::Key::Space) {
                    Ok(()) => "Released Space".to_string(),
                    Err(e) => format!("FAILED: {e}"),
                }
            }
            _ => "Unknown action".to_string(),
        }
    }

    fn sync_active_keys_at_resume(&mut self, driver: &mut dyn KeyboardDriver) -> Vec<PlayerEvent> {
        let mut events = Vec::new();
        let mut pitch_net: HashMap<i32, i32> = HashMap::new();

        let key_chars: Vec<String> = self
            .key_net
            .iter()
            .filter(|(k, &c)| c > 0 && self.key_states.contains_key(*k))
            .map(|(k, _)| k.clone())
            .collect();

        for key_char in key_chars {
            let Some(press_event) = self.key_last_press.get(&key_char).cloned() else {
                continue;
            };
            if let Some(pitch) = press_event.pitch {
                *pitch_net.entry(pitch).or_insert(0) += 1;
            }
            let (modifiers, base_key) = self.get_press_info_from_event(&press_event);
            let base_key_char = base_key.chars().next().unwrap_or(' ');
            if let Some(state) = self.key_states.get_mut(&key_char) {
                state.press();
            }
            let mut guard = kb::pressed(driver, &modifiers);
            let _ = guard.press(kb::Key::Char(base_key_char));
        }

        self.active_pitches = pitch_net
            .into_iter()
            .filter(|&(_, c)| c > 0)
            .map(|(p, _)| p)
            .collect();
        events.push(PlayerEvent::Visualizer(
            self.active_pitches.iter().copied().collect(),
        ));

        if self.pedal_net_down && !self.pedal_is_down {
            self.pedal_is_down = true;
            let _ = driver.press(kb::Key::Space);
        }

        events
    }

    pub fn shutdown(&mut self, driver: &mut dyn KeyboardDriver) -> Vec<PlayerEvent> {
        let events = Vec::new();
        let key_chars: Vec<String> = self.key_states.keys().cloned().collect();
        for key_char in key_chars {
            let is_active = self
                .key_states
                .get(&key_char)
                .map(|s| s.is_active)
                .unwrap_or(false);
            if is_active {
                let base_key = symbol_to_base(&key_char);
                let _ = driver.release(kb::Key::Char(base_key));
            }
            if let Some(state) = self.key_states.get_mut(&key_char) {
                state.release();
            }
        }
        if self.pedal_is_down {
            let _ = driver.release(kb::Key::Space);
            self.pedal_is_down = false;
        }
        for key in [kb::Key::Shift, kb::Key::Ctrl, kb::Key::Alt] {
            let _ = driver.release(key);
        }
        events
    }
}

#[cfg(test)]
mod tests {
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
        fn test_velocity_accent_adds_alt_modifier() {
            let mut cfg = config();
            cfg.use_velocity_accent = true;
            let engine = PlaybackEngine::new(cfg, vec![]);
            let ev = press(0.0, 2, "1", 36, 100);
            let (modifiers, _) = engine.get_press_info_from_event(&ev);
            assert!(modifiers.contains(&kb::Key::Alt));
        }

        #[test]
        fn test_velocity_accent_disabled_by_default() {
            let engine = PlaybackEngine::new(config(), vec![]);
            let ev = press(0.0, 2, "1", 36, 100);
            let (modifiers, _) = engine.get_press_info_from_event(&ev);
            assert!(!modifiers.contains(&kb::Key::Alt));
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
}
