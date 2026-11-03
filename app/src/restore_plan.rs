//! What the "Restore files" view restores, decided without egui.
//!
//! The selection is a list of [`Pick`]s: ticked files, each with the
//! versions ticked, and ticked folders, each with the date chosen. All of
//! it goes to the one destination chosen at the top of the panel. [`plan`]
//! turns that into one [`RestoreRequest`], with the name each
//! version will take, the conflicts, the summary, and why the Restore
//! button waits.

use archeion_core::{
    EntryKind, keys,
    restore::{
        Output, RestoreItem, RestoreRequest, RestoreTarget, date_tag, make_distinct, output_path,
        precise_date_tag,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// Where the selection goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dest {
    /// Next to the original, with the backup date in the name.
    Beside,
    /// The original place; `replace` replaces the file found there.
    Original { replace: bool },
    /// Below this folder, keeping the full path; `replace` replaces the
    /// files found there.
    Folder { path: PathBuf, replace: bool },
}

impl Dest {
    /// The engine's target, and whether existing files are replaced.
    fn target(&self) -> (RestoreTarget, bool) {
        match self {
            Self::Beside => (RestoreTarget::BesideOriginal, false),
            Self::Original { replace } => (RestoreTarget::Original, *replace),
            Self::Folder { path, replace } => (RestoreTarget::Folder { path: path.clone() }, *replace),
        }
    }

    /// Replaces the current file at its original place.
    #[must_use]
    pub fn overwrites_original(&self) -> bool {
        matches!(self, Self::Original { replace: true })
    }
}

/// The three choices of the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DestKind {
    #[default]
    Beside,
    Original,
    Folder,
}

/// The destination as chosen at the top of the panel; "another folder"
/// may still lack its folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultDest {
    pub kind: DestKind,
    pub folder: Option<PathBuf>,
    /// Replace existing files (original place or another folder).
    pub replace: bool,
}

impl DefaultDest {
    /// `None` while "another folder" has no folder.
    #[must_use]
    pub fn resolve(&self) -> Option<Dest> {
        match self.kind {
            DestKind::Beside => Some(Dest::Beside),
            DestKind::Original => Some(Dest::Original { replace: self.replace }),
            DestKind::Folder => self.folder.clone().map(|path| Dest::Folder { path, replace: self.replace }),
        }
    }
}

/// A ticked version of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Archive that first stores this content.
    pub archive: String,
    pub size: u64,
}

/// One card of the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    File {
        key: String,
        /// Its versions are still being read.
        loading: bool,
        /// The ticked versions, newest first.
        versions: Vec<Version>,
    },
    /// The folder as it was at one date, with everything in it.
    Folder {
        key: String,
        /// Archive of the date chosen.
        archive: String,
        /// Bytes of its files at that date, once known.
        size: Option<u64>,
        /// Keys unticked inside it, left out with what they hold.
        exclude: Vec<String>,
    },
}

impl Pick {
    #[must_use]
    pub fn key(&self) -> &str {
        match self {
            Self::File { key, .. } | Self::Folder { key, .. } => key,
        }
    }
}

/// Counts shown above the Restore button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Files with at least one version ticked.
    pub files: usize,
    pub versions: usize,
    pub folders: usize,
    /// Bytes of the versions and of the folders whose size is known.
    pub bytes: u64,
    /// The size of some folder is not known yet: `bytes` is a lower bound.
    pub partial: bool,
}

/// Why the Restore button waits, in the order it is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    /// Nothing ticked.
    Nothing,
    /// The versions of a ticked file are still being read.
    Loading,
    /// The destination is "another folder" with no folder yet.
    NoFolder,
    /// Two different versions would overwrite the same original.
    Conflict,
    /// A backup, merge or restore is running.
    Busy,
}

/// The outcome of [`plan`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub request: RestoreRequest,
    /// Path each ticked version will be written to, by (card, version).
    outputs: BTreeMap<(usize, usize), PathBuf>,
    /// Cards whose items overwrite an original that another card, or
    /// another version of the same file, overwrites too.
    pub conflicts: BTreeSet<usize>,
    pub summary: Summary,
    /// `None` when the restore can start.
    pub blocker: Option<Blocker>,
}

impl Plan {
    /// Where version `version` of card `card` will be written.
    #[must_use]
    pub fn output(&self, card: usize, version: usize) -> Option<&Path> {
        self.outputs.get(&(card, version)).map(PathBuf::as_path)
    }
}

/// An item that replaces what is at its original place: card, key, date.
struct Claim<'a> {
    card: usize,
    key: &'a str,
    archive: &'a str,
    /// Left out of a folder: not claimed.
    exclude: &'a [String],
}

impl Claim<'_> {
    /// Both write the original of some file or folder.
    fn overlaps(&self, other: &Self) -> bool {
        let inside = |a: &Claim<'_>, b: &Claim<'_>| {
            keys::is_within(a.key, b.key) && !b.exclude.iter().any(|x| keys::is_within(a.key, x))
        };
        inside(self, other) || inside(other, self)
    }
}

/// Turns the selection into a request. Several versions of a file sent to
/// the same folder each take their date in the name; overwriting an
/// original is allowed for one version only: a second one, of the file or
/// of a ticked folder holding it, is a conflict.
#[must_use]
pub fn plan(picks: &[Pick], default: &DefaultDest, busy: bool) -> Plan {
    let dest = default.resolve();
    let mut out = Plan::default();
    let missing_folder = dest.is_none();
    let mut loading = false;
    let mut claims: Vec<Claim<'_>> = Vec::new();
    // (card, version) and the file each version writes.
    let mut files: Vec<((usize, usize), Output)> = Vec::new();
    for (card, pick) in picks.iter().enumerate() {
        match pick {
            Pick::File { key, loading: still, versions } => {
                loading |= *still;
                out.summary.files += usize::from(!versions.is_empty());
                for (n, v) in versions.iter().enumerate() {
                    out.summary.versions += 1;
                    out.summary.bytes = out.summary.bytes.saturating_add(v.size);
                    let Some(dest) = &dest else { continue };
                    if dest.overwrites_original() {
                        claims.push(Claim { card, key, archive: &v.archive, exclude: &[] });
                    }
                    let (target, overwrite) = dest.target();
                    let tag = date_tag(&v.archive);
                    let path = output_path(key, EntryKind::File, &target, &tag);
                    let original = target == RestoreTarget::Original;
                    let precise_tag = precise_date_tag(&v.archive);
                    files.push(((card, n), Output { path, tag, precise_tag, original }));
                    out.request.items.push(RestoreItem {
                        archive: v.archive.clone(),
                        key: Some(key.clone()),
                        target,
                        overwrite,
                        exclude: Vec::new(),
                    });
                }
            }
            Pick::Folder { key, archive, size, exclude } => {
                out.summary.folders += 1;
                match size {
                    Some(b) => out.summary.bytes = out.summary.bytes.saturating_add(*b),
                    None => out.summary.partial = true,
                }
                let Some(dest) = &dest else { continue };
                if dest.overwrites_original() {
                    claims.push(Claim { card, key, archive, exclude });
                }
                let (target, overwrite) = dest.target();
                out.request.items.push(RestoreItem {
                    archive: archive.clone(),
                    key: Some(key.clone()),
                    target,
                    overwrite,
                    exclude: exclude.clone(),
                });
            }
        }
    }

    let (ids, mut outputs): (Vec<(usize, usize)>, Vec<Output>) = files.into_iter().unzip();
    make_distinct(&mut outputs);
    out.outputs = ids.into_iter().zip(outputs.into_iter().map(|o| o.path)).collect();

    for (i, a) in claims.iter().enumerate() {
        for b in claims.iter().skip(i + 1) {
            if a.overlaps(b) && a.archive != b.archive {
                out.conflicts.insert(a.card);
                out.conflicts.insert(b.card);
            }
        }
    }

    out.blocker = if out.summary.versions + out.summary.folders == 0 && !loading {
        Some(Blocker::Nothing)
    } else if loading {
        Some(Blocker::Loading)
    } else if missing_folder {
        Some(Blocker::NoFolder)
    } else if !out.conflicts.is_empty() {
        Some(Blocker::Conflict)
    } else if busy {
        Some(Blocker::Busy)
    } else {
        None
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "2026-10-20_090000_C";
    const NEW: &str = "2026-11-01_183300_I";
    const KEY: &str = "C/Docs/report.docx";

    fn version(archive: &str) -> Version {
        Version { archive: archive.into(), size: 10 }
    }

    fn file(key: &str, versions: Vec<Version>) -> Pick {
        Pick::File { key: key.into(), loading: false, versions }
    }

    fn folder(key: &str, archive: &str, size: Option<u64>) -> Pick {
        Pick::Folder { key: key.into(), archive: archive.into(), size, exclude: Vec::new() }
    }

    /// The original place, replacing the current files.
    fn overwrite() -> DefaultDest {
        DefaultDest { kind: DestKind::Original, folder: None, replace: true }
    }

    #[test]
    fn a_file_left_out_of_a_folder_does_not_conflict_with_it() {
        let mut docs = folder("C/Docs", OLD, None);
        if let Pick::Folder { exclude, .. } = &mut docs {
            exclude.push(KEY.into());
        }
        let picks = [file(KEY, vec![version(NEW)]), docs];
        let p = plan(&picks, &overwrite(), false);
        assert!(p.conflicts.is_empty());
        assert_eq!(p.request.items[1].exclude, vec![KEY.to_owned()], "the exclusion reaches the engine");
    }

    fn to_folder(path: &str) -> DefaultDest {
        DefaultDest { kind: DestKind::Folder, folder: Some(PathBuf::from(path)), replace: false }
    }

    fn name(p: Option<&Path>) -> String {
        p.and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    #[test]
    fn nothing_ticked_waits_for_a_tick() {
        let p = plan(&[], &DefaultDest::default(), false);
        assert_eq!(p.blocker, Some(Blocker::Nothing));
        assert!(p.request.items.is_empty());

        // A file whose versions are all unticked counts as nothing.
        let p = plan(&[file(KEY, vec![])], &DefaultDest::default(), false);
        assert_eq!(p.blocker, Some(Blocker::Nothing));
        assert_eq!(p.summary.files, 0);
    }

    #[test]
    fn a_file_still_loading_its_versions_waits() {
        let picks = [Pick::File { key: KEY.into(), loading: true, versions: vec![] }];
        assert_eq!(plan(&picks, &DefaultDest::default(), false).blocker, Some(Blocker::Loading));
    }

    #[test]
    fn versions_and_folders_become_items_with_the_destination() {
        let picks = [file(KEY, vec![version(NEW), version(OLD)]), folder("C/Photos", OLD, Some(1000))];
        let p = plan(&picks, &DefaultDest::default(), false);
        assert_eq!(p.blocker, None);
        assert_eq!(p.request.items.len(), 3);
        assert!(p.request.items.iter().all(|i| i.target == RestoreTarget::BesideOriginal && !i.overwrite));
        assert_eq!(p.request.items[2].key.as_deref(), Some("C/Photos"));
        assert_eq!(p.request.items[2].archive, OLD);
        assert_eq!(
            p.summary,
            Summary { files: 1, versions: 2, folders: 1, bytes: 1020, partial: false },
            "3 files, 5 versions, 1 folder — the size"
        );
        assert_eq!(name(p.output(0, 0)), "report (2026-11-01 18.33).docx");
        assert_eq!(name(p.output(0, 1)), "report (2026-10-20 09.00).docx");
        assert_eq!(p.output(1, 0), None, "a folder's files are named by the engine");
    }

    #[test]
    fn several_versions_to_one_folder_each_take_their_date() {
        let picks = [file(KEY, vec![version(NEW), version(OLD)])];
        let p = plan(&picks, &to_folder("out"), false);
        assert_eq!(name(p.output(0, 0)), "report (2026-11-01 18.33).docx");
        assert_eq!(name(p.output(0, 1)), "report (2026-10-20 09.00).docx");

        // Alone, a version keeps the plain name.
        let p = plan(&[file(KEY, vec![version(NEW)])], &to_folder("out"), false);
        assert_eq!(name(p.output(0, 0)), "report.docx");
        assert!(p.output(0, 0).unwrap().starts_with("out"));
    }

    #[test]
    fn one_version_may_overwrite_the_original_two_conflict() {
        let p = plan(&[file(KEY, vec![version(OLD)])], &overwrite(), false);
        assert_eq!(p.blocker, None);
        assert_eq!(name(p.output(0, 0)), "report.docx", "the overwriting version keeps the name");
        assert!(p.request.items[0].overwrite);

        let two = [file(KEY, vec![version(NEW), version(OLD)])];
        let p = plan(&two, &overwrite(), false);
        assert_eq!(p.blocker, Some(Blocker::Conflict));
        assert_eq!(p.conflicts, BTreeSet::from([0]));
        // Without replace, the current file is kept: no conflict.
        let keep = DefaultDest { replace: false, ..overwrite() };
        assert_eq!(plan(&two, &keep, false).blocker, None);
    }

    #[test]
    fn a_folder_overwriting_the_original_conflicts_with_a_file_inside_it() {
        let picks = [file(KEY, vec![version(NEW)]), folder("C/Docs", OLD, None)];
        let p = plan(&picks, &overwrite(), false);
        assert_eq!(p.conflicts, BTreeSet::from([0, 1]));
        assert_eq!(p.blocker, Some(Blocker::Conflict));

        // The same date is the same content: no conflict.
        let picks = [file(KEY, vec![version(OLD)]), folder("C/Docs", OLD, None)];
        assert!(plan(&picks, &overwrite(), false).conflicts.is_empty());
    }

    #[test]
    fn another_folder_without_its_folder_waits() {
        let default = DefaultDest { kind: DestKind::Folder, folder: None, replace: false };
        let picks = [file(KEY, vec![version(NEW)])];
        assert_eq!(plan(&picks, &default, false).blocker, Some(Blocker::NoFolder));
        assert!(plan(&picks, &default, false).request.items.is_empty());
    }

    #[test]
    fn a_running_operation_blocks_last() {
        let picks = [file(KEY, vec![version(NEW)])];
        assert_eq!(plan(&picks, &DefaultDest::default(), true).blocker, Some(Blocker::Busy));
        assert_eq!(plan(&[], &DefaultDest::default(), true).blocker, Some(Blocker::Nothing));
    }

    #[test]
    fn an_unknown_folder_size_makes_the_total_partial() {
        let picks = [folder("C/Photos", OLD, None), folder("C/Music", OLD, Some(5))];
        let s = plan(&picks, &DefaultDest::default(), false).summary;
        assert_eq!((s.folders, s.bytes, s.partial), (2, 5, true));
    }

    #[test]
    fn another_folder_with_replace_reaches_the_engine() {
        let default = DefaultDest { replace: true, ..to_folder("elsewhere") };
        let p = plan(&[folder("C/Photos", OLD, Some(1))], &default, false);
        let item = &p.request.items[0];
        assert_eq!(item.target, RestoreTarget::Folder { path: PathBuf::from("elsewhere") });
        assert!(item.overwrite);
    }
}
