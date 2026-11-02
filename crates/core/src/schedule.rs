//! When a profile runs automatically.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Automatic run schedule of a profile.
///
/// `Daily` and `Weekly` are the forms written before November 2026; they
/// still load and run, and the editor turns them into `Days` on the first
/// change ([`Schedule::normalized`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
    /// On each of `days`, at each of `times`.
    Days {
        days: Vec<Weekday>,
        times: Vec<TimeOfDay>,
    },
    /// Every day, every `hours` hours from `from`; the last run starts
    /// before `until` (`until` equal to `from`: around the clock).
    Hours {
        hours: u8,
        from: TimeOfDay,
        until: TimeOfDay,
    },
    /// Once a month, on `day` (1–28, so every month has it) at `at`.
    Monthly {
        day: u8,
        at: TimeOfDay,
    },
}

/// At most this many times a day in [`Schedule::Days`].
pub const MAX_TIMES_A_DAY: usize = 6;
/// Interval of [`Schedule::Hours`].
pub const HOURS_RANGE: std::ops::RangeInclusive<u8> = 1..=12;
/// Day of the month of [`Schedule::Monthly`].
pub const MONTH_DAY_RANGE: std::ops::RangeInclusive<u8> = 1..=28;

impl Schedule {
    #[must_use]
    pub fn is_automatic(&self) -> bool {
        !matches!(self, Self::Manual)
    }

    /// The same schedule in its current form: `Daily` and `Weekly` become
    /// `Days`; days and times sorted without repeats (an empty list becomes
    /// every day, or the default time); at most [`MAX_TIMES_A_DAY`] times;
    /// numbers brought within their ranges.
    #[must_use]
    pub fn normalized(&self) -> Self {
        match self {
            Self::Manual => Self::Manual,
            Self::Daily { at } => Self::Days { days: Weekday::ALL.to_vec(), times: vec![*at] },
            Self::Weekly { day, at } => Self::Days { days: vec![*day], times: vec![*at] },
            Self::Days { days, times } => {
                let mut days = days.clone();
                days.sort_unstable();
                days.dedup();
                if days.is_empty() {
                    days = Weekday::ALL.to_vec();
                }
                let mut times = times.clone();
                times.sort_unstable();
                times.dedup();
                times.truncate(MAX_TIMES_A_DAY);
                if times.is_empty() {
                    times.push(TimeOfDay::default());
                }
                Self::Days { days, times }
            }
            Self::Hours { hours, from, until } => Self::Hours {
                hours: (*hours).clamp(*HOURS_RANGE.start(), *HOURS_RANGE.end()),
                from: *from,
                until: *until,
            },
            Self::Monthly { day, at } => {
                Self::Monthly { day: (*day).clamp(*MONTH_DAY_RANGE.start(), *MONTH_DAY_RANGE.end()), at: *at }
            }
        }
    }
}

/// A validated local time of day (hour 0–23, minute 0–59).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    /// Minutes since midnight.
    #[must_use]
    pub fn minutes(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
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

/// Day of the week, Monday first (also the sort order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
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
    fn old_forms_become_days_and_lists_are_cleaned() {
        let at = TimeOfDay::new(9, 0).unwrap();
        let late = TimeOfDay::new(21, 0).unwrap();
        assert_eq!(
            Schedule::Daily { at }.normalized(),
            Schedule::Days { days: Weekday::ALL.to_vec(), times: vec![at] }
        );
        assert_eq!(
            Schedule::Weekly { day: Weekday::Friday, at }.normalized(),
            Schedule::Days { days: vec![Weekday::Friday], times: vec![at] }
        );
        let messy = Schedule::Days {
            days: vec![Weekday::Sunday, Weekday::Monday, Weekday::Sunday],
            times: vec![late, at, late],
        };
        assert_eq!(
            messy.normalized(),
            Schedule::Days { days: vec![Weekday::Monday, Weekday::Sunday], times: vec![at, late] }
        );
        let empty = Schedule::Days { days: Vec::new(), times: Vec::new() }.normalized();
        assert_eq!(empty, Schedule::Days { days: Weekday::ALL.to_vec(), times: vec![TimeOfDay::default()] });
        let many: Vec<TimeOfDay> = (0..10).map(|h| TimeOfDay::new(h, 0).unwrap()).collect();
        let Schedule::Days { times, .. } =
            (Schedule::Days { days: vec![Weekday::Monday], times: many }).normalized()
        else {
            panic!("days")
        };
        assert_eq!(times.len(), MAX_TIMES_A_DAY);
    }

    #[test]
    fn numbers_are_brought_within_range() {
        let at = TimeOfDay::default();
        assert_eq!(
            Schedule::Hours { hours: 0, from: at, until: at }.normalized(),
            Schedule::Hours { hours: 1, from: at, until: at }
        );
        assert_eq!(Schedule::Monthly { day: 31, at }.normalized(), Schedule::Monthly { day: 28, at });
    }

    #[test]
    fn every_form_round_trips_through_toml() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Wrap {
            schedule: Schedule,
        }
        let at = TimeOfDay::new(8, 15).unwrap();
        for schedule in [
            Schedule::Manual,
            Schedule::Days { days: vec![Weekday::Monday, Weekday::Thursday], times: vec![at] },
            Schedule::Hours { hours: 3, from: at, until: TimeOfDay::new(20, 0).unwrap() },
            Schedule::Monthly { day: 1, at },
        ] {
            let w = Wrap { schedule };
            let text = toml::to_string(&w).unwrap();
            assert_eq!(toml::from_str::<Wrap>(&text).unwrap(), w, "{text}");
        }
    }

    #[test]
    fn time_displays_with_two_digits() {
        assert_eq!(TimeOfDay::new(7, 5).unwrap().to_string(), "07:05");
    }
}
