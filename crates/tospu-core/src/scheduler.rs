//! Decides WHEN the next popup happens. Pure: give it "now" and settings.

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Timelike};

use tospu_model::{MessageKind, ScheduleMode, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub at: NaiveDateTime,
    pub kind: MessageKind,
}

/// The first popup strictly after `now`, or `None` if nothing is scheduled.
///
/// In `Daily` mode, each slot is shifted by a jitter that is random-looking
/// but fixed for a given (date, slot), so re-planning after a settings reload
/// or wake from sleep doesn't move today's popups around.
///
/// In `Interval` mode, popups fire on exact `every_minutes` boundaries with no
/// random shift, rotating through all four kinds unless a specific kind is set.
///
/// In both modes, popups that land inside quiet hours are skipped.
pub fn next_fire(now: NaiveDateTime, s: &Settings) -> Option<Fire> {
    match s.schedule_mode {
        ScheduleMode::Daily => next_daily_fire(now, s),
        ScheduleMode::Interval => next_interval_fire(now, s),
    }
}

fn next_interval_fire(now: NaiveDateTime, s: &Settings) -> Option<Fire> {
    let step = i64::from(s.interval.every_minutes.clamp(1, 1440));
    let now_min = now.and_utc().timestamp().div_euclid(60);
    let first_bucket = now_min.div_euclid(step) + 1;
    let max_checks = (24 * 60 / step) + 2;

    for offset in 0..=max_checks {
        let bucket = first_bucket + offset;
        let ts_secs = bucket * step * 60;
        let Some(dt) = DateTime::from_timestamp(ts_secs, 0).map(|d| d.naive_utc()) else {
            continue;
        };
        if dt <= now {
            continue;
        }
        let minute_of_day = dt.hour() * 60 + dt.minute();
        if s.quiet_hours.is_some_and(|q| q.contains(minute_of_day)) {
            continue;
        }
        let kind = s
            .interval
            .kind
            .unwrap_or_else(|| MessageKind::ALL[bucket.rem_euclid(4) as usize]);
        return Some(Fire { at: dt, kind });
    }
    None
}

fn next_daily_fire(now: NaiveDateTime, s: &Settings) -> Option<Fire> {
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
    use chrono::NaiveDate;
    use tospu_model::{QuietHours, Slot};

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

    #[test]
    fn interval_fires_every_minute_and_rotates_kinds() {
        let s = Settings {
            schedule_mode: tospu_model::ScheduleMode::Interval,
            interval: tospu_model::IntervalSchedule {
                every_minutes: 1,
                kind: None,
            },
            quiet_hours: None,
            ..Settings::default()
        };
        let f1 = next_fire(at(21, 10, 0), &s).unwrap();
        assert_eq!(f1.at, at(21, 10, 1));
        let f2 = next_fire(f1.at, &s).unwrap();
        assert_eq!(f2.at, at(21, 10, 2));
        let f3 = next_fire(f2.at, &s).unwrap();
        let f4 = next_fire(f3.at, &s).unwrap();
        let kinds = [f1.kind, f2.kind, f3.kind, f4.kind];
        let unique: std::collections::HashSet<_> = kinds.into_iter().collect();
        assert_eq!(unique.len(), 4, "expected all 4 kinds to rotate: {kinds:?}");
    }

    #[test]
    fn interval_respects_fixed_kind_and_quiet_hours() {
        let s = Settings {
            schedule_mode: tospu_model::ScheduleMode::Interval,
            interval: tospu_model::IntervalSchedule {
                every_minutes: 15,
                kind: Some(Quote),
            },
            quiet_hours: Some(QuietHours {
                start: "23:00".parse().unwrap(),
                end: "07:30".parse().unwrap(),
            }),
            ..Settings::default()
        };
        let f = next_fire(at(21, 22, 50), &s).unwrap();
        assert_eq!(
            f,
            Fire {
                at: at(22, 7, 30),
                kind: Quote
            }
        );
    }
}
