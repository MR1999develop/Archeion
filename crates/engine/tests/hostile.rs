//! A destination written by someone else (a share, a drive lent or found):
//! whatever they put there, Archeion touches nothing outside the
//! destination, keeps what is not its own, and tells when backups were
//! removed or put back.

use archeion_core::{ArchiveName, BackupMode, Container, Destination, Profile, RunEvent};
use archeion_engine::{EventSink, Password, backup, backup_stopping_after, merge_range, verify};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

/// A profile backing up `<dir>/src` to `<dir>/backups`, and a folder of the
/// user outside both, `<dir>/victim`, holding `precious.txt`.
struct Bed {
    dir: tempfile::TempDir,
    profile: Profile,
    password: Option<Password>,
}

impl Bed {
    fn new(container: Container, encrypted: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("src").join("docs");
        fs::create_dir_all(&source).unwrap();
        for name in ["a.txt", "b.txt", "c.txt"] {
            fs::write(source.join(name), format!("content of {name}")).unwrap();
        }
        let victim = dir.path().join("victim");
        fs::create_dir_all(&victim).unwrap();
        fs::write(victim.join("precious.txt"), b"the only copy").unwrap();
        let mut profile = Profile::new("Hostile");
        profile.sources.push(dir.path().join("src"));
        profile.destination = Destination::Local { path: dir.path().join("backups") };
        profile.container = container;
        profile.encrypted = encrypted;
        let password = encrypted.then(|| Password::new("a long enough test password".into()).unwrap());
        Self { dir, profile, password }
    }

    fn pw(&self) -> Option<&Password> {
        self.password.as_ref()
    }

    fn dest(&self) -> PathBuf {
        self.profile.destination.path().to_path_buf()
    }

    fn victim(&self) -> PathBuf {
        self.dir.path().join("victim")
    }

    fn source(&self) -> PathBuf {
        self.dir.path().join("src").join("docs")
    }

    fn backup(&self) -> archeion_core::BackupReport {
        backup(&self.profile, self.pw(), BackupMode::Auto, &Quiet).unwrap()
    }

    /// Committed backups, oldest first.
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dest())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.parse::<ArchiveName>().is_ok())
            .collect();
        names.sort();
        names
    }

    fn victim_intact(&self) -> bool {
        fs::read(self.victim().join("precious.txt")).is_ok_and(|b| b == b"the only copy")
    }
}

fn json(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

/// Waits for the clock to pass a second: backup names have seconds.
fn next_second() {
    std::thread::sleep(std::time::Duration::from_millis(1100));
}

// ------------------------------------------------------------ merge journal

/// A merge journal naming folders of the computer (absolute, or reached
/// with `..`) as members or target: the next run, scheduled or not,
/// renames and deletes nothing outside the destination, and leaves the
/// user's own `.old` folder kept in the destination alone.
#[test]
fn a_planted_merge_journal_touches_nothing_outside_the_destination() {
    for encrypted in [false, true] {
        let container = if encrypted { Container::ZipPerFile } else { Container::Folder };
        let bed = Bed::new(container, encrypted);
        bed.backup();
        let archive = bed.names().pop().unwrap();
        let victim = bed.victim();
        let own = bed.dest().join("Holidays.old");
        fs::create_dir_all(&own).unwrap();
        fs::write(own.join("photo.jpg"), b"photo").unwrap();
        // An encrypted destination opens only sealed manifests; a plain one
        // made elsewhere is put in a folder of its own as the target.
        let plain_target = "2099-01-01_000000_C";
        fs::create_dir_all(bed.dest().join(plain_target)).unwrap();
        fs::write(
            bed.dest().join(plain_target).join("manifest.jsonl"),
            "{\"format\":2,\"kind\":\"complete\",\"entries\":0}\n",
        )
        .unwrap();
        let journals = [
            format!(
                r#"{{"format":1,"target":"{archive}","members":[{}]}}"#,
                json(&victim.display().to_string())
            ),
            format!(
                r#"{{"format":1,"target":"{plain_target}","members":[{}]}}"#,
                json(&victim.display().to_string())
            ),
            format!(r#"{{"format":1,"target":"{archive}","members":[{}]}}"#, json(r"..\victim")),
            format!(
                r#"{{"format":1,"target":{},"members":["{archive}"]}}"#,
                json(&victim.display().to_string())
            ),
        ];
        for journal in &journals {
            fs::write(bed.dest().join("merge.journal"), journal).unwrap();
            let _ = verify(&bed.profile, bed.pw(), None, &Quiet);
            assert!(bed.victim_intact(), "encrypted={encrypted}, {journal}: the user's folder was touched");
            assert!(!bed.dir.path().join("victim.old").exists(), "encrypted={encrypted}, {journal}");
            assert!(
                own.join("photo.jpg").exists(),
                "encrypted={encrypted}, {journal}: the user's .old folder"
            );
            assert!(
                !bed.dest().join("merge.journal").exists(),
                "encrypted={encrypted}: the journal is done with"
            );
            assert!(bed.names().contains(&archive), "encrypted={encrypted}, {journal}: the real backup");
        }
    }
}

// ----------------------------------------------------- list of backups

/// Backups removed from an encrypted destination are noticed even when a
/// leftover folder makes the run believe an interrupted deletion was
/// completed: by the check, and by the next backup, once.
#[test]
fn backups_removed_from_outside_are_noticed_by_the_next_run() {
    let bed = Bed::new(Container::ZipPerFile, true);
    for i in 0..3 {
        fs::write(bed.source().join(format!("new{i}.txt")), b"x").unwrap();
        assert!(!bed.backup().changed_outside, "backup {i}");
        next_second();
    }
    let names = bed.names();
    // The middle one: the newest is still there, so this is not a destination
    // put back from an old copy.
    fs::remove_dir_all(bed.dest().join(&names[1])).unwrap();
    fs::create_dir(bed.dest().join(format!("{}.deleting", names[1]))).unwrap();
    let v = verify(&bed.profile, bed.pw(), None, &Quiet).unwrap();
    assert!(v.problems.iter().any(|p| p.contains("signature")), "{:?}", v.problems);
    let report = bed.backup();
    assert!(report.changed_outside, "the backup tells");
    next_second();
    assert!(!bed.backup().changed_outside, "told once: the backup signed the list it found");
    // The files stored only in the removed backup are missing: the check
    // says so, but no longer that the list was changed.
    let v = verify(&bed.profile, bed.pw(), None, &Quiet).unwrap();
    assert!(!v.problems.iter().any(|p| p.contains("signature")), "{:?}", v.problems);
}

/// An interrupted deletion, completed by the next run, is not a change
/// from outside.
#[test]
fn an_interrupted_deletion_is_not_reported_as_a_change_from_outside() {
    let bed = Bed::new(Container::ZipPerFile, true);
    bed.backup();
    next_second();
    fs::write(bed.source().join("new.txt"), b"x").unwrap();
    bed.backup();
    let names = bed.names();
    // What Repo::remove_archive does before the rename, then the rename.
    archeion_engine::delete_archive(&bed.profile, bed.pw(), &names[1], &Quiet).unwrap();
    next_second();
    fs::write(bed.source().join("new2.txt"), b"x").unwrap();
    assert!(!bed.backup().changed_outside);
    assert!(verify(&bed.profile, bed.pw(), None, &Quiet).unwrap().problems.is_empty());
}

// ------------------------------------------------------- links (Windows)

/// A junction `link` to `target` (no administrator rights needed);
/// false when it cannot be made.
fn junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The folder below `root` that holds a file called `name`.
fn folder_holding(root: &Path, name: &str) -> PathBuf {
    walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .find(|e| e.file_name() == name)
        .and_then(|e| e.path().parent().map(Path::to_path_buf))
        .unwrap_or_else(|| panic!("no {name} below {}", root.display()))
}

/// An unfinished backup whose folder was replaced by a junction to a
/// folder of the computer is not resumed: nothing is written there, and
/// removing it removes the junction, not what it leads to.
#[cfg(windows)]
#[test]
fn an_unfinished_backup_holding_a_junction_is_not_resumed() {
    let bed = Bed::new(Container::Folder, false);
    let stopped = backup_stopping_after(&bed.profile, None, BackupMode::Auto, &Quiet, 1);
    assert!(stopped.is_err(), "the first backup stops after one file");
    let partial = fs::read_dir(bed.dest())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.to_string_lossy().ends_with(".partial"))
        .expect("an unfinished backup");
    let copied = folder_holding(&partial, "a.txt");
    fs::remove_dir_all(&copied).unwrap();
    fs::write(bed.victim().join("b.txt"), b"the user's b").unwrap();
    assert!(junction(&copied, &bed.victim()), "making a junction");
    let report = bed.backup();
    assert_eq!(report.resumed_files, 0, "not resumed");
    assert!(bed.victim_intact());
    assert_eq!(fs::read(bed.victim().join("b.txt")).unwrap(), b"the user's b", "nothing written through it");
    assert!(!bed.victim().join("c.txt").exists(), "nothing written through it");
}

/// A folder of a plain backup replaced by a junction to a folder of the
/// computer: a merge does not copy what it leads to into the destination.
#[cfg(windows)]
#[test]
fn a_merge_never_copies_through_a_junction() {
    for container in [Container::Folder, Container::ZipPerFile] {
        let bed = Bed::new(container, false);
        bed.backup();
        next_second();
        fs::write(bed.source().join("new.txt"), b"x").unwrap();
        bed.backup();
        let names = bed.names();
        let suffix = if container == Container::Folder { "" } else { ".zip" };
        let held = folder_holding(&bed.dest().join(&names[0]), &format!("a.txt{suffix}"));
        // The user's folder holds files named as those in the backup.
        let secret = bed.dir.path().join("secret");
        fs::create_dir_all(&secret).unwrap();
        for entry in fs::read_dir(&held).unwrap().flatten() {
            fs::copy(entry.path(), secret.join(entry.file_name())).unwrap();
        }
        fs::write(secret.join(format!("a.txt{suffix}")), b"SECRET of the user").unwrap();
        fs::remove_dir_all(&held).unwrap();
        assert!(junction(&held, &secret), "making a junction");
        let _ = merge_range(&bed.profile, None, &names[0], &names[1], &Quiet);
        let leaked = walkdir::WalkDir::new(bed.dest())
            .follow_links(false)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .any(|e| fs::read(e.path()).is_ok_and(|b| b.windows(6).any(|w| w == b"SECRET")));
        assert!(!leaked, "{container:?}: the user's file was copied into the destination");
        assert!(fs::read(secret.join(format!("a.txt{suffix}"))).is_ok_and(|b| b == b"SECRET of the user"));
    }
}

// ------------------------------------------------------------- small files

/// The small files of a destination (its description, the key, the
/// signature of the list, the merge journal) padded far beyond their size
/// are refused instead of read whole into memory.
#[test]
fn a_huge_small_file_is_refused() {
    let bed = Bed::new(Container::ZipPerFile, true);
    bed.backup();
    for name in ["archeion.json", "archeion.key.json"] {
        let path = bed.dest().join(name);
        let text = fs::read_to_string(&path).unwrap();
        // Still valid JSON: only its size is wrong.
        fs::write(&path, format!("{text}{}", " ".repeat(2 * 1024 * 1024))).unwrap();
        let err = verify(&bed.profile, bed.pw(), None, &Quiet).unwrap_err();
        assert!(err.to_string().contains("too large"), "{name}: {err}");
        fs::write(&path, text).unwrap();
    }
    assert!(verify(&bed.profile, bed.pw(), None, &Quiet).is_ok());
}
