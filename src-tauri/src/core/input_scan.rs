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
mod tests;
