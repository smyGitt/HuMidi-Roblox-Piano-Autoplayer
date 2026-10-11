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
mod tests {
    use super::*;

    fn node(time: f64, value: f64) -> ThresholdNode {
        ThresholdNode { time, value }
    }

    mod test_sanitize_nodes {
        use super::*;

        #[test]
        fn test_sorts_by_time() {
            let out = sanitize_nodes(&[node(5.0, 0.2), node(1.0, 0.4), node(3.0, 0.3)]);
            let times: Vec<f64> = out.iter().map(|n| n.time).collect();
            assert_eq!(times, vec![1.0, 3.0, 5.0]);
        }

        #[test]
        fn test_clamps_values_into_the_unit_range_and_times_to_zero() {
            let out = sanitize_nodes(&[node(-2.0, 1.5), node(1.0, -0.5)]);
            assert_eq!(out, vec![node(0.0, 1.0), node(1.0, 0.0)]);
        }

        #[test]
        fn test_drops_non_finite_nodes() {
            let out = sanitize_nodes(&[
                node(f64::NAN, 0.5),
                node(1.0, f64::INFINITY),
                node(2.0, 0.5),
            ]);
            assert_eq!(out, vec![node(2.0, 0.5)]);
        }

        #[test]
        fn test_duplicate_times_keep_the_last_node() {
            let out = sanitize_nodes(&[node(1.0, 0.2), node(1.0, 0.8)]);
            assert_eq!(out, vec![node(1.0, 0.8)]);
        }

        #[test]
        fn test_empty_stays_empty() {
            assert!(sanitize_nodes(&[]).is_empty());
        }
    }

    mod test_value_at {
        use super::*;

        #[test]
        fn test_empty_curve_has_no_value() {
            assert_eq!(value_at(&[], 1.0), None);
        }

        #[test]
        fn test_single_node_is_a_flat_line() {
            let nodes = flat_curve(0.4);
            assert_eq!(value_at(&nodes, 0.0), Some(0.4));
            assert_eq!(value_at(&nodes, 99.0), Some(0.4));
        }

        #[test]
        fn test_holds_the_end_values_outside_the_nodes() {
            let nodes = [node(2.0, 0.2), node(4.0, 0.6)];
            assert_eq!(value_at(&nodes, 0.0), Some(0.2));
            assert_eq!(value_at(&nodes, 10.0), Some(0.6));
        }

        #[test]
        fn test_interpolates_linearly_between_nodes() {
            let nodes = [node(0.0, 0.2), node(4.0, 0.6)];
            let mid = value_at(&nodes, 2.0).unwrap();
            assert!((mid - 0.4).abs() < 1e-12);
            let quarter = value_at(&nodes, 1.0).unwrap();
            assert!((quarter - 0.3).abs() < 1e-12);
        }

        #[test]
        fn test_exact_node_time_returns_the_node_value() {
            let nodes = [node(0.0, 0.2), node(4.0, 0.6), node(8.0, 0.1)];
            assert_eq!(value_at(&nodes, 4.0), Some(0.6));
        }
    }

    mod test_curves {
        use super::*;

        fn curves(on: Vec<ThresholdNode>, off: Vec<ThresholdNode>) -> ThresholdCurves {
            ThresholdCurves { on, off }
        }

        #[test]
        fn test_sanitized_needs_both_curves() {
            assert!(curves(vec![], flat_curve(0.3)).sanitized().is_none());
            assert!(curves(flat_curve(0.3), vec![]).sanitized().is_none());
            assert!(curves(flat_curve(0.4), flat_curve(0.3)).sanitized().is_some());
        }

        #[test]
        fn test_sanitized_cleans_both_curves() {
            let out = curves(vec![node(2.0, 2.0), node(1.0, 0.5)], flat_curve(0.3))
                .sanitized()
                .unwrap();
            assert_eq!(out.on, vec![node(1.0, 0.5), node(2.0, 1.0)]);
        }

        #[test]
        fn test_at_caps_the_off_value_at_the_on_value() {
            let c = curves(flat_curve(0.3), flat_curve(0.5));
            assert_eq!(c.at(1.0), Some((0.3, 0.3)));
        }

        #[test]
        fn test_at_is_none_when_a_curve_is_empty() {
            assert_eq!(curves(vec![], flat_curve(0.3)).at(1.0), None);
        }

        #[test]
        fn test_ordered_curves_pass() {
            let c = curves(
                vec![node(0.0, 0.6), node(10.0, 0.4)],
                vec![node(0.0, 0.5), node(10.0, 0.3)],
            );
            assert!(c.is_ordered());
        }

        #[test]
        fn test_a_crossing_at_a_node_is_unordered() {
            let c = curves(
                vec![node(0.0, 0.6), node(10.0, 0.4)],
                vec![node(0.0, 0.5), node(10.0, 0.45)],
            );
            assert!(!c.is_ordered());
        }

        #[test]
        fn test_a_crossing_at_an_off_node_between_on_nodes_is_unordered() {
            let c = curves(
                vec![node(0.0, 0.2), node(10.0, 0.2)],
                vec![node(0.0, 0.1), node(5.0, 0.5), node(10.0, 0.1)],
            );
            assert!(!c.is_ordered());
        }

        #[test]
        fn test_touching_curves_are_ordered() {
            let c = curves(flat_curve(0.4), flat_curve(0.4));
            assert!(c.is_ordered());
        }
    }
}
