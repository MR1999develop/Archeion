//! Chunk files, and where each chunk of a destination can be read.
//!
//! A file saved as chunks (see [`archeion_core::large_files`]) is listed in
//! the manifest as the names and sizes of its chunks. A chunk is read from
//! either place:
//!
//! - a **chunk file** `chunks/<ab>/<name>` in some archive folder, shaped
//!   like the destination's other files: a plain file, a zip holding the
//!   entry `<name>`, or an AES-256 zip holding the entry `data`
//!   (encrypted destinations, openable with the recovery key);
//! - a **byte range** of a whole copy that a complete archive keeps
//!   together with the file's chunk list.
//!
//! Chunks are named by their content, so any copy of a chunk anywhere in
//! the destination is as good as another, and the rule every operation
//! keeps is simple: *every chunk listed by a manifest of the destination
//! can be read from at least one archive of the destination*. Backups add
//! a chunk file only for chunks found nowhere; merges keep a member's chunk
//! file only when a remaining manifest lists the chunk and no remaining
//! archive holds it (see [`crate::merge`]).

use crate::{
    EngineError, Result,
    container::Reader,
    crypto::{Keys, aes_options, zip_error},
};
use archeion_core::{ChunkId, ChunkRef, Container, Manifest};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Folder of the chunk files in an archive folder.
pub(crate) const CHUNKS_DIR: &str = "chunks";
/// Entry of an encrypted chunk's zip, like the entry of an encrypted file.
const VAULT_ENTRY: &str = "data";
/// Suffix of a chunk file being written.
const PARTIAL_SUFFIX: &str = ".part";
/// Longest chunk read back, whatever a damaged zip claims: well above the
/// largest chunk the chunker makes (4 MiB).
const MAX_CHUNK_BYTES: u64 = 64 * 1024 * 1024;

/// How the chunk files of a destination are written and named: like its
/// other files.
#[derive(Debug, Clone)]
pub(crate) enum Form {
    Plain,
    Zip,
    Vault(Keys),
}

impl Form {
    /// The form of a destination using `container`, encrypted with `keys`.
    pub(crate) fn of(container: Container, keys: Option<&Keys>) -> Self {
        match (keys, container) {
            (Some(k), _) => Self::Vault(k.clone()),
            (None, Container::ZipPerFile) => Self::Zip,
            (None, Container::Folder) => Self::Plain,
        }
    }

    /// Name of a chunk whose content is `data`.
    pub(crate) fn id_of(&self, data: &[u8]) -> ChunkId {
        match self {
            Self::Vault(keys) => ChunkId::from_bytes(keys.chunk_id(data)),
            Self::Plain | Self::Zip => ChunkId::from_bytes(*blake3::hash(data).as_bytes()),
        }
    }

    fn file_name(&self, id: &ChunkId) -> String {
        match self {
            Self::Plain => id.to_string(),
            Self::Zip | Self::Vault(_) => format!("{id}.zip"),
        }
    }

    /// `<archive_dir>/chunks/<ab>/<name>`.
    pub(crate) fn path(&self, archive_dir: &Path, id: &ChunkId) -> PathBuf {
        archive_dir.join(CHUNKS_DIR).join(id.shard()).join(self.file_name(id))
    }

    /// Path of a chunk file relative to its archive folder, with `/`: for
    /// messages and for recovery by hand.
    pub(crate) fn relative(&self, id: &ChunkId) -> String {
        format!("{CHUNKS_DIR}/{}/{}", id.shard(), self.file_name(id))
    }

    /// Writes `data`, whose name is `id`, as a chunk file of the archive at
    /// `archive_dir`: under a temporary name, renamed once complete, so a
    /// chunk file with its final name is never half written. `deflate`
    /// compresses it (not for content that is already compressed).
    pub(crate) fn write(&self, archive_dir: &Path, id: &ChunkId, data: &[u8], deflate: bool) -> Result<()> {
        let dest = self.path(archive_dir, id);
        let ctx = format!("writing chunk {}", self.relative(id));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(EngineError::io(ctx.clone()))?;
        }
        let tmp = dest.with_file_name(format!("{}{PARTIAL_SUFFIX}", self.file_name(id)));
        let written = self.write_file(&tmp, id, data, deflate, &ctx);
        let result = written.and_then(|()| fs::rename(&tmp, &dest).map_err(EngineError::io(ctx.clone())));
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    fn write_file(&self, path: &Path, id: &ChunkId, data: &[u8], deflate: bool, ctx: &str) -> Result<()> {
        let file = File::create(path).map_err(EngineError::io(ctx.to_owned()))?;
        let method = if deflate { CompressionMethod::Deflated } else { CompressionMethod::Stored };
        let (entry, options) = match self {
            Self::Plain => {
                let mut out = BufWriter::new(file);
                out.write_all(data).map_err(EngineError::io(ctx.to_owned()))?;
                out.flush().map_err(EngineError::io(ctx.to_owned()))?;
                return Ok(());
            }
            Self::Zip => (id.to_string(), crate::crypto::Options::default().compression_method(method)),
            Self::Vault(keys) => (
                VAULT_ENTRY.to_owned(),
                aes_options(keys.password())
                    .compression_method(method)
                    .last_modified_time(zip::DateTime::default()),
            ),
        };
        let mut zip = ZipWriter::new(BufWriter::new(file));
        zip.start_file(entry.as_str(), options).map_err(zip_error(ctx))?;
        zip.write_all(data).map_err(EngineError::io(ctx.to_owned()))?;
        zip.finish().map_err(zip_error(ctx))?.flush().map_err(EngineError::io(ctx.to_owned()))
    }

    /// Reads the chunk file `id` of the archive at `archive_dir` and checks
    /// that its content has that name (and `size` bytes, when given).
    ///
    /// # Errors
    /// [`EngineError::Damaged`] when the content does not match; `Io` when
    /// the file cannot be read.
    pub(crate) fn read(&self, archive_dir: &Path, id: &ChunkId, size: Option<u32>) -> Result<Vec<u8>> {
        let path = self.path(archive_dir, id);
        let ctx = format!("reading chunk {}", self.relative(id));
        let file = File::open(&path).map_err(EngineError::io(ctx.clone()))?;
        let limit = size.map_or(MAX_CHUNK_BYTES, |s| u64::from(s).saturating_add(1));
        let mut data = Vec::with_capacity(size.map_or(0, |s| s as usize));
        match self {
            Self::Plain => {
                file.take(limit).read_to_end(&mut data).map_err(EngineError::io(ctx.clone()))?;
            }
            Self::Zip | Self::Vault(_) => {
                let mut zip = ZipArchive::new(BufReader::new(file)).map_err(zip_error(&ctx))?;
                let entry = match self {
                    Self::Vault(keys) => zip.by_index_decrypt(0, keys.password().as_bytes()),
                    _ => zip.by_index(0),
                };
                let entry = entry.map_err(zip_error(&ctx))?;
                entry.take(limit).read_to_end(&mut data).map_err(EngineError::io(ctx.clone()))?;
            }
        }
        let size_ok = size.is_none_or(|s| data.len() as u64 == u64::from(s));
        if !size_ok || self.id_of(&data) != *id {
            return Err(damaged(format!("chunk {} has different content", self.relative(id))));
        }
        Ok(data)
    }

    /// Names of the chunk files of the archive at `archive_dir`, read from
    /// the file names (files being written and foreign files are ignored).
    pub(crate) fn list(&self, archive_dir: &Path) -> Result<Vec<ChunkId>> {
        let root = archive_dir.join(CHUNKS_DIR);
        let mut out = Vec::new();
        let shards = match fs::read_dir(&root) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(EngineError::io(format!("listing {}", root.display()))(e)),
        };
        for shard in shards.flatten() {
            let Ok(files) = fs::read_dir(shard.path()) else { continue };
            for file in files.flatten() {
                let name = file.file_name();
                let Some(name) = name.to_str() else { continue };
                let stem = match self {
                    Self::Plain => Some(name),
                    Self::Zip | Self::Vault(_) => name.strip_suffix(".zip"),
                };
                if let Some(id) = stem.and_then(|s| s.parse::<ChunkId>().ok()) {
                    out.push(id);
                }
            }
        }
        out.sort_unstable();
        Ok(out)
    }

    /// Deletes the chunk files of the archive at `archive_dir` that are
    /// not in `keep`, and the files left half written. Returns how many
    /// chunk files were removed.
    pub(crate) fn remove_others(&self, archive_dir: &Path, keep: &HashSet<ChunkId>) -> Result<u64> {
        let mut removed = 0;
        for id in self.list(archive_dir)? {
            if !keep.contains(&id) {
                let path = self.path(archive_dir, &id);
                fs::remove_file(&path).map_err(EngineError::io(format!("removing {}", path.display())))?;
                removed += 1;
            }
        }
        let root = archive_dir.join(CHUNKS_DIR);
        for item in walkdir::WalkDir::new(&root).contents_first(true).into_iter().flatten() {
            let path = item.path();
            if item.file_type().is_file() && path.to_string_lossy().ends_with(PARTIAL_SUFFIX) {
                let _ = fs::remove_file(path);
            } else if item.file_type().is_dir() {
                // Fails while the folder still has files: that is the test.
                let _ = fs::remove_dir(path);
            }
        }
        Ok(removed)
    }
}

fn damaged(message: String) -> EngineError {
    EngineError::Damaged { archive: String::new(), message }
}

/// Where one chunk can be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// A chunk file of archive number `archive`.
    File { archive: usize },
    /// Bytes at `offset` of the whole copy number `copy` of [`ChunkIndex`].
    Range { copy: usize, offset: u64 },
}

/// Every chunk readable in a set of archives, numbered like the caller's
/// list of archive folders.
#[derive(Debug, Default)]
pub(crate) struct ChunkIndex {
    places: HashMap<ChunkId, Place>,
    /// Whole copies listing chunks: (archive number, file key).
    copies: Vec<(usize, String)>,
}

impl ChunkIndex {
    /// Adds the chunk files of archive number `archive`. A chunk file is
    /// preferred to a byte range: it is read without going through the
    /// whole copy.
    pub(crate) fn add_files(&mut self, archive: usize, ids: impl IntoIterator<Item = ChunkId>) {
        for id in ids {
            self.places.insert(id, Place::File { archive });
        }
    }

    /// Adds the chunks readable inside the whole copies of `manifest`, the
    /// manifest of archive number `archive`.
    pub(crate) fn add_whole_copies(&mut self, archive: usize, manifest: &Manifest) {
        for e in manifest.entries.iter().filter(|e| e.whole_copy()) {
            let Some(chunks) = &e.chunks else { continue };
            let copy = self.copies.len();
            self.copies.push((archive, e.key.clone()));
            for (offset, c) in chunks.with_offsets() {
                self.places.entry(c.id).or_insert(Place::Range { copy, offset });
            }
        }
    }

    /// True when the chunk `id` can be read.
    pub(crate) fn contains(&self, id: &ChunkId) -> bool {
        self.places.contains_key(id)
    }

    /// Number of the archive holding a chunk file of `id`, if one does.
    pub(crate) fn file_of(&self, id: &ChunkId) -> Option<usize> {
        match self.places.get(id) {
            Some(Place::File { archive }) => Some(*archive),
            _ => None,
        }
    }
}

/// Reads chunks from the archives of one destination, wherever they are.
#[derive(Debug)]
pub(crate) struct ChunkReader<'a> {
    pub(crate) index: &'a ChunkIndex,
    /// Archive folders, numbered as in the index.
    pub(crate) dirs: &'a [PathBuf],
    pub(crate) form: &'a Form,
    pub(crate) keys: Option<&'a Keys>,
}

/// A run of chunks read one after the other.
enum Segment {
    File {
        archive: usize,
        chunk: ChunkRef,
    },
    /// Increasing offsets of one whole copy.
    Ranges {
        copy: usize,
        chunks: Vec<(u64, ChunkRef)>,
    },
}

impl ChunkReader<'_> {
    /// Writes the content made of `chunks`, in order, to `out`, each chunk
    /// checked against its name. Chunks found as consecutive byte ranges of
    /// a whole copy are read in one pass through it. `on_bytes` is called
    /// as chunks are written.
    ///
    /// # Errors
    /// [`EngineError::Damaged`] when a chunk is missing or different.
    pub(crate) fn write_all(
        &self,
        chunks: &[ChunkRef],
        out: &mut dyn Write,
        on_bytes: &mut dyn FnMut(u64),
    ) -> Result<()> {
        for segment in self.plan(chunks)? {
            match segment {
                Segment::File { archive, chunk } => {
                    let dir = self.dirs.get(archive).ok_or_else(|| missing(&chunk))?;
                    let data = self.form.read(dir, &chunk.id, Some(chunk.size))?;
                    out.write_all(&data).map_err(EngineError::io("writing"))?;
                    on_bytes(data.len() as u64);
                }
                Segment::Ranges { copy, chunks } => {
                    self.read_ranges(copy, &chunks, &mut |_, data| {
                        out.write_all(data).map_err(EngineError::io("writing"))?;
                        on_bytes(data.len() as u64);
                        Ok(())
                    })?;
                }
            }
        }
        Ok(())
    }

    /// Calls `f` with the checked content of each chunk of `wanted`, in no
    /// particular order: for copying chunks into a merged archive.
    ///
    /// # Errors
    /// [`EngineError::Damaged`] when a chunk is missing or different, or the
    /// first error of `f`.
    pub(crate) fn for_each(
        &self,
        wanted: &[ChunkRef],
        f: &mut dyn FnMut(&ChunkRef, &[u8]) -> Result<()>,
    ) -> Result<()> {
        let mut by_copy: HashMap<usize, Vec<(u64, ChunkRef)>> = HashMap::new();
        for chunk in wanted {
            match self.index.places.get(&chunk.id) {
                Some(Place::File { archive }) => {
                    let dir = self.dirs.get(*archive).ok_or_else(|| missing(chunk))?;
                    let data = self.form.read(dir, &chunk.id, Some(chunk.size))?;
                    f(chunk, &data)?;
                }
                Some(Place::Range { copy, offset }) => {
                    by_copy.entry(*copy).or_default().push((*offset, *chunk))
                }
                None => return Err(missing(chunk)),
            }
        }
        for (copy, mut ranges) in by_copy {
            ranges.sort_by_key(|(offset, _)| *offset);
            ranges.dedup_by_key(|(offset, _)| *offset);
            self.read_ranges(copy, &ranges, f)?;
        }
        Ok(())
    }

    /// Groups `chunks` into runs: chunk files one by one, byte ranges of
    /// the same whole copy together while their offsets increase.
    fn plan(&self, chunks: &[ChunkRef]) -> Result<Vec<Segment>> {
        let mut out: Vec<Segment> = Vec::new();
        for chunk in chunks {
            match self.index.places.get(&chunk.id) {
                Some(Place::File { archive }) => out.push(Segment::File { archive: *archive, chunk: *chunk }),
                Some(Place::Range { copy, offset }) => {
                    if let Some(Segment::Ranges { copy: c, chunks: run }) = out.last_mut()
                        && *c == *copy
                        && run
                            .last()
                            .is_some_and(|(o, prev)| o.saturating_add(u64::from(prev.size)) <= *offset)
                    {
                        run.push((*offset, *chunk));
                    } else {
                        out.push(Segment::Ranges { copy: *copy, chunks: vec![(*offset, *chunk)] });
                    }
                }
                None => return Err(missing(chunk)),
            }
        }
        Ok(out)
    }

    /// Reads `ranges` (offsets increasing) from whole copy number `copy` in
    /// one pass, calling `f` with each chunk once checked.
    fn read_ranges(
        &self,
        copy: usize,
        ranges: &[(u64, ChunkRef)],
        f: &mut dyn FnMut(&ChunkRef, &[u8]) -> Result<()>,
    ) -> Result<()> {
        let Some((archive, key)) = self.index.copies.get(copy) else {
            return Err(damaged("unknown whole copy".into()));
        };
        let dir = self.dirs.get(*archive).ok_or_else(|| damaged(format!("no archive holds {key}")))?;
        let mut reader = Reader::open(dir, self.keys)?;
        reader.read(key, |src| {
            let mut position = 0u64;
            let mut buf = Vec::new();
            for (offset, chunk) in ranges {
                let gap =
                    offset.checked_sub(position).ok_or_else(|| damaged("ranges out of order".into()))?;
                let skipped = std::io::copy(&mut (&mut *src).take(gap), &mut std::io::sink())
                    .map_err(EngineError::io(format!("reading {key}")))?;
                buf.clear();
                buf.resize(chunk.size as usize, 0);
                let read = src.read_exact(&mut buf);
                if skipped != gap || read.is_err() {
                    return Err(damaged(format!("the whole copy of {key} is shorter than its chunk list")));
                }
                if self.form.id_of(&buf) != chunk.id {
                    return Err(damaged(format!("{key}: chunk {} has different content", chunk.id)));
                }
                f(chunk, &buf)?;
                position = offset.saturating_add(u64::from(chunk.size));
            }
            Ok(())
        })
    }
}

fn missing(chunk: &ChunkRef) -> EngineError {
    damaged(format!("chunk {} is missing", chunk.id))
}

/// Ids of the chunks of every stored file of `manifest` that holds its
/// content as chunks only.
pub(crate) fn referenced(manifest: &Manifest) -> impl Iterator<Item = &ChunkRef> {
    manifest.entries.iter().filter(|e| e.chunks_only()).flat_map(|e| e.chunks.iter().flat_map(|c| &c.list))
}

/// Ids of the chunks readable inside the whole copies of `manifest`.
pub(crate) fn in_whole_copies(manifest: &Manifest) -> impl Iterator<Item = ChunkId> + '_ {
    manifest
        .entries
        .iter()
        .filter(|e| e.whole_copy())
        .flat_map(|e| e.chunks.iter().flat_map(|c| c.list.iter().map(|r| r.id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{ArchiveKind, Chunks, Entry, EntryKind};

    fn chunk(form: &Form, data: &[u8]) -> ChunkRef {
        ChunkRef { id: form.id_of(data), size: u32::try_from(data.len()).unwrap() }
    }

    #[test]
    fn chunk_files_round_trip_in_every_form_and_damage_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let keys = Keys::from_recovery_key(&"ab".repeat(32)).unwrap();
        for form in [Form::Plain, Form::Zip, Form::Vault(keys)] {
            let archive = dir.path().join(format!("{form:?}").chars().take(4).collect::<String>());
            let data = b"some chunk content".repeat(100);
            let c = chunk(&form, &data);
            form.write(&archive, &c.id, &data, true).unwrap();
            assert_eq!(form.read(&archive, &c.id, Some(c.size)).unwrap(), data);
            assert_eq!(form.list(&archive).unwrap(), vec![c.id]);
            assert!(form.read(&archive, &c.id, Some(c.size + 1)).is_err(), "size checked");

            let other = chunk(&form, b"other");
            fs::create_dir_all(form.path(&archive, &other.id).parent().unwrap()).unwrap();
            fs::copy(form.path(&archive, &c.id), form.path(&archive, &other.id)).unwrap();
            assert!(matches!(form.read(&archive, &other.id, None), Err(EngineError::Damaged { .. })));

            let keep: HashSet<ChunkId> = [c.id].into_iter().collect();
            assert_eq!(form.remove_others(&archive, &keep).unwrap(), 1);
            assert_eq!(form.list(&archive).unwrap(), vec![c.id]);
        }
    }

    #[test]
    fn encrypted_chunks_hide_their_content() {
        let dir = tempfile::tempdir().unwrap();
        let form = Form::Vault(Keys::from_recovery_key(&"cd".repeat(32)).unwrap());
        let data = b"secret secret secret".to_vec();
        let c = chunk(&form, &data);
        form.write(dir.path(), &c.id, &data, true).unwrap();
        let bytes = fs::read(form.path(dir.path(), &c.id)).unwrap();
        assert!(!bytes.windows(6).any(|w| w == b"secret"));
        assert_ne!(c.id, ChunkId::from_bytes(*blake3::hash(&data).as_bytes()), "the name is keyed");
    }

    #[test]
    fn chunks_are_read_from_files_or_from_whole_copies() {
        let dir = tempfile::tempdir().unwrap();
        let form = Form::Plain;
        // Archive 0: a whole copy of "big" listing three chunks.
        let parts: [&[u8]; 3] = [b"aaaa", b"bbbbbb", b"cc"];
        let a0 = dir.path().join("a0");
        let key = "C/big";
        let data_dir = a0.join(crate::repo::DATA_DIR);
        let path = archeion_core::keys::path_under(&data_dir, key);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, parts.concat()).unwrap();
        let list: Vec<ChunkRef> = parts.iter().map(|p| chunk(&form, p)).collect();
        let entry = Entry {
            key: key.into(),
            kind: EntryKind::File,
            size: 12,
            mtime_ns: 0,
            hash: None,
            stored: true,
            blob: None,
            chunks: Some(Chunks { whole: true, list: list.clone() }),
        };
        let m0 = Manifest::new(ArchiveKind::Complete, vec![entry]);
        // Archive 1: one new chunk file.
        let a1 = dir.path().join("a1");
        let new = chunk(&form, b"dddd");
        form.write(&a1, &new.id, b"dddd", false).unwrap();

        let mut index = ChunkIndex::default();
        index.add_whole_copies(0, &m0);
        index.add_files(1, form.list(&a1).unwrap());
        assert!(index.contains(&list[1].id) && index.contains(&new.id));
        assert_eq!(index.file_of(&new.id), Some(1));
        assert_eq!(index.file_of(&list[0].id), None, "readable from a whole copy only");

        let dirs = vec![a0, a1];
        let reader = ChunkReader { index: &index, dirs: &dirs, form: &form, keys: None };
        // Out of order, with a repeat and a chunk file in the middle.
        let wanted = [list[2], list[0], new, list[1], list[1]];
        let mut out = Vec::new();
        let mut seen = 0;
        reader.write_all(&wanted, &mut out, &mut |n| seen += n).unwrap();
        assert_eq!(out, b"ccaaaaddddbbbbbbbbbbbb");
        assert_eq!(seen, out.len() as u64);

        let mut got = Vec::new();
        reader
            .for_each(&wanted, &mut |c, d| {
                assert_eq!(c.size as usize, d.len());
                got.push(d.to_vec());
                Ok(())
            })
            .unwrap();
        assert_eq!(got.len(), 4, "each distinct chunk once");

        let unknown = chunk(&form, b"nowhere");
        assert!(matches!(
            reader.write_all(&[unknown], &mut Vec::new(), &mut |_| {}),
            Err(EngineError::Damaged { .. })
        ));

        // A whole copy that changed under its list is caught.
        fs::write(&path, b"aaaaXbbbbbcc").unwrap();
        assert!(reader.write_all(&[list[1]], &mut Vec::new(), &mut |_| {}).is_err());
    }
}
