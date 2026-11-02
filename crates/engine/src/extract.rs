//! Restore from a destination folder alone, without settings or a profile:
//! what `archeion extract` does on any computer, for example from the
//! portable executable after the original computer is lost.
//!
//! The destination is only read: no lock is taken, no interrupted merge is
//! completed, nothing is written there. Run it while no backup or merge of
//! that destination is running.

use crate::{
    EngineError, EventSink, Result,
    crypto::{Keys, Password},
    repo::{archives_in, ensure_reachable, is_encrypted, read_manifest, unlock},
    restore::{RestoreItem, RestoreRequest, RestoreTarget, restore_from},
};
use archeion_core::{Phase, RestoreReport, RunEvent, keys};
use std::path::{Path, PathBuf};

/// What opens an encrypted destination. Its `Debug` output never shows it.
#[derive(Clone)]
pub enum Secret {
    /// The destination's password.
    Password(Password),
    /// The recovery key: 64 hex digits, spaces and dashes allowed.
    RecoveryKey(String),
    /// Either one: tried as a recovery key when it looks like one, as a
    /// password otherwise or when the recovery key does not open it.
    Either(String),
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Password(_) => "Secret::Password(…)",
            Self::RecoveryKey(_) => "Secret::RecoveryKey(…)",
            Self::Either(_) => "Secret::Either(…)",
        })
    }
}

/// What `archeion extract` restores.
#[derive(Debug, Clone)]
pub struct ExtractRequest {
    /// The destination folder (the one holding `archeion.json` and the
    /// dated backup folders).
    pub destination: PathBuf,
    /// Backup name (`2026-10-29_213000_C`); `None` for the newest.
    pub archive: Option<String>,
    /// A file or folder to restore: its original path
    /// (`C:\Users\me\Documents`) or its key (`C/Users/me/Documents`);
    /// `None` for everything.
    pub path: Option<String>,
    /// Folder that receives the files, below their full path
    /// (`<to>\C\Users\me\...`).
    pub to: PathBuf,
    /// Replace files already in `to`.
    pub overwrite: bool,
    /// Needed when the destination is encrypted.
    pub secret: Option<Secret>,
}

/// Restores `request` from its destination folder alone. Reports progress
/// and the result to `sink` like [`crate::restore`].
///
/// # Errors
/// [`EngineError::DestinationUnavailable`] when the folder is missing,
/// [`EngineError::PasswordNeeded`] or [`EngineError::WrongPassword`] for an
/// encrypted destination, [`EngineError::InvalidProfile`] when the backup
/// or the path does not exist; otherwise see [`EngineError`].
pub fn extract(request: &ExtractRequest, sink: &dyn EventSink) -> Result<RestoreReport> {
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let root = request.destination.as_path();
    ensure_reachable(root)?;
    if !root.is_dir() {
        return Err(EngineError::DestinationUnavailable(root.display().to_string()));
    }
    let archives = archives_in(root)?;
    let archive = match &request.archive {
        Some(name) => name.parse().map_err(|_| invalid(format!("{name} is not a backup name")))?,
        None => *archives.last().ok_or_else(|| invalid(format!("{} holds no backup", root.display())))?,
    };
    if !archives.contains(&archive) {
        return Err(invalid(format!("backup {archive} not found in {}", root.display())));
    }
    let keys = open_keys(root, &archive, request.secret.as_ref())?;
    let key = match &request.path {
        None => None,
        Some(p) => {
            let key = selection_key(p);
            let manifest = read_manifest(&root.join(archive.to_string()), &archive, keys.as_ref())?;
            let found = manifest.entries.iter().any(|e| keys::is_within(&e.key, &key));
            if !found {
                return Err(invalid(format!("{p} is not in backup {archive}")));
            }
            Some(key)
        }
    };
    let restore = RestoreRequest {
        items: vec![RestoreItem {
            archive: archive.to_string(),
            key,
            target: RestoreTarget::Folder { path: request.to.clone() },
            overwrite: request.overwrite,
        }],
    };
    restore_from(root, keys.as_ref(), &restore, sink)
}

fn invalid(message: String) -> EngineError {
    EngineError::InvalidProfile(message)
}

/// Keys of the destination at `root`, from `secret`; `None` when it is not
/// encrypted. A recovery key is checked by opening `archive`'s manifest.
fn open_keys(
    root: &Path,
    archive: &archeion_core::ArchiveName,
    secret: Option<&Secret>,
) -> Result<Option<Keys>> {
    if !is_encrypted(root)? {
        return Ok(None);
    }
    let by_recovery_key = |text: &str| -> Result<Keys> {
        let keys = Keys::from_recovery_key(text).ok_or(EngineError::WrongPassword)?;
        read_manifest(&root.join(archive.to_string()), archive, Some(&keys))?;
        Ok(keys)
    };
    let by_password = |text: &str| -> Result<Option<Keys>> {
        let password = Password::new(text.to_owned()).ok_or(EngineError::PasswordNeeded)?;
        unlock(root, Some(&password))
    };
    match secret {
        None => Err(EngineError::PasswordNeeded),
        Some(Secret::Password(p)) => unlock(root, Some(p)),
        Some(Secret::RecoveryKey(k)) => by_recovery_key(k).map(Some),
        Some(Secret::Either(text)) => match by_recovery_key(text) {
            Ok(keys) => Ok(Some(keys)),
            Err(_) => by_password(text),
        },
    }
}

/// Key of a path given on the command line: an absolute path is turned
/// into its key; anything else is taken as a key, with either slash.
fn selection_key(text: &str) -> String {
    let trimmed = text.trim().trim_end_matches(['\\', '/']);
    match keys::key_for(Path::new(trimmed)) {
        Ok(key) => key,
        Err(_) => trimmed.replace('\\', "/").trim_start_matches('/').to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_the_secret() {
        let text = format!("{:?}", Secret::Either("hunter2".into()));
        assert!(!text.contains("hunter2"));
    }

    #[test]
    fn a_key_or_a_path_selects_the_same_files() {
        assert_eq!(selection_key("C/Users/me/Documents/"), "C/Users/me/Documents");
        assert_eq!(selection_key(r"C\Users\me"), "C/Users/me");
        #[cfg(windows)]
        assert_eq!(selection_key(r"C:\Users\me\Documents\"), "C/Users/me/Documents");
        #[cfg(unix)]
        assert_eq!(selection_key("/home/me/a.txt"), "ROOT/home/me/a.txt");
    }
}
