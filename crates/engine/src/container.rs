//! Content of one archive, in one of two containers:
//!
//! | Container | On disk |
//! | --- | --- |
//! | zip per file (default) | `zipped/C/Users/.../report.docx.zip`, one entry `report.docx` |
//! | plain folder | `data/C/Users/.../report.docx` |
//! | encrypted zip per file | `vault/ab/ab12….zip`, one AES-256 entry `data` (see [`crate::crypto`]) |
//!
//! With one zip per file, every file of the archive gets the `.zip` suffix,
//! so no compressed name can collide with an original one. Formats that are
//! already compressed (photos, videos, Office files, archives) are stored
//! without recompression inside their zip.

use crate::{
    EngineError, Result,
    crypto::{Keys, aes_options},
    repo::{DATA_DIR, VAULT_DIR, ZIPPED_DIR},
};
use archeion_core::{ArchiveName, Container, keys};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const BUFFER: usize = 256 * 1024;

/// Extensions whose content is already compressed: deflating them again
/// costs time and gains nothing.
const ALREADY_COMPRESSED: &[&str] = &[
    "7z", "aac", "avi", "bz2", "docx", "flac", "gif", "gz", "heic", "jpeg", "jpg", "m4a", "m4v", "mkv",
    "mov", "mp3", "mp4", "ogg", "png", "pptx", "rar", "webm", "webp", "xlsx", "xz", "zip", "zst",
];

/// Which container a committed archive uses (encrypted archives are zips).
pub(crate) fn detect(archive_dir: &Path) -> Container {
    if archive_dir.join(ZIPPED_DIR).exists() || archive_dir.join(VAULT_DIR).exists() {
        Container::ZipPerFile
    } else {
        Container::Folder
    }
}

/// The container of an unfinished archive, once it holds content.
pub(crate) fn detect_existing(dir: &Path) -> Option<Container> {
    let any = [ZIPPED_DIR, VAULT_DIR, DATA_DIR].iter().any(|d| dir.join(d).exists());
    any.then(|| detect(dir))
}

/// `<ab>/<name>.zip`, below the vault folder, for the opaque name of an
/// encrypted file in `archive`.
fn vault_relative(keys: &Keys, archive: &str, key: &str) -> String {
    let name = keys.blob_name(archive, key);
    let shard = name.get(..2).unwrap_or("00");
    format!("{shard}/{name}.zip")
}

fn vault_path(base: &Path, keys: &Keys, archive: &str, key: &str) -> PathBuf {
    base.join(vault_relative(keys, archive, key))
}

/// Name of the single entry inside an encrypted file's zip.
const VAULT_ENTRY: &str = "data";

/// `<base>/<key>.zip`.
fn zipped_path(base: &Path, key: &str) -> PathBuf {
    let mut p: OsString = keys::path_under(base, key).into_os_string();
    p.push(".zip");
    PathBuf::from(p)
}

fn method_for(key: &str) -> CompressionMethod {
    let ext = key.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    if ALREADY_COMPRESSED.contains(&ext.as_str()) {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    }
}

fn options_for<'k>(key: &str, size: u64, password: Option<&'k str>) -> crate::crypto::Options<'k> {
    let base = match password {
        Some(p) => aes_options(p),
        None => crate::crypto::Options::default(),
    };
    base.compression_method(method_for(key)).large_file(size >= u64::from(u32::MAX))
}

fn zip_err(context: &str) -> impl FnOnce(zip::result::ZipError) -> EngineError + '_ {
    crate::crypto::zip_error(context)
}

fn create_parent(path: &Path, ctx: &str) -> Result<()> {
    match path.parent() {
        Some(parent) => fs::create_dir_all(parent).map_err(EngineError::io(ctx.to_owned())),
        None => Ok(()),
    }
}

/// Writes the content of an archive being built.
#[derive(Debug)]
pub(crate) enum Writer {
    Folder {
        data: PathBuf,
    },
    ZipPerFile {
        root: PathBuf,
    },
    /// `archive`: name of the archive being built (the stored names depend
    /// on it).
    Vault {
        root: PathBuf,
        keys: Keys,
        archive: String,
    },
}

impl Writer {
    /// With `keys` (an encrypted destination) the content is always an
    /// encrypted vault, whatever `container` says. `archive` is the name the
    /// archive will have once committed. Content already in `staging` (an
    /// interrupted run being resumed) is kept.
    pub(crate) fn create(
        staging: &Path,
        container: Container,
        keys: Option<&Keys>,
        archive: &ArchiveName,
    ) -> Result<Self> {
        if let Some(keys) = keys {
            let root = staging.join(VAULT_DIR);
            fs::create_dir_all(&root).map_err(EngineError::io("creating the vault folder"))?;
            return Ok(Self::Vault { root, keys: keys.clone(), archive: archive.to_string() });
        }
        match container {
            Container::Folder => {
                let data = staging.join(DATA_DIR);
                fs::create_dir_all(&data).map_err(EngineError::io("creating the data folder"))?;
                Ok(Self::Folder { data })
            }
            Container::ZipPerFile => {
                let root = staging.join(ZIPPED_DIR);
                fs::create_dir_all(&root).map_err(EngineError::io("creating the zipped folder"))?;
                Ok(Self::ZipPerFile { root })
            }
        }
    }

    /// Copies `src` as `key`, hashing it on the way. Returns the bytes copied
    /// and the BLAKE3 hash (of the original content) in hex. `on_bytes` is
    /// called as data flows.
    pub(crate) fn add_stream(
        &mut self,
        key: &str,
        src: &mut dyn Read,
        expected_size: u64,
        mtime_ns: i64,
        on_bytes: &mut dyn FnMut(u64),
    ) -> Result<(u64, String)> {
        let ctx = format!("copying {key}");
        self.add_with(key, expected_size, mtime_ns, &mut |dst| {
            copy_into(src, dst, on_bytes).map(|_| ()).map_err(EngineError::io(ctx.clone()))
        })
    }

    /// Stores as `key` the bytes that `fill` writes, hashing them on the
    /// way; a failed `fill` leaves nothing behind. `expected_size` only
    /// chooses the zip format for very large files. Returns the bytes
    /// written and their BLAKE3 hash in hex.
    pub(crate) fn add_with(
        &mut self,
        key: &str,
        expected_size: u64,
        mtime_ns: i64,
        fill: &mut dyn FnMut(&mut dyn Write) -> Result<()>,
    ) -> Result<(u64, String)> {
        let ctx = format!("copying {key}");
        match self {
            Self::Folder { data } => {
                let dest = keys::path_under(data, key);
                create_parent(&dest, &ctx)?;
                let result = write_plain(&dest, mtime_ns, fill, &ctx);
                if result.is_err() {
                    let _ = fs::remove_file(&dest);
                }
                result
            }
            Self::ZipPerFile { root } => {
                let dest = zipped_path(root, key);
                let name = key.rsplit('/').next().unwrap_or(key);
                let options = options_for(key, expected_size, None);
                write_zip_cleanly(&dest, name, options, fill, Some(mtime_ns), &ctx)
            }
            Self::Vault { root, keys, archive } => {
                // No date on an encrypted file, inside or on disk.
                let dest = vault_path(root, keys, archive, key);
                let options = options_for(key, expected_size, Some(keys.password()))
                    .last_modified_time(zip::DateTime::default());
                write_zip_cleanly(&dest, VAULT_ENTRY, options, fill, None, &ctx)
            }
        }
    }

    /// Where an encrypted file is stored, relative to the archive folder,
    /// for the manifest (recovery by hand); `None` for other containers.
    pub(crate) fn stored_as(&self, key: &str) -> Option<String> {
        match self {
            Self::Vault { keys, archive, .. } => {
                Some(format!("{VAULT_DIR}/{}", vault_relative(keys, archive, key)))
            }
            Self::Folder { .. } | Self::ZipPerFile { .. } => None,
        }
    }

    /// Name of `key` in the resume journal: the key itself, or the opaque
    /// stored name in an encrypted archive.
    pub(crate) fn journal_id(&self, key: &str) -> String {
        match self {
            Self::Vault { keys, archive, .. } => keys.blob_name(archive, key),
            Self::Folder { .. } | Self::ZipPerFile { .. } => key.to_owned(),
        }
    }

    /// File holding `key`'s content.
    pub(crate) fn content_path(&self, key: &str) -> PathBuf {
        match self {
            Self::Folder { data } => keys::path_under(data, key),
            Self::ZipPerFile { root } => zipped_path(root, key),
            Self::Vault { root, keys, archive } => vault_path(root, keys, archive, key),
        }
    }

    /// True when the stored copy of `key` is whole: a plain copy of `size`
    /// bytes, or a zip whose directory (written last) is readable and
    /// lists one entry of `size` bytes.
    pub(crate) fn looks_complete(&self, key: &str, size: u64) -> bool {
        let path = self.content_path(key);
        match self {
            Self::Folder { .. } => fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() == size),
            Self::ZipPerFile { .. } | Self::Vault { .. } => File::open(&path)
                .ok()
                .and_then(|f| ZipArchive::new(BufReader::new(f)).ok())
                .is_some_and(|z| z.len() == 1 && z.decompressed_size() == Some(u128::from(size))),
        }
    }

    /// True when `key` is compressed (deflated) on the way in.
    pub(crate) fn deflates(&self, key: &str) -> bool {
        match self {
            Self::Folder { .. } => false,
            Self::ZipPerFile { .. } | Self::Vault { .. } => method_for(key) == CompressionMethod::Deflated,
        }
    }

    /// Deletes the stored files that are not in `keep` (paths from
    /// [`Self::content_path`]) and the folders left empty: leftovers of an
    /// interrupted run whose sources have since changed or gone.
    pub(crate) fn remove_others(&self, keep: &std::collections::HashSet<PathBuf>) -> Result<u64> {
        let root = match self {
            Self::Folder { data } => data,
            Self::ZipPerFile { root } | Self::Vault { root, .. } => root,
        };
        let mut removed = 0;
        for item in walkdir::WalkDir::new(root).contents_first(true).into_iter().flatten() {
            let path = item.path();
            if item.file_type().is_file() && !keep.contains(path) {
                fs::remove_file(path).map_err(EngineError::io(format!("removing {}", path.display())))?;
                removed += 1;
            } else if item.file_type().is_dir() && path != root {
                // Fails while the folder still has files: that is the test.
                let _ = fs::remove_dir(path);
            }
        }
        Ok(removed)
    }

    /// Copies `key` from another archive without recompressing when the
    /// containers match: hard link between folders or per-file zips (copy
    /// when links are not supported).
    pub(crate) fn add_from(&mut self, reader: &mut Reader, key: &str, mtime_ns: i64) -> Result<()> {
        let ctx = format!("merging {key}");
        match (self, reader) {
            (Self::Folder { data }, Reader::Folder { data: src, inside }) => {
                let from = keys::path_under(src, key);
                drop(inside.open(&from, &ctx)?);
                link_or_copy(&from, &keys::path_under(data, key), &ctx)
            }
            (Self::ZipPerFile { root }, Reader::ZipPerFile { root: src, inside }) => {
                let from = zipped_path(src, key);
                drop(inside.open(&from, &ctx)?);
                link_or_copy(&from, &zipped_path(root, key), &ctx)
            }
            (
                Self::Vault { root, keys, archive },
                Reader::Vault { root: src, keys: src_keys, archive: from },
            ) if keys.password() == src_keys.password() => {
                // Same master key: the zip is reused as is, under its new name.
                link_or_copy(
                    &vault_path(src, src_keys, from, key),
                    &vault_path(root, keys, archive, key),
                    &ctx,
                )
            }
            (writer, reader) => {
                reader.read(key, |src| writer.add_stream(key, src, 0, mtime_ns, &mut |_| {}).map(|_| ()))
            }
        }
    }

    /// Finishes the content. Each file is complete when written, so there
    /// is nothing left to do; kept so callers do not depend on that.
    pub(crate) fn finish(self) -> Result<()> {
        Ok(())
    }
}

/// [`write_single_zip`] into `dest`, removing it on failure.
fn write_zip_cleanly(
    dest: &Path,
    entry: &str,
    options: crate::crypto::Options<'_>,
    fill: &mut dyn FnMut(&mut dyn Write) -> Result<()>,
    mtime_ns: Option<i64>,
    ctx: &str,
) -> Result<(u64, String)> {
    create_parent(dest, ctx)?;
    let result = write_single_zip(dest, entry, options, fill, mtime_ns, ctx);
    if result.is_err() {
        let _ = fs::remove_file(dest);
    }
    result
}

/// One zip holding one entry, `entry`, with what `fill` writes; the zip
/// file gets `mtime_ns` as its modification time when given.
fn write_single_zip(
    dest: &Path,
    entry: &str,
    options: crate::crypto::Options<'_>,
    fill: &mut dyn FnMut(&mut dyn Write) -> Result<()>,
    mtime_ns: Option<i64>,
    ctx: &str,
) -> Result<(u64, String)> {
    let file = File::create(dest).map_err(EngineError::io(ctx.to_owned()))?;
    let mut zip = ZipWriter::new(BufWriter::with_capacity(BUFFER, file));
    zip.start_file(entry, options).map_err(zip_err(ctx))?;
    let mut hashing = HashingWriter::new(&mut zip);
    fill(&mut hashing)?;
    let (n, hash, _) = hashing.finish();
    let file = zip
        .finish()
        .map_err(zip_err(ctx))?
        .into_inner()
        .map_err(|e| EngineError::io(ctx.to_owned())(e.into_error()))?;
    if let Some(t) = mtime_ns {
        let _ = file.set_modified(system_time(t));
    }
    // On the disk before the archive is committed: a committed file of
    // zeros would be taken as unchanged by the next backups.
    file.sync_all().map_err(EngineError::io(ctx.to_owned()))?;
    Ok((n, hash))
}

/// A plain copy of what `fill` writes, dated `mtime_ns`.
fn write_plain(
    dest: &Path,
    mtime_ns: i64,
    fill: &mut dyn FnMut(&mut dyn Write) -> Result<()>,
    ctx: &str,
) -> Result<(u64, String)> {
    let file = File::create(dest).map_err(EngineError::io(ctx.to_owned()))?;
    let mut hashing = HashingWriter::new(BufWriter::with_capacity(BUFFER, file));
    fill(&mut hashing)?;
    let (n, hash, buffered) = hashing.finish();
    let file = buffered.into_inner().map_err(|e| EngineError::io(ctx.to_owned())(e.into_error()))?;
    let _ = file.set_modified(system_time(mtime_ns));
    // As in `write_single_zip`.
    file.sync_all().map_err(EngineError::io(ctx.to_owned()))?;
    Ok((n, hash))
}

/// Passes writes on to `inner`, counting and hashing (BLAKE3) them.
pub(crate) struct HashingWriter<W> {
    inner: W,
    hasher: blake3::Hasher,
    written: u64,
}

impl<W> std::fmt::Debug for HashingWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HashingWriter").field("written", &self.written).finish_non_exhaustive()
    }
}

impl<W: Write> HashingWriter<W> {
    pub(crate) fn new(inner: W) -> Self {
        Self { inner, hasher: blake3::Hasher::new(), written: 0 }
    }

    /// Bytes written, their hash in hex, and the inner writer (flushed).
    pub(crate) fn finish(mut self) -> (u64, String, W) {
        let _ = self.inner.flush();
        (self.written, self.hasher.finalize().to_hex().to_string(), self.inner)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        let done = buf.get(..n).unwrap_or(buf);
        self.hasher.update(done);
        self.written = self.written.saturating_add(done.len() as u64);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Hard link `src` as `dest` (a copy where links are not supported).
pub(crate) fn link_or_copy(src: &Path, dest: &Path, ctx: &str) -> Result<()> {
    create_parent(dest, ctx)?;
    if fs::hard_link(src, dest).is_err() {
        // No links (exFAT, FAT32, some shares): a copy, on the disk before
        // the archive holding it is committed and the original removed.
        fs::copy(src, dest).map_err(EngineError::io(ctx.to_owned()))?;
        File::options()
            .write(true)
            .open(dest)
            .and_then(|f| f.sync_all())
            .map_err(EngineError::io(ctx.to_owned()))?;
    }
    Ok(())
}

/// Reads the content of a committed archive.
///
/// Plain files and plain zips are read only when they really lie inside
/// the destination (`inside`, its real path): a link or junction put in a
/// backup could otherwise make a merge copy any file of the computer into
/// the destination. Encrypted files need the key, so a file from
/// elsewhere does not open as one.
#[derive(Debug)]
pub(crate) enum Reader {
    Folder { data: PathBuf, inside: Inside },
    ZipPerFile { root: PathBuf, inside: Inside },
    Vault { root: PathBuf, keys: Keys, archive: String },
}

/// The real path of a destination folder, for checking that the files read
/// from it are inside it.
#[derive(Debug, Clone)]
pub(crate) struct Inside {
    real: PathBuf,
    archive: String,
}

impl Inside {
    /// The destination folder holding `archive_dir`.
    fn of(archive_dir: &Path) -> Result<Self> {
        let root = archive_dir.parent().unwrap_or(archive_dir);
        let real = fs::canonicalize(root).map_err(EngineError::io(format!("opening {}", root.display())))?;
        let archive = archive_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(Self { real, archive })
    }

    /// Fails unless `file`, opened at `path`, lies below the destination.
    fn check(&self, file: &File, path: &Path) -> Result<()> {
        match archeion_win::real_path(file) {
            Ok(real) if real.starts_with(&self.real) => Ok(()),
            Ok(_) => Err(EngineError::Damaged {
                archive: self.archive.clone(),
                message: format!("{} leads outside the destination (a link)", path.display()),
            }),
            // A system that cannot tell: nothing to compare.
            Err(e) if e.kind() == std::io::ErrorKind::Unsupported => Ok(()),
            Err(e) => Err(EngineError::io(format!("reading {}", path.display()))(e)),
        }
    }

    /// Opens `path` for reading once [`Self::check`] passes.
    fn open(&self, path: &Path, ctx: &str) -> Result<File> {
        let file = File::open(path).map_err(EngineError::io(ctx.to_owned()))?;
        self.check(&file, path)?;
        Ok(file)
    }
}

impl Reader {
    /// Opens `archive_dir`, whichever container it uses. An encrypted
    /// archive needs `keys`.
    pub(crate) fn open(archive_dir: &Path, keys: Option<&Keys>) -> Result<Self> {
        let vault = archive_dir.join(VAULT_DIR);
        // With keys, only encrypted files are read: plain files put in an
        // encrypted destination are not its backups.
        if vault.exists() || keys.is_some() {
            let keys = keys.ok_or(EngineError::PasswordNeeded)?;
            let archive =
                archive_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            return Ok(Self::Vault { root: vault, keys: keys.clone(), archive });
        }
        let inside = Inside::of(archive_dir)?;
        match detect(archive_dir) {
            Container::ZipPerFile => Ok(Self::ZipPerFile { root: archive_dir.join(ZIPPED_DIR), inside }),
            Container::Folder => Ok(Self::Folder { data: archive_dir.join(DATA_DIR), inside }),
        }
    }

    /// Runs `f` on a reader of `key`'s original content.
    pub(crate) fn read<T>(&mut self, key: &str, f: impl FnOnce(&mut dyn Read) -> Result<T>) -> Result<T> {
        let ctx = format!("reading {key}");
        match self {
            Self::Folder { data, inside } => {
                let mut file = inside.open(&keys::path_under(data, key), &ctx)?;
                f(&mut file)
            }
            Self::ZipPerFile { root, inside } => {
                let file = inside.open(&zipped_path(root, key), &ctx)?;
                let mut zip =
                    ZipArchive::new(BufReader::with_capacity(BUFFER, file)).map_err(zip_err(&ctx))?;
                let mut entry = zip.by_index(0).map_err(zip_err(&ctx))?;
                f(&mut entry)
            }
            Self::Vault { root, keys, archive } => {
                let file =
                    File::open(vault_path(root, keys, archive, key)).map_err(EngineError::io(ctx.clone()))?;
                let mut zip =
                    ZipArchive::new(BufReader::with_capacity(BUFFER, file)).map_err(zip_err(&ctx))?;
                let mut entry = zip.by_index_decrypt(0, keys.password().as_bytes()).map_err(zip_err(&ctx))?;
                // The zip library reads a plain entry even when given a key.
                if !entry.encrypted() {
                    return Err(EngineError::Damaged {
                        archive: archive.clone(),
                        message: format!("{key} is not encrypted"),
                    });
                }
                f(&mut entry)
            }
        }
    }
}

/// Copies `src` to `dst`, calling `on_bytes` as data flows; returns the
/// bytes copied. `dst` is flushed.
pub(crate) fn copy_into(
    src: &mut dyn Read,
    dst: &mut dyn Write,
    on_bytes: &mut dyn FnMut(u64),
) -> std::io::Result<u64> {
    let mut buf = vec![0u8; BUFFER];
    let mut total = 0u64;
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        dst.write_all(buf.get(..n).unwrap_or_default())?;
        total = total.saturating_add(n as u64);
        on_bytes(n as u64);
    }
    dst.flush()?;
    Ok(total)
}

/// Copies `src` to `dst` and returns (bytes, BLAKE3 hex).
pub(crate) fn copy_hashing(
    src: &mut dyn Read,
    dst: &mut dyn Write,
    on_bytes: &mut dyn FnMut(u64),
) -> std::io::Result<(u64, String)> {
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; BUFFER];
    let mut total = 0u64;
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let chunk = buf.get(..n).unwrap_or_default();
        hasher.update(chunk);
        dst.write_all(chunk)?;
        total += n as u64;
        on_bytes(n as u64);
    }
    dst.flush()?;
    Ok((total, hasher.finalize().to_hex().to_string()))
}

/// Hash of everything `src` yields.
pub(crate) fn hash_stream(
    src: &mut dyn Read,
    on_bytes: &mut dyn FnMut(u64),
) -> std::io::Result<(u64, String)> {
    copy_hashing(src, &mut std::io::sink(), on_bytes)
}

pub(crate) fn system_time(mtime_ns: i64) -> SystemTime {
    let ns = mtime_ns.unsigned_abs();
    let d = Duration::new(ns / 1_000_000_000, u32::try_from(ns % 1_000_000_000).unwrap_or(0));
    if mtime_ns >= 0 { SystemTime::UNIX_EPOCH + d } else { SystemTime::UNIX_EPOCH - d }
}

pub(crate) fn mtime_ns(t: SystemTime) -> i64 {
    match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
        Err(e) => i64::try_from(e.duration().as_nanos()).map_or(i64::MIN, |n| -n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtime_round_trips() {
        // Multiples of 100 ns: the resolution of Windows `SystemTime` and NTFS.
        for ns in [0, 1_700_000_000_123_456_700, -5_000_000_100] {
            assert_eq!(mtime_ns(system_time(ns)), ns);
        }
    }

    #[test]
    fn copy_hashing_matches_blake3() {
        let data = vec![7u8; 600_000];
        let mut out = Vec::new();
        let mut seen = 0;
        let (n, h) = copy_hashing(&mut data.as_slice(), &mut out, &mut |b| seen += b).unwrap();
        assert_eq!(n, 600_000);
        assert_eq!(seen, 600_000);
        assert_eq!(out, data);
        assert_eq!(h, blake3::hash(&data).to_hex().to_string());
    }

    #[test]
    fn compressed_formats_are_stored_others_deflated() {
        assert_eq!(method_for("C/x/photo.JPG"), CompressionMethod::Stored);
        assert_eq!(method_for("C/x/notes.txt"), CompressionMethod::Deflated);
        assert_eq!(method_for("C/x/Makefile"), CompressionMethod::Deflated);
    }

    #[test]
    fn zipped_name_appends_the_suffix() {
        let p = zipped_path(Path::new("z"), "C/a/report.docx");
        assert_eq!(p, Path::new("z").join("C").join("a").join("report.docx.zip"));
    }
}
