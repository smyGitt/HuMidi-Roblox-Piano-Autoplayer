use crate::core::midi::KeyMapper;
use crate::core::models::KeyEvent;

pub const BLOCKED_INPUT_PREFIX: &str = "Blocked unsafe input: ";

const ACTION_PRESS: &str = "press";
const ACTION_RELEASE: &str = "release";
const ACTION_PEDAL: &str = "pedal";
const PEDAL_DOWN: &str = "down";
const PEDAL_UP: &str = "up";
const MIN_VELOCITY: i32 = 0;
const MAX_VELOCITY: i32 = 127;
const MAX_REPORTED_CHARS: usize = 24;
const TRUNCATION_MARK: &str = "...";

fn printable(text: &str) -> String {
    let mut shown: String = text
        .chars()
        .take(MAX_REPORTED_CHARS)
        .flat_map(char::escape_default)
        .collect();
    if text.chars().count() > MAX_REPORTED_CHARS {
        shown.push_str(TRUNCATION_MARK);
    }
    shown
}

fn key_matches_pitch(layouts: &[KeyMapper], pitch: i32, key_char: &str) -> bool {
    layouts.iter().any(|mapper| {
        mapper
            .get_key_data(pitch)
            .is_some_and(|data| key_char == data.key.to_string())
    })
}

fn invalid_reason(event: &KeyEvent, layouts: &[KeyMapper]) -> Option<String> {
    if !event.time.is_finite() {
        return Some("has an invalid time".to_string());
    }
    match event.action.as_str() {
        ACTION_PRESS | ACTION_RELEASE => {
            let Some(pitch) = event.pitch else {
                return Some(format!(
                    "{} the key '{}' without a note",
                    event.action,
                    printable(&event.key_char)
                ));
            };
            if event
                .velocity
                .is_some_and(|v| !(MIN_VELOCITY..=MAX_VELOCITY).contains(&v))
            {
                return Some("has an invalid velocity".to_string());
            }
            if !key_matches_pitch(layouts, pitch, &event.key_char) {
                return Some(format!(
                    "{} the key '{}', which is not a piano key for note {}",
                    event.action,
                    printable(&event.key_char),
                    pitch
                ));
            }
            None
        }
        ACTION_PEDAL => {
            if event.key_char == PEDAL_DOWN || event.key_char == PEDAL_UP {
                None
            } else {
                Some(format!(
                    "sends the unknown pedal command '{}'",
                    printable(&event.key_char)
                ))
            }
        }
        other => Some(format!("has the unknown action '{}'", printable(other))),
    }
}

pub fn scan_events(events: &[KeyEvent]) -> Result<(), String> {
    let layouts = [KeyMapper::new(false), KeyMapper::new(true)];
    for (index, event) in events.iter().enumerate() {
        if let Some(reason) = invalid_reason(event, &layouts) {
            return Err(format!("{}event #{} {}", BLOCKED_INPUT_PREFIX, index, reason));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(pitch: i32, key: &str) -> KeyEvent {
        let mut e = KeyEvent::new(0.0, 2, "press", key);
        e.pitch = Some(pitch);
        e.velocity = Some(80);
        e
    }

    fn release(pitch: i32, key: &str) -> KeyEvent {
        let mut e = KeyEvent::new(1.0, 4, "release", key);
        e.pitch = Some(pitch);
        e
    }

    fn middle_c_key() -> String {
        KeyMapper::new(false).get_key_data(60).unwrap().key.to_string()
    }

    #[test]
    fn test_empty_is_valid() {
        assert!(scan_events(&[]).is_ok());
    }

    #[test]
    fn test_press_release_and_pedal_from_the_mapper_are_valid() {
        let key = middle_c_key();
        let events = [
            press(60, &key),
            release(60, &key),
            KeyEvent::new(0.5, 1, "pedal", "down"),
            KeyEvent::new(0.9, 0, "pedal", "up"),
        ];
        assert!(scan_events(&events).is_ok());
    }

    #[test]
    fn test_every_key_of_both_layouts_is_valid() {
        for use_88 in [false, true] {
            let mapper = KeyMapper::new(use_88);
            for pitch in mapper.min_pitch..=mapper.max_pitch {
                let key = mapper.get_key_data(pitch).unwrap().key.to_string();
                assert!(scan_events(&[press(pitch, &key)]).is_ok(), "pitch {pitch}");
            }
        }
    }

    #[test]
    fn test_a_key_that_does_not_belong_to_the_pitch_is_blocked() {
        let err = scan_events(&[press(60, "\n")]).unwrap_err();
        assert!(err.starts_with(BLOCKED_INPUT_PREFIX));
        assert!(err.contains("event #0"));
        assert!(err.contains("\\n"));
    }

    #[test]
    fn test_a_press_without_a_pitch_is_blocked() {
        let events = [KeyEvent::new(0.0, 2, "press", "r")];
        let err = scan_events(&events).unwrap_err();
        assert!(err.contains("without a note"));
    }

    #[test]
    fn test_a_release_without_a_pitch_is_blocked() {
        let events = [KeyEvent::new(0.0, 4, "release", "r")];
        assert!(scan_events(&events).is_err());
    }

    #[test]
    fn test_an_unknown_action_is_blocked() {
        let events = [KeyEvent::new(0.0, 2, "shortcut", "ctrl+alt+delete")];
        let err = scan_events(&events).unwrap_err();
        assert!(err.contains("unknown action 'shortcut'"));
    }

    #[test]
    fn test_an_unknown_pedal_command_is_blocked() {
        let events = [KeyEvent::new(0.0, 1, "pedal", "enter")];
        let err = scan_events(&events).unwrap_err();
        assert!(err.contains("unknown pedal command 'enter'"));
    }

    #[test]
    fn test_a_non_finite_time_is_blocked() {
        let mut e = KeyEvent::new(f64::NAN, 1, "pedal", "down");
        assert!(scan_events(&[e.clone()]).is_err());
        e.time = f64::INFINITY;
        assert!(scan_events(&[e]).is_err());
    }

    #[test]
    fn test_an_out_of_range_velocity_is_blocked() {
        let key = middle_c_key();
        let mut e = press(60, &key);
        e.velocity = Some(MAX_VELOCITY + 1);
        assert!(scan_events(&[e.clone()]).is_err());
        e.velocity = Some(MIN_VELOCITY - 1);
        assert!(scan_events(&[e]).is_err());
    }

    #[test]
    fn test_the_first_bad_event_is_reported_by_its_position() {
        let key = middle_c_key();
        let events = [press(60, &key), release(60, &key), KeyEvent::new(2.0, 2, "run", "calc")];
        let err = scan_events(&events).unwrap_err();
        assert!(err.contains("event #2"));
    }

    #[test]
    fn test_a_long_untrusted_value_is_shortened_in_the_message() {
        let long = "x".repeat(MAX_REPORTED_CHARS * 40);
        let events = [KeyEvent::new(0.0, 2, &long, "a")];
        let err = scan_events(&events).unwrap_err();
        assert!(err.contains(TRUNCATION_MARK));
        assert!(err.len() < long.len());
    }
}
