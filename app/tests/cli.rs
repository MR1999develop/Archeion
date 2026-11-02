//! End-to-end tests of the executable: `archeion run`, `restore`, `verify`,
//! `merge`, `delete` and `extract` on real folders, as Task Scheduler and
//! the window start them.
//!
//! Each test builds a sandbox in a temporary folder: source files, a
//! destination, and a data folder holding `settings.toml` with one profile,
//! named by `ARCHEION_DATA_DIR` so the user's settings, saved passwords and
//! scheduled tasks are never touched. The password of an encrypted profile
//! is passed in `ARCHEION_PASSWORD` (the profile id is new, so none is
//! saved). Restored folders are compared file by file with what the source
//! held when each backup ran.

use archeion_core::{
    Container, Destination, Profile, RestoreItem, RestoreRequest, RestoreTarget, Retention, Settings, keys,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread::sleep,
    time::Duration,
};

const PASSWORD: &str = "correct horse battery staple";
/// Exit codes of the executable (see `worker.rs`).
const EXIT_FAILED: i32 = 1;
const EXIT_PASSWORD: i32 = 4;

struct Sandbox {
    _dir: tempfile::TempDir,
    root: PathBuf,
    data: PathBuf,
    source: PathBuf,
    profile: Profile,
}

/// Source files with nested folders, accents, an empty folder and a file
/// above the large-file threshold (saved as chunks).
fn sandbox(container: Container, encrypted: bool) -> Sandbox {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let source = root.join("source folder");
    write(&source.join("a.txt"), b"alpha");
    write(&source.join("ünicode è.txt"), "accents ü è".as_bytes());
    write(&source.join("sub").join("b.txt"), b"beta");
    write(&source.join("sub").join("deep").join("c.txt"), b"gamma");
    write(&source.join("big.bin"), &big(3 * 1024 * 1024, 7));
    fs::create_dir_all(source.join("empty")).unwrap();

    let mut profile = Profile::new("End to end");
    profile.sources.push(source.clone());
    profile.destination = Destination::Local { path: root.join("backups") };
    profile.container = container;
    profile.encrypted = encrypted;
    profile.large_files.threshold_mb = 1;
    profile.merge.rules = Retention::KEEP_ALL;
    let data = root.join("data");
    fs::create_dir_all(&data).unwrap();
    let settings = Settings { profiles: vec![profile.clone()], ..Settings::default() };
    fs::write(data.join("settings.toml"), toml::to_string_pretty(&settings).unwrap()).unwrap();
    Sandbox { _dir: dir, root, data, source, profile }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// Bytes that do not repeat, so chunks differ.
fn big(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x.to_le_bytes()[0]
        })
        .collect()
}

/// Runs the executable in the sandbox; `password` goes in the environment.
fn archeion(s: &Sandbox, args: &[&str], password: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_archeion"));
    cmd.args(args).env("ARCHEION_DATA_DIR", &s.data).env_remove("ARCHEION_PASSWORD");
    cmd.env_remove("ARCHEION_RECOVERY_KEY");
    if let Some(p) = password {
        cmd.env("ARCHEION_PASSWORD", p);
    }
    cmd.output().unwrap()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

/// Fails with the output of the run when it did not succeed.
fn ok(out: &Output) {
    assert_eq!(
        code(out),
        0,
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The `event` names of the JSON lines on stdout.
fn events(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| v.get("event").and_then(|e| e.as_str()).map(str::to_owned))
        .collect()
}

fn backup(s: &Sandbox, password: Option<&str>) {
    let id = s.profile.id.to_string();
    let out = archeion(s, &["run", &id], password);
    ok(&out);
    assert!(events(&out).iter().any(|e| e == "finished"), "{:?}", events(&out));
    // Backup names have one-second precision.
    sleep(Duration::from_millis(1100));
}

/// Backup names, newest first.
fn archives(s: &Sandbox, password: Option<&str>) -> Vec<String> {
    let pw = password.map(|p| archeion_engine::Password::new(p.to_owned()).unwrap());
    archeion_engine::list_archives(&s.profile, pw.as_ref()).unwrap().into_iter().map(|a| a.name).collect()
}

fn key(s: &Sandbox, relative: &str) -> String {
    let path = if relative.is_empty() { s.source.clone() } else { s.source.join(relative) };
    keys::key_for(&path).unwrap()
}

/// Restores `items` through a request file, as the window does.
fn restore(s: &Sandbox, items: Vec<RestoreItem>, password: Option<&str>) -> Output {
    let request = s.root.join("request.json");
    fs::write(&request, serde_json::to_string(&RestoreRequest { items }).unwrap()).unwrap();
    let id = s.profile.id.to_string();
    let out = archeion(s, &["restore", &id, "--request", request.to_str().unwrap()], password);
    assert!(!request.exists(), "the request file is deleted once read");
    out
}

fn item(archive: &str, key: String, to: &Path, exclude: Vec<String>) -> RestoreItem {
    RestoreItem {
        archive: archive.to_owned(),
        key: Some(key),
        target: RestoreTarget::Folder { path: to.to_path_buf() },
        overwrite: false,
        exclude,
    }
}

/// Every file below `dir` with its content, by relative path with `/`.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    out
}

/// What a restore into `to` left of the source folder.
fn restored(s: &Sandbox, to: &Path) -> BTreeMap<String, Vec<u8>> {
    snapshot(&keys::path_under(to, &key(s, "")))
}

fn without(mut files: BTreeMap<String, Vec<u8>>, prefix: &str) -> BTreeMap<String, Vec<u8>> {
    files.retain(|k, _| k != prefix && !k.starts_with(&format!("{prefix}/")));
    files
}

/// Two backups with changes between them, then a restore at each date.
fn backup_change_restore(container: Container, encrypted: bool) {
    let pw = encrypted.then_some(PASSWORD);
    let s = sandbox(container, encrypted);
    let first = snapshot(&s.source);
    backup(&s, pw);

    write(&s.source.join("a.txt"), b"alpha v2");
    fs::remove_file(s.source.join("sub").join("b.txt")).unwrap();
    write(&s.source.join("sub").join("new.txt"), b"new");
    let mut changed = big(3 * 1024 * 1024, 7);
    changed[1_500_000] ^= 0xFF;
    write(&s.source.join("big.bin"), &changed);
    let second = snapshot(&s.source);
    backup(&s, pw);

    let names = archives(&s, pw);
    assert_eq!(names.len(), 2, "{names:?}");
    let (newest, oldest) = (&names[0], &names[1]);

    let to_old = s.root.join("restored old");
    ok(&restore(&s, vec![item(oldest, key(&s, ""), &to_old, vec![])], pw));
    assert_eq!(restored(&s, &to_old), first, "the source as at the first backup");

    let to_new = s.root.join("restored new");
    let exclude = vec![key(&s, "sub/deep"), key(&s, "a.txt")];
    ok(&restore(&s, vec![item(newest, key(&s, ""), &to_new, exclude)], pw));
    let expected = without(without(second, "sub/deep"), "a.txt");
    assert_eq!(restored(&s, &to_new), expected, "the newest backup minus what was unticked");
}

#[test]
fn folder_backups_restore_any_date_with_exclusions() {
    backup_change_restore(Container::Folder, false);
}

#[test]
fn zip_backups_restore_any_date_with_exclusions() {
    backup_change_restore(Container::ZipPerFile, false);
}

#[test]
fn encrypted_backups_restore_any_date_with_exclusions() {
    backup_change_restore(Container::ZipPerFile, true);
}

#[test]
fn an_encrypted_profile_needs_the_right_password() {
    let s = sandbox(Container::ZipPerFile, true);
    let id = s.profile.id.to_string();
    let out = archeion(&s, &["run", &id], None);
    assert_eq!(code(&out), EXIT_PASSWORD);
    assert!(events(&out).iter().any(|e| e == "password_needed"), "{:?}", events(&out));
    backup(&s, Some(PASSWORD));
    let out = archeion(&s, &["run", &id], Some("wrong password here"));
    assert_eq!(code(&out), EXIT_PASSWORD);
    assert!(events(&out).iter().any(|e| e == "wrong_password"), "{:?}", events(&out));
}

#[test]
fn extract_needs_only_the_destination_folder() {
    for encrypted in [false, true] {
        let pw = encrypted.then_some(PASSWORD);
        let s = sandbox(Container::ZipPerFile, encrypted);
        let files = snapshot(&s.source);
        backup(&s, pw);
        let to = s.root.join("extracted");
        let dest = s.profile.destination.path().to_string_lossy().into_owned();
        let out = archeion(&s, &["extract", "--destination", &dest, "--to", to.to_str().unwrap()], pw);
        ok(&out);
        assert_eq!(restored(&s, &to), files, "encrypted: {encrypted}");
    }
}

#[test]
fn verify_merge_and_delete_keep_the_backups_restorable() {
    let s = sandbox(Container::Folder, false);
    let id = s.profile.id.to_string();
    backup(&s, None);
    for n in 1..=3 {
        write(&s.source.join("a.txt"), format!("alpha v{n}").as_bytes());
        backup(&s, None);
    }
    let last = snapshot(&s.source);
    ok(&archeion(&s, &["verify", &id], None));
    ok(&archeion(&s, &["verify", &id, "--sample", "2"], None));

    let names = archives(&s, None);
    assert_eq!(names.len(), 4);
    // Merge the two middle incrementals into one.
    ok(&archeion(&s, &["merge", &id, "--from", &names[2], "--to", &names[1]], None));
    let names = archives(&s, None);
    assert_eq!(names.len(), 3, "{names:?}");
    ok(&archeion(&s, &["delete", &id, "--archive", &names[1]], None));
    let names = archives(&s, None);
    assert_eq!(names.len(), 2, "{names:?}");
    ok(&archeion(&s, &["verify", &id], None));

    let to = s.root.join("restored");
    ok(&restore(&s, vec![item(&names[0], key(&s, ""), &to, vec![])], None));
    assert_eq!(restored(&s, &to), last, "the newest backup is whole after merge and delete");
}

#[test]
fn an_unknown_profile_fails_without_touching_anything() {
    let s = sandbox(Container::Folder, false);
    let out = archeion(&s, &["run", &archeion_core::ProfileId::new_random().to_string()], None);
    assert_eq!(code(&out), EXIT_FAILED);
    assert!(events(&out).iter().any(|e| e == "failed"), "{:?}", events(&out));
    assert!(!s.profile.destination.path().exists());
}
