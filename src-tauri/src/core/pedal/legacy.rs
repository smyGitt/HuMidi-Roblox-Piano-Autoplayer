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
mod tests;
