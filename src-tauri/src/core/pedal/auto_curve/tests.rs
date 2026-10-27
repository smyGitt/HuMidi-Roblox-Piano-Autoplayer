use super::*;
use crate::core::pedal::FPS;

fn oscillation(frames: usize, base: f32, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .map(|i| base + amplitude * ((i as f32) * 0.3).sin())
        .collect()
}

fn seconds(s: f64) -> usize {
    (s * FPS) as usize
}

mod test_estimate_thresholds {
    use super::*;

    #[test]
    fn test_empty_has_no_estimate() {
        assert_eq!(estimate_thresholds(&[]), None);
    }

    #[test]
    fn test_flat_input_has_no_estimate() {
        assert_eq!(estimate_thresholds(&[0.5; 50]), None);
    }

    #[test]
    fn test_bimodal_input_puts_on_above_off_inside_the_observed_range() {
        let mut values = vec![0.2f32; 50];
        values.extend(vec![0.8f32; 50]);
        values.extend(oscillation(100, 0.5, 0.05));
        let (on, off) = estimate_thresholds(&values).unwrap();
        assert!(on > off, "on {on} should be above off {off}");
        assert!(off >= 0.2 - 1e-4 && on <= 0.8 + 1e-4);
    }

    #[test]
    fn test_on_is_never_below_off_for_a_wide_spread() {
        let values = oscillation(500, 0.5, 0.45);
        let (on, off) = estimate_thresholds(&values).unwrap();
        assert!(on >= off);
    }

    #[test]
    fn test_scales_with_the_observed_range() {
        let low = oscillation(500, 0.2, 0.1);
        let high = oscillation(500, 0.7, 0.1);
        let (low_on, _) = estimate_thresholds(&low).unwrap();
        let (high_on, _) = estimate_thresholds(&high).unwrap();
        assert!(high_on > low_on + 0.4);
    }
}

mod test_auto_curves {
    use super::*;

    #[test]
    fn test_nothing_active_gives_no_curves() {
        assert!(auto_curves(&vec![0.0; 500], FPS).is_none());
    }

    #[test]
    fn test_flat_confidence_gives_no_curves() {
        assert!(auto_curves(&vec![0.5; 500], FPS).is_none());
    }

    #[test]
    fn test_a_short_song_gets_the_same_value_at_every_node() {
        let preds = oscillation(seconds(8.0), 0.5, 0.3);
        let curves = auto_curves(&preds, FPS).unwrap();
        let first = curves.on[0].value;
        assert!(curves.on.iter().all(|n| (n.value - first).abs() < 1e-9));
        let first_off = curves.off[0].value;
        assert!(curves.off.iter().all(|n| (n.value - first_off).abs() < 1e-9));
    }

    #[test]
    fn test_nodes_are_evenly_spaced_and_start_at_zero() {
        let preds = oscillation(seconds(60.0), 0.5, 0.3);
        let curves = auto_curves(&preds, FPS).unwrap();
        assert_eq!(curves.on[0].time, 0.0);
        for pair in curves.on.windows(2) {
            assert!((pair[1].time - pair[0].time - NODE_SPACING_SECONDS).abs() < 1e-9);
        }
        assert_eq!(curves.on.len(), curves.off.len());
    }

    #[test]
    fn test_a_louder_second_half_raises_the_thresholds_there() {
        let mut preds = oscillation(seconds(90.0), 0.25, 0.1);
        preds.extend(oscillation(seconds(90.0), 0.7, 0.2));
        let curves = auto_curves(&preds, FPS).unwrap();
        let early = curves.on.first().unwrap().value;
        let late = curves.on.last().unwrap().value;
        assert!(
            late > early + 0.2,
            "late on-threshold {late} should be well above early {early}"
        );
    }

    #[test]
    fn test_off_stays_at_or_below_on_at_every_node() {
        let mut preds = oscillation(seconds(60.0), 0.3, 0.15);
        preds.extend(oscillation(seconds(60.0), 0.75, 0.2));
        let curves = auto_curves(&preds, FPS).unwrap();
        assert!(curves.is_ordered());
        for (on, off) in curves.on.iter().zip(curves.off.iter()) {
            assert!(on.value >= off.value);
        }
    }

    #[test]
    fn test_a_silent_stretch_is_bridged_between_its_neighbours() {
        let mut preds = oscillation(seconds(40.0), 0.3, 0.15);
        preds.extend(vec![0.0; seconds(60.0)]);
        preds.extend(oscillation(seconds(40.0), 0.7, 0.2));
        let curves = auto_curves(&preds, FPS).unwrap();
        let middle_index = curves.on.len() / 2;
        let before = curves.on[0].value;
        let after = curves.on.last().unwrap().value;
        let middle = curves.on[middle_index].value;
        assert!(middle > before.min(after) - 1e-6 && middle < before.max(after) + 1e-6);
    }

    #[test]
    fn test_a_window_that_is_all_noise_is_replaced_by_its_neighbours() {
        let mut preds = oscillation(seconds(40.0), 0.3, 0.2);
        preds.extend(oscillation(seconds(60.0), 0.5, 0.001));
        preds.extend(oscillation(seconds(40.0), 0.3, 0.2));
        let curves = auto_curves(&preds, FPS).unwrap();
        let middle = curves.on[curves.on.len() / 2].value;
        assert!(
            (middle - curves.on[0].value).abs() < 0.2,
            "the noise window should not produce its own extreme threshold, got {middle}"
        );
    }

    #[test]
    fn test_thresholds_stay_inside_the_unit_range() {
        let preds = oscillation(seconds(120.0), 0.5, 0.49);
        let curves = auto_curves(&preds, FPS).unwrap();
        assert!(curves
            .on
            .iter()
            .chain(curves.off.iter())
            .all(|n| (0.0..=1.0).contains(&n.value)));
    }
}

mod test_fill_gaps {
    use super::*;

    const GLOBAL: (f32, f32) = (0.9, 0.8);

    #[test]
    fn test_all_missing_uses_the_global_pair() {
        assert_eq!(fill_gaps(vec![None, None], GLOBAL), vec![GLOBAL, GLOBAL]);
    }

    #[test]
    fn test_a_gap_is_interpolated_by_position() {
        let out = fill_gaps(vec![Some((0.2, 0.1)), None, None, Some((0.5, 0.4))], GLOBAL);
        assert!((out[1].0 - 0.3).abs() < 1e-6);
        assert!((out[2].0 - 0.4).abs() < 1e-6);
        assert!((out[1].1 - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_leading_and_trailing_gaps_copy_the_nearest_valid_value() {
        let out = fill_gaps(vec![None, Some((0.3, 0.2)), None], GLOBAL);
        assert_eq!(out[0], (0.3, 0.2));
        assert_eq!(out[2], (0.3, 0.2));
    }
}
