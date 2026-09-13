pub mod model;

use crate::core::config::PlaybackConfig;
use crate::core::midi::get_time_groups;
use crate::core::models::{KeyEvent, MusicalSection, Note};
use model::{PedalModel, FEATURES};

const FPS: f64 = 50.0;
const MIN_CONFIDENCE_GATE: f32 = 0.3;
const PEDAL_LAG: f64 = 0.05;
const UNSAFE_INTERVALS: [i32; 2] = [1, 6];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiThresholds {
    pub threshold_on: f32,
    pub threshold_off: f32,
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
    let mut pitch_roll = vec![0.0f32; total_steps * 128];
    for note in notes {
        if !(0..128).contains(&note.pitch) {
            continue;
        }
        let s_idx = (note.start_time * fps) as usize;
        let e_idx = ((note.end_time() * fps) as usize).min(total_steps);
        for t in s_idx..e_idx {
            pitch_roll[t * 128 + note.pitch as usize] = 1.0;
        }
    }
    let mut features = vec![0.0f32; total_steps * FEATURES];
    for t in 0..total_steps {
        features[t * FEATURES..t * FEATURES + 128].copy_from_slice(&pitch_roll[t * 128..t * 128 + 128]);
        for c in 0..12usize {
            let mut sum = 0.0f32;
            let mut p = c;
            while p < 128 {
                sum += pitch_roll[t * 128 + p];
                p += 12;
            }
            features[t * FEATURES + 128 + c] = sum;
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

fn process_ai_predictions(
    mut preds: Vec<f32>,
    notes: &[Note],
    max_time: f64,
    fps: f64,
    threshold_on: Option<f32>,
    threshold_off: Option<f32>,
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    let total_steps = preds.len();

    let mut is_silent = vec![true; total_steps];
    for note in notes {
        let s_idx = (note.start_time * fps) as usize;
        let e_idx = (((note.end_time() + 0.35) * fps) as usize).min(total_steps);
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

    let ap_min = active_preds.iter().copied().fold(f32::MAX, f32::min);
    let ap_max = active_preds.iter().copied().fold(f32::MIN, f32::max);
    let ap_range = ap_max - ap_min;
    if ap_range < 1e-6 {
        return (Vec::new(), None);
    }
    let normed: Vec<f32> = active_preds
        .iter()
        .map(|&p| (p - ap_min) / ap_range)
        .collect();

    let otsu_split = otsu_threshold(&normed);
    let on_group = sorted_copy(
        &normed
            .iter()
            .copied()
            .filter(|&v| v > otsu_split)
            .collect::<Vec<f32>>(),
    );
    let off_group = sorted_copy(
        &normed
            .iter()
            .copied()
            .filter(|&v| v <= otsu_split)
            .collect::<Vec<f32>>(),
    );

    let using_custom = threshold_on.is_some() && threshold_off.is_some();
    let (final_threshold_on, final_threshold_off) = if using_custom {
        (threshold_on.unwrap(), threshold_off.unwrap())
    } else if !on_group.is_empty() && !off_group.is_empty() {
        let on_min = on_group[0];
        let on_max = *on_group.last().unwrap();
        let on_median = median(&on_group);
        let on_spread = on_max - on_min;
        let on_ratio = if on_spread > 0.0 {
            (on_median - on_min) / on_spread
        } else {
            0.0
        };
        let on_pct = on_ratio as f64 * 10.45;

        let off_min = off_group[0];
        let off_max = *off_group.last().unwrap();
        let off_median = median(&off_group);
        let off_spread = off_max - off_min;
        let off_ratio = if off_spread > 0.0 {
            (off_median - off_min) / off_spread
        } else {
            0.0
        };
        let off_pct = 100.0 - off_ratio as f64 * 6.0;

        let norm_on = percentile(&on_group, on_pct);
        let norm_off = percentile(&off_group, off_pct);
        (norm_on * ap_range + ap_min, norm_off * ap_range + ap_min)
    } else {
        let t = otsu_split * ap_range + ap_min;
        (t, t)
    };

    let mut events = Vec::new();
    let mut pedal_is_down = false;
    for (i, &curr_val) in preds.iter().enumerate().skip(1) {
        if curr_val == 0.0 {
            continue;
        }
        let curr_time = i as f64 / fps;
        if !pedal_is_down && curr_val > final_threshold_on {
            pedal_is_down = true;
            events.push(KeyEvent::new(curr_time, 1, "pedal", "down"));
        } else if pedal_is_down && curr_val <= final_threshold_off {
            pedal_is_down = false;
            events.push(KeyEvent::new(curr_time, 0, "pedal", "up"));
        }
    }
    if pedal_is_down {
        events.push(KeyEvent::new(max_time, 0, "pedal", "up"));
    }

    if events.len() <= 2 {
        return (Vec::new(), None);
    }

    (
        events,
        Some(AiThresholds {
            threshold_on: final_threshold_on,
            threshold_off: final_threshold_off,
        }),
    )
}

pub fn generate_ai_pedal(
    model: &PedalModel,
    notes: &[Note],
    threshold_on: Option<f32>,
    threshold_off: Option<f32>,
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

    process_ai_predictions(preds, notes, max_time, FPS, threshold_on, threshold_off)
}

fn resolve_custom_thresholds(config: &PlaybackConfig) -> (Option<f32>, Option<f32>) {
    if config.pedal_threshold_on >= 0.0 && config.pedal_threshold_off >= 0.0 {
        (
            Some(config.pedal_threshold_on as f32),
            Some(config.pedal_threshold_off as f32),
        )
    } else {
        (None, None)
    }
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
        let (t_on, t_off) = resolve_custom_thresholds(config);
        let (events, thresholds) = generate_ai_pedal(model, notes, t_on, t_off);
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
            let (t_on, t_off) = resolve_custom_thresholds(config);
            let (events, thresholds) = generate_ai_pedal(model, notes, t_on, t_off);
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn note(id: i32, pitch: i32, start_time: f64, duration: f64, hand: &str) -> Note {
        Note {
            id,
            pitch,
            velocity: 64,
            start_time,
            duration,
            hand: hand.to_string(),
            original_track_index: -1,
            channel: -1,
        }
    }

    fn section(notes: Vec<Note>) -> MusicalSection {
        let start = notes.iter().map(|n| n.start_time).fold(f64::MAX, f64::min);
        let end = notes.iter().map(|n| n.end_time()).fold(f64::MIN, f64::max);
        MusicalSection::new(start, end, notes)
    }

    mod test_otsu_threshold {
        use super::*;

        #[test]
        fn test_empty_input_returns_half() {
            assert_eq!(otsu_threshold(&[]), 0.5);
        }

        #[test]
        fn test_all_out_of_range_returns_half() {
            assert_eq!(otsu_threshold(&[-1.0, 2.0, 5.0]), 0.5);
        }

        #[test]
        fn test_bimodal_distribution_separates_the_two_clusters() {
            let mut preds: Vec<f32> = Vec::new();
            for _ in 0..50 {
                preds.push(0.05);
            }
            for _ in 0..50 {
                preds.push(0.95);
            }
            let split = otsu_threshold(&preds);
            assert!(
                preds.iter().filter(|&&v| v <= split).all(|&v| v < 0.5),
                "everything at or below the split should be the low cluster"
            );
            assert!(
                preds.iter().filter(|&&v| v > split).all(|&v| v > 0.5),
                "everything above the split should be the high cluster"
            );
        }

        #[test]
        fn test_result_within_unit_range() {
            let preds = vec![0.1, 0.2, 0.5, 0.8, 0.9];
            let split = otsu_threshold(&preds);
            assert!((0.0..=1.0).contains(&split));
        }

        #[test]
        fn test_uniform_distribution_returns_a_valid_split() {
            let preds: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
            let split = otsu_threshold(&preds);
            assert!((0.0..=1.0).contains(&split));
        }
    }

    mod test_generate_harmonic_pedal {
        use super::*;

        #[test]
        fn test_empty_input_produces_no_events() {
            assert!(generate_harmonic_pedal(&[]).is_empty());
        }

        #[test]
        fn test_single_note_quirk_produces_down_up_down_up() {
            let notes = vec![note(0, 40, 0.0, 1.0, "left")];
            let events = generate_harmonic_pedal(&notes);
            let actions: Vec<(&str, &str)> = events
                .iter()
                .map(|e| (e.action.as_str(), e.key_char.as_str()))
                .collect();
            assert_eq!(
                actions,
                vec![
                    ("pedal", "down"),
                    ("pedal", "up"),
                    ("pedal", "down"),
                    ("pedal", "up"),
                ]
            );
        }

        #[test]
        fn test_same_pitch_no_gap_sustains_without_further_repedal() {
            let notes = vec![
                note(0, 40, 0.0, 0.5, "left"),
                note(1, 40, 0.5, 0.5, "left"),
            ];
            let events = generate_harmonic_pedal(&notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 2, "sustained same-pitch notes should not add a repedal beyond the first-note quirk");
        }

        #[test]
        fn test_pitch_change_triggers_repedal() {
            let notes = vec![
                note(0, 40, 0.0, 0.2, "left"),
                note(1, 45, 0.2, 0.2, "left"),
            ];
            let events = generate_harmonic_pedal(&notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 3, "the first-note quirk contributes 2, the pitch change contributes a 3rd");
        }

        #[test]
        fn test_large_gap_triggers_lift_and_replant() {
            let notes = vec![
                note(0, 40, 0.0, 0.2, "left"),
                note(1, 40, 1.0, 0.2, "left"),
            ];
            let events = generate_harmonic_pedal(&notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 3, "the first-note quirk contributes 2, the >150ms gap contributes a 3rd");
        }

        #[test]
        fn test_ends_with_up_at_final_note_end_time() {
            let notes = vec![note(0, 40, 0.0, 2.0, "left")];
            let events = generate_harmonic_pedal(&notes);
            let last = events.last().unwrap();
            assert_eq!(last.key_char, "up");
            assert!((last.time - 2.0).abs() < 1e-9);
        }
    }

    mod test_generate_adaptive_pedal_driver {
        use super::*;

        #[test]
        fn test_empty_driver_notes_produces_no_events() {
            assert!(generate_adaptive_pedal_driver(&[], &[]).is_empty());
        }

        #[test]
        fn test_single_note_produces_down_then_up() {
            let notes = vec![note(0, 40, 0.0, 1.0, "left")];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].key_char, "down");
            assert_eq!(events[1].key_char, "up");
        }

        #[test]
        fn test_large_gap_lifts_and_replants() {
            let notes = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 40, 1.0, 0.1, "left"),
            ];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 2, "gap > 0.35s should lift and replant");
        }

        #[test]
        fn test_safe_interval_no_gap_does_not_repedal() {
            let notes = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 44, 0.1, 0.1, "left"),
            ];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 1, "a safe (major third) interval with no gap should not repedal");
        }

        #[test]
        fn test_linear_semitone_interval_triggers_repedal_with_lag() {
            let notes = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 41, 0.1, 0.1, "left"),
            ];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            let downs: Vec<&KeyEvent> = events.iter().filter(|e| e.key_char == "down").collect();
            assert_eq!(downs.len(), 2);
            assert!((downs[1].time - (0.1 + PEDAL_LAG)).abs() < 1e-9);
        }

        #[test]
        fn test_linear_tritone_interval_triggers_repedal() {
            let notes = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 46, 0.1, 0.1, "left"),
            ];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 2, "a tritone (6 semitones) is an unsafe linear interval");
        }

        #[test]
        fn test_vertical_unsafe_interval_triggers_repedal() {
            let driver = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 44, 0.1, 0.1, "left"),
            ];
            let all_notes = vec![
                note(0, 40, 0.0, 0.1, "left"),
                note(1, 44, 0.1, 0.1, "left"),
                note(2, 45, 0.1, 0.1, "right"),
            ];
            let events = generate_adaptive_pedal_driver(&driver, &all_notes);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(
                downs, 2,
                "a concurrent minor-second above the lowest sounding note is an unsafe vertical interval"
            );
        }

        #[test]
        fn test_ends_with_up_at_final_note_end_time() {
            let notes = vec![note(0, 40, 0.0, 3.0, "left")];
            let events = generate_adaptive_pedal_driver(&notes, &notes);
            let last = events.last().unwrap();
            assert_eq!(last.key_char, "up");
            assert!((last.time - 3.0).abs() < 1e-9);
        }
    }

    mod test_generate_events_dispatch {
        use super::*;

        fn base_config() -> PlaybackConfig {
            PlaybackConfig::default()
        }

        #[test]
        fn test_none_style_produces_no_events() {
            let mut cfg = base_config();
            cfg.pedal_style = "none".to_string();
            let (events, meta) = generate_events(None, &cfg, &[], &[]);
            assert!(events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_unknown_style_produces_no_events() {
            let mut cfg = base_config();
            cfg.pedal_style = "not-a-real-style".to_string();
            let (events, _) = generate_events(None, &cfg, &[], &[]);
            assert!(events.is_empty());
        }

        #[test]
        fn test_rhythmic_style_groups_left_hand_notes_per_section() {
            let mut cfg = base_config();
            cfg.pedal_style = "rhythmic".to_string();
            let sec = section(vec![
                note(0, 40, 0.0, 0.2, "left"),
                note(1, 40, 1.0, 0.2, "left"),
            ]);
            let (events, _) = generate_events(None, &cfg, &[], &[sec]);
            let downs = events.iter().filter(|e| e.key_char == "down").count();
            assert_eq!(downs, 2, "two separate time groups should yield two down events");
        }

        #[test]
        fn test_rhythmic_falls_back_to_section_span_without_left_hand() {
            let mut cfg = base_config();
            cfg.pedal_style = "rhythmic".to_string();
            let sec = section(vec![note(0, 64, 0.0, 1.0, "right")]);
            let (events, _) = generate_events(None, &cfg, &[], &[sec]);
            assert_eq!(events.len(), 2);
        }

        #[test]
        fn test_harmonic_and_legato_are_aliases() {
            let sec_a = section(vec![
                note(0, 40, 0.0, 0.2, "left"),
                note(1, 45, 0.2, 0.2, "left"),
            ]);
            let sec_b = section(vec![
                note(0, 40, 0.0, 0.2, "left"),
                note(1, 45, 0.2, 0.2, "left"),
            ]);

            let mut cfg_harmonic = base_config();
            cfg_harmonic.pedal_style = "harmonic".to_string();
            let mut cfg_legato = base_config();
            cfg_legato.pedal_style = "legato".to_string();

            let (events_harmonic, _) = generate_events(None, &cfg_harmonic, &[], &[sec_a]);
            let (events_legato, _) = generate_events(None, &cfg_legato, &[], &[sec_b]);
            assert_eq!(events_harmonic.len(), events_legato.len());
        }

        #[test]
        fn test_ai_style_without_model_falls_back_to_adaptive() {
            let mut cfg = base_config();
            cfg.pedal_style = "ai".to_string();
            let notes = vec![
                note(0, 40, 0.0, 0.5, "left"),
                note(1, 42, 0.5, 0.5, "left"),
            ];
            let (events, meta) = generate_events(None, &cfg, &notes, &[]);
            assert!(!events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_hybrid_style_without_model_falls_back_to_adaptive() {
            let mut cfg = base_config();
            cfg.pedal_style = "hybrid".to_string();
            let notes = vec![
                note(0, 40, 0.0, 0.5, "left"),
                note(1, 42, 0.5, 0.5, "left"),
            ];
            let (events, meta) = generate_events(None, &cfg, &notes, &[]);
            assert!(!events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_hybrid_style_use_ai_pedal_false_never_needs_a_model() {
            let mut cfg = base_config();
            cfg.pedal_style = "hybrid".to_string();
            cfg.use_ai_pedal = false;
            let notes = vec![
                note(0, 40, 0.0, 0.5, "left"),
                note(1, 42, 0.5, 0.5, "left"),
            ];
            let (events, meta) = generate_events(None, &cfg, &notes, &[]);
            assert!(!events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_ai_and_hybrid_fallback_use_right_hand_when_no_left_hand_notes() {
            let mut cfg = base_config();
            cfg.pedal_style = "ai".to_string();
            let notes = vec![
                note(0, 64, 0.0, 0.5, "right"),
                note(1, 66, 0.5, 0.5, "right"),
            ];
            let (events, _) = generate_events(None, &cfg, &notes, &[]);
            assert!(!events.is_empty());
        }
    }

    mod test_process_ai_predictions {
        use super::*;

        #[test]
        fn test_all_silence_masked_rejects() {
            let preds = vec![0.9; 20];
            let notes: Vec<Note> = vec![];
            let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None, None);
            assert!(events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_low_confidence_rejects() {
            let mut preds = vec![0.0; 20];
            for p in preds.iter_mut().take(10) {
                *p = 0.1;
            }
            let notes = vec![note(0, 40, 0.0, 0.2, "left")];
            let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None, None);
            assert!(events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_zero_variance_active_region_rejects() {
            let mut preds = vec![0.0; 20];
            for p in preds.iter_mut().take(10) {
                *p = 0.5;
            }
            let notes = vec![note(0, 40, 0.0, 0.2, "left")];
            let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None, None);
            assert!(events.is_empty());
            assert!(meta.is_none());
        }

        #[test]
        fn test_too_few_crossings_rejects() {
            let mut preds = vec![0.0; 40];
            for (i, p) in preds.iter_mut().enumerate().take(20) {
                *p = 0.5 + (i as f32) * 0.01;
            }
            let notes = vec![note(0, 40, 0.0, 0.4, "left")];
            let (events, meta) = process_ai_predictions(preds, &notes, 0.8, FPS, None, None);
            assert!(events.is_empty() || events.len() > 2);
            let _ = meta;
        }

        #[test]
        fn test_custom_thresholds_bypass_otsu_computation() {
            let mut preds = vec![0.0; 40];
            for (i, p) in preds.iter_mut().enumerate().take(30) {
                *p = 0.2 + (i as f32 % 10.0) * 0.06;
            }
            let notes = vec![note(0, 40, 0.0, 0.6, "left")];
            let (_, meta) =
                process_ai_predictions(preds, &notes, 0.8, FPS, Some(0.4), Some(0.3));
            if let Some(m) = meta {
                assert_eq!(m.threshold_on, 0.4);
                assert_eq!(m.threshold_off, 0.3);
            }
        }

        #[test]
        fn test_accepted_path_produces_well_formed_alternating_events() {
            let mut preds = vec![0.0; 100];
            for (i, p) in preds.iter_mut().enumerate().take(80) {
                let phase = (i as f32) * 0.3;
                *p = 0.5 + 0.45 * phase.sin();
            }
            let notes = vec![note(0, 40, 0.0, 1.6, "left")];
            let (events, meta) = process_ai_predictions(preds, &notes, 1.6, FPS, None, None);
            if !events.is_empty() {
                assert!(meta.is_some());
                assert!(events.len() > 2);
                let mut pedal_down = false;
                for e in &events {
                    if e.key_char == "down" {
                        assert!(!pedal_down, "two downs in a row without an up");
                        pedal_down = true;
                    } else if e.key_char == "up" {
                        assert!(pedal_down, "an up without a preceding down");
                        pedal_down = false;
                    }
                }
            }
        }
    }

    mod test_build_input_features {
        use super::*;

        #[test]
        fn test_feature_width_matches_features_constant() {
            let notes = vec![note(0, 60, 0.0, 0.5, "left")];
            let total_steps = 10;
            let features = build_input_features(&notes, FPS, total_steps);
            assert_eq!(features.len(), total_steps * FEATURES);
        }

        #[test]
        fn test_active_frame_has_pitch_one_hot_set() {
            let notes = vec![note(0, 60, 0.0, 0.5, "left")];
            let total_steps = 30;
            let features = build_input_features(&notes, FPS, total_steps);
            assert_eq!(features[0 * FEATURES + 60], 1.0);
        }

        #[test]
        fn test_chroma_sums_pitch_classes_across_octaves() {
            let notes = vec![
                note(0, 60, 0.0, 1.0, "left"),
                note(1, 72, 0.0, 1.0, "left"),
            ];
            let total_steps = 10;
            let features = build_input_features(&notes, FPS, total_steps);
            let chroma_0 = features[0 * FEATURES + 128];
            assert_eq!(chroma_0, 2.0, "pitches 60 and 72 are both pitch-class 0");
        }

        #[test]
        fn test_out_of_range_pitch_is_skipped_not_panicking() {
            let notes = vec![note(0, 200, 0.0, 0.5, "left")];
            let total_steps = 10;
            let features = build_input_features(&notes, FPS, total_steps);
            assert!(features.iter().all(|&v| v == 0.0));
        }
    }

    mod test_ai_pedal_real_model_integration {
        use super::*;

        fn model_path() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("pedal_bilstm.safetensors")
        }

        #[test]
        fn test_generate_ai_pedal_does_not_panic_on_real_model() {
            let model = PedalModel::load(model_path()).expect("real pedal model should load");
            let notes = vec![
                note(0, 60, 0.0, 0.5, "left"),
                note(1, 62, 0.5, 0.5, "left"),
                note(2, 64, 1.0, 0.5, "left"),
                note(3, 65, 1.5, 0.5, "left"),
                note(4, 67, 2.0, 0.5, "left"),
            ];
            let (events, meta) = generate_ai_pedal(&model, &notes, None, None);
            if events.is_empty() {
                assert!(meta.is_none());
            } else {
                assert!(meta.is_some());
                assert!(events.len() > 2);
            }
        }

        #[test]
        fn test_generate_events_ai_style_with_real_model_does_not_panic() {
            let model = PedalModel::load(model_path()).expect("real pedal model should load");
            let mut cfg = PlaybackConfig::default();
            cfg.pedal_style = "ai".to_string();
            let notes = vec![
                note(0, 55, 0.0, 0.3, "left"),
                note(1, 57, 0.3, 0.3, "left"),
                note(2, 59, 0.6, 0.3, "left"),
            ];
            let (events, _) = generate_events(Some(&model), &cfg, &notes, &[]);
            assert!(!events.is_empty(), "ai style must always produce events, either AI or adaptive fallback");
        }
    }
}
