//! The copy of its profile that each backup keeps at the destination: the
//! rules come back from the backups alone, on any computer, sealed when the
//! destination is encrypted, and a copy put there by someone else is not
//! taken for it.

use archeion_core::{BackupMode, Container, Destination, Exclusion, Profile, RunEvent};
use archeion_engine::{EngineError, EventSink, Password, adopt_destination, backup, read_profile_copy};
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

const PLAIN: &str = "archeion.profile.toml";
const SEALED: &str = "archeion.profile.zip";

/// A profile backing up `<dir>/Private folder` to `<dir>/backups`.
struct Bed {
    dir: tempfile::TempDir,
    profile: Profile,
    password: Option<Password>,
}

impl Bed {
    fn new(encrypted: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Private folder");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("a.txt"), b"alpha").unwrap();
        let mut profile = Profile::new("Tax papers");
        profile.sources.push(source);
        profile.excludes.push(Exclusion::Extension { ext: "tmp".into() });
        profile.destination = Destination::Local { path: dir.path().join("backups") };
        profile.container = Container::ZipPerFile;
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

    fn backup(&self) {
        backup(&self.profile, self.pw(), BackupMode::Auto, &Quiet).unwrap();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}

#[test]
fn each_backup_keeps_the_profile_at_its_destination() {
    let mut bed = Bed::new(false);
    bed.backup();
    assert!(bed.dest().join(PLAIN).is_file());
    assert!(!bed.dest().join(format!("{PLAIN}.tmp")).exists(), "no temporary file left");
    assert_eq!(read_profile_copy(&bed.dest(), None).unwrap(), bed.profile);

    // A rule changed since: the next backup keeps the new one.
    bed.profile.excludes.push(Exclusion::Extension { ext: "bak".into() });
    bed.backup();
    assert_eq!(read_profile_copy(&bed.dest(), None).unwrap().excludes, bed.profile.excludes);
}

#[test]
fn an_encrypted_destination_keeps_it_sealed() {
    let bed = Bed::new(true);
    bed.backup();
    assert!(!bed.dest().join(PLAIN).exists(), "never in the clear");
    let sealed = fs::read(bed.dest().join(SEALED)).unwrap();
    for word in ["Private folder", "Tax papers", "tmp"] {
        assert!(!sealed.windows(word.len()).any(|w| w == word.as_bytes()), "{word} shows");
    }

    assert!(matches!(read_profile_copy(&bed.dest(), None), Err(EngineError::PasswordNeeded)));
    let wrong = Password::new("not the password at all".into()).unwrap();
    assert!(matches!(read_profile_copy(&bed.dest(), Some(&wrong)), Err(EngineError::WrongPassword)));
    assert_eq!(read_profile_copy(&bed.dest(), bed.pw()).unwrap(), bed.profile);
}

/// The drive has another letter on the new computer, or the folder was
/// copied elsewhere: the profile points at the folder it was read from.
#[test]
fn the_copy_names_the_folder_it_is_read_from() {
    for encrypted in [false, true] {
        let bed = Bed::new(encrypted);
        bed.backup();
        let moved = bed.dir.path().join("other drive").join("Backups of the old PC");
        copy_dir(&bed.dest(), &moved);
        let read = read_profile_copy(&moved, bed.pw()).unwrap();
        assert_eq!(read.destination, Destination::Local { path: moved.clone() });
        assert_eq!(read.id, bed.profile.id, "the id that owns the backups");
        assert_eq!(read.sources, bed.profile.sources);
        assert_eq!(read.encrypted, encrypted);
    }
}

/// A profile that took the backups over owns them before its first backup
/// rewrites the copy: the copy read carries the owner's id.
#[test]
fn the_profile_read_is_the_owner_of_the_backups() {
    let bed = Bed::new(false);
    bed.backup();
    let mut heir = Profile::new("Re-created");
    heir.destination = bed.profile.destination.clone();
    adopt_destination(&heir).unwrap();
    let read = read_profile_copy(&bed.dest(), None).unwrap();
    assert_eq!(read.id, heir.id);
    assert_eq!(read.name, bed.profile.name, "the rules are still the copy's");
}

#[test]
fn folders_without_backups_or_without_a_copy_are_told_apart() {
    let bed = Bed::new(false);
    let empty = bed.dir.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    assert!(matches!(read_profile_copy(&empty, None), Err(EngineError::NotADestination)));
    let missing = bed.dir.path().join("missing");
    assert!(matches!(read_profile_copy(&missing, None), Err(EngineError::NotADestination)));

    bed.backup();
    // Backups made before the copy was kept.
    fs::remove_file(bed.dest().join(PLAIN)).unwrap();
    assert!(matches!(read_profile_copy(&bed.dest(), None), Err(EngineError::NoProfileCopy)));
}

/// Whoever can write to an encrypted destination cannot hand over rules
/// of their own: a plain copy is not read (and the next backup removes
/// it), and a sealed one from elsewhere does not open.
#[test]
fn a_copy_put_in_an_encrypted_destination_is_not_taken() {
    let bed = Bed::new(true);
    bed.backup();
    let mut planted = bed.profile.clone();
    planted.sources = vec![PathBuf::from(r"\\attacker\share")];
    fs::write(bed.dest().join(PLAIN), archeion_core::profile_text::encode(&planted).unwrap()).unwrap();
    assert_eq!(read_profile_copy(&bed.dest(), bed.pw()).unwrap().sources, bed.profile.sources);
    fs::remove_file(bed.dest().join(SEALED)).unwrap();
    assert!(matches!(read_profile_copy(&bed.dest(), bed.pw()), Err(EngineError::NoProfileCopy)));
    bed.backup();
    assert!(!bed.dest().join(PLAIN).exists(), "the planted copy is removed");
    assert!(bed.dest().join(SEALED).exists());

    // The sealed copy of another encrypted destination.
    let other = Bed::new(true);
    other.backup();
    fs::copy(other.dest().join(SEALED), bed.dest().join(SEALED)).unwrap();
    let err = read_profile_copy(&bed.dest(), bed.pw()).unwrap_err();
    assert!(matches!(err, EngineError::Damaged { .. }), "{err}");
}

#[test]
fn a_damaged_or_huge_copy_is_refused() {
    let bed = Bed::new(false);
    bed.backup();
    let path = bed.dest().join(PLAIN);
    fs::write(&path, "format = 1\nprofile = \"not a profile\"\n").unwrap();
    assert!(matches!(read_profile_copy(&bed.dest(), None), Err(EngineError::Damaged { .. })));
    let text = archeion_core::profile_text::encode(&bed.profile).unwrap();
    // Still a valid profile: only its size is wrong.
    fs::write(&path, format!("{text}{}", "#\n".repeat(1024 * 1024))).unwrap();
    let err = read_profile_copy(&bed.dest(), None).unwrap_err();
    assert!(err.to_string().contains("too large"), "{err}");
}
