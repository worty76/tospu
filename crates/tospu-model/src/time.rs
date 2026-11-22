use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wall-clock time of day, stored as minutes since midnight and written
/// as `"HH:MM"` in settings.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}
