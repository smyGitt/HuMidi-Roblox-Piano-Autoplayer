use super::curve::{ThresholdCurves, ThresholdNode};
use super::{median, otsu_threshold, percentile, sorted_copy};

const WINDOW_HALF_SECONDS: f64 = 10.0;
const NODE_SPACING_SECONDS: f64 = 5.0;
const MIN_WINDOW_ACTIVE_SECONDS: f64 = 2.0;
const MIN_WINDOW_RANGE_RATIO: f32 = 0.1;
const DEGENERATE_RANGE: f32 = 1e-6;
const ON_PERCENTILE_SCALE: f64 = 10.45;
const OFF_PERCENTILE_SCALE: f64 = 6.0;
const FULL_PERCENT: f64 = 100.0;

pub fn estimate_thresholds(active: &[f32]) -> Option<(f32, f32)> {
    if active.is_empty() {
        return None;
    }
    let min = active.iter().copied().fold(f32::MAX, f32::min);
    let max = active.iter().copied().fold(f32::MIN, f32::max);
    let range = max - min;
    if range < DEGENERATE_RANGE {
        return None;
    }
    let normed: Vec<f32> = active.iter().map(|&p| (p - min) / range).collect();

    let split = otsu_threshold(&normed);
    let on_group = sorted_copy(&normed.iter().copied().filter(|&v| v > split).collect::<Vec<f32>>());
    let off_group = sorted_copy(&normed.iter().copied().filter(|&v| v <= split).collect::<Vec<f32>>());

    if on_group.is_empty() || off_group.is_empty() {
        let t = split * range + min;
        return Some((t, t));
    }

    let on_pct = group_ratio(&on_group) as f64 * ON_PERCENTILE_SCALE;
    let off_pct = FULL_PERCENT - group_ratio(&off_group) as f64 * OFF_PERCENTILE_SCALE;
    let norm_on = percentile(&on_group, on_pct);
    let norm_off = percentile(&off_group, off_pct);
    Some((norm_on * range + min, norm_off * range + min))
}

fn group_ratio(sorted_group: &[f32]) -> f32 {
    let min = sorted_group[0];
    let spread = sorted_group[sorted_group.len() - 1] - min;
    if spread > 0.0 {
        (median(sorted_group) - min) / spread
    } else {
        0.0
    }
}

fn active_range(active: &[f32]) -> f32 {
    let min = active.iter().copied().fold(f32::MAX, f32::min);
    let max = active.iter().copied().fold(f32::MIN, f32::max);
    max - min
}

fn window_thresholds(
    preds: &[f32],
    center_frame: usize,
    half_frames: usize,
    min_active_frames: usize,
    min_range: f32,
) -> Option<(f32, f32)> {
    let start = center_frame.saturating_sub(half_frames);
    let end = (center_frame + half_frames + 1).min(preds.len());
    let active: Vec<f32> = preds[start..end].iter().copied().filter(|&p| p > 0.0).collect();
    if active.len() < min_active_frames || active_range(&active) < min_range {
        return None;
    }
    estimate_thresholds(&active)
}

fn fill_gaps(values: Vec<Option<(f32, f32)>>, global: (f32, f32)) -> Vec<(f32, f32)> {
    let valid: Vec<usize> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_some())
        .map(|(i, _)| i)
        .collect();
    if valid.is_empty() {
        return vec![global; values.len()];
    }
    (0..values.len())
        .map(|i| {
            if let Some(v) = values[i] {
                return v;
            }
            let prev = valid.iter().rev().find(|&&p| p < i).copied();
            let next = valid.iter().find(|&&n| n > i).copied();
            match (prev, next) {
                (Some(p), Some(n)) => {
                    let (pv, nv) = (values[p].unwrap(), values[n].unwrap());
                    let t = (i - p) as f32 / (n - p) as f32;
                    (pv.0 + (nv.0 - pv.0) * t, pv.1 + (nv.1 - pv.1) * t)
                }
                (Some(p), None) => values[p].unwrap(),
                (None, Some(n)) => values[n].unwrap(),
                (None, None) => global,
            }
        })
        .collect()
}

pub fn auto_curves(preds: &[f32], fps: f64) -> Option<ThresholdCurves> {
    let all_active: Vec<f32> = preds.iter().copied().filter(|&p| p > 0.0).collect();
    let global = estimate_thresholds(&all_active)?;
    let min_range = active_range(&all_active) * MIN_WINDOW_RANGE_RATIO;

    let half_frames = (WINDOW_HALF_SECONDS * fps) as usize;
    let min_active_frames = (MIN_WINDOW_ACTIVE_SECONDS * fps) as usize;
    let total_seconds = preds.len() as f64 / fps;
    let node_count = (total_seconds / NODE_SPACING_SECONDS).floor() as usize + 1;

    let raw: Vec<Option<(f32, f32)>> = (0..node_count)
        .map(|k| {
            let center = (k as f64 * NODE_SPACING_SECONDS * fps).round() as usize;
            window_thresholds(preds, center, half_frames, min_active_frames, min_range)
        })
        .collect();
    let filled = fill_gaps(raw, global);

    let mut on = Vec::with_capacity(node_count);
    let mut off = Vec::with_capacity(node_count);
    for (k, (on_value, off_value)) in filled.into_iter().enumerate() {
        let time = k as f64 * NODE_SPACING_SECONDS;
        on.push(ThresholdNode {
            time,
            value: on_value as f64,
        });
        off.push(ThresholdNode {
            time,
            value: off_value as f64,
        });
    }
    ThresholdCurves { on, off }.sanitized()
}

#[cfg(test)]
mod tests;
