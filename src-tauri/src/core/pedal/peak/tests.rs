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
