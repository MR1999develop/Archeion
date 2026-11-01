//! Manifest of an archive: every file and folder present at that time.
//!
//! Stored as `manifest.jsonl` in the archive folder: a header line, then one
//! JSON object per entry, sorted by key. JSON lines keep it readable with any
//! text editor and let a damaged line be reported precisely.
//!
//! `stored` tells whether this archive holds the entry's content. When it
//! does not, the content is in the newest *earlier* archive that stores the
//! same key. Merging never has to rewrite later manifests: the merged
//! archive replaces the group and stores the newest content of the group.

use crate::ArchiveKind;
use serde::{Deserialize, Serialize};

/// Current on-disk format; readers reject newer ones.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    File,
    Dir,
}

/// One file or folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// See [`crate::keys`].
    pub key: String,
    pub kind: EntryKind,
    /// Bytes; 0 for folders.
    #[serde(default)]
    pub size: u64,
    /// Modification time, nanoseconds since the Unix epoch.
    #[serde(default)]
    pub mtime_ns: i64,
    /// BLAKE3 of the content, hex; absent for folders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// This archive holds the content.
    #[serde(default)]
    pub stored: bool,
    /// Where the content is stored in the archive folder, when its name is
    /// not the file's own (encrypted destinations): for recovery by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl Entry {
    /// Same content as `other` as far as change detection can tell.
    #[must_use]
    pub fn unchanged_from(&self, other: &Self) -> bool {
        self.kind == other.kind && self.size == other.size && self.mtime_ns == other.mtime_ns
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestHeader {
    pub format: u32,
    pub kind: ManifestKind,
    /// Number of entries that follow, to detect a truncated file.
    pub entries: u64,
}

/// Serialized form of [`ArchiveKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestKind {
    Complete,
    Incremental,
}

impl From<ArchiveKind> for ManifestKind {
    fn from(k: ArchiveKind) -> Self {
        match k {
            ArchiveKind::Complete => Self::Complete,
            ArchiveKind::Incremental => Self::Incremental,
        }
    }
}

/// Full manifest in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub kind: ArchiveKind,
    /// Sorted by key, no duplicates.
    pub entries: Vec<Entry>,
}

/// The manifest file cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    #[error("the manifest is empty")]
    Empty,
    #[error("line {line}: {message}")]
    Line { line: usize, message: String },
    #[error("format {0} is newer than this version of Archeion")]
    TooNew(u32),
    #[error("the manifest is truncated: {found} of {expected} entries")]
    Truncated { found: u64, expected: u64 },
}

impl Manifest {
    /// Sorts the entries and removes duplicate keys (last wins).
    pub fn normalize(&mut self) {
        self.entries.sort_by(|a, b| a.key.cmp(&b.key));
        self.entries.dedup_by(|later, earlier| {
            if later.key == earlier.key {
                std::mem::swap(later, earlier);
                true
            } else {
                false
            }
        });
    }

    /// Entry by key (entries must be sorted).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.binary_search_by(|e| e.key.as_str().cmp(key)).ok().and_then(|i| self.entries.get(i))
    }

    /// The `manifest.jsonl` text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let header = ManifestHeader {
            format: FORMAT_VERSION,
            kind: self.kind.into(),
            entries: self.entries.len() as u64,
        };
        let mut out = serde_json::to_string(&header).expect("header serializes");
        out.push('\n');
        for e in &self.entries {
            out.push_str(&serde_json::to_string(e).expect("entry serializes"));
            out.push('\n');
        }
        out
    }

    /// Parses `manifest.jsonl`.
    pub fn from_text(text: &str) -> Result<Self, ManifestError> {
        let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
        let (_, first) = lines.next().ok_or(ManifestError::Empty)?;
        let header: ManifestHeader = serde_json::from_str(first)
            .map_err(|e| ManifestError::Line { line: 1, message: e.to_string() })?;
        if header.format > FORMAT_VERSION {
            return Err(ManifestError::TooNew(header.format));
        }
        let mut entries = Vec::with_capacity(usize::try_from(header.entries).unwrap_or(0).min(1 << 20));
        for (i, line) in lines {
            let e: Entry = serde_json::from_str(line)
                .map_err(|e| ManifestError::Line { line: i + 1, message: e.to_string() })?;
            entries.push(e);
        }
        if entries.len() as u64 != header.entries {
            return Err(ManifestError::Truncated { found: entries.len() as u64, expected: header.entries });
        }
        let kind = match header.kind {
            ManifestKind::Complete => ArchiveKind::Complete,
            ManifestKind::Incremental => ArchiveKind::Incremental,
        };
        let mut m = Self { kind, entries };
        m.normalize();
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(key: &str, stored: bool) -> Entry {
        Entry {
            key: key.into(),
            kind: EntryKind::File,
            size: 3,
            mtime_ns: 1_700_000_000_000_000_000,
            hash: Some("ab".into()),
            stored,
            blob: None,
        }
    }

    #[test]
    fn manifest_round_trips() {
        let mut m = Manifest {
            kind: ArchiveKind::Incremental,
            entries: vec![file("C/b.txt", true), file("C/a \"q\".txt", false)],
        };
        m.normalize();
        assert_eq!(Manifest::from_text(&m.to_text()).unwrap(), m);
    }

    #[test]
    fn truncated_manifest_is_detected() {
        let m = Manifest { kind: ArchiveKind::Complete, entries: vec![file("C/a", true), file("C/b", true)] };
        let text = m.to_text();
        let cut: String = text.lines().take(2).map(|l| format!("{l}\n")).collect();
        assert_eq!(Manifest::from_text(&cut), Err(ManifestError::Truncated { found: 1, expected: 2 }));
    }

    #[test]
    fn damaged_line_is_located() {
        let text = "{\"format\":1,\"kind\":\"complete\",\"entries\":1}\n{oops\n";
        assert!(matches!(Manifest::from_text(text), Err(ManifestError::Line { line: 2, .. })));
    }

    #[test]
    fn newer_format_is_rejected() {
        let text = "{\"format\":99,\"kind\":\"complete\",\"entries\":0}\n";
        assert_eq!(Manifest::from_text(text), Err(ManifestError::TooNew(99)));
    }

    #[test]
    fn lookup_and_dedup_keep_the_last_entry() {
        let mut m = Manifest {
            kind: ArchiveKind::Complete,
            entries: vec![file("C/a", false), file("C/b", true), file("C/a", true)],
        };
        m.normalize();
        assert_eq!(m.entries.len(), 2);
        assert!(m.get("C/a").unwrap().stored);
        assert!(m.get("C/zz").is_none());
    }
}
