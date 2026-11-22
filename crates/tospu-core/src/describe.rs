//! Human-readable formatting for the next scheduled popup time.

use chrono::NaiveDateTime;

/// Formats `at` relative to `now`: `"Today 14:07"`, `"Tomorrow 09:03"`,
/// or `"Mon 09:00"` within 7 days (otherwise `"Nov 29 09:00"`).
pub fn describe_next(now: NaiveDateTime, at: NaiveDateTime) -> String {
    let days = (at.date() - now.date()).num_days();
    let time = at.format("%H:%M");
    match days {
        0 => format!("Today {time}"),
        1 => format!("Tomorrow {time}"),
        2..=6 => format!("{} {time}", at.format("%a")),
        _ => format!("{} {time}", at.format("%b %-d")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn today_tomorrow_and_weekday() {
        let now = dt(2026, 11, 22, 10, 0); // Sunday
        assert_eq!(describe_next(now, dt(2026, 11, 22, 14, 7)), "Today 14:07");
        assert_eq!(describe_next(now, dt(2026, 11, 23, 9, 3)), "Tomorrow 09:03");
        assert_eq!(describe_next(now, dt(2026, 11, 25, 9, 0)), "Wed 09:00");
    }
}
