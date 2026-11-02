//! The pieces every view draws the same way: titles, tabs, buttons by
//! importance, dialog windows. Built on the tokens of [`super::theme`].

use super::theme::{self, Tokens};
use eframe::egui::{
    self, Align2, Button, Response, RichText, Sense, Stroke, Ui, WidgetInfo, accesskit::Role,
};

/// Title of a group of controls: semibold body text.
pub fn section_title(ui: &mut Ui, text: impl Into<String>) -> Response {
    ui.label(section_text(text))
}

/// The text of a section title, for headers drawn by egui.
pub fn section_text(text: impl Into<String>) -> RichText {
    RichText::new(text).family(theme::semibold()).size(theme::FONT_BODY)
}

/// The one main action of a view: accent fill.
pub fn primary_button(ui: &Ui, text: impl Into<String>) -> Button<'static> {
    let t = Tokens::current(ui);
    Button::new(RichText::new(text).color(t.on_accent)).fill(t.accent).stroke(Stroke::NONE)
}

/// An action that destroys data, in its confirmation: error fill.
pub fn danger_button(ui: &Ui, text: impl Into<String>) -> Button<'static> {
    let t = Tokens::current(ui);
    Button::new(RichText::new(text).color(t.on_accent)).fill(t.error).stroke(Stroke::NONE)
}

/// Tabs drawn as text with an accent bar under the selected one, so they
/// do not look like the selected item of a list. `small` for the tabs
/// inside a tab.
pub fn tabs<T: PartialEq + Copy>(
    ui: &mut Ui,
    current: &mut T,
    items: &[(T, String, Option<String>)],
    small: bool,
) {
    let t = Tokens::current(ui);
    let size = if small { theme::FONT_BODY } else { 15.0 };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = if small { theme::SPACE_M } else { theme::SPACE_L };
        for (value, label, hint) in items {
            let on = *current == *value;
            let text = RichText::new(label.as_str()).size(size).color(if on { t.text } else { t.text_weak });
            let text = if on { text.family(theme::semibold()) } else { text };
            let galley = egui::WidgetText::from(text).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            let (rect, resp) = ui.allocate_exact_size(galley.size() + egui::vec2(0.0, 10.0), Sense::click());
            let resp = match hint {
                Some(h) => resp.on_hover_text(h.as_str()),
                None => resp,
            };
            resp.widget_info(|| WidgetInfo::selected(Role::Tab, true, on, label.as_str()));
            if ui.is_rect_visible(rect) {
                let color = if on || resp.hovered() { t.text } else { t.text_weak };
                ui.painter().galley(rect.left_top(), galley, color);
                if on || resp.hovered() {
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), rect.bottom() - 3.0),
                        egui::vec2(rect.width(), 3.0),
                    );
                    let fill = if on { t.accent } else { t.stroke_weak };
                    ui.painter().rect_filled(bar, 1.5, fill);
                }
            }
            if resp.clicked() {
                *current = *value;
            }
        }
    });
}

/// A secondary window (Preferences, Help, About) centred on the main one
/// the first time it opens.
pub fn dialog(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> egui::Window<'static> {
    egui::Window::new(title)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
}

/// A modal dialog with the margins of a window (egui's default is the
/// tighter frame of a popup).
pub fn modal(ctx: &egui::Context, id: egui::Id) -> egui::Modal {
    egui::Modal::new(id).frame(egui::Frame::window(&ctx.global_style()))
}
