//! Merge: archives fused into one that keeps the newest content.
//!
//! The scheduled merge follows the profile's rules and never touches the
//! initial complete archive; a manual merge fuses the archives the user
//! chose, including the complete one if asked. Deleting an archive that
//! later ones depend on is a merge with the next one.
//!
//! Chunks (see [`crate::chunks`]): the merged archive keeps a member's
//! chunk file only when a manifest that remains lists the chunk and no
//! archive outside the group, nor a whole copy in the merged archive, can
//! provide it; a chunk needed that way and found only inside a member's
//! whole copy is cut out of it into a chunk file. The rest is dropped with
//! the members. A merged *complete* archive holds every file whole, so a
//! file kept as chunks only is put back together into a whole copy.
//!
//! Every file carried into a merged archive is listed as moved, and every
//! version the merge or deletion gets rid of as dropped
//! ([`archeion_core::RunEvent::Files`]).

use crate::{
    EngineError, EventSink, Result,
    chunks::{self, CHUNKS_DIR, ChunkIndex, ChunkReader, Form},
    container::{Reader, Writer, link_or_copy},
    crypto::Password,
    progress::Progress,
    repo::{Purpose, Repo, dir_size},
};
use archeion_core::{
    ArchiveKind, ArchiveName, ChunkId, ChunkRef, Chunks, Entry, EntryKind, FileAction, FileDone, Manifest,
    MergeGroup, MergeReport, Phase, Profile, RunEvent, Summary, keys,
    merge_plan::{Deletion, automatic_keep_set, deletion, merge_groups, range_group},
};
use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
    path::PathBuf,
};

/// Applies the profile's merge rules to its archives.
///
/// Each group of consecutive archives that the rules do not keep is merged
/// with the kept archive that follows it; the result carries that archive's
/// date and holds the newest content of the group. Older versions of the
/// files are dropped, which frees space; files deleted at the source inside
/// the group are kept with their last content when the profile says so
/// ([`archeion_core::MergePolicy::keep_deleted`]), dropped otherwise. The
/// initial complete archive and the backups of the recent days are always
/// kept.
///
/// # Errors
/// See [`EngineError`].
pub fn merge(profile: &Profile, password: Option<&Password>, sink: &dyn EventSink) -> Result<MergeReport> {
    run(profile, password, sink, profile.merge.keep_deleted, |archives| {
        Ok(merge_groups(&automatic_keep_set(archives, &profile.merge.rules, local_now()))
            .into_iter()
            .map(Step::Merge)
            .collect())
    })
}

/// Merges the archives from `first` to `last` (names as listed, either
/// order) and every archive between them into one, dated like the newest.
/// Including the initial complete archive produces a new complete archive.
/// Files deleted at the source inside the range are kept or dropped as
/// [`archeion_core::MergePolicy::keep_deleted`] says.
///
/// # Errors
/// [`EngineError::InvalidProfile`] when fewer than two archives are chosen
/// or a name does not exist; otherwise see [`EngineError`].
pub fn merge_range(
    profile: &Profile,
    password: Option<&Password>,
    first: &str,
    last: &str,
    sink: &dyn EventSink,
) -> Result<MergeReport> {
    let (a, b) = (parse_name(first)?, parse_name(last)?);
    run(profile, password, sink, profile.merge.keep_deleted, |archives| {
        range_group(archives, &a, &b)
            .map(|g| vec![Step::Merge(g)])
            .map_err(|e| EngineError::InvalidProfile(e.to_string()))
    })
}

/// Deletes one archive. The newest one (or the only one) is removed; any
/// other incremental archive is merged with the next one, so later backups
/// keep the content they need and only the versions found in the deleted
/// archive alone are lost. Complete archives cannot be deleted while others
/// exist (see [`archeion_core::merge_plan::deletion`]).
///
/// # Errors
/// [`EngineError::InvalidProfile`] when the archive cannot be deleted or
/// does not exist; otherwise see [`EngineError`].
pub fn delete_archive(
    profile: &Profile,
    password: Option<&Password>,
    archive: &str,
    sink: &dyn EventSink,
) -> Result<MergeReport> {
    let name = parse_name(archive)?;
    // The deleted backup's files go with it.
    run(profile, password, sink, false, |archives| {
        match deletion(archives, &name).map_err(|e| EngineError::InvalidProfile(e.to_string()))? {
            Deletion::Remove(i) => Ok(vec![Step::Remove(i)]),
            Deletion::MergeWithNext(g) => Ok(vec![Step::Merge(g)]),
        }
    })
}

/// True when the profile's merge rules would merge something now. Reads
/// only the archive names: no lock, no password, cheap enough to ask
/// often.
///
/// # Errors
/// [`EngineError::DestinationUnavailable`] when the drive or share is not
/// reachable.
pub fn merge_pending(profile: &Profile) -> Result<bool> {
    let root = crate::repo::destination_of(profile)?;
    crate::repo::ensure_reachable(root)?;
    if root.as_os_str().is_empty() || !root.exists() {
        return Ok(false);
    }
    let archives = crate::repo::archives_in(root)?;
    Ok(!merge_groups(&automatic_keep_set(&archives, &profile.merge.rules, local_now())).is_empty())
}

/// Local wall-clock time, the clock archive names are written in.
fn local_now() -> jiff::civil::DateTime {
    jiff::Zoned::now().datetime()
}

fn parse_name(s: &str) -> Result<ArchiveName> {
    s.parse::<ArchiveName>().map_err(|_| EngineError::InvalidProfile(format!("{s} is not a backup name")))
}

/// One change to the archives.
enum Step {
    /// Fuse these archives into one.
    Merge(RangeInclusive<usize>),
    /// Delete this archive, which nothing depends on.
    Remove(usize),
}

/// Merges or removes archives as `plan` says. `keep_deleted`: a merged
/// archive keeps the files deleted inside its group.
fn run(
    profile: &Profile,
    password: Option<&Password>,
    sink: &dyn EventSink,
    keep_deleted: bool,
    plan: impl FnOnce(&[ArchiveName]) -> Result<Vec<Step>>,
) -> Result<MergeReport> {
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let Some(repo) = Repo::open(profile, false, password)? else {
        let report = MergeReport::default();
        sink.emit(RunEvent::Finished { summary: Summary::Merge(report.clone()) });
        return Ok(report);
    };
    let archives = repo.archives()?;
    let steps = plan(&archives)?;
    let mut report = MergeReport {
        archives_before: archives.len() as u64,
        changed_outside: repo.changed_outside(),
        ..MergeReport::default()
    };
    let size_before = dir_size(repo.root());

    sink.emit(RunEvent::Phase { phase: Phase::Merging });
    let mut plans = Vec::new();
    let mut removals = Vec::new();
    for step in steps {
        match step {
            Step::Merge(g) => {
                let oldest = *g.start() == 0;
                plans.push(GroupPlan::new(&repo, archives.get(g).unwrap_or_default(), oldest, keep_deleted)?);
            }
            Step::Remove(i) => removals.extend(archives.get(i).copied()),
        }
    }
    let total = plans.iter().map(|p| p.bytes).sum();
    let files = plans.iter().map(|p| p.holder.len() as u64).sum();
    let mut progress = Progress::new(sink, total, files);
    let groups = plans.len() as u64;
    for (i, plan) in plans.iter().enumerate() {
        progress.group(plan.describe(i as u64 + 1, groups));
        plan.execute(&repo, profile, &mut progress)?;
    }
    for name in &removals {
        // Listing what goes is worth a manifest read, not a failed deletion.
        let gone = repo.manifest(name);
        if let Err(e) = &gone {
            tracing::warn!(archive = %name, error = %e, "files of the deleted archive not listed");
        }
        repo.remove_archive(name)?;
        for e in gone.iter().flat_map(|m| &m.entries).filter(|e| e.kind == EntryKind::File && e.stored) {
            progress.listed(dropped(&e.key, e.size));
        }
        tracing::info!(archive = %name, "archive deleted");
    }
    progress.finish();

    report.archives_after = repo.archives()?.len() as u64;
    report.bytes_freed = size_before.saturating_sub(dir_size(repo.root()));
    tracing::info!(
        before = report.archives_before,
        after = report.archives_after,
        freed = report.bytes_freed,
        "merge done"
    );
    sink.emit(RunEvent::Finished { summary: Summary::Merge(report.clone()) });
    Ok(report)
}

/// The line of the file list for a version of `key`, of `size` bytes,
/// that no backup keeps any more.
fn dropped(key: &str, size: u64) -> FileDone {
    FileDone::new(keys::path_for(key).display().to_string(), FileAction::Dropped, size)
}

/// What one group becomes.
struct GroupPlan {
    members: Vec<ArchiveName>,
    target: ArchiveName,
    /// The last member's manifest, each file marked stored when a member
    /// holds its newest content, with that member's chunk list.
    manifest: Manifest,
    /// Key -> index in `members` of the archive holding its newest content.
    holder: HashMap<String, usize>,
    /// Versions held by members and not by the merged archive: key, size.
    dropped: Vec<(String, u64)>,
    bytes: u64,
    /// Some member lists chunks.
    lists_chunks: bool,
}

impl GroupPlan {
    /// The merged archive is complete when the group holds every file it
    /// lists and contains a complete archive (a group starting with the
    /// initial complete one always does); otherwise it is incremental and
    /// the files it does not hold stay in the archives before the group.
    /// `oldest` tells that the group starts with the oldest archive, which
    /// must hold everything. `keep_deleted`: the files and folders listed
    /// by a member but not by the last one (deleted at the source inside
    /// the group) stay, as the last member listing them had them.
    fn new(repo: &Repo, members: &[ArchiveName], oldest: bool, keep_deleted: bool) -> Result<Self> {
        let Some(last) = members.last() else {
            return Err(EngineError::Damaged { archive: String::new(), message: "empty merge group".into() });
        };

        let mut holder = HashMap::new();
        let mut lists: HashMap<String, Option<Chunks>> = HashMap::new();
        let mut manifest = None;
        let mut format = 0;
        let mut lists_chunks = false;
        // Every stored version: key, member, size.
        let mut versions: Vec<(String, usize, u64)> = Vec::new();
        // Every entry of the group, as the last member listing it has it.
        let mut seen: HashMap<String, Entry> = HashMap::new();
        for (j, name) in members.iter().enumerate() {
            let mut m = repo.manifest(name)?;
            format = format.max(m.format);
            if keep_deleted {
                seen.extend(m.entries.iter().map(|e| (e.key.clone(), Entry { chunks: None, ..e.clone() })));
            }
            for e in m.entries.iter_mut().filter(|e| e.stored) {
                if e.kind == EntryKind::File {
                    versions.push((e.key.clone(), j, e.size));
                }
                lists_chunks |= e.chunks.is_some();
                holder.insert(e.key.clone(), j);
                lists.insert(e.key.clone(), e.chunks.take());
            }
            manifest = Some(m);
        }
        let mut manifest = manifest.expect("group is not empty");
        manifest.format = format;
        if keep_deleted {
            let gone: Vec<Entry> = seen.into_values().filter(|e| manifest.get(&e.key).is_none()).collect();
            if !gone.is_empty() {
                manifest.entries.extend(gone);
                manifest.entries.sort_by(|a, b| a.key.cmp(&b.key));
            }
        }
        let mut missing = None;
        for e in manifest.entries.iter_mut().filter(|e| e.kind == EntryKind::File) {
            e.stored = holder.contains_key(&e.key);
            e.chunks = if e.stored { lists.remove(&e.key).flatten() } else { None };
            if !e.stored && missing.is_none() {
                missing = Some(e.key.clone());
            }
        }
        let has_complete = members.iter().any(|m| m.kind == ArchiveKind::Complete);
        let kind = match &missing {
            None if has_complete => ArchiveKind::Complete,
            Some(key) if oldest => {
                return Err(EngineError::Damaged {
                    archive: last.to_string(),
                    message: format!("no archive holds the content of {key}"),
                });
            }
            _ => ArchiveKind::Incremental,
        };
        manifest.kind = kind;
        let target = ArchiveName { time: last.time, kind };
        holder.retain(|k, _| manifest.get(k).is_some_and(|e| e.stored));
        let dropped = versions
            .into_iter()
            .filter(|(key, j, _)| holder.get(key) != Some(j))
            .map(|(key, _, size)| (key, size))
            .collect();
        // A file kept as chunks only costs nothing to carry, unless a
        // complete archive needs it whole.
        let complete = kind == ArchiveKind::Complete;
        let bytes = manifest
            .entries
            .iter()
            .filter(|e| e.stored && (complete || !e.chunks_only()))
            .map(|e| e.size)
            .fold(0u64, u64::saturating_add);
        Ok(Self { members: members.to_vec(), target, manifest, holder, dropped, bytes, lists_chunks })
    }

    /// This group as shown in the window: number `index` of `of`.
    fn describe(&self, index: u64, of: u64) -> MergeGroup {
        MergeGroup {
            index,
            of,
            archives: self.members.len() as u64,
            first: self.members.first().map(ArchiveName::label).unwrap_or_default(),
            last: self.members.last().map(ArchiveName::label).unwrap_or_default(),
        }
    }

    fn execute(&self, repo: &Repo, profile: &Profile, progress: &mut Progress<'_>) -> Result<()> {
        let staging = repo.staging(&self.target, Purpose::Merge)?;
        // Keep the destination's format, whatever the profile says now.
        let container =
            self.members.last().map_or(profile.container, |m| crate::container::detect(&repo.dir(m)));
        let mut writer = Writer::create(&staging, container, repo.keys(), &self.target)?;
        let form = Form::of(container, repo.keys());
        let with_chunks =
            self.lists_chunks || self.members.iter().any(|m| repo.dir(m).join(CHUNKS_DIR).exists());
        let chunk_places =
            if with_chunks { Some(ChunkPlaces::read(repo, &self.members, &form)?) } else { None };
        let complete = self.target.kind == ArchiveKind::Complete;

        let mut manifest = self.manifest.clone();
        let mut readers: Vec<Option<Reader>> = self.members.iter().map(|_| None).collect();
        for e in manifest.entries.iter_mut().filter(|e| e.stored) {
            progress.check_stop()?;
            let Some(&j) = self.holder.get(&e.key) else { continue };
            progress.start_file(keys::path_for(&e.key).display().to_string());
            if e.chunks_only() {
                if complete {
                    // A complete archive holds every file whole.
                    let (Some(places), Some(chunks)) = (chunk_places.as_ref(), e.chunks.as_mut()) else {
                        continue;
                    };
                    let reader = places.reader(&places.all, &form, repo);
                    let (_, hash) = writer.add_with(&e.key, e.size, e.mtime_ns, &mut |out| {
                        reader.write_all(&chunks.list, out, &mut |n| progress.add(n))
                    })?;
                    if e.hash.as_ref().is_some_and(|h| *h != hash) {
                        return Err(EngineError::Damaged {
                            archive: self.target.to_string(),
                            message: format!("{} does not match its hash once put together", e.key),
                        });
                    }
                    chunks.whole = true;
                }
                progress.file_done(FileAction::Moved, e.size);
                continue;
            }
            let (Some(slot), Some(name)) = (readers.get_mut(j), self.members.get(j)) else { continue };
            if slot.is_none() {
                *slot = Some(Reader::open(&repo.dir(name), repo.keys())?);
            }
            let reader = slot.as_mut().expect("opened above");
            writer.add_from(reader, &e.key, e.mtime_ns)?;
            progress.add(e.size);
            progress.file_done(FileAction::Moved, e.size);
        }
        drop(readers);
        if let Some(places) = &chunk_places {
            places.keep_needed(&manifest, &staging, &form, repo)?;
        }
        for e in &mut manifest.entries {
            e.blob = if e.whole_copy() { writer.stored_as(&e.key) } else { None };
        }
        writer.finish()?;
        repo.write_manifest(&staging, &manifest)?;
        repo.commit_merge(&staging, &self.target, &self.members)?;
        for (key, size) in &self.dropped {
            progress.listed(dropped(key, *size));
        }
        tracing::info!(target_archive = %self.target, members = self.members.len(), "group merged");
        Ok(())
    }
}

/// Where the chunks of the destination are, seen from one merge group.
struct ChunkPlaces {
    /// Every archive folder, oldest first.
    dirs: Vec<PathBuf>,
    /// Chunks readable anywhere.
    all: ChunkIndex,
    /// Chunks readable inside the group's members.
    members: ChunkIndex,
    /// Chunks readable in the archives before the group, which stay as
    /// they are. Only those count: a restore or a check of an archive reads
    /// the chunks of that archive and of the earlier ones, never of later
    /// ones (which may be merged or deleted on their own).
    outside: HashSet<ChunkId>,
    /// Chunks the archives after the group list and find in no archive
    /// they can read except the group's: the merged archive must hold them.
    needed: HashSet<ChunkRef>,
}

impl ChunkPlaces {
    /// Reads every archive of the destination as it is now.
    fn read(repo: &Repo, members: &[ArchiveName], form: &Form) -> Result<Self> {
        let archives = repo.archives()?;
        let dirs: Vec<PathBuf> = archives.iter().map(|a| repo.dir(a)).collect();
        let mut places = Self {
            dirs: Vec::new(),
            all: ChunkIndex::default(),
            members: ChunkIndex::default(),
            outside: HashSet::new(),
            needed: HashSet::new(),
        };
        let first_member = archives.iter().position(|a| members.contains(a)).unwrap_or(archives.len());
        // Chunks of the archives after the group, up to the one being read:
        // what each of them can read besides the earlier archives.
        let mut after: HashSet<ChunkId> = HashSet::new();
        for (i, (name, dir)) in archives.iter().zip(&dirs).enumerate() {
            let files = form.list(dir)?;
            let m = repo.manifest(name)?;
            places.all.add_files(i, files.iter().copied());
            places.all.add_whole_copies(i, &m);
            if members.contains(name) {
                places.members.add_files(i, files);
                places.members.add_whole_copies(i, &m);
            } else if i < first_member {
                places.outside.extend(files);
                places.outside.extend(chunks::in_whole_copies(&m));
            } else {
                after.extend(files);
                after.extend(chunks::in_whole_copies(&m));
                let missing = chunks::referenced(&m)
                    .filter(|c| !places.outside.contains(&c.id) && !after.contains(&c.id));
                places.needed.extend(missing.copied());
            }
        }
        places.dirs = dirs;
        Ok(places)
    }

    fn reader<'a>(&'a self, index: &'a ChunkIndex, form: &'a Form, repo: &'a Repo) -> ChunkReader<'a> {
        ChunkReader { index, dirs: &self.dirs, form, keys: repo.keys() }
    }

    /// Puts into the merged archive in `staging` (whose manifest is
    /// `merged`) a chunk file for every chunk that a remaining manifest
    /// lists and that nothing else remaining provides: linked from a
    /// member's chunk file, or cut out of a member's whole copy.
    fn keep_needed(
        &self,
        merged: &Manifest,
        staging: &std::path::Path,
        form: &Form,
        repo: &Repo,
    ) -> Result<()> {
        let in_target: HashSet<ChunkId> = chunks::in_whole_copies(merged).collect();
        let mut wanted: HashSet<ChunkRef> = self.needed.clone();
        wanted.extend(chunks::referenced(merged).copied());
        let mut from_copies = Vec::new();
        let mut linked = 0u64;
        for chunk in wanted {
            if self.outside.contains(&chunk.id) || in_target.contains(&chunk.id) {
                continue;
            }
            match self.members.file_of(&chunk.id).and_then(|a| self.dirs.get(a)) {
                Some(dir) => {
                    let ctx = format!("merging chunk {}", form.relative(&chunk.id));
                    link_or_copy(&form.path(dir, &chunk.id), &form.path(staging, &chunk.id), &ctx)?;
                    linked += 1;
                }
                None => from_copies.push(chunk),
            }
        }
        let cut = from_copies.len();
        self.reader(&self.members, form, repo)
            .for_each(&from_copies, &mut |chunk, data| form.write(staging, &chunk.id, data, true))?;
        tracing::info!(linked, cut, "chunks kept in the merged archive");
        Ok(())
    }
}
