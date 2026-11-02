//! Which archives to keep, and which to merge into the next kept one.
//!
//! The rules follow restic's `forget`: every archive of the last
//! `keep_within_days` days is kept (backups just made are never merged);
//! for each other rule the newest archive of each period (day, ISO week,
//! month, year) is kept, for that many periods; an archive is kept when any
//! rule selects it; the newest archive is always kept. An incremental archive cannot simply be deleted because later ones
//! may need its content, so every run of archives that are not kept is
//! merged with the kept archive that follows it.
//!
//! The automatic merge never touches the initial complete archive
//! ([`automatic_keep_set`]); a manual merge of chosen archives may include
//! it ([`range_group`]).

use crate::{ArchiveKind, ArchiveName, Retention};
use jiff::civil::DateTime;
use std::ops::RangeInclusive;

/// Which archives the rules keep at local time `now`. `archives` must be
/// sorted oldest first.
#[must_use]
pub fn keep_set(archives: &[ArchiveName], rules: &Retention, now: DateTime) -> Vec<bool> {
    let n = archives.len();
    if rules.keeps_everything() {
        return vec![true; n];
    }
    let mut keep = vec![false; n];
    if let Some(last) = keep.last_mut() {
        *last = true;
    }
    let times: Vec<DateTime> = archives.iter().map(|a| a.time).collect();
    for k in keep.iter_mut().rev().take(rules.keep_last as usize) {
        *k = true;
    }
    if rules.keep_within_days > 0 {
        let hours = i64::from(rules.keep_within_days).saturating_mul(24);
        // An unrepresentable date (far past) keeps nothing more.
        if let Ok(since) = now.checked_sub(jiff::SignedDuration::from_hours(hours)) {
            for (k, t) in keep.iter_mut().zip(&times) {
                *k |= *t > since;
            }
        }
    }
    keep_per_period(&mut keep, &times, rules.keep_daily, |t| {
        (i32::from(t.year()), i32::from(t.month()), i32::from(t.day()))
    });
    keep_per_period(&mut keep, &times, rules.keep_weekly, |t| {
        let w = t.date().iso_week_date();
        (i32::from(w.year()), i32::from(w.week()), 0)
    });
    keep_per_period(&mut keep, &times, rules.keep_monthly, |t| {
        (i32::from(t.year()), i32::from(t.month()), 0)
    });
    keep_per_period(&mut keep, &times, rules.keep_yearly, |t| (i32::from(t.year()), 0, 0));
    keep
}

/// [`keep_set`] for the automatic merge: the initial complete archive is
/// always kept, so it is never merged automatically.
#[must_use]
pub fn automatic_keep_set(archives: &[ArchiveName], rules: &Retention, now: DateTime) -> Vec<bool> {
    let mut keep = keep_set(archives, rules, now);
    if let (Some(first), Some(k)) = (archives.first(), keep.first_mut())
        && first.kind == ArchiveKind::Complete
    {
        *k = true;
    }
    keep
}

/// Why a manual selection cannot be merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RangeError {
    #[error("choose at least two backups")]
    TooFew,
    #[error("a chosen backup does not exist")]
    Unknown,
}

/// Index range of the archives from `first` to `last` (in either order),
/// for a manual merge. Everything between them is merged too, because an
/// incremental archive cannot skip the ones before it.
pub fn range_group(
    archives: &[ArchiveName],
    first: &ArchiveName,
    last: &ArchiveName,
) -> Result<RangeInclusive<usize>, RangeError> {
    let a = archives.iter().position(|x| x == first).ok_or(RangeError::Unknown)?;
    let b = archives.iter().position(|x| x == last).ok_or(RangeError::Unknown)?;
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    if lo == hi { Err(RangeError::TooFew) } else { Ok(lo..=hi) }
}

/// True when a manual merge of `range` replaces the initial complete
/// archive (the window warns before doing it).
#[must_use]
pub fn includes_initial_complete(archives: &[ArchiveName], range: &RangeInclusive<usize>) -> bool {
    *range.start() == 0 && archives.first().is_some_and(|a| a.kind == ArchiveKind::Complete)
}

/// How one archive is deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deletion {
    /// Nothing depends on it (the newest, or the only one): remove it.
    Remove(usize),
    /// Later archives may need its content: merge it with the next one,
    /// which keeps its date. Versions found only in the deleted archive are
    /// lost.
    MergeWithNext(RangeInclusive<usize>),
}

/// Why an archive cannot be deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeleteError {
    #[error("the backup does not exist")]
    Unknown,
    #[error("a complete backup cannot be deleted")]
    Complete,
    #[error("the initial complete backup can be deleted only when it is the only one")]
    InitialWithOthers,
}

/// How to delete `name`. Incremental archives can always be deleted; the
/// initial complete archive only when it is the only one; later complete
/// archives never (they start a chain other archives depend on).
pub fn deletion(archives: &[ArchiveName], name: &ArchiveName) -> Result<Deletion, DeleteError> {
    let i = archives.iter().position(|a| a == name).ok_or(DeleteError::Unknown)?;
    let newest = i + 1 == archives.len();
    match name.kind {
        ArchiveKind::Complete if i == 0 && newest => Ok(Deletion::Remove(0)),
        ArchiveKind::Complete if i == 0 => Err(DeleteError::InitialWithOthers),
        ArchiveKind::Complete => Err(DeleteError::Complete),
        ArchiveKind::Incremental if newest => Ok(Deletion::Remove(i)),
        ArchiveKind::Incremental => Ok(Deletion::MergeWithNext(i..=i + 1)),
    }
}

/// Keeps the newest archive of each of the `count` most recent periods.
fn keep_per_period(
    keep: &mut [bool],
    times: &[DateTime],
    count: u32,
    period: impl Fn(DateTime) -> (i32, i32, i32),
) {
    let mut seen = 0;
    let mut last = None;
    for (i, t) in times.iter().enumerate().rev() {
        let p = period(*t);
        if last != Some(p) {
            if seen == count {
                break;
            }
            seen += 1;
            last = Some(p);
            if let Some(k) = keep.get_mut(i) {
                *k = true;
            }
        }
    }
}

/// Index ranges to merge: each ends with a kept archive and starts after the
/// previous kept one. Single kept archives are not listed.
#[must_use]
pub fn merge_groups(keep: &[bool]) -> Vec<RangeInclusive<usize>> {
    let mut groups = Vec::new();
    let mut start = None;
    for (i, &k) in keep.iter().enumerate() {
        if !k {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            groups.push(s..=i);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArchiveKind;
    use jiff::civil::date;

    fn daily(n: i8) -> Vec<ArchiveName> {
        (1..=n)
            .map(|d| ArchiveName {
                time: date(2026, 3, d).at(12, 0, 0, 0),
                kind: if d == 1 { ArchiveKind::Complete } else { ArchiveKind::Incremental },
            })
            .collect()
    }

    /// Long after every test archive: the recent-days rule keeps nothing.
    fn later() -> DateTime {
        date(2030, 1, 1).at(0, 0, 0, 0)
    }

    fn rules(last: u32, daily: u32, weekly: u32, monthly: u32, yearly: u32) -> Retention {
        Retention {
            keep_within_days: 0,
            keep_last: last,
            keep_daily: daily,
            keep_weekly: weekly,
            keep_monthly: monthly,
            keep_yearly: yearly,
        }
    }

    #[test]
    fn keep_everything_merges_nothing() {
        let keep = keep_set(&daily(5), &Retention::KEEP_ALL, later());
        assert!(keep.iter().all(|&k| k));
        assert!(merge_groups(&keep).is_empty());
    }

    #[test]
    fn newest_is_always_kept() {
        let keep = keep_set(&daily(3), &rules(0, 0, 0, 0, 1), later());
        assert_eq!(keep, vec![false, false, true]);
        assert_eq!(merge_groups(&keep), vec![0..=2]);
    }

    #[test]
    fn three_daily_keeps_the_last_three_days() {
        let keep = keep_set(&daily(6), &rules(0, 3, 0, 0, 0), later());
        assert_eq!(keep, vec![false, false, false, true, true, true]);
        assert_eq!(merge_groups(&keep), vec![0..=3]);
    }

    #[test]
    fn two_backups_on_the_same_day_keep_the_newer() {
        let mut a = daily(2);
        a.insert(1, ArchiveName { time: date(2026, 3, 1).at(18, 0, 0, 0), kind: ArchiveKind::Incremental });
        // 1 Mar 12:00, 1 Mar 18:00, 2 Mar 12:00
        let keep = keep_set(&a, &rules(0, 2, 0, 0, 0), later());
        assert_eq!(keep, vec![false, true, true]);
    }

    #[test]
    fn weekly_rule_keeps_one_per_iso_week() {
        // 1–14 March 2026: Sunday 1, Sunday 8, Saturday 14 end ISO weeks 9, 10, 11.
        let keep = keep_set(&daily(14), &rules(0, 0, 3, 0, 0), later());
        let kept: Vec<usize> = keep.iter().enumerate().filter(|(_, k)| **k).map(|(i, _)| i + 1).collect();
        assert_eq!(kept, vec![1, 8, 14]);
        assert_eq!(merge_groups(&keep), vec![1..=7, 8..=13]);
    }

    #[test]
    fn rules_combine() {
        let keep = keep_set(&daily(10), &rules(1, 2, 0, 1, 0), later());
        // last + days 9,10 + month of March -> newest (10); so 9 and 10.
        assert_eq!(keep.iter().filter(|k| **k).count(), 2);
    }

    #[test]
    fn automatic_merge_never_touches_the_initial_complete() {
        let a = daily(6);
        let r = rules(1, 0, 0, 0, 0);
        assert!(!keep_set(&a, &r, later())[0]);
        let keep = automatic_keep_set(&a, &r, later());
        assert_eq!(keep, vec![true, false, false, false, false, true]);
        assert_eq!(merge_groups(&keep), vec![1..=5]);
    }

    #[test]
    fn backups_of_the_last_day_are_never_merged() {
        // The user's case: initial complete, two incrementals and a new
        // complete made minutes apart, then the automatic merge runs.
        let at = |d: i8, h: i8, m: i8, kind| ArchiveName { time: date(2026, 11, d).at(h, m, 0, 0), kind };
        let a = vec![
            at(1, 12, 26, ArchiveKind::Complete),
            at(1, 17, 15, ArchiveKind::Incremental),
            at(1, 17, 22, ArchiveKind::Incremental),
            at(1, 17, 22, ArchiveKind::Complete),
        ];
        // Only the newest by count, so that the daily rule decides.
        let r = Retention { keep_last: 1, ..Retention::default() };
        let now = date(2026, 11, 1).at(17, 25, 0, 0);
        assert!(merge_groups(&automatic_keep_set(&a, &r, now)).is_empty());
        // A day later the daily rule keeps only the newest of 1 November.
        let tomorrow = date(2026, 11, 2).at(17, 30, 0, 0);
        assert_eq!(merge_groups(&automatic_keep_set(&a, &r, tomorrow)), vec![1..=3]);
    }

    #[test]
    fn the_recent_days_rule_counts_whole_days_back_from_now() {
        let a = daily(5); // 1–5 March at 12:00
        let r = Retention { keep_within_days: 2, ..rules(0, 0, 0, 0, 1) };
        let keep = keep_set(&a, &r, date(2026, 3, 5).at(13, 0, 0, 0));
        assert_eq!(keep, vec![false, false, false, true, true]);
    }

    #[test]
    fn manual_range_spans_everything_between_and_flags_the_complete() {
        let a = daily(5);
        assert_eq!(range_group(&a, &a[3], &a[1]), Ok(1..=3));
        assert!(!includes_initial_complete(&a, &(1..=3)));
        let g = range_group(&a, &a[0], &a[2]).unwrap();
        assert!(includes_initial_complete(&a, &g));
        assert_eq!(range_group(&a, &a[2], &a[2]), Err(RangeError::TooFew));
        let other = ArchiveName { time: date(2030, 1, 1).at(0, 0, 0, 0), kind: ArchiveKind::Incremental };
        assert_eq!(range_group(&a, &a[0], &other), Err(RangeError::Unknown));
    }

    #[test]
    fn deleting_follows_the_chain_rules() {
        let a = daily(4); // C I I I
        assert_eq!(deletion(&a, &a[1]), Ok(Deletion::MergeWithNext(1..=2)));
        assert_eq!(deletion(&a, &a[3]), Ok(Deletion::Remove(3)));
        assert_eq!(deletion(&a, &a[0]), Err(DeleteError::InitialWithOthers));
        let only = daily(1);
        assert_eq!(deletion(&only, &only[0]), Ok(Deletion::Remove(0)));
        let mut with_second = daily(3);
        with_second[2].kind = ArchiveKind::Complete;
        assert_eq!(deletion(&with_second, &with_second[2]), Err(DeleteError::Complete));
        let other = ArchiveName { time: date(2030, 1, 1).at(0, 0, 0, 0), kind: ArchiveKind::Incremental };
        assert_eq!(deletion(&a, &other), Err(DeleteError::Unknown));
    }

    #[test]
    fn groups_end_at_kept_archives() {
        assert_eq!(merge_groups(&[true, false, true, false, false, true]), vec![1..=2, 3..=5]);
        assert!(merge_groups(&[]).is_empty());
    }

    #[test]
    fn every_group_ends_with_a_kept_archive() {
        for n in 1..20 {
            for r in [rules(0, 3, 1, 0, 0), rules(2, 0, 0, 1, 1), rules(0, 0, 0, 0, 1)] {
                let a = daily(n);
                let keep = keep_set(&a, &r, later());
                assert!(keep.last().copied().unwrap_or(true));
                for g in merge_groups(&keep) {
                    assert!(keep[*g.end()]);
                    assert!(g.clone().take(g.clone().count() - 1).all(|i| !keep[i]));
                }
            }
        }
    }
}
