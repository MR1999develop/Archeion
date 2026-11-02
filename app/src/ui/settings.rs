//! The Preferences window (File > Preferences…, Ctrl+,): the interface
//! language, how the window behaves, and where the data is.
//!
//! It is a small non-modal window: every change applies at once, so there
//! is no OK or Cancel; ✕ or Esc closes it. This module only draws it and
//! reports the new values; the caller saves and applies them.

use super::{help, theme, widgets};
use crate::i18n::Tr;
use archeion_core::{Language, WindowOptions};
use eframe::egui::{self, Id, Key, KeyboardShortcut, LayerId, Modifiers, Order, RichText};
use std::path::Path;

/// Opens the window.
pub const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma);
const WINDOW_ID: &str = "preferences";
const WIDTH: f32 = 420.0;

/// The values the window edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Values {
    pub language: Language,
    pub window: WindowOptions,
}

/// Whether the window is open.
#[derive(Debug, Default)]
pub struct Preferences {
    open: bool,
}

impl Preferences {
    /// Opens the window, or brings it to the front when already open.
    pub fn open(&mut self, ctx: &egui::Context) {
        if self.open {
            ctx.move_to_top(LayerId::new(Order::Middle, Id::unique(WINDOW_ID)));
        }
        self.open = true;
    }

    /// Draws the window when open; call once per frame, after the panels.
    /// `tray`: Windows gave a notification-area icon (closing to it is
    /// possible). Returns the values once the user changed one.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        tr: &Tr,
        data_dir: &Path,
        tray: bool,
        current: Values,
    ) -> Option<Values> {
        if !self.open {
            return None;
        }
        let id = Id::unique(WINDOW_ID);
        // Esc closes the window when it is the front one and no popup (the
        // language list, a menu) or modal (About) takes the key first.
        let in_front = ctx.top_layer_id() == Some(LayerId::new(Order::Middle, id));
        if in_front
            && !egui::Popup::is_any_open(ctx)
            && ctx.memory(|m| m.top_modal_layer()).is_none()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.open = false;
            return None;
        }
        let mut values = current;
        widgets::dialog(ctx, tr.t("preferences-title")).id(id).open(&mut self.open).resizable(false).show(
            ctx,
            |ui| {
                ui.set_max_width(WIDTH);
                language_section(ui, tr, &mut values.language, tray);
                ui.add_space(theme::SPACE_M);
                window_section(ui, tr, &mut values.window, tray);
                ui.add_space(theme::SPACE_M);
                help::folder_row(ui, tr, &tr.t("data-folder"), data_dir);
            },
        );
        (values != current).then_some(values)
    }
}

/// "System language" first, then every language by its own name.
fn language_section(ui: &mut egui::Ui, tr: &Tr, language: &mut Language, tray: bool) {
    widgets::section_title(ui, tr.t("language"));
    let name = |l: Language| l.native_name().map_or_else(|| tr.t("language-system"), str::to_owned);
    egui::ComboBox::from_id_salt("language")
        .selected_text(name(*language))
        .show_ui(ui, |ui| {
            ui.selectable_value(language, Language::System, name(Language::System));
            for l in Language::ALL {
                ui.selectable_value(language, l, name(l));
            }
        })
        .response
        .accessible_name(tr.t("language"));
    if tray {
        ui.label(RichText::new(tr.t("language-tray-restart")).weak().small());
    }
}

/// The two window options, with their hints.
fn window_section(ui: &mut egui::Ui, tr: &Tr, window: &mut WindowOptions, tray: bool) {
    widgets::section_title(ui, tr.t("preferences-window"));
    ui.add_enabled(tray, egui::Checkbox::new(&mut window.close_to_tray, tr.t("option-tray")))
        .on_hover_text(tr.t("option-tray-hint"))
        .on_disabled_hover_text(tr.t("option-tray-unavailable"));
    ui.checkbox(&mut window.start_with_windows, tr.t("option-autostart"))
        .on_hover_text(tr.t("option-autostart-hint"));
}

#[cfg(test)]
mod tests {
    /// Every key of the window exists in both languages.
    #[test]
    fn every_key_is_translated() {
        let en = include_str!("../../locales/en-US/main.ftl");
        let it = include_str!("../../locales/it-IT/main.ftl");
        let keys = [
            "menu-preferences",
            "preferences-title",
            "preferences-window",
            "language",
            "language-system",
            "language-tray-restart",
            "data-folder",
        ];
        for file in [en, it] {
            for key in keys {
                let defined = file.lines().any(|l| l.starts_with(&format!("{key} =")));
                assert!(defined, "{key} is not defined");
            }
        }
    }
}
