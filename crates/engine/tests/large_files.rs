//! End-to-end tests of files saved as chunks (see `docs/large-files.md`),
//! with small chunks so the files stay small.

use archeion_core::{
    ArchiveKind, BackupMode, Container, Destination, FileAction, FileDone, LargeFiles, Manifest, Profile,
    Retention, RunEvent,
};
use archeion_engine::{
    ChunkParams, EngineError, EventSink, ExtractRequest, Password, RestoreRequest, RestoreTarget, Secret,
    Tuning, archive_manifest, backup_tuned, delete_archive, extract, file_versions, list_archives, merge,
    merge_range, recovery_key, restore, verify,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread::sleep,
    time::Duration,
};

#[derive(Default)]
struct Recorder(Mutex<Vec<RunEvent>>);

impl EventSink for Recorder {
    fn emit(&self, event: RunEvent) {
        self.0.lock().unwrap().push(event);
    }
}

const MB: usize = 1024 * 1024;

fn small_chunks() -> Tuning {
    Tuning { chunks: ChunkParams::new(1024, 4096, 16384), ..Tuning::default() }
}

/// Deterministic pseudo-random bytes (xorshift).
fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x.to_le_bytes()[0]
        })
        .collect()
}

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    restore_to: PathBuf,
    profile: Profile,
    password: Option<Password>,
}

/// A source with `a.txt`, `sub/b.txt` and the 2 MB `big.bin`; files of at
/// least 1 MB are chunked.
fn fixture(container: Container, encrypted: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("a.txt"), "alpha").unwrap();
    fs::write(source.join("sub").join("b.txt"), "beta").unwrap();
    fs::write(source.join("big.bin"), noise(2 * MB, 1)).unwrap();
    let mut profile = Profile::new("Large");
    profile.sources.push(source.clone());
    profile.destination = Destination::Local { path: dir.path().join("backups") };
    profile.container = container;
    profile.encrypted = encrypted;
    profile.large_files = LargeFiles { threshold_mb: 1 };
    profile.merge.rules = Retention::KEEP_ALL;
    let password = encrypted.then(|| Password::new("correct horse battery".into()).unwrap());
    let restore_to = dir.path().join("restored");
    Fixture { _dir: dir, source, restore_to, profile, password }
}

impl Fixture {
    fn pw(&self) -> Option<&Password> {
        self.password.as_ref()
    }

    fn backup(&self, mode: BackupMode) -> archeion_core::BackupReport {
        // Archive names have one-second resolution.
        sleep(Duration::from_millis(1100));
        backup_tuned(&self.profile, self.pw(), mode, &Recorder::default(), small_chunks()).unwrap()
    }

    /// Archive names, oldest first.
    fn names(&self) -> Vec<String> {
        list_archives(&self.profile, self.pw()).unwrap().into_iter().rev().map(|a| a.name).collect()
    }

    fn kinds(&self) -> String {
        list_archives(&self.profile, self.pw())
            .unwrap()
            .iter()
            .rev()
            .map(|a| if a.kind == ArchiveKind::Complete { 'C' } else { 'I' })
            .collect()
    }

    fn manifest(&self, archive: &str) -> Manifest {
        archive_manifest(&self.profile, self.pw(), archive).unwrap()
    }

    fn key(&self, relative: &str) -> String {
        archeion_core::keys::key_for(&self.source.join(relative)).unwrap()
    }

    fn archive_dir(&self, archive: &str) -> PathBuf {
        self.profile.destination.path().join(archive)
    }

    /// Restores everything of `archive` into a fresh folder.
    fn restore_all(&self, archive: &str) {
        let _ = fs::remove_dir_all(&self.restore_to);
        let req = RestoreRequest::single(
            archive,
            Vec::new(),
            &RestoreTarget::Folder { path: self.restore_to.clone() },
            true,
        );
        let r = restore(&self.profile, self.pw(), &req, &Recorder::default()).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    }

    fn restored(&self, relative: &str) -> Vec<u8> {
        fs::read(archeion_core::keys::path_under(&self.restore_to, &self.key(relative))).unwrap()
    }

    fn verify_clean(&self) {
        let v = verify(&self.profile, self.pw(), None, &Recorder::default()).unwrap();
        assert!(v.problems.is_empty(), "{:?}", v.problems);
        let s = verify(&self.profile, self.pw(), Some(50), &Recorder::default()).unwrap();
        assert!(s.problems.is_empty(), "{:?}", s.problems);
    }
}

/// Every file below `dir`.
fn files_below(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn chunk_files(archive_dir: &Path) -> Vec<PathBuf> {
    files_below(&archive_dir.join("chunks"))
}

/// `data` with `with` written over `len` bytes at `at`.
fn overwrite(data: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    out[at..at + with.len()].copy_from_slice(with);
    out
}

/// `data` with `with` inserted at `at`.
fn insert(data: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    out.splice(at..at, with.iter().copied());
    out
}

fn check_both_containers(test: impl Fn(Container)) {
    test(Container::ZipPerFile);
    test(Container::Folder);
}

#[test]
fn an_edit_in_the_middle_stores_only_a_few_chunks_and_every_date_restores() {
    check_both_containers(|c| {
        let f = fixture(c, false);
        let v1 = noise(2 * MB, 1);
        let first = f.backup(BackupMode::Auto);
        assert!(first.complete);
        let m = f.manifest(&first.archive);
        let big = m.get(&f.key("big.bin")).unwrap();
        assert!(big.whole_copy() && big.chunks.as_ref().is_some_and(|c| c.whole), "whole copy with its list");
        assert!(m.get(&f.key("a.txt")).unwrap().chunks.is_none(), "small files stay as they were");
        assert!(chunk_files(&f.archive_dir(&first.archive)).is_empty(), "no chunk file in a complete backup");

        let v2 = insert(&v1, MB, b"a few bytes inserted in the middle of the file");
        fs::write(f.source.join("big.bin"), &v2).unwrap();
        let second = f.backup(BackupMode::Auto);
        assert!(!second.complete);
        assert_eq!((second.files_changed, second.files_as_changes), (1, 1));
        assert!(second.bytes_copied < 64 * 1024, "copied {} bytes", second.bytes_copied);
        let new_chunks = chunk_files(&f.archive_dir(&second.archive)).len();
        assert!((1..=4).contains(&new_chunks), "{new_chunks} new chunks");
        let e = f.manifest(&second.archive).get(&f.key("big.bin")).cloned().unwrap();
        assert!(e.chunks_only() && e.blob.is_none());

        let v3 = overwrite(&v2, 100, b"edited near the start");
        fs::write(f.source.join("big.bin"), &v3).unwrap();
        f.backup(BackupMode::Auto);

        let names = f.names();
        for (name, expected) in names.iter().zip([&v1, &v2, &v3]) {
            f.restore_all(name);
            assert_eq!(&f.restored("big.bin"), expected, "big.bin at {name}");
            assert_eq!(f.restored("a.txt"), b"alpha");
        }
        assert_eq!(file_versions(&f.profile, None, &f.key("big.bin")).unwrap().len(), 3);
        f.verify_clean();
    });
}

#[test]
fn the_threshold_decides_which_files_are_chunked() {
    let mut f = fixture(Container::ZipPerFile, false);
    fs::write(f.source.join("just below.bin"), noise(MB - 1, 2)).unwrap();
    fs::write(f.source.join("exactly.bin"), noise(MB, 3)).unwrap();
    f.backup(BackupMode::Auto);
    fs::write(f.source.join("just below.bin"), noise(MB - 1, 4)).unwrap();
    fs::write(f.source.join("exactly.bin"), overwrite(&noise(MB, 3), 10, b"x")).unwrap();
    let r = f.backup(BackupMode::Auto);
    assert_eq!(r.files_as_changes, 1);
    let m = f.manifest(&r.archive);
    let below = m.get(&f.key("just below.bin")).unwrap();
    assert!(below.stored && below.chunks.is_none(), "below the threshold: a whole zip");
    assert!(m.get(&f.key("exactly.bin")).unwrap().chunks_only(), "at the threshold: chunks");

    // "Never": changed files are whole again, even large ones.
    f.profile.large_files = LargeFiles::NEVER;
    fs::write(f.source.join("big.bin"), noise(2 * MB, 5)).unwrap();
    let r = f.backup(BackupMode::Auto);
    assert_eq!(r.files_as_changes, 0);
    let m = f.manifest(&r.archive);
    assert!(m.get(&f.key("big.bin")).unwrap().chunks.is_none());
    for name in f.names() {
        f.restore_all(&name);
    }
    f.verify_clean();
}

#[test]
fn an_encrypted_destination_chunks_every_file_and_hides_the_chunks() {
    use std::io::Read as _;
    let f = fixture(Container::ZipPerFile, true);
    let first = f.backup(BackupMode::Auto);
    let m = f.manifest(&first.archive);
    assert!(
        m.get(&f.key("a.txt")).unwrap().chunks.as_ref().is_some_and(|c| c.whole),
        "listed in the complete"
    );

    fs::write(f.source.join("a.txt"), "alpha, second version").unwrap();
    let second = f.backup(BackupMode::Auto);
    assert_eq!(second.files_as_changes, 1);
    let dir = f.archive_dir(&second.archive);
    let chunks = chunk_files(&dir);
    assert_eq!(chunks.len(), 1, "one small file, one chunk");
    let entry = f.manifest(&second.archive).get(&f.key("a.txt")).cloned().unwrap();
    let list = &entry.chunks.as_ref().unwrap().list;
    let plain_hash = blake3::hash(b"alpha, second version").to_hex().to_string();
    assert_ne!(list[0].id.to_string(), plain_hash, "chunk names are keyed");
    for p in files_below(f.profile.destination.path()) {
        let bytes = fs::read(&p).unwrap();
        assert!(!bytes.windows(6).any(|w| w == b"second"), "content in clear: {}", p.display());
        let name = p.to_string_lossy().into_owned();
        assert!(!name.contains("a.txt") && !name.contains(&plain_hash), "name visible: {name}");
    }

    // What 7-Zip does with the recovery key: open the chunk named in the
    // manifest and read the content.
    let key = recovery_key(&f.profile, f.pw().unwrap()).unwrap().unwrap().replace(' ', "");
    let mut z = zip::ZipArchive::new(fs::File::open(&chunks[0]).unwrap()).unwrap();
    let mut content = String::new();
    z.by_index_decrypt(0, key.as_bytes()).unwrap().read_to_string(&mut content).unwrap();
    assert_eq!(content, "alpha, second version");
    assert!(chunks[0].to_string_lossy().contains(&list[0].id.to_string()));

    let names = f.names();
    f.restore_all(&names[0]);
    assert_eq!(f.restored("a.txt"), b"alpha");
    f.restore_all(&names[1]);
    assert_eq!(f.restored("a.txt"), b"alpha, second version");
    assert_eq!(f.restored("big.bin"), noise(2 * MB, 1));
    f.verify_clean();
}

/// C, then three incrementals editing `big.bin` in two places, the third
/// reusing a chunk the first one introduced.
fn edited_three_times(f: &Fixture) -> [Vec<u8>; 4] {
    let v1 = noise(2 * MB, 1);
    f.backup(BackupMode::Auto);
    let v2 = overwrite(&v1, MB, b"first edit, in the middle");
    fs::write(f.source.join("big.bin"), &v2).unwrap();
    f.backup(BackupMode::Auto);
    let v3 = overwrite(&v2, 200_000, b"second edit, near the start");
    fs::write(f.source.join("big.bin"), &v3).unwrap();
    f.backup(BackupMode::Auto);
    let v4 = insert(&v3, 1_800_000, b"third edit, near the end");
    fs::write(f.source.join("big.bin"), &v4).unwrap();
    f.backup(BackupMode::Auto);
    [v1, v2, v3, v4]
}

#[test]
fn the_automatic_merge_keeps_the_chunks_later_backups_need() {
    let mut f = fixture(Container::ZipPerFile, false);
    let [v1, _, _, v4] = edited_three_times(&f);
    f.profile.merge.rules = Retention { keep_last: 1, ..Retention::KEEP_ALL };
    let report = merge(&f.profile, None, &Recorder::default()).unwrap();
    assert_eq!((report.archives_before, report.archives_after), (4, 2));
    assert_eq!(f.kinds(), "CI");
    let names = f.names();
    f.restore_all(&names[1]);
    assert_eq!(f.restored("big.bin"), v4, "the first edit's chunk was carried into the merged backup");
    f.restore_all(&names[0]);
    assert_eq!(f.restored("big.bin"), v1);
    f.verify_clean();
}

#[test]
fn a_merge_of_middle_backups_keeps_what_the_newest_needs() {
    let f = fixture(Container::Folder, false);
    let [_, _, v3, v4] = edited_three_times(&f);
    let names = f.names();
    // Fuse I1 and I2: I3 lists chunks that only I1 and I2 hold.
    merge_range(&f.profile, None, &names[1], &names[2], &Recorder::default()).unwrap();
    let after = f.names();
    assert_eq!(after.len(), 3);
    f.restore_all(&after[1]);
    assert_eq!(f.restored("big.bin"), v3);
    f.restore_all(&after[2]);
    assert_eq!(f.restored("big.bin"), v4);
    f.verify_clean();
}

#[test]
fn a_merge_into_a_complete_backup_puts_files_back_together_whole() {
    let f = fixture(Container::ZipPerFile, false);
    let [_, _, v3, v4] = edited_three_times(&f);
    let names = f.names();
    merge_range(&f.profile, None, &names[0], &names[2], &Recorder::default()).unwrap();
    assert_eq!(f.kinds(), "CI");
    let after = f.names();
    let m = f.manifest(&after[0]);
    let big = m.get(&f.key("big.bin")).unwrap();
    assert!(big.whole_copy() && big.chunks.as_ref().is_some_and(|c| c.whole), "whole again, list kept");
    f.restore_all(&after[0]);
    assert_eq!(f.restored("big.bin"), v3);
    f.restore_all(&after[1]);
    assert_eq!(f.restored("big.bin"), v4, "the next backup still finds its chunks");
    f.verify_clean();
}

#[test]
fn deleting_a_middle_backup_keeps_the_later_ones_whole() {
    let f = fixture(Container::ZipPerFile, true);
    let [_, _, v3, v4] = edited_three_times(&f);
    let names = f.names();
    delete_archive(&f.profile, f.pw(), &names[1], &Recorder::default()).unwrap();
    let after = f.names();
    assert_eq!(after, vec![names[0].clone(), names[2].clone(), names[3].clone()]);
    f.restore_all(&names[2]);
    assert_eq!(f.restored("big.bin"), v3);
    f.restore_all(&names[3]);
    assert_eq!(f.restored("big.bin"), v4);
    delete_archive(&f.profile, f.pw(), &names[3], &Recorder::default()).unwrap();
    f.verify_clean();
}

#[test]
fn the_check_finds_a_damaged_or_missing_chunk() {
    let f = fixture(Container::Folder, false);
    f.backup(BackupMode::Auto);
    fs::write(f.source.join("big.bin"), overwrite(&noise(2 * MB, 1), MB, b"edit")).unwrap();
    let second = f.backup(BackupMode::Auto);
    f.verify_clean();
    let chunk = chunk_files(&f.archive_dir(&second.archive)).pop().unwrap();
    let good = fs::read(&chunk).unwrap();
    fs::write(&chunk, overwrite(&good, 0, b"X")).unwrap();
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert_eq!(v.problems.len(), 1, "{:?}", v.problems);
    let req = RestoreRequest::single(
        &second.archive,
        vec![f.key("big.bin")],
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert_eq!(r.skipped.len(), 1, "a damaged chunk is never restored");

    fs::remove_file(&chunk).unwrap();
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.iter().any(|p| p.contains("missing")), "{:?}", v.problems);
    let sample = verify(&f.profile, None, Some(50), &Recorder::default()).unwrap();
    assert!(!sample.problems.is_empty(), "the restore test reads the chunks too");
}

#[test]
fn an_interrupted_chunked_backup_reuses_its_chunks() {
    let f = fixture(Container::ZipPerFile, false);
    f.backup(BackupMode::Auto);
    // A new large file, copied first (names sort), then a stop.
    let fresh = noise(MB + 12_345, 9);
    fs::write(f.source.join("0 fresh.bin"), &fresh).unwrap();
    fs::write(f.source.join("big.bin"), overwrite(&noise(2 * MB, 1), MB, b"edit")).unwrap();
    sleep(Duration::from_millis(1100));
    let stop = Tuning { stop_after_files: Some(1), ..small_chunks() };
    assert!(backup_tuned(&f.profile, None, BackupMode::Auto, &Recorder::default(), stop).is_err());
    let partial = fs::read_dir(f.profile.destination.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().ends_with(".partial"))
        .unwrap();
    let mut written = chunk_files(&partial);
    assert!(written.len() > 100, "the new file's chunks are in the unfinished backup");
    // One of them is damaged: it is written again, the others reused.
    let damaged = written.pop().unwrap();
    fs::write(&damaged, b"garbage").unwrap();

    let r = backup_tuned(&f.profile, None, BackupMode::Auto, &Recorder::default(), small_chunks()).unwrap();
    assert_eq!(r.files_as_changes, 2);
    assert!(r.bytes_copied < 100_000, "copied {} bytes again", r.bytes_copied);
    f.restore_all(&r.archive);
    assert_eq!(f.restored("0 fresh.bin"), fresh);
    f.verify_clean();
}

#[test]
fn a_destination_of_an_older_version_starts_a_new_chain_first() {
    let mut f = fixture(Container::ZipPerFile, false);
    f.profile.large_files = LargeFiles::NEVER;
    f.backup(BackupMode::Auto);
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    f.backup(BackupMode::Auto);
    // Make the backups look written by a version without chunks.
    for name in f.names() {
        let path = f.archive_dir(&name).join("manifest.jsonl");
        let text = fs::read_to_string(&path).unwrap().replacen("{\"format\":2,", "{\"format\":1,", 1);
        fs::write(&path, text).unwrap();
    }
    fs::write(f.source.join("big.bin"), noise(2 * MB, 7)).unwrap();
    // Chunks still off: the chain goes on.
    assert!(!f.backup(BackupMode::Auto).complete);
    for name in f.names() {
        let path = f.archive_dir(&name).join("manifest.jsonl");
        let text = fs::read_to_string(&path).unwrap().replacen("{\"format\":2,", "{\"format\":1,", 1);
        fs::write(&path, text).unwrap();
    }
    f.profile.large_files = LargeFiles { threshold_mb: 1 };
    // An incremental the user asks for stays whole, in the old chain.
    fs::write(f.source.join("big.bin"), noise(2 * MB, 8)).unwrap();
    let asked = f.backup(BackupMode::Incremental);
    assert!(!asked.complete && asked.files_as_changes == 0);
    // The automatic one starts a new chain, after which files are chunked.
    let auto = f.backup(BackupMode::Auto);
    assert!(auto.complete, "a complete backup first");
    fs::write(f.source.join("big.bin"), overwrite(&noise(2 * MB, 8), 5, b"edit")).unwrap();
    let next = f.backup(BackupMode::Auto);
    assert!(!next.complete && next.files_as_changes == 1);
    assert_eq!(f.kinds(), "CIIICI");
    for name in f.names() {
        f.restore_all(&name);
    }
    f.verify_clean();
}

#[test]
fn extract_restores_from_the_destination_alone() {
    let f = fixture(Container::Folder, false);
    let [_, v2, _, v4] = edited_three_times(&f);
    let names = f.names();
    let out = f.restore_to.join("extracted");
    let request = |archive: Option<String>, path: Option<String>| ExtractRequest {
        destination: f.profile.destination.path().to_path_buf(),
        archive,
        path,
        to: out.clone(),
        overwrite: true,
        secret: None,
    };
    let r = extract(&request(None, None), &Recorder::default()).unwrap();
    assert!(r.skipped.is_empty() && r.files_restored == 3, "{r:?}");
    let big = archeion_core::keys::path_under(&out, &f.key("big.bin"));
    assert_eq!(fs::read(&big).unwrap(), v4, "the newest by default");

    let only = Some(f.source.join("big.bin").to_string_lossy().into_owned());
    let r = extract(&request(Some(names[1].clone()), only), &Recorder::default()).unwrap();
    assert_eq!(r.files_restored, 1);
    assert_eq!(fs::read(&big).unwrap(), v2);

    let missing = Some(f.source.join("nothing here").to_string_lossy().into_owned());
    assert!(matches!(
        extract(&request(None, missing), &Recorder::default()),
        Err(EngineError::InvalidProfile(_))
    ));
    let wrong = ExtractRequest { destination: f.restore_to.join("nowhere"), ..request(None, None) };
    assert!(extract(&wrong, &Recorder::default()).is_err());
}

#[test]
fn extract_opens_an_encrypted_destination_with_the_password_or_the_recovery_key() {
    let f = fixture(Container::ZipPerFile, true);
    f.backup(BackupMode::Auto);
    fs::write(f.source.join("a.txt"), "alpha 2").unwrap();
    f.backup(BackupMode::Auto);
    let key = recovery_key(&f.profile, f.pw().unwrap()).unwrap().unwrap();
    let out = f.restore_to.join("extracted");
    let request = |secret: Option<Secret>| ExtractRequest {
        destination: f.profile.destination.path().to_path_buf(),
        archive: None,
        path: None,
        to: out.clone(),
        overwrite: true,
        secret,
    };
    let a = archeion_core::keys::path_under(&out, &f.key("a.txt"));
    for secret in [
        Secret::Password(f.pw().unwrap().clone()),
        Secret::RecoveryKey(key.clone()),
        Secret::Either(key.to_uppercase()),
        Secret::Either("correct horse battery".into()),
    ] {
        let _ = fs::remove_dir_all(&out);
        let r = extract(&request(Some(secret)), &Recorder::default()).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!(fs::read(&a).unwrap(), b"alpha 2");
    }
    assert!(matches!(extract(&request(None), &Recorder::default()), Err(EngineError::PasswordNeeded)));
    let wrong = Some(Secret::Either("not it".into()));
    assert!(matches!(extract(&request(wrong), &Recorder::default()), Err(EngineError::WrongPassword)));
    let bad_key = Some(Secret::RecoveryKey("00".repeat(32)));
    assert!(matches!(extract(&request(bad_key), &Recorder::default()), Err(EngineError::WrongPassword)));
}

#[test]
fn the_file_list_says_a_large_file_was_saved_as_changes_and_how_much_was_written() {
    let f = fixture(Container::Folder, false);
    f.backup(BackupMode::Auto);
    let v2 = overwrite(&noise(2 * MB, 1), MB, b"edited in the middle");
    fs::write(f.source.join("big.bin"), &v2).unwrap();
    sleep(Duration::from_millis(1100));
    let r = Recorder::default();
    let report = backup_tuned(&f.profile, None, BackupMode::Auto, &r, small_chunks()).unwrap();
    let files: Vec<FileDone> =
        r.0.lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                RunEvent::Files { files } => Some(files.clone()),
                _ => None,
            })
            .flatten()
            .collect();
    assert_eq!(files.len(), 1, "only big.bin changed: {files:?}");
    let big = &files[0];
    assert_eq!((big.action, big.bytes), (FileAction::Changed, 2 * MB as u64));
    assert!(big.path.ends_with("big.bin"));
    let written = big.new_chunk_bytes.expect("saved as changes");
    assert!(written > 0 && written < (2 * MB) as u64 / 4, "{written} bytes of new chunks");
    assert_eq!(report.files_as_changes, 1);
}
