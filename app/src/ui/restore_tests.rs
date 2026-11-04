//! The "Restore files" view driven like a user, without a window: ticks
//! in the tree, then the Restore button, checked on the boxes shown and on
//! the request the view hands to the worker.

use super::{Action, RestoreView};
use crate::i18n::Tr;
use crate::ui::symbol;
use archeion_core::{BackupMode, Container, Destination, Language, Profile, RunEvent, keys};
use archeion_engine::EventSink;
use eframe::egui::{self, accesskit::Toggled};
use egui_kittest::{
    Harness,
    kittest::{By, NodeT as _, Queryable as _},
};
use std::{
    fs,
    time::{Duration, Instant},
};

struct Quiet;

impl EventSink for Quiet {
    fn emit(&self, _: RunEvent) {}
}

struct Fixture {
    _dir: tempfile::TempDir,
    /// The backups' folder when it is elsewhere (`ARCHEION_TEST_DEST`).
    _elsewhere: Option<tempfile::TempDir>,
    profile: Profile,
    source: std::path::PathBuf,
}

/// Where a test keeps its backups: `local`, or a new folder below
/// `ARCHEION_TEST_DEST` when it is set, to run the tests against another
/// drive or a network share. The guard removes that folder at the end.
fn backups_folder(local: std::path::PathBuf) -> (std::path::PathBuf, Option<tempfile::TempDir>) {
    match std::env::var_os("ARCHEION_TEST_DEST").filter(|d| !d.is_empty()) {
        Some(root) => {
            let dir = tempfile::Builder::new()
                .prefix("archeion-test-")
                .tempdir_in(&root)
                .unwrap_or_else(|e| panic!("a folder in ARCHEION_TEST_DEST {}: {e}", root.to_string_lossy()));
            (dir.path().join("backups"), Some(dir))
        }
        None => (local, None),
    }
}

/// One backup of `source/{a.txt, sub/b.txt, sub/deep/c.txt}`.
fn backed_up() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    fs::create_dir_all(source.join("sub").join("deep")).unwrap();
    fs::write(source.join("a.txt"), "alpha").unwrap();
    fs::write(source.join("sub").join("b.txt"), "beta").unwrap();
    fs::write(source.join("sub").join("deep").join("c.txt"), "gamma").unwrap();
    let mut profile = Profile::new("UI");
    profile.sources.push(source.clone());
    let (backups, elsewhere) = backups_folder(dir.path().join("backups"));
    profile.destination = Destination::Local { path: backups };
    profile.container = Container::Folder;
    archeion_engine::backup(&profile, None, BackupMode::Auto, &Quiet).unwrap();
    Fixture { _dir: dir, _elsewhere: elsewhere, profile, source }
}

type State = (RestoreView, Option<Action>);

/// The fonts of the window: the system fonts, as eframe adds them
/// (`system_fonts`), and the theme's interface and icon fonts.
fn fonts(ctx: &egui::Context) {
    ctx.add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
    crate::ui::theme::apply(ctx);
}

fn harness(f: &Fixture) -> Harness<'static, State> {
    let (tr, profile) = (Tr::new(Language::English), f.profile.clone());
    let builder = Harness::builder().with_size(egui::vec2(1200.0, 900.0)).with_context_setup(fonts);
    let mut h = builder.build_ui_state(
        move |ui, state: &mut State| {
            if let Some(a) = state.0.show(ui, &tr, &profile, false) {
                state.1 = Some(a);
            }
        },
        (RestoreView::default(), None),
    );
    // The tree is read on a background thread.
    let started = Instant::now();
    while h.query_all_by_label_contains(&file("a.txt")).next().is_none() {
        assert!(started.elapsed() < Duration::from_secs(10), "the tree was not read");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    h
}

/// A few frames: enough for a click, while the folder size may still be
/// read (its spinner keeps asking for frames).
fn settle(h: &mut Harness<'_, State>) {
    h.run_steps(4);
}

/// The label of a file or folder row of the tree.
fn file(name: &str) -> String {
    symbol::with(symbol::FILE, name)
}

fn folder(name: &str) -> String {
    symbol::with(symbol::FOLDER, name)
}

fn tick_box<'h>(h: &'h Harness<'_, State>, label: &'h str) -> egui_kittest::Node<'h> {
    h.get(By::new().role(egui::accesskit::Role::CheckBox).label_contains(label))
}

fn toggled(h: &Harness<'_, State>, label: &str) -> Option<Toggled> {
    tick_box(h, label).accesskit_node().toggled()
}

/// Opens a collapsed folder of the tree.
fn expand(h: &mut Harness<'_, State>, label: &str) {
    let row = tick_box(h, label).rect();
    // The arrow sits left of the box.
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: egui::pos2(row.left() - 10.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: egui::pos2(row.left() - 10.0, row.center().y),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(h);
}

fn request(h: &mut Harness<'_, State>) -> archeion_engine::RestoreRequest {
    // A frame first: a click counts on the button as last drawn, which may
    // have been disabled until the versions were read.
    settle(h);
    h.get_by_label("Restore").click();
    settle(h);
    match h.state_mut().1.take() {
        Some(Action::Restore(r)) => r,
        other => panic!("no restore started: {other:?}"),
    }
}

#[test]
fn ticking_a_folder_ticks_its_content_and_unticking_inside_leaves_it_out() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True), "a file of the ticked folder");
    assert_eq!(toggled(&h, &folder("sub")), Some(Toggled::True), "a subfolder of the ticked folder");
    expand(&mut h, &folder("sub"));
    assert_eq!(toggled(&h, &file("b.txt")), Some(Toggled::True), "a file of the subfolder");

    tick_box(&h, &file("b.txt")).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("b.txt")), Some(Toggled::False));
    assert_ne!(toggled(&h, &folder("sub")), Some(Toggled::True), "the subfolder is partial");
    assert_ne!(toggled(&h, &root), Some(Toggled::True), "the ticked folder is partial");
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True), "the rest stays ticked");

    let req = request(&mut h);
    assert_eq!(req.items.len(), 1);
    let source_key = keys::key_for(&f.source).unwrap();
    assert_eq!(req.items[0].key.as_deref(), Some(source_key.as_str()));
    assert_eq!(req.items[0].exclude, vec![format!("{source_key}/sub/b.txt")]);
}

#[test]
fn clicking_the_filled_box_ticks_the_whole_folder_again() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    tick_box(&h, &file("a.txt")).click();
    settle(&mut h);
    assert_ne!(toggled(&h, &root), Some(Toggled::True));
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True));
    assert!(request(&mut h).items[0].exclude.is_empty());
}

#[test]
fn a_ticked_file_fills_the_folders_above_it() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    expand(&mut h, &folder("sub"));
    tick_box(&h, &file("b.txt")).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("b.txt")), Some(Toggled::True));
    assert_ne!(toggled(&h, &folder("sub")), Some(Toggled::True));
    assert_ne!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False), "a file beside it stays unticked");
}

#[test]
fn a_folder_whose_items_are_all_ticked_one_by_one_shows_ticked() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    expand(&mut h, &folder("sub"));
    expand(&mut h, &folder("deep"));
    for item in [file("a.txt"), file("b.txt"), file("c.txt")] {
        assert_ne!(toggled(&h, &root), Some(Toggled::True), "not everything is ticked yet");
        tick_box(&h, &item).click();
        settle(&mut h);
    }
    assert_eq!(toggled(&h, &folder("deep")), Some(Toggled::True), "its only file is ticked");
    assert_eq!(toggled(&h, &folder("sub")), Some(Toggled::True));
    assert_eq!(toggled(&h, &root), Some(Toggled::True), "every item inside is ticked");

    // Clicking it unticks everything inside.
    tick_box(&h, &root).click();
    settle(&mut h);
    for item in [root.clone(), folder("sub"), file("a.txt"), file("b.txt"), file("c.txt")] {
        assert_eq!(toggled(&h, &item), Some(Toggled::False), "{item}");
    }
}

#[test]
fn clear_selection_beside_restore_unticks_everything() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &file("a.txt")).click();
    tick_box(&h, &folder("sub")).click();
    settle(&mut h);
    assert_ne!(toggled(&h, &root), Some(Toggled::False));
    h.get_by(|n| {
        n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some("Clear selection")
    })
    .click();
    settle(&mut h);
    for item in [root, folder("sub"), file("a.txt")] {
        assert_eq!(toggled(&h, &item), Some(Toggled::False), "{item}");
    }
}

/// The name of a file or folder row, beside its box.
fn name<'h>(h: &'h Harness<'_, State>, label: &'h str) -> Option<egui_kittest::Node<'h>> {
    h.query_by(move |n| {
        n.role() != egui::accesskit::Role::CheckBox && n.label().is_some_and(|l| l.contains(label))
    })
}

fn click_at(h: &mut Harness<'_, State>, pos: egui::Pos2) {
    for pressed in [true, false] {
        h.input_mut().events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    settle(h);
}

/// The versions shown on the right, newest first.
fn versions<'h>(h: &'h Harness<'_, State>) -> Vec<egui_kittest::Node<'h>> {
    h.query_all_by(|n| {
        n.role() == egui::accesskit::Role::RadioButton
            && n.label().is_some_and(|l| l.starts_with("Backup of"))
    })
    .collect()
}

fn wait_for_versions(h: &mut Harness<'_, State>, count: usize) {
    let started = Instant::now();
    while versions(h).len() < count {
        assert!(started.elapsed() < Duration::from_secs(10), "the versions were not read");
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
}

#[test]
fn a_click_on_a_name_ticks_nothing_and_a_file_shows_its_versions() {
    let f = backed_up();
    let mut h = harness(&f);
    // A folder's name is plain text: a click right of its box.
    let box_rect = tick_box(&h, &folder("sub")).rect();
    click_at(&mut h, egui::pos2(box_rect.right() + 30.0, box_rect.center().y));
    assert_eq!(toggled(&h, &folder("sub")), Some(Toggled::False), "a folder's name does not tick");
    assert!(versions(&h).is_empty(), "nor shows versions");

    name(&h, &file("a.txt")).expect("the file's name").click();
    settle(&mut h);
    wait_for_versions(&mut h, 1);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False), "a file's name does not tick");
    assert_eq!(versions(&h)[0].accesskit_node().toggled(), Some(Toggled::True), "the newest is the choice");
}

#[test]
fn the_version_chosen_is_restored_and_the_newest_without_a_choice() {
    let f = backed_up();
    std::thread::sleep(Duration::from_millis(1100));
    fs::write(f.source.join("a.txt"), "alpha, changed").unwrap();
    archeion_engine::backup(&f.profile, None, BackupMode::Auto, &Quiet).unwrap();
    let list =
        archeion_engine::file_versions(&f.profile, None, &keys::key_for(&f.source.join("a.txt")).unwrap())
            .unwrap();
    let (newest, oldest) = (list[0].archive.clone(), list[1].archive.clone());

    // Without a choice: the newest.
    let mut h = harness(&f);
    tick_box(&h, &file("a.txt")).click();
    settle(&mut h);
    let started = Instant::now();
    while h.state().0.picks.iter().any(|p| matches!(p, super::Pick::File { loading: true, .. })) {
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
        h.step();
    }
    assert_eq!(request(&mut h).items[0].archive, newest);

    // The older one, chosen before ticking.
    let mut h = harness(&f);
    name(&h, &file("a.txt")).unwrap().click();
    settle(&mut h);
    wait_for_versions(&mut h, 2);
    versions(&h)[1].click();
    settle(&mut h);
    tick_box(&h, &file("a.txt")).click();
    settle(&mut h);
    assert_eq!(request(&mut h).items[0].archive, oldest, "chosen first, ticked after");

    // Chosen after ticking: the ticked file takes it.
    let mut h = harness(&f);
    tick_box(&h, &file("a.txt")).click();
    name(&h, &file("a.txt")).unwrap().click();
    settle(&mut h);
    wait_for_versions(&mut h, 2);
    versions(&h)[1].click();
    settle(&mut h);
    assert_eq!(request(&mut h).items[0].archive, oldest, "ticked first, chosen after");
}

fn shown(h: &Harness<'_, State>) -> Option<String> {
    h.state()
        .0
        .shown
        .as_deref()
        .map(|k| keys::path_for(k).file_name().unwrap().to_string_lossy().into_owned())
}

#[test]
fn the_arrow_keys_move_between_the_files_shown() {
    let f = backed_up();
    let mut h = harness(&f);
    expand(&mut h, &folder("sub"));
    // Files as drawn: sub/b.txt, then a.txt (folders first).
    name(&h, &file("a.txt")).unwrap().click();
    settle(&mut h);
    assert_eq!(shown(&h).as_deref(), Some("a.txt"));
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert_eq!(shown(&h).as_deref(), Some("b.txt"), "up: the file above");
    assert_eq!(
        name(&h, &file("b.txt")).unwrap().accesskit_node().toggled(),
        Some(Toggled::True),
        "it stands out"
    );
    assert_eq!(name(&h, &file("a.txt")).unwrap().accesskit_node().toggled(), Some(Toggled::False));
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert_eq!(shown(&h).as_deref(), Some("b.txt"), "the first stays");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(shown(&h).as_deref(), Some("a.txt"), "down: the file below");
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False), "moving ticks nothing");
}

#[test]
fn a_file_inside_a_ticked_folder_can_come_back_in_another_version() {
    let f = backed_up();
    std::thread::sleep(Duration::from_millis(1100));
    fs::write(f.source.join("a.txt"), "alpha, changed").unwrap();
    archeion_engine::backup(&f.profile, None, BackupMode::Auto, &Quiet).unwrap();
    let a = keys::key_for(&f.source.join("a.txt")).unwrap();
    let list = archeion_engine::file_versions(&f.profile, None, &a).unwrap();
    let oldest = list[1].archive.clone();

    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    name(&h, &file("a.txt")).unwrap().click();
    settle(&mut h);
    wait_for_versions(&mut h, 2);
    assert_eq!(
        versions(&h)[0].accesskit_node().toggled(),
        Some(Toggled::True),
        "the folder's date: the newest"
    );
    versions(&h)[1].click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True), "still ticked");
    assert_eq!(toggled(&h, &root), Some(Toggled::True), "the folder is still whole");
    assert_eq!(
        versions(&h)[1].accesskit_node().toggled(),
        Some(Toggled::True),
        "the older one is the choice"
    );

    let req = request(&mut h);
    assert_eq!(req.items.len(), 2, "{req:?}");
    let folder_item = req
        .items
        .iter()
        .find(|i| i.key.as_deref() == Some(keys::key_for(&f.source).unwrap().as_str()))
        .unwrap();
    assert_eq!(folder_item.exclude, vec![a.clone()], "the file leaves the folder");
    let file_item = req.items.iter().find(|i| i.key.as_deref() == Some(a.as_str())).unwrap();
    assert_eq!(file_item.archive, oldest, "and comes back in the version chosen");
}

#[test]
fn a_ticked_folder_ticks_its_deleted_files_too() {
    let f = backed_up();
    std::thread::sleep(Duration::from_millis(1100));
    fs::remove_file(f.source.join("a.txt")).unwrap();
    archeion_engine::backup(&f.profile, None, BackupMode::Auto, &Quiet).unwrap();
    let a = keys::key_for(&f.source.join("a.txt")).unwrap();
    let root_key = keys::key_for(&f.source).unwrap();

    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True), "deleted, it comes back with the folder");
    let req = request(&mut h);
    assert_eq!(req.items.len(), 1);
    assert!(req.items[0].with_deleted && req.items[0].exclude.is_empty(), "{:?}", req.items[0]);

    // Unticked inside: left out of the folder.
    let mut h = harness(&f);
    tick_box(&h, &root).click();
    settle(&mut h);
    tick_box(&h, &file("a.txt")).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False));
    let req = request(&mut h);
    assert_eq!(req.items[0].key.as_deref(), Some(root_key.as_str()));
    assert_eq!(req.items[0].exclude, vec![a]);
}

#[test]
fn the_tick_box_above_the_tree_leaves_deleted_files_out_of_folders() {
    let f = backed_up();
    std::thread::sleep(Duration::from_millis(1100));
    fs::remove_file(f.source.join("a.txt")).unwrap();
    archeion_engine::backup(&f.profile, None, BackupMode::Auto, &Quiet).unwrap();
    let include = "Include deleted files in ticked folders";

    let mut h = harness(&f);
    let root = symbol::with(symbol::FOLDER, f.source.display());
    assert_eq!(tick_box(&h, include).accesskit_node().toggled(), Some(Toggled::True), "on to begin with");
    tick_box(&h, include).click();
    settle(&mut h);
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False), "off: the deleted file stays out");
    let req = request(&mut h);
    assert!(!req.items[0].with_deleted, "{:?}", req.items[0]);

    // On again, with the folder ticked: in at once.
    let mut h = harness(&f);
    tick_box(&h, &root).click();
    settle(&mut h);
    tick_box(&h, include).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False));
    tick_box(&h, include).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::True), "on: with the folder");
    assert!(request(&mut h).items[0].with_deleted);
}

#[test]
fn a_file_excluded_after_its_backups_stays_out_of_its_folder_and_can_be_ticked_alone() {
    let f = backed_up();
    std::thread::sleep(Duration::from_millis(1100));
    let mut profile = f.profile.clone();
    profile.excludes.push(archeion_core::Exclusion::File { path: f.source.join("a.txt") });
    archeion_engine::backup(&profile, None, BackupMode::Auto, &Quiet).unwrap();

    let mut h = harness(&f);
    let excluded = format!("{}  excluded", symbol::EXCLUDED);
    assert!(h.query_all_by_label_contains(&excluded).next().is_some(), "shown as excluded, not deleted");
    let deleted = format!("{}  deleted", symbol::DELETED);
    assert!(h.query_all_by_label_contains(&deleted).next().is_none(), "not as deleted");
    let root = symbol::with(symbol::FOLDER, f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &file("a.txt")), Some(Toggled::False), "the folder leaves it out");
    assert_eq!(toggled(&h, &symbol::with(symbol::FOLDER, "sub")), Some(Toggled::True), "the rest comes back");
    tick_box(&h, &file("a.txt")).click();
    settle(&mut h);
    let req = request(&mut h);
    let key = keys::key_for(&f.source.join("a.txt")).unwrap();
    assert!(req.items.iter().any(|i| i.key.as_deref() == Some(key.as_str())), "{:?}", req.items);
}

/// Every character the interface draws (symbols in the code, texts of
/// every language, the icons) has a glyph in the fonts the window draws
/// with: the theme's, egui's own and the system fonts.
#[test]
fn every_character_of_the_interface_can_be_drawn() {
    // Lines as written (whole sentences: fonts are chosen per run of text).
    let mut lines: Vec<String> = Vec::new();
    for dir in [
        "src",
        "src/ui",
        "locales/en-US",
        "locales/es-ES",
        "locales/fr-FR",
        "locales/hi-IN",
        "locales/it-IT",
        "locales/ja-JP",
        "locales/pt-BR",
        "locales/ru-RU",
        "locales/zh-CN",
    ] {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "rs" || x == "ftl") {
                let text = fs::read_to_string(&path).unwrap();
                let wide = text.lines().filter(|l| !l.is_ascii()).map(|l| l.trim().to_owned());
                lines.extend(wide);
            }
        }
    }
    lines.extend(symbol::ALL.iter().map(|s| (*s).to_owned()));
    // The harness fails on the first character no font can draw.
    let mut h = Harness::builder().with_context_setup(fonts).build_ui(move |ui| {
        for l in &lines {
            ui.label(l.as_str());
        }
    });
    h.run();
}
