//! Backup: scan, compare with the previous manifest, copy what changed (or
//! everything, for a complete backup) into a new archive.
//!
//! Each file to copy is stored as the profile and the destination say
//! ([`archeion_core::large_files`]): a whole copy, a whole copy with the
//! list of its chunks (complete archives), or only the chunks the
//! destination does not hold yet (see [`crate::chunks`]).

use crate::{
    EngineError, EventSink, Result,
    chunker::{ChunkParams, Chunker},
    chunks::{self, Form},
    container::{Writer, hash_stream, mtime_ns},
    crypto::Password,
    progress::Progress,
    repo::{Purpose, Repo},
    resume::{Done, Journal},
    space,
};
use archeion_core::exclusion::PathRules;
use archeion_core::large_files::{self, ChunkPolicy, FileStorage};
use archeion_core::{
    ArchiveKind, ArchiveName, BackupMode, BackupReport, ChunkId, ChunkRef, Chunks, Entry, EntryKind,
    Exclusion, FileAction, FileDone, Manifest, Phase, Profile, RunEvent, Skipped, Summary,
    backup_kind::next_kind, keys,
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

/// Bytes read from a source at a time when it is chunked.
const READ_BUFFER: usize = 256 * 1024;

/// Settings of a backup that tests change; production code uses the
/// default.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Tuning {
    /// Chunk sizes; `None` for [`ChunkParams::PRODUCTION`].
    pub chunks: Option<ChunkParams>,
    /// Stop, as if cancelled, once this many files have been copied.
    pub stop_after_files: Option<u64>,
    /// The destination's file system as given, instead of asking Windows.
    pub file_system: Option<&'static str>,
    /// The size FAT refuses, instead of 4 GiB.
    pub fat_max_file: Option<u64>,
}

/// Backs up `profile` into a new archive. The first backup is complete;
/// later ones are incremental or complete as `mode` (and, for
/// [`BackupMode::Auto`], the profile's full-backup rule) says. A complete
/// backup copies every file and starts a new chain. An automatic backup is
/// also complete when chunked files are on and the newest chain was written
/// before they existed ([`large_files::needs_new_chain`]).
///
/// Files that cannot be read (in use, access denied) are reported as
/// [`RunEvent::Skipped`]; their previous version stays restorable.
///
/// Every file stored and every file deleted since the previous backup is
/// listed by [`RunEvent::Files`]; unchanged files of an incremental backup
/// are only counted.
///
/// A backup that stopped before committing (Cancel, full disk, unplugged
/// drive, power cut) is resumed by the next one: files already copied and
/// unchanged since are kept, and so are chunks already written (see
/// [`crate::resume`]).
///
/// # Errors
/// See [`EngineError`]. An encrypted destination needs `password`;
/// [`EngineError::NotEnoughSpace`] when the destination is (or becomes)
/// too full.
pub fn backup(
    profile: &Profile,
    password: Option<&Password>,
    mode: BackupMode,
    sink: &dyn EventSink,
) -> Result<BackupReport> {
    run(profile, password, mode, sink, Tuning::default())
}

/// [`backup`] that stops, as if cancelled, once `files` files have been
/// copied: for testing the resumption of an interrupted backup.
///
/// # Errors
/// As [`backup`]; an `Io` error "stopped for a test" when it stops.
#[doc(hidden)]
pub fn backup_stopping_after(
    profile: &Profile,
    password: Option<&Password>,
    mode: BackupMode,
    sink: &dyn EventSink,
    files: u64,
) -> Result<BackupReport> {
    run(profile, password, mode, sink, Tuning { stop_after_files: Some(files), ..Tuning::default() })
}

/// [`backup`] with test settings (small chunks, a stop after some files).
///
/// # Errors
/// As [`backup`].
#[doc(hidden)]
pub fn backup_tuned(
    profile: &Profile,
    password: Option<&Password>,
    mode: BackupMode,
    sink: &dyn EventSink,
    tuning: Tuning,
) -> Result<BackupReport> {
    run(profile, password, mode, sink, tuning)
}

fn run(
    profile: &Profile,
    password: Option<&Password>,
    mode: BackupMode,
    sink: &dyn EventSink,
    tuning: Tuning,
) -> Result<BackupReport> {
    profile.validate().map_err(|errors| {
        let text: Vec<String> = errors.iter().map(ToString::to_string).collect();
        EngineError::InvalidProfile(text.join(", "))
    })?;
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let repo = Repo::open(profile, true, password)?.expect("created when missing");
    let archives = repo.archives()?;
    let previous = match archives.last() {
        Some(name) => Some(repo.manifest(name)?),
        None => None,
    };
    // The destination decides whether it is encrypted, not the profile.
    let encrypted = repo.keys().is_some();
    let chain_format = newest_complete_format(&repo, &archives, previous.as_ref())?;
    let new_chain = large_files::needs_new_chain(
        large_files::chunking_enabled(encrypted, profile.large_files),
        chain_format,
    );
    if new_chain {
        tracing::info!("the newest backups predate chunked files: an automatic backup starts a new chain");
    }
    let wanted = next_kind(&archives, profile.full_backup, mode, jiff::Zoned::now().datetime(), new_chain);
    let resumed = match repo.take_partial(&archives)? {
        Some(p) if can_resume(p.kind, wanted, mode, archives.is_empty()) => Some(p),
        Some(p) => {
            repo.discard_partial(&p)?;
            None
        }
        None => None,
    };
    // A resumed backup keeps its name: encrypted names depend on it.
    let (name, kind) = match resumed {
        Some(p) => {
            tracing::info!(archive = %p, "resuming an unfinished backup");
            (p, p.kind)
        }
        None => (unique_name(&archives, wanted), wanted),
    };

    sink.emit(RunEvent::Phase { phase: Phase::Scanning });
    let mut report = BackupReport { complete: kind == ArchiveKind::Complete, ..BackupReport::default() };
    let excludes = Excluder::new(&profile.excludes)?;
    let mut scan = Scan::new(previous.as_ref(), &excludes, kind == ArchiveKind::Complete);
    let mut scanning = Progress::new(sink, 0, 0);
    for source in top_level_sources(&profile.sources) {
        scan.walk(&source, sink, &mut scanning, &mut report);
    }
    scanning.finish();

    let staging = match resumed {
        Some(p) => repo.partial_dir(&p),
        None => repo.staging(&name, Purpose::Backup)?,
    };
    // Once a destination has backups, their format wins over the profile's:
    // a destination never mixes the two. Likewise for a resumed backup.
    let container = match archives.last() {
        Some(a) => crate::container::detect(&repo.dir(a)),
        None => crate::container::detect_existing(&staging).unwrap_or(profile.container),
    };
    let writer = Writer::create(&staging, container, repo.keys(), &name)?;
    let mut journal = Journal::open(&staging, repo.keys())?;
    let form = Form::of(container, repo.keys());
    let policy = ChunkPolicy {
        encrypted,
        large_files: profile.large_files,
        complete: kind == ArchiveKind::Complete,
        chain_has_chunks: large_files::chain_has_chunks(chain_format),
    };
    let params = tuning.chunks.unwrap_or(ChunkParams::PRODUCTION);

    // What each file needs; files the interrupted run already copied whole,
    // unchanged since, are kept as they are.
    let mut pending: Vec<Pending> = Vec::with_capacity(scan.to_copy.len());
    let mut kept: Vec<FileDone> = Vec::new();
    for (idx, path, change) in std::mem::take(&mut scan.to_copy) {
        let Some(entry) = scan.entries.get_mut(idx) else { continue };
        let storage = policy.storage(entry.size);
        let job = if storage == FileStorage::ChunksOnly {
            Job::Chunks
        } else {
            let list = storage == FileStorage::WholeAndChunkList;
            match journal.get(&writer.journal_id(&entry.key)) {
                Some(done)
                    if done.size == entry.size
                        && done.mtime_ns == entry.mtime_ns
                        && writer.looks_complete(&entry.key, done.size) =>
                {
                    if list {
                        // The chunk list is not journalled: read it again.
                        Job::ListOnly { hash: done.hash.clone() }
                    } else {
                        entry.hash = Some(done.hash.clone());
                        entry.stored = true;
                        entry.blob = writer.stored_as(&entry.key);
                        report.resumed_files += 1;
                        report.bytes_copied = report.bytes_copied.saturating_add(entry.size);
                        let file = FileDone::new(path.display().to_string(), change.action(), entry.size);
                        kept.push(FileDone { resumed: true, ..file });
                        continue;
                    }
                }
                _ if list => Job::CopyAndList,
                _ => Job::Copy,
            }
        };
        pending.push(Pending { idx, path, change, job });
    }

    if resumed.is_some() {
        tracing::info!(journalled = journal.len(), kept = report.resumed_files, "unfinished backup reopened");
    }

    let store = if pending.iter().any(|p| p.job == Job::Chunks) {
        Some(NewChunks::open(&repo, &archives, previous.as_ref(), &staging, &form)?)
    } else {
        None
    };
    let mut copier = Copier { writer, store, form: &form, params };

    // Room for what is left to copy.
    let previous_size = |key: &str| previous.as_ref().and_then(|m| m.get(key)).map(|e| e.size);
    let file_estimate = |copier: &Copier<'_>, p: &Pending, entries: &[Entry]| {
        entries.get(p.idx).map_or(0, |e| copier.estimate(&p.job, e, previous_size(&e.key)))
    };
    let mut to_write: u64 = pending.iter().map(|p| file_estimate(&copier, p, &scan.entries)).sum();
    space::ensure(repo.root(), to_write)?;

    sink.emit(RunEvent::Phase { phase: Phase::Copying });
    let total: u64 = pending.iter().filter_map(|p| scan.entries.get(p.idx)).map(|e| e.size).sum();
    let mut progress = Progress::new(sink, total, pending.len() as u64);
    for file in kept {
        progress.listed(file);
    }
    let mut drop_entries = HashSet::new();
    let mut copied_now = 0u64;
    // FAT and FAT32 hold no file of 4 GiB or more: such a file is skipped
    // with the reason (its last version kept), not left to fail half
    // written.
    let file_system =
        tuning.file_system.map(str::to_owned).or_else(|| archeion_win::file_system(repo.root()));
    let fat = file_system.is_some_and(|f| f.starts_with("FAT"));
    for p in pending {
        if tuning.stop_after_files.is_some_and(|n| copied_now >= n) {
            return Err(EngineError::Io {
                context: "backup".into(),
                source: std::io::Error::other("stopped for a test"),
            });
        }
        progress.check_stop()?;
        let estimate = file_estimate(&copier, &p, &scan.entries);
        let Some(entry) = scan.entries.get_mut(p.idx) else { continue };
        progress.start_file(p.path.display().to_string());
        let copied = if fat
            && entry.size >= tuning.fat_max_file.unwrap_or(FAT_MAX_FILE)
            && p.job != Job::Chunks
        {
            Err(EngineError::Io { context: "copying".into(), source: std::io::Error::other(FAT_TOO_LARGE) })
        } else {
            copier.copy_file(&p.job, &p.path, entry, &mut |n| progress.add(n))
        };
        match copied {
            Ok(stored) => {
                // A file that changed while being read is copied again next time.
                if stored.journal && stored.size == entry.size {
                    let done =
                        Done { size: stored.size, mtime_ns: entry.mtime_ns, hash: stored.hash.clone() };
                    journal.record(&copier.writer.journal_id(&entry.key), done).map_err(|e| {
                        if space::is_disk_full(&e) { space::full(repo.root(), to_write) } else { e }
                    })?;
                }
                entry.size = stored.size;
                entry.hash = Some(stored.hash);
                entry.stored = true;
                entry.blob = if stored.whole { copier.writer.stored_as(&entry.key) } else { None };
                entry.chunks = stored.chunks;
                if stored.resumed {
                    report.resumed_files += 1;
                    report.bytes_copied = report.bytes_copied.saturating_add(stored.size);
                } else {
                    report.bytes_copied = report.bytes_copied.saturating_add(stored.written);
                    copied_now += 1;
                }
                let as_changes = p.job == Job::Chunks;
                if as_changes {
                    report.files_as_changes += 1;
                }
                progress.done_with(FileDone {
                    new_chunk_bytes: as_changes.then_some(stored.written),
                    resumed: stored.resumed,
                    ..FileDone::new(p.path.display().to_string(), p.change.action(), stored.size)
                });
            }
            Err(e) if space::is_disk_full(&e) => {
                // Stop: skipping file after file would fill the log, not
                // the backup. What is copied stays for the next run.
                tracing::warn!(archive = %name, "destination full");
                return Err(space::full(repo.root(), to_write));
            }
            Err(e) => {
                let skipped = Skipped { path: p.path.display().to_string(), reason: reason(&e) };
                tracing::warn!(path = %skipped.path, reason = %skipped.reason, "file skipped");
                progress.skipped(skipped.clone());
                report.skipped.push(skipped);
                // Keep the last good version, if there is one.
                match previous.as_ref().and_then(|m| m.get(&entry.key)) {
                    Some(old) => {
                        *entry = old.clone();
                        entry.stored = false;
                        entry.blob = None;
                        entry.chunks = None;
                    }
                    None => {
                        drop_entries.insert(p.idx);
                    }
                }
                match p.change {
                    Change::New => report.files_new = report.files_new.saturating_sub(1),
                    Change::Changed => report.files_changed = report.files_changed.saturating_sub(1),
                    Change::Unchanged => {}
                }
            }
        }
        to_write = to_write.saturating_sub(estimate);
    }

    let entries: Vec<Entry> = scan
        .entries
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop_entries.contains(i))
        .map(|(_, e)| e)
        .collect();
    let mut manifest = Manifest::new(kind, entries);
    manifest.normalize();
    report.files_unchanged = scan.unchanged;
    if !encrypted {
        report.files_efs_in_clear = scan.efs;
    }
    if let Some(prev) = &previous {
        let now: HashSet<&str> = manifest.entries.iter().map(|e| e.key.as_str()).collect();
        let gone = prev.entries.iter().filter(|e| e.kind == EntryKind::File && !now.contains(e.key.as_str()));
        for gone in gone {
            report.files_deleted += 1;
            let path = keys::path_for(&gone.key).display().to_string();
            progress.listed(FileDone::new(path, FileAction::Deleted, gone.size));
        }
    }
    progress.finish();

    let Copier { writer, .. } = copier;
    if resumed.is_some() {
        let keep: HashSet<PathBuf> =
            manifest.entries.iter().filter(|e| e.whole_copy()).map(|e| writer.content_path(&e.key)).collect();
        let removed = writer.remove_others(&keep)?;
        if removed > 0 {
            tracing::info!(removed, "leftovers of the interrupted run removed");
        }
    }
    // Chunks written for a file that then failed, or left by an
    // interrupted run and no longer needed.
    let referenced: HashSet<ChunkId> = chunks::referenced(&manifest).map(|c| c.id).collect();
    let removed = form.remove_others(&staging, &referenced)?;
    if removed > 0 {
        tracing::info!(removed, "unused chunks removed");
    }
    writer.finish()?;
    repo.write_manifest(&staging, &manifest)?;
    journal.remove()?;
    repo.commit(&staging, &name)?;
    report.archive = name.to_string();
    tracing::info!(
        archive = %name,
        copied = report.bytes_copied,
        resumed = report.resumed_files,
        as_changes = report.files_as_changes,
        "archive committed"
    );
    sink.emit(RunEvent::Finished { summary: Summary::Backup(report.clone()) });
    Ok(report)
}

/// Format of the newest complete archive: the start of the chain an
/// incremental would continue. `previous` is the newest archive's manifest.
fn newest_complete_format(
    repo: &Repo,
    archives: &[ArchiveName],
    previous: Option<&Manifest>,
) -> Result<Option<u32>> {
    let Some(pos) = archives.iter().rposition(|a| a.kind == ArchiveKind::Complete) else { return Ok(None) };
    if pos + 1 == archives.len() {
        return Ok(previous.map(|m| m.format));
    }
    archives.get(pos).map(|a| repo.manifest(a).map(|m| m.format)).transpose()
}

/// A file to store, as found by the scan.
struct Pending {
    /// Index into the scan's entries.
    idx: usize,
    path: PathBuf,
    change: Change,
    job: Job,
}

/// What storing one file takes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Job {
    /// A whole copy.
    Copy,
    /// A whole copy and its chunk list.
    CopyAndList,
    /// The whole copy is already there (interrupted run) and its hash is
    /// `hash`: only the chunk list is computed again.
    ListOnly { hash: String },
    /// Only the chunks the destination lacks, and the list.
    Chunks,
}

/// How one file was stored.
struct Stored {
    size: u64,
    /// BLAKE3 of the content, hex.
    hash: String,
    chunks: Option<Chunks>,
    /// Bytes written to the destination (new chunks, or the whole copy).
    written: u64,
    /// The archive holds a whole copy.
    whole: bool,
    /// Record the whole copy in the resume journal.
    journal: bool,
    /// Kept from the interrupted run.
    resumed: bool,
}

/// Stores files into the archive being built.
struct Copier<'a> {
    writer: Writer,
    /// Set when some file is stored as chunks only.
    store: Option<NewChunks<'a>>,
    form: &'a Form,
    params: ChunkParams,
}

impl Copier<'_> {
    /// Stores `entry`'s content, read from `path`, as `job` says.
    fn copy_file(
        &mut self,
        job: &Job,
        path: &Path,
        entry: &Entry,
        on_bytes: &mut dyn FnMut(u64),
    ) -> Result<Stored> {
        let ctx = path.display().to_string();
        let mut file = File::open(path).map_err(EngineError::io(ctx.clone()))?;
        match job {
            Job::Copy => {
                let (size, hash) =
                    self.writer.add_stream(&entry.key, &mut file, entry.size, entry.mtime_ns, on_bytes)?;
                Ok(Stored {
                    size,
                    hash,
                    chunks: None,
                    written: size,
                    whole: true,
                    journal: true,
                    resumed: false,
                })
            }
            Job::CopyAndList => {
                let mut lister = ListingReader::new(&mut file, self.form, self.params);
                let (size, hash) =
                    self.writer.add_stream(&entry.key, &mut lister, entry.size, entry.mtime_ns, on_bytes)?;
                let list = lister.finish().map_err(EngineError::io(ctx))?;
                let chunks = Some(Chunks { whole: true, list });
                Ok(Stored { size, hash, chunks, written: size, whole: true, journal: true, resumed: false })
            }
            Job::ListOnly { hash: journalled } => {
                let mut lister = ListingReader::new(&mut file, self.form, self.params);
                let (size, hash) =
                    hash_stream(&mut lister, on_bytes).map_err(EngineError::io(ctx.clone()))?;
                let list = lister.finish().map_err(EngineError::io(ctx))?;
                if size != entry.size || hash != *journalled {
                    // Changed in place without a new date: copy it again.
                    return self.copy_file(&Job::CopyAndList, path, entry, on_bytes);
                }
                let chunks = Some(Chunks { whole: true, list });
                Ok(Stored { size, hash, chunks, written: 0, whole: true, journal: false, resumed: true })
            }
            Job::Chunks => {
                let store = self.store.as_mut().ok_or_else(|| EngineError::Io {
                    context: ctx.clone(),
                    source: std::io::Error::other("no chunk store"),
                })?;
                let deflate = self.writer.deflates(&entry.key);
                let (size, hash, list, written) =
                    store_chunks(&mut file, store, self.params, deflate, on_bytes, &ctx)?;
                let chunks = Some(Chunks { whole: false, list });
                Ok(Stored { size, hash, chunks, written, whole: false, journal: false, resumed: false })
            }
        }
    }

    /// Bytes `job` is expected to write for `entry`, which had `before`
    /// bytes in the previous backup (`None`: a new file).
    fn estimate(&self, job: &Job, entry: &Entry, before: Option<u64>) -> u64 {
        let deflated = self.writer.deflates(&entry.key);
        match job {
            Job::Copy | Job::CopyAndList => space::estimate(entry.size, deflated),
            Job::ListOnly { .. } => 0,
            Job::Chunks => space::estimate(chunk_estimate(entry.size, before, self.params.max()), deflated),
        }
    }
}

/// Expected bytes of new chunks for a file of `size` bytes that had
/// `before` bytes (`None`: a new file, all new). An edited file counts its
/// growth and one chunk for the edit; a file rewritten throughout counts
/// less than it writes, and the full-disk check during the copy stops the
/// run in time, keeping what was written for the next one.
fn chunk_estimate(size: u64, before: Option<u64>, max_chunk: usize) -> u64 {
    match before {
        None => size,
        Some(b) => size.saturating_sub(b).saturating_add(max_chunk as u64).min(size),
    }
}

/// Reads `src` to its end as chunks; the ones the destination lacks are
/// written into the archive being built. Returns the size, the BLAKE3 hash
/// of the content, the chunk list and the bytes written.
fn store_chunks(
    src: &mut dyn Read,
    store: &mut NewChunks<'_>,
    params: ChunkParams,
    deflate: bool,
    on_bytes: &mut dyn FnMut(u64),
    ctx: &str,
) -> Result<(u64, String, Vec<ChunkRef>, u64)> {
    let mut hasher = blake3::Hasher::new();
    let mut chunker = Chunker::new(params);
    let mut list = Vec::new();
    let mut written = 0u64;
    let mut total = 0u64;
    let mut emit = |data: &[u8]| -> Result<()> {
        let chunk = chunk_ref(store.form, data).map_err(EngineError::io(ctx.to_owned()))?;
        if store.put(&chunk.id, data, deflate)? {
            written = written.saturating_add(u64::from(chunk.size));
        }
        list.push(chunk);
        Ok(())
    };
    let mut buf = vec![0u8; READ_BUFFER];
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(EngineError::io(ctx.to_owned())(e)),
        };
        let data = buf.get(..n).unwrap_or_default();
        hasher.update(data);
        total = total.saturating_add(n as u64);
        on_bytes(n as u64);
        chunker.push(data, &mut emit)?;
    }
    chunker.finish(&mut emit)?;
    Ok((total, hasher.finalize().to_hex().to_string(), list, written))
}

fn chunk_ref(form: &Form, data: &[u8]) -> std::io::Result<ChunkRef> {
    let size = u32::try_from(data.len()).map_err(std::io::Error::other)?;
    Ok(ChunkRef { id: form.id_of(data), size })
}

/// Passes reads through, listing the chunks of what was read: for a whole
/// copy whose chunk list is kept beside it.
struct ListingReader<'a> {
    src: &'a mut dyn Read,
    form: &'a Form,
    chunker: Chunker,
    list: Vec<ChunkRef>,
}

impl<'a> ListingReader<'a> {
    fn new(src: &'a mut dyn Read, form: &'a Form, params: ChunkParams) -> Self {
        Self { src, form, chunker: Chunker::new(params), list: Vec::new() }
    }

    /// The chunks of everything read.
    fn finish(self) -> std::io::Result<Vec<ChunkRef>> {
        let Self { form, chunker, mut list, .. } = self;
        chunker.finish(&mut |d: &[u8]| -> std::io::Result<()> {
            list.push(chunk_ref(form, d)?);
            Ok(())
        })?;
        Ok(list)
    }
}

impl Read for ListingReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.src.read(buf)?;
        let (form, list) = (self.form, &mut self.list);
        self.chunker.push(buf.get(..n).unwrap_or_default(), &mut |d: &[u8]| -> std::io::Result<()> {
            list.push(chunk_ref(form, d)?);
            Ok(())
        })?;
        Ok(n)
    }
}

/// The chunks of the archive being built, and those the destination
/// already holds: a chunk is written only when found nowhere.
struct NewChunks<'a> {
    form: &'a Form,
    staging: &'a Path,
    /// Readable in a committed archive (chunk file or whole copy).
    committed: HashSet<ChunkId>,
    /// Chunk files of the archive being built, known to be whole.
    staged: HashSet<ChunkId>,
    /// Chunk files left by an interrupted run, not checked yet.
    unchecked: HashSet<ChunkId>,
}

impl<'a> NewChunks<'a> {
    /// Indexes the chunks of every archive (`previous` is the newest one's
    /// manifest, already read) and of the unfinished archive in `staging`.
    fn open(
        repo: &Repo,
        archives: &[ArchiveName],
        previous: Option<&Manifest>,
        staging: &'a Path,
        form: &'a Form,
    ) -> Result<Self> {
        let mut committed = HashSet::new();
        for (i, name) in archives.iter().enumerate() {
            let dir = repo.dir(name);
            committed.extend(form.list(&dir)?);
            if i + 1 == archives.len()
                && let Some(m) = previous
            {
                committed.extend(chunks::in_whole_copies(m));
            } else {
                committed.extend(chunks::in_whole_copies(&repo.manifest(name)?));
            }
        }
        let unchecked: HashSet<ChunkId> = form.list(staging)?.into_iter().collect();
        if !unchecked.is_empty() {
            tracing::info!(chunks = unchecked.len(), "chunks of the interrupted run found");
        }
        Ok(Self { form, staging, committed, staged: HashSet::new(), unchecked })
    }

    /// Makes sure the chunk `id`, whose content is `data`, can be read from
    /// the destination once the archive is committed. Returns true when it
    /// had to be written.
    fn put(&mut self, id: &ChunkId, data: &[u8], deflate: bool) -> Result<bool> {
        if self.unchecked.remove(id) {
            // Left by the interrupted run: reused once read back whole.
            let size = u32::try_from(data.len()).ok();
            if self.form.read(self.staging, id, size).is_ok() {
                self.staged.insert(*id);
                return Ok(false);
            }
            let path = self.form.path(self.staging, id);
            tracing::info!(chunk = %id, "damaged chunk of the interrupted run written again");
            let _ = std::fs::remove_file(path);
        }
        if self.staged.contains(id) || self.committed.contains(id) {
            return Ok(false);
        }
        self.form.write(self.staging, id, data, deflate)?;
        self.staged.insert(*id);
        Ok(true)
    }
}

/// True when an unfinished backup of `partial` kind can be resumed by a
/// backup that would otherwise be of `wanted` kind.
fn can_resume(partial: ArchiveKind, wanted: ArchiveKind, mode: BackupMode, first: bool) -> bool {
    if first {
        // The first archive must be complete.
        partial == ArchiveKind::Complete
    } else {
        // An explicit choice wins; a scheduled backup takes what was begun.
        mode == BackupMode::Auto || partial == wanted
    }
}

/// How a file compares with the previous backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    New,
    Changed,
    /// Copied anyway by a complete backup.
    Unchanged,
}

impl Change {
    fn action(self) -> FileAction {
        match self {
            Self::New => FileAction::New,
            Self::Changed => FileAction::Changed,
            Self::Unchanged => FileAction::Unchanged,
        }
    }
}

/// Result of walking the sources.
struct Scan<'a> {
    previous: HashMap<&'a str, &'a Entry>,
    excludes: &'a Excluder,
    /// Copy unchanged files too (complete backup).
    copy_all: bool,
    entries: Vec<Entry>,
    /// Files encrypted by Windows (EFS) found.
    efs: u64,
    /// Entries whose content must be copied: index into `entries`, path.
    to_copy: Vec<(usize, PathBuf, Change)>,
    unchanged: u64,
}

impl<'a> Scan<'a> {
    fn new(previous: Option<&'a Manifest>, excludes: &'a Excluder, copy_all: bool) -> Self {
        let previous =
            previous.map(|m| m.entries.iter().map(|e| (e.key.as_str(), e)).collect()).unwrap_or_default();
        Self { previous, excludes, copy_all, entries: Vec::new(), efs: 0, to_copy: Vec::new(), unchanged: 0 }
    }

    /// Walks `source`; `scanning` counts the files found and shows the
    /// folder being read.
    fn walk(
        &mut self,
        source: &Path,
        sink: &dyn EventSink,
        scanning: &mut Progress<'_>,
        report: &mut BackupReport,
    ) {
        if !source.exists() {
            skip(sink, report, source, "folder not found".to_owned());
            return;
        }
        let excludes = self.excludes;
        let walker = walkdir::WalkDir::new(source)
            .follow_links(false)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| !excludes.excludes(e.path(), e.file_type().is_dir()));
        for item in walker {
            let item = match item {
                Ok(i) => i,
                Err(e) => {
                    let path = e.path().map(Path::to_path_buf).unwrap_or_else(|| source.to_path_buf());
                    skip(sink, report, &path, e.to_string());
                    continue;
                }
            };
            let ft = item.file_type();
            if ft.is_file() {
                let folder = item.path().parent().unwrap_or(item.path());
                scanning.start_file(folder.display().to_string());
            }
            if ft.is_symlink() {
                // Junctions and links are not followed: what they point to
                // is backed up only when it is among the sources. Windows'
                // own hidden ones (Documents\My Music…) go unsaid.
                let attributes = item.metadata().map(|m| attributes(&m)).unwrap_or(0);
                if source_kind(attributes) != SourceKind::WindowsLink {
                    skip(sink, report, item.path(), LINK_SKIPPED.to_owned());
                }
                continue;
            }
            let key = match keys::key_for(item.path()) {
                Ok(k) => k,
                Err(e) => {
                    skip(sink, report, item.path(), e.to_string());
                    continue;
                }
            };
            if ft.is_dir() {
                scanning.folder();
                self.entries.push(Entry {
                    key,
                    kind: EntryKind::Dir,
                    size: 0,
                    mtime_ns: 0,
                    hash: None,
                    stored: false,
                    blob: None,
                    chunks: None,
                });
                continue;
            }
            let meta = match item.metadata() {
                Ok(m) => m,
                Err(e) => {
                    skip(sink, report, item.path(), e.to_string());
                    continue;
                }
            };
            match source_kind(attributes(&meta)) {
                SourceKind::OnlineOnly => {
                    // Reading it would download it: its last saved version
                    // stays, and the reason is in the log.
                    skip(sink, report, item.path(), ONLINE_ONLY.to_owned());
                    if let Some(old) = self.previous.get(key.as_str()) {
                        let mut kept = (*old).clone();
                        kept.stored = false;
                        kept.blob = None;
                        kept.chunks = None;
                        self.entries.push(kept);
                    }
                    continue;
                }
                SourceKind::Encrypted => self.efs += 1,
                SourceKind::Plain | SourceKind::WindowsLink => {}
            }
            let mut entry = Entry {
                key,
                kind: EntryKind::File,
                size: meta.len(),
                mtime_ns: meta.modified().map(mtime_ns).unwrap_or(0),
                hash: None,
                stored: false,
                blob: None,
                chunks: None,
            };
            let idx = self.entries.len();
            match self.previous.get(entry.key.as_str()) {
                Some(old) if entry.unchanged_from(old) => {
                    entry.hash.clone_from(&old.hash);
                    self.unchanged += 1;
                    scanning.count(FileAction::Unchanged);
                    if self.copy_all {
                        self.to_copy.push((idx, item.path().to_path_buf(), Change::Unchanged));
                    }
                }
                Some(_) => {
                    report.files_changed += 1;
                    scanning.count(FileAction::Changed);
                    self.to_copy.push((idx, item.path().to_path_buf(), Change::Changed));
                }
                None => {
                    report.files_new += 1;
                    scanning.count(FileAction::New);
                    self.to_copy.push((idx, item.path().to_path_buf(), Change::New));
                }
            }
            self.entries.push(entry);
        }
    }
}

const LINK_SKIPPED: &str = "a junction or symbolic link: not followed, so what it points to is backed up only if it is among the sources";
const ONLINE_ONLY: &str = "an online-only file (OneDrive or a similar service): reading it would download it; \
     its last saved version is kept. Make it always available on this device to back it up";

/// What the attributes of a file found in a source say about backing it up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Plain,
    /// Encrypted by Windows (EFS): read in the clear.
    Encrypted,
    /// Not on this PC: a cloud placeholder or an offline file.
    OnlineOnly,
    /// A hidden system link Windows keeps for old programs.
    WindowsLink,
}

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
const FILE_ATTRIBUTE_ENCRYPTED: u32 = 0x4000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x4_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

fn source_kind(attributes: u32) -> SourceKind {
    let has = |flag: u32| attributes & flag != 0;
    if has(FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        || has(FILE_ATTRIBUTE_RECALL_ON_OPEN)
        || has(FILE_ATTRIBUTE_OFFLINE)
    {
        SourceKind::OnlineOnly
    } else if has(FILE_ATTRIBUTE_HIDDEN) && has(FILE_ATTRIBUTE_SYSTEM) {
        SourceKind::WindowsLink
    } else if has(FILE_ATTRIBUTE_ENCRYPTED) {
        SourceKind::Encrypted
    } else {
        SourceKind::Plain
    }
}

#[cfg(windows)]
fn attributes(meta: &std::fs::Metadata) -> u32 {
    std::os::windows::fs::MetadataExt::file_attributes(meta)
}

#[cfg(not(windows))]
fn attributes(_meta: &std::fs::Metadata) -> u32 {
    0
}

fn skip(sink: &dyn EventSink, report: &mut BackupReport, path: &Path, reason: String) {
    let s = Skipped { path: path.display().to_string(), reason };
    tracing::warn!(path = %s.path, reason = %s.reason, "skipped while scanning");
    sink.emit(RunEvent::Skipped(s.clone()));
    report.skipped.push(s);
}

/// Sources without the ones nested inside another source.
fn top_level_sources(sources: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for s in sources {
        if !sources.iter().any(|other| other != s && s.starts_with(other)) && !out.contains(s) {
            out.push(s.clone());
        }
    }
    out
}

/// The profile's exclusion rules, ready to test paths.
struct Excluder {
    rules: PathRules,
    globs: GlobSet,
}

impl Excluder {
    fn new(rules: &[Exclusion]) -> Result<Self> {
        let mut b = GlobSetBuilder::new();
        for rule in rules.iter().filter(|r| !r.is_blank()) {
            if let Exclusion::Pattern { glob } = rule {
                let g = GlobBuilder::new(&glob.trim().replace('\\', "/"))
                    .case_insensitive(true)
                    .literal_separator(false)
                    .build()
                    .map_err(|e| EngineError::InvalidProfile(format!("exclusion {glob:?}: {e}")))?;
                b.add(g);
            }
        }
        let globs = b.build().map_err(|e| EngineError::InvalidProfile(e.to_string()))?;
        Ok(Self { rules: PathRules::new(rules), globs })
    }

    fn excludes(&self, path: &Path, is_dir: bool) -> bool {
        self.rules.excludes(path, is_dir)
            || (!self.globs.is_empty() && self.globs.is_match(path.to_string_lossy().replace('\\', "/")))
    }
}

/// Name for a new archive now, one second later if taken.
fn unique_name(existing: &[ArchiveName], kind: ArchiveKind) -> ArchiveName {
    let now = jiff::Zoned::now().datetime();
    let mut time = now.with().subsec_nanosecond(0).build().unwrap_or(now);
    if let Some(latest) = existing.last().map(|a| a.time)
        && latest >= time
    {
        time = latest.checked_add(jiff::Span::new().seconds(1)).unwrap_or(latest);
    }
    ArchiveName { time, kind }
}

/// Bytes from which a file does not fit on FAT32.
const FAT_MAX_FILE: u64 = 4 * 1024 * 1024 * 1024;
const FAT_TOO_LARGE: &str =
    "the destination drive uses FAT32, which cannot hold files of 4 GiB or more: format it as exFAT or NTFS";

fn reason(e: &EngineError) -> String {
    match e {
        EngineError::Io { source, .. }
            if source.raw_os_error() == Some(32) || source.raw_os_error() == Some(33) =>
        {
            "in use by another program".to_owned()
        }
        EngineError::Io { source, .. } => source.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_tell_online_only_encrypted_and_windows_own_links() {
        assert_eq!(source_kind(0x20), SourceKind::Plain);
        // OneDrive "online only", "always keep" not set.
        assert_eq!(source_kind(0x40_0000 | 0x20), SourceKind::OnlineOnly);
        assert_eq!(source_kind(0x4_0000), SourceKind::OnlineOnly);
        assert_eq!(source_kind(0x1000), SourceKind::OnlineOnly);
        assert_eq!(source_kind(0x4000 | 0x20), SourceKind::Encrypted);
        // Documents\My Music and the like: hidden, system, reparse point.
        assert_eq!(source_kind(0x2 | 0x4 | 0x400 | 0x10), SourceKind::WindowsLink);
        assert_eq!(source_kind(0x2), SourceKind::Plain, "hidden alone");
    }

    #[test]
    fn nested_sources_are_backed_up_once() {
        let s = vec![PathBuf::from("/a"), PathBuf::from("/a/b"), PathBuf::from("/c"), PathBuf::from("/c")];
        assert_eq!(top_level_sources(&s), vec![PathBuf::from("/a"), PathBuf::from("/c")]);
    }

    fn pattern(glob: &str) -> Exclusion {
        Exclusion::Pattern { glob: glob.into() }
    }

    #[test]
    fn patterns_match_case_insensitively_with_either_slash() {
        let ex = Excluder::new(&[pattern("*.TMP"), pattern(r"**\node_modules"), pattern("")]).unwrap();
        assert!(ex.excludes(Path::new("/x/y/file.tmp"), false));
        assert!(ex.excludes(Path::new("/x/node_modules"), true));
        assert!(!ex.excludes(Path::new("/x/notes.txt"), false));
    }

    #[test]
    fn folder_file_and_type_rules_apply_too() {
        let ex = Excluder::new(&[
            Exclusion::Folder { path: "/x/cache".into() },
            Exclusion::File { path: "/x/big.iso".into() },
            Exclusion::extension("bak").unwrap(),
        ])
        .unwrap();
        assert!(ex.excludes(Path::new("/x/cache"), true));
        assert!(ex.excludes(Path::new("/x/big.iso"), false));
        assert!(ex.excludes(Path::new("/x/old.BAK"), false));
        assert!(!ex.excludes(Path::new("/x/keep.txt"), false));
    }

    #[test]
    fn invalid_pattern_is_reported() {
        assert!(Excluder::new(&[pattern("a[")]).is_err());
    }

    #[test]
    fn an_unfinished_backup_is_resumed_unless_the_user_chose_otherwise() {
        use ArchiveKind::{Complete as C, Incremental as I};
        assert!(can_resume(C, C, BackupMode::Auto, true));
        assert!(!can_resume(I, C, BackupMode::Auto, true));
        assert!(can_resume(C, I, BackupMode::Auto, false));
        assert!(can_resume(I, C, BackupMode::Auto, false));
        assert!(can_resume(I, I, BackupMode::Incremental, false));
        assert!(!can_resume(C, I, BackupMode::Incremental, false));
        assert!(!can_resume(I, C, BackupMode::Complete, false));
    }

    #[test]
    fn the_space_estimate_counts_growth_and_one_chunk() {
        assert_eq!(chunk_estimate(1000, None, 100), 1000, "a new file is all new");
        assert_eq!(chunk_estimate(1000, Some(1000), 100), 100, "an edit: one chunk");
        assert_eq!(chunk_estimate(1500, Some(1000), 100), 600, "growth and one chunk");
        assert_eq!(chunk_estimate(50, Some(50), 100), 50, "never more than the file");
        assert_eq!(chunk_estimate(500, Some(1000), 100), 100, "shrunk");
    }

    #[test]
    fn new_name_is_later_than_existing() {
        let far: ArchiveName = "2999-01-01_000000_C".parse().unwrap();
        let n = unique_name(&[far], ArchiveKind::Incremental);
        assert!(n.time > far.time);
    }
}
