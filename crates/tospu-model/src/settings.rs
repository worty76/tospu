use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::kind::MessageKind;
use crate::time::HhMm;

pub const MAX_SLOTS: usize = 12;
pub const MIN_DISPLAY_SECONDS: u32 = 3;
pub const MAX_DISPLAY_SECONDS: u32 = 60;
pub const MAX_JITTER_MINUTES: u32 = 30;
pub const MIN_INTERVAL_MINUTES: u32 = 1;
pub const MAX_INTERVAL_MINUTES: u32 = 1440;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleMode {
    #[default]
    Daily,
    Interval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IntervalSchedule {
    pub every_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MessageKind>,
}

impl Default for IntervalSchedule {
    fn default() -> Self {
        Self {
            every_minutes: 30,
            kind: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub time: HhMm,
    pub kind: MessageKind,
}

/// A daily window with no popups. `start > end` wraps past midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietHours {
    pub start: HhMm,
    pub end: HhMm,
}

impl QuietHours {
    pub fn contains(&self, minute_of_day: u32) -> bool {
        let (s, e) = (self.start.minutes(), self.end.minutes());
        if s <= e {
            (s..e).contains(&minute_of_day)
        } else {
            minute_of_day >= s || minute_of_day < e
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schedule_mode: ScheduleMode,
    pub slots: Vec<Slot>,
    pub interval: IntervalSchedule,
    pub jitter_minutes: u32,
    pub quiet_hours: Option<QuietHours>,
    pub quiet_mode: bool,
    pub active_pack: String,
    pub display_seconds: u32,
    pub privacy_blur: bool,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let slot = |h, kind| Slot {
            time: HhMm::new(h, 0).unwrap(),
            kind,
        };
        Self {
            schedule_mode: ScheduleMode::Daily,
            slots: vec![
                slot(9, MessageKind::Goal),
                slot(14, MessageKind::Quote),
                slot(21, MessageKind::Gratitude),
            ],
            interval: IntervalSchedule::default(),
            jitter_minutes: 10,
            quiet_hours: Some(QuietHours {
                start: HhMm::new(23, 0).unwrap(),
                end: HhMm::new(7, 30).unwrap(),
            }),
            quiet_mode: false,
            active_pack: "mochi".into(),
            display_seconds: 8,
            privacy_blur: true,
            onboarded: false,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.slots.len() > MAX_SLOTS {
            errors.push(format!(
                "slots: at most {MAX_SLOTS} daily times are allowed"
            ));
        }

        let mut seen = HashSet::new();
        for slot in &self.slots {
            if !seen.insert(slot.time) {
                errors.push(format!("slots: duplicate time {}", slot.time));
            }
        }

        if !(MIN_DISPLAY_SECONDS..=MAX_DISPLAY_SECONDS).contains(&self.display_seconds) {
            errors.push(format!(
                "displaySeconds: must be between {MIN_DISPLAY_SECONDS} and {MAX_DISPLAY_SECONDS}"
            ));
        }

        if self.jitter_minutes > MAX_JITTER_MINUTES {
            errors.push(format!(
                "jitterMinutes: must be at most {MAX_JITTER_MINUTES}"
            ));
        }

        if let Some(q) = self.quiet_hours {
            if q.start == q.end {
                errors.push("quietHours: start and end must be different".into());
            }
        }

        if self.active_pack.trim().is_empty() {
            errors.push("activePack: must not be empty".into());
        }

        if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&self.interval.every_minutes) {
            errors.push(format!(
                "interval: must be between {MIN_INTERVAL_MINUTES} and {MAX_INTERVAL_MINUTES} minutes"
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_past_midnight() {
        let q = QuietHours {
            start: "23:00".parse().unwrap(),
            end: "07:30".parse().unwrap(),
        };
        assert!(q.contains(23 * 60 + 30));
        assert!(q.contains(3 * 60));
        assert!(!q.contains(7 * 60 + 30));
        assert!(!q.contains(12 * 60));
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let s: Settings = serde_json::from_str(r#"{"activePack":"fox"}"#).unwrap();
        assert_eq!(s.active_pack, "fox");
        assert_eq!(s.display_seconds, 8);
        assert_eq!(s.slots.len(), 3);
    }

    #[test]
    fn old_file_without_new_fields_gets_privacy_blur_true_and_onboarded_false() {
        let s: Settings = serde_json::from_str(r#"{"activePack":"mochi"}"#).unwrap();
        assert!(s.privacy_blur);
        assert!(!s.onboarded);
    }

    #[test]
    fn validate_accepts_defaults() {
        assert!(Settings::default().validate().is_ok());
    }

    #[test]
    fn validate_rejects() {
        // 13 slots -> "slots"
        let s = Settings {
            slots: (0..13)
                .map(|i| Slot {
                    time: HhMm::new(i, 0).unwrap(),
                    kind: MessageKind::Goal,
                })
                .collect(),
            ..Default::default()
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("slots")),
            "expected slots error, got {errs:?}"
        );

        // duplicate time "09:00" -> "09:00"
        let s = Settings {
            slots: vec![
                Slot {
                    time: HhMm::new(9, 0).unwrap(),
                    kind: MessageKind::Goal,
                },
                Slot {
                    time: HhMm::new(9, 0).unwrap(),
                    kind: MessageKind::Quote,
                },
            ],
            ..Default::default()
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("09:00")),
            "expected 09:00 error, got {errs:?}"
        );

        // display_seconds = 2 and 61 -> "displaySeconds"
        for bad in [2, 61] {
            let s = Settings {
                display_seconds: bad,
                ..Default::default()
            };
            let errs = s.validate().unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains("displaySeconds")),
                "expected displaySeconds error for {bad}, got {errs:?}"
            );
        }

        // jitter_minutes = 31 -> "jitterMinutes"
        let s = Settings {
            jitter_minutes: 31,
            ..Default::default()
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("jitterMinutes")),
            "expected jitterMinutes error, got {errs:?}"
        );

        // quiet hours 22:00-22:00 -> "quietHours"
        let t = HhMm::new(22, 0).unwrap();
        let s = Settings {
            quiet_hours: Some(QuietHours { start: t, end: t }),
            ..Default::default()
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("quietHours")),
            "expected quietHours error, got {errs:?}"
        );

        // active_pack = "" -> "activePack"
        let s = Settings {
            active_pack: String::new(),
            ..Default::default()
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("activePack")),
            "expected activePack error, got {errs:?}"
        );

        // interval = 0 and 1441 -> "interval"
        for bad in [0, 1441] {
            let s = Settings {
                schedule_mode: ScheduleMode::Interval,
                interval: IntervalSchedule {
                    every_minutes: bad,
                    kind: None,
                },
                ..Default::default()
            };
            let errs = s.validate().unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains("interval")),
                "expected interval error for {bad}, got {errs:?}"
            );
        }
    }
}
