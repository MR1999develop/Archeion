//! Folder tree of one archive, built once from its manifest.
//!
//! Nodes live in a `Vec` and refer to each other by index. The roots are the
//! backed-up folders (entries whose parent is not in the manifest), shown
//! with their full original path. Every node knows whether this archive
//! saved it: a file whose content it holds, a folder that leads to one.

use archeion_core::{Entry, EntryKind, keys};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Node {
    /// Last path segment, or the full original path for a root.
    pub label: String,
    pub key: String,
    pub kind: EntryKind,
    pub size: u64,
    /// Saved by this archive: a file whose content it holds (new or changed
    /// since the previous backup, or every file of a complete one), or a
    /// folder containing such a file.
    pub stored: bool,
    /// Folders first, then files, each by name.
    pub children: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    pub files: usize,
    /// Files whose content this archive holds.
    pub stored_files: usize,
}

impl Tree {
    /// `entries` sorted by key, as in a manifest.
    pub fn build(entries: &[Entry]) -> Self {
        let mut tree = Self::default();
        let mut saved_folders: HashSet<&str> = HashSet::new();
        for e in entries.iter().filter(|e| e.kind == EntryKind::File && e.stored) {
            let mut key = e.key.as_str();
            while let Some((parent, _)) = key.rsplit_once('/') {
                if !saved_folders.insert(parent) {
                    break; // its ancestors are in already
                }
                key = parent;
            }
        }
        let mut index: HashMap<&str, usize> = HashMap::with_capacity(entries.len());
        for e in entries {
            let parent = e.key.rsplit_once('/').map(|(p, _)| p);
            let parent_idx = parent.and_then(|p| index.get(p).copied());
            let label = match parent_idx {
                Some(_) => e.key.rsplit('/').next().unwrap_or(&e.key).to_owned(),
                None => keys::path_for(&e.key).display().to_string(),
            };
            let idx = tree.nodes.len();
            tree.nodes.push(Node {
                label,
                key: e.key.clone(),
                kind: e.kind,
                size: e.size,
                stored: match e.kind {
                    EntryKind::File => e.stored,
                    EntryKind::Dir => saved_folders.contains(e.key.as_str()),
                },
                children: Vec::new(),
            });
            if e.kind == EntryKind::File {
                tree.files += 1;
                tree.stored_files += usize::from(e.stored);
            }
            index.insert(e.key.as_str(), idx);
            match parent_idx.and_then(|p| tree.nodes.get_mut(p)) {
                Some(p) => p.children.push(idx),
                None => tree.roots.push(idx),
            }
        }
        let order: Vec<(bool, String)> =
            tree.nodes.iter().map(|n| (n.kind == EntryKind::File, n.label.to_lowercase())).collect();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, kind: EntryKind) -> Entry {
        Entry { key: key.into(), kind, size: 1, mtime_ns: 0, hash: None, stored: true, blob: None }
    }

    #[test]
    fn roots_are_the_backed_up_folders_and_folders_come_first() {
        let mut entries = vec![
            entry("C/Users/m/Docs", EntryKind::Dir),
            entry("C/Users/m/Docs/b.txt", EntryKind::File),
            entry("C/Users/m/Docs/A.txt", EntryKind::File),
            entry("C/Users/m/Docs/sub", EntryKind::Dir),
            entry("C/Users/m/Docs/sub/c.txt", EntryKind::File),
            entry("D/Photos", EntryKind::Dir),
        ];
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        let t = Tree::build(&entries);
        assert_eq!(t.roots.len(), 2);
        assert_eq!(t.files, 3);
        let docs = t.node(t.roots[0]).unwrap();
        assert_eq!(docs.key, "C/Users/m/Docs");
        let labels: Vec<&str> = docs.children.iter().map(|&i| t.nodes[i].label.as_str()).collect();
        assert_eq!(labels, vec!["sub", "A.txt", "b.txt"]);
    }

    #[test]
    fn saved_files_and_the_folders_leading_to_them_are_marked() {
        let mut entries = vec![
            entry("C/Docs", EntryKind::Dir),
            entry("C/Docs/new.txt", EntryKind::File),
            entry("C/Docs/old", EntryKind::Dir),
            entry("C/Docs/old/same.txt", EntryKind::File),
            entry("C/Docs/sub", EntryKind::Dir),
            entry("C/Docs/sub/deep", EntryKind::Dir),
            entry("C/Docs/sub/deep/changed.txt", EntryKind::File),
        ];
        entries[3].stored = false;
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        let t = Tree::build(&entries);
        assert_eq!((t.files, t.stored_files), (3, 2));
        let saved: Vec<&str> = t.nodes.iter().filter(|n| n.stored).map(|n| n.key.as_str()).collect();
        assert_eq!(
            saved,
            vec!["C/Docs", "C/Docs/new.txt", "C/Docs/sub", "C/Docs/sub/deep", "C/Docs/sub/deep/changed.txt"]
        );
        let grey: Vec<&str> = t.nodes.iter().filter(|n| !n.stored).map(|n| n.key.as_str()).collect();
        assert_eq!(grey, vec!["C/Docs/old", "C/Docs/old/same.txt"], "unchanged parts stay, greyed");
    }

    #[test]
    fn empty_manifest_gives_an_empty_tree() {
        let t = Tree::build(&[]);
        assert!(t.roots.is_empty());
    }
}
