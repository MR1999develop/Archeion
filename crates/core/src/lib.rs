//! Domain model of Archeion.
//!
//! Backups are stored as Areca-style **archives**: the first one is
//! complete, the following ones are incremental and hold only new or
//! changed files. Every archive carries a [`manifest`] listing *all* files
//! present at that time, so any date can be restored and deletions are
//! visible. Old archives are not deleted but **merged** into the next one
//! that the [`Retention`] rules keep ([`merge_plan`]).
//!
//! A [`Profile`] says what to back up, where, when, and when to merge. The
//! [`run`] module defines the events a run reports, shared by the window and
//! by the headless worker started by Windows Task Scheduler; [`restore`]
//! what a restore is asked to do and the names of the files it writes.
//!
//! This crate performs no I/O.

pub mod archive;
pub mod backup_kind;
pub mod destination;
pub mod exclusion;
pub mod keys;
pub mod large_files;
pub mod manifest;
pub mod merge_plan;
pub mod password_policy;
pub mod profile;
pub mod restore;
pub mod retention;
pub mod run;
pub mod schedule;
pub mod settings;

pub use archive::{ArchiveKind, ArchiveName};
pub use backup_kind::{BackupMode, FullBackupRule};
pub use destination::Destination;
pub use exclusion::Exclusion;
pub use large_files::LargeFiles;
pub use manifest::{ChunkId, ChunkRef, Chunks, Entry, EntryKind, Manifest, ManifestHeader};
pub use profile::{Alerts, Container, EventTriggers, MergePolicy, Profile, ProfileError, ProfileId};
pub use restore::{RestoreItem, RestoreRequest, RestoreTarget};
pub use retention::Retention;
pub use run::{
    ArchiveInfo, BackupReport, FileAction, FileDone, MergeGroup, MergeReport, Phase, PhaseDetail,
    RestoreReport, RunEvent, Skipped, Summary, Tally, VerifyReport,
};
pub use schedule::{Schedule, TimeOfDay, Weekday};
pub use settings::{Language, Settings, WindowOptions, WindowSize};
