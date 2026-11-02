//! The Help menu: the user guide and the About window.
//!
//! The guide is a resizable window that stays open beside the main window
//! while the user tries what it describes; About is a small modal.

use super::{shell, symbol, theme, widgets};
use crate::{
    i18n::{Tr, arg},
    storage::Storage,
};
use eframe::egui::{self, Id, Key, KeyboardShortcut, LayerId, Modifiers, Order, RichText};
use std::path::Path;

use Block::{Cli, Text};

/// Opens the guide.
pub const DOCUMENTATION: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F1);
const REPOSITORY: &str = "https://github.com/MR1999develop/archeion";
const DOCS_ID: &str = "help-docs";
const ABOUT_ID: &str = "help-about";
const ABOUT_WIDTH: f32 = 420.0;

/// One piece of a guide section.
enum Block {
    /// A paragraph: its text key.
    Text(&'static str),
    /// A command line, shown as typed, and the key of its description.
    Cli(&'static str, &'static str),
}

/// A collapsible section of the guide: title key and content.
struct Section {
    title: &'static str,
    blocks: &'static [Block],
}

const SECTIONS: &[Section] = &[
    Section {
        title: "doc-what-title",
        blocks: &[Text("doc-what-backups"), Text("doc-what-storage"), Text("doc-what-safe")],
    },
    Section {
        title: "doc-profiles-title",
        blocks: &[
            Text("doc-profiles-body"),
            Text("doc-profiles-exclusions"),
            Text("doc-profiles-format"),
            Text("doc-profiles-delete"),
        ],
    },
    Section {
        title: "doc-auto-title",
        blocks: &[
            Text("doc-auto-schedule"),
            Text("doc-auto-missed"),
            Text("doc-auto-alerts"),
            Text("doc-auto-now"),
        ],
    },
    Section {
        title: "doc-merge-title",
        blocks: &[
            Text("doc-merge-body"),
            Text("doc-merge-rules"),
            Text("doc-merge-first"),
            Text("doc-merge-manual"),
            Text("doc-merge-delete"),
        ],
    },
    Section {
        title: "doc-encrypt-title",
        blocks: &[Text("doc-encrypt-body"), Text("doc-encrypt-password"), Text("doc-encrypt-recovery")],
    },
    Section {
        title: "doc-large-title",
        blocks: &[Text("doc-large-body"), Text("doc-large-complete"), Text("doc-large-upgrade")],
    },
    Section {
        title: "doc-restore-title",
        blocks: &[Text("doc-restore-body"), Text("doc-restore-versions"), Text("doc-restore-where")],
    },
    Section { title: "doc-verify-title", blocks: &[Text("doc-verify-body")] },
    Section { title: "doc-activity-title", blocks: &[Text("doc-activity-body")] },
    Section { title: "doc-window-title", blocks: &[Text("doc-window-body")] },
    Section {
        title: "doc-cli-title",
        blocks: &[
            Text("doc-cli-body"),
            Cli("archeion run <profile id>", "doc-cli-run"),
            Cli("archeion merge <profile id>", "doc-cli-merge"),
            Cli("archeion verify <profile id>", "doc-cli-verify"),
            Cli("archeion delete <profile id> --archive <name>", "doc-cli-delete"),
            Cli("archeion restore <profile id> --request <file>", "doc-cli-restore"),
            Cli("archeion watch", "doc-cli-watch"),
            Cli("archeion extract --destination <folder> --to <folder>", "doc-cli-extract"),
            Text("doc-cli-extract-options"),
            Text("doc-cli-exit"),
        ],
    },
    Section { title: "doc-data-title", blocks: &[Text("doc-data-body"), Text("doc-data-backups")] },
];

/// Which Help windows are open.
#[derive(Debug, Default)]
pub struct Help {
    docs_open: bool,
    about_open: bool,
}

impl Help {
    /// Opens the guide (F1, Help > Documentation).
    pub fn open_docs(&mut self) {
        self.docs_open = true;
    }

    /// Opens the About window (tests).
    #[cfg(test)]
    pub fn open_about(&mut self) {
        self.about_open = true;
    }

    /// The entries of the Help menu.
    pub fn menu(&mut self, ui: &mut egui::Ui, tr: &Tr) {
        let docs = egui::Button::new(tr.t("menu-documentation")).shortcut_text("F1");
        if ui.add(docs).clicked() {
            self.open_docs();
        }
        ui.separator();
        if ui.button(tr.t("menu-about")).clicked() {
            self.about_open = true;
        }
    }

    /// Draws the open windows; call once per frame, after the panels.
    pub fn show(&mut self, ctx: &egui::Context, tr: &Tr, storage: &Storage) {
        if self.about_open {
            self.about_open = about(ctx, tr, storage);
        }
        if self.docs_open {
            self.docs(ctx, tr);
        }
    }

    fn docs(&mut self, ctx: &egui::Context, tr: &Tr) {
        let id = Id::unique(DOCS_ID);
        // Esc closes the guide when it is the front window and nothing
        // above it (About, a menu) takes the key first.
        let in_front = ctx.top_layer_id() == Some(LayerId::new(Order::Middle, id));
        if in_front
            && !self.about_open
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.docs_open = false;
            return;
        }
        widgets::dialog(ctx, tr.t("doc-title"))
            .id(id)
            .open(&mut self.docs_open)
            .resizable(true)
            .vscroll(true)
            .default_size([560.0, 480.0])
            .min_width(320.0)
            .show(ctx, |ui| {
                ui.set_max_width(theme::FORM_MAX_WIDTH);
                ui.label(tr.t("doc-intro"));
                ui.add_space(theme::SPACE_S);
                for (n, section) in SECTIONS.iter().enumerate() {
                    egui::CollapsingHeader::new(widgets::section_text(tr.t(section.title)))
                        .id_salt(section.title)
                        .default_open(n == 0)
                        .show(ui, |ui| section_body(ui, tr, section));
                }
            });
    }
}

fn section_body(ui: &mut egui::Ui, tr: &Tr, section: &Section) {
    for block in section.blocks {
        match block {
            Text(key) => {
                ui.label(tr.t_args(key, &symbol::args()));
            }
            Cli(line, key) => {
                ui.label(RichText::new(*line).monospace().strong());
                ui.indent(*line, |ui| ui.label(tr.t(key)));
            }
        }
    }
}

/// Draws the About modal; false once the user closed it.
fn about(ctx: &egui::Context, tr: &Tr, storage: &Storage) -> bool {
    let mut close = false;
    let response = widgets::modal(ctx, Id::unique(ABOUT_ID)).show(ctx, |ui| {
        ui.set_max_width(ABOUT_WIDTH);
        ui.horizontal(|ui| {
            ui.heading("Archeion");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, symbol::REMOVE, tr.t("close")).clicked() {
                    close = true;
                }
            });
        });
        ui.label(tr.t_args("about-version", &[("version", arg(env!("CARGO_PKG_VERSION")))]));
        ui.label(tr.t("about-name"));
        ui.add_space(theme::SPACE_S);
        ui.label(tr.t("about-author"));
        ui.label(tr.t("about-license"));
        ui.hyperlink_to(tr.t("about-repository"), REPOSITORY);
        ui.add_space(theme::SPACE_S);
        folder_row(ui, tr, &tr.t("data-folder"), storage.dir());
        folder_row(ui, tr, &tr.t("about-logs-folder"), &storage.logs_dir());
        ui.add_space(theme::SPACE_S);
        // Inside a row, so the right-aligned layout takes one line, not
        // the whole height left.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr.t("close")).clicked() {
                    close = true;
                }
            });
        });
    });
    // Esc and a click outside close it too.
    !(close || response.should_close())
}

/// A folder's name and path, with a button that opens it in File Explorer
/// (disabled while the folder does not exist, e.g. no log written yet).
pub(super) fn folder_row(ui: &mut egui::Ui, tr: &Tr, label: &str, path: &Path) {
    widgets::section_title(ui, label);
    ui.horizontal(|ui| {
        let exists = path.is_dir();
        if ui
            .add_enabled(exists, egui::Button::new(tr.t("about-open-folder")))
            .on_disabled_hover_text(tr.t("about-folder-missing"))
            .clicked()
        {
            shell::open(path.to_path_buf());
        }
        ui.add(egui::Label::new(RichText::new(path.display().to_string()).weak()).wrap());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key of the guide and of About exists, in both languages: a missing key
    /// would show its id instead of a text.
    #[test]
    fn every_guide_key_is_translated() {
        let en = include_str!("../../locales/en-US/main.ftl");
        let it = include_str!("../../locales/it-IT/main.ftl");
        let mut keys = vec![
            "doc-title",
            "doc-intro",
            "menu-documentation",
            "menu-about",
            "about-version",
            "about-name",
            "about-author",
            "about-license",
            "about-repository",
            "about-logs-folder",
            "about-open-folder",
            "about-folder-missing",
        ];
        for s in SECTIONS {
            keys.push(s.title);
            for b in s.blocks {
                keys.push(match b {
                    Text(k) | Cli(_, k) => *k,
                });
            }
        }
        for file in [en, it] {
            for key in &keys {
                let defined = file.lines().any(|l| l.starts_with(&format!("{key} =")));
                assert!(defined, "{key} is not defined");
            }
        }
    }
}
