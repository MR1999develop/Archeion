//! Events and reports of a run (backup, merge, restore, verify).
//!
//! The headless worker (`archeion run|merge <profile>`) writes one JSON
//! [`RunEvent`] per line on stdout; the window reads them to show progress.
//! Every file a run handles is also reported, in batches, by
//! [`RunEvent::Files`]: the complete list of the run's full log.

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

/// Most recent files one progress event carries; the window shows as many.
pub const RECENT_FILES: usize = 20;

/// What happened to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    /// Backup: not in the previous backup (found while scanning, copied
    /// while copying).
    New,
    /// Backup: different from the previous backup.
    Changed,
    /// Backup: same as in the previous backup; a complete backup copies it
    /// again.
    Unchanged,
    /// Merge: carried into the merged backup.
    Moved,
    /// Restore: written.
    Restored,
    /// Restore: already present and not overwritten.
    Kept,
    /// Check: content matches.
    Checked,
    /// Check: content missing or different.
    Damaged,
    /// Backup: in the previous backup, no longer in the sources.
    Deleted,
    /// Merge or deletion: an older version no backup keeps any more.
    Dropped,
}

/// Files per [`FileAction`] so far in the current phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub new: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub changed: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unchanged: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub moved: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub restored: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub kept: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub checked: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub damaged: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub deleted: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dropped: u64,
}

impl Tally {
    /// Counts one more file with `action`.
    pub fn add(&mut self, action: FileAction) {
        let n = self.slot(action);
        *n = n.saturating_add(1);
    }

    /// Files counted with `action`.
    #[must_use]
    pub fn get(&self, action: FileAction) -> u64 {
        match action {
            FileAction::New => self.new,
            FileAction::Changed => self.changed,
            FileAction::Unchanged => self.unchanged,
            FileAction::Moved => self.moved,
            FileAction::Restored => self.restored,
            FileAction::Kept => self.kept,
            FileAction::Checked => self.checked,
            FileAction::Damaged => self.damaged,
            FileAction::Deleted => self.deleted,
            FileAction::Dropped => self.dropped,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn slot(&mut self, action: FileAction) -> &mut u64 {
        match action {
            FileAction::New => &mut self.new,
            FileAction::Changed => &mut self.changed,
            FileAction::Unchanged => &mut self.unchanged,
            FileAction::Moved => &mut self.moved,
            FileAction::Restored => &mut self.restored,
            FileAction::Kept => &mut self.kept,
            FileAction::Checked => &mut self.checked,
            FileAction::Damaged => &mut self.damaged,
            FileAction::Deleted => &mut self.deleted,
            FileAction::Dropped => &mut self.dropped,
        }
    }
}

/// One file handled: listed among the most recent ones and in the run's
/// complete list of files ([`RunEvent::Files`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDone {
    /// Full path.
    pub path: String,
    pub action: FileAction,
    /// Size of the file (of the dropped or deleted version for those).
    pub bytes: u64,
    /// Backup: saved as chunks, of which only the ones not already in the
    /// destination were written: this many bytes of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_chunk_bytes: Option<u64>,
    /// Backup: already copied by an interrupted run of the same backup and
    /// kept as it was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resumed: bool,
    /// What went wrong (a damaged file), in English.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl FileDone {
    /// A file with `action` and nothing more to say.
    #[must_use]
    pub fn new(path: impl Into<String>, action: FileAction, bytes: u64) -> Self {
        Self { path: path.into(), action, bytes, new_chunk_bytes: None, resumed: false, note: String::new() }
    }
}

/// The group of backups a merge is fusing now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeGroup {
    /// 1-based position among the groups of this run.
    pub index: u64,
    pub of: u64,
    /// Backups in the group.
    pub archives: u64,
    /// Oldest and newest backup of the group, as labelled in the window.
    pub first: String,
    pub last: String,
}

/// What the current phase has done so far, beyond bytes and files: shown
/// by the Activity tab. Empty from workers that do not report it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseDetail {
    /// Folders read so far (scanning).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub folders: u64,
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    pub tally: Tally,
    /// Files finished since the previous progress event, oldest first, at
    /// most [`RECENT_FILES`] (older ones are dropped, not counted twice).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<FileDone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<MergeGroup>,
}

impl PhaseDetail {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Signature required by `skip_serializing_if`.
fn is_zero(n: &u64) -> bool {
    *n == 0
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
    /// New or changed files saved as chunks, of which only the chunks not
    /// already in the destination were written.
    #[serde(default)]
    pub files_as_changes: u64,
    /// Files encrypted by Windows (EFS) saved without that encryption, to a
    /// destination that is not encrypted.
    #[serde(default)]
    pub files_efs_in_clear: u64,
    /// The encrypted destination held another list of backups than the one
    /// Archeion signed: backups were removed or put back from outside.
    #[serde(default)]
    pub changed_outside: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeReport {
    /// Archives before and after the merge.
    pub archives_before: u64,
    pub archives_after: u64,
    pub bytes_freed: u64,
    /// As [`BackupReport::changed_outside`].
    #[serde(default)]
    pub changed_outside: bool,
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
        /// Counts and most recent files of the phase.
        #[serde(default, skip_serializing_if = "PhaseDetail::is_empty")]
        detail: PhaseDetail,
    },
    Skipped(Skipped),
    /// Files handled since the previous `Files` event, in the order they
    /// were handled. Unlike [`PhaseDetail::recent`] none is ever dropped:
    /// together they are every file the run worked on, except the
    /// unchanged files of an incremental backup (only counted), the files
    /// a full check found intact (only counted) and the skipped files
    /// (reported by [`RunEvent::Skipped`]). Sent at least every second
    /// while files are handled and before the run's last event.
    Files {
        files: Vec<FileDone>,
    },
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
                detail: PhaseDetail {
                    folders: 2,
                    tally: Tally { new: 1, damaged: 4, ..Tally::default() },
                    recent: vec![FileDone::new(r"C:\a b\ü.txt", FileAction::New, 7)],
                    group: Some(MergeGroup {
                        index: 1,
                        of: 2,
                        archives: 3,
                        first: "2026-10-01 21:30".into(),
                        last: "2026-10-03 21:30".into(),
                    }),
                },
            },
            RunEvent::Skipped(Skipped { path: r"C:\a b\ü.pst".into(), reason: "locked".into() }),
            RunEvent::Files {
                files: vec![
                    FileDone::new(r"C:\x.txt", FileAction::Deleted, 3),
                    FileDone {
                        new_chunk_bytes: Some(4096),
                        resumed: true,
                        note: "differs".into(),
                        ..FileDone::new(r"C:\big.vhdx", FileAction::Changed, 1 << 33)
                    },
                ],
            },
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
    fn progress_without_detail_stays_short_and_old_lines_still_parse() {
        let bare = RunEvent::Progress {
            done: 1,
            total: 2,
            files_done: 0,
            files_total: 0,
            current: String::new(),
            detail: PhaseDetail::default(),
        };
        assert!(!bare.to_line().contains("detail"));
        let old = r#"{"event":"progress","done":1,"total":2,"files_done":0,"files_total":0}"#;
        assert_eq!(RunEvent::from_line(old).unwrap(), bare);
        let sparse = r#"{"event":"progress","done":0,"total":0,"detail":{"tally":{"kept":3}}}"#;
        let RunEvent::Progress { detail, .. } = RunEvent::from_line(sparse).unwrap() else { panic!() };
        assert_eq!(detail.tally.get(FileAction::Kept), 3);
        assert_eq!(detail.tally.get(FileAction::New), 0);
    }

    #[test]
    fn a_plain_file_line_stays_short_and_old_recent_files_still_parse() {
        let line = RunEvent::Files { files: vec![FileDone::new("a", FileAction::Dropped, 1)] }.to_line();
        assert_eq!(line, r#"{"event":"files","files":[{"path":"a","action":"dropped","bytes":1}]}"#);
        let old = r#"{"event":"progress","done":0,"total":0,"detail":{"recent":[{"path":"a","action":"new","bytes":2}]}}"#;
        let RunEvent::Progress { detail, .. } = RunEvent::from_line(old).unwrap() else { panic!() };
        assert_eq!(detail.recent, vec![FileDone::new("a", FileAction::New, 2)]);
    }

    #[test]
    fn a_tally_counts_each_action_apart() {
        let mut t = Tally::default();
        assert!(t.is_empty());
        t.add(FileAction::Changed);
        t.add(FileAction::Changed);
        t.add(FileAction::Moved);
        assert_eq!((t.get(FileAction::Changed), t.get(FileAction::Moved), t.get(FileAction::New)), (2, 1, 0));
        assert!(!t.is_empty());
    }

    #[test]
    fn only_final_events_are_terminal() {
        assert!(!RunEvent::from_line(r#"{"event":"progress","done":0,"total":0}"#).unwrap().is_terminal());
        assert!(RunEvent::Busy.is_terminal());
    }
}
