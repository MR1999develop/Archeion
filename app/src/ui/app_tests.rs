//! Tests that drive the whole window, without one on screen: a sandbox
//! data folder, the profile settings, the confirmation dialogs.

use super::ArcheionApp;
use crate::storage::Storage;
use archeion_core::{BackupMode, Container, Destination, Language, Profile, RunEvent, Settings};
use archeion_engine::EventSink;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};
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
    button(&h, "Undo").click();
    steps(&mut h);
    assert_eq!(storage.load().unwrap().profiles.len(), 1);
}
