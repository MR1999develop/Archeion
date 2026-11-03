//! The pieces every view draws the same way: titles, tabs, buttons by
//! importance, dialog windows. Built on the tokens of [`super::theme`].

use super::theme::{self, Tokens};
use eframe::egui::{
    self, Align2, Button, Response, RichText, Sense, Stroke, Ui, WidgetInfo, accesskit::Role,
};

/// A message of the status bar (an outcome, an Undo offer): a label that a
/// screen reader announces when it appears or changes (a polite live
/// region), in the warning colour when `warn`.
pub fn status_text(ui: &mut Ui, text: impl Into<String>, warn: bool) -> Response {
    let text = text.into();
    let rich = RichText::new(text.as_str());
    let rich = if warn { rich.color(ui.visuals().warn_fg_color) } else { rich };
    let resp = ui.add(egui::Label::new(rich).sense(Sense::hover()));
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, text.as_str()));
    ui.ctx().accesskit_node_builder(resp.id, |node| node.set_live(egui::accesskit::Live::Polite));
    resp
}

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
            let (rect, resp) = ui.allocate_exact_size(
                galley.size() + egui::vec2(0.0, theme::TAB_EXTRA_HEIGHT),
                Sense::click(),
            );
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

/// A checkbox whose ticked box is filled with the accent, as in Windows 11
/// (egui only draws the tick).
pub fn checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
    let text = text.into();
    let alone = text.is_empty();
    let resp = ui.checkbox(checked, text);
    if *checked {
        let t = Tokens::current(ui);
        let rect = tick_box_rect(ui, &resp, alone);
        let fill = if ui.is_enabled() { t.accent } else { t.stroke };
        ui.painter().rect_filled(rect, tick_box_radius(ui), fill);
        let inner = rect.shrink(rect.width() * 0.25);
        let points = vec![
            egui::pos2(inner.left(), inner.center().y),
            egui::pos2(inner.left() + inner.width() * 0.4, inner.bottom()),
            egui::pos2(inner.right(), inner.top()),
        ];
        ui.painter().add(egui::Shape::line(points, Stroke::new(1.5, t.on_accent)));
    }
    resp
}

/// A checkbox for "some of it": a filled accent square in an empty box,
/// distinct from the ticked box. Clicking it reports a change.
pub fn partial_checkbox(ui: &mut Ui, text: impl Into<egui::WidgetText>) -> Response {
    let text = text.into();
    let alone = text.is_empty();
    let resp = ui.add(egui::Checkbox::new(&mut false, text).indeterminate(true));
    let rect = tick_box_rect(ui, &resp, alone);
    // Covers egui's dash.
    let inner = rect.shrink(1.0);
    ui.painter().rect_filled(inner, tick_box_radius(ui), ui.style().interact(&resp).bg_fill);
    let fill = if ui.is_enabled() { Tokens::current(ui).accent } else { Tokens::current(ui).stroke };
    ui.painter().rect_filled(rect.shrink(rect.width() * 0.25), 1.0, fill);
    resp
}

/// The box of a checkbox drawn by egui: at the left of its text, or in
/// the middle of its room when it has none (`alone`).
fn tick_box_rect(ui: &Ui, resp: &Response, alone: bool) -> egui::Rect {
    let size = ui.spacing().icon_width;
    // Alone, egui still leaves the gap before a text: box and gap are
    // centred together.
    let x = if alone {
        resp.rect.center().x - ui.spacing().icon_spacing * 0.5
    } else {
        resp.rect.left() + size * 0.5
    };
    let center = egui::pos2(x, resp.rect.center().y);
    egui::Rect::from_center_size(center, egui::Vec2::splat(size))
}

/// The corners of a checkbox box, as egui caps them.
fn tick_box_radius(ui: &Ui) -> egui::CornerRadius {
    let cap = (0.3 * ui.spacing().icon_width).round();
    egui::CornerRadius::from(f32::from(theme::RADIUS).min(cap))
}

/// A quiet button with one icon (remove, refresh): no border or fill until
/// hovered. `hint` names the action for the tooltip and screen readers.
pub fn icon_button(ui: &mut Ui, icon: &str, hint: impl Into<String>) -> Response {
    let hint = hint.into();
    let button = Button::new(RichText::new(icon).size(theme::FONT_SMALL)).frame_when_inactive(false).small();
    ui.add(button).on_hover_text(&hint).accessible_name(hint)
}

/// An item of a list as wide as the list, with its title and an optional
/// weak second line, selected and hovered as one (egui's selectable label
/// covers only its own text), with the accent mark of Windows 11 lists.
pub fn list_item(ui: &mut Ui, selected: bool, title: RichText, detail: Option<&str>) -> Response {
    let t = Tokens::current(ui);
    let pad = egui::vec2(theme::SPACE_S, theme::SPACE_XS);
    let width = ui.available_width() - 2.0 * pad.x;
    let wrap = Some(egui::TextWrapMode::Truncate);
    let title_galley =
        egui::WidgetText::from(title.clone()).into_galley(ui, wrap, width, egui::TextStyle::Body);
    let detail_galley = detail.map(|d| {
        let text = RichText::new(d).size(theme::FONT_SMALL);
        egui::WidgetText::from(text).into_galley(ui, wrap, width, egui::TextStyle::Small)
    });
    let text_height = title_galley.size().y + detail_galley.as_ref().map_or(0.0, |g| g.size().y);
    let height = (text_height + 2.0 * pad.y).max(ui.spacing().interact_size.y);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, title.text()));
    if ui.is_rect_visible(rect) {
        let fill = if selected { Some(t.selected) } else { resp.hovered().then_some(t.control_hover) };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, theme::RADIUS, fill);
        }
        if selected {
            accent_mark(ui, rect, height * 0.5, t.accent);
        }
        let top = egui::pos2(rect.left() + pad.x, rect.center().y - text_height / 2.0);
        let below = top + egui::vec2(0.0, title_galley.size().y);
        ui.painter().galley(top, title_galley, t.text);
        if let Some(g) = detail_galley {
            ui.painter().galley(below, g, t.text_weak);
        }
    }
    resp
}

/// A name that can be chosen in place (a file of the restore tree): plain
/// text that takes the look of a selected list item, with its accent mark,
/// while `selected`, and a lighter fill when hovered. It does not keep the
/// keyboard focus, so the arrow keys stay with the caller.
pub fn chosen_name(ui: &mut Ui, selected: bool, text: impl Into<egui::WidgetText>) -> Response {
    let t = Tokens::current(ui);
    let text = text.into();
    let name = text.text().to_owned();
    let behind = ui.painter().add(egui::Shape::Noop);
    // Room for the accent mark before the text.
    ui.add_space(theme::SPACE_S);
    let label = egui::Label::new(text).selectable(false).sense(Sense::click());
    let resp = ui.add(label);
    if resp.has_focus() {
        resp.surrender_focus();
    }
    resp.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, &name));
    let rect = egui::Rect::from_min_max(
        resp.rect.min - egui::vec2(theme::SPACE_S, theme::CHOSEN_PAD_Y),
        resp.rect.max + egui::vec2(theme::SPACE_XS, theme::CHOSEN_PAD_Y),
    );
    let fill = if selected { Some(t.selected) } else { resp.hovered().then_some(t.control_hover) };
    if let Some(fill) = fill {
        ui.painter().set(behind, egui::Shape::rect_filled(rect, theme::RADIUS, fill));
    }
    if selected {
        accent_mark(ui, rect, rect.height() * 0.6, t.accent);
    }
    resp
}

/// The accent mark at the left edge of `rect`, `height` high at most.
fn accent_mark(ui: &Ui, rect: egui::Rect, height: f32, color: egui::Color32) {
    let half = theme::ACCENT_MARK_WIDTH / 2.0;
    let mark = egui::Rect::from_center_size(
        egui::pos2(rect.left() + half, rect.center().y),
        egui::vec2(theme::ACCENT_MARK_WIDTH, height.min(theme::ACCENT_MARK_MAX_HEIGHT)),
    );
    ui.painter().rect_filled(mark, half, color);
}
