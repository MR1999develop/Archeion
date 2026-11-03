//! "Manage backups" view of the Backups tab: the list of dates with their
//! kind and size, the manual merge and the deletion of one backup. Kept
//! apart from restoring files.

use super::{
    load::{Load, Problem},
    restore::Action,
    symbol, theme, widgets,
};
use crate::{
    credentials,
    i18n::{Tr, arg},
};
use archeion_core::{
    ArchiveInfo, ArchiveKind, ArchiveName, Profile,
    merge_plan::{DeleteError, Deletion, deletion},
};
use eframe::egui::{self, RichText, Ui};
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub struct BackupsView {
    archives: Load<Vec<ArchiveInfo>>,
    /// Backup chosen in the list, for the deletion.
    selected: Option<String>,
    /// Choosing backups to merge.
    merge_mode: bool,
    /// Backups ticked for the merge; everything between the oldest and the
    /// newest ticked one is merged.
    merge_marks: BTreeSet<String>,
    /// "I understand" for a merge that replaces the initial complete backup.
    merge_ack: bool,
    /// The merged backup keeps the files deleted between the chosen ones;
    /// set from the profile when the choice starts.
    keep_deleted: bool,
    /// Backup whose deletion is being confirmed.
    deleting: Option<String>,
    /// "I understand" for the deletion.
    delete_ack: bool,
}

impl BackupsView {
    /// Draws the list; returns a merge or deletion to start. `refresh` is
    /// set when the user asks to read the backups again.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        profile: &Profile,
        busy: bool,
        refresh: &mut bool,
    ) -> Option<Action> {
        self.archives.poll(&tr.t("unexpected-stop"));
        if matches!(self.archives, Load::Idle) {
            let p = profile.clone();
            self.archives = Load::spawn(ui.ctx(), move || {
                let pw = credentials::get_or_none(p.id);
                archeion_engine::list_archives(&p, pw.as_ref()).map_err(Problem::from)
            });
        }
        ui.set_max_width(theme::FORM_MAX_WIDTH);
        ui.add_space(theme::SPACE_S);
        ui.horizontal(|ui| {
            widgets::section_title(ui, tr.t("dates"));
            if widgets::icon_button(ui, symbol::REFRESH, tr.t("refresh")).clicked() {
                *refresh = true;
            }
        });
        let Some(list) = self.archives.ready().cloned() else {
            self.archives.status_ui(ui, tr, &tr.t("reading-backups"));
            return None;
        };
        if list.is_empty() {
            ui.weak(tr.t("no-backups-yet"));
            return None;
        }
        if self.selected.as_ref().is_none_or(|s| !list.iter().any(|a| &a.name == s)) {
            self.selected = list.first().map(|a| a.name.clone());
        }
        // `list` is newest first: the range is between the two ticked ends.
        let ticked: Vec<usize> = list
            .iter()
            .enumerate()
            .filter(|(_, a)| self.merge_marks.contains(&a.name))
            .map(|(i, _)| i)
            .collect();
        let range = match (ticked.first(), ticked.last()) {
            (Some(&lo), Some(&hi)) => Some(lo..=hi),
            _ => None,
        };

        let mut action = None;
        if self.merge_mode {
            action = self.merge_controls(ui, tr, &list, range.clone(), busy);
        } else if list.len() >= 2
            && ui.button(tr.t("merge-choose")).on_hover_text(tr.t("merge-choose-hint")).clicked()
        {
            self.merge_mode = true;
            self.keep_deleted = profile.merge.keep_deleted;
        }
        ui.add_space(theme::SPACE_S);

        let mut clicked = None;
        let mut toggled = None;
        egui::ScrollArea::vertical().id_salt("dates").max_height(ui.available_height() * 0.7).show(
            ui,
            |ui| {
                for (i, a) in list.iter().enumerate() {
                    let kind = tr.t(match a.kind {
                        ArchiveKind::Complete => "kind-complete",
                        ArchiveKind::Incremental => "kind-incremental",
                    });
                    ui.horizontal(|ui| {
                        if self.merge_mode {
                            let mut on = range.as_ref().is_some_and(|r| r.contains(&i));
                            if widgets::checkbox(ui, &mut on, "").on_hover_text(tr.t("merge-tick")).changed()
                            {
                                toggled = Some(a.name.clone());
                            }
                        }
                        let selected = self.selected.as_deref() == Some(a.name.as_str());
                        let text = format!("{}  ·  {kind}  ·  {}", a.time_label, tr.bytes(a.bytes_stored));
                        let resp = widgets::list_item(ui, selected, egui::RichText::new(text), None)
                            .on_hover_text(tr.t_args(
                                "archive-tooltip",
                                &[
                                    ("files", arg(a.files_total)),
                                    ("stored", arg(a.files_stored)),
                                    ("size", arg(tr.bytes(a.bytes_stored))),
                                ],
                            ));
                        if resp.clicked() {
                            clicked = Some(a.name.clone());
                        }
                    });
                }
            },
        );
        if let Some(name) = toggled {
            self.toggle_mark(&list, name);
        }
        if let Some(name) = clicked {
            self.selected = Some(name);
        }
        if !self.merge_mode
            && let Some(a) = self.delete_controls(ui, tr, &list, busy)
        {
            action = Some(a);
        }
        action
    }

    /// "Delete this backup…" for the selected date, with its confirmation.
    fn delete_controls(&mut self, ui: &mut Ui, tr: &Tr, list: &[ArchiveInfo], busy: bool) -> Option<Action> {
        let selected = self.selected.clone()?;
        let info = list.iter().find(|a| a.name == selected)?;
        // `list` is newest first; the plan wants oldest first.
        let names: Vec<ArchiveName> = list.iter().rev().filter_map(|a| a.name.parse().ok()).collect();
        let name: ArchiveName = selected.parse().ok()?;
        let plan = deletion(&names, &name);
        if self.deleting.as_deref() != Some(selected.as_str()) {
            self.deleting = None;
            self.delete_ack = false;
        }

        ui.add_space(theme::SPACE_S);
        ui.separator();
        let mut action = None;
        let confirming = self.deleting.is_some();
        match (confirming, &plan) {
            (false, _) => {
                let why = match &plan {
                    Ok(_) if busy => Some(tr.t("busy-hint")),
                    Ok(_) => None,
                    Err(DeleteError::InitialWithOthers) => Some(tr.t("delete-initial-hint")),
                    Err(DeleteError::Complete) => Some(tr.t("delete-complete-hint")),
                    Err(DeleteError::Unknown) => Some(tr.t("unexpected-stop")),
                };
                let resp = ui.add_enabled(why.is_none(), egui::Button::new(tr.t("delete-backup")));
                let resp = match why {
                    Some(w) => resp.on_disabled_hover_text(w),
                    None => resp.on_hover_text(tr.t("delete-backup-hint")),
                };
                if resp.clicked() {
                    self.deleting = Some(selected.clone());
                    self.delete_ack = false;
                }
            }
            (true, Ok(d)) => {
                let date = arg(info.time_label.clone());
                let text = match d {
                    Deletion::Remove(_) if list.len() == 1 => {
                        tr.t_args("delete-only-warning", &[("date", date)])
                    }
                    Deletion::Remove(_) => tr.t_args("delete-newest-warning", &[("date", date)]),
                    Deletion::MergeWithNext(_) => tr.t_args("delete-middle-warning", &[("date", date)]),
                };
                ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, text));
                widgets::checkbox(ui, &mut self.delete_ack, tr.t("merge-complete-ack"));
                ui.horizontal(|ui| {
                    let ready = self.delete_ack && !busy;
                    let why = if busy { tr.t("busy-hint") } else { tr.t("merge-ack-hint") };
                    let button = egui::Button::new(
                        RichText::new(tr.t("delete-backup-confirm")).color(ui.visuals().error_fg_color),
                    );
                    if ui.add_enabled(ready, button).on_disabled_hover_text(why).clicked() {
                        action = Some(Action::Delete { archive: selected.clone() });
                    }
                    if ui.button(tr.t("cancel")).clicked() || action.is_some() {
                        self.deleting = None;
                        self.delete_ack = false;
                    }
                });
            }
            (true, Err(_)) => self.deleting = None,
        }
        action
    }

    /// The marks are the two ends of the range. Clicking an end removes it;
    /// clicking another backup adds it, replacing the nearer end when both
    /// are already set.
    fn toggle_mark(&mut self, list: &[ArchiveInfo], name: String) {
        self.merge_ack = false;
        if self.merge_marks.remove(&name) {
            return;
        }
        if self.merge_marks.len() >= 2 {
            let pos = |n: &str| list.iter().position(|a| a.name == n).unwrap_or(0);
            let target = pos(&name);
            let nearer = self.merge_marks.iter().min_by_key(|m| pos(m).abs_diff(target)).cloned();
            if let Some(n) = nearer {
                self.merge_marks.remove(&n);
            }
        }
        self.merge_marks.insert(name);
    }

    /// Summary, warning and buttons of the manual merge.
    fn merge_controls(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        list: &[ArchiveInfo],
        range: Option<std::ops::RangeInclusive<usize>>,
        busy: bool,
    ) -> Option<Action> {
        ui.weak(tr.t("merge-tick-hint"));
        let mut action = None;
        let chosen = range.as_ref().map_or(0, |r| r.clone().count());
        let (newest, oldest) = match &range {
            Some(r) => (list.get(*r.start()), list.get(*r.end())),
            None => (None, None),
        };
        // The initial complete backup is the oldest of all.
        let replaces_complete = oldest.is_some_and(|o| {
            o.kind == ArchiveKind::Complete && list.last().is_some_and(|l| l.name == o.name)
        });
        if chosen >= 2
            && let Some(n) = newest
        {
            ui.label(
                tr.t_args("merge-summary", &[("count", arg(chosen)), ("date", arg(n.time_label.clone()))]),
            );
        }
        if chosen >= 2 && replaces_complete {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("merge-complete-warning")),
            );
            widgets::checkbox(ui, &mut self.merge_ack, tr.t("merge-complete-ack"));
        }
        if chosen >= 2 {
            widgets::checkbox(ui, &mut self.keep_deleted, tr.t("merge-keep-deleted"))
                .on_hover_text(tr.t("merge-keep-deleted-hint"));
        }
        ui.horizontal(|ui| {
            let ready = chosen >= 2 && !busy && (!replaces_complete || self.merge_ack);
            let why = if busy {
                tr.t("busy-hint")
            } else if chosen < 2 {
                tr.t("merge-two-hint")
            } else {
                tr.t("merge-ack-hint")
            };
            if ui
                .add_enabled(ready, egui::Button::new(tr.t("merge-selected")))
                .on_disabled_hover_text(why)
                .clicked()
                && let (Some(n), Some(o)) = (newest, oldest)
            {
                action = Some(Action::Merge {
                    first: o.name.clone(),
                    last: n.name.clone(),
                    keep_deleted: self.keep_deleted,
                });
            }
            if ui.button(tr.t("cancel")).clicked() || action.is_some() {
                self.merge_mode = false;
                self.merge_marks.clear();
                self.merge_ack = false;
            }
        });
        action
    }
}
