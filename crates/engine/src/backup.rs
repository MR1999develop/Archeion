//! Backup: scan, compare with the previous manifest, copy what changed (or
//! everything, for a complete backup) into a new archive.

use crate::{
    EngineError, EventSink, Result,
    container::{Writer, mtime_ns},
    crypto::Password,
    progress::Progress,
    repo::{Purpose, Repo},
    resume::{Done, Journal},
    space,
};
use archeion_core::exclusion::PathRules;
use archeion_core::{
    ArchiveKind, ArchiveName, BackupMode, BackupReport, Entry, EntryKind, Exclusion, Manifest, Phase,
    Profile, RunEvent, Skipped, Summary, backup_kind::next_kind, keys,
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
};

/// Backs up `profile` into a new archive. The first backup is complete;
/// later ones are incremental or complete as `mode` (and, for
/// [`BackupMode::Auto`], the profile's full-backup rule) says. A complete
/// backup copies every file and starts a new chain.
///
/// Files that cannot be read (in use, access denied) are reported as
/// [`RunEvent::Skipped`]; their previous version stays restorable.
///
/// A backup that stopped before committing (Cancel, full disk, unplugged
/// drive, power cut) is resumed by the next one: files already copied and
/// unchanged since are kept (see [`crate::resume`]).
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
    run(profile, password, mode, sink, None)
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
    run(profile, password, mode, sink, Some(files))
}

fn run(
    profile: &Profile,
    password: Option<&Password>,
    mode: BackupMode,
    sink: &dyn EventSink,
    stop_after: Option<u64>,
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
    let wanted = next_kind(&archives, profile.full_backup, mode, jiff::Zoned::now().datetime());
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
    let mut writer = Writer::create(&staging, container, repo.keys(), &name)?;
    let mut journal = Journal::open(&staging, repo.keys())?;

    // Files the interrupted run already copied, unchanged since.
    let mut pending = Vec::with_capacity(scan.to_copy.len());
    for (idx, path, change) in std::mem::take(&mut scan.to_copy) {
        let Some(entry) = scan.entries.get_mut(idx) else { continue };
        let id = writer.journal_id(&entry.key);
        match journal.get(&id) {
            Some(done)
                if done.size == entry.size
                    && done.mtime_ns == entry.mtime_ns
                    && writer.looks_complete(&entry.key, done.size) =>
            {
                entry.hash = Some(done.hash.clone());
                entry.stored = true;
                entry.blob = writer.stored_as(&entry.key);
                report.resumed_files += 1;
                report.bytes_copied = report.bytes_copied.saturating_add(entry.size);
            }
            _ => pending.push((idx, path, change)),
        }
    }

    if resumed.is_some() {
        tracing::info!(journalled = journal.len(), kept = report.resumed_files, "unfinished backup reopened");
    }

    // Room for what is left to copy.
    let mut to_write: u64 =
        pending.iter().map(|(i, _, _)| stored_estimate(&writer, scan.entries.get(*i))).sum();
    space::ensure(repo.root(), to_write)?;

    sink.emit(RunEvent::Phase { phase: Phase::Copying });
    let total: u64 = pending.iter().filter_map(|(i, _, _)| scan.entries.get(*i)).map(|e| e.size).sum();
    let mut progress = Progress::new(sink, total, pending.len() as u64);
    let mut drop_entries = HashSet::new();
    let mut copied_now = 0u64;
    for (idx, path, change) in pending {
        if stop_after.is_some_and(|n| copied_now >= n) {
            return Err(EngineError::Io {
                context: "backup".into(),
                source: std::io::Error::other("stopped for a test"),
            });
        }
        let estimate = stored_estimate(&writer, scan.entries.get(idx));
        let Some(entry) = scan.entries.get_mut(idx) else { continue };
        progress.start_file(path.display().to_string());
        let copied =
            File::open(&path).map_err(EngineError::io(path.display().to_string())).and_then(|mut f| {
                writer.add_stream(&entry.key, &mut f, entry.size, entry.mtime_ns, &mut |n| progress.add(n))
            });
        match copied {
            Ok((size, hash)) => {
                // A file that changed while being read is copied again next time.
                if size == entry.size {
                    let done = Done { size, mtime_ns: entry.mtime_ns, hash: hash.clone() };
                    journal.record(&writer.journal_id(&entry.key), done).map_err(|e| {
                        if space::is_disk_full(&e) { space::full(repo.root(), to_write) } else { e }
                    })?;
                }
                entry.size = size;
                entry.hash = Some(hash);
                entry.stored = true;
                entry.blob = writer.stored_as(&entry.key);
                report.bytes_copied = report.bytes_copied.saturating_add(size);
                copied_now += 1;
            }
            Err(e) if space::is_disk_full(&e) => {
                // Stop: skipping file after file would fill the log, not
                // the backup. What is copied stays for the next run.
                tracing::warn!(archive = %name, "destination full");
                return Err(space::full(repo.root(), to_write));
            }
            Err(e) => {
                let skipped = Skipped { path: path.display().to_string(), reason: reason(&e) };
                tracing::warn!(path = %skipped.path, reason = %skipped.reason, "file skipped");
                sink.emit(RunEvent::Skipped(skipped.clone()));
                report.skipped.push(skipped);
                // Keep the last good version, if there is one.
                match previous.as_ref().and_then(|m| m.get(&entry.key)) {
                    Some(old) => {
                        *entry = old.clone();
                        entry.stored = false;
                        entry.blob = None;
                    }
                    None => {
                        drop_entries.insert(idx);
                    }
                }
                match change {
                    Change::New => report.files_new = report.files_new.saturating_sub(1),
                    Change::Changed => report.files_changed = report.files_changed.saturating_sub(1),
                    Change::Unchanged => {}
                }
            }
        }
        to_write = to_write.saturating_sub(estimate);
    }
    progress.finish();

    let entries: Vec<Entry> = scan
        .entries
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop_entries.contains(i))
        .map(|(_, e)| e)
        .collect();
    let mut manifest = Manifest { kind, entries };
    manifest.normalize();
    report.files_unchanged = scan.unchanged;
    if let Some(prev) = &previous {
        let now: HashSet<&str> = manifest.entries.iter().map(|e| e.key.as_str()).collect();
        report.files_deleted = prev
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::File && !now.contains(e.key.as_str()))
            .count() as u64;
    }

    if resumed.is_some() {
        let keep: HashSet<PathBuf> =
            manifest.entries.iter().filter(|e| e.stored).map(|e| writer.content_path(&e.key)).collect();
        let removed = writer.remove_others(&keep)?;
        if removed > 0 {
            tracing::info!(removed, "leftovers of the interrupted run removed");
        }
    }
    writer.finish()?;
    repo.write_manifest(&staging, &manifest)?;
    journal.remove()?;
    repo.commit(&staging, &name)?;
    report.archive = name.to_string();
    tracing::info!(archive = %name, copied = report.bytes_copied, resumed = report.resumed_files, "archive committed");
    sink.emit(RunEvent::Finished { summary: Summary::Backup(report.clone()) });
    Ok(report)
}

/// Expected stored size of `entry`'s content.
fn stored_estimate(writer: &Writer, entry: Option<&Entry>) -> u64 {
    entry.map_or(0, |e| space::estimate(e.size, writer.deflates(&e.key)))
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

/// Result of walking the sources.
struct Scan<'a> {
    previous: HashMap<&'a str, &'a Entry>,
    excludes: &'a Excluder,
    /// Copy unchanged files too (complete backup).
    copy_all: bool,
    entries: Vec<Entry>,
    /// Entries whose content must be copied: index into `entries`, path.
    to_copy: Vec<(usize, PathBuf, Change)>,
    unchanged: u64,
}

impl<'a> Scan<'a> {
    fn new(previous: Option<&'a Manifest>, excludes: &'a Excluder, copy_all: bool) -> Self {
        let previous =
            previous.map(|m| m.entries.iter().map(|e| (e.key.as_str(), e)).collect()).unwrap_or_default();
        Self { previous, excludes, copy_all, entries: Vec::new(), to_copy: Vec::new(), unchanged: 0 }
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
                continue; // junctions and links: their targets are backed up on their own
            }
            let key = match keys::key_for(item.path()) {
                Ok(k) => k,
                Err(e) => {
                    skip(sink, report, item.path(), e.to_string());
                    continue;
                }
            };
            if ft.is_dir() {
                self.entries.push(Entry {
                    key,
                    kind: EntryKind::Dir,
                    size: 0,
                    mtime_ns: 0,
                    hash: None,
                    stored: false,
                    blob: None,
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
            let mut entry = Entry {
                key,
                kind: EntryKind::File,
                size: meta.len(),
                mtime_ns: meta.modified().map(mtime_ns).unwrap_or(0),
                hash: None,
                stored: false,
                blob: None,
            };
            let idx = self.entries.len();
            match self.previous.get(entry.key.as_str()) {
                Some(old) if entry.unchanged_from(old) => {
                    entry.hash.clone_from(&old.hash);
                    self.unchanged += 1;
                    if self.copy_all {
                        self.to_copy.push((idx, item.path().to_path_buf(), Change::Unchanged));
                    }
                }
                Some(_) => {
                    report.files_changed += 1;
                    self.to_copy.push((idx, item.path().to_path_buf(), Change::Changed));
                }
                None => {
                    report.files_new += 1;
                    self.to_copy.push((idx, item.path().to_path_buf(), Change::New));
                }
            }
            self.entries.push(entry);
        }
    }
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
    fn new_name_is_later_than_existing() {
        let far: ArchiveName = "2999-01-01_000000_C".parse().unwrap();
        let n = unique_name(&[far], ArchiveKind::Incremental);
        assert!(n.time > far.time);
    }
}
