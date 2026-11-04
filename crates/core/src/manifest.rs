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
//!
//! From format 2 a stored file may carry the list of its [`Chunks`]
//! (content-defined pieces named by their hash, see `docs/large-files.md`):
//! either beside a whole copy (complete archives: the chunks can be read
//! as byte ranges of that copy), or alone, in which case the file is the
//! concatenation of the chunks, wherever in the destination they are.

use crate::ArchiveKind;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Current on-disk format; readers reject newer ones.
pub const FORMAT_VERSION: u32 = 2;

/// First format whose archives may store files as chunks. A chain whose
/// complete archive is older does not take chunked files: the next
/// automatic backup starts a new chain instead.
pub const CHUNKS_FORMAT: u32 = 2;

/// Name of a chunk: the BLAKE3 hash of its bytes (keyed with a secret of
/// the destination when it is encrypted, so the hash reveals nothing).
/// Written as 64 lowercase hex digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkId([u8; 32]);

impl ChunkId {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The two hex digits of the first byte: the subfolder of the chunk's
    /// file, so no folder holds more than a few thousand chunks.
    #[must_use]
    pub fn shard(&self) -> String {
        format!("{:02x}", self.0[0])
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkId({self})")
    }
}

/// The text is not 64 hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not a chunk name")]
pub struct NotAChunkId;

impl FromStr for ChunkId {
    type Err = NotAChunkId;

    /// 64 hex digits, either case. Nothing else is accepted: the name
    /// becomes a file name.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = s.as_bytes();
        if b.len() != 64 {
            return Err(NotAChunkId);
        }
        let mut out = [0u8; 32];
        for (slot, &[h, l]) in out.iter_mut().zip(b.as_chunks::<2>().0) {
            let digit = |c: u8| char::from(c).to_digit(16).ok_or(NotAChunkId);
            let (hi, lo) = (digit(h)?, digit(l)?);
            *slot = u8::try_from(hi * 16 + lo).map_err(|_| NotAChunkId)?;
        }
        Ok(Self(out))
    }
}

impl Serialize for ChunkId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ChunkId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// One chunk of a file: its name and length. Written as `["<hex>", size]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(ChunkId, u32)", into = "(ChunkId, u32)")]
pub struct ChunkRef {
    pub id: ChunkId,
    pub size: u32,
}

impl From<(ChunkId, u32)> for ChunkRef {
    fn from((id, size): (ChunkId, u32)) -> Self {
        Self { id, size }
    }
}

impl From<ChunkRef> for (ChunkId, u32) {
    fn from(c: ChunkRef) -> Self {
        (c.id, c.size)
    }
}

/// A stored file's content as a list of chunks, in file order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunks {
    /// The archive also holds the whole file (complete archives): each
    /// chunk can be read from it at the offset given by the sizes of the
    /// chunks before it. Without it, only the chunks hold the content.
    #[serde(default)]
    pub whole: bool,
    pub list: Vec<ChunkRef>,
}

impl Chunks {
    /// Sum of the chunk sizes: the file's size.
    #[must_use]
    pub fn total_size(&self) -> u64 {
        self.list.iter().map(|c| u64::from(c.size)).fold(0u64, u64::saturating_add)
    }

    /// Each chunk with its offset in the file.
    pub fn with_offsets(&self) -> impl Iterator<Item = (u64, &ChunkRef)> {
        self.list.iter().scan(0u64, |offset, c| {
            let at = *offset;
            *offset = offset.saturating_add(u64::from(c.size));
            Some((at, c))
        })
    }
}

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
    /// The content as chunks, when it is stored that way (format 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunks: Option<Chunks>,
    /// Left out by an exclusion added after it was backed up: kept as its
    /// last backup saw it (never stored here), so it is not taken for a
    /// file deleted at the source.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub excluded: bool,
}

impl Entry {
    /// Same content as `other` as far as change detection can tell.
    #[must_use]
    pub fn unchanged_from(&self, other: &Self) -> bool {
        self.kind == other.kind && self.size == other.size && self.mtime_ns == other.mtime_ns
    }

    /// True when this archive holds the content only as chunks (no whole
    /// copy here): restoring it means putting the chunks together.
    #[must_use]
    pub fn chunks_only(&self) -> bool {
        self.stored && self.chunks.as_ref().is_some_and(|c| !c.whole)
    }

    /// True when this archive holds a whole copy of the content (a zip, a
    /// plain copy or an encrypted zip), with or without a chunk list.
    #[must_use]
    pub fn whole_copy(&self) -> bool {
        self.stored && self.chunks.as_ref().is_none_or(|c| c.whole)
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
    /// Format it was written in ([`FORMAT_VERSION`] for a new one).
    pub format: u32,
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
    /// A new manifest in the current format.
    #[must_use]
    pub fn new(kind: ArchiveKind, entries: Vec<Entry>) -> Self {
        Self { format: FORMAT_VERSION, kind, entries }
    }

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
        // A file stored as chunks needs a reader that knows them.
        let needs_chunks = self.entries.iter().any(|e| e.chunks.is_some());
        let format = if needs_chunks { self.format.max(CHUNKS_FORMAT) } else { self.format };
        let header = ManifestHeader { format, kind: self.kind.into(), entries: self.entries.len() as u64 };
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
            // Restores write where the keys say: only keys a path gives.
            if let Err(err) = crate::keys::check(&e.key) {
                return Err(ManifestError::Line { line: i + 1, message: err.to_string() });
            }
            // Restores trust the list: it must describe the whole file.
            if e.chunks.as_ref().is_some_and(|c| c.total_size() != e.size) {
                let message = "the chunk sizes do not add up to the file size".to_owned();
                return Err(ManifestError::Line { line: i + 1, message });
            }
            entries.push(e);
        }
        if entries.len() as u64 != header.entries {
            return Err(ManifestError::Truncated { found: entries.len() as u64, expected: header.entries });
        }
        let kind = match header.kind {
            ManifestKind::Complete => ArchiveKind::Complete,
            ManifestKind::Incremental => ArchiveKind::Incremental,
        };
        let mut m = Self { format: header.format, kind, entries };
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
            chunks: None,
            excluded: false,
        }
    }

    fn id(byte: u8) -> ChunkId {
        ChunkId::from_bytes([byte; 32])
    }

    #[test]
    fn manifest_round_trips() {
        let mut m = Manifest::new(
            ArchiveKind::Incremental,
            vec![file("C/b.txt", true), file("C/a \"q\".txt", false)],
        );
        m.normalize();
        assert_eq!(Manifest::from_text(&m.to_text()).unwrap(), m);
    }

    #[test]
    fn chunk_lists_round_trip_compactly() {
        let mut e = file("C/big.vhdx", true);
        e.chunks = Some(Chunks {
            whole: false,
            list: vec![ChunkRef { id: id(0xab), size: 2 }, ChunkRef { id: id(1), size: 1 }],
        });
        let m = Manifest::new(ArchiveKind::Incremental, vec![e]);
        let text = m.to_text();
        assert!(text.contains(&format!("[\"{}\",2]", "ab".repeat(32))), "{text}");
        let back = Manifest::from_text(&text).unwrap();
        assert_eq!(back, m);
        assert!(back.entries[0].chunks_only() && !back.entries[0].whole_copy());
        let offsets: Vec<u64> =
            back.entries[0].chunks.as_ref().unwrap().with_offsets().map(|(o, _)| o).collect();
        assert_eq!(offsets, vec![0, 2]);
    }

    #[test]
    fn a_chunk_list_that_does_not_match_the_size_is_refused() {
        let mut e = file("C/big", true);
        e.chunks = Some(Chunks { whole: true, list: vec![ChunkRef { id: id(1), size: 2 }] });
        let text = Manifest::new(ArchiveKind::Complete, vec![e]).to_text();
        assert!(matches!(Manifest::from_text(&text), Err(ManifestError::Line { line: 2, .. })));
    }

    #[test]
    fn chunk_names_are_strict_hex() {
        let hex = "0F".repeat(32);
        let parsed: ChunkId = hex.parse().unwrap();
        assert_eq!(parsed.to_string(), hex.to_lowercase());
        assert_eq!(parsed.shard(), "0f");
        let bad =
            ["".to_owned(), "zz".to_owned(), "0".repeat(63), "0".repeat(65), format!("..{}", "0".repeat(62))];
        for b in bad {
            assert!(b.parse::<ChunkId>().is_err(), "{b}");
        }
    }

    #[test]
    fn format_one_manifests_still_read_and_keep_their_format() {
        let text = "{\"format\":1,\"kind\":\"complete\",\"entries\":1}\n\
                    {\"key\":\"C/a\",\"kind\":\"file\",\"size\":3,\"stored\":true}\n";
        let m = Manifest::from_text(text).unwrap();
        assert_eq!(m.format, 1);
        assert!(m.entries[0].whole_copy());
        assert!(m.to_text().starts_with("{\"format\":1,"));
    }

    #[test]
    fn truncated_manifest_is_detected() {
        let m = Manifest::new(ArchiveKind::Complete, vec![file("C/a", true), file("C/b", true)]);
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
        let mut m = Manifest::new(
            ArchiveKind::Complete,
            vec![file("C/a", false), file("C/b", true), file("C/a", true)],
        );
        m.normalize();
        assert_eq!(m.entries.len(), 2);
        assert!(m.get("C/a").unwrap().stored);
        assert!(m.get("C/zz").is_none());
    }
}
