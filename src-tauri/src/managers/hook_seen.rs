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
mod tests {
    use super::*;
    use crate::managers::hotkey_manager::Modifier;

    const BEFORE_EXPIRY: Duration = Duration::from_millis(100);
    const AFTER_EXPIRY: Duration = Duration::from_millis(500);
    const ONE_MILLISECOND: Duration = Duration::from_millis(1);

    mod test_hook_seen {
        use super::*;

        const G: RawKey = RawKey::Char('g');
        const H: RawKey = RawKey::Char('h');

        #[test]
        fn test_a_recorded_press_makes_the_next_matching_press_a_duplicate_once() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(seen.is_duplicate_at(G, true, now));
            assert!(!seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_a_recorded_release_makes_the_next_matching_release_a_duplicate_once() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, false, now);
            assert!(seen.is_duplicate_at(G, false, now));
            assert!(!seen.is_duplicate_at(G, false, now));
        }

        #[test]
        fn test_nothing_recorded_means_nothing_is_a_duplicate() {
            let seen = HookSeen::new();
            assert!(!seen.is_duplicate_at(G, true, Instant::now()));
            assert!(!seen.is_duplicate_at(G, false, Instant::now()));
        }

        #[test]
        fn test_the_other_direction_is_not_a_duplicate_and_keeps_the_record() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(!seen.is_duplicate_at(G, false, now));
            assert!(seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_another_key_is_not_a_duplicate_and_keeps_the_record() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(!seen.is_duplicate_at(H, true, now));
            assert!(seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_left_and_right_modifiers_match_each_other_and_the_generic_modifier() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(RawKey::ModLeft(Modifier::Ctrl), true, now);
            assert!(seen.is_duplicate_at(RawKey::ModRight(Modifier::Ctrl), true, now));
            seen.record_at(RawKey::ModRight(Modifier::Alt), false, now);
            assert!(seen.is_duplicate_at(RawKey::Mod(Modifier::Alt), false, now));
        }

        #[test]
        fn test_a_newer_record_of_the_other_direction_replaces_the_older_one() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, false, now);
            seen.record_at(G, true, now);
            seen.record_at(G, false, now);
            assert!(!seen.is_duplicate_at(G, true, now));
            assert!(seen.is_duplicate_at(G, false, now));
        }

        #[test]
        fn test_a_page_event_after_a_press_and_release_is_not_mistaken_for_the_first_press() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.record_at(G, false, now);
            assert!(!seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_a_new_record_after_a_consumed_one_is_fresh_again() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(seen.is_duplicate_at(G, true, now));
            seen.record_at(G, true, now);
            assert!(seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_a_record_still_counts_before_it_expires_and_exactly_at_the_limit() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(seen.is_duplicate_at(G, true, now + BEFORE_EXPIRY));
            seen.record_at(G, true, now);
            assert!(seen.is_duplicate_at(G, true, now + SEEN_MAX_AGE));
        }

        #[test]
        fn test_a_record_expires_and_cannot_swallow_a_later_real_event() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            assert!(!seen.is_duplicate_at(G, true, now + SEEN_MAX_AGE + ONE_MILLISECOND));
            seen.record_at(G, true, now);
            assert!(!seen.is_duplicate_at(G, true, now + AFTER_EXPIRY));
        }

        #[test]
        fn test_a_check_dated_before_the_record_still_counts() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now + BEFORE_EXPIRY);
            assert!(seen.is_duplicate_at(G, true, now));
        }

        #[test]
        fn test_a_suppressed_save_is_reported_once_for_the_matching_press() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now);
            assert!(seen.take_suppressed_save_at(G, true, now));
            assert!(!seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_nothing_marked_means_no_suppressed_save() {
            let seen = HookSeen::new();
            let now = Instant::now();
            assert!(!seen.take_suppressed_save_at(G, true, now));
            seen.record_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_a_suppressed_save_belongs_to_its_own_key_and_direction() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(H, true, now));
            assert!(!seen.take_suppressed_save_at(G, false, now));
            assert!(seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_marking_a_press_with_no_record_or_another_direction_does_nothing() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.mark_save_suppressed_at(G, true, now);
            seen.record_at(G, false, now);
            seen.mark_save_suppressed_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(G, true, now));
            assert!(!seen.take_suppressed_save_at(G, false, now));
            seen.record_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_a_newer_record_clears_an_earlier_suppressed_save() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now);
            seen.record_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_a_suppressed_save_still_counts_before_it_expires_and_not_after() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now);
            assert!(seen.take_suppressed_save_at(G, true, now + SEEN_MAX_AGE));
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now);
            assert!(!seen.take_suppressed_save_at(G, true, now + SEEN_MAX_AGE + ONE_MILLISECOND));
        }

        #[test]
        fn test_marking_an_expired_record_does_nothing() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(G, true, now);
            seen.mark_save_suppressed_at(G, true, now + AFTER_EXPIRY);
            assert!(!seen.take_suppressed_save_at(G, true, now));
        }

        #[test]
        fn test_the_suppressed_save_follows_modifier_sides() {
            let seen = HookSeen::new();
            let now = Instant::now();
            seen.record_at(RawKey::ModLeft(Modifier::Ctrl), true, now);
            seen.mark_save_suppressed_at(RawKey::ModLeft(Modifier::Ctrl), true, now);
            assert!(seen.take_suppressed_save_at(RawKey::ModRight(Modifier::Ctrl), true, now));
        }

        #[test]
        fn test_the_public_suppressed_save_methods_use_the_current_time() {
            let seen = HookSeen::new();
            seen.record(G, true);
            seen.mark_save_suppressed(G, true);
            assert!(seen.take_suppressed_save(G, true));
            assert!(!seen.take_suppressed_save(G, true));
        }

        #[test]
        fn test_the_public_methods_use_the_current_time() {
            let seen = HookSeen::new();
            seen.record(G, true);
            assert!(seen.is_duplicate(G, true));
            assert!(!seen.is_duplicate(G, true));
        }

        #[test]
        fn test_a_poisoned_record_never_panics_and_never_reports_a_duplicate() {
            let seen = HookSeen::new();
            std::thread::scope(|scope| {
                let poisoner = scope.spawn(|| {
                    let _guard = seen.latest.lock().unwrap();
                    panic!("poison the record");
                });
                let _ = poisoner.join();
            });
            seen.record(G, true);
            assert!(!seen.is_duplicate(G, true));
        }
    }
}
