//! Which archives survive a merge (restic `forget`-style rules).

use serde::{Deserialize, Serialize};

/// Rules applied by the merge. Each field keeps the newest archive of that
/// many periods; an archive is kept when any rule selects it. All zero means
/// "keep every archive" (the merge does nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Retention {
    pub keep_last: u32,
    pub keep_daily: u32,
    pub keep_weekly: u32,
    pub keep_monthly: u32,
    pub keep_yearly: u32,
}

impl Default for Retention {
    /// A week of daily archives, a month of weekly ones, a year of monthly
    /// ones and three yearly ones.
    fn default() -> Self {
        Self { keep_last: 1, keep_daily: 7, keep_weekly: 4, keep_monthly: 12, keep_yearly: 3 }
    }
}

impl Retention {
    /// Never merge.
    pub const KEEP_ALL: Self =
        Self { keep_last: 0, keep_daily: 0, keep_weekly: 0, keep_monthly: 0, keep_yearly: 0 };

    #[must_use]
    pub fn keeps_everything(&self) -> bool {
        *self == Self::KEEP_ALL
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
    }
}
