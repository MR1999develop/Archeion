//! Tests that drive the whole window, without one on screen: a sandbox
//! data folder, the profile settings, the confirmation dialogs.

use super::ArcheionApp;
use crate::storage::Storage;
use archeion_core::{BackupMode, Container, Destination, Language, Profile, RunEvent, Settings};
use archeion_engine::EventSink;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

/// A data folder with one profile and one backup of it.
fn sandbox() -> (tempfile::TempDir, Storage, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.txt"), "alpha").unwrap();
    let backups = dir.path().join("backups");
    let mut profile = Profile::new("Docs");
    profile.sources.push(source);
    profile.destination = Destination::Local { path: backups.clone() };
    profile.container = Container::Folder;
    archeion_engine::backup(&profile, None, BackupMode::Auto, &Quiet).unwrap();
    let storage = Storage::at(dir.path().join("data")).unwrap();
    let settings = Settings { language: Language::English, profiles: vec![profile], ..Settings::default() };
    storage.save(&settings).unwrap();
    (dir, storage, backups)
}

fn harness(storage: Storage) -> Harness<'static, ArcheionApp> {
    Harness::builder().with_size(egui::vec2(1200.0, 1000.0)).build_eframe(move |cc| {
        cc.egui_ctx
            .add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
        super::theme::apply(&cc.egui_ctx);
        ArcheionApp::state(storage)
    })
}

fn steps(h: &mut Harness<'_, ArcheionApp>) {
    for _ in 0..4 {
        h.step();
    }
}

/// The button labelled `label` drawn last: the one of an open dialog, in
/// front of the form's.
fn button<'h>(h: &'h Harness<'_, ArcheionApp>, label: &'h str) -> egui_kittest::Node<'h> {
    h.query_all_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(label))
        .last()
        .unwrap_or_else(|| panic!("no button {label}"))
}

fn checkbox<'h>(h: &'h Harness<'_, ArcheionApp>, starts: &'h str) -> egui_kittest::Node<'h> {
    h.query_all_by(move |n| n.role() == Role::CheckBox && n.label().is_some_and(|l| l.starts_with(starts)))
        .last()
        .unwrap_or_else(|| panic!("no checkbox {starts}"))
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

#[test]
fn deleting_a_profile_with_its_backups_moves_them_to_the_recycle_bin() {
    let (dir, storage, backups) = sandbox();
    let _clean = RecycledUnder(dir.path().to_path_buf());
    let mut h = harness(storage.clone());
    steps(&mut h);
    button(&h, "Delete profile").click();
    steps(&mut h);
    checkbox(&h, "Also move the backups in").click();
    steps(&mut h);
    assert!(h.query_by_label_contains("can be restored from the Recycle Bin").is_some(), "the warning");
    button(&h, "Delete profile, backups to Recycle Bin").click();

    let started = Instant::now();
    while backups.exists() {
        assert!(started.elapsed() < Duration::from_secs(20), "the backups were not moved");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    steps(&mut h);
    assert!(storage.load().unwrap().profiles.is_empty(), "the profile is gone");
    let parent = dir.path().to_string_lossy().to_lowercase();
    let in_bin = trash::os_limited::list().unwrap().into_iter().any(|i| {
        i.name.to_string_lossy() == "backups" && i.original_parent.to_string_lossy().to_lowercase() == parent
    });
    assert!(in_bin, "the folder is in the Recycle Bin");
    // The status bar says so once the move has ended.
    let started = Instant::now();
    let text = loop {
        if let Some(super::Wipe::Done { text, warn }) = &h.state().wipe {
            assert!(!warn, "{text}");
            break text.clone();
        }
        assert!(started.elapsed() < Duration::from_secs(30), "the move did not end");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    };
    assert!(text.contains("are in the Recycle Bin"), "{text}");
    assert!(h.query_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Undo")).is_none());
}

#[test]
fn deleting_a_profile_alone_keeps_its_backups_and_offers_undo() {
    let (_dir, storage, backups) = sandbox();
    let mut h = harness(storage.clone());
    steps(&mut h);
    button(&h, "Delete profile").click();
    steps(&mut h);
    button(&h, "Delete profile").click();
    steps(&mut h);
    assert!(storage.load().unwrap().profiles.is_empty());
    assert!(backups.join("archeion.json").exists(), "the backups stay");
    // The message is announced to screen readers.
    let message = h
        .query_all_by(|n| n.value().is_some_and(|v| v.contains("Docs") && v.contains("deleted")))
        .next()
        .expect("the status message");
    assert_eq!(message.accesskit_node().live(), egui::accesskit::Live::Polite);
    button(&h, "Undo").click();
    steps(&mut h);
    assert_eq!(storage.load().unwrap().profiles.len(), 1);
}

#[test]
fn a_paused_schedule_registers_no_backup_task() {
    let mut p = Profile::new("Imported");
    p.schedule =
        archeion_core::Schedule::Monthly { day: 3, at: archeion_core::TimeOfDay::new(21, 30).unwrap() };
    p.schedule_paused = true;
    let exe = std::path::Path::new(r"C:\Archeion\archeion.exe");
    let backup = super::task_specs(&p, exe, "user")
        .into_iter()
        .find(|s| s.kind == archeion_scheduler::TaskKind::Backup);
    assert_eq!(backup.unwrap().schedule, archeion_core::Schedule::Manual);
    p.schedule_paused = false;
    let backup = super::task_specs(&p, exe, "user")
        .into_iter()
        .find(|s| s.kind == archeion_scheduler::TaskKind::Backup);
    assert!(backup.unwrap().schedule.is_automatic());
}

#[test]
fn the_backups_of_a_deleted_profile_are_taken_over_without_blocking_the_window() {
    let (_dir, storage, backups) = sandbox();
    // The owner is gone; another profile points at its backups.
    let mut settings = storage.load().unwrap();
    let old = settings.profiles.remove(0);
    let mut heir = Profile::new("Heir");
    heir.sources = old.sources.clone();
    heir.destination = old.destination.clone();
    heir.container = Container::Folder;
    let heir_id = heir.id;
    settings.profiles.push(heir.clone());
    storage.save(&settings).unwrap();

    let mut h = harness(storage);
    let started = Instant::now();
    while h
        .query_all_by(|n| {
            n.role() == Role::Button && n.label().as_deref() == Some("Use these backups for this profile")
        })
        .next()
        .is_none()
    {
        assert!(started.elapsed() < Duration::from_secs(20), "no offer to take the backups over");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    button(&h, "Use these backups for this profile").click();
    let started = Instant::now();
    while archeion_engine::destination_owner(&heir).ok().flatten() != Some(heir_id) {
        assert!(started.elapsed() < Duration::from_secs(20), "not taken over");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    assert!(backups.exists());
}

#[test]
fn a_typed_exclusion_is_checked_against_the_button_pressed() {
    let (_dir, storage, _backups) = sandbox();
    let mut h = harness(storage.clone());
    steps(&mut h);
    h.query_all_by(|n| n.label().is_some_and(|l| l == "Exclusions")).next().expect("the section").click();
    steps(&mut h);
    let field = "**/node_modules · .iso";
    let typed = |h: &mut Harness<'_, ArcheionApp>, text: &str, button_label: &str| {
        let f = h.query_all_by(|n| n.label().as_deref() == Some(field)).next().expect("the field");
        f.click();
        h.step();
        // What an error left there is replaced.
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.step();
        h.query_all_by(|n| n.label().as_deref() == Some(field)).next().unwrap().type_text(text);
        steps(h);
        button(h, button_label).click();
        steps(h);
    };
    let rules = |s: &Storage| {
        s.load().unwrap().profiles[0].excludes.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    let before = rules(&storage);

    typed(&mut h, ".iso", "Exclude pattern");
    assert!(h.query_by_label_contains("is a file type: press").is_some(), "told to use the other button");
    assert_eq!(rules(&storage), before, "nothing added");

    button(&h, "Exclude type").click();
    steps(&mut h);
    assert!(rules(&storage).contains(&".iso".to_owned()), "{:?}", rules(&storage));

    typed(&mut h, "node_modules", "Exclude pattern");
    let kept = h.query_all_by(|n| n.label().as_deref() == Some(field)).next().unwrap();
    assert_eq!(kept.value().as_deref(), Some("node_modules"), "kept to be corrected");
    assert!(h.query_by_label_contains("type **/node_modules").is_some(), "the pattern meant is suggested");
    typed(&mut h, "**/cache", "Exclude type");
    assert!(h.query_by_label_contains("is a pattern: press").is_some());
    button(&h, "Exclude pattern").click();
    steps(&mut h);
    assert!(rules(&storage).contains(&"**/cache".to_owned()), "{:?}", rules(&storage));
}

#[test]
fn network_folders_of_an_imported_profile_wait_for_the_user() {
    let (_dir, storage, _backups) = sandbox();
    let mut settings = storage.load().unwrap();
    let p = &mut settings.profiles[0];
    p.sources.push(PathBuf::from(format!("{0}{0}unknown-host.invalid{0}share", std::path::MAIN_SEPARATOR)));
    p.network_unconfirmed = true;
    storage.save(&settings).unwrap();

    let mut h = harness(storage.clone());
    steps(&mut h);
    let warning = "other computers (unknown-host.invalid)";
    assert!(h.query_all_by_label_contains(warning).next().is_some(), "the computer is named");
    assert!(h.state().format.free.is_none(), "the destination was not looked at");
    button(&h, "Use these network folders").click();
    steps(&mut h);
    assert!(!storage.load().unwrap().profiles[0].network_unconfirmed);
    assert!(h.query_all_by_label_contains(warning).next().is_none(), "the warning is gone");
}
