//! Folder tree of every file and folder a profile's backups ever listed,
//! built once from the catalog.
//!
//! Nodes live in a `Vec`, in key order (so a key is found by binary
//! search), and refer to each other by index. The roots are the backed-up
//! folders (entries whose parent is not listed), shown with their full
//! original path. A node not listed by the newest backup was deleted at
//! the source since then.

use archeion_core::{EntryKind, keys};
use archeion_engine::{Catalog, CatalogArchive, CatalogEntry};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Node {
    /// Last path segment, or the full original path for a root.
    pub label: String,
    /// Key, kind, size as last listed, and the backups listing it.
    pub entry: CatalogEntry,
    /// Not listed by the newest backup.
    pub deleted: bool,
    /// Folders first, then files, each by name.
    pub children: Vec<usize>,
}

impl Node {
    pub fn key(&self) -> &str {
        &self.entry.key
    }

    pub fn is_dir(&self) -> bool {
        self.entry.kind == EntryKind::Dir
    }
}

/// Something ticked in the selection, as the tree needs it.
#[derive(Debug, Clone, Copy)]
pub struct Ticked<'a> {
    pub key: &'a str,
    /// For a folder, the backup it comes back as.
    pub folder_archive: Option<&'a str>,
    /// For a folder, the keys unticked inside it.
    pub exclude: &'a [String],
}

/// How the tick box of a node shows the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    Off,
    /// Ticked itself, with nothing left out.
    On,
    /// A folder with something ticked inside it, or a ticked folder with
    /// something unticked inside it.
    Partial,
    /// A folder not ticked itself whose every item is ticked one by one:
    /// shown ticked, and clicking it unticks them all.
    Full,
    /// Inside a ticked folder, in the backup that folder comes back as:
    /// restored with it. `folder` is the nearest ticked folder.
    InFolder {
        folder: usize,
    },
    /// Inside a ticked folder and unticked: left out of it.
    LeftOut {
        folder: usize,
    },
    /// Inside a ticked folder but not in the backup it comes back as: the
    /// folder does not bring it back.
    NotInFolderDate {
        folder: usize,
    },
}

#[derive(Debug, Default)]
pub struct Tree {
    /// Sorted by key.
    nodes: Vec<Node>,
    pub roots: Vec<usize>,
    /// The backups, oldest first.
    pub archives: Vec<CatalogArchive>,
    pub files: usize,
    pub deleted_files: usize,
}

impl Tree {
    pub fn build(catalog: Catalog) -> Self {
        let count = catalog.archives.len();
        let mut tree = Self { archives: catalog.archives, ..Self::default() };
        let mut index: HashMap<String, usize> = HashMap::with_capacity(catalog.entries.len());
        for e in catalog.entries {
            let deleted = e.deleted(count);
            let parent = e.key.rsplit_once('/').map(|(p, _)| p);
            let parent_idx = parent.and_then(|p| index.get(p).copied());
            let label = match parent_idx {
                Some(_) => e.key.rsplit('/').next().unwrap_or(&e.key).to_owned(),
                None => keys::path_for(&e.key).display().to_string(),
            };
            let idx = tree.nodes.len();
            if e.kind == EntryKind::File {
                tree.files += 1;
                tree.deleted_files += usize::from(deleted);
            } else {
                index.insert(e.key.clone(), idx);
            }
            tree.nodes.push(Node { label, entry: e, deleted, children: Vec::new() });
            match parent_idx.and_then(|p| tree.nodes.get_mut(p)) {
                Some(p) => p.children.push(idx),
                None => tree.roots.push(idx),
            }
        }
        let order: Vec<(bool, String)> =
            tree.nodes.iter().map(|n| (!n.is_dir(), n.label.to_lowercase())).collect();
        let by = |a: &usize, b: &usize| order.get(*a).cmp(&order.get(*b));
        for n in &mut tree.nodes {
            n.children.sort_by(by);
        }
        tree.roots.sort_by(by);
        tree
    }

    pub fn node(&self, idx: usize) -> Option<&Node> {
        self.nodes.get(idx)
    }

    /// The node of `key`.
    pub fn find(&self, key: &str) -> Option<&Node> {
        let idx = self.nodes.binary_search_by(|n| n.key().cmp(key)).ok()?;
        self.nodes.get(idx)
    }

    /// The backups listing `node`, newest first.
    pub fn dates(&self, node: &Node) -> Vec<&CatalogArchive> {
        node.entry.archives_newest_first().filter_map(|i| self.archives.get(i)).collect()
    }

    /// How `node` shows `ticked`. Its own tick wins (partial when something
    /// inside is left out); then the nearest ticked folder holding it;
    /// then, for a folder, what is ticked inside: all of it or some.
    pub fn tick(&self, node: &Node, ticked: &[Ticked<'_>]) -> Tick {
        let key = node.key();
        // Left out of a folder, but ticked on its own (another version):
        // nothing is missing there.
        let missing = |x: &String| !ticked.iter().any(|t| t.key == x.as_str() && t.folder_archive.is_none());
        if let Some(own) = ticked.iter().find(|t| t.key == key) {
            return if own.exclude.iter().any(missing) { Tick::Partial } else { Tick::On };
        }
        let holder = ticked
            .iter()
            .filter(|t| t.folder_archive.is_some() && keys::is_within(key, t.key))
            .max_by_key(|t| t.key.len());
        if let Some(h) = holder
            && let Ok(folder) = self.nodes.binary_search_by(|n| n.key().cmp(h.key))
        {
            if h.exclude.iter().any(|x| keys::is_within(key, x)) {
                return Tick::LeftOut { folder };
            }
            let listed =
                self.archives.iter().position(|a| Some(a.name.as_str()) == h.folder_archive).is_some_and(
                    |pos| node.entry.present.iter().any(|&(first, last)| (first..=last).contains(&pos)),
                );
            if !listed {
                return Tick::NotInFolderDate { folder };
            }
            if node.is_dir() && h.exclude.iter().filter(|x| missing(x)).any(|x| keys::is_within(x, key)) {
                return Tick::Partial;
            }
            return Tick::InFolder { folder };
        }
        if node.is_dir() && ticked.iter().any(|t| t.key != key && keys::is_within(t.key, key)) {
            return if self.all_ticked(node, ticked) { Tick::Full } else { Tick::Partial };
        }
        Tick::Off
    }

    /// Every item of the folder `node` is ticked, or full itself. Stops at
    /// the first one that is not, so a partial folder costs little.
    fn all_ticked(&self, node: &Node, ticked: &[Ticked<'_>]) -> bool {
        !node.children.is_empty()
            && node
                .children
                .iter()
                .filter_map(|&c| self.nodes.get(c))
                .all(|c| matches!(self.tick(c, ticked), Tick::On | Tick::Full | Tick::InFolder { .. }))
    }

    /// Ticks `key` again inside a folder whose exclusions are `exclude`:
    /// when it is left out because a folder above it is, that folder's
    /// exclusion becomes one for each other item on the way down, so only
    /// the path to `key` comes back.
    pub fn include(&self, exclude: &mut Vec<String>, key: &str) {
        let Some(pos) = exclude.iter().position(|x| keys::is_within(key, x)) else { return };
        let above = exclude.swap_remove(pos);
        let Some(mut node) = self.find(&above) else { return };
        while node.key() != key {
            let next = node
                .children
                .iter()
                .filter_map(|&c| self.nodes.get(c))
                .find(|c| keys::is_within(key, c.key()));
            for c in node.children.iter().filter_map(|&c| self.nodes.get(c)) {
                if next.is_none_or(|n| n.key() != c.key()) {
                    exclude.push(c.key().to_owned());
                }
            }
            match next {
                Some(n) => node = n,
                None => break,
            }
        }
    }

    /// Files and folders whose name contains `query`, whatever the case,
    /// in key order (a folder before what it holds), at most `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.label.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .take(limit)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, kind: EntryKind, present: Vec<(usize, usize)>) -> CatalogEntry {
        CatalogEntry { key: key.into(), kind, size: 1, present }
    }

    fn catalog(mut entries: Vec<CatalogEntry>) -> Catalog {
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        let archives = (0..3)
            .map(|i| CatalogArchive { name: format!("a{i}"), label: format!("2026-11-0{} 10:00", i + 1) })
            .collect();
        Catalog { archives, entries }
    }

    #[test]
    fn roots_are_the_backed_up_folders_and_folders_come_first() {
        let t = Tree::build(catalog(vec![
            entry("C/Users/m/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Users/m/Docs/b.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Users/m/Docs/A.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Users/m/Docs/sub", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Users/m/Docs/sub/c.txt", EntryKind::File, vec![(0, 2)]),
            entry("D/Photos", EntryKind::Dir, vec![(0, 2)]),
        ]));
        assert_eq!(t.roots.len(), 2);
        assert_eq!(t.files, 3);
        let docs = t.node(t.roots[0]).unwrap();
        assert_eq!(docs.key(), "C/Users/m/Docs");
        let labels: Vec<&str> = docs.children.iter().map(|&i| t.nodes[i].label.as_str()).collect();
        assert_eq!(labels, vec!["sub", "A.txt", "b.txt"]);
    }

    #[test]
    fn files_missing_from_the_newest_backup_are_deleted() {
        let t = Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/gone.txt", EntryKind::File, vec![(0, 1)]),
            entry("C/Docs/back.txt", EntryKind::File, vec![(0, 0), (2, 2)]),
        ]));
        assert_eq!((t.files, t.deleted_files), (2, 1));
        assert!(t.find("C/Docs/gone.txt").unwrap().deleted);
        let back = t.find("C/Docs/back.txt").unwrap();
        assert!(!back.deleted);
        let dates: Vec<&str> = t.dates(back).iter().map(|a| a.name.as_str()).collect();
        assert_eq!(dates, vec!["a2", "a0"], "newest first, only where listed");
        assert!(t.find("C/Docs/none.txt").is_none());
    }

    #[test]
    fn search_matches_file_and_folder_names_whatever_the_case() {
        let t = Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/Report", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/Report/notes.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/Report/REPORT.docx", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/JPEG", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/JPEG/Città.jpg", EntryKind::File, vec![(0, 2)]),
        ]));
        let hits = |q: &str| -> Vec<&str> { t.search(q, 10).iter().map(|&i| t.nodes[i].key()).collect() };
        assert_eq!(hits(" report "), vec!["C/Docs/Report", "C/Docs/Report/REPORT.docx"]);
        assert_eq!(hits("Jpeg"), vec!["C/Docs/JPEG"], "a folder, any case");
        assert_eq!(hits("CITTÀ"), vec!["C/Docs/JPEG/Città.jpg"], "accented capitals");
        assert!(t.search("", 10).is_empty());
        assert_eq!(t.search("t", 1).len(), 1);
    }

    fn tick_of(t: &Tree, key: &str, ticked: &[Ticked<'_>]) -> Tick {
        t.tick(t.find(key).unwrap(), ticked)
    }

    fn folder<'a>(key: &'a str, archive: &'a str) -> Ticked<'a> {
        Ticked { key, folder_archive: Some(archive), exclude: &[] }
    }

    fn file(key: &str) -> Ticked<'_> {
        Ticked { key, folder_archive: None, exclude: &[] }
    }

    fn idx(t: &Tree, key: &str) -> usize {
        t.nodes.binary_search_by(|n| n.key().cmp(key)).unwrap()
    }

    fn docs_tree() -> Tree {
        Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/a.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/sub", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub/b.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/sub/deep", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub/deep/c.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/sub/deep/d.txt", EntryKind::File, vec![(0, 2)]),
        ]))
    }

    #[test]
    fn an_unticked_file_inside_a_ticked_folder_is_left_out_and_the_folders_fill() {
        let t = docs_tree();
        let docs = idx(&t, "C/Docs");
        let exclude = vec!["C/Docs/sub/b.txt".to_owned()];
        let ticked = [Ticked { key: "C/Docs", folder_archive: Some("a2"), exclude: &exclude }];
        assert_eq!(tick_of(&t, "C/Docs", &ticked), Tick::Partial, "the ticked folder itself");
        assert_eq!(tick_of(&t, "C/Docs/sub", &ticked), Tick::Partial, "a folder holding it");
        assert_eq!(tick_of(&t, "C/Docs/sub/b.txt", &ticked), Tick::LeftOut { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs/sub/deep", &ticked), Tick::InFolder { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs/a.txt", &ticked), Tick::InFolder { folder: docs });

        // A left-out folder leaves out what it holds.
        let exclude = vec!["C/Docs/sub".to_owned()];
        let ticked = [Ticked { key: "C/Docs", folder_archive: Some("a2"), exclude: &exclude }];
        assert_eq!(tick_of(&t, "C/Docs/sub/deep/c.txt", &ticked), Tick::LeftOut { folder: docs });
    }

    #[test]
    fn ticking_again_inside_a_left_out_folder_brings_back_only_that_path() {
        let t = docs_tree();
        let mut exclude = vec!["C/Docs/a.txt".to_owned(), "C/Docs/sub".to_owned()];
        t.include(&mut exclude, "C/Docs/sub/deep/c.txt");
        exclude.sort();
        assert_eq!(exclude, vec!["C/Docs/a.txt", "C/Docs/sub/b.txt", "C/Docs/sub/deep/d.txt"]);

        // Left out on its own: the exclusion goes.
        t.include(&mut exclude, "C/Docs/a.txt");
        assert_eq!(exclude.len(), 2);
        assert!(!exclude.contains(&"C/Docs/a.txt".to_owned()));

        // Not left out: nothing changes.
        t.include(&mut exclude, "C/Docs/sub/deep/c.txt");
        assert_eq!(exclude.len(), 2);
    }

    #[test]
    fn a_ticked_folder_ticks_what_it_held_at_its_date() {
        let t = Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/old.txt", EntryKind::File, vec![(0, 0)]),
            entry("C/Docs/sub", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub/a.txt", EntryKind::File, vec![(1, 2)]),
            entry("C/Docs2/b.txt", EntryKind::File, vec![(0, 2)]),
        ]));
        let docs = t.nodes.binary_search_by(|n| n.key().cmp("C/Docs")).unwrap();
        let ticked = [folder("C/Docs", "a2")];
        assert_eq!(tick_of(&t, "C/Docs", &ticked), Tick::On);
        assert_eq!(tick_of(&t, "C/Docs/sub", &ticked), Tick::InFolder { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs/sub/a.txt", &ticked), Tick::InFolder { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs/old.txt", &ticked), Tick::NotInFolderDate { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs2/b.txt", &ticked), Tick::Off, "a sibling sharing the prefix");

        // At the oldest date, old.txt is there and a.txt is not.
        let ticked = [folder("C/Docs", "a0")];
        assert_eq!(tick_of(&t, "C/Docs/old.txt", &ticked), Tick::InFolder { folder: docs });
        assert_eq!(tick_of(&t, "C/Docs/sub/a.txt", &ticked), Tick::NotInFolderDate { folder: docs });
    }

    #[test]
    fn the_nearest_ticked_folder_decides() {
        let t = Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub/a.txt", EntryKind::File, vec![(2, 2)]),
        ]));
        let sub = t.nodes.binary_search_by(|n| n.key().cmp("C/Docs/sub")).unwrap();
        let ticked = [folder("C/Docs", "a0"), folder("C/Docs/sub", "a2")];
        assert_eq!(tick_of(&t, "C/Docs/sub", &ticked), Tick::On);
        assert_eq!(tick_of(&t, "C/Docs/sub/a.txt", &ticked), Tick::InFolder { folder: sub });
    }

    #[test]
    fn folders_above_a_ticked_file_are_partial() {
        let t = Tree::build(catalog(vec![
            entry("C/Docs", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Docs/sub/a.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Docs/z.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Other", EntryKind::Dir, vec![(0, 2)]),
        ]));
        let ticked = [file("C/Docs/sub/a.txt")];
        assert_eq!(tick_of(&t, "C/Docs", &ticked), Tick::Partial);
        assert_eq!(tick_of(&t, "C/Docs/sub", &ticked), Tick::Full, "its only file is ticked");
        assert_eq!(tick_of(&t, "C/Docs/sub/a.txt", &ticked), Tick::On);
        assert_eq!(tick_of(&t, "C/Docs/z.txt", &ticked), Tick::Off, "a file is never partial");
        assert_eq!(tick_of(&t, "C/Other", &ticked), Tick::Off);
        assert_eq!(tick_of(&t, "C/Docs", &[]), Tick::Off);
    }

    #[test]
    fn a_folder_whose_items_are_all_ticked_one_by_one_is_full() {
        let t = docs_tree();
        let mut ticked = vec![file("C/Docs/a.txt"), file("C/Docs/sub/b.txt"), file("C/Docs/sub/deep/c.txt")];
        assert_eq!(tick_of(&t, "C/Docs/sub/deep", &ticked), Tick::Partial);
        assert_eq!(tick_of(&t, "C/Docs", &ticked), Tick::Partial);
        ticked.push(file("C/Docs/sub/deep/d.txt"));
        assert_eq!(tick_of(&t, "C/Docs/sub/deep", &ticked), Tick::Full);
        assert_eq!(tick_of(&t, "C/Docs/sub", &ticked), Tick::Full, "full all the way up");
        assert_eq!(tick_of(&t, "C/Docs", &ticked), Tick::Full);
        let with_folder = [file("C/Docs/a.txt"), folder("C/Docs/sub", "a2")];
        assert_eq!(tick_of(&t, "C/Docs", &with_folder), Tick::Full, "a ticked folder counts");
        let partly = [
            file("C/Docs/a.txt"),
            Ticked { exclude: &["C/Docs/sub/b.txt".into()], ..folder("C/Docs/sub", "a2") },
        ];
        assert_eq!(tick_of(&t, "C/Docs", &partly), Tick::Partial, "something left out inside");
    }

    #[test]
    fn empty_catalog_gives_an_empty_tree() {
        let t = Tree::build(Catalog::default());
        assert!(t.roots.is_empty());
    }
}
