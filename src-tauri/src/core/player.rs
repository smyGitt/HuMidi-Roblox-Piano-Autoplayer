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

    pub fn should_release_pedal_on_start(&self) -> bool {
        self.config.release_pedal_on_start
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

    pub fn release_pedal(&mut self, driver: &mut dyn KeyboardDriver) {
        let _ = driver.release(kb::Key::Space);
        self.pedal_is_down = false;
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

    pub fn current_time(&self, now: f64) -> f64 {
        if self.paused {
            self.playback_time(self.last_pause_timestamp)
        } else {
            self.playback_time(now)
        }
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
        let modifiers: Vec<kb::Key> = key_data
            .modifiers
            .iter()
            .map(|m| match m {
                Modifier::Ctrl => kb::Key::Ctrl,
                Modifier::Shift => kb::Key::Shift,
            })
            .collect();
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
mod tests;
