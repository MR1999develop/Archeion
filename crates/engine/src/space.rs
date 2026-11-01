//! Free space on the destination.
//!
//! Before copying, a backup checks that the destination can hold what it
//! is about to write, plus a margin; during the copy, a full disk stops the
//! run instead of skipping file after file. Either way nothing is
//! committed, and what was already copied stays in the unfinished archive
//! for the next run to reuse (see [`crate::resume`]).

use crate::{EngineError, Result};
use std::{io, path::Path};

/// Kept free on top of the estimate: manifests, journals and file system
/// overhead.
pub(crate) const MARGIN: u64 = 64 * 1024 * 1024;

/// Free bytes for this user on the volume of `dir`; `None` when unknown
/// (some network shares do not say).
pub(crate) fn available(dir: &Path) -> Option<u64> {
    match fs2::available_space(dir) {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::warn!(error = %e, dir = %dir.display(), "free space unknown");
            None
        }
    }
}

/// Fails with [`EngineError::NotEnoughSpace`] when `dir` has less than
/// `needed` plus [`MARGIN`] free. Unknown free space lets the run try.
pub(crate) fn ensure(dir: &Path, needed: u64) -> Result<()> {
    let needed = needed.saturating_add(MARGIN);
    match available(dir) {
        Some(available) if available < needed => Err(EngineError::NotEnoughSpace { needed, available }),
        _ => Ok(()),
    }
}

/// [`EngineError::NotEnoughSpace`] for a disk that filled up while `needed`
/// more bytes were still to be written.
pub(crate) fn full(dir: &Path, needed: u64) -> EngineError {
    EngineError::NotEnoughSpace {
        needed: needed.saturating_add(MARGIN),
        available: available(dir).unwrap_or(0),
    }
}

/// True when `e` comes from a full disk or an exhausted quota.
pub(crate) fn is_disk_full(e: &EngineError) -> bool {
    match e {
        EngineError::Io { source, .. } => io_full(source),
        EngineError::NotEnoughSpace { .. } => true,
        _ => false,
    }
}

fn io_full(e: &io::Error) -> bool {
    // ERROR_HANDLE_DISK_FULL (39) and ERROR_DISK_FULL (112) on Windows,
    // ENOSPC (28) and EDQUOT elsewhere: all mapped to these kinds by std.
    matches!(e.kind(), io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded)
        || (cfg!(windows) && matches!(e.raw_os_error(), Some(39 | 112)))
        || e.get_ref().and_then(|inner| inner.downcast_ref::<io::Error>()).is_some_and(io_full)
}

/// Bytes a file of `size` is expected to take once stored. Deflated files
/// count half: text and documents usually shrink more, and a file that
/// does not is caught by the full-disk check during the copy.
pub(crate) fn estimate(size: u64, deflated: bool) -> u64 {
    if deflated { size / 2 } else { size }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_disk_errors_are_recognised() {
        let full =
            EngineError::Io { context: "x".into(), source: io::Error::from(io::ErrorKind::StorageFull) };
        assert!(is_disk_full(&full));
        let wrapped = EngineError::Io {
            context: "x".into(),
            source: io::Error::other(io::Error::from(io::ErrorKind::StorageFull)),
        };
        assert!(is_disk_full(&wrapped));
        let denied =
            EngineError::Io { context: "x".into(), source: io::Error::from(io::ErrorKind::PermissionDenied) };
        assert!(!is_disk_full(&denied));
    }

    #[test]
    fn a_huge_need_is_refused_with_the_margin() {
        let dir = tempfile::tempdir().unwrap();
        match ensure(dir.path(), u64::MAX / 2) {
            Err(EngineError::NotEnoughSpace { needed, .. }) => assert_eq!(needed, u64::MAX / 2 + MARGIN),
            other => panic!("unexpected {other:?}"),
        }
        assert!(ensure(dir.path(), 0).is_ok());
    }
}
