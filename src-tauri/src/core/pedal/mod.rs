pub mod auto_curve;
pub mod curve;
pub mod legacy;
pub mod model;
pub mod peak;

use crate::core::config::PlaybackConfig;
use crate::core::midi::get_time_groups;
use crate::core::models::{KeyEvent, MusicalSection, Note};
use auto_curve::auto_curves;
use curve::ThresholdCurves;
use model::{PedalModel, FEATURES};

pub const FPS: f64 = 50.0;
const MIN_CONFIDENCE_GATE: f32 = 0.3;
const PEDAL_LAG: f64 = 0.05;
const SILENCE_TAIL_SECONDS: f64 = 0.35;
const MIN_EVENTS_EXCLUSIVE: usize = 2;
const UNSAFE_INTERVALS: [i32; 2] = [1, 6];
const PITCHES: usize = 128;
const PITCH_CLASSES: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct AiThresholds {
    pub curves: ThresholdCurves,
    pub confidence: Vec<f32>,
}

pub fn otsu_threshold(preds: &[f32]) -> f32 {
    const NUM_BINS: usize = 256;
    let mut counts = [0u32; NUM_BINS];
    for &v in preds {
        if !(0.0..=1.0).contains(&v) {
            continue;
        }
        let bin = ((v * NUM_BINS as f32) as usize).min(NUM_BINS - 1);
        counts[bin] += 1;
    }
    let total: u32 = counts.iter().sum();
    if total == 0 {
        return 0.5;
    }

    let bin_edges: Vec<f32> = (0..=NUM_BINS).map(|i| i as f32 / NUM_BINS as f32).collect();
    let bin_centers: Vec<f32> = (0..NUM_BINS)
        .map(|i| (bin_edges[i] + bin_edges[i + 1]) / 2.0)
        .collect();

    let p: Vec<f64> = counts.iter().map(|&c| c as f64 / total as f64).collect();
    let mut omega = vec![0.0f64; NUM_BINS];
    let mut mu = vec![0.0f64; NUM_BINS];
    let mut running_omega = 0.0;
    let mut running_mu = 0.0;
    for i in 0..NUM_BINS {
        running_omega += p[i];
        running_mu += p[i] * bin_centers[i] as f64;
        omega[i] = running_omega;
        mu[i] = running_mu;
    }
    let mu_total = mu[NUM_BINS - 1];

    let mut best_idx = 0usize;
    let mut best_val = f64::MIN;
    for i in 0..NUM_BINS - 1 {
        let denom = omega[i] * (1.0 - omega[i]);
        let sigma_b_sq = if denom > 1e-12 {
            (mu_total * omega[i] - mu[i]).powi(2) / denom
        } else {
            0.0
        };
        if sigma_b_sq > best_val {
            best_val = sigma_b_sq;
            best_idx = i;
        }
    }
    bin_edges[best_idx + 1]
}

pub fn generate_harmonic_pedal(bass_notes: &[Note]) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    if bass_notes.is_empty() {
        return events;
    }
    let mut current_bass_pitch: i32 = -1;
    let mut has_gap = false;
    let mut prev_end = 0.0;
    for (i, note) in bass_notes.iter().enumerate() {
        let is_new_harmony = note.pitch != current_bass_pitch;
        if i == 0 {
            events.push(KeyEvent::new(note.start_time, 1, "pedal", "down"));
        } else {
            prev_end = bass_notes[i - 1].end_time();
            has_gap = (note.start_time - prev_end) > 0.15;
        }
        if i > 0 && has_gap {
            events.push(KeyEvent::new(prev_end, 0, "pedal", "up"));
            events.push(KeyEvent::new(note.start_time, 1, "pedal", "down"));
        } else if is_new_harmony {
            events.push(KeyEvent::new(note.start_time, 0, "pedal", "up"));
            events.push(KeyEvent::new(note.start_time, 1, "pedal", "down"));
        }
        current_bass_pitch = note.pitch;
    }
    let final_end = bass_notes
        .iter()
        .map(|n| n.end_time())
        .fold(f64::MIN, f64::max);
    events.push(KeyEvent::new(final_end, 0, "pedal", "up"));
    events
}

pub fn generate_adaptive_pedal_driver(driver_notes: &[Note], all_notes: &[Note]) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    if driver_notes.is_empty() {
        return events;
    }
    let all_note_times: Vec<f64> = all_notes.iter().map(|n| n.start_time).collect();

    for i in 0..driver_notes.len() {
        let curr = &driver_notes[i];
        let next_n = driver_notes.get(i + 1);

        if i == 0 {
            events.push(KeyEvent::new(curr.start_time, 1, "pedal", "down"));
        }

        let gap = next_n
            .map(|n| n.start_time - curr.end_time())
            .unwrap_or(0.0);

        if gap > 0.35 {
            events.push(KeyEvent::new(curr.end_time(), 0, "pedal", "up"));
            if let Some(next_n) = next_n {
                events.push(KeyEvent::new(next_n.start_time, 1, "pedal", "down"));
            }
        } else if let Some(next_n) = next_n {
            let mut should_repedal = false;
            let linear_interval = (next_n.pitch - curr.pitch).abs() % 12;
            if UNSAFE_INTERVALS.contains(&linear_interval) {
                should_repedal = true;
            }
            if !should_repedal {
                let lo = all_note_times.partition_point(|&t| t < next_n.start_time - 0.05);
                let hi = all_note_times.partition_point(|&t| t <= next_n.start_time + 0.05);
                let concurrent = &all_notes[lo..hi];
                if !concurrent.is_empty() {
                    let lowest_pitch = concurrent.iter().map(|n| n.pitch).min().unwrap();
                    for n in concurrent {
                        let vertical_interval = (n.pitch - lowest_pitch).abs() % 12;
                        if UNSAFE_INTERVALS.contains(&vertical_interval) {
                            should_repedal = true;
                            break;
                        }
                    }
                }
            }
            if should_repedal {
                events.push(KeyEvent::new(next_n.start_time, 0, "pedal", "up"));
                events.push(KeyEvent::new(next_n.start_time + PEDAL_LAG, 1, "pedal", "down"));
            }
        }
    }

    let final_end = driver_notes
        .iter()
        .map(|n| n.end_time())
        .fold(f64::MIN, f64::max);
    events.push(KeyEvent::new(final_end, 0, "pedal", "up"));
    events
}

fn rhythmic_strategy(sections: &[MusicalSection]) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    for section in sections {
        let mut lh_notes: Vec<Note> = section
            .notes
            .iter()
            .filter(|n| n.hand == "left")
            .cloned()
            .collect();
        lh_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        if lh_notes.is_empty() {
            let start = section.notes[0].start_time;
            let end = section
                .notes
                .iter()
                .map(|n| n.end_time())
                .fold(f64::MIN, f64::max);
            events.push(KeyEvent::new(start, 1, "pedal", "down"));
            events.push(KeyEvent::new(end, 0, "pedal", "up"));
            continue;
        }
        for group in get_time_groups(&lh_notes) {
            let start = group[0].start_time;
            let end = group.iter().map(|n| n.end_time()).fold(f64::MIN, f64::max);
            events.push(KeyEvent::new(start, 1, "pedal", "down"));
            events.push(KeyEvent::new(end, 0, "pedal", "up"));
        }
    }
    events
}

fn harmonic_strategy(sections: &[MusicalSection]) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    for section in sections {
        let mut lh_notes: Vec<Note> = section
            .notes
            .iter()
            .filter(|n| n.hand == "left")
            .cloned()
            .collect();
        lh_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        if lh_notes.is_empty() {
            let start = section.notes[0].start_time;
            let end = section
                .notes
                .iter()
                .map(|n| n.end_time())
                .fold(f64::MIN, f64::max);
            events.push(KeyEvent::new(start, 1, "pedal", "down"));
            events.push(KeyEvent::new(end, 0, "pedal", "up"));
            continue;
        }
        events.extend(generate_harmonic_pedal(&lh_notes));
    }
    events
}

fn build_input_features(notes: &[Note], fps: f64, total_steps: usize) -> Vec<f32> {
    let mut pitch_roll = vec![false; total_steps * PITCHES];
    for note in notes {
        if !(0..PITCHES as i32).contains(&note.pitch) {
            continue;
        }
        let s_idx = (note.start_time * fps) as usize;
        let e_idx = ((note.end_time() * fps) as usize).min(total_steps);
        for t in s_idx..e_idx {
            pitch_roll[t * PITCHES + note.pitch as usize] = true;
        }
    }
    let mut features = vec![0.0f32; total_steps * FEATURES];
    for t in 0..total_steps {
        for p in 0..PITCHES {
            let starts_here = pitch_roll[t * PITCHES + p] && (t == 0 || !pitch_roll[(t - 1) * PITCHES + p]);
            if starts_here {
                features[t * FEATURES + p] = 1.0;
                features[t * FEATURES + PITCHES + p % PITCH_CLASSES] += 1.0;
            }
        }
    }
    features
}

fn sorted_copy(values: &[f32]) -> Vec<f32> {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

fn median(sorted_data: &[f32]) -> f32 {
    let n = sorted_data.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        sorted_data[n / 2]
    } else {
        (sorted_data[n / 2 - 1] + sorted_data[n / 2]) / 2.0
    }
}

fn percentile(sorted_data: &[f32], pct: f64) -> f32 {
    let n = sorted_data.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted_data[0];
    }
    let pct = pct.clamp(0.0, 100.0);
    let idx = (n - 1) as f64 * pct / 100.0;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted_data[lo]
    } else {
        let frac = (idx - lo as f64) as f32;
        sorted_data[lo] + (sorted_data[hi] - sorted_data[lo]) * frac
    }
}

fn curve_events(preds: &[f32], curves: &ThresholdCurves, fps: f64, max_time: f64) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    let mut pedal_is_down = false;
    for (i, &curr_val) in preds.iter().enumerate().skip(1) {
        if curr_val == 0.0 {
            continue;
        }
        let curr_time = i as f64 / fps;
        let Some((threshold_on, threshold_off)) = curves.at(curr_time) else {
            continue;
        };
        let value = f64::from(curr_val);
        if !pedal_is_down && value > threshold_on {
            pedal_is_down = true;
            events.push(KeyEvent::new(curr_time, 1, "pedal", "down"));
        } else if pedal_is_down && value <= threshold_off {
            pedal_is_down = false;
            events.push(KeyEvent::new(curr_time, 0, "pedal", "up"));
        }
    }
    if pedal_is_down {
        events.push(KeyEvent::new(max_time, 0, "pedal", "up"));
    }
    events
}

fn process_ai_predictions(
    mut preds: Vec<f32>,
    notes: &[Note],
    max_time: f64,
    fps: f64,
    custom: Option<&ThresholdCurves>,
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    let total_steps = preds.len();

    let mut is_silent = vec![true; total_steps];
    for note in notes {
        let s_idx = (note.start_time * fps) as usize;
        let e_idx = (((note.end_time() + SILENCE_TAIL_SECONDS) * fps) as usize).min(total_steps);
        for t in s_idx..e_idx {
            is_silent[t] = false;
        }
    }
    for t in 0..total_steps {
        if is_silent[t] {
            preds[t] = 0.0;
        }
    }

    let active_preds: Vec<f32> = preds.iter().copied().filter(|&p| p > 0.0).collect();
    if active_preds.is_empty() {
        return (Vec::new(), None);
    }
    let max_active = active_preds.iter().copied().fold(f32::MIN, f32::max);
    if max_active < MIN_CONFIDENCE_GATE {
        return (Vec::new(), None);
    }

    let Some(auto) = auto_curves(&preds, fps) else {
        return (Vec::new(), None);
    };
    let (curves, events) = match custom {
        Some(c) => (c.clone(), curve_events(&preds, c, fps, max_time)),
        None => {
            let events = peak::peak_events(&preds, fps, max_time);
            (auto, events)
        }
    };

    if events.len() <= MIN_EVENTS_EXCLUSIVE {
        return (Vec::new(), None);
    }

    (
        events,
        Some(AiThresholds {
            curves,
            confidence: preds,
        }),
    )
}

pub fn generate_ai_pedal(
    model: &PedalModel,
    notes: &[Note],
    custom: Option<&ThresholdCurves>,
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    if notes.is_empty() {
        return (Vec::new(), None);
    }

    let max_time = notes
        .iter()
        .map(|n| n.end_time())
        .fold(f64::MIN, f64::max);
    let total_steps = (max_time * FPS).ceil() as usize + 1;

    let features = build_input_features(notes, FPS, total_steps);

    let preds = match model.forward(&features, total_steps) {
        Ok(p) => p,
        Err(_) => return (Vec::new(), None),
    };

    process_ai_predictions(preds, notes, max_time, FPS, custom)
}

fn resolve_custom_curves(config: &PlaybackConfig) -> Option<ThresholdCurves> {
    ThresholdCurves {
        on: config.pedal_curve_on.clone(),
        off: config.pedal_curve_off.clone(),
    }
    .sanitized()
}

fn adaptive_fallback(notes: &[Note]) -> Vec<KeyEvent> {
    let mut bass_notes: Vec<Note> = notes.iter().filter(|n| n.hand == "left").cloned().collect();
    bass_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    if bass_notes.is_empty() {
        let mut treble_notes: Vec<Note> =
            notes.iter().filter(|n| n.hand == "right").cloned().collect();
        treble_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        return generate_adaptive_pedal_driver(&treble_notes, notes);
    }
    generate_adaptive_pedal_driver(&bass_notes, notes)
}

fn ai_strategy(
    model: Option<&PedalModel>,
    config: &PlaybackConfig,
    notes: &[Note],
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    if let Some(model) = model {
        let custom = resolve_custom_curves(config);
        let (events, thresholds) = generate_ai_pedal(model, notes, custom.as_ref());
        if !events.is_empty() {
            return (events, thresholds);
        }
    }
    (adaptive_fallback(notes), None)
}

fn hybrid_strategy(
    model: Option<&PedalModel>,
    config: &PlaybackConfig,
    notes: &[Note],
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    if config.use_ai_pedal {
        if let Some(model) = model {
            let custom = resolve_custom_curves(config);
            let (events, thresholds) = generate_ai_pedal(model, notes, custom.as_ref());
            if !events.is_empty() {
                return (events, thresholds);
            }
        }
    }
    (adaptive_fallback(notes), None)
}

pub fn generate_events(
    model: Option<&PedalModel>,
    config: &PlaybackConfig,
    notes: &[Note],
    sections: &[MusicalSection],
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    match config.pedal_style.as_str() {
        "none" => (Vec::new(), None),
        "rhythmic" => (rhythmic_strategy(sections), None),
        "harmonic" | "legato" => (harmonic_strategy(sections), None),
        "ai" => ai_strategy(model, config, notes),
        "hybrid" => hybrid_strategy(model, config, notes),
        _ => (Vec::new(), None),
    }
}

#[cfg(test)]
mod tests;
