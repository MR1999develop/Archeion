//! Tighter merge rules wait for the user: the automatic merge finds no
//! work until they are confirmed.

use archeion_core::{BackupMode, Container, Destination, Profile, Retention, RunEvent};
use archeion_engine::{EventSink, archive_names, backup, merge, merge_pending};
use std::{fs, thread::sleep, time::Duration};

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

fn keep_last(n: u32) -> Retention {
    Retention {
        keep_within_days: 0,
        keep_last: n,
        keep_daily: 0,
        keep_weekly: 0,
        keep_monthly: 0,
        keep_yearly: 0,
    }
}

#[test]
fn tighter_rules_merge_only_once_confirmed() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("src");
    fs::create_dir_all(&source).unwrap();
    let mut profile = Profile::new("Wait");
    profile.sources.push(source.clone());
    profile.destination = Destination::Local { path: dir.path().join("backups") };
    profile.container = Container::Folder;
    profile.merge.rules = Retention::KEEP_ALL;
    for i in 0..4 {
        fs::write(source.join("a.txt"), format!("version {i}")).unwrap();
        backup(&profile, None, BackupMode::Auto, &Quiet).unwrap();
        // Backup names have one-second precision.
        sleep(Duration::from_millis(1100));
    }
    assert!(!merge_pending(&profile).unwrap(), "every backup is kept");

    let archives = archive_names(&profile).unwrap();
    assert_eq!(archives.len(), 4);
    let now = jiff::Zoned::now().datetime();
    profile.merge.set_rules(keep_last(2), Some(&archives), now);
    assert!(profile.merge.approved.is_some(), "tighter: to confirm");
    assert!(!merge_pending(&profile).unwrap(), "the automatic merge waits");

    profile.merge.confirm();
    assert!(merge_pending(&profile).unwrap(), "confirmed: work to do");
    merge(&profile, None, &Quiet).unwrap();
    // The initial complete backup and the last two stay.
    assert_eq!(archive_names(&profile).unwrap().len(), 3);
    assert!(!merge_pending(&profile).unwrap());
}
