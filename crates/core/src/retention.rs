//! Which archives survive a merge (restic `forget`-style rules).

use serde::{Deserialize, Serialize};

/// Rules applied by the merge. `keep_within_days` keeps every archive of
/// that many recent days, so today's backups are never merged right after
/// they are made; each other field keeps the newest archive of that many
/// periods. An archive is kept when any rule selects it. All the period
/// rules at zero means "keep every archive" (the merge does nothing),
/// whatever `keep_within_days` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Retention {
    /// Every archive younger than this many days (24 hours each).
    pub keep_within_days: u32,
    pub keep_last: u32,
    pub keep_daily: u32,
    pub keep_weekly: u32,
    pub keep_monthly: u32,
    pub keep_yearly: u32,
}

impl Default for Retention {
    /// Every archive of the last day and the five most recent, then a
    /// week of daily archives, a month of weekly ones, a year of monthly
    /// ones and ten yearly ones.
    fn default() -> Self {
        Self {
            keep_within_days: 1,
            keep_last: 5,
            keep_daily: 7,
            keep_weekly: 4,
            keep_monthly: 12,
            keep_yearly: 10,
        }
    }
}

impl Retention {
    /// Never merge.
    pub const KEEP_ALL: Self = Self {
        keep_within_days: 0,
        keep_last: 0,
        keep_daily: 0,
        keep_weekly: 0,
        keep_monthly: 0,
        keep_yearly: 0,
    };

    /// True when no period rule is set: nothing is ever merged. The
    /// recent-days rule alone only keeps, it never selects what to merge.
    #[must_use]
    pub fn keeps_everything(&self) -> bool {
        Self { keep_within_days: 0, ..*self } == Self::KEEP_ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let r: Retention = serde_json::from_str(r#"{"keep_daily":3}"#).unwrap();
        assert_eq!(r.keep_daily, 3);
        assert_eq!(r.keep_monthly, Retention::default().keep_monthly);
    }

    #[test]
    fn only_all_zero_keeps_everything() {
        assert!(Retention::KEEP_ALL.keeps_everything());
        assert!(!Retention::default().keeps_everything());
        let recent_only = Retention { keep_within_days: 7, ..Retention::KEEP_ALL };
        assert!(recent_only.keeps_everything());
    }

    #[test]
    fn rules_saved_before_the_recent_days_rule_keep_the_last_day() {
        let r: Retention = serde_json::from_str(
            r#"{"keep_last":1,"keep_daily":7,"keep_weekly":4,"keep_monthly":12,"keep_yearly":3}"#,
        )
        .unwrap();
        assert_eq!(r.keep_within_days, 1);
        let never: Retention = serde_json::from_str(
            r#"{"keep_last":0,"keep_daily":0,"keep_weekly":0,"keep_monthly":0,"keep_yearly":0}"#,
        )
        .unwrap();
        assert!(never.keeps_everything(), "a profile that never merged still never merges");
    }
}
