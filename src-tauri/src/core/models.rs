#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub id: i32,
    pub pitch: i32,
    pub velocity: i32,
    pub start_time: f64,
    pub duration: f64,
    pub hand: String,
    pub original_track_index: i32,
    pub channel: i32,
}

impl Note {
    pub fn new(id: i32, pitch: i32, velocity: i32, start_time: f64, duration: f64) -> Self {
        Note {
            id,
            pitch,
            velocity,
            start_time,
            duration,
            hand: "unknown".to_string(),
            original_track_index: -1,
            channel: -1,
        }
    }

    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
}

#[derive(Debug, Clone)]
pub struct MidiTrack {
    pub index: i32,
    pub name: String,
    pub program_change: i32,
    pub is_drum: bool,
    pub notes: Vec<Note>,
}

impl MidiTrack {
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    pub fn instrument_name(&self) -> String {
        if self.is_drum {
            return "Drums/Percussion".to_string();
        }
        match self.program_change {
            0..=7 => "Piano".to_string(),
            8..=15 => "Chromatic Perc".to_string(),
            16..=23 => "Organ".to_string(),
            24..=31 => "Guitar".to_string(),
            32..=39 => "Bass".to_string(),
            40..=47 => "Strings".to_string(),
            48..=55 => "Ensemble".to_string(),
            n => format!("Instrument {n}"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyEvent {
    pub time: f64,
    pub priority: i32,
    pub action: String,
    pub key_char: String,
    pub pitch: Option<i32>,
    pub velocity: Option<i32>,
}

impl KeyEvent {
    pub fn new(time: f64, priority: i32, action: &str, key_char: &str) -> Self {
        KeyEvent {
            time,
            priority,
            action: action.to_string(),
            key_char: key_char.to_string(),
            pitch: None,
            velocity: None,
        }
    }
}

impl PartialEq for KeyEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time && self.priority == other.priority
    }
}
impl Eq for KeyEvent {}

impl PartialOrd for KeyEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for KeyEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .partial_cmp(&other.time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(self.priority.cmp(&other.priority))
    }
}

#[derive(Debug, Clone)]
pub struct MusicalSection {
    pub start_time: f64,
    pub end_time: f64,
    pub notes: Vec<Note>,
    pub articulation_label: String,
    pub pace_label: String,
    pub start_beat: f64,
    pub end_beat: f64,
}

impl MusicalSection {
    pub fn new(start_time: f64, end_time: f64, notes: Vec<Note>) -> Self {
        MusicalSection {
            start_time,
            end_time,
            notes,
            articulation_label: "unknown".to_string(),
            pace_label: "normal".to_string(),
            start_beat: 0.0,
            end_beat: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyState {
    pub key_char: String,
    pub is_active: bool,
    pub is_sustained: bool,
}

impl KeyState {
    pub fn new(key_char: &str) -> Self {
        KeyState {
            key_char: key_char.to_string(),
            is_active: false,
            is_sustained: false,
        }
    }

    pub fn press(&mut self) {
        self.is_active = true;
    }

    pub fn release(&mut self) {
        self.is_active = false;
        self.is_sustained = false;
    }

    pub fn is_physically_down(&self) -> bool {
        self.is_active
    }
}

#[cfg(test)]
mod tests;
