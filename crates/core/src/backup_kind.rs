//! Whether the next backup is complete or incremental.
//!
//! A complete backup copies every file and starts a new chain: the
//! incrementals after it depend only on it. The user can ask for one at any
//! time; scheduled backups follow the profile's [`FullBackupRule`].

use crate::{ArchiveKind, ArchiveName};
use jiff::{Span, civil::DateTime};
use serde::{Deserialize, Serialize};

/// When scheduled backups make a complete backup instead of an incremental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "every", rename_all = "snake_case")]
pub enum FullBackupRule {
    /// Only the first backup is complete.
    #[default]
    Never,
    /// Every `count` backups, the complete one included: 7 gives one
    /// complete backup followed by six incrementals.
    Backups { count: u32 },
    /// When the newest complete backup is at least `count` days old.
    Days { count: u32 },
}

/// The kind of backup asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupMode {
    /// As the profile's [`FullBackupRule`] says (scheduled backups).
    #[default]
    Auto,
    Incremental,
    Complete,
}

/// Kind of the next archive. `archives` must be sorted oldest first; the
/// first backup of a destination is always complete. `new_chain` asks an
/// automatic backup to be complete whatever the rule says: the newest
/// chain was written before files could be chunked (see
/// [`crate::large_files::needs_new_chain`]); the user's explicit choice
/// still wins.
#[must_use]
pub fn next_kind(
    archives: &[ArchiveName],
    rule: FullBackupRule,
    mode: BackupMode,
    now: DateTime,
    new_chain: bool,
) -> ArchiveKind {
    if archives.is_empty() {
        return ArchiveKind::Complete;
    }
    let complete = match mode {
        BackupMode::Complete => true,
        BackupMode::Incremental => false,
        BackupMode::Auto => new_chain || rule_due(archives, rule, now),
    };
    if complete { ArchiveKind::Complete } else { ArchiveKind::Incremental }
}

fn rule_due(archives: &[ArchiveName], rule: FullBackupRule, now: DateTime) -> bool {
    match rule {
        FullBackupRule::Never => false,
        FullBackupRule::Backups { count } => {
            let since = archives.iter().rev().take_while(|a| a.kind == ArchiveKind::Incremental).count();
            since.saturating_add(1) >= count as usize
        }
        FullBackupRule::Days { count } => {
            match archives.iter().rev().find(|a| a.kind == ArchiveKind::Complete) {
                None => true,
                Some(c) => c.time.checked_add(Span::new().days(i64::from(count))).is_ok_and(|due| now >= due),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn chain(kinds: &str) -> Vec<ArchiveName> {
        kinds
            .chars()
            .zip(1i8..)
            .map(|(k, d)| ArchiveName {
                time: date(2026, 3, d).at(12, 0, 0, 0),
                kind: if k == 'C' { ArchiveKind::Complete } else { ArchiveKind::Incremental },
            })
            .collect()
    }

    fn at(day: i8) -> DateTime {
        date(2026, 3, day).at(13, 0, 0, 0)
    }

    #[test]
    fn first_backup_is_always_complete() {
        for mode in [BackupMode::Auto, BackupMode::Incremental, BackupMode::Complete] {
            assert_eq!(next_kind(&[], FullBackupRule::Never, mode, at(1), false), ArchiveKind::Complete);
        }
    }

    #[test]
    fn the_users_choice_wins_over_the_rule() {
        let a = chain("CIIIIII");
        let rule = FullBackupRule::Backups { count: 3 };
        assert_eq!(next_kind(&a, rule, BackupMode::Incremental, at(9), false), ArchiveKind::Incremental);
        let rule = FullBackupRule::Never;
        assert_eq!(next_kind(&a, rule, BackupMode::Complete, at(9), false), ArchiveKind::Complete);
    }

    #[test]
    fn every_n_backups_counts_the_complete_one() {
        let rule = FullBackupRule::Backups { count: 3 };
        assert_eq!(next_kind(&chain("C"), rule, BackupMode::Auto, at(9), false), ArchiveKind::Incremental);
        assert_eq!(next_kind(&chain("CI"), rule, BackupMode::Auto, at(9), false), ArchiveKind::Incremental);
        assert_eq!(next_kind(&chain("CII"), rule, BackupMode::Auto, at(9), false), ArchiveKind::Complete);
        assert_eq!(
            next_kind(&chain("CIICI"), rule, BackupMode::Auto, at(9), false),
            ArchiveKind::Incremental
        );
        let always = FullBackupRule::Backups { count: 1 };
        assert_eq!(next_kind(&chain("C"), always, BackupMode::Auto, at(9), false), ArchiveKind::Complete);
    }

    #[test]
    fn every_n_days_looks_at_the_newest_complete() {
        let rule = FullBackupRule::Days { count: 7 };
        let a = chain("CIIC"); // newest complete on 4 March
        assert_eq!(next_kind(&a, rule, BackupMode::Auto, at(10), false), ArchiveKind::Incremental);
        assert_eq!(next_kind(&a, rule, BackupMode::Auto, at(11), false), ArchiveKind::Complete);
    }

    #[test]
    fn never_keeps_incrementals() {
        assert_eq!(
            next_kind(&chain("CIIIIIIIII"), FullBackupRule::Never, BackupMode::Auto, at(28), false),
            ArchiveKind::Incremental
        );
    }

    #[test]
    fn an_old_chain_makes_the_next_automatic_backup_complete() {
        let a = chain("CI");
        let never = FullBackupRule::Never;
        assert_eq!(next_kind(&a, never, BackupMode::Auto, at(9), true), ArchiveKind::Complete);
        assert_eq!(next_kind(&a, never, BackupMode::Incremental, at(9), true), ArchiveKind::Incremental);
        assert_eq!(next_kind(&a, never, BackupMode::Auto, at(9), false), ArchiveKind::Incremental);
    }

    #[test]
    fn rule_round_trips() {
        let r = FullBackupRule::Days { count: 30 };
        let text = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<FullBackupRule>(&text).unwrap(), r);
    }
}
