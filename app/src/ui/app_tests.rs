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

#[test]
fn deleting_a_profile_with_its_backups_asks_twice_then_deletes_the_folder() {
    let (_dir, storage, backups) = sandbox();
    let mut h = harness(storage.clone());
    steps(&mut h);
    button(&h, "Delete profile").click();
    steps(&mut h);
    assert!(!button(&h, "Delete profile").accesskit_node().is_disabled(), "without the backups: one click");

    checkbox(&h, "Also delete the backups in").click();
    steps(&mut h);
    assert!(
        button(&h, "Delete profile and backups").accesskit_node().is_disabled(),
        "waits for \"I understand\""
    );
    checkbox(&h, "I understand").click();
    steps(&mut h);
    button(&h, "Delete profile and backups").click();

    let started = Instant::now();
    while backups.exists() {
        assert!(started.elapsed() < Duration::from_secs(10), "the backups were not deleted");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    steps(&mut h);
    assert!(storage.load().unwrap().profiles.is_empty(), "the profile is gone");
    assert!(h.query_by_label_contains("are deleted").is_some(), "the status bar says so");
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
