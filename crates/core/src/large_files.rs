//! Which files are saved as chunks, so that a changed file stores only its
//! changed parts (see `docs/large-files.md`).
//!
//! - On an encrypted destination every file is chunked.
//! - On other destinations, files of at least [`LargeFiles::threshold_mb`]
//!   are chunked; smaller ones stay whole zips or plain copies that open
//!   with any tool.
//! - A complete archive still holds every file whole, so it can be
//!   recovered without Archeion; for chunked files it also lists their
//!   chunks, which the incrementals after it reuse ([`FileStorage`]).
//! - Chunked files start with a complete archive written by a version that
//!   knows chunks: on a destination whose newest chain is older, the next
//!   automatic backup is complete ([`needs_new_chain`]).

use crate::manifest::CHUNKS_FORMAT;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Bytes in the unit of [`LargeFiles::threshold_mb`] (shown as "MB").
pub const MB: u64 = 1024 * 1024;

/// The profile's choice for large files on unencrypted destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LargeFiles {
    /// Files of at least this many MB are saved as chunks; 0 = never.
    pub threshold_mb: u32,
}

impl Default for LargeFiles {
    fn default() -> Self {
        Self { threshold_mb: 500 }
    }
}

impl LargeFiles {
    /// Range of [`Self::threshold_mb`] in the editor when it is on: 1 MB
    /// to 1 TB.
    pub const THRESHOLD_RANGE_MB: RangeInclusive<u32> = 1..=1024 * 1024;

    /// Never chunk files on unencrypted destinations.
    pub const NEVER: Self = Self { threshold_mb: 0 };

    /// Threshold in bytes; `None` for "never".
    #[must_use]
    pub fn threshold_bytes(&self) -> Option<u64> {
        (self.threshold_mb > 0).then(|| u64::from(self.threshold_mb).saturating_mul(MB))
    }
}

/// How one file's content goes into a new archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStorage {
    /// A whole copy, as always (zip, plain copy or encrypted zip).
    Whole,
    /// A whole copy and the list of its chunks (complete archives), so the
    /// following incrementals store only chunks that changed.
    WholeAndChunkList,
    /// Only the chunks not already in the destination, and the list.
    ChunksOnly,
}

/// What decides [`FileStorage`] for the files of one new archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkPolicy {
    /// The destination is encrypted: every file is chunked.
    pub encrypted: bool,
    pub large_files: LargeFiles,
    /// The new archive is complete.
    pub complete: bool,
    /// The chain the new archive continues began with a complete archive
    /// that lists chunks (format [`CHUNKS_FORMAT`] or later). Not used for
    /// a complete archive, which begins its own chain.
    pub chain_has_chunks: bool,
}

impl ChunkPolicy {
    /// True when some file may be chunked at this destination.
    #[must_use]
    pub fn enabled(&self) -> bool {
        chunking_enabled(self.encrypted, self.large_files)
    }

    /// How a file of `size` bytes is stored.
    #[must_use]
    pub fn storage(&self, size: u64) -> FileStorage {
        let chunked = self.encrypted || self.large_files.threshold_bytes().is_some_and(|t| size >= t);
        if !chunked {
            FileStorage::Whole
        } else if self.complete {
            FileStorage::WholeAndChunkList
        } else if self.chain_has_chunks {
            FileStorage::ChunksOnly
        } else {
            // An incremental of an older chain the user asked for: whole,
            // like the rest of that chain.
            FileStorage::Whole
        }
    }
}

/// True when some file may be chunked: always on an encrypted destination,
/// otherwise unless the profile says "never".
#[must_use]
pub fn chunking_enabled(encrypted: bool, large_files: LargeFiles) -> bool {
    encrypted || large_files.threshold_bytes().is_some()
}

/// True when the next automatic backup must start a new chain so that
/// files can be chunked: chunking is on and the newest complete archive
/// (of format `newest_complete_format`) was written before chunks existed.
#[must_use]
pub fn needs_new_chain(chunking: bool, newest_complete_format: Option<u32>) -> bool {
    chunking && newest_complete_format.is_some_and(|f| f < CHUNKS_FORMAT)
}

/// True when the chain whose complete archive has format
/// `newest_complete_format` can take chunked files.
#[must_use]
pub fn chain_has_chunks(newest_complete_format: Option<u32>) -> bool {
    newest_complete_format.is_some_and(|f| f >= CHUNKS_FORMAT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(encrypted: bool, mb: u32, complete: bool, chain: bool) -> ChunkPolicy {
        ChunkPolicy {
            encrypted,
            large_files: LargeFiles { threshold_mb: mb },
            complete,
            chain_has_chunks: chain,
        }
    }

    #[test]
    fn unencrypted_files_are_chunked_from_the_threshold() {
        let p = policy(false, 1, false, true);
        assert_eq!(p.storage(MB - 1), FileStorage::Whole);
        assert_eq!(p.storage(MB), FileStorage::ChunksOnly);
        assert_eq!(p.storage(MB + 1), FileStorage::ChunksOnly);
        let never = policy(false, 0, false, true);
        assert_eq!(never.storage(u64::MAX), FileStorage::Whole);
        assert!(!never.enabled());
    }

    #[test]
    fn encrypted_destinations_chunk_every_file() {
        let p = policy(true, 0, false, true);
        assert!(p.enabled());
        assert_eq!(p.storage(0), FileStorage::ChunksOnly);
        assert_eq!(p.storage(1), FileStorage::ChunksOnly);
    }

    #[test]
    fn complete_archives_keep_whole_copies_with_the_list() {
        for encrypted in [false, true] {
            let p = policy(encrypted, 1, true, false);
            assert_eq!(p.storage(2 * MB), FileStorage::WholeAndChunkList);
        }
        assert_eq!(policy(false, 1, true, true).storage(10), FileStorage::Whole);
    }

    #[test]
    fn an_older_chain_keeps_whole_copies() {
        assert_eq!(policy(true, 0, false, false).storage(10), FileStorage::Whole);
        assert_eq!(policy(false, 1, false, false).storage(2 * MB), FileStorage::Whole);
    }

    #[test]
    fn an_old_chain_needs_a_new_complete_only_when_chunking_is_on() {
        assert!(needs_new_chain(true, Some(1)));
        assert!(!needs_new_chain(true, Some(CHUNKS_FORMAT)));
        assert!(!needs_new_chain(false, Some(1)));
        assert!(!needs_new_chain(true, None));
        assert!(chain_has_chunks(Some(CHUNKS_FORMAT)));
        assert!(!chain_has_chunks(Some(1)));
        assert!(!chain_has_chunks(None));
    }

    #[test]
    fn the_threshold_is_in_mebibytes_and_saturates() {
        assert_eq!(LargeFiles::default().threshold_bytes(), Some(500 * MB));
        assert_eq!(LargeFiles::NEVER.threshold_bytes(), None);
        let max = LargeFiles { threshold_mb: u32::MAX };
        assert_eq!(max.threshold_bytes(), Some(u64::from(u32::MAX) * MB));
        assert_eq!(*LargeFiles::THRESHOLD_RANGE_MB.end(), 1_048_576);
    }

    #[test]
    fn older_profiles_get_the_default_threshold() {
        let lf: LargeFiles = serde_json::from_str("{}").unwrap();
        assert_eq!(lf, LargeFiles::default());
    }
}
