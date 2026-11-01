//! Merge: archives fused into one that keeps the newest content.
//!
//! The scheduled merge follows the profile's rules and never touches the
//! initial complete archive; a manual merge fuses the archives the user
//! chose, including the complete one if asked. Deleting an archive that
//! later ones depend on is a merge with the next one.

use crate::{
    EngineError, EventSink, Result,
    container::{Reader, Writer},
    crypto::Password,
    progress::Progress,
    repo::{Purpose, Repo, dir_size},
};
use archeion_core::{
    ArchiveKind, ArchiveName, EntryKind, Manifest, MergeReport, Phase, Profile, RunEvent, Summary,
    merge_plan::{Deletion, automatic_keep_set, deletion, merge_groups, range_group},
};
use std::{collections::HashMap, ops::RangeInclusive};

/// Applies the profile's merge rules to its archives.
///
/// Each group of consecutive archives that the rules do not keep is merged
/// with the kept archive that follows it; the result carries that archive's
/// date and holds the newest content of the group. Files deleted or
/// replaced inside the group are dropped, which frees space. The initial
/// complete archive is always kept.
///
/// # Errors
/// See [`EngineError`].
pub fn merge(profile: &Profile, password: Option<&Password>, sink: &dyn EventSink) -> Result<MergeReport> {
    run(profile, password, sink, |archives| {
        Ok(merge_groups(&automatic_keep_set(archives, &profile.merge.rules))
            .into_iter()
            .map(Step::Merge)
            .collect())
    })
}

/// Merges the archives from `first` to `last` (names as listed, either
/// order) and every archive between them into one, dated like the newest.
/// Including the initial complete archive produces a new complete archive.
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
    run(profile, password, sink, |archives| {
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
    run(profile, password, sink, |archives| {
        match deletion(archives, &name).map_err(|e| EngineError::InvalidProfile(e.to_string()))? {
            Deletion::Remove(i) => Ok(vec![Step::Remove(i)]),
            Deletion::MergeWithNext(g) => Ok(vec![Step::Merge(g)]),
        }
    })
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

fn run(
    profile: &Profile,
    password: Option<&Password>,
    sink: &dyn EventSink,
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
    let mut report = MergeReport { archives_before: archives.len() as u64, ..MergeReport::default() };
    let size_before = dir_size(repo.root());

    sink.emit(RunEvent::Phase { phase: Phase::Merging });
    let mut plans = Vec::new();
    let mut removals = Vec::new();
    for step in steps {
        match step {
            Step::Merge(g) => {
                let oldest = *g.start() == 0;
                plans.push(GroupPlan::new(&repo, archives.get(g).unwrap_or_default(), oldest)?);
            }
            Step::Remove(i) => removals.extend(archives.get(i).copied()),
        }
    }
    let total = plans.iter().map(|p| p.bytes).sum();
    let files = plans.iter().map(|p| p.holder.len() as u64).sum();
    let mut progress = Progress::new(sink, total, files);
    for plan in &plans {
        plan.execute(&repo, profile, &mut progress)?;
    }
    for name in &removals {
        repo.remove_archive(name)?;
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

/// What one group becomes.
struct GroupPlan {
    members: Vec<ArchiveName>,
    target: ArchiveName,
    manifest: Manifest,
    /// Key -> index in `members` of the archive holding its newest content.
    holder: HashMap<String, usize>,
    bytes: u64,
}

impl GroupPlan {
    /// The merged archive is complete when the group holds every file it
    /// lists and contains a complete archive (a group starting with the
    /// initial complete one always does); otherwise it is incremental and
    /// the files it does not hold stay in the archives before the group.
    /// `oldest` tells that the group starts with the oldest archive, which
    /// must hold everything.
    fn new(repo: &Repo, members: &[ArchiveName], oldest: bool) -> Result<Self> {
        let Some(last) = members.last() else {
            return Err(EngineError::Damaged { archive: String::new(), message: "empty merge group".into() });
        };

        let mut holder = HashMap::new();
        let mut manifest = None;
        for (j, name) in members.iter().enumerate() {
            let m = repo.manifest(name)?;
            for e in m.entries.iter().filter(|e| e.stored) {
                holder.insert(e.key.clone(), j);
            }
            manifest = Some(m);
        }
        let mut manifest = manifest.expect("group is not empty");
        let mut bytes = 0u64;
        let mut missing = None;
        for e in manifest.entries.iter_mut().filter(|e| e.kind == EntryKind::File) {
            e.stored = holder.contains_key(&e.key);
            if e.stored {
                bytes = bytes.saturating_add(e.size);
            } else if missing.is_none() {
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
        Ok(Self { members: members.to_vec(), target, manifest, holder, bytes })
    }

    fn execute(&self, repo: &Repo, profile: &Profile, progress: &mut Progress<'_>) -> Result<()> {
        let staging = repo.staging(&self.target, Purpose::Merge)?;
        // Keep the destination's format, whatever the profile says now.
        let container =
            self.members.last().map_or(profile.container, |m| crate::container::detect(&repo.dir(m)));
        let mut writer = Writer::create(&staging, container, repo.keys(), &self.target)?;
        let mut readers: Vec<Option<Reader>> = self.members.iter().map(|_| None).collect();
        for e in self.manifest.entries.iter().filter(|e| e.stored) {
            let Some(&j) = self.holder.get(&e.key) else { continue };
            let (Some(slot), Some(name)) = (readers.get_mut(j), self.members.get(j)) else { continue };
            if slot.is_none() {
                *slot = Some(Reader::open(&repo.dir(name), repo.keys())?);
            }
            let reader = slot.as_mut().expect("opened above");
            progress.start_file(archeion_core::keys::path_for(&e.key).display().to_string());
            writer.add_from(reader, &e.key, e.mtime_ns)?;
            progress.add(e.size);
        }
        drop(readers);
        let mut manifest = self.manifest.clone();
        for e in &mut manifest.entries {
            e.blob = if e.stored { writer.stored_as(&e.key) } else { None };
        }
        writer.finish()?;
        repo.write_manifest(&staging, &manifest)?;
        repo.commit_merge(&staging, &self.target, &self.members)?;
        tracing::info!(target_archive = %self.target, members = self.members.len(), "group merged");
        Ok(())
    }
}
