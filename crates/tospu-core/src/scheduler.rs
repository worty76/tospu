//! Decides WHEN the next popup happens. Pure: give it "now" and settings.

use chrono::{Datelike, Duration, NaiveDateTime, Timelike};

use crate::settings::{MessageKind, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub at: NaiveDateTime,
    pub kind: MessageKind,
}

/// The first popup strictly after `now`, or `None` if nothing is scheduled.
///
/// Each slot is shifted by a jitter that is random-looking but fixed for a
/// given (date, slot), so re-planning after a settings reload or wake from
/// sleep doesn't move today's popups around. Slots that land in quiet hours
/// (after jitter) are skipped.
pub fn next_fire(now: NaiveDateTime, s: &Settings) -> Option<Fire> {
    // Today and tomorrow always contain the next slot unless every slot is
    // quiet; a third day covers jitter pushing yesterday's late slot over.
    (-1..=2)
        .flat_map(|day| {
            let date = now.date() + Duration::days(day);
            s.slots.iter().enumerate().map(move |(i, slot)| {
                let base = date.and_hms_opt(0, 0, 0).unwrap()
                    + Duration::minutes(slot.time.minutes() as i64);
                let at =
                    base + Duration::minutes(jitter(date.num_days_from_ce(), i, s.jitter_minutes));
                Fire {
                    at,
                    kind: slot.kind,
                }
            })
        })
        .filter(|f| f.at > now)
        .filter(|f| {
            !s.quiet_hours
                .is_some_and(|q| q.contains(f.at.hour() * 60 + f.at.minute()))
        })
        .min_by_key(|f| f.at)
}

/// Deterministic offset in `[-max, max]` minutes for one slot on one day.
fn jitter(day: i32, slot: usize, max: u32) -> i64 {
    if max == 0 {
        return 0;
    }
    // FNV-1a over the two numbers: tiny, stable across builds and platforms.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in day
        .to_le_bytes()
        .into_iter()
        .chain((slot as u64).to_le_bytes())
    {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let span = 2 * max as u64 + 1;
    (h % span) as i64 - max as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{QuietHours, Slot};
    use chrono::NaiveDate;

    fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, d)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    fn settings(
        slots: &[(&str, MessageKind)],
        jitter: u32,
        quiet: Option<(&str, &str)>,
    ) -> Settings {
        Settings {
            slots: slots
                .iter()
                .map(|(t, k)| Slot {
                    time: t.parse().unwrap(),
                    kind: *k,
                })
                .collect(),
            jitter_minutes: jitter,
            quiet_hours: quiet.map(|(s, e)| QuietHours {
                start: s.parse().unwrap(),
                end: e.parse().unwrap(),
            }),
            ..Settings::default()
        }
    }

    use MessageKind::*;

    #[test]
    fn picks_the_next_slot_today() {
        let s = settings(&[("09:00", Goal), ("14:00", Quote)], 0, None);
        assert_eq!(
            next_fire(at(21, 10, 0), &s),
            Some(Fire {
                at: at(21, 14, 0),
                kind: Quote
            })
        );
    }

    #[test]
    fn rolls_over_to_tomorrow_after_the_last_slot() {
        let s = settings(&[("09:00", Goal), ("14:00", Quote)], 0, None);
        assert_eq!(
            next_fire(at(21, 15, 0), &s),
            Some(Fire {
                at: at(22, 9, 0),
                kind: Goal
            })
        );
    }

    #[test]
    fn a_slot_exactly_now_is_not_repeated() {
        let s = settings(&[("09:00", Goal)], 0, None);
        assert_eq!(next_fire(at(21, 9, 0), &s).unwrap().at, at(22, 9, 0));
    }

    #[test]
    fn jitter_is_stable_and_bounded() {
        let s = settings(&[("12:00", Goal)], 10, None);
        let a = next_fire(at(21, 0, 0), &s).unwrap();
        let b = next_fire(at(21, 0, 1), &s).unwrap();
        assert_eq!(a, b, "re-planning must not move the popup");
        let off = (a.at - at(21, 12, 0)).num_minutes().abs();
        assert!(off <= 10, "offset {off} out of range");
        let days: Vec<_> = (1..=10)
            .map(|d| next_fire(at(d, 0, 0), &s).unwrap().at.minute())
            .collect();
        assert!(
            days.windows(2).any(|w| w[0] != w[1]),
            "jitter should vary by day"
        );
    }

    #[test]
    fn skips_slots_inside_quiet_hours_across_midnight() {
        let s = settings(
            &[("23:30", Proud), ("06:00", Goal), ("08:00", Quote)],
            0,
            Some(("23:00", "07:30")),
        );
        assert_eq!(
            next_fire(at(21, 22, 0), &s),
            Some(Fire {
                at: at(22, 8, 0),
                kind: Quote
            })
        );
    }

    #[test]
    fn nothing_scheduled_gives_none() {
        assert_eq!(next_fire(at(21, 9, 0), &settings(&[], 0, None)), None);
        let all_quiet = settings(&[("02:00", Goal)], 0, Some(("00:00", "06:00")));
        assert_eq!(next_fire(at(21, 9, 0), &all_quiet), None);
    }
}
