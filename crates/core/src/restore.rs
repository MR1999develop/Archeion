//! What a restore is asked to do, and the names of the files it writes.
//!
//! A [`RestoreRequest`] is a list of [`RestoreItem`]s: a file or folder as
//! it was in one archive, sent to one place. The window writes the request
//! to a file for the worker; the request files of earlier versions (one
//! archive, several keys, one target) are still read.
//!
//! Where each file goes is decided here, so the window can show the names
//! the engine will use: [`output_path`] for one file, then
//! [`make_distinct`] when several files would land on the same path.

use crate::{ArchiveName, EntryKind, keys};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// Where restored files go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum RestoreTarget {
    /// Their original location.
    Original,
    /// Next to the original, with the backup date in the name:
    /// `report (2026-11-02 21.30).docx`. The current file is never touched.
    BesideOriginal,
    /// Below this folder, keeping the full path (`<folder>\C\Users\...`).
    Folder { path: PathBuf },
}

/// One thing to restore: a file or folder as it was in one archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreItem {
    /// Archive name, as listed: the date restored.
    pub archive: String,
    /// Key of the file or folder; `None` = everything in the archive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub target: RestoreTarget,
    /// Replace files that already exist.
    #[serde(default)]
    pub overwrite: bool,
}

/// What to restore, in one run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RequestFile")]
pub struct RestoreRequest {
    pub items: Vec<RestoreItem>,
}

/// The request file as written by this and by earlier versions.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RequestFile {
    Items {
        items: Vec<RestoreItem>,
    },
    /// Before several items could be restored at once: one archive, the
    /// keys to restore from it (none = everything), one target.
    Single {
        archive: String,
        #[serde(default)]
        keys: Vec<String>,
        target: RestoreTarget,
        #[serde(default)]
        overwrite: bool,
    },
}

impl From<RequestFile> for RestoreRequest {
    fn from(file: RequestFile) -> Self {
        match file {
            RequestFile::Items { items } => Self { items },
            RequestFile::Single { archive, keys, target, overwrite } => {
                Self::single(&archive, keys, &target, overwrite)
            }
        }
    }
}

impl RestoreRequest {
    /// The `keys` of one archive (none = everything) to one target.
    #[must_use]
    pub fn single(archive: &str, keys: Vec<String>, target: &RestoreTarget, overwrite: bool) -> Self {
        let item = |key| RestoreItem { archive: archive.to_owned(), key, target: target.clone(), overwrite };
        let items =
            if keys.is_empty() { vec![item(None)] } else { keys.into_iter().map(Some).map(item).collect() };
        Self { items }
    }

    /// The archives restored from, sorted, each once.
    #[must_use]
    pub fn archives(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.items.iter().map(|i| i.archive.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

/// The date of an archive as it goes into a file name:
/// `2026-11-02_213000_I` -> `2026-11-02 21.30` (a colon is not allowed in
/// Windows file names). Text that is not an archive name is used as it is.
#[must_use]
pub fn date_tag(archive: &str) -> String {
    archive.parse::<ArchiveName>().map_or_else(|_| archive.to_owned(), |n| n.label().replace(':', "."))
}

/// [`date_tag`] with the seconds: `2026-11-02 21.30.05`, to tell apart two
/// backups made within the same minute.
#[must_use]
pub fn precise_date_tag(archive: &str) -> String {
    archive.parse::<ArchiveName>().map_or_else(
        |_| archive.to_owned(),
        |n| format!("{}.{:02}", n.label().replace(':', "."), n.time.second()),
    )
}

/// `C:\a\report.docx` -> `C:\a\report (2026-11-02 21.30).docx`.
#[must_use]
pub fn with_date_tag(path: &Path, tag: &str) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem} ({tag}).{}", ext.to_string_lossy()),
        None => format!("{stem} ({tag})"),
    };
    path.with_file_name(name)
}

/// Where the file or folder `key` goes for `target`; `tag` is the date of
/// its archive ([`date_tag`]). Next to the original, only files take the
/// date: their folders are the original ones.
#[must_use]
pub fn output_path(key: &str, kind: EntryKind, target: &RestoreTarget, tag: &str) -> PathBuf {
    match target {
        RestoreTarget::Original => keys::path_for(key),
        RestoreTarget::BesideOriginal if kind == EntryKind::File => with_date_tag(&keys::path_for(key), tag),
        RestoreTarget::BesideOriginal => keys::path_for(key),
        RestoreTarget::Folder { path } => keys::path_under(path, key),
    }
}

/// One file a restore is about to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// From [`output_path`]; [`make_distinct`] may change it.
    pub path: PathBuf,
    /// Date of its archive ([`date_tag`]).
    pub tag: String,
    /// The same with the seconds ([`precise_date_tag`]), used when two
    /// outputs of the same minute would get the same name.
    pub precise_tag: String,
    /// Restored to its original place: when it is the only one of a group
    /// sharing a path, it keeps the plain name.
    pub original: bool,
}

/// Gives every output its own path. When several share one (two versions
/// of a file restored to the same folder), each takes its date in the name,
/// as next to the original, except a single one restored to its original
/// place, which keeps the plain name; the date takes the seconds when two
/// backups of the same minute collide, and a number follows when even that
/// name is taken. Outputs that share no path are left alone, so the
/// result does not depend on the order.
pub fn make_distinct(outputs: &mut [Output]) {
    // Per path: how many outputs, and how many of them to the original place.
    let mut groups: HashMap<PathBuf, (usize, usize)> = HashMap::new();
    for o in outputs.iter() {
        let g = groups.entry(o.path.clone()).or_default();
        g.0 += 1;
        g.1 += usize::from(o.original);
    }
    let keeps = |o: &Output| {
        groups.get(&o.path).is_none_or(|&(n, originals)| n == 1 || (o.original && originals == 1))
    };
    let mut used: HashSet<PathBuf> = outputs.iter().filter(|o| keeps(o)).map(|o| o.path.clone()).collect();
    for o in outputs.iter_mut().filter(|o| !keeps(o)) {
        let mut tagged = with_date_tag(&o.path, &o.tag);
        if used.contains(&tagged) {
            tagged = with_date_tag(&o.path, &o.precise_tag);
        }
        let mut candidate = tagged.clone();
        let mut n = 2u32;
        while used.contains(&candidate) {
            candidate = with_date_tag(&tagged, &n.to_string());
            n = n.saturating_add(1);
        }
        used.insert(candidate.clone());
        o.path = candidate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_tag_goes_before_the_extension() {
        let p = Path::new("dir").join("report.final.docx");
        assert_eq!(
            with_date_tag(&p, "2026-11-02 21.30"),
            Path::new("dir").join("report.final (2026-11-02 21.30).docx")
        );
        let p = Path::new("dir").join("README");
        assert_eq!(with_date_tag(&p, "x"), Path::new("dir").join("README (x)"));
    }

    #[test]
    fn date_tag_comes_from_the_archive_name() {
        assert_eq!(date_tag("2026-11-01_183300_I"), "2026-11-01 18.33");
        assert_eq!(date_tag("not a name"), "not a name");
    }

    #[test]
    fn beside_dates_files_but_not_folders() {
        let t = RestoreTarget::BesideOriginal;
        let file = output_path("C/a/report.docx", EntryKind::File, &t, "D");
        assert_eq!(file.file_name().unwrap(), "report (D).docx");
        let dir = output_path("C/a", EntryKind::Dir, &t, "D");
        assert_eq!(dir, keys::path_for("C/a"));
        let under =
            output_path("C/a/r.txt", EntryKind::File, &RestoreTarget::Folder { path: "out".into() }, "D");
        assert_eq!(under, keys::path_under(Path::new("out"), "C/a/r.txt"));
    }

    fn out(path: &str, tag: &str, original: bool) -> Output {
        Output { path: PathBuf::from(path), tag: tag.into(), precise_tag: format!("{tag}.00"), original }
    }

    fn paths(outputs: &[Output]) -> Vec<String> {
        outputs.iter().map(|o| o.path.to_string_lossy().replace('\\', "/")).collect()
    }

    #[test]
    fn versions_sharing_a_path_each_take_their_date() {
        let mut o = vec![out("x/r.txt", "A", false), out("x/r.txt", "B", false), out("x/s.txt", "A", false)];
        make_distinct(&mut o);
        assert_eq!(paths(&o), vec!["x/r (A).txt", "x/r (B).txt", "x/s.txt"]);
    }

    #[test]
    fn a_single_original_keeps_its_name() {
        let mut o = vec![out("x/r.txt", "A", true), out("x/r.txt", "B", false)];
        make_distinct(&mut o);
        assert_eq!(paths(&o), vec!["x/r.txt", "x/r (B).txt"]);

        // Two to the original place: neither may take the plain name.
        let mut o = vec![out("x/r.txt", "A", true), out("x/r.txt", "B", true)];
        make_distinct(&mut o);
        assert_eq!(paths(&o), vec!["x/r (A).txt", "x/r (B).txt"]);
    }

    #[test]
    fn versions_of_the_same_minute_take_the_seconds() {
        let mut o = vec![
            Output {
                path: PathBuf::from(r"C:\r\a.txt"),
                tag: "2026-11-02 13.13".into(),
                precise_tag: "2026-11-02 13.13.05".into(),
                original: false,
            },
            Output {
                path: PathBuf::from(r"C:\r\a.txt"),
                tag: "2026-11-02 13.13".into(),
                precise_tag: "2026-11-02 13.13.40".into(),
                original: false,
            },
        ];
        make_distinct(&mut o);
        assert_eq!(o[0].path, PathBuf::from(r"C:\r\a (2026-11-02 13.13).txt"));
        assert_eq!(o[1].path, PathBuf::from(r"C:\r\a (2026-11-02 13.13.40).txt"));
        assert_eq!(precise_date_tag("2026-11-02_131340_I"), "2026-11-02 13.13.40");
    }

    #[test]
    fn a_taken_dated_name_gets_a_number() {
        let mut o =
            vec![out("x/r (A).txt", "Z", false), out("x/r.txt", "A", false), out("x/r.txt", "A", false)];
        make_distinct(&mut o);
        // The dated name is taken: the seconds first, then a number.
        assert_eq!(paths(&o), vec!["x/r (A).txt", "x/r (A.00).txt", "x/r (A.00) (2).txt"]);
    }

    #[test]
    fn nothing_to_do_without_shared_paths() {
        let mut o = vec![out("x/r.txt", "A", false), out("y/r.txt", "B", true)];
        make_distinct(&mut o);
        assert_eq!(paths(&o), vec!["x/r.txt", "y/r.txt"]);
        make_distinct(&mut []);
    }

    #[test]
    fn request_round_trips_through_json() {
        let r = RestoreRequest {
            items: vec![
                RestoreItem {
                    archive: "2026-10-29_213000_C".into(),
                    key: Some("C/Users/a.txt".into()),
                    target: RestoreTarget::Folder { path: PathBuf::from("out") },
                    overwrite: false,
                },
                RestoreItem {
                    archive: "2026-10-30_213000_I".into(),
                    key: None,
                    target: RestoreTarget::Original,
                    overwrite: true,
                },
            ],
        };
        let text = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<RestoreRequest>(&text).unwrap(), r);
    }

    #[test]
    fn an_old_request_file_is_still_read() {
        let old = r#"{"archive":"2026-10-29_213000_C","keys":["C/a.txt","C/b"],"target":{"to":"beside_original"},"overwrite":true}"#;
        let r: RestoreRequest = serde_json::from_str(old).unwrap();
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.items[1].key.as_deref(), Some("C/b"));
        assert!(r.items.iter().all(|i| i.overwrite && i.target == RestoreTarget::BesideOriginal));

        let everything =
            r#"{"archive":"2026-10-29_213000_C","keys":[],"target":{"to":"original"},"overwrite":false}"#;
        let r: RestoreRequest = serde_json::from_str(everything).unwrap();
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].key, None);
        assert_eq!(r.archives(), vec!["2026-10-29_213000_C"]);
    }
}
