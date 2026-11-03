//! Archive engine of Archeion.
//!
//! Layout of a destination folder:
//!
//! ```text
//! archeion.json                  which profile owns the folder (and
//!                                whether it is encrypted)
//! archeion.lock                  held while a run uses the folder
//! archeion.key.json              wrapped master key (encrypted only)
//! archeion.archives.mac          signed list of archives (encrypted only)
//! 2026-10-29_213000_C/           complete archive
//!   manifest.jsonl               every file and folder at that time
//!   zipped/C/Users/.../a.txt.zip content, one zip per file (default), or
//!   data/C/Users/.../a.txt       plain copies
//! 2026-10-30_213000_I/           incremental: only new or changed files
//!   chunks/ab/<hash>[.zip]       new chunks of files saved as chunks
//! ```
//!
//! Large files (every file, on an encrypted destination) are saved as
//! content-defined chunks: a complete archive keeps them whole and lists
//! their chunks, an incremental stores only the chunks the destination
//! does not hold yet (see the `chunks` module and `docs/large-files.md`).
//!
//! Every change is made in a temporary folder (`*.partial`, `*.merging`)
//! and committed by a rename, so an interrupted run (Cancel, power loss,
//! unplugged drive, full disk) never leaves a half-written archive. The
//! next run completes an interrupted merge and resumes an interrupted
//! backup from its journal (`*.partial/resume.jsonl`), copying only what
//! is missing. Cancellation is done by the caller killing the worker
//! process.

mod backup;
mod browse;
mod chunker;
mod chunks;
mod container;
mod crypto;
mod extract;
mod merge;
mod progress;
mod repo;
mod restore;
mod resume;
mod space;
mod verify;

pub use backup::{Tuning, backup, backup_stopping_after, backup_tuned};
pub use browse::{
    Catalog, CatalogArchive, CatalogEntry, ExistingFormat, FileVersion, SearchHit, archive_manifest, catalog,
    change_password, check_password, destination_reachable, existing_format, file_versions, free_space,
    recovery_key, search,
};
pub use chunker::ChunkParams;
pub use crypto::Password;
pub use extract::{ExtractRequest, Secret, extract};
pub use merge::{delete_archive, merge, merge_pending, merge_range};
pub use repo::{
    DestinationTrashed, adopt_destination, destination_busy, destination_owner, list_archives,
    trash_destination,
};
pub use restore::{RestoreItem, RestoreRequest, RestoreTarget, restore};
pub use verify::verify;

use archeion_core::RunEvent;

/// Receives the events of a run. Called from the run's thread.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: RunEvent);
}

/// Failure of an engine operation, by how the caller reacts.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Drive unplugged or share offline: retry later.
    #[error("destination not available: {0}")]
    DestinationUnavailable(String),
    /// Another backup or merge of the same destination is running.
    #[error("the destination is in use by another backup or merge")]
    Busy,
    /// The profile cannot run as configured.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    /// The destination folder belongs to another profile.
    #[error("the destination is used by another profile")]
    ForeignDestination,
    /// The destination's drive has no Recycle Bin (a network share, a
    /// removable drive): nothing was moved.
    #[error("{0} has no Recycle Bin: nothing was moved")]
    NoRecycleBin(String),
    /// Windows deleted this instead of moving it to the Recycle Bin.
    #[error("{0} was deleted instead of going to the Recycle Bin")]
    NotRecycled(String),
    /// The profile is encrypted but the destination, which has backups, no
    /// longer says so: its description was removed or changed outside
    /// Archeion. Nothing is written in the clear.
    #[error(
        "the backups at the destination are not described as encrypted, but the profile or the backups are"
    )]
    EncryptionMissing,
    /// The destination is encrypted and no password was given.
    #[error("the destination is encrypted: a password is needed")]
    PasswordNeeded,
    /// The password does not open the destination.
    #[error("the password does not open the destination")]
    WrongPassword,
    /// The destination is too full: `needed` bytes (with a safety margin)
    /// against `available`. Nothing was committed; what was copied is
    /// reused by the next backup.
    #[error("not enough free space on the destination: {needed} bytes needed, {available} available")]
    NotEnoughSpace { needed: u64, available: u64 },
    /// An archive is damaged or missing content.
    #[error("archive {archive}: {message}")]
    Damaged { archive: String, message: String },
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}

impl EngineError {
    pub(crate) fn io(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> Self {
        let context = context.into();
        move |source| Self::Io { context, source }
    }
}

/// Result of an engine operation.
pub type Result<T> = std::result::Result<T, EngineError>;
