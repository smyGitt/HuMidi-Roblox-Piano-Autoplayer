use serde_json::Value;

use super::curve::{flat_curve, ThresholdCurves, ThresholdNode};

const CURVE_ON_KEY: &str = "pedal_curve_on";
const CURVE_OFF_KEY: &str = "pedal_curve_off";
const LEGACY_ON_KEY: &str = "pedal_threshold_on";
const LEGACY_OFF_KEY: &str = "pedal_threshold_off";
const LEGACY_UNSET: f64 = 0.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SavedPedalCurves {
    pub on: Vec<ThresholdNode>,
    pub off: Vec<ThresholdNode>,
    pub legacy: bool,
}

fn curves_from_settings(settings: &Value) -> Option<ThresholdCurves> {
    let on: Vec<ThresholdNode> = serde_json::from_value(settings.get(CURVE_ON_KEY)?.clone()).ok()?;
    let off: Vec<ThresholdNode> = serde_json::from_value(settings.get(CURVE_OFF_KEY)?.clone()).ok()?;
    ThresholdCurves { on, off }.sanitized()
}

fn legacy_pair_from_settings(settings: &Value) -> Option<(f64, f64)> {
    let on = settings.get(LEGACY_ON_KEY)?.as_f64()?;
    let off = settings.get(LEGACY_OFF_KEY)?.as_f64()?;
    let valid = on.is_finite() && off.is_finite() && on >= LEGACY_UNSET && off >= LEGACY_UNSET;
    valid.then_some((on, off.min(on)))
}

pub fn saved_pedal_curves(settings: Option<&Value>) -> SavedPedalCurves {
    let Some(settings) = settings else {
        return SavedPedalCurves::default();
    };
    if let Some(curves) = curves_from_settings(settings) {
        return SavedPedalCurves {
            on: curves.on,
            off: curves.off,
            legacy: false,
        };
    }
    match legacy_pair_from_settings(settings) {
        Some((on, off)) => SavedPedalCurves {
            on: flat_curve(on),
            off: flat_curve(off),
            legacy: true,
        },
        None => SavedPedalCurves::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_no_settings_gives_empty_curves() {
        assert_eq!(saved_pedal_curves(None), SavedPedalCurves::default());
    }

    #[test]
    fn test_settings_without_either_form_give_empty_curves() {
        let out = saved_pedal_curves(Some(&json!({ "tempo": 1.0 })));
        assert_eq!(out, SavedPedalCurves::default());
    }

    #[test]
    fn test_saved_curves_are_returned_as_they_are_and_not_legacy() {
        let settings = json!({
            "pedal_curve_on": [{ "time": 0.0, "value": 0.6 }, { "time": 5.0, "value": 0.4 }],
            "pedal_curve_off": [{ "time": 0.0, "value": 0.5 }, { "time": 5.0, "value": 0.3 }],
        });
        let out = saved_pedal_curves(Some(&settings));
        assert!(!out.legacy);
        assert_eq!(out.on.len(), 2);
        assert_eq!(out.off[1].value, 0.3);
    }

    #[test]
    fn test_empty_saved_curves_fall_through_to_the_legacy_pair() {
        let settings = json!({
            "pedal_curve_on": [],
            "pedal_curve_off": [],
            "pedal_threshold_on": 0.6,
            "pedal_threshold_off": 0.5,
        });
        let out = saved_pedal_curves(Some(&settings));
        assert!(out.legacy);
    }

    #[test]
    fn test_a_legacy_pair_becomes_two_flat_lines_and_is_flagged() {
        let settings = json!({ "pedal_threshold_on": 0.57, "pedal_threshold_off": 0.54 });
        let out = saved_pedal_curves(Some(&settings));
        assert!(out.legacy);
        assert_eq!(out.on, flat_curve(0.57));
        assert_eq!(out.off, flat_curve(0.54));
    }

    #[test]
    fn test_the_unset_sentinel_is_not_a_legacy_pair() {
        let settings = json!({ "pedal_threshold_on": -1.0, "pedal_threshold_off": -1.0 });
        assert_eq!(saved_pedal_curves(Some(&settings)), SavedPedalCurves::default());
    }

    #[test]
    fn test_one_legacy_value_alone_is_ignored() {
        let settings = json!({ "pedal_threshold_on": 0.6 });
        assert_eq!(saved_pedal_curves(Some(&settings)), SavedPedalCurves::default());
    }

    #[test]
    fn test_a_legacy_off_above_on_is_capped_at_on() {
        let settings = json!({ "pedal_threshold_on": 0.4, "pedal_threshold_off": 0.7 });
        let out = saved_pedal_curves(Some(&settings));
        assert_eq!(out.off, flat_curve(0.4));
    }

    #[test]
    fn test_non_numeric_legacy_values_are_ignored() {
        let settings = json!({ "pedal_threshold_on": "high", "pedal_threshold_off": null });
        assert_eq!(saved_pedal_curves(Some(&settings)), SavedPedalCurves::default());
    }

    #[test]
    fn test_malformed_saved_curves_fall_back_to_the_legacy_pair() {
        let settings = json!({
            "pedal_curve_on": "nope",
            "pedal_curve_off": [],
            "pedal_threshold_on": 0.5,
            "pedal_threshold_off": 0.4,
        });
        assert!(saved_pedal_curves(Some(&settings)).legacy);
    }
}
