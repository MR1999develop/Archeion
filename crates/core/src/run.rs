//! Events and reports of a run (backup, merge, restore, verify).
//!
//! The headless worker (`archeion run|merge <profile>`) writes one JSON
//! [`RunEvent`] per line on stdout; the window reads them to show progress.

use crate::{ArchiveKind, Container};
use serde::{Deserialize, Serialize};

/// Stage of a run, shown next to the progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Opening,
    Scanning,
    Copying,
    Merging,
    Restoring,
    Verifying,
}

/// A file that could not be read or written, typically because another
/// program has it locked (no Volume Shadow Copy is used).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReport {
    /// Name of the new archive.
    pub archive: String,
    /// The new archive is complete (a new chain).
    #[serde(default)]
    pub complete: bool,
    pub files_new: u64,
    pub files_changed: u64,
    pub files_unchanged: u64,
    pub files_deleted: u64,
    pub bytes_copied: u64,
    pub skipped: Vec<Skipped>,
    /// Files already copied by an interrupted run of the same backup and
    /// kept as they were.
    #[serde(default)]
    pub resumed_files: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeReport {
    /// Archives before and after the merge.
    pub archives_before: u64,
    pub archives_after: u64,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreReport {
    pub files_restored: u64,
    /// Already present and not overwritten.
    pub files_kept: u64,
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    pub files_checked: u64,
    /// Missing or different content, one line each.
    pub problems: Vec<String>,
}

/// Result of a finished run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Summary {
    Backup(BackupReport),
    Merge(MergeReport),
    Restore(RestoreReport),
    Verify(VerifyReport),
}

/// One step of a run's progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum RunEvent {
    Phase {
        phase: Phase,
    },
    /// How far the current phase is. Totals of 0 are unknown (for
    /// example while scanning, where `files_done` counts the files found).
    Progress {
        done: u64,
        total: u64,
        /// Files started so far: the current file is number `files_done`.
        #[serde(default)]
        files_done: u64,
        #[serde(default)]
        files_total: u64,
        /// File or folder being worked on, as a full path.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        current: String,
    },
    Skipped(Skipped),
    Finished {
        summary: Summary,
    },
    /// Drive unplugged or share offline: Task Scheduler retries later, the
    /// window asks to connect it.
    DestinationUnavailable {
        location: String,
    },
    /// Another backup or merge of this profile is running.
    Busy,
    /// The destination is encrypted and no password is saved for it.
    PasswordNeeded,
    /// The saved password does not open the destination.
    WrongPassword,
    /// The destination has too little free space; nothing was committed.
    /// What was already copied is kept and reused by the next run.
    NotEnoughSpace {
        needed: u64,
        available: u64,
    },
    Failed {
        message: String,
    },
}

impl RunEvent {
    /// One line of the stdout protocol, without the trailing newline.
    #[must_use]
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("RunEvent always serializes")
    }

    /// Parses one protocol line.
    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim_end())
    }

    /// True for the last event of a run.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Finished { .. }
                | Self::Failed { .. }
                | Self::DestinationUnavailable { .. }
                | Self::Busy
                | Self::PasswordNeeded
                | Self::WrongPassword
                | Self::NotEnoughSpace { .. }
        )
    }
}

/// An archive as listed in the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveInfo {
    pub name: String,
    pub kind: ArchiveKind,
    /// Local time, `YYYY-MM-DD HH:MM`.
    pub time_label: String,
    /// Files present at that time.
    pub files_total: u64,
    /// Files whose content this archive holds.
    pub files_stored: u64,
    pub bytes_stored: u64,
    pub container: Container,
}

impl Serialize for ArchiveKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Self::Complete => "complete",
            Self::Incremental => "incremental",
        })
    }
}

impl<'de> Deserialize<'de> for ArchiveKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match String::deserialize(d)?.as_str() {
            "complete" => Ok(Self::Complete),
            "incremental" => Ok(Self::Incremental),
            other => Err(serde::de::Error::unknown_variant(other, &["complete", "incremental"])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_round_trips_through_a_line() {
        let events = [
            RunEvent::Phase { phase: Phase::Copying },
            RunEvent::Progress {
                done: 10,
                total: 100,
                files_done: 3,
                files_total: 9,
                current: r"C:\a b\c.txt".into(),
            },
            RunEvent::Skipped(Skipped { path: r"C:\a b\ü.pst".into(), reason: "locked".into() }),
            RunEvent::Finished { summary: Summary::Backup(BackupReport::default()) },
            RunEvent::Finished {
                summary: Summary::Merge(MergeReport { bytes_freed: 5, ..MergeReport::default() }),
            },
            RunEvent::DestinationUnavailable { location: r"E:\Backup".into() },
            RunEvent::Busy,
            RunEvent::PasswordNeeded,
            RunEvent::WrongPassword,
            RunEvent::NotEnoughSpace { needed: 10, available: 3 },
            RunEvent::Failed { message: "line\nbreak".into() },
        ];
        for ev in events {
            let line = ev.to_line();
            assert!(!line.contains('\n'), "one event per line: {line}");
            assert_eq!(RunEvent::from_line(&format!("{line}\r\n")).unwrap(), ev);
        }
    }

    #[test]
    fn only_final_events_are_terminal() {
        assert!(!RunEvent::from_line(r#"{"event":"progress","done":0,"total":0}"#).unwrap().is_terminal());
        assert!(RunEvent::Busy.is_terminal());
    }
}
