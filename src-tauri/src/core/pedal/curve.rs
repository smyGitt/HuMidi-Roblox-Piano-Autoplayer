use serde::{Deserialize, Serialize};

const MIN_VALUE: f64 = 0.0;
const MAX_VALUE: f64 = 1.0;
const MIN_TIME: f64 = 0.0;
const FLAT_CURVE_TIME: f64 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThresholdNode {
    pub time: f64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ThresholdCurves {
    pub on: Vec<ThresholdNode>,
    pub off: Vec<ThresholdNode>,
}

pub fn flat_curve(value: f64) -> Vec<ThresholdNode> {
    vec![ThresholdNode {
        time: FLAT_CURVE_TIME,
        value,
    }]
}

pub fn sanitize_nodes(nodes: &[ThresholdNode]) -> Vec<ThresholdNode> {
    let mut cleaned: Vec<ThresholdNode> = nodes
        .iter()
        .filter(|n| n.time.is_finite() && n.value.is_finite())
        .map(|n| ThresholdNode {
            time: n.time.max(MIN_TIME),
            value: n.value.clamp(MIN_VALUE, MAX_VALUE),
        })
        .collect();
    cleaned.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    let mut unique: Vec<ThresholdNode> = Vec::with_capacity(cleaned.len());
    for node in cleaned {
        match unique.last_mut() {
            Some(last) if last.time == node.time => *last = node,
            _ => unique.push(node),
        }
    }
    unique
}

pub fn value_at(nodes: &[ThresholdNode], time: f64) -> Option<f64> {
    let first = nodes.first()?;
    let last = nodes.last()?;
    if time <= first.time {
        return Some(first.value);
    }
    if time >= last.time {
        return Some(last.value);
    }
    let upper = nodes.partition_point(|n| n.time <= time);
    let a = nodes[upper - 1];
    let b = nodes[upper];
    let span = b.time - a.time;
    if span <= 0.0 {
        return Some(b.value);
    }
    Some(a.value + (b.value - a.value) * (time - a.time) / span)
}

impl ThresholdCurves {
    pub fn sanitized(&self) -> Option<ThresholdCurves> {
        let on = sanitize_nodes(&self.on);
        let off = sanitize_nodes(&self.off);
        if on.is_empty() || off.is_empty() {
            return None;
        }
        Some(ThresholdCurves { on, off })
    }

    pub fn at(&self, time: f64) -> Option<(f64, f64)> {
        let on = value_at(&self.on, time)?;
        let off = value_at(&self.off, time)?;
        Some((on, off.min(on)))
    }

    pub fn is_ordered(&self) -> bool {
        self.on
            .iter()
            .chain(self.off.iter())
            .all(|n| match (value_at(&self.on, n.time), value_at(&self.off, n.time)) {
                (Some(on), Some(off)) => on >= off,
                _ => false,
            })
    }
}

#[cfg(test)]
mod tests;
