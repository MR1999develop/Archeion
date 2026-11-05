//! The list of profiles at the left of the window. A click selects a
//! profile; the right button opens its commands (rename, duplicate, move
//! up or down, delete), which the File menu and the keyboard also have; dragging a
//! profile changes the order, with a line where it will go. A rename
//! happens in place: Enter or a click elsewhere keeps the name, Esc
//! cancels.

use super::{symbol, widgets};
use crate::i18n::Tr;
use archeion_core::{Profile, ProfileId};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, RichText, Ui};

pub const RENAME: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::F2);
pub const DUPLICATE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
pub const MOVE_UP: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp);
pub const MOVE_DOWN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::ArrowDown);
pub const DELETE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);

/// What the user did in the list, for the window to carry out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Select(ProfileId),
    /// A new name, typed and confirmed (never blank).
    Rename(ProfileId, String),
    /// A copy of the profile, beside it, to adapt.
    Duplicate(ProfileId),
    /// The profile at `from` goes to position `to`.
    Move {
        from: usize,
        to: usize,
    },
    /// Open the "Delete profile" warning for this profile.
    Delete(ProfileId),
}

/// What the list remembers between frames.
#[derive(Debug, Default)]
pub struct ProfileList {
    /// The profile being renamed, and the name being typed.
    renaming: Option<(ProfileId, String)>,
    /// The name field takes the keyboard at the next frame.
    focus: bool,
}

impl ProfileList {
    /// Turns the name of `p` in the list into a field.
    pub fn start_rename(&mut self, p: &Profile) {
        self.renaming = Some((p.id, p.name.clone()));
        self.focus = true;
    }

    /// True while a name is being typed.
    pub fn renaming(&self) -> bool {
        self.renaming.is_some()
    }

    /// Draws the profiles; `running` is the profile of the job in progress.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        profiles: &[Profile],
        selected: Option<ProfileId>,
        running: Option<ProfileId>,
    ) -> Option<Action> {
        let mut action = None;
        let last = profiles.len().saturating_sub(1);
        for (i, p) in profiles.iter().enumerate() {
            if let Some(done) = self.rename_field(ui, p) {
                action = done;
                continue;
            }
            let name = if p.name.trim().is_empty() { tr.t("unnamed") } else { p.name.clone() };
            let text = if running == Some(p.id) { symbol::with(symbol::RUNNING, name) } else { name };
            let resp = widgets::draggable_list_item(ui, selected == Some(p.id), RichText::new(text))
                .on_hover_text(tr.t("profile-item-hint"));
            resp.dnd_set_drag_payload(i);
            let above = |resp: &egui::Response| {
                ui.ctx().pointer_interact_pos().is_some_and(|pos| pos.y < resp.rect.center().y)
            };
            if let Some(from) = resp.dnd_hover_payload::<usize>()
                && *from != i
            {
                widgets::drop_line(ui, resp.rect, above(&resp));
            }
            if let Some(from) = resp.dnd_release_payload::<usize>()
                && let Some(to) = moved_index(*from, i, above(&resp))
            {
                action = Some(Action::Move { from: *from, to });
            }
            // The right button selects too, as in Explorer: the commands
            // act on the profile shown.
            if resp.clicked() || resp.secondary_clicked() {
                action = Some(Action::Select(p.id));
            }
            resp.context_menu(|ui| {
                if let Some(a) = self.commands(ui, tr, p, i, last) {
                    action = Some(a);
                }
            });
        }
        action
    }

    /// The commands of profile `p`, at position `i` of `0..=last`: the
    /// context menu of the list and the File menu.
    pub fn commands(&mut self, ui: &mut Ui, tr: &Tr, p: &Profile, i: usize, last: usize) -> Option<Action> {
        let [rename, duplicate, up, down] =
            [RENAME, DUPLICATE, MOVE_UP, MOVE_DOWN].map(|s| ui.ctx().format_shortcut(&s));
        let mut action = None;
        if ui.add(egui::Button::new(tr.t("rename-profile")).shortcut_text(rename)).clicked() {
            self.start_rename(p);
            action = Some(Action::Select(p.id));
        }
        let copy = egui::Button::new(tr.t("duplicate-profile")).shortcut_text(duplicate);
        if ui.add(copy).on_hover_text(tr.t("duplicate-profile-hint")).clicked() {
            action = Some(Action::Duplicate(p.id));
        }
        let up = egui::Button::new(tr.t("move-profile-up")).shortcut_text(up);
        if ui.add_enabled(i > 0, up).on_disabled_hover_text(tr.t("move-profile-first")).clicked() {
            action = Some(Action::Move { from: i, to: i - 1 });
        }
        let down = egui::Button::new(tr.t("move-profile-down")).shortcut_text(down);
        if ui.add_enabled(i < last, down).on_disabled_hover_text(tr.t("move-profile-last")).clicked() {
            action = Some(Action::Move { from: i, to: i + 1 });
        }
        ui.separator();
        let text = RichText::new(tr.t("delete-profile-menu")).color(ui.visuals().error_fg_color);
        let delete = egui::Button::new(text).shortcut_text(tr.t("key-delete"));
        if ui.add(delete).on_hover_text(tr.t("delete-profile-hint")).clicked() {
            action = Some(Action::Delete(p.id));
        }
        action
    }

    /// The name field of `p` while it is being renamed: `None` when `p` is
    /// not, otherwise what the field gave (a new name once confirmed).
    fn rename_field(&mut self, ui: &mut Ui, p: &Profile) -> Option<Option<Action>> {
        let (id, text) = self.renaming.as_mut().filter(|(id, _)| *id == p.id)?;
        let id = *id;
        let field = ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
        if std::mem::take(&mut self.focus) {
            field.request_focus();
            select_all(ui, &field, text.chars().count());
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) && (field.has_focus() || field.lost_focus()) {
            self.renaming = None;
            return Some(None);
        }
        if !field.lost_focus() {
            return Some(None);
        }
        let name = text.trim().to_owned();
        self.renaming = None;
        // A blank name is not taken: the old one stays.
        Some((!name.is_empty() && name != p.name).then_some(Action::Rename(id, name)))
    }
}

/// Selects the whole text of the field `field`, as a rename starts.
fn select_all(ui: &Ui, field: &egui::Response, chars: usize) {
    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), field.id) {
        let all = egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(chars));
        state.cursor.set_char_range(Some(all));
        state.store(ui.ctx(), field.id);
    }
}

/// Where the item at `from` ends up when dropped on the item at `onto`,
/// in its upper half (`above`) or its lower one; `None` when it stays.
pub fn moved_index(from: usize, onto: usize, above: bool) -> Option<usize> {
    let before = if above { onto } else { onto + 1 };
    let to = if before > from { before - 1 } else { before };
    (to != from).then_some(to)
}

#[cfg(test)]
mod tests {
    use super::moved_index;

    #[test]
    fn a_dropped_item_goes_where_the_line_shows() {
        // Four items 0 1 2 3; 0 dropped on 2.
        assert_eq!(moved_index(0, 2, true), Some(1), "above 2: between 1 and 2");
        assert_eq!(moved_index(0, 2, false), Some(2), "below 2: between 2 and 3");
        assert_eq!(moved_index(3, 0, true), Some(0), "to the top");
        assert_eq!(moved_index(0, 3, false), Some(3), "to the bottom");
        assert_eq!(moved_index(2, 1, false), None, "just below its neighbour above: where it is");
        assert_eq!(moved_index(1, 2, true), None, "just above its neighbour below: where it is");
        assert_eq!(moved_index(1, 1, true), None, "on itself");
    }
}
