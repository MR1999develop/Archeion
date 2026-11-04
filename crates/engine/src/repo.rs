//! The destination folder of a profile: ownership, lock, archive list,
//! manifests and crash recovery.

use crate::{
    EngineError, Result,
    crypto::{KEY_FILE, KEY_FORMAT, Keys, LIST_MAC_FILE, Password, decrypt_single, encrypt_single},
};
use archeion_core::{
    ArchiveInfo, ArchiveName, Destination, EntryKind, Manifest, Profile, ProfileId, profile_text,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, TryLockError},
    io::Write as _,
    path::{Path, PathBuf},
};

const DESCRIPTOR: &str = "archeion.json";
const LOCK: &str = "archeion.lock";
const JOURNAL: &str = "merge.journal";
/// The copy of the profile at an unencrypted destination: its text, as in
/// a profile file.
const PROFILE_COPY: &str = "archeion.profile.toml";
/// The copy of the profile at an encrypted destination: sealed like the
/// manifests, so it opens with the recovery key in 7-Zip too.
const PROFILE_COPY_SEALED: &str = "archeion.profile.zip";
/// Name of the entry inside [`PROFILE_COPY_SEALED`].
const PROFILE_ENTRY: &str = "profile.toml";
pub(crate) const MANIFEST: &str = "manifest.jsonl";
/// Encrypted manifest of an archive of an encrypted destination.
pub(crate) const MANIFEST_ZIP: &str = "manifest.zip";
pub(crate) const DATA_DIR: &str = "data";
pub(crate) const ZIPPED_DIR: &str = "zipped";
/// Content of an encrypted archive (see [`crate::crypto`]).
pub(crate) const VAULT_DIR: &str = "vault";

const PARTIAL: &str = "partial";
const MERGING: &str = "merging";
const OLD: &str = "old";
const DELETING: &str = "deleting";
/// Suffix of an archive folder found damaged during a recovery and set
/// aside: never removed by Archeion, it may hold the only copy of a file.
const DAMAGED: &str = "damaged";

#[derive(Serialize, Deserialize)]
struct Descriptor {
    format: u32,
    profile: ProfileId,
    /// Present when the destination is encrypted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encryption: Option<EncryptionInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
struct EncryptionInfo {
    /// Format of the key file; the first test version had none.
    #[serde(default)]
    version: u32,
}

impl EncryptionInfo {
    /// Unlocks with `password` (see [`crate::crypto`]).
    fn unlock(&self, root: &Path, password: Option<&Password>) -> Result<Keys> {
        if self.version != KEY_FORMAT {
            return Err(EngineError::Damaged {
                archive: DESCRIPTOR.to_owned(),
                message: "encrypted by a test version of Archeion that is no longer supported: \
                          use a new, empty destination"
                    .into(),
            });
        }
        Keys::unlock(root, password.ok_or(EngineError::PasswordNeeded)?)
    }
}

/// The swap of a merge, written before any archive is moved.
///
/// Read from the destination, which others may write to: its names are
/// archive names or the journal does not read, so recovering a merge can
/// only ever rename and remove archive folders of the destination.
#[derive(Serialize, Deserialize)]
struct Journal {
    /// [`JOURNAL_FORMAT`]; journals of earlier versions have none.
    #[serde(default)]
    format: u32,
    target: ArchiveName,
    members: Vec<ArchiveName>,
}

/// Format of [`Journal`]: a newer one is left alone.
const JOURNAL_FORMAT: u32 = 1;

/// An opened destination, locked for exclusive use until dropped.
#[derive(Debug)]
pub(crate) struct Repo {
    root: PathBuf,
    _lock: File,
    /// Set for an encrypted destination.
    keys: Option<Keys>,
    /// Whether the list of backups matched its signature once opened
    /// (encrypted destinations).
    list_signed: Option<bool>,
}

/// The profile that owns the destination of `profile`, as its descriptor
/// says; `None` for a folder Archeion has not used yet.
///
/// # Errors
/// [`EngineError::DestinationUnavailable`], or [`EngineError::Damaged`] for
/// an unreadable descriptor.
pub fn destination_owner(profile: &Profile) -> Result<Option<ProfileId>> {
    let root = crate::repo::destination_of(profile)?;
    ensure_reachable(root)?;
    Ok(read_descriptor(root)?.map(|d| d.profile))
}

/// The profile whose backups are in the folder `root`, read from the copy
/// its last backup kept there: for importing it on this or another
/// computer once its settings are lost. Its id is the one that owns the
/// backups and its destination is `root`, whatever the copy says, so the
/// imported profile goes on with these backups; whether it is encrypted is
/// what the destination says. Only reads: no lock, nothing recovered or
/// written.
///
/// An encrypted destination is read from its sealed copy only: a plain
/// one put there is not looked at.
///
/// # Errors
/// [`EngineError::NotADestination`] for a folder without Archeion's
/// backups, [`EngineError::NoProfileCopy`] for backups made before the
/// copy was kept, [`EngineError::PasswordNeeded`] or
/// [`EngineError::WrongPassword`] for an encrypted destination,
/// [`EngineError::Damaged`] for a copy that does not read.
pub fn read_profile_copy(root: &Path, password: Option<&Password>) -> Result<Profile> {
    ensure_reachable(root)?;
    let descriptor = read_descriptor(root)?.ok_or(EngineError::NotADestination)?;
    let name = if descriptor.encryption.is_some() { PROFILE_COPY_SEALED } else { PROFILE_COPY };
    let path = root.join(name);
    let damaged = |message: String| EngineError::Damaged { archive: name.to_owned(), message };
    let bytes = match read_small_bytes_io(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(EngineError::NoProfileCopy),
        Err(e) => return Err(EngineError::io(format!("reading {}", path.display()))(e)),
    };
    let bytes = match &descriptor.encryption {
        None => bytes,
        Some(info) => {
            let keys = info.unlock(root, password)?;
            // The password opened the key: a copy that does not open with it
            // was not sealed at this destination.
            decrypt_single(&bytes, keys.password(), MAX_SMALL_BYTES).map_err(|e| match e {
                EngineError::WrongPassword => damaged("not sealed with this destination's key".to_owned()),
                other => damaged(other.to_string()),
            })?
        }
    };
    let text = String::from_utf8(bytes).map_err(|e| damaged(e.to_string()))?;
    let mut profile = profile_text::decode(&text).map_err(|e| damaged(e.to_string()))?;
    profile.id = descriptor.profile;
    profile.destination = Destination::for_folder(root);
    profile.encrypted = descriptor.encryption.is_some();
    Ok(profile)
}

/// Gives the backups of another profile to `profile`: for a profile
/// re-created after the old one was deleted. The archives, the format and
/// the encryption stay as they are; only the owner changes.
///
/// # Errors
/// [`EngineError::Busy`] while a run uses the destination; otherwise see
/// [`EngineError`].
pub fn adopt_destination(profile: &Profile) -> Result<()> {
    let root = crate::repo::destination_of(profile)?;
    ensure_reachable(root)?;
    let Some(mut descriptor) = read_descriptor(root)? else { return Ok(()) };
    let lock_path = root.join(LOCK);
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(EngineError::io(format!("opening {}", lock_path.display())))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Err(EngineError::Busy),
        Err(TryLockError::Error(e)) => return Err(EngineError::io("locking the destination")(e)),
    }
    tracing::info!(from = %descriptor.profile, to = %profile.id, "destination taken over");
    descriptor.profile = profile.id;
    let tmp = root.join(format!("{DESCRIPTOR}.tmp"));
    write_durably(
        &tmp,
        serde_json::to_string_pretty(&descriptor).expect("descriptor serializes").as_bytes(),
    )?;
    let path = root.join(DESCRIPTOR);
    retry(|| archeion_win::rename_durably(&tmp, &path))
        .map_err(EngineError::io(format!("replacing {}", path.display())))
}

/// What [`trash_destination`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DestinationTrashed {
    /// The whole folder went to the Recycle Bin: it held nothing else.
    pub folder_trashed: bool,
    /// Names in the folder that Archeion did not write, left in place.
    pub left: Vec<String>,
}

/// Moves the backups of `profile` to the Windows Recycle Bin, from where
/// they can be restored until it is emptied: the whole destination folder
/// when it holds nothing else, otherwise each thing Archeion wrote in it
/// (archives, chunks, key, descriptor, unfinished backups); files Archeion
/// did not write are left, and named.
///
/// Windows deletes outright what it cannot recycle (a network share, a
/// removable drive), and says it recycled it: so a small probe file goes
/// first and must be found in the Recycle Bin, and so must everything
/// moved after it; at the first that is not, it stops.
///
/// The folder must hold this profile's backups: a folder of another
/// profile is refused, one without backups is left alone.
///
/// # Errors
/// [`EngineError::NoRecycleBin`] before anything is moved,
/// [`EngineError::NotRecycled`] when Windows deleted something instead of
/// recycling it, [`EngineError::ForeignDestination`],
/// [`EngineError::Busy`] or [`EngineError::DestinationUnavailable`];
/// otherwise see [`EngineError`].
pub fn trash_destination(profile: &Profile) -> Result<DestinationTrashed> {
    let root = crate::repo::destination_of(profile)?;
    if root.as_os_str().is_empty() {
        return Ok(DestinationTrashed::default());
    }
    ensure_reachable(root)?;
    if !root.exists() {
        return Ok(DestinationTrashed { folder_trashed: true, left: Vec::new() });
    }
    match read_descriptor(root)? {
        Some(d) if d.profile == profile.id => {}
        Some(_) => return Err(EngineError::ForeignDestination),
        None => {
            // No backups here: nothing of Archeion's to move.
            return Ok(DestinationTrashed { folder_trashed: false, left: names_in(root)? });
        }
    }
    let lock_path = root.join(LOCK);
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(EngineError::io(format!("opening {}", lock_path.display())))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Err(EngineError::Busy),
        Err(TryLockError::Error(e)) => return Err(EngineError::io("locking the destination")(e)),
    }
    probe_recycle_bin(root)?;
    tracing::info!(profile = %profile.id, root = %root.display(), "moving the backups to the Recycle Bin");
    let (ours, left): (Vec<String>, Vec<String>) =
        names_in(root)?.into_iter().partition(|n| written_by_archeion(n));
    drop(lock);
    if left.is_empty() {
        recycle(root)?;
        return Ok(DestinationTrashed { folder_trashed: true, left });
    }
    // The descriptor last, so what is still there stays a destination.
    let (last, first): (Vec<String>, Vec<String>) = ours.into_iter().partition(|n| n == DESCRIPTOR);
    for name in first.iter().chain(&last) {
        recycle(&root.join(name))?;
    }
    Ok(DestinationTrashed { folder_trashed: false, left })
}

/// Moves `path` to the Recycle Bin and checks that it is there.
fn recycle(path: &Path) -> Result<()> {
    let since = std::time::SystemTime::now();
    trash::delete(path).map_err(|e| {
        EngineError::io(format!("moving {} to the Recycle Bin", path.display()))(std::io::Error::other(
            e.to_string(),
        ))
    })?;
    if in_recycle_bin(path, since)? {
        Ok(())
    } else {
        Err(EngineError::NotRecycled(path.display().to_string()))
    }
}

/// Sends a probe file of `root` to the Recycle Bin and back out of it:
/// a drive without one deletes it instead.
fn probe_recycle_bin(root: &Path) -> Result<()> {
    let stamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let probe = root.join(format!("archeion.recycle-probe-{}-{stamp}", std::process::id()));
    fs::write(&probe, b"Archeion checks that this drive has a Recycle Bin.")
        .map_err(EngineError::io(format!("writing {}", probe.display())))?;
    match recycle(&probe) {
        Ok(()) => {
            purge_from_recycle_bin(&probe);
            Ok(())
        }
        Err(EngineError::NotRecycled(_)) => Err(EngineError::NoRecycleBin(root.display().to_string())),
        Err(e) => {
            let _ = fs::remove_file(&probe);
            Err(e)
        }
    }
}

/// The item of the Recycle Bin that was `path`, put there since `since`.
fn recycled_item(path: &Path, since: std::time::SystemTime) -> Result<Option<trash::TrashItem>> {
    let (Some(name), Some(parent)) = (path.file_name(), path.parent()) else { return Ok(None) };
    let since = since
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        // Deletion times are whole seconds.
        .saturating_sub(2);
    let same = |a: &Path, b: &Path| a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy());
    let items = trash::os_limited::list()
        .map_err(|e| EngineError::io("reading the Recycle Bin")(std::io::Error::other(e.to_string())))?;
    Ok(items
        .into_iter()
        .find(|i| i.name == name && same(&i.original_parent, parent) && i.time_deleted >= since))
}

fn in_recycle_bin(path: &Path, since: std::time::SystemTime) -> Result<bool> {
    // Windows may list an item a moment after moving it.
    for wait_ms in [0, 100, 300, 600] {
        std::thread::sleep(std::time::Duration::from_millis(wait_ms));
        if recycled_item(path, since)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Removes the probe from the Recycle Bin: it is not the user's.
fn purge_from_recycle_bin(probe: &Path) {
    let since = std::time::SystemTime::UNIX_EPOCH;
    if let Ok(Some(item)) = recycled_item(probe, since)
        && let Err(e) = trash::os_limited::purge_all([item])
    {
        tracing::warn!(error = %e, "removing the probe from the Recycle Bin");
    }
}

/// Names Archeion writes at the root of a destination.
fn written_by_archeion(name: &str) -> bool {
    let archive_folder = |n: &str| n.parse::<ArchiveName>().is_ok();
    let set_aside = name.rsplit_once('.').is_some_and(|(stem, suffix)| {
        [PARTIAL, MERGING, OLD, DELETING, DAMAGED].contains(&suffix) && archive_folder(stem)
    });
    name.starts_with("archeion.")
        || [JOURNAL, PARTIAL, MERGING, OLD, DELETING, crate::chunks::CHUNKS_DIR].contains(&name)
        || archive_folder(name)
        || set_aside
}

/// The names in `dir`, sorted.
fn names_in(dir: &Path) -> Result<Vec<String>> {
    let read = fs::read_dir(dir).map_err(EngineError::io(format!("reading {}", dir.display())))?;
    let mut names: Vec<String> =
        read.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort_unstable();
    Ok(names)
}

/// True while a backup, merge or other run holds the destination of
/// `profile` (its lock file is locked). A snapshot: the run may start or
/// end right after; runs still check the lock themselves.
#[must_use]
pub fn destination_busy(profile: &Profile) -> bool {
    let Ok(root) = destination_of(profile) else { return false };
    let path = root.join(LOCK);
    let Ok(file) = File::options().write(true).open(&path) else { return false };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(TryLockError::WouldBlock) => true,
        Err(TryLockError::Error(_)) => false,
    }
}

/// Fails when the drive or share holding `root` is not reachable.
/// The destination folder of `profile`, refused while its network folders
/// wait for the user's confirmation: the one way the engine reaches a
/// profile's destination.
pub(crate) fn destination_of(profile: &Profile) -> Result<&Path> {
    if profile.network_unconfirmed {
        return Err(EngineError::NetworkUnconfirmed);
    }
    Ok(profile.destination.path())
}

pub(crate) fn ensure_reachable(root: &Path) -> Result<()> {
    let volume = root.ancestors().last().unwrap_or(root);
    if root.exists() || volume.as_os_str().is_empty() || volume.exists() {
        Ok(())
    } else {
        Err(EngineError::DestinationUnavailable(root.display().to_string()))
    }
}

impl Repo {
    /// Opens the destination of `profile`, creating it when `create` is set.
    /// Returns `None` when the folder does not exist and `create` is false.
    ///
    /// A new destination (or one without archives, when `create` is set) is
    /// encrypted as the profile says; an existing one keeps what it is.
    /// `password` is needed for an encrypted destination.
    pub(crate) fn open(profile: &Profile, create: bool, password: Option<&Password>) -> Result<Option<Self>> {
        let root = crate::repo::destination_of(profile)?.to_path_buf();
        ensure_reachable(&root)?;
        if !root.exists() {
            if !create {
                return Ok(None);
            }
            if profile.encrypted && password.is_none() {
                return Err(EngineError::PasswordNeeded);
            }
            fs::create_dir_all(&root).map_err(EngineError::io(format!("creating {}", root.display())))?;
        }
        let descriptor = read_descriptor(&root)?;
        if descriptor.as_ref().is_some_and(|d| d.profile != profile.id) {
            return Err(EngineError::ForeignDestination);
        }

        let lock_path = root.join(LOCK);
        let lock = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(EngineError::io(format!("opening {}", lock_path.display())))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(EngineError::Busy),
            Err(TryLockError::Error(e)) => return Err(EngineError::io("locking the destination")(e)),
        }
        let mut repo = Self { root, _lock: lock, keys: None, list_signed: None };
        let encryption = descriptor.as_ref().and_then(|d| d.encryption.clone());
        // The keys before anything is recovered: whether an interrupted
        // merge finished shows in its sealed manifest, and a plain manifest
        // put in an encrypted destination must not count. Without them (no
        // password: a destination about to be made plain again) nothing is
        // recovered.
        if let (Some(info), Some(_)) = (&encryption, password) {
            repo.keys = info.unlock(&repo.root, password).ok();
        }
        let recovered = if encryption.is_some() && repo.keys.is_none() { false } else { repo.recover()? };

        let empty = archives_in(&repo.root)?.is_empty();
        // Backups of an encrypted profile are never written in the clear: a
        // destination with backups that no longer says it is encrypted was
        // changed from outside.
        let described_encrypted = descriptor.as_ref().is_some_and(|d| d.encryption.is_some());
        if !described_encrypted && !empty && (profile.encrypted || holds_sealed_archives(&repo.root)?) {
            return Err(EngineError::EncryptionMissing);
        }
        let rewrite = match &descriptor {
            None => create,
            Some(d) => create && empty && d.encryption.is_some() != profile.encrypted,
        };
        if rewrite {
            // An unfinished backup made with the old setting cannot be resumed.
            repo.remove_partials(None)?;
            repo.keys = repo.write_descriptor(profile, profile.encrypted && empty, password)?;
        } else if let Some(info) = encryption {
            if repo.keys.is_none() {
                repo.keys = Some(info.unlock(&repo.root, password)?);
            }
            repo.list_signed = repo.list_matches()?;
            if recovered && repo.list_signed == Some(true) {
                // A merge or a deletion completed or undone, as signed before
                // it began: the signature becomes one line again. A list that
                // matches no signed line keeps the old signature, so the
                // check still sees it.
                repo.refresh_list_mac()?;
            }
            if repo.list_signed == Some(false) {
                tracing::warn!("the list of backups does not match its signature");
            }
        }
        Ok(Some(repo))
    }

    /// True when, once opened, the destination held another list of backups
    /// than the one Archeion signed: backups removed or put back from
    /// outside. The run goes on (and signs the new list); the caller tells
    /// the user.
    pub(crate) fn changed_outside(&self) -> bool {
        self.list_signed == Some(false)
    }

    /// Signs the current list of archives (encrypted destinations), so a
    /// backup removed or put back from outside is noticed by the check.
    fn refresh_list_mac(&self) -> Result<()> {
        let Some(keys) = &self.keys else { return Ok(()) };
        let mac = keys.list_mac(&self.archives()?);
        self.write_list_mac(&mac)
    }

    /// Signs both the current list and `future`, the list once the commit
    /// about to happen is done: an interruption in between leaves a
    /// signature that still matches.
    fn expect_list(&self, future: &mut Vec<ArchiveName>) -> Result<()> {
        let Some(keys) = &self.keys else { return Ok(()) };
        future.sort_unstable();
        future.dedup();
        let text = format!("{}\n{}", keys.list_mac(&self.archives()?), keys.list_mac(future));
        self.write_list_mac(&text)
    }

    /// Replaces the signature file whole: a temporary file, then a rename.
    fn write_list_mac(&self, text: &str) -> Result<()> {
        let path = self.root.join(LIST_MAC_FILE);
        let tmp = self.root.join(format!("{LIST_MAC_FILE}.tmp"));
        write_durably(&tmp, text.as_bytes())?;
        retry(|| archeion_win::rename_durably(&tmp, &path))
            .map_err(EngineError::io(format!("replacing {}", path.display())))
    }

    /// `Some(false)` when the signed list of archives does not match the
    /// archives found; `None` for a destination that is not encrypted.
    pub(crate) fn list_matches(&self) -> Result<Option<bool>> {
        let Some(keys) = &self.keys else { return Ok(None) };
        let archives = self.archives()?;
        let saved = read_small_io(&self.root.join(LIST_MAC_FILE)).unwrap_or_default();
        let signed = saved.lines().any(|l| keys.list_signed_as(&archives, l));
        Ok(Some(archives.is_empty() && saved.trim().is_empty() || signed))
    }

    /// Re-wraps the master key with a new password.
    pub(crate) fn change_password(&self, new: &Password) -> Result<()> {
        match &self.keys {
            Some(keys) => keys.write_key_file(&self.root, new),
            None => Err(EngineError::InvalidProfile("the destination is not encrypted".into())),
        }
    }

    /// Keeps a copy of `profile`'s rules at the destination, replacing the
    /// previous one, for [`read_profile_copy`]. At an encrypted destination
    /// it is sealed with its keys and reveals none of the folders; a copy
    /// of the other kind (left from before the destination was emptied and
    /// encrypted, or put there by someone else) is removed. Written whole:
    /// a temporary file, flushed, then renamed.
    pub(crate) fn write_profile_copy(&self, profile: &Profile) -> Result<()> {
        let text = profile_text::encode(profile).map_err(|e| EngineError::InvalidProfile(e.to_string()))?;
        let (name, other, bytes) = match &self.keys {
            None => (PROFILE_COPY, PROFILE_COPY_SEALED, text.into_bytes()),
            Some(keys) => (
                PROFILE_COPY_SEALED,
                PROFILE_COPY,
                encrypt_single(text.as_bytes(), PROFILE_ENTRY, keys.password())?,
            ),
        };
        let path = self.root.join(name);
        let tmp = self.root.join(format!("{name}.tmp"));
        write_durably(&tmp, &bytes)?;
        retry(|| archeion_win::rename_durably(&tmp, &path))
            .map_err(EngineError::io(format!("replacing {}", path.display())))?;
        let stale = self.root.join(other);
        match fs::remove_file(&stale) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(EngineError::io(format!("removing {}", stale.display()))(e)),
        }
    }

    /// Writes the descriptor of the destination, setting up encryption when
    /// `encrypt` is set (only ever for a destination without archives).
    /// Returns the keys of a new encrypted destination.
    fn write_descriptor(
        &self,
        profile: &Profile,
        encrypt: bool,
        password: Option<&Password>,
    ) -> Result<Option<Keys>> {
        let (encryption, keys) = if encrypt {
            let password = password.ok_or(EngineError::PasswordNeeded)?;
            let keys = Keys::create(&self.root, password)?;
            (Some(EncryptionInfo { version: KEY_FORMAT }), Some(keys))
        } else {
            for name in [KEY_FILE, LIST_MAC_FILE] {
                let path = self.root.join(name);
                if path.exists() {
                    fs::remove_file(&path)
                        .map_err(EngineError::io(format!("removing {}", path.display())))?;
                }
            }
            (None, None)
        };
        let d =
            Descriptor { format: archeion_core::manifest::FORMAT_VERSION, profile: profile.id, encryption };
        write_durably(
            &self.root.join(DESCRIPTOR),
            serde_json::to_string_pretty(&d).expect("descriptor serializes").as_bytes(),
        )?;
        Ok(keys)
    }

    /// Keys of an encrypted destination.
    pub(crate) fn keys(&self) -> Option<&Keys> {
        self.keys.as_ref()
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Committed archives, oldest first.
    pub(crate) fn archives(&self) -> Result<Vec<ArchiveName>> {
        archives_in(&self.root)
    }

    pub(crate) fn dir(&self, name: &ArchiveName) -> PathBuf {
        self.root.join(name.to_string())
    }

    pub(crate) fn manifest(&self, name: &ArchiveName) -> Result<Manifest> {
        read_manifest(&self.dir(name), name, self.keys())
    }

    /// Writes the manifest of an archive being built.
    pub(crate) fn write_manifest(&self, dir: &Path, m: &Manifest) -> Result<()> {
        write_manifest(dir, m, self.keys())
    }

    /// Deletes a committed archive that nothing depends on. It is renamed
    /// first, so an interruption leaves a `.deleting` folder that the next
    /// run removes, never a half-deleted archive.
    pub(crate) fn remove_archive(&self, name: &ArchiveName) -> Result<()> {
        let dir = self.dir(name);
        let doomed = self.root.join(format!("{name}.{DELETING}"));
        let mut future: Vec<ArchiveName> = self.archives()?.into_iter().filter(|a| a != name).collect();
        self.expect_list(&mut future)?;
        retry(|| archeion_win::rename_durably(&dir, &doomed))
            .map_err(EngineError::io(format!("deleting {}", dir.display())))?;
        self.refresh_list_mac()?;
        // The archive is gone already; a folder still in use is removed
        // by the next run.
        remove_dir_best_effort(&doomed);
        Ok(())
    }

    /// Unfinished backups (`<archive>.partial`), oldest first.
    fn partials(&self) -> Result<Vec<ArchiveName>> {
        let suffix = format!(".{PARTIAL}");
        let mut list: Vec<ArchiveName> = fs::read_dir(&self.root)
            .map_err(EngineError::io("listing the destination"))?
            .flatten()
            .filter(is_real_dir)
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_owned();
                name.strip_suffix(&suffix)?.parse().ok()
            })
            .collect();
        list.sort_unstable();
        Ok(list)
    }

    /// Removes every unfinished backup except `keep`.
    fn remove_partials(&self, keep: Option<ArchiveName>) -> Result<()> {
        for p in self.partials()?.into_iter().filter(|p| Some(*p) != keep) {
            self.discard_partial(&p)?;
        }
        Ok(())
    }

    /// The unfinished backup that the next backup can resume: the newest
    /// one started after the newest archive. The others are removed (an
    /// older one would sort before archives made since). One holding a
    /// link (symbolic link, junction) is not resumed: Archeion never makes
    /// one, and writing through it could reach any folder of the computer.
    pub(crate) fn take_partial(&self, archives: &[ArchiveName]) -> Result<Option<ArchiveName>> {
        let latest = archives.last().map(|a| a.time);
        let mut keep = self.partials()?.into_iter().rev().find(|p| latest.is_none_or(|t| p.time > t));
        if let Some(p) = keep
            && holds_links(&self.partial_dir(&p))
        {
            tracing::warn!(archive = %p, "an unfinished backup holds links: not resumed");
            keep = None;
        }
        self.remove_partials(keep)?;
        Ok(keep)
    }

    /// Folder of the unfinished backup `name`.
    pub(crate) fn partial_dir(&self, name: &ArchiveName) -> PathBuf {
        self.root.join(format!("{name}.{PARTIAL}"))
    }

    /// Deletes the unfinished backup `name`.
    pub(crate) fn discard_partial(&self, name: &ArchiveName) -> Result<()> {
        let dir = self.partial_dir(name);
        tracing::info!(folder = %dir.display(), "discarding an unfinished backup");
        retry(|| fs::remove_dir_all(&dir)).map_err(EngineError::io(format!("removing {}", dir.display())))
    }

    /// Empty staging folder for a new archive.
    pub(crate) fn staging(&self, name: &ArchiveName, purpose: Purpose) -> Result<PathBuf> {
        let suffix = match purpose {
            Purpose::Backup => PARTIAL,
            Purpose::Merge => MERGING,
        };
        let dir = self.root.join(format!("{name}.{suffix}"));
        if dir.exists() {
            retry(|| fs::remove_dir_all(&dir))
                .map_err(EngineError::io(format!("removing {}", dir.display())))?;
        }
        fs::create_dir(&dir).map_err(EngineError::io(format!("creating {}", dir.display())))?;
        Ok(dir)
    }

    /// Makes a finished backup archive visible.
    pub(crate) fn commit(&self, staging: &Path, name: &ArchiveName) -> Result<()> {
        let dir = self.dir(name);
        let mut future = self.archives()?;
        future.push(*name);
        self.expect_list(&mut future)?;
        retry(|| archeion_win::rename_durably(staging, &dir))
            .map_err(EngineError::io(format!("committing {}", dir.display())))?;
        self.refresh_list_mac()
    }

    /// Replaces `members` by the merged archive in `staging`, whose files
    /// and manifest are on the disk already.
    ///
    /// A journal is written first (whole, under a temporary name, then
    /// renamed); [`Self::recover`] completes the swap if the process stops
    /// in the middle.
    pub(crate) fn commit_merge(
        &self,
        staging: &Path,
        target: &ArchiveName,
        members: &[ArchiveName],
    ) -> Result<()> {
        let journal = Journal { format: JOURNAL_FORMAT, target: *target, members: members.to_vec() };
        let mut future: Vec<ArchiveName> =
            self.archives()?.into_iter().filter(|a| !members.contains(a)).collect();
        future.push(*target);
        self.expect_list(&mut future)?;
        let jpath = self.root.join(JOURNAL);
        let tmp = self.root.join(format!("{JOURNAL}.tmp"));
        write_durably(&tmp, serde_json::to_string(&journal).expect("journal serializes").as_bytes())?;
        retry(|| archeion_win::rename_durably(&tmp, &jpath))
            .map_err(EngineError::io("writing the merge journal"))?;
        self.finish_merge(staging, &journal)?;
        self.refresh_list_mac()
    }

    /// Swaps the members for the merged archive, then removes them. Each
    /// rename is retried while Windows reports a file in use. The members
    /// are removed only once the merged archive is in place with a
    /// readable manifest; when it is not (its folder was lost), they are
    /// put back and the merge counts as not done. When an old member
    /// cannot be removed yet, the journal stays and the next run removes
    /// what is left.
    fn finish_merge(&self, staging: &Path, journal: &Journal) -> Result<()> {
        for m in &journal.members {
            let dir = self.dir(m);
            let old = self.root.join(format!("{m}.{OLD}"));
            // With `name.old` there, `name` is the merged archive (it takes
            // the name of the newest member) or the move was done already.
            if dir.exists() && !old.exists() {
                retry(|| archeion_win::rename_durably(&dir, &old))
                    .map_err(EngineError::io(format!("moving {}", dir.display())))?;
            }
        }
        let target = self.dir(&journal.target);
        if staging.exists() && !target.exists() {
            retry(|| archeion_win::rename_durably(staging, &target))
                .map_err(EngineError::io(format!("committing {}", target.display())))?;
        }
        if !self.merged_in_place(journal) {
            tracing::warn!(target_archive = %journal.target, "the merged archive is missing: the merge is undone");
            // There but unreadable (renamed from the staging folder, now
            // gone): set aside, so that a member of the same name (the
            // newest, when the merged archive takes its name) gets its
            // place back instead of being removed as a leftover.
            if target.exists() && !staging.exists() {
                self.set_aside_damaged(&journal.target)?;
            }
            for m in &journal.members {
                self.put_back(m)?;
            }
            return self.remove_journal();
        }
        let mut left = false;
        for m in &journal.members {
            let old = self.root.join(format!("{m}.{OLD}"));
            if old.exists() {
                left |= !remove_dir_best_effort(&old);
            }
        }
        if left {
            return Ok(());
        }
        self.remove_journal()
    }

    /// True when the merged archive of `journal` is in place and its
    /// manifest reads.
    fn merged_in_place(&self, journal: &Journal) -> bool {
        let dir = self.dir(&journal.target);
        dir.is_dir() && read_manifest(&dir, &journal.target, self.keys()).is_ok()
    }

    /// Renames the archive folder `name`, found unreadable during a
    /// recovery, to `<name>.damaged` (`.damaged-2`… when taken). Nothing
    /// removes it: it may still hold the only copy of some file.
    fn set_aside_damaged(&self, name: &ArchiveName) -> Result<()> {
        let dir = self.dir(name);
        let mut aside = self.root.join(format!("{name}.{DAMAGED}"));
        for n in 2.. {
            if !aside.exists() {
                break;
            }
            aside = self.root.join(format!("{name}.{DAMAGED}-{n}"));
        }
        tracing::warn!(archive = %name, folder = %aside.display(), "an unreadable archive set aside");
        retry(|| archeion_win::rename_durably(&dir, &aside))
            .map_err(EngineError::io(format!("setting aside {}", dir.display())))
    }

    /// Folders set aside by [`Self::set_aside_damaged`], sorted: the check
    /// names them, since nothing else does.
    pub(crate) fn set_aside(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = fs::read_dir(&self.root)
            .map_err(EngineError::io("listing the destination"))?
            .flatten()
            .filter(is_real_dir)
            .filter_map(|e| e.file_name().to_str().map(str::to_owned))
            .filter(|n| {
                n.split_once('.').is_some_and(|(stem, rest)| {
                    stem.parse::<ArchiveName>().is_ok()
                        && (rest == DAMAGED || rest.strip_prefix(DAMAGED).is_some_and(|r| r.starts_with('-')))
                })
            })
            .collect();
        names.sort_unstable();
        Ok(names)
    }

    /// `name.old` becomes `name` again, when there is no `name`.
    fn put_back(&self, name: &ArchiveName) -> Result<()> {
        let old = self.root.join(format!("{name}.{OLD}"));
        let dir = self.dir(name);
        if old.is_dir() && !dir.exists() {
            retry(|| archeion_win::rename_durably(&old, &dir))
                .map_err(EngineError::io(format!("putting back {}", dir.display())))?;
        }
        Ok(())
    }

    fn remove_journal(&self) -> Result<()> {
        let jpath = self.root.join(JOURNAL);
        retry(|| fs::remove_file(&jpath)).map_err(EngineError::io("removing the merge journal"))
    }

    /// Completes an interrupted merge and removes abandoned merge and
    /// deletion folders. Unfinished backups are kept for the next backup to
    /// resume ([`Self::take_partial`]). Returns true when the list of
    /// archives changed (a merge or a deletion was completed or undone).
    ///
    /// Nothing is removed that may be the only copy: an `.old` member whose
    /// archive is not in place gets its name back, whatever the journal
    /// says (on exFAT or a share the journal itself may have been lost).
    /// Only folders named after an archive are touched: `<archive>.old`,
    /// `.merging`, `.deleting`; anything else in the folder is not Archeion's.
    fn recover(&self) -> Result<bool> {
        let jpath = self.root.join(JOURNAL);
        let mut changed = jpath.exists();
        if changed {
            let text = read_small(&jpath)?;
            match serde_json::from_str::<Journal>(&text) {
                Ok(journal) if journal.format > JOURNAL_FORMAT => {
                    return Err(EngineError::Damaged {
                        archive: JOURNAL.to_owned(),
                        message: format!("format {} is newer than this version of Archeion", journal.format),
                    });
                }
                Ok(journal) => {
                    tracing::warn!(target_archive = %journal.target, "completing an interrupted merge");
                    let staging = self.root.join(format!("{}.{MERGING}", journal.target));
                    self.finish_merge(&staging, &journal)?;
                }
                // Not written whole, so nothing was moved after it: the
                // loop below puts back whatever may have been.
                Err(e) => {
                    tracing::warn!(error = %e, "an unreadable merge journal: the merge counts as not done");
                    self.remove_journal()?;
                }
            }
        }
        // (folder name, archive, suffix) of every `<archive>.<suffix>` folder.
        let leftovers: Vec<(String, ArchiveName, &str)> = fs::read_dir(&self.root)
            .map_err(EngineError::io("listing the destination"))?
            .flatten()
            .filter(is_real_dir)
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_owned();
                let (stem, suffix) = name.rsplit_once('.')?;
                let suffix = [MERGING, DELETING, OLD].into_iter().find(|s| *s == suffix)?;
                Some((name.clone(), stem.parse().ok()?, suffix))
            })
            .collect();
        // A member moved aside whose archive is not back: its name again.
        // One whose place is taken by an archive that does not read (a
        // merged archive damaged before its journal was found) gets it
        // back too: the moved-aside copy is never removed for a broken one.
        for (_, member, suffix) in &leftovers {
            if *suffix != OLD {
                continue;
            }
            let dir = self.dir(member);
            if dir.exists() && read_manifest(&dir, member, self.keys()).is_err() {
                self.set_aside_damaged(member)?;
            }
            if !dir.exists() {
                tracing::warn!(
                    archive = %member,
                    "putting back an archive left aside by an interrupted merge"
                );
                self.put_back(member)?;
                changed = true;
            }
        }
        for (name, _, suffix) in &leftovers {
            let path = self.root.join(name);
            // With the journal handled above, an `.old` folder still there
            // has its archive in place: a leftover.
            if path.is_dir() {
                tracing::info!(folder = %name, "removing an interrupted run");
                // Still in use: tried again next time; the run goes on.
                remove_dir_best_effort(&path);
                // An interrupted deletion had already removed an archive.
                changed |= *suffix == DELETING;
            }
        }
        Ok(changed)
    }
}

/// Runs a file operation again for a few seconds while Windows reports
/// access denied or a sharing violation: antivirus, the search indexer and
/// thumbnail makers open new files for a moment, and a folder holding an
/// open file can be neither renamed nor removed.
pub(crate) fn retry<T>(mut op: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    const WAITS_MS: [u64; 8] = [50, 100, 200, 400, 600, 800, 1000, 1500];
    let mut result = op();
    for wait in WAITS_MS {
        match &result {
            Err(e) if is_transient(e) => {
                tracing::debug!(error = %e, wait_ms = wait, "file in use: trying again");
                std::thread::sleep(std::time::Duration::from_millis(wait));
                result = op();
            }
            _ => break,
        }
    }
    result
}

/// Access denied (5), sharing violation (32) or lock violation (33) on
/// Windows; elsewhere permission errors are permanent.
fn is_transient(e: &std::io::Error) -> bool {
    cfg!(windows)
        && (e.kind() == std::io::ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(5 | 32 | 33)))
}

/// Removes `dir` with [`retry`]; false (and a warning) when it stays.
fn remove_dir_best_effort(dir: &Path) -> bool {
    match retry(|| fs::remove_dir_all(dir)) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(folder = %dir.display(), error = %e, "folder still in use: removed by a later run");
            false
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Purpose {
    Backup,
    Merge,
}

fn read_descriptor(root: &Path) -> Result<Option<Descriptor>> {
    let path = root.join(DESCRIPTOR);
    match read_small_io(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| EngineError::Damaged { archive: DESCRIPTOR.to_owned(), message: e.to_string() }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(EngineError::io(format!("reading {}", path.display()))(e)),
    }
}

/// Keys of the destination at `root` for read-only queries (no lock).
/// `Ok(None)` when it is not encrypted.
///
/// # Errors
/// [`EngineError::PasswordNeeded`] or [`EngineError::WrongPassword`] for an
/// encrypted destination without the right password.
pub(crate) fn unlock(root: &Path, password: Option<&Password>) -> Result<Option<Keys>> {
    match read_descriptor(root)?.and_then(|d| d.encryption) {
        None => Ok(None),
        Some(info) => info.unlock(root, password).map(Some),
    }
}

/// True when the destination at `root` is encrypted.
pub(crate) fn is_encrypted(root: &Path) -> Result<bool> {
    Ok(read_descriptor(root)?.is_some_and(|d| d.encryption.is_some()))
}

/// True when an archive at `root` has an encrypted manifest, whatever the
/// description says: removing the description does not make a destination
/// plain.
pub(crate) fn holds_sealed_archives(root: &Path) -> Result<bool> {
    Ok(archives_in(root)?.iter().any(|a| root.join(a.to_string()).join(MANIFEST_ZIP).exists()))
}

/// A folder itself, not a link to one: a symbolic link or junction in a
/// destination was not made by Archeion, and following it could reach
/// any folder of the computer.
fn is_real_dir(entry: &fs::DirEntry) -> bool {
    entry.file_type().is_ok_and(|t| t.is_dir())
}

/// True when `dir` holds a symbolic link or a junction anywhere below it
/// (one that cannot be listed counts as one).
pub(crate) fn holds_links(dir: &Path) -> bool {
    walkdir::WalkDir::new(dir).into_iter().any(|e| match e {
        Ok(e) => e.path_is_symlink(),
        Err(_) => true,
    })
}

pub(crate) fn archives_in(root: &Path) -> Result<Vec<ArchiveName>> {
    let mut list: Vec<ArchiveName> = fs::read_dir(root)
        .map_err(EngineError::io(format!("listing {}", root.display())))?
        .flatten()
        .filter(is_real_dir)
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.parse().ok()))
        .collect();
    list.sort_unstable();
    Ok(list)
}

pub(crate) fn read_manifest(dir: &Path, name: &ArchiveName, keys: Option<&Keys>) -> Result<Manifest> {
    let damaged = |message: String| EngineError::Damaged { archive: name.to_string(), message };
    let sealed = dir.join(MANIFEST_ZIP);
    // An encrypted destination reads sealed manifests only: a plain one put
    // in its place could say anything.
    let text = if sealed.exists() || keys.is_some() {
        let keys = keys.ok_or(EngineError::PasswordNeeded)?;
        if !sealed.exists() {
            return Err(damaged("the encrypted manifest is missing".to_owned()));
        }
        let bytes = read_limited(&sealed)?;
        let plain = decrypt_single(&bytes, keys.password(), MAX_MANIFEST_BYTES)?;
        String::from_utf8(plain).map_err(|e| damaged(e.to_string()))?
    } else {
        let path = dir.join(MANIFEST);
        String::from_utf8(read_limited(&path)?).map_err(|e| damaged(e.to_string()))?
    };
    Manifest::from_text(&text).map_err(|e| damaged(e.to_string()))
}

/// Largest manifest read: about ten million files. A bigger file (or one
/// that unpacks bigger) is refused instead of filling the memory.
pub(crate) const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// The bytes of `path`, refused beyond [`MAX_MANIFEST_BYTES`].
fn read_limited(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read as _;
    let ctx = || format!("reading {}", path.display());
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|f| f.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes))
        .map_err(EngineError::io(ctx()))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(EngineError::io(ctx())(std::io::Error::other("the manifest is too large")));
    }
    Ok(bytes)
}

/// Largest small file of a destination read: the descriptor, the key file,
/// the signature of the list, the merge journal and the copy of the
/// profile are a few kilobytes.
pub(crate) const MAX_SMALL_BYTES: u64 = 1024 * 1024;

/// The bytes of `path`, a small file of the destination: refused (as
/// `InvalidData`) beyond [`MAX_SMALL_BYTES`], so a huge file put there
/// cannot fill the memory.
fn read_small_bytes_io(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_SMALL_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SMALL_BYTES {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "the file is too large"));
    }
    Ok(bytes)
}

/// The text of `path`, a small file of the destination: refused (as
/// `InvalidData`) beyond [`MAX_SMALL_BYTES`] or when it is not UTF-8.
pub(crate) fn read_small_io(path: &Path) -> std::io::Result<String> {
    String::from_utf8(read_small_bytes_io(path)?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// [`read_small_io`] with the file named in the error.
pub(crate) fn read_small(path: &Path) -> Result<String> {
    read_small_io(path).map_err(EngineError::io(format!("reading {}", path.display())))
}

/// Writes `manifest.jsonl`, or the encrypted `manifest.zip` with `keys`.
pub(crate) fn write_manifest(dir: &Path, m: &Manifest, keys: Option<&Keys>) -> Result<()> {
    match keys {
        None => write_durably(&dir.join(MANIFEST), m.to_text().as_bytes()),
        Some(k) => {
            let sealed = encrypt_single(m.to_text().as_bytes(), MANIFEST, k.password())?;
            write_durably(&dir.join(MANIFEST_ZIP), &sealed)
        }
    }
}

/// Writes and flushes to disk before returning.
pub(crate) fn write_durably(path: &Path, bytes: &[u8]) -> Result<()> {
    let ctx = || format!("writing {}", path.display());
    let mut f = File::create(path).map_err(EngineError::io(ctx()))?;
    f.write_all(bytes).map_err(EngineError::io(ctx()))?;
    f.sync_all().map_err(EngineError::io(ctx()))
}

/// Archives of `profile`, newest first. Does not lock: a running backup is
/// invisible until committed.
///
/// # Errors
/// [`EngineError::DestinationUnavailable`] when the drive or share is not
/// reachable, [`EngineError::Damaged`] for an unreadable manifest,
/// [`EngineError::PasswordNeeded`] or [`EngineError::WrongPassword`] for an
/// encrypted destination.
pub fn list_archives(profile: &Profile, password: Option<&Password>) -> Result<Vec<ArchiveInfo>> {
    let root = crate::repo::destination_of(profile)?;
    ensure_reachable(root)?;
    if !root.exists() {
        return Ok(Vec::new());
    }
    let names = archives_in(root)?;
    let keys = if names.is_empty() { None } else { unlock(root, password)? };
    let mut out = Vec::new();
    for name in names.into_iter().rev() {
        let dir = root.join(name.to_string());
        let m = read_manifest(&dir, &name, keys.as_ref())?;
        let files = m.entries.iter().filter(|e| e.kind == EntryKind::File);
        let (mut total, mut stored, mut bytes) = (0, 0, 0u64);
        for e in files {
            total += 1;
            if e.stored {
                stored += 1;
                bytes = bytes.saturating_add(e.size);
            }
        }
        out.push(ArchiveInfo {
            name: name.to_string(),
            kind: name.kind,
            time_label: name.label(),
            files_total: total,
            files_stored: stored,
            bytes_stored: bytes,
            container: crate::container::detect(&dir),
        });
    }
    Ok(out)
}

/// Total size of the files under `dir`.
pub(crate) fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}
