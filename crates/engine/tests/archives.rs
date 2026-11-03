//! End-to-end tests of the archive engine in temporary folders.

use archeion_core::{
    ArchiveKind, BackupMode, Container, Destination, FileAction, FileDone, FullBackupRule, Profile,
    Retention, RunEvent, run::RECENT_FILES,
};
use archeion_engine::{
    EngineError, EventSink, ExistingFormat, Password, RestoreItem, RestoreRequest, RestoreTarget, backup,
    backup_stopping_after, check_password, delete_archive, file_versions, list_archives, merge, merge_range,
    restore, search, verify,
};
use std::{fs, path::PathBuf, sync::Mutex, thread::sleep, time::Duration};

#[derive(Default)]
struct Recorder(Mutex<Vec<RunEvent>>);

impl EventSink for Recorder {
    fn emit(&self, event: RunEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    restore_to: PathBuf,
    profile: Profile,
}

fn fixture(container: Container) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source folder");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::create_dir_all(source.join("empty")).unwrap();
    fs::write(source.join("a.txt"), "alpha").unwrap();
    fs::write(source.join("sub").join("ünicode è.txt"), "beta").unwrap();
    fs::write(source.join("skip.tmp"), "temp").unwrap();
    let mut profile = Profile::new("Test");
    profile.sources.push(source.clone());
    profile.excludes.push(archeion_core::Exclusion::extension("tmp").unwrap());
    profile.destination = Destination::Local { path: dir.path().join("backups") };
    profile.container = container;
    profile.merge.rules = Retention::KEEP_ALL;
    let restore_to = dir.path().join("restored");
    Fixture { _dir: dir, source, restore_to, profile }
}

fn run_backup(p: &Profile) -> archeion_core::BackupReport {
    // Archive names have one-second resolution.
    sleep(Duration::from_millis(1100));
    backup(p, None, BackupMode::Auto, &Recorder::default()).unwrap()
}

fn restore_all(f: &Fixture, archive: &str) {
    let req = RestoreRequest::single(
        archive,
        Vec::new(),
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
}

fn restored(f: &Fixture, relative: &str) -> PathBuf {
    let key = archeion_core::keys::key_for(&f.source.join(relative)).unwrap();
    archeion_core::keys::path_under(&f.restore_to, &key)
}

fn check_both_containers(test: impl Fn(Container)) {
    test(Container::ZipPerFile);
    test(Container::Folder);
}

#[test]
fn first_backup_is_complete_and_restorable() {
    check_both_containers(|c| {
        let f = fixture(c);
        let report = run_backup(&f.profile);
        assert_eq!(report.files_new, 2, "the .tmp file is excluded");

        let list = list_archives(&f.profile, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, ArchiveKind::Complete);
        assert_eq!(list[0].container, c);

        restore_all(&f, &list[0].name);
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
        assert_eq!(fs::read_to_string(restored(&f, "sub/ünicode è.txt")).unwrap(), "beta");
        assert!(restored(&f, "empty").is_dir());
        assert!(!restored(&f, "skip.tmp").exists());
    });
}

#[test]
fn incremental_stores_only_changes_and_any_date_restores() {
    check_both_containers(|c| {
        let f = fixture(c);
        run_backup(&f.profile);
        fs::write(f.source.join("a.txt"), "alpha v2").unwrap();
        fs::remove_file(f.source.join("sub").join("ünicode è.txt")).unwrap();
        fs::write(f.source.join("new.txt"), "gamma").unwrap();

        let report = run_backup(&f.profile);
        assert_eq!((report.files_new, report.files_changed, report.files_deleted), (1, 1, 1));
        assert_eq!(report.files_unchanged, 0);

        let list = list_archives(&f.profile, None).unwrap(); // newest first
        assert_eq!(list[0].kind, ArchiveKind::Incremental);
        assert_eq!(list[0].files_stored, 2);

        restore_all(&f, &list[1].name); // the first date
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
        assert!(restored(&f, "sub/ünicode è.txt").exists());

        fs::remove_dir_all(&f.restore_to).unwrap();
        restore_all(&f, &list[0].name); // the second date
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha v2");
        assert!(!restored(&f, "sub/ünicode è.txt").exists());
        assert_eq!(fs::read_to_string(restored(&f, "new.txt")).unwrap(), "gamma");
    });
}

#[test]
fn the_first_format_stays_even_if_the_profile_changes() {
    let mut f = fixture(Container::ZipPerFile);
    assert_eq!(archeion_engine::existing_format(&f.profile).unwrap(), None);
    run_backup(&f.profile);
    f.profile.container = Container::Folder;
    fs::write(f.source.join("a.txt"), "changed").unwrap();
    run_backup(&f.profile);
    assert!(list_archives(&f.profile, None).unwrap().iter().all(|a| a.container == Container::ZipPerFile));
    assert_eq!(
        archeion_engine::existing_format(&f.profile).unwrap(),
        Some(ExistingFormat { container: Container::ZipPerFile, encrypted: false })
    );
}

#[test]
fn excluded_folders_and_files_are_left_out() {
    let mut f = fixture(Container::Folder);
    fs::create_dir_all(f.source.join("cache")).unwrap();
    fs::write(f.source.join("cache").join("x.bin"), "x").unwrap();
    f.profile.excludes.push(archeion_core::Exclusion::Folder { path: f.source.join("CACHE") });
    f.profile.excludes.push(archeion_core::Exclusion::File { path: f.source.join("a.txt") });
    let report = run_backup(&f.profile);
    assert_eq!(report.files_new, 1, "only sub/ünicode è.txt");
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    restore_all(&f, &name);
    assert!(!restored(&f, "cache").exists());
    assert!(!restored(&f, "a.txt").exists());
}

#[test]
fn unchanged_files_are_not_copied_again() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let report = run_backup(&f.profile);
    assert_eq!(report.bytes_copied, 0);
    assert_eq!(report.files_unchanged, 2);
}

#[test]
fn automatic_merge_keeps_the_initial_complete_and_fuses_the_rest() {
    check_both_containers(|c| {
        let mut f = fixture(c);
        run_backup(&f.profile); // C
        fs::write(f.source.join("a.txt"), "v2").unwrap();
        run_backup(&f.profile); // I1
        fs::write(f.source.join("a.txt"), "v3").unwrap();
        fs::write(f.source.join("late.txt"), "late").unwrap();
        run_backup(&f.profile); // I2

        f.profile.merge.rules = Retention { keep_last: 1, ..Retention::KEEP_ALL };
        let report = merge(&f.profile, None, &Recorder::default()).unwrap();
        assert_eq!((report.archives_before, report.archives_after), (3, 2));
        assert!(report.bytes_freed > 0, "the v2 content is gone");

        let list = list_archives(&f.profile, None).unwrap(); // newest first
        assert_eq!(list[1].kind, ArchiveKind::Complete, "the initial complete is untouched");
        assert_eq!(list[0].kind, ArchiveKind::Incremental);
        restore_all(&f, &list[0].name);
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v3");
        assert_eq!(fs::read_to_string(restored(&f, "late.txt")).unwrap(), "late");
        assert_eq!(fs::read_to_string(restored(&f, "sub/ünicode è.txt")).unwrap(), "beta");
        fs::remove_dir_all(&f.restore_to).unwrap();
        restore_all(&f, &list[1].name);
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");

        let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
        assert!(v.problems.is_empty(), "{:?}", v.problems);
    });
}

/// C, I1 (a v2), I2 (b), I3 (c): a and b live in older archives.
fn four_backups() -> Fixture {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    run_backup(&f.profile);
    fs::write(f.source.join("b.txt"), "b").unwrap();
    run_backup(&f.profile);
    fs::write(f.source.join("c.txt"), "c").unwrap();
    run_backup(&f.profile);
    f
}

#[test]
fn manual_merge_of_chosen_incrementals() {
    let f = four_backups();
    let names: Vec<String> =
        list_archives(&f.profile, None).unwrap().into_iter().rev().map(|a| a.name).collect();
    // Choose I2 and I1, in either order.
    merge_range(&f.profile, None, &names[2], &names[1], &Recorder::default()).unwrap();

    let after = list_archives(&f.profile, None).unwrap();
    assert_eq!(after.len(), 3);
    assert_eq!(after[2].name, names[0], "the complete is untouched");
    assert_eq!(after[1].name, names[2], "the merged archive keeps the newest date");
    restore_all(&f, &after[0].name);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2");
    assert_eq!(fs::read_to_string(restored(&f, "c.txt")).unwrap(), "c");
}

#[test]
fn manual_merge_can_include_the_initial_complete() {
    let f = four_backups();
    let names: Vec<String> =
        list_archives(&f.profile, None).unwrap().into_iter().rev().map(|a| a.name).collect();
    merge_range(&f.profile, None, &names[0], &names[2], &Recorder::default()).unwrap();

    let after = list_archives(&f.profile, None).unwrap(); // newest first
    assert_eq!(after.len(), 2);
    assert_eq!(after[1].kind, ArchiveKind::Complete, "a new, later complete archive");
    restore_all(&f, &after[0].name);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2");
    assert_eq!(fs::read_to_string(restored(&f, "b.txt")).unwrap(), "b");
    assert_eq!(fs::read_to_string(restored(&f, "c.txt")).unwrap(), "c");
    fs::remove_dir_all(&f.restore_to).unwrap();
    restore_all(&f, &after[1].name);
    assert!(!restored(&f, "c.txt").exists());

    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

/// A complete backup, then `note.txt` created, backed up, deleted at the
/// source and `a.txt` changed, backed up: three backups, newest first.
fn a_file_deleted_between_backups(keep_deleted: bool) -> (Fixture, Vec<String>) {
    let mut f = fixture(Container::ZipPerFile);
    f.profile.merge.keep_deleted = keep_deleted;
    run_backup(&f.profile);
    fs::write(f.source.join("note.txt"), "short-lived").unwrap();
    run_backup(&f.profile);
    fs::remove_file(f.source.join("note.txt")).unwrap();
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    run_backup(&f.profile);
    let names = list_archives(&f.profile, None).unwrap().into_iter().map(|a| a.name).collect();
    (f, names)
}

#[test]
fn a_merge_keeps_the_files_deleted_inside_it_when_asked() {
    let (f, names) = a_file_deleted_between_backups(true);
    // The two incrementals.
    merge_range(&f.profile, None, &names[1], &names[0], &Recorder::default()).unwrap();
    let after = list_archives(&f.profile, None).unwrap();
    assert_eq!(after.len(), 2);
    restore_all(&f, &after[0].name);
    assert_eq!(
        fs::read_to_string(restored(&f, "note.txt")).unwrap(),
        "short-lived",
        "kept, with its content"
    );
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2", "the newest content");
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

#[test]
fn a_merge_drops_the_files_deleted_inside_it_when_not_asked() {
    let (f, names) = a_file_deleted_between_backups(false);
    merge_range(&f.profile, None, &names[1], &names[0], &Recorder::default()).unwrap();
    let after = list_archives(&f.profile, None).unwrap();
    restore_all(&f, &after[0].name);
    assert!(!restored(&f, "note.txt").exists(), "dropped");
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2");
}

#[test]
fn a_merge_from_the_first_complete_keeps_deleted_files_in_the_new_complete() {
    let (f, names) = a_file_deleted_between_backups(true);
    fs::remove_file(f.source.join("a.txt")).unwrap();
    run_backup(&f.profile);
    // From the first complete to the backup that no longer lists note.txt.
    merge_range(&f.profile, None, &names[2], &names[0], &Recorder::default()).unwrap();
    let after = list_archives(&f.profile, None).unwrap();
    assert_eq!(after[1].kind, ArchiveKind::Complete);
    restore_all(&f, &after[1].name);
    assert_eq!(fs::read_to_string(restored(&f, "note.txt")).unwrap(), "short-lived");
    fs::remove_dir_all(&f.restore_to).unwrap();
    // The backup after the merge still says a.txt was deleted after it.
    restore_all(&f, &after[0].name);
    assert!(!restored(&f, "a.txt").exists());
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

/// `sub/ünicode è.txt` and `a.txt` backed up, then `gone.txt` created and
/// backed up, then `gone.txt` and the whole `sub` deleted at the source and
/// backed up again: the newest backup lists neither.
fn files_deleted_since() -> (Fixture, String) {
    let f = fixture(Container::ZipPerFile);
    fs::write(f.source.join("gone.txt"), "short-lived").unwrap();
    run_backup(&f.profile);
    fs::remove_file(f.source.join("gone.txt")).unwrap();
    fs::remove_dir_all(f.source.join("sub")).unwrap();
    run_backup(&f.profile);
    let newest = list_archives(&f.profile, None).unwrap().remove(0).name;
    (f, newest)
}

#[test]
fn a_folder_with_its_deleted_files_brings_them_back_marked() {
    let (f, newest) = files_deleted_since();
    let mut it = item(&newest, &key_of(&f, ""), &RestoreTarget::Folder { path: f.restore_to.clone() }, false);
    it.with_deleted = true;
    restore(&f.profile, None, &RestoreRequest { items: vec![it] }, &Recorder::default()).unwrap();
    let root = restored(&f, "a.txt");
    let dir = root.parent().unwrap();
    assert_eq!(fs::read_to_string(dir.join("a.txt")).unwrap(), "alpha", "still at the source: its own name");
    assert_eq!(fs::read_to_string(dir.join("_deleted_gone.txt")).unwrap(), "short-lived");
    assert!(!dir.join("gone.txt").exists());
    assert_eq!(
        fs::read_to_string(dir.join("sub").join("_deleted_ünicode è.txt")).unwrap(),
        "beta",
        "in a deleted folder too"
    );
}

#[test]
fn without_deleted_files_a_folder_comes_back_as_at_its_date() {
    let (f, newest) = files_deleted_since();
    let it = item(&newest, &key_of(&f, ""), &RestoreTarget::Folder { path: f.restore_to.clone() }, false);
    restore(&f.profile, None, &RestoreRequest { items: vec![it] }, &Recorder::default()).unwrap();
    let dir = restored(&f, "a.txt").parent().unwrap().to_path_buf();
    assert!(dir.join("a.txt").exists());
    assert!(!dir.join("_deleted_gone.txt").exists() && !dir.join("gone.txt").exists());
}

#[test]
fn a_deleted_file_restored_alone_is_marked() {
    let (f, _) = files_deleted_since();
    let oldest = list_archives(&f.profile, None).unwrap().pop().unwrap().name;
    let mut it =
        item(&oldest, &key_of(&f, "gone.txt"), &RestoreTarget::Folder { path: f.restore_to.clone() }, false);
    it.with_deleted = true;
    restore(&f.profile, None, &RestoreRequest { items: vec![it] }, &Recorder::default()).unwrap();
    let dir = restored(&f, "gone.txt").parent().unwrap().to_path_buf();
    assert_eq!(fs::read_to_string(dir.join("_deleted_gone.txt")).unwrap(), "short-lived");
}

#[test]
fn manual_merge_needs_two_backups() {
    let f = four_backups();
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    assert!(matches!(
        merge_range(&f.profile, None, &name, &name, &Recorder::default()),
        Err(EngineError::InvalidProfile(_))
    ));
}

#[test]
fn verify_detects_damaged_content() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    let stored =
        archeion_core::keys::path_under(&f.profile.destination.path().join(&name).join("data"), &key);
    fs::write(&stored, "tampered").unwrap();

    let full = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert_eq!(full.problems.len(), 1, "{:?}", full.problems);
    let sample = verify(&f.profile, None, Some(10), &Recorder::default()).unwrap();
    assert_eq!(sample.files_checked, 2);
    assert_eq!(sample.problems.len(), 1);
}

#[test]
fn restore_does_not_overwrite_unless_asked() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    fs::write(f.source.join("a.txt"), "edited").unwrap();
    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    let req =
        |overwrite| RestoreRequest::single(&name, vec![key.clone()], &RestoreTarget::Original, overwrite);
    let r = restore(&f.profile, None, &req(false), &Recorder::default()).unwrap();
    assert_eq!((r.files_restored, r.files_kept), (0, 1));
    assert_eq!(fs::read_to_string(f.source.join("a.txt")).unwrap(), "edited");

    let r = restore(&f.profile, None, &req(true), &Recorder::default()).unwrap();
    assert_eq!(r.files_restored, 1);
    assert_eq!(fs::read_to_string(f.source.join("a.txt")).unwrap(), "alpha");
}

#[test]
fn an_unfinished_backup_older_than_the_last_archive_is_removed() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let leftover = f.profile.destination.path().join("2026-01-01_000000_I.partial");
    fs::create_dir_all(leftover.join("data")).unwrap();
    run_backup(&f.profile);
    assert!(!leftover.exists());
    assert_eq!(list_archives(&f.profile, None).unwrap().len(), 2);
}

#[test]
fn destination_in_use_is_busy() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let lock =
        fs::File::options().write(true).open(f.profile.destination.path().join("archeion.lock")).unwrap();
    assert!(!backup_engine_busy(&f.profile), "a lock file alone is not a run");
    lock.lock().unwrap();
    assert!(backup_engine_busy(&f.profile));
    assert!(matches!(
        backup(&f.profile, None, BackupMode::Auto, &Recorder::default()),
        Err(EngineError::Busy)
    ));
    drop(lock);
    assert!(!backup_engine_busy(&f.profile));
}

/// Empties from the Recycle Bin what a test put there from `dir`: the
/// tests use the user's real one.
struct RecycledUnder(PathBuf);

impl Drop for RecycledUnder {
    fn drop(&mut self) {
        let under = self.0.to_string_lossy().to_lowercase();
        let Ok(items) = trash::os_limited::list() else { return };
        let ours: Vec<_> = items
            .into_iter()
            .filter(|i| i.original_parent.to_string_lossy().to_lowercase().starts_with(&under))
            .collect();
        let _ = trash::os_limited::purge_all(ours);
    }
}

fn recycled(dir: &std::path::Path, name: &str) -> bool {
    let parent = dir.to_string_lossy().to_lowercase();
    trash::os_limited::list().unwrap().iter().any(|i| {
        i.name.to_string_lossy() == name && i.original_parent.to_string_lossy().to_lowercase() == parent
    })
}

#[test]
fn trashing_the_backups_moves_the_whole_folder_to_the_recycle_bin() {
    let f = fixture(Container::ZipPerFile);
    let _clean = RecycledUnder(f._dir.path().to_path_buf());
    run_backup(&f.profile);
    run_backup(&f.profile);
    let root = f.profile.destination.path().to_path_buf();
    let done = archeion_engine::trash_destination(&f.profile).unwrap();
    assert_eq!(done, archeion_engine::DestinationTrashed { folder_trashed: true, left: Vec::new() });
    assert!(!root.exists());
    assert!(recycled(f._dir.path(), "backups"), "the folder is in the Recycle Bin");
    let probes = trash::os_limited::list().unwrap();
    assert!(
        !probes.iter().any(|i| i.name.to_string_lossy().starts_with("archeion.recycle-probe")),
        "probe left"
    );
    assert!(f.source.join("a.txt").exists(), "the source is not touched");
}

#[test]
fn trashing_the_backups_leaves_other_files_and_their_folder() {
    let f = fixture(Container::Folder);
    let _clean = RecycledUnder(f._dir.path().to_path_buf());
    run_backup(&f.profile);
    let root = f.profile.destination.path().to_path_buf();
    let archive = list_archives(&f.profile, None).unwrap().remove(0).name;
    fs::write(root.join("notes.txt"), "mine").unwrap();
    fs::create_dir(root.join("Photos")).unwrap();
    let done = archeion_engine::trash_destination(&f.profile).unwrap();
    assert!(!done.folder_trashed);
    assert_eq!(done.left, vec!["Photos".to_owned(), "notes.txt".to_owned()]);
    let mut names: Vec<String> =
        fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["Photos", "notes.txt"], "only what Archeion did not write");
    assert!(recycled(&root, &archive), "the archive is in the Recycle Bin");
    assert!(recycled(&root, "archeion.json"));
}

#[test]
fn the_backups_of_another_profile_or_in_use_are_not_moved() {
    let f = fixture(Container::Folder);
    let _clean = RecycledUnder(f._dir.path().to_path_buf());
    run_backup(&f.profile);
    let root = f.profile.destination.path().to_path_buf();
    let mut other = Profile::new("Other");
    other.destination = f.profile.destination.clone();
    assert!(matches!(archeion_engine::trash_destination(&other), Err(EngineError::ForeignDestination)));
    let lock = fs::File::options().write(true).open(root.join("archeion.lock")).unwrap();
    lock.lock().unwrap();
    assert!(matches!(archeion_engine::trash_destination(&f.profile), Err(EngineError::Busy)));
    drop(lock);
    assert_eq!(list_archives(&f.profile, None).unwrap().len(), 1, "nothing was moved");

    // A folder without backups is left alone.
    let mut empty = Profile::new("Empty");
    let plain = f._dir.path().join("plain");
    fs::create_dir(&plain).unwrap();
    fs::write(plain.join("x.txt"), "x").unwrap();
    empty.destination = Destination::Local { path: plain.clone() };
    let done = archeion_engine::trash_destination(&empty).unwrap();
    assert_eq!(done.left, vec!["x.txt".to_owned()]);
    assert!(plain.join("x.txt").exists());
}

/// Windows deletes what is on a share instead of recycling it, and says
/// it recycled it: nothing must be moved there.
#[cfg(windows)]
#[test]
fn on_a_share_without_a_recycle_bin_nothing_is_moved() {
    let mut f = fixture(Container::Folder);
    run_backup(&f.profile);
    let local = f.profile.destination.path().to_path_buf();
    let text = local.to_string_lossy().into_owned();
    let Some(rest) = text.strip_prefix(r"C:\") else { return };
    let unc = PathBuf::from(format!(r"\\localhost\C$\{rest}"));
    if !unc.join("archeion.json").exists() {
        // No administrative share here: nothing to test.
        return;
    }
    f.profile.destination = Destination::Network { unc: unc.to_string_lossy().into_owned() };
    assert!(matches!(archeion_engine::trash_destination(&f.profile), Err(EngineError::NoRecycleBin(_))));
    assert_eq!(list_archives(&f.profile, None).unwrap().len(), 1, "the backups are all there");
    let left: Vec<String> = fs::read_dir(&local)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!left.iter().any(|n| n.starts_with("archeion.recycle-probe")), "no probe left: {left:?}");
}

fn backup_engine_busy(p: &Profile) -> bool {
    archeion_engine::destination_busy(p)
}

#[test]
fn a_pending_merge_is_seen_from_the_archive_names_alone() {
    let mut f = four_backups();
    assert!(!archeion_engine::merge_pending(&f.profile).unwrap(), "keep everything: nothing to merge");
    f.profile.merge.rules = Retention { keep_last: 2, ..Retention::KEEP_ALL };
    assert!(archeion_engine::merge_pending(&f.profile).unwrap());
    merge(&f.profile, None, &Recorder::default()).unwrap();
    assert!(!archeion_engine::merge_pending(&f.profile).unwrap(), "merged: nothing left");
    let mut empty = fixture(Container::Folder);
    empty.profile.merge.rules = Retention { keep_last: 1, ..Retention::KEEP_ALL };
    assert!(!archeion_engine::merge_pending(&empty.profile).unwrap(), "no destination yet");
}

#[test]
fn destination_of_another_profile_is_refused() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let mut other = f.profile.clone();
    other.id = archeion_core::ProfileId::new_random();
    assert!(matches!(
        backup(&other, None, BackupMode::Auto, &Recorder::default()),
        Err(EngineError::ForeignDestination)
    ));

    // A profile re-created after the old one was deleted takes the backups over.
    assert_eq!(archeion_engine::destination_owner(&other).unwrap(), Some(f.profile.id));
    archeion_engine::adopt_destination(&other).unwrap();
    assert_eq!(archeion_engine::destination_owner(&other).unwrap(), Some(other.id));
    let report = backup_as(&other, BackupMode::Incremental);
    assert!(!report.complete, "the old chain goes on");
    assert_eq!(list_archives(&other, None).unwrap().len(), 2);
    assert!(matches!(
        backup(&f.profile, None, BackupMode::Auto, &Recorder::default()),
        Err(EngineError::ForeignDestination)
    ));
}

#[cfg(windows)]
#[test]
fn missing_drive_is_unavailable() {
    let mut f = fixture(Container::Folder);
    if std::path::Path::new(r"Q:\").exists() {
        return;
    }
    f.profile.destination = Destination::Local { path: r"Q:\backups".into() };
    assert!(matches!(
        backup(&f.profile, None, BackupMode::Auto, &Recorder::default()),
        Err(EngineError::DestinationUnavailable(_))
    ));
}

#[cfg(windows)]
#[test]
fn locked_file_is_skipped_and_its_old_version_kept() {
    use std::os::windows::fs::OpenOptionsExt as _;
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "changed while locked").unwrap();
    // No sharing: other programs cannot open it, like an open Outlook .pst.
    let _held = fs::OpenOptions::new().read(true).share_mode(0).open(f.source.join("a.txt")).unwrap();

    let report = run_backup(&f.profile);
    assert_eq!(report.skipped.len(), 1);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    restore_all(&f, &name);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
}

#[test]
fn zip_per_file_keeps_the_name_inside_and_opens_with_any_zip_tool() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    let mut zipped =
        archeion_core::keys::path_under(&f.profile.destination.path().join(&name).join("zipped"), &key)
            .into_os_string();
    zipped.push(".zip");
    let mut zip = zip::ZipArchive::new(fs::File::open(PathBuf::from(zipped)).unwrap()).unwrap();
    assert_eq!(zip.len(), 1);
    let mut entry = zip.by_index(0).unwrap();
    assert_eq!(entry.name().unwrap(), "a.txt");
    let mut text = String::new();
    std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
    assert_eq!(text, "alpha");
}

#[test]
fn versions_list_only_real_changes_and_each_restores_beside_the_original() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile); // v1 "alpha"
    run_backup(&f.profile); // unchanged: no new version
    fs::write(f.source.join("a.txt"), "second").unwrap();
    run_backup(&f.profile); // v2

    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    let versions = file_versions(&f.profile, None, &key).unwrap();
    assert_eq!(versions.len(), 2, "newest first, one per content");
    assert_eq!(versions[0].size, 6);
    assert_eq!(versions[1].size, 5);

    let req = RestoreRequest::single(&versions[1].archive, vec![key], &RestoreTarget::BesideOriginal, false);
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert_eq!(r.files_restored, 1);
    assert_eq!(fs::read_to_string(f.source.join("a.txt")).unwrap(), "second", "current file untouched");
    let tag = versions[1].backup_label.replace(':', ".");
    let beside = f.source.join(format!("a ({tag}).txt"));
    assert_eq!(fs::read_to_string(beside).unwrap(), "alpha");
}

fn item(archive: &str, key: &str, target: &RestoreTarget, overwrite: bool) -> RestoreItem {
    RestoreItem {
        with_deleted: false,
        archive: archive.to_owned(),
        key: Some(key.to_owned()),
        target: target.clone(),
        overwrite,
        exclude: Vec::new(),
    }
}

fn key_of(f: &Fixture, relative: &str) -> String {
    archeion_core::keys::key_for(&f.source.join(relative)).unwrap()
}

#[test]
fn several_versions_to_one_folder_each_get_their_date_in_the_name() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile); // "alpha"
    fs::write(f.source.join("a.txt"), "second").unwrap();
    run_backup(&f.profile);
    let key = key_of(&f, "a.txt");
    let versions = file_versions(&f.profile, None, &key).unwrap();
    assert_eq!(versions.len(), 2);

    let to = RestoreTarget::Folder { path: f.restore_to.clone() };
    let req = RestoreRequest { items: versions.iter().map(|v| item(&v.archive, &key, &to, false)).collect() };
    let rec = Recorder::default();
    let r = restore(&f.profile, None, &req, &rec).unwrap();
    assert_eq!(r.files_restored, 2, "{:?}", r.skipped);
    let plain = restored(&f, "a.txt");
    assert!(!plain.exists(), "neither version takes the plain name");
    // Each version takes its date in the name (with the seconds when both
    // backups fall in the same minute, as in this test).
    for (v, text) in versions.iter().zip(["second", "alpha"]) {
        let minute =
            archeion_core::restore::with_date_tag(&plain, &archeion_core::restore::date_tag(&v.archive));
        let second = archeion_core::restore::with_date_tag(
            &plain,
            &archeion_core::restore::precise_date_tag(&v.archive),
        );
        let found = [&minute, &second].into_iter().any(|p| fs::read_to_string(p).is_ok_and(|t| t == text));
        assert!(found, "{text}: neither {} nor {}", minute.display(), second.display());
    }
    let events = rec.0.lock().unwrap();
    let finished = events.iter().filter(|e| matches!(e, RunEvent::Finished { .. })).count();
    assert_eq!(finished, 1, "one run, one report");
}

#[test]
fn a_folder_comes_back_as_it_was_at_an_old_date_beside_a_newer_file() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::write(f.source.join("sub").join("ünicode è.txt"), "beta v2").unwrap();
    fs::write(f.source.join("sub").join("later.txt"), "later").unwrap();
    run_backup(&f.profile);
    let list = list_archives(&f.profile, None).unwrap(); // newest first

    let to = RestoreTarget::Folder { path: f.restore_to.clone() };
    let req = RestoreRequest {
        items: vec![
            item(&list[1].name, &key_of(&f, "sub"), &to, false),
            // Unchanged since the first backup: read from there.
            item(&list[0].name, &key_of(&f, "a.txt"), &to, false),
        ],
    };
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    assert_eq!(r.files_restored, 2);
    assert_eq!(fs::read_to_string(restored(&f, "sub/ünicode è.txt")).unwrap(), "beta");
    assert!(!restored(&f, "sub/later.txt").exists(), "not there at that date");
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
}

#[test]
fn a_folder_leaves_out_what_was_unticked_inside_it() {
    let f = fixture(Container::Folder);
    fs::create_dir_all(f.source.join("sub").join("deep")).unwrap();
    fs::write(f.source.join("sub").join("deep").join("d.txt"), "delta").unwrap();
    fs::write(f.source.join("sub").join("keep.txt"), "keep").unwrap();
    run_backup(&f.profile);
    let list = list_archives(&f.profile, None).unwrap();

    let to = RestoreTarget::Folder { path: f.restore_to.clone() };
    let mut folder = item(&list[0].name, &key_of(&f, "sub"), &to, false);
    folder.exclude = vec![key_of(&f, "sub/deep"), key_of(&f, "sub/ünicode è.txt")];
    let req = RestoreRequest { items: vec![folder] };
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    assert_eq!(fs::read_to_string(restored(&f, "sub/keep.txt")).unwrap(), "keep");
    assert!(!restored(&f, "sub/ünicode è.txt").exists(), "an unticked file");
    assert!(!restored(&f, "sub/deep").exists(), "an unticked folder, with what it holds");
    assert_eq!(r.files_restored, 1);
}

#[test]
fn one_version_overwrites_the_original_and_another_goes_beside_it() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile); // "alpha"
    fs::write(f.source.join("a.txt"), "second").unwrap();
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "edited").unwrap();
    let key = key_of(&f, "a.txt");
    let versions = file_versions(&f.profile, None, &key).unwrap(); // newest first

    let req = RestoreRequest {
        items: vec![
            item(&versions[1].archive, &key, &RestoreTarget::Original, true),
            item(&versions[0].archive, &key, &RestoreTarget::BesideOriginal, false),
        ],
    };
    let r = restore(&f.profile, None, &req, &Recorder::default()).unwrap();
    assert_eq!((r.files_restored, r.files_kept), (2, 0), "{:?}", r.skipped);
    assert_eq!(fs::read_to_string(f.source.join("a.txt")).unwrap(), "alpha");
    let tag = archeion_core::restore::date_tag(&versions[0].archive);
    let beside = archeion_core::restore::with_date_tag(&f.source.join("a.txt"), &tag);
    assert_eq!(fs::read_to_string(beside).unwrap(), "second");
}

#[test]
fn catalog_lists_every_file_ever_saved_and_marks_the_deleted_ones() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::remove_file(f.source.join("a.txt")).unwrap();
    fs::write(f.source.join("new.txt"), "gamma").unwrap();
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "back").unwrap();
    run_backup(&f.profile);
    fs::remove_file(f.source.join("new.txt")).unwrap();
    run_backup(&f.profile);

    let c = archeion_engine::catalog(&f.profile, None).unwrap();
    assert_eq!(c.archives.len(), 4);
    let entry = |rel: &str| c.entries.iter().find(|e| e.key == key_of(&f, rel)).unwrap().clone();
    let a = entry("a.txt");
    assert_eq!(a.present, vec![(0, 0), (2, 3)]);
    assert!(!a.deleted(c.archives.len()));
    assert_eq!(a.archives_newest_first().collect::<Vec<_>>(), vec![3, 2, 0]);
    let new = entry("new.txt");
    assert_eq!(new.present, vec![(1, 2)]);
    assert!(new.deleted(c.archives.len()), "gone at the newest backup");
    let sub = entry("sub");
    assert_eq!(sub.present, vec![(0, 3)]);
    let mut keys: Vec<&str> = c.entries.iter().map(|e| e.key.as_str()).collect();
    let sorted = keys.clone();
    keys.sort_unstable();
    assert_eq!(keys, sorted, "sorted by key");
}

#[test]
fn search_finds_files_by_name_across_dates() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::remove_file(f.source.join("a.txt")).unwrap();
    run_backup(&f.profile);

    let hits = search(&f.profile, None, "A.TX", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(!hits[0].present_now, "deleted in the newest backup");
    assert_eq!(hits[0].versions, 1);
    assert!(search(&f.profile, None, "  ", 10).unwrap().is_empty());
    assert_eq!(search(&f.profile, None, "txt", 1).unwrap().len(), 1, "limit applies");
}

fn backup_as(p: &Profile, mode: BackupMode) -> archeion_core::BackupReport {
    sleep(Duration::from_millis(1100));
    backup(p, None, mode, &Recorder::default()).unwrap()
}

fn kinds(f: &Fixture) -> String {
    list_archives(&f.profile, None)
        .unwrap()
        .iter()
        .rev()
        .map(|a| if a.kind == ArchiveKind::Complete { 'C' } else { 'I' })
        .collect()
}

#[test]
fn complete_backup_on_demand_starts_a_new_chain() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    backup_as(&f.profile, BackupMode::Incremental);
    let report = backup_as(&f.profile, BackupMode::Complete);
    assert!(report.complete);
    assert_eq!(report.files_unchanged, 2, "nothing changed, everything copied");
    assert!(report.bytes_copied > 0);
    assert_eq!(kinds(&f), "CIC");

    let newest = list_archives(&f.profile, None).unwrap().remove(0);
    assert_eq!(newest.files_stored, newest.files_total, "a complete archive holds every file");
    restore_all(&f, &newest.name);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2");

    let key = archeion_core::keys::key_for(&f.source.join("sub").join("ünicode è.txt")).unwrap();
    assert_eq!(file_versions(&f.profile, None, &key).unwrap().len(), 1, "copied again, same version");
    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    assert_eq!(file_versions(&f.profile, None, &key).unwrap().len(), 2);
}

#[test]
fn full_backup_rule_applies_to_automatic_backups_only() {
    let mut f = fixture(Container::Folder);
    f.profile.full_backup = FullBackupRule::Backups { count: 2 };
    run_backup(&f.profile);
    run_backup(&f.profile);
    run_backup(&f.profile);
    assert_eq!(kinds(&f), "CIC");
    backup_as(&f.profile, BackupMode::Incremental);
    backup_as(&f.profile, BackupMode::Incremental);
    assert_eq!(kinds(&f), "CICII", "the user's choice wins");
}

#[test]
fn automatic_merge_across_chains_keeps_every_date_restorable() {
    let mut f = fixture(Container::ZipPerFile);
    run_backup(&f.profile); // C
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    run_backup(&f.profile); // I
    backup_as(&f.profile, BackupMode::Complete); // C2
    fs::write(f.source.join("c.txt"), "c").unwrap();
    run_backup(&f.profile); // I
    f.profile.merge.rules = Retention { keep_last: 1, ..Retention::KEEP_ALL };
    merge(&f.profile, None, &Recorder::default()).unwrap();
    assert_eq!(kinds(&f), "CC", "the initial complete stays; the rest becomes one complete archive");

    let list = list_archives(&f.profile, None).unwrap();
    restore_all(&f, &list[0].name);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2");
    assert_eq!(fs::read_to_string(restored(&f, "c.txt")).unwrap(), "c");
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

#[test]
fn deleting_an_intermediate_backup_keeps_the_later_ones_whole() {
    let f = four_backups(); // C, I1 (a v2), I2 (b), I3 (c)
    let names: Vec<String> =
        list_archives(&f.profile, None).unwrap().into_iter().rev().map(|a| a.name).collect();

    delete_archive(&f.profile, None, &names[1], &Recorder::default()).unwrap();
    let after: Vec<String> =
        list_archives(&f.profile, None).unwrap().into_iter().rev().map(|a| a.name).collect();
    assert_eq!(after, vec![names[0].clone(), names[2].clone(), names[3].clone()]);
    restore_all(&f, &names[3]);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "v2", "moved into the next backup");
    assert_eq!(fs::read_to_string(restored(&f, "b.txt")).unwrap(), "b");
    fs::remove_dir_all(&f.restore_to).unwrap();
    restore_all(&f, &names[0]);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");

    delete_archive(&f.profile, None, &names[3], &Recorder::default()).unwrap();
    assert_eq!(list_archives(&f.profile, None).unwrap().len(), 2, "the newest is simply removed");

    let refused = delete_archive(&f.profile, None, &names[0], &Recorder::default());
    assert!(matches!(refused, Err(EngineError::InvalidProfile(_))), "the initial complete stays");
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

#[test]
fn the_only_backup_can_be_deleted() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    delete_archive(&f.profile, None, &name, &Recorder::default()).unwrap();
    assert!(list_archives(&f.profile, None).unwrap().is_empty());
    assert_eq!(archeion_engine::existing_format(&f.profile).unwrap(), None, "the format is free again");
    assert!(run_backup(&f.profile).complete, "the next backup starts over");
}

fn pw(s: &str) -> Password {
    Password::new(s.to_owned()).unwrap()
}

/// Every path below `dir`, relative, with `/`.
fn all_paths(dir: &std::path::Path) -> Vec<String> {
    walkdir_paths(dir).into_iter().map(|p| p.to_string_lossy().replace('\\', "/")).collect()
}

fn walkdir_paths(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            out.push(p.strip_prefix(dir).unwrap().to_path_buf());
            if p.is_dir() {
                stack.push(p);
            }
        }
    }
    out
}

#[test]
fn encrypted_backups_hide_names_and_content_and_need_the_password() {
    let mut f = fixture(Container::ZipPerFile);
    f.profile.encrypted = true;
    let secret = pw("correct horse battery");
    let rec = Recorder::default();
    assert!(matches!(backup(&f.profile, None, BackupMode::Auto, &rec), Err(EngineError::PasswordNeeded)));
    backup(&f.profile, Some(&secret), BackupMode::Auto, &rec).unwrap();
    sleep(Duration::from_millis(1100));
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    backup(&f.profile, Some(&secret), BackupMode::Auto, &rec).unwrap();

    let dest = f.profile.destination.path().to_path_buf();
    for p in all_paths(&dest) {
        assert!(!p.contains("a.txt") && !p.contains("sub") && !p.contains("nicode"), "name visible: {p}");
        assert!(!p.ends_with("manifest.jsonl"), "manifest in clear: {p}");
    }
    for p in walkdir_paths(&dest).into_iter().map(|p| dest.join(p)).filter(|p| p.is_file()) {
        let bytes = fs::read(&p).unwrap();
        assert!(!bytes.windows(5).any(|w| w == b"alpha"), "content in clear: {}", p.display());
        assert!(!bytes.windows(5).any(|w| w == b"a.txt"), "name inside: {}", p.display());
    }

    assert!(matches!(list_archives(&f.profile, None), Err(EngineError::PasswordNeeded)));
    assert!(matches!(list_archives(&f.profile, Some(&pw("nope"))), Err(EngineError::WrongPassword)));
    assert!(matches!(check_password(&f.profile, &pw("nope")), Err(EngineError::WrongPassword)));
    check_password(&f.profile, &secret).unwrap();
    assert_eq!(
        archeion_engine::existing_format(&f.profile).unwrap(),
        Some(ExistingFormat { container: Container::ZipPerFile, encrypted: true })
    );

    let list = list_archives(&f.profile, Some(&secret)).unwrap();
    assert_eq!(list.len(), 2);
    let req = RestoreRequest::single(
        &list[1].name,
        Vec::new(),
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, Some(&secret), &req, &rec).unwrap();
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(restored(&f, "sub/ünicode è.txt")).unwrap(), "beta");

    let key = archeion_core::keys::key_for(&f.source.join("a.txt")).unwrap();
    assert_eq!(file_versions(&f.profile, Some(&secret), &key).unwrap().len(), 2);
    assert_eq!(search(&f.profile, Some(&secret), "a.tx", 10).unwrap().len(), 1);

    f.profile.merge.rules = Retention { keep_last: 1, ..Retention::KEEP_ALL };
    merge_range(&f.profile, Some(&secret), &list[1].name, &list[0].name, &rec).unwrap();
    let v = verify(&f.profile, Some(&secret), None, &rec).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
    assert_eq!(v.files_checked, 2);
}

#[test]
fn encryption_switched_on_once_backups_exist_writes_nothing_in_the_clear() {
    let mut f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    f.profile.encrypted = true;
    sleep(Duration::from_millis(1100));
    let r = backup(&f.profile, Some(&pw("late")), BackupMode::Auto, &Recorder::default());
    assert!(matches!(r, Err(EngineError::EncryptionMissing)), "{r:?}");
    assert_eq!(
        archeion_engine::existing_format(&f.profile).unwrap(),
        Some(ExistingFormat { container: Container::ZipPerFile, encrypted: false })
    );
    assert_eq!(list_archives(&f.profile, None).unwrap().len(), 1, "no new backup");
}

/// An encrypted destination with one backup, and its password.
fn encrypted_backup() -> (Fixture, Password, String) {
    let mut f = fixture(Container::ZipPerFile);
    f.profile.encrypted = true;
    let secret = pw("correct horse battery");
    backup(&f.profile, Some(&secret), BackupMode::Auto, &Recorder::default()).unwrap();
    let name = list_archives(&f.profile, Some(&secret)).unwrap().remove(0).name;
    (f, secret, name)
}

#[test]
fn removing_the_encryption_description_does_not_make_backups_plain() {
    let (mut f, secret, _) = encrypted_backup();
    let dest = f.profile.destination.path().to_path_buf();
    let descriptor = dest.join("archeion.json");
    let mut d: serde_json::Value = serde_json::from_str(&fs::read_to_string(&descriptor).unwrap()).unwrap();
    d.as_object_mut().unwrap().remove("encryption");
    fs::write(&descriptor, d.to_string()).unwrap();

    sleep(Duration::from_millis(1100));
    for encrypted in [true, false] {
        f.profile.encrypted = encrypted;
        let r = backup(&f.profile, Some(&secret), BackupMode::Auto, &Recorder::default());
        assert!(matches!(r, Err(EngineError::EncryptionMissing)), "profile encrypted {encrypted}: {r:?}");
    }
    // The settings see the destination as encrypted, so the profile follows it.
    assert_eq!(
        archeion_engine::existing_format(&f.profile).unwrap(),
        Some(ExistingFormat { container: Container::ZipPerFile, encrypted: true })
    );
    // Also with the description gone altogether.
    fs::remove_file(&descriptor).unwrap();
    let r = backup(&f.profile, Some(&secret), BackupMode::Auto, &Recorder::default());
    assert!(matches!(r, Err(EngineError::EncryptionMissing)), "{r:?}");
    assert!(!all_paths(&dest).iter().any(|p| p.ends_with("manifest.jsonl")), "nothing in the clear");
}

#[test]
fn a_plain_archive_put_in_an_encrypted_destination_is_not_restored() {
    let (f, secret, name) = encrypted_backup();
    let archive = f.profile.destination.path().join(&name);
    // A plain backup of other content, moved in under the same name.
    let mut other = fixture(Container::Folder);
    fs::write(other.source.join("a.txt"), "planted").unwrap();
    other.profile.sources = f.profile.sources.clone();
    backup(&other.profile, None, BackupMode::Auto, &Recorder::default()).unwrap();
    let plain =
        other.profile.destination.path().join(list_archives(&other.profile, None).unwrap().remove(0).name);
    fs::remove_dir_all(&archive).unwrap();
    fs::rename(&plain, &archive).unwrap();

    let req = RestoreRequest::single(
        &name,
        Vec::new(),
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, Some(&secret), &req, &Recorder::default());
    assert!(matches!(r, Err(EngineError::Damaged { .. })), "{r:?}");
    assert!(!restored(&f, "a.txt").exists());
}

#[test]
fn a_plain_zip_put_in_the_vault_is_not_restored() {
    use std::io::Write as _;
    let (f, secret, name) = encrypted_backup();
    let vault = f.profile.destination.path().join(&name).join("vault");
    for zip_path in walkdir_paths(&vault).into_iter().map(|p| vault.join(p)).filter(|p| p.is_file()) {
        let mut z = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        z.start_file("data", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b"planted").unwrap();
        z.finish().unwrap();
    }
    let req = RestoreRequest::single(
        &name,
        Vec::new(),
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, Some(&secret), &req, &Recorder::default()).unwrap();
    assert!(!r.skipped.is_empty(), "the planted files are refused");
    assert!(
        !restored(&f, "a.txt").exists() || fs::read_to_string(restored(&f, "a.txt")).unwrap() != "planted"
    );
}

#[test]
fn a_doctored_key_cannot_restore_outside_the_folder() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    let name = list_archives(&f.profile, None).unwrap().remove(0).name;
    let manifest = f.profile.destination.path().join(&name).join("manifest.jsonl");
    let text = fs::read_to_string(&manifest).unwrap();
    let doctored: Vec<String> = text
        .lines()
        .map(|l| {
            if !l.contains("/a.txt\"") {
                return l.to_owned();
            }
            let mut e: serde_json::Value = serde_json::from_str(l).unwrap();
            let key = e["key"].as_str().unwrap().replace("/a.txt", "/../../../../evil.txt");
            e["key"] = key.into();
            e.to_string()
        })
        .collect();
    fs::write(&manifest, doctored.join("\n") + "\n").unwrap();

    let req = RestoreRequest::single(
        &name,
        Vec::new(),
        &RestoreTarget::Folder { path: f.restore_to.clone() },
        true,
    );
    let r = restore(&f.profile, None, &req, &Recorder::default());
    assert!(matches!(r, Err(EngineError::Damaged { .. })), "{r:?}");
    assert!(!f._dir.path().join("evil.txt").exists());
    assert!(!f.restore_to.parent().unwrap().join("evil.txt").exists());
}

#[test]
fn the_recovery_key_opens_the_zips_and_the_password_can_change() {
    use std::io::Read as _;
    let mut f = fixture(Container::ZipPerFile);
    f.profile.encrypted = true;
    let rec = Recorder::default();
    let old = pw("first password");
    backup(&f.profile, Some(&old), BackupMode::Auto, &rec).unwrap();
    let key = archeion_engine::recovery_key(&f.profile, &old).unwrap().unwrap().replace(' ', "");
    assert_eq!(key.len(), 64);

    // What 7-Zip does with the recovery key: open the manifest, find the
    // stored name of a file, open that file.
    let archive = list_archives(&f.profile, Some(&old)).unwrap().remove(0).name;
    let dir = f.profile.destination.path().join(&archive);
    let mut z = zip::ZipArchive::new(fs::File::open(dir.join("manifest.zip")).unwrap()).unwrap();
    let mut text = String::new();
    z.by_index_decrypt(0, key.as_bytes()).unwrap().read_to_string(&mut text).unwrap();
    let line = text.lines().find(|l| l.contains("/a.txt\"")).unwrap();
    let entry: serde_json::Value = serde_json::from_str(line).unwrap();
    let blob = entry["blob"].as_str().unwrap().to_owned();
    assert!(blob.starts_with("vault/"));
    let mut z = zip::ZipArchive::new(fs::File::open(dir.join(&blob)).unwrap()).unwrap();
    let mut content = String::new();
    z.by_index_decrypt(0, key.as_bytes()).unwrap().read_to_string(&mut content).unwrap();
    assert_eq!(content, "alpha");

    let new = pw("second password");
    archeion_engine::change_password(&f.profile, &old, &new).unwrap();
    assert!(matches!(list_archives(&f.profile, Some(&old)), Err(EngineError::WrongPassword)));
    assert_eq!(list_archives(&f.profile, Some(&new)).unwrap().len(), 1);
    let same = archeion_engine::recovery_key(&f.profile, &new).unwrap().unwrap().replace(' ', "");
    assert_eq!(same, key, "the backups were not re-encrypted");
}

#[test]
fn a_backup_removed_from_outside_is_noticed_by_the_check() {
    let mut f = fixture(Container::ZipPerFile);
    f.profile.encrypted = true;
    let secret = pw("a long enough secret");
    let rec = Recorder::default();
    backup(&f.profile, Some(&secret), BackupMode::Auto, &rec).unwrap();
    sleep(Duration::from_millis(1100));
    fs::write(f.source.join("a.txt"), "v2").unwrap();
    backup(&f.profile, Some(&secret), BackupMode::Auto, &rec).unwrap();
    let clean = verify(&f.profile, Some(&secret), None, &rec).unwrap();
    assert!(clean.problems.is_empty(), "{:?}", clean.problems);

    let newest = list_archives(&f.profile, Some(&secret)).unwrap().remove(0).name;
    fs::remove_dir_all(f.profile.destination.path().join(newest)).unwrap();
    let v = verify(&f.profile, Some(&secret), None, &rec).unwrap();
    assert!(v.problems.iter().any(|p| p.contains("signature")), "{:?}", v.problems);
}

/// Adds `b1.txt` … `b4.txt`, copied in name order after `a.txt`.
fn more_files(f: &Fixture) {
    for i in 1..=4 {
        fs::write(f.source.join(format!("b{i}.txt")), format!("bravo {i}")).unwrap();
    }
}

fn partials(f: &Fixture) -> Vec<PathBuf> {
    fs::read_dir(f.profile.destination.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".partial"))
        .collect()
}

fn stop_after(f: &Fixture, password: Option<&Password>, mode: BackupMode, files: u64) {
    sleep(Duration::from_millis(1100));
    let r = backup_stopping_after(&f.profile, password, mode, &Recorder::default(), files);
    assert!(r.is_err(), "the test run stops");
}

#[test]
fn an_interrupted_backup_resumes_where_it_stopped() {
    check_both_containers(|c| {
        let f = fixture(c);
        more_files(&f);
        stop_after(&f, None, BackupMode::Auto, 2);
        assert_eq!(partials(&f).len(), 1, "the unfinished backup is kept");
        assert!(list_archives(&f.profile, None).unwrap().is_empty(), "nothing committed");

        let report = backup(&f.profile, None, BackupMode::Auto, &Recorder::default()).unwrap();
        assert_eq!(report.resumed_files, 2);
        assert_eq!(report.files_new, 6);
        assert!(report.complete);
        assert!(partials(&f).is_empty());
        let dir = f.profile.destination.path().join(&report.archive);
        assert!(!dir.join("resume.jsonl").exists(), "the journal is not committed");

        restore_all(&f, &report.archive);
        assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
        assert_eq!(fs::read_to_string(restored(&f, "b4.txt")).unwrap(), "bravo 4");
        let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
        assert!(v.problems.is_empty(), "{:?}", v.problems);
    });
}

#[test]
fn files_changed_or_deleted_since_the_interruption_are_handled() {
    let f = fixture(Container::Folder);
    more_files(&f);
    stop_after(&f, None, BackupMode::Auto, 3); // a.txt, b1.txt, b2.txt
    sleep(Duration::from_millis(50));
    fs::write(f.source.join("a.txt"), "alpha, edited").unwrap();
    fs::remove_file(f.source.join("b1.txt")).unwrap();

    let report = backup(&f.profile, None, BackupMode::Auto, &Recorder::default()).unwrap();
    assert_eq!(report.resumed_files, 1, "only b2.txt is kept as it was");
    let paths = all_paths(&f.profile.destination.path().join(&report.archive));
    assert!(!paths.iter().any(|p| p.ends_with("b1.txt")), "leftover removed: {paths:?}");
    restore_all(&f, &report.archive);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha, edited");
    let v = verify(&f.profile, None, None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
}

#[test]
fn a_different_kind_chosen_by_the_user_starts_over() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    more_files(&f);
    stop_after(&f, None, BackupMode::Incremental, 2);
    let report = backup_as(&f.profile, BackupMode::Complete);
    assert_eq!(report.resumed_files, 0);
    assert_eq!(kinds(&f), "CC");
    assert!(partials(&f).is_empty());
}

#[test]
fn a_scheduled_backup_resumes_an_incremental() {
    let f = fixture(Container::ZipPerFile);
    run_backup(&f.profile);
    more_files(&f);
    stop_after(&f, None, BackupMode::Incremental, 2);
    let report = backup(&f.profile, None, BackupMode::Auto, &Recorder::default()).unwrap();
    assert_eq!(report.resumed_files, 2);
    assert_eq!(report.files_new, 4);
    assert_eq!(kinds(&f), "CI");
}

#[test]
fn an_encrypted_backup_resumes_without_revealing_names() {
    let mut f = fixture(Container::ZipPerFile);
    f.profile.encrypted = true;
    let secret = pw("correct horse battery");
    more_files(&f);
    stop_after(&f, Some(&secret), BackupMode::Auto, 2);
    let partial = partials(&f).pop().unwrap();
    let journal = fs::read_to_string(partial.join("resume.jsonl")).unwrap();
    assert_eq!(journal.lines().count(), 2);
    assert!(!journal.contains("a.txt") && !journal.contains("b1.txt"), "names in the journal: {journal}");
    let alpha = blake3::hash(b"alpha").to_hex().to_string();
    assert!(!journal.contains(&alpha), "hashes in the journal");

    assert!(matches!(
        backup(&f.profile, Some(&pw("wrong password")), BackupMode::Auto, &Recorder::default()),
        Err(EngineError::WrongPassword)
    ));
    let report = backup(&f.profile, Some(&secret), BackupMode::Auto, &Recorder::default()).unwrap();
    assert_eq!(report.resumed_files, 2);
    let v = verify(&f.profile, Some(&secret), None, &Recorder::default()).unwrap();
    assert!(v.problems.is_empty(), "{:?}", v.problems);
    assert_eq!(v.files_checked, 6);
}

#[test]
fn a_damaged_copy_in_the_unfinished_backup_is_copied_again() {
    let f = fixture(Container::ZipPerFile);
    more_files(&f);
    stop_after(&f, None, BackupMode::Auto, 2);
    let partial = partials(&f).pop().unwrap();
    let zip = walkdir_paths(&partial)
        .into_iter()
        .map(|p| partial.join(p))
        .find(|p| p.to_string_lossy().ends_with("a.txt.zip"))
        .unwrap();
    let bytes = fs::read(&zip).unwrap();
    fs::write(&zip, &bytes[..bytes.len() / 2]).unwrap();

    let report = backup(&f.profile, None, BackupMode::Auto, &Recorder::default()).unwrap();
    assert_eq!(report.resumed_files, 1);
    restore_all(&f, &report.archive);
    assert_eq!(fs::read_to_string(restored(&f, "a.txt")).unwrap(), "alpha");
}

impl Recorder {
    /// Every file of the [`RunEvent::Files`] events, in order.
    fn listed(&self) -> Vec<FileDone> {
        let events = self.0.lock().unwrap();
        let batches = events.iter().filter_map(|e| match e {
            RunEvent::Files { files } => Some(files.clone()),
            _ => None,
        });
        batches.flatten().collect()
    }
}

fn named(files: &[FileDone], action: FileAction) -> Vec<String> {
    let names = files.iter().filter(|f| f.action == action);
    names.map(|f| f.path.rsplit(['/', '\\']).next().unwrap_or_default().to_owned()).collect()
}

#[test]
fn a_backup_lists_every_file_it_handled_beyond_the_latest_ones() {
    let f = fixture(Container::Folder);
    let many = RECENT_FILES + 15;
    let name = |i: usize| format!("many {i:02}.txt");
    for i in 0..many {
        fs::write(f.source.join(name(i)), "x").unwrap();
    }
    sleep(Duration::from_millis(1100));
    let first = Recorder::default();
    backup(&f.profile, None, BackupMode::Auto, &first).unwrap();
    assert_eq!(named(&first.listed(), FileAction::New).len(), many + 2, "every file of the first backup");

    for i in 0..many {
        fs::write(f.source.join(name(i)), format!("changed {i}")).unwrap();
    }
    fs::remove_file(f.source.join("sub").join("ünicode è.txt")).unwrap();
    sleep(Duration::from_millis(1100));
    let second = Recorder::default();
    let report = backup(&f.profile, None, BackupMode::Auto, &second).unwrap();
    let files = second.listed();
    let changed = named(&files, FileAction::Changed);
    assert_eq!(changed, (0..many).map(name).collect::<Vec<_>>(), "all of them, in order, once each");
    assert_eq!(named(&files, FileAction::Deleted), ["ünicode è.txt"]);
    assert!(named(&files, FileAction::Unchanged).is_empty(), "a.txt is only counted");
    assert_eq!(report.files_unchanged, 1);
    assert_eq!(files.len(), many + 1);
    let deleted = files.iter().find(|f| f.action == FileAction::Deleted).unwrap();
    assert_eq!(deleted.bytes, 4, "size of the version in the previous backup");
    assert!(deleted.path.contains("source folder"), "full path: {}", deleted.path);

    // Every file is listed before the end of the run.
    let events = second.0.lock().unwrap();
    let last_files = events.iter().rposition(|e| matches!(e, RunEvent::Files { .. })).unwrap();
    let finished = events.iter().position(|e| matches!(e, RunEvent::Finished { .. })).unwrap();
    assert!(last_files < finished);
}

#[test]
fn a_merge_lists_the_files_it_carries_and_the_versions_it_drops() {
    let f = fixture(Container::Folder);
    run_backup(&f.profile);
    fs::write(f.source.join("a.txt"), "alpha v2").unwrap();
    run_backup(&f.profile);
    let list = list_archives(&f.profile, None).unwrap(); // newest first
    let r = Recorder::default();
    merge_range(&f.profile, None, &list[1].name, &list[0].name, &r).unwrap();
    let files = r.listed();
    let mut moved = named(&files, FileAction::Moved);
    moved.sort();
    assert_eq!(moved, ["a.txt", "ünicode è.txt"]);
    let dropped: Vec<&FileDone> = files.iter().filter(|f| f.action == FileAction::Dropped).collect();
    assert_eq!(dropped.len(), 1, "{files:?}");
    assert!(dropped[0].path.ends_with("a.txt") && dropped[0].bytes == 5, "the first a.txt: {:?}", dropped[0]);
}
