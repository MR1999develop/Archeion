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

    /// Files whose name contains `query` (case-insensitive), in key order,
    /// at most `limit`.
    pub fn search(&self, query: &str, limit: usize) -> Vec<usize> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.is_dir())
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
    fn search_matches_file_names_only() {
        let t = Tree::build(catalog(vec![
            entry("C/Report", EntryKind::Dir, vec![(0, 2)]),
            entry("C/Report/notes.txt", EntryKind::File, vec![(0, 2)]),
            entry("C/Report/REPORT.docx", EntryKind::File, vec![(0, 2)]),
        ]));
        let hits: Vec<&str> = t.search(" report ", 10).iter().map(|&i| t.nodes[i].key()).collect();
        assert_eq!(hits, vec!["C/Report/REPORT.docx"]);
        assert!(t.search("", 10).is_empty());
        assert_eq!(t.search("t", 1).len(), 1);
    }

    #[test]
    fn empty_catalog_gives_an_empty_tree() {
        let t = Tree::build(Catalog::default());
        assert!(t.roots.is_empty());
    }
}
