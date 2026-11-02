//! Restore files as they were in given archives.
//!
//! A request lists items, each a file or folder of one archive sent to one
//! place; they run together: one free-space check, one progress, one
//! report. Where each file goes, and the date added to the name when two
//! would land on the same path, come from `archeion_core::restore`.

use crate::{
    EngineError, EventSink, Result,
    chunks::{ChunkIndex, ChunkReader, Form},
    container::{HashingWriter, Reader, copy_into, system_time},
    crypto::{Keys, Password},
    progress::Progress,
    repo::{Repo, archives_in, read_manifest},
    space,
};
pub use archeion_core::restore::{RestoreItem, RestoreRequest, RestoreTarget};
use archeion_core::{
    ArchiveName, Chunks, Entry, EntryKind, FileAction, Manifest, Phase, Profile, RestoreReport, RunEvent,
    Skipped, Summary,
    restore::{Output, date_tag, make_distinct, output_path, precise_date_tag},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet, hash_map},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

/// Restores the items of `request`.
///
/// Each file's content comes from the newest archive, up to the item's
/// own, that stores it; a file stored as chunks is put back together from
/// its chunks, each checked against its name, and the whole file against
/// its hash. Files are written to a temporary name and renamed, so an
/// interruption never leaves a half-written file under the real name.
///
/// # Errors
/// See [`EngineError`]; [`EngineError::Damaged`] when an item's archive does
/// not exist. Files that cannot be written are reported as skipped, not as
/// errors.
pub fn restore(
    profile: &Profile,
    password: Option<&Password>,
    request: &RestoreRequest,
    sink: &dyn EventSink,
) -> Result<RestoreReport> {
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let missing = || EngineError::Damaged {
        archive: request.items.first().map(|i| i.archive.clone()).unwrap_or_default(),
        message: "archive not found".into(),
    };
    let repo = Repo::open(profile, false, password)?.ok_or_else(missing)?;
    restore_from(repo.root(), repo.keys(), request, sink)
}

/// One file or folder about to be written.
#[derive(Debug)]
struct Planned<'a> {
    entry: &'a Entry,
    /// Number of the archive whose date is restored.
    archive: usize,
    dest: PathBuf,
    /// Sent to its original place.
    original: bool,
    overwrite: bool,
    /// Newest archive up to `archive` that stores the file, with its chunks
    /// when it stores them.
    holder: Option<(usize, Option<Chunks>)>,
}

/// [`restore`] from the destination folder `root`, already opened (and
/// unlocked with `keys` when encrypted) by the caller.
pub(crate) fn restore_from(
    root: &Path,
    keys: Option<&Keys>,
    request: &RestoreRequest,
    sink: &dyn EventSink,
) -> Result<RestoreReport> {
    let archives = archives_in(root)?;
    let dirs: Vec<PathBuf> = archives.iter().map(|a| root.join(a.to_string())).collect();
    let positions =
        request.items.iter().map(|i| position(&archives, &i.archive)).collect::<Result<Vec<usize>>>()?;
    // The manifest of every date restored, read once.
    let mut chosen: BTreeMap<usize, Manifest> = BTreeMap::new();
    for &k in &positions {
        if let (false, Some(name), Some(dir)) = (chosen.contains_key(&k), archives.get(k), dirs.get(k)) {
            chosen.insert(k, read_manifest(dir, name, keys)?);
        }
    }
    let mut planned = select(request, &positions, &chosen, &archives);
    let mut index = ChunkIndex::default();
    let last = find_holders(&mut planned, &chosen, &archives, &dirs, keys, &mut index)?;
    // Every archive of a destination has its first format.
    let form = Form::of(
        crate::container::detect(last.and_then(|j| dirs.get(j)).map_or(root, PathBuf::as_path)),
        keys,
    );
    if planned.iter().any(|p| p.holder.as_ref().is_some_and(|(_, c)| c.as_ref().is_some_and(|c| !c.whole))) {
        for (j, dir) in dirs.iter().enumerate().take(last.map_or(0, |l| l + 1)) {
            index.add_files(j, form.list(dir)?);
        }
    }
    let chunk_reader = ChunkReader { index: &index, dirs: &dirs, form: &form, keys };
    ensure_room(
        &planned
            .iter()
            .filter(|p| p.entry.kind == EntryKind::File)
            .map(|p| (p.entry.size, p.dest.as_path(), p.overwrite))
            .collect::<Vec<_>>(),
    )?;
    // Folders first; files grouped by the archive they are read from.
    planned.sort_by(|a, b| {
        let order = |p: &Planned<'_>| (p.entry.kind == EntryKind::File, p.holder.as_ref().map(|h| h.0));
        order(a).cmp(&order(b)).then_with(|| a.entry.key.cmp(&b.entry.key))
    });

    sink.emit(RunEvent::Phase { phase: Phase::Restoring });
    let files = planned.iter().filter(|p| p.entry.kind == EntryKind::File);
    let (total, count) = files.fold((0u64, 0u64), |(b, n), p| (b.saturating_add(p.entry.size), n + 1));
    let mut progress = Progress::new(sink, total, count);
    let mut report = RestoreReport::default();
    let mut readers: Vec<Option<Reader>> = archives.iter().map(|_| None).collect();
    let skip = |progress: &mut Progress<'_>, report: &mut RestoreReport, path: &Path, reason: String| {
        let s = Skipped { path: path.display().to_string(), reason };
        progress.skipped(s.clone());
        report.skipped.push(s);
    };

    for p in &planned {
        let (e, dest) = (p.entry, &p.dest);
        if e.kind == EntryKind::Dir {
            if let Err(err) = fs::create_dir_all(dest) {
                skip(&mut progress, &mut report, dest, err.to_string());
            }
            continue;
        }
        progress.start_file(dest.display().to_string());
        if dest.exists() && !p.overwrite {
            report.files_kept += 1;
            progress.add(e.size);
            progress.file_done(FileAction::Kept, e.size);
            continue;
        }
        let Some((j, chunks)) = &p.holder else {
            skip(&mut progress, &mut report, dest, "no archive holds this file".into());
            continue;
        };
        let result = match chunks {
            Some(c) if !c.whole => {
                let mut fill = |out: &mut dyn std::io::Write| {
                    chunk_reader.write_all(&c.list, out, &mut |n| progress.add(n))
                };
                write_file(dest, e.mtime_ns, e.hash.as_deref(), &e.key, &mut fill)
            }
            _ => {
                let (Some(slot), Some(dir)) = (readers.get_mut(*j), dirs.get(*j)) else {
                    skip(&mut progress, &mut report, dest, "no archive holds this file".into());
                    continue;
                };
                if slot.is_none() {
                    *slot = Some(Reader::open(dir, keys)?);
                }
                let reader = slot.as_mut().expect("opened above");
                let mut fill = |out: &mut dyn std::io::Write| {
                    reader.read(&e.key, |src| {
                        copy_into(src, out, &mut |n| progress.add(n))
                            .map(|_| ())
                            .map_err(EngineError::io(format!("restoring {}", e.key)))
                    })
                };
                write_file(dest, e.mtime_ns, e.hash.as_deref(), &e.key, &mut fill)
            }
        };
        match result {
            Ok(()) => {
                report.files_restored += 1;
                progress.file_done(FileAction::Restored, e.size);
            }
            Err(err) => skip(&mut progress, &mut report, dest, err.to_string()),
        }
    }
    progress.finish();
    sink.emit(RunEvent::Finished { summary: Summary::Restore(report.clone()) });
    Ok(report)
}

/// Number of the archive called `archive` in `archives`.
fn position(archives: &[ArchiveName], archive: &str) -> Result<usize> {
    let missing =
        || EngineError::Damaged { archive: archive.to_owned(), message: "archive not found".into() };
    let name: ArchiveName = archive.parse().map_err(|_| missing())?;
    archives.iter().position(|a| *a == name).ok_or_else(missing)
}

/// Every file and folder the items ask for, with where it goes. The same
/// file of the same archive asked twice for the same place is written
/// once; files that would land on the same path get distinct names
/// ([`make_distinct`]).
fn select<'a>(
    request: &RestoreRequest,
    positions: &[usize],
    chosen: &'a BTreeMap<usize, Manifest>,
    archives: &[ArchiveName],
) -> Vec<Planned<'a>> {
    let mut planned: Vec<Planned<'a>> = Vec::new();
    let mut seen: HashMap<(usize, &'a str, PathBuf), usize> = HashMap::new();
    for (item, &k) in request.items.iter().zip(positions) {
        let Some(manifest) = chosen.get(&k) else { continue };
        let tag = archives.get(k).map(|a| date_tag(&a.to_string())).unwrap_or_default();
        let original = item.target == RestoreTarget::Original;
        let wanted = |e: &&Entry| item.wants(&e.key);
        for entry in manifest.entries.iter().filter(wanted) {
            let dest = output_path(&entry.key, entry.kind, &item.target, &tag);
            match seen.entry((k, entry.key.as_str(), dest.clone())) {
                hash_map::Entry::Occupied(o) => {
                    if let Some(p) = planned.get_mut(*o.get()) {
                        p.overwrite |= item.overwrite;
                    }
                }
                hash_map::Entry::Vacant(v) => {
                    v.insert(planned.len());
                    let overwrite = item.overwrite;
                    planned.push(Planned { entry, archive: k, dest, original, overwrite, holder: None });
                }
            }
        }
    }
    let files: Vec<usize> = (0..planned.len())
        .filter(|&i| planned.get(i).is_some_and(|p| p.entry.kind == EntryKind::File))
        .collect();
    let mut outputs: Vec<Output> = files
        .iter()
        .filter_map(|&i| planned.get(i))
        .map(|p| Output {
            path: p.dest.clone(),
            tag: archives.get(p.archive).map(|a| date_tag(&a.to_string())).unwrap_or_default(),
            precise_tag: archives
                .get(p.archive)
                .map(|a| precise_date_tag(&a.to_string()))
                .unwrap_or_default(),
            original: p.original,
        })
        .collect();
    make_distinct(&mut outputs);
    for (&i, o) in files.iter().zip(outputs) {
        if let Some(p) = planned.get_mut(i) {
            p.dest = o.path;
        }
    }
    planned
}

/// Finds, for every planned file, the newest archive up to its own that
/// stores it, reading each manifest once (those in `chosen` are not read
/// again), and adds their whole copies to `index`. Returns the newest
/// archive looked at, `None` when nothing is planned.
fn find_holders<'a>(
    planned: &mut [Planned<'a>],
    chosen: &BTreeMap<usize, Manifest>,
    archives: &[ArchiveName],
    dirs: &[PathBuf],
    keys: Option<&Keys>,
    index: &mut ChunkIndex,
) -> Result<Option<usize>> {
    let Some(last) = planned.iter().map(|p| p.archive).max() else { return Ok(None) };
    let wanted: HashSet<&'a str> = planned
        .iter()
        .filter(|p| p.entry.kind == EntryKind::File)
        .map(|p| {
            let e: &'a Entry = p.entry;
            e.key.as_str()
        })
        .collect();
    let mut latest: HashMap<&'a str, (usize, Option<Chunks>)> = HashMap::new();
    for (j, (name, dir)) in archives.iter().zip(dirs).enumerate().take(last + 1) {
        let fresh;
        let manifest = match chosen.get(&j) {
            Some(m) => m,
            None => {
                fresh = read_manifest(dir, name, keys)?;
                &fresh
            }
        };
        index.add_whole_copies(j, manifest);
        for e in manifest.entries.iter().filter(|e| e.stored) {
            if let Some(&key) = wanted.get(e.key.as_str()) {
                latest.insert(key, (j, e.chunks.clone()));
            }
        }
        if chosen.contains_key(&j) {
            for p in planned.iter_mut().filter(|p| p.archive == j && p.entry.kind == EntryKind::File) {
                p.holder = latest.get(p.entry.key.as_str()).cloned();
            }
        }
    }
    Ok(Some(last))
}

/// Fails with [`EngineError::NotEnoughSpace`] when a drive cannot hold the
/// files about to be written to it: `(size, path, overwrite)` for every
/// file of every item. Files kept as they are (they exist and are not to
/// be replaced) need no room; a replaced file is written beside the old
/// one before the swap, so it counts in full.
fn ensure_room(files: &[(u64, &Path, bool)]) -> Result<()> {
    let mut by_drive: BTreeMap<PathBuf, (u64, &Path)> = BTreeMap::new();
    for &(size, dest, overwrite) in files {
        if !overwrite && dest.exists() {
            continue;
        }
        let drive = dest.ancestors().last().unwrap_or(dest).to_path_buf();
        let slot = by_drive.entry(drive).or_insert((0, dest));
        slot.0 = slot.0.saturating_add(size);
    }
    for (needed, sample) in by_drive.into_values() {
        if let Some(dir) = space::existing_ancestor(sample) {
            space::ensure(dir, needed)?;
        }
    }
    Ok(())
}

/// Writes what `fill` produces to `dest`: to a temporary name first,
/// checked against `expected_hash`, flushed to the disk, then renamed.
fn write_file(
    dest: &Path,
    mtime_ns: i64,
    expected_hash: Option<&str>,
    key: &str,
    fill: &mut dyn FnMut(&mut dyn std::io::Write) -> Result<()>,
) -> Result<()> {
    let ctx = format!("restoring {}", dest.display());
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(EngineError::io(ctx.clone()))?;
    }
    let tmp = dest.with_file_name(format!(
        "{}.archeion-tmp",
        dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    match write_checked(&tmp, mtime_ns, expected_hash, key, fill, &ctx) {
        Ok(()) => fs::rename(&tmp, dest).map_err(EngineError::io(ctx)),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// The body of [`write_file`]: writes `path` and checks its hash.
fn write_checked(
    path: &Path,
    mtime_ns: i64,
    expected_hash: Option<&str>,
    key: &str,
    fill: &mut dyn FnMut(&mut dyn std::io::Write) -> Result<()>,
    ctx: &str,
) -> Result<()> {
    let file = fs::File::create(path).map_err(EngineError::io(ctx.to_owned()))?;
    let mut out = HashingWriter::new(std::io::BufWriter::new(file));
    fill(&mut out)?;
    out.flush().map_err(EngineError::io(ctx.to_owned()))?;
    let (_, hash, buffered) = out.finish();
    if expected_hash.is_some_and(|h| h != hash) {
        return Err(EngineError::Damaged {
            archive: key.to_owned(),
            message: "content does not match".into(),
        });
    }
    let file = buffered.into_inner().map_err(|e| EngineError::io(ctx.to_owned())(e.into_error()))?;
    file.sync_all().map_err(EngineError::io(ctx.to_owned()))?;
    let _ = file.set_modified(system_time(mtime_ns));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restore_too_large_for_the_drive_is_refused_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out").join("big.bin");
        let huge = u64::MAX / 4;
        assert!(matches!(ensure_room(&[(huge, &dest, true)]), Err(EngineError::NotEnoughSpace { .. })));
        assert!(!dest.exists());

        // A file kept as it is needs no room.
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, "old").unwrap();
        assert!(ensure_room(&[(huge, &dest, false)]).is_ok());
        assert!(ensure_room(&[(10, &dest, true)]).is_ok());
    }

    #[test]
    fn the_space_check_adds_up_every_item() {
        let dir = tempfile::tempdir().unwrap();
        let Some(free) = space::available(dir.path()) else { return };
        // Each fits alone, both together do not.
        let each = free.saturating_sub(space::MARGIN) / 4 * 3;
        let (a, b) = (dir.path().join("a.bin"), dir.path().join("b.bin"));
        assert!(ensure_room(&[(each, &a, true)]).is_ok());
        assert!(matches!(
            ensure_room(&[(each, &a, true), (each, &b, true)]),
            Err(EngineError::NotEnoughSpace { .. })
        ));
    }
}
