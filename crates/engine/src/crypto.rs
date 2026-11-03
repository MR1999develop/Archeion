//! Encrypted destinations.
//!
//! Every file is an AES-256 zip (WinZip AE-2). The zips are not encrypted
//! with the user's password but with a random 256-bit **master key**; the
//! password only unlocks `archeion.key.json`, which holds the master key
//! wrapped with a key derived from the password by Argon2id (64 MiB, 3
//! passes). So:
//!
//! - guessing the password costs an Argon2id run per attempt, and the zips
//!   themselves cannot be attacked (their password is 256 random bits);
//! - the password can be changed by re-wrapping the master key alone;
//! - the master key, written as 64 hex characters, is the **recovery key**:
//!   7-Zip opens every zip (and the encrypted manifests) with it.
//!
//! Nothing reveals file or folder names: a file is `vault/<ab>/<64 hex>.zip`,
//! the hex being a hash of the archive name and the file's path keyed with
//! a secret derived from the master key, so the same file has unrelated
//! names in different archives; the entry inside is called `data` and
//! carries no date. Each archive's manifest is `manifest.zip`, encrypted the
//! same way; it lists the stored name of each file, for recovery by hand.
//! The archive folders keep their dates.

use crate::{EngineError, Result};
use archeion_core::ArchiveName;
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{AesMode, ZipArchive, ZipWriter, result::ZipError, write::FileOptions};

/// Zip entry options borrowing the password.
pub(crate) type Options<'k> = FileOptions<'k, 'static, ()>;

/// The wrapped master key and how to unwrap it.
pub(crate) const KEY_FILE: &str = "archeion.key.json";
/// Signature of the list of archives.
pub(crate) const LIST_MAC_FILE: &str = "archeion.archives.mac";
/// Format of [`KEY_FILE`].
/// 3 since the key derivation contexts below changed.
pub(crate) const KEY_FORMAT: u32 = 3;
// Key derivation contexts: fixed forever from now on, whatever the program
// is called (changing them makes existing backups unreadable).
const NAME_KEY_CONTEXT: &str = "Archeion 2026-11-02 file name key v3";
const LIST_KEY_CONTEXT: &str = "Archeion 2026-11-02 archive list key v3";
const NOTE_KEY_CONTEXT: &str = "Archeion 2026-11-02 resume note key v3";
const CHUNK_KEY_CONTEXT: &str = "Archeion 2026-11-01 chunk name key v1";
/// Bytes of the authentication tag of a sealed note.
const NOTE_TAG: usize = 16;
const WRAP_ENTRY: &str = "master-key";

/// Argon2id cost for new key files: 64 MiB, 3 passes, 1 lane (a few tenths
/// of a second on a desktop).
const ARGON_MEMORY_KIB: u32 = 64 * 1024;
const ARGON_PASSES: u32 = 3;
const ARGON_LANES: u32 = 1;
/// Costs accepted when reading a key file, so a doctored file can neither
/// weaken the derivation much nor exhaust the memory.
const ARGON_MEMORY_RANGE: std::ops::RangeInclusive<u32> = (8 * 1024)..=(1024 * 1024);
const ARGON_PASSES_RANGE: std::ops::RangeInclusive<u32> = 1..=20;
const ARGON_LANES_RANGE: std::ops::RangeInclusive<u32> = 1..=16;

/// A destination password. Its `Debug` output never shows it.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    /// `None` for an empty text.
    #[must_use]
    pub fn new(text: String) -> Option<Self> {
        (!text.is_empty()).then_some(Self(text))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(…)")
    }
}

/// `archeion.key.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyFile {
    format: u32,
    kdf: String,
    memory_kib: u32,
    passes: u32,
    lanes: u32,
    /// Hex.
    salt: String,
    /// Hex of a zip holding the master key, encrypted with the Argon2id
    /// output (as hex) as its password.
    wrapped: String,
}

/// An unlocked encrypted destination.
#[derive(Clone)]
pub(crate) struct Keys {
    master: [u8; 32],
    /// The master key as hex: the password of every zip.
    zip_password: String,
    names: [u8; 32],
    list: [u8; 32],
    /// Seals the notes of the resume journal.
    notes: [u8; 32],
}

impl fmt::Debug for Keys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Keys(…)")
    }
}

impl Keys {
    fn from_master(master: [u8; 32]) -> Self {
        Self {
            master,
            zip_password: to_hex(&master),
            names: blake3::derive_key(NAME_KEY_CONTEXT, &master),
            list: blake3::derive_key(LIST_KEY_CONTEXT, &master),
            notes: blake3::derive_key(NOTE_KEY_CONTEXT, &master),
        }
    }

    /// The keys behind a recovery key (64 hex digits; spaces, dashes and
    /// case do not matter): for opening a destination without the
    /// password. `None` when the text is not a recovery key; whether it is
    /// the right one shows when a manifest is opened.
    pub(crate) fn from_recovery_key(text: &str) -> Option<Self> {
        let hex: String = text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
        let master: [u8; 32] = from_hex(&hex)?.try_into().ok()?;
        Some(Self::from_master(master))
    }

    /// Name of a chunk with content `data`: BLAKE3 keyed with a secret of
    /// the destination, so the name cannot be matched against the hash of
    /// a known file. The same content has the same name in every archive
    /// of the destination, which is what lets a chunk be stored once.
    pub(crate) fn chunk_id(&self, data: &[u8]) -> [u8; 32] {
        // Derived at each call rather than kept: a few hundred nanoseconds
        // per chunk, and `Keys` stays small inside the readers and writers.
        let key = blake3::derive_key(CHUNK_KEY_CONTEXT, &self.master);
        *blake3::keyed_hash(&key, data).as_bytes()
    }

    /// Password of the zips (the recovery key).
    pub(crate) fn password(&self) -> &str {
        &self.zip_password
    }

    /// Recovery key for people: the master key in groups of four.
    pub(crate) fn recovery_key(&self) -> String {
        let hex = to_hex(&self.master);
        hex.as_bytes()
            .chunks(4)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Opaque stored name of the file `key` in `archive`: 64 hex characters.
    pub(crate) fn blob_name(&self, archive: &str, key: &str) -> String {
        let mut h = blake3::Hasher::new_keyed(&self.names);
        h.update(archive.as_bytes());
        h.update(&[0]);
        h.update(key.as_bytes());
        h.finalize().to_hex().to_string()
    }

    /// Signature of the list of archives, hex.
    pub(crate) fn list_mac(&self, archives: &[ArchiveName]) -> String {
        let mut h = blake3::Hasher::new_keyed(&self.list);
        for a in archives {
            h.update(a.to_string().as_bytes());
            h.update(b"\n");
        }
        h.finalize().to_hex().to_string()
    }

    /// `plain` encrypted and authenticated for the journal line of `id`
    /// (an opaque stored name), as hex. Each id is written once per archive
    /// being built, so its keystream is never reused for different text.
    pub(crate) fn seal_note(&self, id: &str, plain: &str) -> String {
        let mut bytes = plain.as_bytes().to_vec();
        self.note_keystream(id, &mut bytes);
        let tag = self.note_tag(id, plain.as_bytes());
        bytes.extend_from_slice(&tag);
        to_hex(&bytes)
    }

    /// The text sealed by [`Self::seal_note`]; `None` when `sealed` was not
    /// written for `id` with these keys.
    pub(crate) fn open_note(&self, id: &str, sealed: &str) -> Option<String> {
        let mut bytes = from_hex(sealed)?;
        let split = bytes.len().checked_sub(NOTE_TAG)?;
        let tag = bytes.split_off(split);
        self.note_keystream(id, &mut bytes);
        (self.note_tag(id, &bytes).as_slice() == tag.as_slice()).then(|| String::from_utf8(bytes).ok())?
    }

    fn note_keystream(&self, id: &str, bytes: &mut [u8]) {
        let mut h = blake3::Hasher::new_keyed(&self.notes);
        h.update(b"stream\0");
        h.update(id.as_bytes());
        let mut stream = vec![0u8; bytes.len()];
        h.finalize_xof().fill(&mut stream);
        for (b, k) in bytes.iter_mut().zip(stream) {
            *b ^= k;
        }
    }

    fn note_tag(&self, id: &str, plain: &[u8]) -> [u8; NOTE_TAG] {
        let mut h = blake3::Hasher::new_keyed(&self.notes);
        h.update(b"tag\0");
        h.update(id.as_bytes());
        h.update(&[0]);
        h.update(plain);
        let mut tag = [0u8; NOTE_TAG];
        h.finalize_xof().fill(&mut tag);
        tag
    }

    /// Sets up a new encrypted destination: a random master key wrapped
    /// with `password`.
    pub(crate) fn create(root: &Path, password: &Password) -> Result<Self> {
        let mut master = [0u8; 32];
        random(&mut master)?;
        let keys = Self::from_master(master);
        keys.write_key_file(root, password)?;
        Ok(keys)
    }

    /// Unwraps the master key with `password`.
    ///
    /// # Errors
    /// [`EngineError::WrongPassword`] when it does not open the key file.
    pub(crate) fn unlock(root: &Path, password: &Password) -> Result<Self> {
        let path = root.join(KEY_FILE);
        let text =
            std::fs::read_to_string(&path).map_err(EngineError::io(format!("reading {}", path.display())))?;
        let damaged =
            |message: &str| EngineError::Damaged { archive: KEY_FILE.to_owned(), message: message.into() };
        let file: KeyFile = serde_json::from_str(&text).map_err(|e| damaged(&e.to_string()))?;
        let valid = file.format == KEY_FORMAT
            && file.kdf == "argon2id"
            && ARGON_MEMORY_RANGE.contains(&file.memory_kib)
            && ARGON_PASSES_RANGE.contains(&file.passes)
            && ARGON_LANES_RANGE.contains(&file.lanes);
        if !valid {
            return Err(damaged("unsupported key file"));
        }
        let salt = from_hex(&file.salt).ok_or_else(|| damaged("bad salt"))?;
        let wrapped = from_hex(&file.wrapped).ok_or_else(|| damaged("bad key"))?;
        let kek = derive(password, &salt, file.memory_kib, file.passes, file.lanes)?;
        let plain = decrypt_single(&wrapped, &to_hex(&kek)).map_err(|_| EngineError::WrongPassword)?;
        let master: [u8; 32] = plain.try_into().map_err(|_| damaged("bad key length"))?;
        Ok(Self::from_master(master))
    }

    /// Wraps the master key with `password` and replaces the key file
    /// (temporary file, then rename: never half written).
    pub(crate) fn write_key_file(&self, root: &Path, password: &Password) -> Result<()> {
        let mut salt = [0u8; 16];
        random(&mut salt)?;
        let kek = derive(password, &salt, ARGON_MEMORY_KIB, ARGON_PASSES, ARGON_LANES)?;
        let wrapped = encrypt_single(&self.master, WRAP_ENTRY, &to_hex(&kek))?;
        let file = KeyFile {
            format: KEY_FORMAT,
            kdf: "argon2id".into(),
            memory_kib: ARGON_MEMORY_KIB,
            passes: ARGON_PASSES,
            lanes: ARGON_LANES,
            salt: to_hex(&salt),
            wrapped: to_hex(&wrapped),
        };
        let text = serde_json::to_string_pretty(&file).expect("key file serializes");
        let tmp = root.join(format!("{KEY_FILE}.tmp"));
        crate::repo::write_durably(&tmp, text.as_bytes())?;
        let path = root.join(KEY_FILE);
        std::fs::rename(&tmp, &path).map_err(EngineError::io(format!("replacing {}", path.display())))
    }
}

/// Argon2id of `password`: 32 bytes.
fn derive(password: &Password, salt: &[u8], memory_kib: u32, passes: u32, lanes: u32) -> Result<[u8; 32]> {
    let bad =
        |e: argon2::Error| EngineError::Damaged { archive: KEY_FILE.to_owned(), message: e.to_string() };
    let params = Params::new(memory_kib, passes, lanes, Some(32)).map_err(bad)?;
    let mut out = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_str().as_bytes(), salt, &mut out)
        .map_err(bad)?;
    Ok(out)
}

fn random(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf).map_err(|e| EngineError::Io {
        context: "generating a random key".into(),
        source: std::io::Error::other(e.to_string()),
    })
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let t = text.as_bytes();
    if !t.len().is_multiple_of(2) {
        return None;
    }
    t.chunks(2)
        .map(|pair| std::str::from_utf8(pair).ok().and_then(|p| u8::from_str_radix(p, 16).ok()))
        .collect()
}

pub(crate) fn aes_options(password: &str) -> Options<'_> {
    Options::default().with_aes_encryption(AesMode::Aes256, password)
}

/// A zip in memory holding `data` as the encrypted entry `name`, undated.
pub(crate) fn encrypt_single(data: &[u8], name: &str, password: &str) -> Result<Vec<u8>> {
    let ctx = "encrypting";
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = aes_options(password)
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    zip.start_file(name, options).map_err(zip_error(ctx))?;
    zip.write_all(data).map_err(EngineError::io(ctx))?;
    Ok(zip.finish().map_err(zip_error(ctx))?.into_inner())
}

/// The first entry of an encrypted zip in memory.
pub(crate) fn decrypt_single(bytes: &[u8], password: &str) -> Result<Vec<u8>> {
    let ctx = "decrypting";
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(zip_error(ctx))?;
    let entry = zip.by_index_decrypt(0, password.as_bytes()).map_err(zip_error(ctx))?;
    // The zip library reads a plain entry even when given a password.
    if !entry.encrypted() {
        return Err(EngineError::Io {
            context: ctx.to_owned(),
            source: std::io::Error::other("the entry is not encrypted"),
        });
    }
    let mut out = Vec::new();
    let limit = crate::repo::MAX_MANIFEST_BYTES;
    entry.take(limit + 1).read_to_end(&mut out).map_err(|_| EngineError::WrongPassword)?;
    if out.len() as u64 > limit {
        return Err(EngineError::Io {
            context: ctx.to_owned(),
            source: std::io::Error::other("the entry unpacks too large"),
        });
    }
    Ok(out)
}

/// Maps a zip error; a refused password becomes [`EngineError::WrongPassword`].
pub(crate) fn zip_error(context: &str) -> impl FnOnce(ZipError) -> EngineError + '_ {
    move |e| match e {
        ZipError::InvalidPassword => EngineError::WrongPassword,
        // Kept as is, so a full disk is recognised.
        ZipError::Io(source) => EngineError::Io { context: context.to_owned(), source },
        other => EngineError::Io { context: context.to_owned(), source: std::io::Error::other(other) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pw(s: &str) -> Password {
        Password::new(s.to_owned()).unwrap()
    }

    /// The key of a password never changes with the argon2 crate: key files
    /// written before must still open. Expected value from the reference
    /// implementation (argon2-cffi, Argon2id v1.3).
    #[test]
    fn argon2id_matches_the_reference_implementation() {
        let key = derive(&pw("correct horse battery"), b"archeion-salt-01", 256, 2, 1).unwrap();
        assert_eq!(to_hex(&key), "da531be59561517857e112303d1fcb39130eee7922990404ab47a25fd819ed52");
    }

    #[test]
    fn single_entry_round_trips_and_wrong_password_fails() {
        let bytes = encrypt_single(b"secret text", "data", "correct horse").unwrap();
        assert!(!bytes.windows(6).any(|w| w == b"secret"), "content is not in clear");
        assert_eq!(decrypt_single(&bytes, "correct horse").unwrap(), b"secret text");
        assert!(decrypt_single(&bytes, "wrong").is_err());
    }

    #[test]
    fn notes_open_only_for_their_id_and_reveal_nothing() {
        let keys = Keys::from_master([7; 32]);
        let sealed = keys.seal_note("abc", "1234 5678 deadbeef");
        assert!(!sealed.contains("1234") && !sealed.contains("deadbeef"));
        assert_eq!(keys.open_note("abc", &sealed).as_deref(), Some("1234 5678 deadbeef"));
        assert_eq!(keys.open_note("abd", &sealed), None);
        assert_eq!(Keys::from_master([8; 32]).open_note("abc", &sealed), None);
        let mut flipped = sealed.into_bytes();
        flipped[0] = if flipped[0] == b'0' { b'1' } else { b'0' };
        assert_eq!(keys.open_note("abc", &String::from_utf8(flipped).unwrap()), None);
        assert_eq!(keys.open_note("abc", "zz"), None);
    }

    #[test]
    fn the_key_file_opens_with_the_password_only_and_survives_a_password_change() {
        let dir = tempfile::tempdir().unwrap();
        let k = Keys::create(dir.path(), &pw("pässword one")).unwrap();
        let again = Keys::unlock(dir.path(), &pw("pässword one")).unwrap();
        assert_eq!(k.password(), again.password());
        assert!(matches!(Keys::unlock(dir.path(), &pw("other")), Err(EngineError::WrongPassword)));

        k.write_key_file(dir.path(), &pw("second")).unwrap();
        assert!(matches!(Keys::unlock(dir.path(), &pw("pässword one")), Err(EngineError::WrongPassword)));
        assert_eq!(Keys::unlock(dir.path(), &pw("second")).unwrap().password(), k.password());
        let text = std::fs::read_to_string(dir.path().join(KEY_FILE)).unwrap();
        assert!(!text.contains(k.password()), "the master key is never in clear");
    }

    #[test]
    fn a_master_key_put_in_without_encryption_opens_with_no_password() {
        let dir = tempfile::tempdir().unwrap();
        Keys::create(dir.path(), &pw("the real one")).unwrap();
        // Someone who can write to the destination wraps a key of their own
        // in a plain zip: it must not open with any password.
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(WRAP_ENTRY, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(&[7u8; 32]).unwrap();
        let planted = to_hex(&zip.finish().unwrap().into_inner());
        let path = dir.path().join(KEY_FILE);
        let mut file: KeyFile = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        file.wrapped = planted;
        std::fs::write(&path, serde_json::to_string(&file).unwrap()).unwrap();
        assert!(matches!(Keys::unlock(dir.path(), &pw("anything")), Err(EngineError::WrongPassword)));
    }

    #[test]
    fn a_weakened_key_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        Keys::create(dir.path(), &pw("x")).unwrap();
        let path = dir.path().join(KEY_FILE);
        let text =
            std::fs::read_to_string(&path).unwrap().replace("\"memory_kib\": 65536", "\"memory_kib\": 16");
        std::fs::write(&path, text).unwrap();
        assert!(matches!(Keys::unlock(dir.path(), &pw("x")), Err(EngineError::Damaged { .. })));
    }

    #[test]
    fn names_depend_on_archive_and_path_and_hide_the_path() {
        let a = Keys::from_master([1; 32]);
        let b = Keys::from_master([2; 32]);
        let n = a.blob_name("2026-10-30_120000_C", "C/Users/me/report.docx");
        assert_eq!(n.len(), 64);
        assert!(!n.contains("report"));
        assert_ne!(n, b.blob_name("2026-10-30_120000_C", "C/Users/me/report.docx"));
        assert_ne!(n, a.blob_name("2026-10-31_120000_I", "C/Users/me/report.docx"));
        assert_ne!(n, a.blob_name("2026-10-30_120000_C", "C/Users/me/report2.docx"));
    }

    #[test]
    fn the_recovery_key_is_the_zip_password_in_groups() {
        let k = Keys::from_master([0xab; 32]);
        assert_eq!(k.recovery_key().replace(' ', ""), k.password());
        assert_eq!(k.recovery_key().split(' ').count(), 16);
    }

    #[test]
    fn a_recovery_key_gives_back_the_same_keys() {
        let k = Keys::from_master([0x5c; 32]);
        let typed = k.recovery_key().to_uppercase();
        let back = Keys::from_recovery_key(&typed).unwrap();
        assert_eq!(back.password(), k.password());
        assert_eq!(back.chunk_id(b"x"), k.chunk_id(b"x"));
        assert!(Keys::from_recovery_key("not a key").is_none());
        assert!(Keys::from_recovery_key(&"ab".repeat(31)).is_none());
    }

    #[test]
    fn chunk_names_are_keyed() {
        let a = Keys::from_master([1; 32]);
        let b = Keys::from_master([2; 32]);
        assert_ne!(a.chunk_id(b"same"), b.chunk_id(b"same"));
        assert_ne!(a.chunk_id(b"same"), *blake3::hash(b"same").as_bytes());
        assert_eq!(a.chunk_id(b"same"), a.chunk_id(b"same"));
    }

    #[test]
    fn hex_round_trips_and_rejects_garbage() {
        assert_eq!(from_hex(&to_hex(&[0, 15, 255])), Some(vec![0, 15, 255]));
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
    }

    #[test]
    fn debug_hides_the_password() {
        assert!(!format!("{:?}", pw("hunter2")).contains("hunter2"));
        assert_eq!(Password::new(String::new()), None);
    }
}
