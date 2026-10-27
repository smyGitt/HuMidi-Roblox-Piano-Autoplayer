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
mod tests;
