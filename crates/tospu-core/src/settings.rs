use std::fmt;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Goal,
    Quote,
    Gratitude,
    Proud,
}

/// A wall-clock time of day, stored as minutes since midnight and written
/// as `"HH:MM"` in settings.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HhMm(u32);

impl HhMm {
    pub fn new(hour: u32, minute: u32) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self(hour * 60 + minute))
    }
    pub fn minutes(self) -> u32 {
        self.0
    }
}

impl FromStr for HhMm {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (h, m) = s
            .split_once(':')
            .ok_or_else(|| format!("\"{s}\" is not HH:MM"))?;
        let parse = |v: &str| {
            v.trim()
                .parse::<u32>()
                .map_err(|_| format!("\"{s}\" is not HH:MM"))
        };
        HhMm::new(parse(h)?, parse(m)?).ok_or_else(|| format!("\"{s}\" is out of range"))
    }
}

impl fmt::Display for HhMm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

impl Serialize for HhMm {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for HhMm {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub time: HhMm,
    pub kind: MessageKind,
}

/// A daily window with no popups. `start > end` wraps past midnight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub slots: Vec<Slot>,
    pub jitter_minutes: u32,
    pub quiet_hours: Option<QuietHours>,
    pub quiet_mode: bool,
    pub active_pack: String,
    pub display_seconds: u32,
}

impl Default for Settings {
    fn default() -> Self {
        let slot = |h, kind| Slot {
            time: HhMm::new(h, 0).unwrap(),
            kind,
        };
        Self {
            slots: vec![
                slot(9, MessageKind::Goal),
                slot(14, MessageKind::Quote),
                slot(21, MessageKind::Gratitude),
            ],
            jitter_minutes: 10,
            quiet_hours: Some(QuietHours {
                start: HhMm::new(23, 0).unwrap(),
                end: HhMm::new(7, 30).unwrap(),
            }),
            quiet_mode: false,
            active_pack: "mochi".into(),
            display_seconds: 8,
        }
    }
}

impl Settings {
    /// Load settings, creating the file with defaults if it doesn't exist.
    /// A broken file is left untouched (so the user can fix it) and the
    /// defaults are used, with the reason returned alongside.
    pub fn load_or_create(path: &Path) -> (Settings, Option<String>) {
        match fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(s) => (s, None),
                Err(e) => (Settings::default(), Some(format!("settings.json: {e}"))),
            },
            Err(_) => {
                let s = Settings::default();
                let err = s
                    .save(path)
                    .err()
                    .map(|e| format!("could not create settings.json: {e}"));
                (s, err)
            }
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).expect("settings always serialize");
        fs::write(path, text + "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hhmm_round_trips_through_json() {
        let t: HhMm = serde_json::from_str("\"07:05\"").unwrap();
        assert_eq!(t.minutes(), 7 * 60 + 5);
        assert_eq!(serde_json::to_string(&t).unwrap(), "\"07:05\"");
        assert!(serde_json::from_str::<HhMm>("\"25:00\"").is_err());
    }

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
    fn missing_file_is_created_and_broken_file_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (s, err) = Settings::load_or_create(&path);
        assert_eq!((s, err), (Settings::default(), None));
        assert!(path.exists());

        fs::write(&path, "{ nope").unwrap();
        let (s, err) = Settings::load_or_create(&path);
        assert_eq!(s, Settings::default());
        assert!(err.unwrap().starts_with("settings.json"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ nope");
    }
}
