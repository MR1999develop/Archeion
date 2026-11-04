//! Screenshots of every screen, for looking at the interface without
//! opening it: `cargo test -p archeion --bin archeion screens -- --ignored`
//! writes PNG files to `target/ui-screens`, in Italian and English, light
//! and dark. Ignored by default: it needs a GPU (or the WARP adapter) and
//! checks nothing.

use super::{ArcheionApp, Tab};
use crate::{storage::Storage, worker};
use archeion_core::{BackupMode, Container, Destination, Language, Profile, Settings};
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable as _};
use std::{fs, path::Path, time::Duration};

/// A data folder with two profiles; the first has two backups and their
/// logs, made by the same code as the Run button.
fn sandbox(root: &Path, language: Language) -> Storage {
    let source = root.join("Documenti");
    fs::create_dir_all(source.join("Lavoro").join("2026")).unwrap();
    fs::create_dir_all(source.join("Foto")).unwrap();
    fs::write(source.join("Lavoro").join("relazione.docx"), "v1").unwrap();
    fs::write(source.join("Lavoro").join("2026").join("bilancio.xlsx"), "x".repeat(40_000)).unwrap();
    fs::write(source.join("Foto").join("mare.jpg"), "y".repeat(900_000)).unwrap();
    fs::write(source.join("note.txt"), "note").unwrap();

    let mut docs = Profile::new("Documenti");
    docs.sources.push(source.clone());
    docs.destination = Destination::Local { path: root.join("Backup") };
    docs.container = Container::Folder;
    let mut photos = Profile::new("Foto di famiglia");
    photos.sources.push(root.join("Immagini"));
    photos.destination = Destination::Local { path: root.join("Disco esterno") };
    photos.encrypted = true;

    let storage = Storage::at(root.join("data")).unwrap();
    let settings = Settings { language, profiles: vec![docs.clone(), photos], ..Settings::default() };
    storage.save(&settings).unwrap();
    let backup = || worker::Work::Backup { mode: BackupMode::Auto };
    let _ = worker::run(&storage, docs.id, backup(), false, None);
    std::thread::sleep(Duration::from_millis(1100));
    fs::write(source.join("Lavoro").join("relazione.docx"), "v2").unwrap();
    let _ = worker::run(&storage, docs.id, backup(), false, None);
    storage
}

fn harness(storage: Storage, theme: egui::Theme) -> Harness<'static, ArcheionApp> {
    Harness::builder()
        .with_size(egui::Vec2::from(super::super::DEFAULT_SIZE))
        .with_theme(theme)
        .wgpu()
        .build_eframe(move |cc| {
            super::theme::apply(&cc.egui_ctx);
            cc.egui_ctx
                .add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
            ArcheionApp::state(storage)
        })
}

/// A few frames, with time for background reads (tree, sizes, logs).
fn settle(h: &mut Harness<'_, ArcheionApp>) {
    for _ in 0..30 {
        h.step();
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn shot(h: &mut Harness<'_, ArcheionApp>, dir: &Path, name: &str) {
    settle(h);
    let image = h.render().unwrap();
    image.save(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
#[ignore = "writes screenshots to target/ui-screens; run on demand"]
fn screens() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("target").join("ui-screens");
    fs::create_dir_all(&out).unwrap();
    for (language, lang) in [(Language::Italian, "it"), (Language::English, "en")] {
        for (theme, mode) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let dir = tempfile::tempdir().unwrap();
            let storage = sandbox(dir.path(), language);
            let mut h = harness(storage, theme);
            let name = |screen: &str| format!("{lang}-{mode}-{screen}");

            shot(&mut h, &out, &name("1-profile"));
            h.state_mut().tab = Tab::Backups;
            shot(&mut h, &out, &name("2-restore"));
            if let Some(n) =
                h.query_all_by_label_contains(&super::symbol::with(super::symbol::FOLDER, "Lavoro")).next()
            {
                n.click();
            }
            shot(&mut h, &out, &name("3-restore-ticked"));
            // The versions of a file, on the right, after a click on its name.
            let note = super::symbol::with(super::symbol::FILE, "note.txt");
            if let Some(n) = h
                .query_all_by(|n| {
                    n.role() == egui::accesskit::Role::ListItem
                        && n.label().is_some_and(|l| l.contains(&note))
                })
                .next()
            {
                n.click();
            }
            shot(&mut h, &out, &name("3b-restore-versions"));
            // The pointer elsewhere, after an arrow key: the file shown
            // must still stand out.
            h.input_mut().events.push(egui::Event::PointerMoved(egui::pos2(1000.0, 700.0)));
            h.key_press(egui::Key::ArrowDown);
            shot(&mut h, &out, &name("3c-restore-shown"));
            h.state_mut().restore.open_manage();
            shot(&mut h, &out, &name("4-manage"));
            h.state_mut().tab = Tab::Activity;
            shot(&mut h, &out, &name("5-activity"));
            h.state_mut().tab = Tab::Settings;
            let ctx = h.ctx.clone();
            h.state_mut().preferences.open(&ctx);
            shot(&mut h, &out, &name("6-preferences"));
            h.state_mut().preferences = super::Preferences::default();
            h.state_mut().help.open_docs();
            shot(&mut h, &out, &name("7-guide"));
            h.state_mut().help = super::Help::default();
            h.state_mut().help.open_about();
            shot(&mut h, &out, &name("8-about"));
            h.state_mut().help = super::Help::default();
            h.state_mut().selected = h.state().settings.profiles.get(1).map(|p| p.id);
            shot(&mut h, &out, &name("9-profile-encrypted"));
            // The deletion warning, with the backups ticked.
            h.state_mut().selected = h.state().settings.profiles.first().map(|p| p.id);
            settle(&mut h);
            let delete = h.state().tr.t("delete-profile");
            h.query_all_by(|n| {
                n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some(delete.as_str())
            })
            .last()
            .unwrap()
            .click();
            settle(&mut h);
            // The text up to the folder, which sits inside the sentence.
            let marker = "\u{1}";
            let with =
                h.state().tr.t_args("delete-profile-with-backups", &[("path", crate::i18n::arg(marker))]);
            let with = with.split(marker).next().unwrap_or_default().trim().to_owned();
            if let Some(n) =
                h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with(with.as_str()))).last()
            {
                n.click();
            }
            shot(&mut h, &out, &name("10-delete"));
            // The merge section, open: its rules and the deleted files tick.
            let cancel = h.state().tr.t("cancel");
            if let Some(n) = h.query_all_by(|n| n.label().as_deref() == Some(cancel.as_str())).last() {
                n.click();
            }
            settle(&mut h);
            let merge = h.state().tr.t("merge");
            if let Some(n) = h.query_all_by(|n| n.label().as_deref() == Some(merge.as_str())).last() {
                n.click();
            }
            shot(&mut h, &out, &name("11-merge-settings"));
            // The exclusions, open, with a typed rule refused.
            if let Some(n) = h.query_all_by(|n| n.label().as_deref() == Some(merge.as_str())).last() {
                n.click();
            }
            let exclusions = h.state().tr.t("exclusions");
            if let Some(n) = h.query_all_by(|n| n.label().as_deref() == Some(exclusions.as_str())).last() {
                n.click();
            }
            settle(&mut h);
            h.state_mut().drafts.rule = "node_modules".into();
            let pattern = h.state().tr.t("exclude-pattern");
            if let Some(n) = h.query_all_by(|n| n.label().as_deref() == Some(pattern.as_str())).last() {
                n.click();
            }
            shot(&mut h, &out, &name("12-exclusions"));
        }
    }
}
