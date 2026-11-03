//! Deep tests of the engine: random histories with a printed seed,
//! interruptions at every file, every crash state of a merge, doctored
//! manifests and damaged backups. Each test checks one promise against an
//! oracle kept by the test itself: a snapshot of the source taken at each
//! backup.
//!
//! `ARCHEION_SEED=<n>` replays one seed; `ARCHEION_DEEP=<n>` runs `n` seeds
//! per test instead of the default two (slow: about twenty seconds per
//! seed). In PowerShell: `$env:ARCHEION_DEEP = 20; cargo test -p
//! archeion_engine --test deep`.

use archeion_core::{BackupMode, Container, Destination, Profile, Retention, RunEvent, keys, run::Phase};
use archeion_engine::{
    ChunkParams, EngineError, EventSink, Password, RestoreRequest, RestoreTarget, Tuning, backup_tuned,
    delete_archive, list_archives, merge, merge_range, restore, verify,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::sleep,
    time::Duration,
};

// ---------------------------------------------------------------- helpers

/// xorshift64*: reproducible from the printed seed, no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n.max(1) as u64)).unwrap()
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| u8::try_from(self.next() >> 56).unwrap()).collect()
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() { None } else { items.get(self.below(items.len())) }
    }
}

/// The seeds to run: `ARCHEION_SEED`, or `ARCHEION_DEEP` of them, or two.
fn seeds(salt: u64) -> Vec<u64> {
    if let Some(s) = std::env::var("ARCHEION_SEED").ok().and_then(|s| s.parse().ok()) {
        return vec![s];
    }
    let n = std::env::var("ARCHEION_DEEP").ok().and_then(|s| s.parse().ok()).unwrap_or(2u64);
    (1..=n).map(|i| i * 7919 + salt).collect()
}

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

/// Asks to stop at file `after` of the working phase (copying, restoring,
/// merging): the engine asks once before each file.
struct StopAfter {
    after: u64,
    working: Mutex<bool>,
    started: AtomicU64,
}

impl StopAfter {
    fn new(after: u64) -> Self {
        Self { after, working: Mutex::new(false), started: AtomicU64::new(0) }
    }
}

impl EventSink for StopAfter {
    fn emit(&self, event: RunEvent) {
        if let RunEvent::Phase { phase } = event {
            *self.working.lock().unwrap() =
                matches!(phase, Phase::Copying | Phase::Restoring | Phase::Merging);
        }
    }

    fn stop_requested(&self) -> bool {
        *self.working.lock().unwrap() && self.started.fetch_add(1, Ordering::Relaxed) >= self.after
    }
}

/// Files and folders below a folder, relative with `/`: the oracle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Tree {
    files: BTreeMap<String, Vec<u8>>,
    dirs: Vec<String>,
}

fn tree(root: &Path) -> Tree {
    let mut t = Tree::default();
    if !root.exists() {
        return t;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if p.is_dir() {
                t.dirs.push(rel);
                stack.push(p);
            } else {
                t.files.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    t.dirs.sort();
    t
}

/// A profile, its source and a place for restores, in a temporary folder.
struct Bed {
    dir: tempfile::TempDir,
    source: PathBuf,
    profile: Profile,
    password: Option<Password>,
    tuning: Tuning,
    /// Source as each backup saw it, by the time in the backup's name.
    snapshots: BTreeMap<String, Tree>,
    restores: u32,
}

fn time_of(name: &str) -> String {
    name.rsplit_once('_').map_or(name, |(t, _)| t).to_owned()
}

impl Bed {
    fn new(container: Container, encrypted: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("sörce dir");
        fs::create_dir_all(&source).unwrap();
        let mut profile = Profile::new("Deep");
        profile.sources.push(source.clone());
        profile.destination = Destination::Local { path: dir.path().join("backups") };
        profile.container = container;
        profile.encrypted = encrypted;
        profile.large_files.threshold_mb = 1;
        profile.merge.rules = Retention::KEEP_ALL;
        profile.merge.keep_deleted = false;
        let password = encrypted.then(|| Password::new("deep test password".into()).unwrap());
        let tuning = Tuning { chunks: ChunkParams::new(4096, 16384, 65536), ..Tuning::default() };
        Self { dir, source, profile, password, tuning, snapshots: BTreeMap::new(), restores: 0 }
    }

    fn pw(&self) -> Option<&Password> {
        self.password.as_ref()
    }

    /// Names of the backups, oldest first.
    fn names(&self) -> Vec<String> {
        let mut n: Vec<String> =
            list_archives(&self.profile, self.pw()).unwrap().into_iter().map(|a| a.name).collect();
        n.sort();
        n
    }

    /// A backup (names have one-second resolution), with an interruption
    /// first when `stop_after` says so; then the source is remembered.
    fn backup(&mut self, mode: BackupMode, stop_after: Option<u64>) {
        sleep(Duration::from_millis(1100));
        let first =
            stop_after.map(|n| backup_tuned(&self.profile, self.pw(), mode, &StopAfter::new(n), self.tuning));
        let report = match first {
            // Fewer files than the stop: it was not interrupted.
            Some(Ok(report)) => report,
            Some(Err(EngineError::Cancelled)) | None => {
                backup_tuned(&self.profile, self.pw(), mode, &Quiet, self.tuning).unwrap()
            }
            Some(Err(e)) => panic!("interrupted backup: {e}"),
        };
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        let newest = self.names().pop().unwrap();
        self.snapshots.insert(time_of(&newest), tree(&self.source));
    }

    /// Restores `name` into a new folder and returns what came back,
    /// relative to the source.
    fn restored(&mut self, name: &str) -> Tree {
        self.restores += 1;
        let to = self.dir.path().join(format!("restore {}", self.restores));
        let req = RestoreRequest::single(name, Vec::new(), &RestoreTarget::Folder { path: to.clone() }, true);
        let r = restore(&self.profile, self.pw(), &req, &Quiet).unwrap();
        assert!(r.skipped.is_empty(), "{name}: {:?}", r.skipped);
        tree(&keys::path_under(&to, &keys::key_for(&self.source).unwrap()))
    }

    /// Every backup there restores to the source as it was at its time, and
    /// a full check finds nothing wrong.
    fn check_all(&mut self, seed: u64) {
        for name in self.names() {
            let expected = self.snapshots.get(&time_of(&name)).cloned().unwrap_or_else(|| {
                panic!("seed {seed}: backup {name} matches no snapshot ({:?})", self.snapshots.keys())
            });
            let got = self.restored(&name);
            assert_trees(&got, &expected, &format!("seed {seed}, backup {name}"));
        }
        let v = verify(&self.profile, self.pw(), None, &Quiet).unwrap();
        assert!(v.problems.is_empty(), "seed {seed}: {:?}", v.problems);
    }
}

fn assert_trees(got: &Tree, expected: &Tree, what: &str) {
    let names = |t: &Tree| t.files.keys().cloned().collect::<Vec<_>>();
    assert_eq!(names(got), names(expected), "{what}: files");
    for (k, v) in &expected.files {
        assert!(got.files.get(k) == Some(v), "{what}: content of {k}");
    }
    assert_eq!(got.dirs, expected.dirs, "{what}: folders");
}

/// Names a source may hold: accents, spaces, dots, a long one.
const NAMES: [&str; 10] = [
    "a.txt",
    "Ünïcode è.txt",
    "with space.doc",
    ".hidden",
    "b",
    "c.tar.gz",
    "x.jpg",
    "nota",
    "long_name_",
    "z.bin",
];
const FOLDERS: [&str; 4] = ["sub", "Sub Two", "deep/er", "ø"];

/// Changes the source: new, changed (in place, grown, shrunk, with bytes
/// inserted into a large file), deleted files and folders.
fn mutate(rng: &mut Rng, source: &Path) {
    for _ in 0..1 + rng.below(6) {
        let files: Vec<PathBuf> = tree(source).files.keys().map(|k| source.join(k)).collect();
        match rng.below(9) {
            0..=2 => {
                let folder = if rng.chance(50) {
                    source.to_path_buf()
                } else {
                    source.join(rng.pick(&FOLDERS).unwrap())
                };
                let mut name = (*rng.pick(&NAMES).unwrap()).to_owned();
                if name == "long_name_" {
                    name.push_str(&"n".repeat(120));
                }
                fs::create_dir_all(&folder).unwrap();
                let size = match rng.below(10) {
                    0 => 0,
                    1 | 2 => 1_100_000 + rng.below(900_000),
                    _ => 1 + rng.below(5000),
                };
                let data = rng.bytes(size);
                fs::write(folder.join(name), data).unwrap();
            }
            3 | 4 => {
                if let Some(f) = rng.pick(&files) {
                    let mut data = fs::read(f).unwrap();
                    match rng.below(4) {
                        0 if !data.is_empty() => {
                            // Bytes inserted in the middle: chunks around them stay.
                            let at = rng.below(data.len());
                            let n = 1 + rng.below(300);
                            let extra = rng.bytes(n);
                            data.splice(at..at, extra);
                        }
                        1 if !data.is_empty() => {
                            let at = rng.below(data.len());
                            let n = (1 + rng.below(1000)).min(data.len() - at);
                            let fresh = rng.bytes(n);
                            data.splice(at..at + n, fresh);
                        }
                        2 => data.truncate(rng.below(data.len() + 1)),
                        _ => {
                            let n = 1 + rng.below(2000);
                            data.extend(rng.bytes(n));
                        }
                    }
                    fs::write(f, data).unwrap();
                }
            }
            5 | 6 => {
                if let Some(f) = rng.pick(&files) {
                    fs::remove_file(f).unwrap();
                }
            }
            7 => {
                let d = source.join(rng.pick(&FOLDERS).unwrap());
                if d.exists() && rng.chance(50) {
                    fs::remove_dir_all(d).unwrap();
                } else {
                    fs::create_dir_all(d.join("empty")).unwrap();
                }
            }
            _ => {
                // A copy: same content under another name.
                if let Some(f) = rng.pick(&files) {
                    let copy =
                        f.with_file_name(format!("copy of {}", f.file_name().unwrap().to_string_lossy()));
                    if copy.to_string_lossy().len() < 240 {
                        fs::copy(f, copy).unwrap();
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------- random histories

/// Random backups (some interrupted and resumed, some complete), merges by
/// hand and deletions; every backup left must restore exactly to the
/// source of its time.
fn random_history(container: Container, encrypted: bool, salt: u64) {
    for seed in seeds(salt) {
        eprintln!("random history {container:?} encrypted={encrypted}: seed {seed}");
        let mut rng = Rng::new(seed);
        let mut bed = Bed::new(container, encrypted);
        mutate(&mut rng, &bed.source);
        bed.backup(BackupMode::Auto, None);
        for _ in 0..6 {
            mutate(&mut rng, &bed.source);
            let mode = if rng.chance(15) { BackupMode::Complete } else { BackupMode::Auto };
            let stop = rng.chance(30).then(|| rng.below(4) as u64);
            bed.backup(mode, stop);
            eprintln!("  backup {mode:?} stop {stop:?} -> {:?}", bed.names());
            let names = bed.names();
            if names.len() >= 3 && rng.chance(30) {
                let i = 1 + rng.below(names.len() - 2);
                eprintln!("  merge {} .. {}", names[i], names[i + 1]);
                match merge_range(&bed.profile, bed.pw(), &names[i], &names[i + 1], &Quiet) {
                    Ok(_) | Err(EngineError::InvalidProfile(_)) => {}
                    Err(e) => panic!("seed {seed}: merge: {e}"),
                }
            } else if names.len() >= 3 && rng.chance(20) {
                let victim = rng.pick(&names).unwrap().clone();
                eprintln!("  delete {victim}");
                match delete_archive(&bed.profile, bed.pw(), &victim, &Quiet) {
                    Ok(_) => {
                        // The newest gone: the next backup compares with the one before.
                        if bed.names().iter().all(|n| time_of(n) != time_of(&victim)) {
                            bed.snapshots.remove(&time_of(&victim));
                        }
                    }
                    Err(EngineError::InvalidProfile(_)) => {}
                    Err(e) => panic!("seed {seed}: delete: {e}"),
                }
            }
            bed.check_all(seed);
        }
    }
}

#[test]
fn random_histories_of_plain_copies_restore_exactly() {
    random_history(Container::Folder, false, 1);
}

#[test]
fn random_histories_of_zips_restore_exactly() {
    random_history(Container::ZipPerFile, false, 2);
}

#[test]
fn random_histories_of_encrypted_backups_restore_exactly() {
    random_history(Container::ZipPerFile, true, 3);
}

#[test]
fn automatic_merges_keep_every_backup_left_restorable() {
    for seed in seeds(4) {
        eprintln!("automatic merges: seed {seed}");
        let mut rng = Rng::new(seed);
        let mut bed = Bed::new(Container::ZipPerFile, false);
        bed.profile.merge.rules = Retention { keep_last: 2, ..Retention::KEEP_ALL };
        for _ in 0..6 {
            mutate(&mut rng, &bed.source);
            bed.backup(BackupMode::Auto, None);
            eprintln!("  backup -> {:?}", bed.names());
            merge(&bed.profile, bed.pw(), &Quiet).unwrap();
            eprintln!("  merge -> {:?}", bed.names());
            bed.check_all(seed);
        }
        assert!(bed.names().len() <= 4, "merged: {:?}", bed.names());
    }
}

// ------------------------------------------------------- interruptions

/// A source of several files, one of them large (chunked).
fn varied_source(bed: &Bed, rng: &mut Rng) {
    for (i, name) in NAMES.iter().enumerate() {
        let size = if i == 2 { 1_500_000 } else { 10 + 997 * i };
        let folder = if i % 3 == 0 { bed.source.join("sub") } else { bed.source.clone() };
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(name), rng.bytes(size)).unwrap();
    }
}

#[test]
fn a_backup_stopped_at_any_file_then_resumed_restores_exactly() {
    for (container, encrypted) in
        [(Container::Folder, false), (Container::ZipPerFile, false), (Container::ZipPerFile, true)]
    {
        let mut rng = Rng::new(11);
        for stop in 0..NAMES.len() as u64 {
            let mut bed = Bed::new(container, encrypted);
            varied_source(&bed, &mut rng);
            bed.backup(BackupMode::Auto, Some(stop));
            assert_eq!(bed.names().len(), 1, "{container:?}: one backup, resumed");
            bed.check_all(stop);
        }
    }
}

#[test]
fn a_restore_stopped_at_any_file_leaves_no_wrong_file_and_can_be_done_again() {
    let mut rng = Rng::new(12);
    let mut bed = Bed::new(Container::ZipPerFile, true);
    varied_source(&bed, &mut rng);
    bed.backup(BackupMode::Auto, None);
    let name = bed.names().remove(0);
    let expected = tree(&bed.source);
    let mut stopped = 0;
    for stop in 0..NAMES.len() as u64 {
        let to = bed.dir.path().join(format!("stopped {stop}"));
        let req =
            RestoreRequest::single(&name, Vec::new(), &RestoreTarget::Folder { path: to.clone() }, true);
        let r = restore(&bed.profile, bed.pw(), &req, &StopAfter::new(stop));
        match r {
            Err(EngineError::Cancelled) => stopped += 1,
            Ok(_) => {}
            Err(e) => panic!("stopped at {stop}: {e}"),
        }
        let root = keys::path_under(&to, &keys::key_for(&bed.source).unwrap());
        let partial = tree(&root);
        for (k, v) in &partial.files {
            assert_eq!(Some(v), expected.files.get(k), "stopped at {stop}: {k} is whole or absent");
        }
        restore(&bed.profile, bed.pw(), &req, &Quiet).unwrap();
        assert_trees(&tree(&root), &expected, &format!("restored again after a stop at {stop}"));
    }
    assert!(stopped >= NAMES.len() / 2, "most restores were stopped ({stopped})");
}

#[test]
fn a_merge_stopped_at_any_file_leaves_every_backup_restorable() {
    let mut rng = Rng::new(13);
    for stop in 0..6 {
        let mut bed = Bed::new(Container::ZipPerFile, false);
        varied_source(&bed, &mut rng);
        bed.backup(BackupMode::Auto, None);
        for _ in 0..3 {
            mutate(&mut rng, &bed.source);
            bed.backup(BackupMode::Auto, None);
        }
        let names = bed.names();
        let r = merge_range(&bed.profile, bed.pw(), &names[1], &names[3], &StopAfter::new(stop));
        assert!(matches!(r, Err(EngineError::Cancelled) | Ok(_)), "{r:?}");
        bed.check_all(stop);
    }
}

// ------------------------------------------------------- merge crash states

/// Every state an interrupted merge of the three newest backups can leave
/// (members moved aside or not, the merged backup in place or lost, the
/// journal whole, half written or lost): the next run recovers, nothing is
/// lost unless the merge was complete, and the newest state restores.
#[test]
fn every_crash_state_of_a_merge_recovers_without_losing_data() {
    let mut rng = Rng::new(14);
    let mut base = Bed::new(Container::Folder, false);
    varied_source(&base, &mut rng);
    base.backup(BackupMode::Auto, None);
    for _ in 0..3 {
        mutate(&mut rng, &base.source);
        base.backup(BackupMode::Auto, None);
    }
    let names = base.names();
    let members = &names[1..];
    let newest = names.last().unwrap().clone();
    let dest = base.profile.destination.path().to_path_buf();
    // The real merged backup, made on a copy.
    let scratch = tempfile::tempdir().unwrap();
    let merged_dest = scratch.path().join("backups");
    copy_dir(&dest, &merged_dest);
    let mut scratch_profile = base.profile.clone();
    scratch_profile.destination = Destination::Local { path: merged_dest.clone() };
    merge_range(&scratch_profile, None, &members[0], &newest, &Quiet).unwrap();
    let target = list_archives(&scratch_profile, None)
        .unwrap()
        .into_iter()
        .map(|a| a.name)
        .find(|n| time_of(n) == time_of(&newest))
        .unwrap();

    let mut states = 0;
    for moved in 0u32..(1 << members.len()) {
        for merged_in_place in [false, true] {
            for journal in ["none", "whole", "torn"] {
                // Moving aside goes oldest first; the merged backup comes
                // after all of them.
                let aside: Vec<&String> = members
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| moved & (1 << i) != 0)
                    .map(|(_, m)| m)
                    .collect();
                if merged_in_place && aside.len() != members.len() {
                    continue;
                }
                states += 1;
                let copy = tempfile::tempdir().unwrap();
                let d = copy.path().join("backups");
                copy_dir(&dest, &d);
                for m in &aside {
                    fs::rename(d.join(m), d.join(format!("{m}.old"))).unwrap();
                }
                if merged_in_place {
                    copy_dir(&merged_dest.join(&target), &d.join(&target));
                }
                let text =
                    serde_json::json!({ "format": 1, "target": target, "members": members }).to_string();
                match journal {
                    "whole" => fs::write(d.join("merge.journal"), &text).unwrap(),
                    "torn" => fs::write(d.join("merge.journal"), &text[..text.len() / 2]).unwrap(),
                    _ => {}
                }
                let what = format!("moved {moved:03b}, merged {merged_in_place}, journal {journal}");
                let mut bed = Bed::new(Container::Folder, false);
                bed.profile = base.profile.clone();
                bed.profile.sources = base.profile.sources.clone();
                bed.profile.destination = Destination::Local { path: d.clone() };
                bed.source = base.source.clone();
                bed.snapshots = base.snapshots.clone();
                let v = verify(&bed.profile, None, None, &Quiet).unwrap_or_else(|e| panic!("{what}: {e}"));
                assert!(v.problems.is_empty(), "{what}: {:?}", v.problems);
                let after = bed.names();
                assert!(
                    after.iter().any(|n| time_of(n) == time_of(&newest)),
                    "{what}: the newest state: {after:?}"
                );
                if !merged_in_place {
                    assert_eq!(after, names, "{what}: nothing merged, nothing lost");
                }
                assert!(!d.join("merge.journal").exists(), "{what}: the journal is done with");
                for n in &after {
                    let got = bed.restored(n);
                    assert_trees(&got, &bed.snapshots[&time_of(n)], &format!("{what}: {n}"));
                }
            }
        }
    }
    assert_eq!(states, 8 * 3 + 3);
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let target = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &target);
        } else {
            fs::copy(&p, &target).unwrap();
        }
    }
}

// ------------------------------------------------------- doctored manifests

/// Random damage to a manifest: a restore either refuses the backup or
/// writes only below the folder chosen, and never panics.
#[test]
fn doctored_manifests_never_lead_a_restore_outside_its_folder() {
    let mut rng = Rng::new(15);
    let mut bed = Bed::new(Container::Folder, false);
    varied_source(&bed, &mut rng);
    bed.backup(BackupMode::Auto, None);
    let name = bed.names().remove(0);
    let manifest = bed.profile.destination.path().join(&name).join("manifest.jsonl");
    let original = fs::read_to_string(&manifest).unwrap();
    let outside_before = outside_listing(bed.dir.path());
    // Escapes that would land in the temporary folder, where the test sees
    // them (an absolute segment, a climb with `..`), and other malformed
    // segments. Nothing here points at the rest of the disk, even if the
    // check that refuses them were gone.
    let landing = bed.dir.path().join("escaped.txt").to_string_lossy().into_owned();
    let nasty = ["..", ".", "", r"a\b", "evil\u{0}", landing.as_str()];
    let mut refused = 0;
    for round in 0..150 {
        let mut lines: Vec<String> = original.lines().map(str::to_owned).collect();
        for _ in 0..1 + rng.below(3) {
            let i = 1 + rng.below(lines.len() - 1);
            match rng.below(6) {
                // A climb from `restores/<round>/C` up to the temporary
                // folder.
                5 => {
                    let mut e: serde_json::Value = serde_json::from_str(&lines[i]).unwrap_or_default();
                    if e.get("key").is_some() {
                        e["key"] = "C/../../../escaped.txt".into();
                        lines[i] = e.to_string();
                    }
                }
                // A key with a nasty segment.
                0 | 1 => {
                    let mut e: serde_json::Value = serde_json::from_str(&lines[i]).unwrap_or_default();
                    if let Some(key) = e.get("key").and_then(|k| k.as_str()).map(str::to_owned) {
                        let mut segs: Vec<String> = key.split('/').map(str::to_owned).collect();
                        let at = rng.below(segs.len());
                        segs[at] = (*rng.pick(&nasty).unwrap()).to_owned();
                        e["key"] = segs.join("/").into();
                        lines[i] = e.to_string();
                    }
                }
                2 => {
                    let dup = lines[i].clone();
                    lines.insert(i, dup);
                }
                3 => {
                    lines.remove(i);
                }
                _ => {
                    let mut bytes = lines[i].clone().into_bytes();
                    let at = rng.below(bytes.len());
                    bytes[at] = u8::try_from(rng.below(128)).unwrap();
                    lines[i] = String::from_utf8_lossy(&bytes).into_owned();
                }
            }
        }
        fs::write(&manifest, lines.join("\n") + "\n").unwrap();
        let to = bed.dir.path().join("restores").join(round.to_string());
        let req = RestoreRequest::single(&name, Vec::new(), &RestoreTarget::Folder { path: to }, true);
        if restore(&bed.profile, None, &req, &Quiet).is_err() {
            refused += 1;
        }
        assert_eq!(
            outside_listing(bed.dir.path()),
            outside_before,
            "round {round}: something written outside"
        );
    }
    fs::write(&manifest, &original).unwrap();
    assert!(refused > 30, "most doctored manifests are refused ({refused})");
}

/// Everything in `root` except the restores folder.
fn outside_listing(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|n| n == "restores") && d == root {
                continue;
            }
            out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned());
            if p.is_dir() {
                stack.push(p);
            }
        }
    }
    out.sort();
    out
}

// ------------------------------------------------------- damaged backups

/// Bytes flipped in stored files: the full check reports it, and a restore
/// never brings back a wrong file (each file is right, or missing and
/// reported).
fn damage_is_found(container: Container, encrypted: bool, seed: u64) {
    let mut rng = Rng::new(seed);
    let mut bed = Bed::new(container, encrypted);
    varied_source(&bed, &mut rng);
    bed.backup(BackupMode::Auto, None);
    mutate(&mut rng, &bed.source);
    bed.backup(BackupMode::Auto, None);
    let dest = bed.profile.destination.path().to_path_buf();
    let stored: Vec<PathBuf> = all_files(&dest)
        .into_iter()
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            !n.starts_with("manifest") && !n.starts_with("archeion") && !n.ends_with(".lock")
        })
        .collect();
    assert!(!stored.is_empty());
    for _ in 0..3 {
        let victim = rng.pick(&stored).unwrap().clone();
        let mut bytes = fs::read(&victim).unwrap();
        if bytes.is_empty() {
            continue;
        }
        let at = rng.below(bytes.len());
        bytes[at] ^= 0x5A;
        fs::write(&victim, bytes).unwrap();
    }
    let v = verify(&bed.profile, bed.pw(), None, &Quiet).unwrap();
    assert!(!v.problems.is_empty(), "{container:?} encrypted={encrypted}: damage not found");

    for name in bed.names() {
        let expected = bed.snapshots[&time_of(&name)].clone();
        bed.restores += 1;
        let to = bed.dir.path().join(format!("damaged {}", bed.restores));
        let req =
            RestoreRequest::single(&name, Vec::new(), &RestoreTarget::Folder { path: to.clone() }, true);
        match restore(&bed.profile, bed.pw(), &req, &Quiet) {
            Ok(r) => {
                let got = tree(&keys::path_under(&to, &keys::key_for(&bed.source).unwrap()));
                for (k, v) in &got.files {
                    assert_eq!(Some(v), expected.files.get(k), "{name}: {k} restored wrong");
                }
                let missing = expected.files.keys().filter(|k| !got.files.contains_key(*k)).count();
                assert!(
                    missing <= r.skipped.len(),
                    "{name}: {missing} missing, {} reported",
                    r.skipped.len()
                );
            }
            Err(EngineError::Damaged { .. }) => {}
            Err(e) => panic!("{name}: {e}"),
        }
    }
}

fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out.sort();
    out
}

#[test]
fn damage_to_plain_copies_is_found_and_never_restored() {
    for seed in seeds(21) {
        damage_is_found(Container::Folder, false, seed);
    }
}

#[test]
fn damage_to_zips_is_found_and_never_restored() {
    for seed in seeds(22) {
        damage_is_found(Container::ZipPerFile, false, seed);
    }
}

#[test]
fn damage_to_encrypted_backups_is_found_and_never_restored() {
    for seed in seeds(23) {
        damage_is_found(Container::ZipPerFile, true, seed);
    }
}
