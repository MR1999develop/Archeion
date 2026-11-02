//! The "Restore files" view driven like a user, without a window: ticks
//! in the tree, then the Restore button, checked on the boxes shown and on
//! the request the view hands to the worker.

use super::{Action, RestoreView};
use crate::i18n::Tr;
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
    profile: Profile,
    source: std::path::PathBuf,
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
    profile.destination = Destination::Local { path: dir.path().join("backups") };
    profile.container = Container::Folder;
    archeion_engine::backup(&profile, None, BackupMode::Auto, &Quiet).unwrap();
    Fixture { _dir: dir, profile, source }
}

type State = (RestoreView, Option<Action>);

/// The system fonts, as eframe adds them to the window (`system_fonts`).
fn system_fonts(ctx: &egui::Context) {
    ctx.add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
}

fn harness(f: &Fixture) -> Harness<'static, State> {
    let (tr, profile) = (Tr::new(Language::English), f.profile.clone());
    let builder = Harness::builder().with_size(egui::vec2(1200.0, 900.0)).with_context_setup(system_fonts);
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
    while h.query_by_label_contains("📄 a.txt").is_none() {
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

/// The tick box of the tree whose label contains `label` (the cards on
/// the right repeat names, not boxes).
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
    let root = format!("📁 {}", f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, "📄 a.txt"), Some(Toggled::True), "a file of the ticked folder");
    assert_eq!(toggled(&h, "📁 sub"), Some(Toggled::True), "a subfolder of the ticked folder");
    expand(&mut h, "📁 sub");
    assert_eq!(toggled(&h, "📄 b.txt"), Some(Toggled::True), "a file of the subfolder");

    tick_box(&h, "📄 b.txt").click();
    settle(&mut h);
    assert_eq!(toggled(&h, "📄 b.txt"), Some(Toggled::False));
    assert_ne!(toggled(&h, "📁 sub"), Some(Toggled::True), "the subfolder is partial");
    assert_ne!(toggled(&h, &root), Some(Toggled::True), "the ticked folder is partial");
    assert_eq!(toggled(&h, "📄 a.txt"), Some(Toggled::True), "the rest stays ticked");

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
    let root = format!("📁 {}", f.source.display());
    tick_box(&h, &root).click();
    settle(&mut h);
    tick_box(&h, "📄 a.txt").click();
    settle(&mut h);
    assert_ne!(toggled(&h, &root), Some(Toggled::True));
    tick_box(&h, &root).click();
    settle(&mut h);
    assert_eq!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, "📄 a.txt"), Some(Toggled::True));
    assert!(request(&mut h).items[0].exclude.is_empty());
}

#[test]
fn a_ticked_file_fills_the_folders_above_it() {
    let f = backed_up();
    let mut h = harness(&f);
    let root = format!("📁 {}", f.source.display());
    expand(&mut h, "📁 sub");
    tick_box(&h, "📄 b.txt").click();
    settle(&mut h);
    assert_eq!(toggled(&h, "📄 b.txt"), Some(Toggled::True));
    assert_ne!(toggled(&h, "📁 sub"), Some(Toggled::True));
    assert_ne!(toggled(&h, &root), Some(Toggled::True));
    assert_eq!(toggled(&h, "📄 a.txt"), Some(Toggled::False), "a file beside it stays unticked");
}

/// Every character the interface draws (symbols in the code, texts of
/// every language) has a glyph in the fonts eframe draws with: egui's own
/// and the system fonts.
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
    // The harness fails on the first character no font can draw.
    let mut h = Harness::builder().with_context_setup(system_fonts).build_ui(move |ui| {
        for l in &lines {
            ui.label(l.as_str());
        }
    });
    h.run();
}
