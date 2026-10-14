use super::{percentile, sorted_copy};
use crate::core::models::KeyEvent;

const SMOOTHING_FRAMES: usize = 5;
const PROMINENCE_WINDOW_SECONDS: f64 = 10.0;
const MIN_PROMINENCE: f32 = 0.15;
const LEVEL_FRACTION: f32 = 0.5;
const FALLBACK_VALLEY_PERCENTILE: f64 = 10.0;

struct Peak {
    index: usize,
    height: f32,
    valley: f32,
    start: usize,
    end: usize,
}

fn smoothed(preds: &[f32]) -> Vec<f32> {
    let half = SMOOTHING_FRAMES / 2;
    (0..preds.len())
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half + 1).min(preds.len());
            preds[lo..hi].iter().sum::<f32>() / SMOOTHING_FRAMES as f32
        })
        .collect()
}

fn first_max_index(values: &[f32]) -> usize {
    let mut best = 0;
    for (i, &v) in values.iter().enumerate() {
        if v > values[best] {
            best = i;
        }
    }
    best
}

fn peak_indices(preds: &[f32]) -> Vec<usize> {
    let smooth = smoothed(preds);
    let half = SMOOTHING_FRAMES / 2;
    let mut indices = Vec::new();
    let mut last = None;
    for i in 1..preds.len().saturating_sub(1) {
        let is_local_max = smooth[i] > smooth[i - 1] && smooth[i] >= smooth[i + 1] && preds[i] > 0.0;
        if !is_local_max {
            continue;
        }
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(preds.len());
        let k = lo + first_max_index(&preds[lo..hi]);
        if last != Some(k) {
            indices.push(k);
            last = Some(k);
        }
    }
    indices
}

fn lowest_active(values: &[f32]) -> Option<f32> {
    values
        .iter()
        .copied()
        .filter(|&p| p > 0.0)
        .fold(None, |lowest, p| Some(lowest.map_or(p, |m: f32| m.min(p))))
}

fn peak_at(preds: &[f32], index: usize, window: usize, fallback_valley: f32) -> Peak {
    let height = preds[index];
    let mut start = index.saturating_sub(window);
    if let Some(j) = preds[start..index].iter().rposition(|&p| p > height) {
        start += j + 1;
    }
    let mut end = (index + 1 + window).min(preds.len());
    if let Some(j) = preds[index + 1..end].iter().position(|&p| p > height) {
        end = index + 1 + j;
    }
    let valley = match (lowest_active(&preds[start..index]), lowest_active(&preds[index + 1..end])) {
        (Some(left), Some(right)) => left.max(right),
        (Some(only), None) | (None, Some(only)) => only,
        (None, None) => fallback_valley,
    };
    Peak { index, height, valley, start, end }
}

fn band(preds: &[f32], peak: &Peak) -> (usize, usize) {
    let level = peak.valley + LEVEL_FRACTION * (peak.height - peak.valley);
    let start = preds[peak.start..peak.index]
        .iter()
        .rposition(|&p| p < level)
        .map_or(peak.start, |j| peak.start + j + 1);
    let end = preds[peak.index + 1..peak.end]
        .iter()
        .position(|&p| p < level)
        .map_or(peak.end, |j| peak.index + 1 + j);
    (start, end)
}

fn events_from_mask(down: &[bool], fps: f64, max_time: f64) -> Vec<KeyEvent> {
    let mut events = Vec::new();
    let mut pedal_is_down = false;
    for (i, &is_down) in down.iter().enumerate() {
        if is_down == pedal_is_down {
            continue;
        }
        let time = i as f64 / fps;
        if is_down {
            events.push(KeyEvent::new(time, 1, "pedal", "down"));
        } else {
            events.push(KeyEvent::new(time, 0, "pedal", "up"));
        }
        pedal_is_down = is_down;
    }
    if pedal_is_down {
        events.push(KeyEvent::new(max_time, 0, "pedal", "up"));
    }
    events
}

pub fn peak_events(preds: &[f32], fps: f64, max_time: f64) -> Vec<KeyEvent> {
    let clean: Vec<f32> = preds.iter().map(|&p| if p.is_finite() { p } else { 0.0 }).collect();
    let active: Vec<f32> = clean.iter().copied().filter(|&p| p > 0.0).collect();
    let fallback_valley = percentile(&sorted_copy(&active), FALLBACK_VALLEY_PERCENTILE);
    let window = (PROMINENCE_WINDOW_SECONDS * fps) as usize;
    let mut down = vec![false; clean.len()];
    for index in peak_indices(&clean) {
        let peak = peak_at(&clean, index, window, fallback_valley);
        if peak.height - peak.valley < MIN_PROMINENCE {
            continue;
        }
        let (start, end) = band(&clean, &peak);
        for flag in &mut down[start..end] {
            *flag = true;
        }
    }
    if let Some(first) = down.first_mut() {
        *first = false;
    }
    events_from_mask(&down, fps, max_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pedal::FPS;

    const BASELINE: f32 = 0.1;
    const TOP: f32 = 0.9;
    const HALF_WIDTH: usize = 21;
    const LEAD_FRAMES: usize = 100;
    const TAIL_FRAMES: usize = 100;
    const MAX_TIME: f64 = 99.0;
    const LAST_FRAME_AT_OR_ABOVE_HALF_HEIGHT: usize = 10;
    const FIRST_FRAME_BELOW_HALF_HEIGHT: usize = 11;

    fn triangle(center: usize, half_width: usize, base: f32, top: f32) -> impl Fn(usize) -> f32 {
        move |i| {
            let distance = i.abs_diff(center);
            if distance >= half_width {
                base
            } else {
                top - (top - base) * distance as f32 / half_width as f32
            }
        }
    }

    fn baseline_with_peaks(len: usize, peaks: &[(usize, f32)]) -> Vec<f32> {
        (0..len)
            .map(|i| {
                peaks
                    .iter()
                    .map(|&(center, top)| triangle(center, HALF_WIDTH, BASELINE, top)(i))
                    .fold(BASELINE, f32::max)
            })
            .collect()
    }

    fn times(events: &[KeyEvent]) -> Vec<f64> {
        events.iter().map(|e| e.time).collect()
    }

    mod test_peak_events {
        use super::*;

        #[test]
        fn test_one_peak_gives_one_band_between_its_half_height_crossings() {
            let center = LEAD_FRAMES;
            let preds = baseline_with_peaks(LEAD_FRAMES + TAIL_FRAMES, &[(center, TOP)]);
            let events = peak_events(&preds, FPS, MAX_TIME);
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].key_char, "down");
            assert_eq!(events[1].key_char, "up");
            let expected_down = (center - LAST_FRAME_AT_OR_ABOVE_HALF_HEIGHT) as f64 / FPS;
            let expected_up = (center + FIRST_FRAME_BELOW_HALF_HEIGHT) as f64 / FPS;
            assert_eq!(times(&events), vec![expected_down, expected_up]);
        }

        #[test]
        fn test_a_rise_below_the_prominence_floor_is_ignored() {
            let small_top = BASELINE + MIN_PROMINENCE / 2.0;
            let preds = baseline_with_peaks(LEAD_FRAMES + TAIL_FRAMES, &[(LEAD_FRAMES, small_top)]);
            assert!(peak_events(&preds, FPS, MAX_TIME).is_empty());
        }

        #[test]
        fn test_two_separated_peaks_give_two_bands() {
            let gap = 3 * HALF_WIDTH;
            let second = LEAD_FRAMES + gap;
            let preds = baseline_with_peaks(second + TAIL_FRAMES, &[(LEAD_FRAMES, TOP), (second, TOP)]);
            let events = peak_events(&preds, FPS, MAX_TIME);
            let actions: Vec<&str> = events.iter().map(|e| e.key_char.as_str()).collect();
            assert_eq!(actions, vec!["down", "up", "down", "up"]);
        }

        #[test]
        fn test_a_smaller_peak_gets_a_proportionately_narrower_band() {
            let gap = 4 * HALF_WIDTH;
            let second = LEAD_FRAMES + gap;
            let small_top = BASELINE + (TOP - BASELINE) / 2.0;
            let preds = baseline_with_peaks(second + TAIL_FRAMES, &[(LEAD_FRAMES, TOP), (second, small_top)]);
            let events = peak_events(&preds, FPS, MAX_TIME);
            assert_eq!(events.len(), 4);
            let tall = events[1].time - events[0].time;
            let small = events[3].time - events[2].time;
            assert!(small > 0.0 && small < tall, "tall {tall}, small {small}");
        }

        #[test]
        fn test_a_band_still_open_at_the_end_closes_at_max_time() {
            let len = LEAD_FRAMES + 2;
            let preds = baseline_with_peaks(len, &[(len - 1, TOP)]);
            let events = peak_events(&preds, FPS, MAX_TIME);
            assert_eq!(events.last().unwrap().time, MAX_TIME);
            assert_eq!(events.last().unwrap().key_char, "up");
        }

        #[test]
        fn test_a_nan_frame_is_treated_as_silent_and_never_presses() {
            let mut preds = baseline_with_peaks(LEAD_FRAMES + TAIL_FRAMES, &[(LEAD_FRAMES, TOP)]);
            preds[0] = f32::NAN;
            let events = peak_events(&preds, FPS, MAX_TIME);
            assert_eq!(events.len(), 2);
            assert!(events[0].time > 0.0);
        }

        #[test]
        fn test_no_confidence_gives_no_events() {
            assert!(peak_events(&[], FPS, MAX_TIME).is_empty());
            assert!(peak_events(&vec![0.0; LEAD_FRAMES], FPS, MAX_TIME).is_empty());
        }
    }
}
