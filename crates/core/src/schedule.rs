//! When a profile runs automatically.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Automatic run schedule of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "every", rename_all = "snake_case")]
pub enum Schedule {
    /// Only when the user presses "Back up now".
    #[default]
    Manual,
    Daily {
        at: TimeOfDay,
    },
    Weekly {
        day: Weekday,
        at: TimeOfDay,
    },
}

impl Schedule {
    #[must_use]
    pub fn is_automatic(&self) -> bool {
        !matches!(self, Self::Manual)
    }
}

/// A validated local time of day (hour 0–23, minute 0–59).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "RawTime", into = "RawTime")]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
}

impl TimeOfDay {
    /// Returns `None` when the hour or minute is out of range.
    #[must_use]
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }

    #[must_use]
    pub fn hour(self) -> u8 {
        self.hour
    }

    #[must_use]
    pub fn minute(self) -> u8 {
        self.minute
    }
}

impl Default for TimeOfDay {
    /// 12:30, when a desktop computer is most likely to be on.
    fn default() -> Self {
        Self { hour: 12, minute: 30 }
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Serialize, Deserialize)]
struct RawTime {
    hour: u8,
    minute: u8,
}

impl TryFrom<RawTime> for TimeOfDay {
    type Error = String;
    fn try_from(raw: RawTime) -> Result<Self, Self::Error> {
        Self::new(raw.hour, raw.minute).ok_or_else(|| "time of day out of range".to_owned())
    }
}

impl From<TimeOfDay> for RawTime {
    fn from(t: TimeOfDay) -> Self {
        Self { hour: t.hour, minute: t.minute }
    }
}

/// Day of the week, Monday first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    #[default]
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_of_day_rejects_out_of_range() {
        assert!(TimeOfDay::new(23, 59).is_some());
        assert!(TimeOfDay::new(24, 0).is_none());
        assert!(TimeOfDay::new(0, 60).is_none());
    }

    #[test]
    fn invalid_time_in_file_is_an_error() {
        let r: Result<TimeOfDay, _> = serde_json::from_str(r#"{"hour":25,"minute":0}"#);
        assert!(r.is_err());
    }

    #[test]
    fn schedule_round_trips() {
        let s = Schedule::Weekly { day: Weekday::Friday, at: TimeOfDay::new(18, 5).unwrap() };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Schedule>(&json).unwrap(), s);
    }

    #[test]
    fn time_displays_with_two_digits() {
        assert_eq!(TimeOfDay::new(7, 5).unwrap().to_string(), "07:05");
    }
}
