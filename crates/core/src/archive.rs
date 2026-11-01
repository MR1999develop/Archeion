//! Archive names: `YYYY-MM-DD_HHMMSS_C` (complete) or `_I` (incremental).
//!
//! The name is the archive's folder on the destination. Local time is used
//! so that the folders read naturally in File Explorer; names sort in time
//! order.

use jiff::civil::DateTime;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArchiveKind {
    /// Holds every file: the first archive, or the result of merging it.
    Complete,
    /// Holds only files new or changed since the previous archive.
    Incremental,
}

impl ArchiveKind {
    fn letter(self) -> char {
        match self {
            Self::Complete => 'C',
            Self::Incremental => 'I',
        }
    }
}

/// Name of one archive: its creation time (local, to the second) and kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchiveName {
    pub time: DateTime,
    pub kind: ArchiveKind,
}

impl ArchiveName {
    /// `2026-10-29 21:30`, for the interface.
    #[must_use]
    pub fn label(&self) -> String {
        let t = self.time;
        format!("{:04}-{:02}-{:02} {:02}:{:02}", t.year(), t.month(), t.day(), t.hour(), t.minute())
    }
}

impl PartialOrd for ArchiveName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ArchiveName {
    /// By time; at equal time (never in practice) complete first.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time).then(self.kind.cmp(&other.kind))
    }
}

impl fmt::Display for ArchiveName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.time;
        write!(
            f,
            "{:04}-{:02}-{:02}_{:02}{:02}{:02}_{}",
            t.year(),
            t.month(),
            t.day(),
            t.hour(),
            t.minute(),
            t.second(),
            self.kind.letter()
        )
    }
}

/// The text is not an archive name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not an archive name")]
pub struct NotAnArchiveName;

impl FromStr for ArchiveName {
    type Err = NotAnArchiveName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 2026-10-29_213000_C
        let b = s.as_bytes();
        if b.len() != 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'_' || b[17] != b'_' {
            return Err(NotAnArchiveName);
        }
        let num = |range: std::ops::Range<usize>| -> Result<i16, NotAnArchiveName> {
            let part = s.get(range).ok_or(NotAnArchiveName)?;
            if !part.bytes().all(|c| c.is_ascii_digit()) {
                return Err(NotAnArchiveName);
            }
            part.parse().map_err(|_| NotAnArchiveName)
        };
        let small = |range| -> Result<i8, NotAnArchiveName> {
            i8::try_from(num(range)?).map_err(|_| NotAnArchiveName)
        };
        let kind = match b[18] {
            b'C' => ArchiveKind::Complete,
            b'I' => ArchiveKind::Incremental,
            _ => return Err(NotAnArchiveName),
        };
        let time = DateTime::new(
            num(0..4)?,
            small(5..7)?,
            small(8..10)?,
            small(11..13)?,
            small(13..15)?,
            small(15..17)?,
            0,
        )
        .map_err(|_| NotAnArchiveName)?;
        Ok(Self { time, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn name_round_trips() {
        let n = ArchiveName { time: date(2026, 10, 29).at(21, 30, 5, 0), kind: ArchiveKind::Incremental };
        assert_eq!(n.to_string(), "2026-10-29_213005_I");
        assert_eq!("2026-10-29_213005_I".parse::<ArchiveName>().unwrap(), n);
    }

    #[test]
    fn other_folder_names_are_rejected() {
        for s in [
            "",
            "archeion.json",
            "2026-10-29_213005_X",
            "2026-13-01_000000_C",
            "2026-10-29_213005_C.partial",
            "2026-1a-29_213005_C",
        ] {
            assert!(s.parse::<ArchiveName>().is_err(), "{s}");
        }
    }

    #[test]
    fn names_sort_by_time() {
        let a: ArchiveName = "2026-01-01_000000_C".parse().unwrap();
        let b: ArchiveName = "2026-01-01_000001_I".parse().unwrap();
        assert!(a < b);
    }

    #[test]
    fn label_drops_seconds() {
        let n: ArchiveName = "2026-10-29_213005_C".parse().unwrap();
        assert_eq!(n.label(), "2026-10-29 21:30");
    }
}
