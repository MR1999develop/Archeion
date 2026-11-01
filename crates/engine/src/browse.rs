//! Read-only queries for the restore browser: the tree of a date, the
//! versions of one file, search by name. They take no lock: a running backup
//! is invisible until committed.

use crate::{
    EngineError, Result,
    crypto::Password,
    repo::{archives_in, ensure_reachable, is_encrypted, read_manifest, unlock},
};
use archeion_core::{ArchiveName, Container, EntryKind, Manifest, Profile};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

/// One distinct version of a file: an archive that stores its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVersion {
    /// Archive to restore from.
    pub archive: String,
    /// Backup date, `YYYY-MM-DD HH:MM`.
    pub backup_label: String,
    /// The file's own modification time, local, `YYYY-MM-DD HH:MM`.
    pub modified_label: String,
    pub size: u64,
}

/// A file whose name matches a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub key: String,
    /// Number of distinct versions.
    pub versions: usize,
    /// Newest archive listing the file.
    pub latest_archive: String,
    /// The file still exists in the newest backup.
    pub present_now: bool,
}

/// Folder of the profile's archives, or `None` when it does not exist yet.
fn root(profile: &Profile) -> Result<Option<&Path>> {
    let root = profile.destination.path();
    ensure_reachable(root)?;
    Ok(root.exists().then_some(root))
}

fn manifests(root: &Path, password: Option<&Password>) -> Result<Vec<(ArchiveName, Manifest)>> {
    let names = archives_in(root)?;
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let keys = unlock(root, password)?;
    names
        .into_iter()
        .map(|name| read_manifest(&root.join(name.to_string()), &name, keys.as_ref()).map(|m| (name, m)))
        .collect()
}

/// How the backups already at a destination are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExistingFormat {
    pub container: Container,
    pub encrypted: bool,
}

/// Format of the backups already at the destination: `Ok(None)` when it
/// has none yet, so the format and encryption can still be chosen. Needs no
/// password.
///
/// # Errors
/// [`EngineError::DestinationUnavailable`] when the drive or share is not
/// reachable.
pub fn existing_format(profile: &Profile) -> Result<Option<ExistingFormat>> {
    let Some(root) = root(profile)? else { return Ok(None) };
    let Some(last) = archives_in(root)?.last().copied() else { return Ok(None) };
    let container = crate::container::detect(&root.join(last.to_string()));
    Ok(Some(ExistingFormat { container, encrypted: is_encrypted(root)? }))
}

/// The recovery key of an encrypted destination: 64 hex characters in
/// groups of four, the password that opens every zip with 7-Zip.
/// `Ok(None)` for a destination that is not encrypted or does not exist.
///
/// # Errors
/// [`EngineError::WrongPassword`], or the destination is not reachable.
pub fn recovery_key(profile: &Profile, password: &Password) -> Result<Option<String>> {
    let Some(root) = root(profile)? else { return Ok(None) };
    Ok(unlock(root, Some(password))?.map(|k| k.recovery_key()))
}

/// Replaces the password of an encrypted destination. The backups are not
/// rewritten: only the wrapped master key changes. Waits for no run: fails
/// with [`EngineError::Busy`] while one uses the destination.
///
/// # Errors
/// [`EngineError::WrongPassword`] when `old` is wrong, or see [`EngineError`].
pub fn change_password(profile: &Profile, old: &Password, new: &Password) -> Result<()> {
    let repo = crate::repo::Repo::open(profile, false, Some(old))?
        .ok_or_else(|| EngineError::InvalidProfile("the destination does not exist".into()))?;
    repo.change_password(new)
}

/// Checks that `password` opens the profile's destination. A destination
/// that does not exist yet or is not encrypted accepts any password.
///
/// # Errors
/// [`EngineError::WrongPassword`], or the destination is not reachable.
pub fn check_password(profile: &Profile, password: &Password) -> Result<()> {
    let Some(root) = root(profile)? else { return Ok(()) };
    unlock(root, Some(password)).map(|_| ())
}

/// Every file and folder of one archive (the tree of that date).
///
/// # Errors
/// [`EngineError::DestinationUnavailable`], or [`EngineError::Damaged`] when
/// the archive does not exist or its manifest is unreadable.
pub fn archive_manifest(profile: &Profile, password: Option<&Password>, archive: &str) -> Result<Manifest> {
    let missing =
        || EngineError::Damaged { archive: archive.to_owned(), message: "archive not found".into() };
    let root = root(profile)?.ok_or_else(missing)?;
    let name: ArchiveName = archive.parse().map_err(|_| missing())?;
    let keys = unlock(root, password)?;
    read_manifest(&root.join(archive), &name, keys.as_ref())
}

/// Distinct versions of the file `key`, newest first. A version is listed
/// once, at the archive that first stores it, however many later backups
/// kept it unchanged or copied it again (complete backups).
///
/// # Errors
/// See [`archive_manifest`].
pub fn file_versions(profile: &Profile, password: Option<&Password>, key: &str) -> Result<Vec<FileVersion>> {
    let Some(root) = root(profile)? else { return Ok(Vec::new()) };
    let mut out: Vec<FileVersion> = Vec::new();
    let mut last: Option<(u64, i64, Option<String>)> = None;
    for (name, m) in manifests(root, password)? {
        let Some(e) = m.get(key).filter(|e| e.kind == EntryKind::File && e.stored) else { continue };
        let same = last.as_ref().is_some_and(|(size, mtime, hash)| {
            *size == e.size && *mtime == e.mtime_ns && (hash.is_none() || *hash == e.hash)
        });
        last = Some((e.size, e.mtime_ns, e.hash.clone()));
        if same {
            continue;
        }
        out.push(FileVersion {
            archive: name.to_string(),
            backup_label: name.label(),
            modified_label: local_label(e.mtime_ns),
            size: e.size,
        });
    }
    out.reverse();
    Ok(out)
}

/// Files whose name contains `query` (case-insensitive) in any archive,
/// sorted by path, at most `limit`.
///
/// # Errors
/// See [`archive_manifest`].
pub fn search(
    profile: &Profile,
    password: Option<&Password>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let Some(root) = root(profile)? else { return Ok(Vec::new()) };
    let all = manifests(root, password)?;
    let latest = all.last().map(|(n, _)| *n);
    let mut hits: BTreeMap<String, SearchHit> = BTreeMap::new();
    let mut last_stored: HashMap<String, (u64, i64)> = HashMap::new();
    for (name, m) in &all {
        for e in m.entries.iter().filter(|e| e.kind == EntryKind::File) {
            let file_name = e.key.rsplit('/').next().unwrap_or(&e.key);
            if !file_name.to_lowercase().contains(&needle) {
                continue;
            }
            let hit = hits.entry(e.key.clone()).or_insert_with(|| SearchHit {
                key: e.key.clone(),
                versions: 0,
                latest_archive: String::new(),
                present_now: false,
            });
            // A complete backup stores unchanged files again: count content
            // changes, not copies.
            if e.stored
                && last_stored.insert(e.key.clone(), (e.size, e.mtime_ns)) != Some((e.size, e.mtime_ns))
            {
                hit.versions += 1;
            }
            hit.latest_archive = name.to_string();
            hit.present_now = Some(*name) == latest;
        }
    }
    Ok(hits.into_values().take(limit).collect())
}

/// Nanoseconds since the epoch as local `YYYY-MM-DD HH:MM`.
fn local_label(mtime_ns: i64) -> String {
    jiff::Timestamp::from_nanosecond(i128::from(mtime_ns))
        .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}
