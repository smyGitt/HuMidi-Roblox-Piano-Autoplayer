use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::managers::hotkey_manager::{normalize, RawKey};

const SEEN_MAX_AGE: Duration = Duration::from_millis(250);

struct Seen {
    is_press: bool,
    at: Instant,
    claimed: bool,
    save_suppressed: bool,
}

fn is_fresh(seen: &Seen, now: Instant) -> bool {
    now.checked_duration_since(seen.at)
        .map_or(true, |age| age <= SEEN_MAX_AGE)
}

#[derive(Default)]
pub struct HookSeen {
    latest: Mutex<HashMap<RawKey, Seen>>,
}

impl HookSeen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, raw: RawKey, is_press: bool) {
        self.record_at(raw, is_press, Instant::now());
    }

    pub fn is_duplicate(&self, raw: RawKey, is_press: bool) -> bool {
        self.is_duplicate_at(raw, is_press, Instant::now())
    }

    pub fn mark_save_suppressed(&self, raw: RawKey, is_press: bool) {
        self.mark_save_suppressed_at(raw, is_press, Instant::now());
    }

    pub fn take_suppressed_save(&self, raw: RawKey, is_press: bool) -> bool {
        self.take_suppressed_save_at(raw, is_press, Instant::now())
    }

    fn record_at(&self, raw: RawKey, is_press: bool, now: Instant) {
        let Ok(mut latest) = self.latest.lock() else {
            return;
        };
        latest.insert(
            normalize(raw),
            Seen {
                is_press,
                at: now,
                claimed: false,
                save_suppressed: false,
            },
        );
    }

    fn mark_save_suppressed_at(&self, raw: RawKey, is_press: bool, now: Instant) {
        let Ok(mut latest) = self.latest.lock() else {
            return;
        };
        if let Some(seen) = latest.get_mut(&normalize(raw)) {
            if seen.is_press == is_press && is_fresh(seen, now) {
                seen.save_suppressed = true;
            }
        }
    }

    fn take_suppressed_save_at(&self, raw: RawKey, is_press: bool, now: Instant) -> bool {
        let Ok(mut latest) = self.latest.lock() else {
            return false;
        };
        match latest.get_mut(&normalize(raw)) {
            Some(seen) if seen.is_press == is_press && seen.save_suppressed && is_fresh(seen, now) => {
                seen.save_suppressed = false;
                true
            }
            _ => false,
        }
    }

    fn is_duplicate_at(&self, raw: RawKey, is_press: bool, now: Instant) -> bool {
        let Ok(mut latest) = self.latest.lock() else {
            return false;
        };
        match latest.get_mut(&normalize(raw)) {
            Some(seen) if seen.is_press == is_press && !seen.claimed && is_fresh(seen, now) => {
                seen.claimed = true;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;
