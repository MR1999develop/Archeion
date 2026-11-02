//! Read-only queries for the restore browser: every file ever saved, the
//! tree of a date, the versions of one file, search by name. They take no
//! lock: a running backup is invisible until committed.

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

/// Every file and folder a profile's backups ever listed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    /// The backups, oldest first; [`CatalogEntry::present`] counts in this
    /// list.
    pub archives: Vec<CatalogArchive>,
    /// Sorted by key: a folder comes before what it holds.
    pub entries: Vec<CatalogEntry>,
}

/// One backup of a [`Catalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogArchive {
    /// Archive name, to restore from.
    pub name: String,
    /// Backup date, `YYYY-MM-DD HH:MM`.
    pub label: String,
}

/// A file or folder of a [`Catalog`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub key: String,
    pub kind: EntryKind,
    /// Bytes, as last listed.
    pub size: u64,
    /// The backups listing it, as runs of consecutive positions in
    /// [`Catalog::archives`], oldest first: `[(0, 3), (6, 6)]` was deleted
    /// after the fourth backup and came back in the seventh.
    pub present: Vec<(usize, usize)>,
}

impl CatalogEntry {
    /// Not listed by the newest backup: deleted at the source since.
    #[must_use]
    pub fn deleted(&self, archives: usize) -> bool {
        self.present.last().is_none_or(|&(_, last)| last.saturating_add(1) != archives)
    }

    /// Positions of the backups listing it, newest first.
    pub fn archives_newest_first(&self) -> impl Iterator<Item = usize> + '_ {
        self.present.iter().rev().flat_map(|&(first, last)| (first..=last).rev())
    }
}

/// Every file and folder listed by any backup of the profile, with the
/// backups listing it. Manifests are read one at a time.
///
/// # Errors
/// See [`archive_manifest`].
pub fn catalog(profile: &Profile, password: Option<&Password>) -> Result<Catalog> {
    let Some(root) = root(profile)? else { return Ok(Catalog::default()) };
    let names = archives_in(root)?;
    if names.is_empty() {
        return Ok(Catalog::default());
    }
    let keys = unlock(root, password)?;
    let mut entries: BTreeMap<String, CatalogEntry> = BTreeMap::new();
    for (j, name) in names.iter().enumerate() {
        let manifest = read_manifest(&root.join(name.to_string()), name, keys.as_ref())?;
        for e in manifest.entries {
            match entries.get_mut(e.key.as_str()) {
                Some(c) => {
                    c.size = e.size;
                    c.kind = e.kind;
                    match c.present.last_mut() {
                        Some(run) if run.1.saturating_add(1) == j => run.1 = j,
                        _ => c.present.push((j, j)),
                    }
                }
                None => {
                    let key = e.key;
                    let c =
                        CatalogEntry { key: key.clone(), kind: e.kind, size: e.size, present: vec![(j, j)] };
                    entries.insert(key, c);
                }
            }
        }
    }
    let archives = names.iter().map(|n| CatalogArchive { name: n.to_string(), label: n.label() }).collect();
    Ok(Catalog { archives, entries: entries.into_values().collect() })
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

/// True when the drive or share of the destination is there (the folder
/// itself may not exist yet). Cheap: for event backups that must skip
/// quietly when another drive was plugged in.
#[must_use]
pub fn destination_reachable(profile: &Profile) -> bool {
    let root = profile.destination.path();
    !root.as_os_str().is_empty() && ensure_reachable(root).is_ok()
}

/// Free bytes on the drive or share of the destination (of its nearest
/// existing parent when the folder is not created yet); `None` when it is
/// not reachable or does not say.
#[must_use]
pub fn free_space(profile: &Profile) -> Option<u64> {
    let root = profile.destination.path();
    if root.as_os_str().is_empty() || ensure_reachable(root).is_err() {
        return None;
    }
    crate::space::available(crate::space::existing_ancestor(root)?)
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
