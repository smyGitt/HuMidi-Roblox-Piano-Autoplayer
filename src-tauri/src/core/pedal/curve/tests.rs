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
