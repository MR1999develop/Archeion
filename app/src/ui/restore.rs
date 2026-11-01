//! Backups tab: the list of dates (with the manual merge and the deletion
//! of one backup), the tree of a date, search across every date, the
//! versions of one file, and the restore options.
//!
//! An encrypted destination is read with the password saved in Credential
//! Manager.
//!
//! Every query reads manifests on a background thread; the restore itself
//! runs in the worker process like a backup.

use super::{theme, tree::Tree};
use crate::{
    credentials,
    i18n::{Tr, arg},
};
use archeion_core::{
    ArchiveInfo, ArchiveKind, ArchiveName, EntryKind, Profile, ProfileId, keys,
    merge_plan::{DeleteError, Deletion, deletion},
};
use archeion_engine::{EngineError, FileVersion, RestoreRequest, RestoreTarget, SearchHit};
use eframe::egui::{self, Context, RichText, Ui, collapsing_header::CollapsingState};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

/// Typing pause before a search starts.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);
const SEARCH_LIMIT: usize = 200;

/// Why a query failed, as the tab explains it.
#[derive(Debug)]
enum Problem {
    PasswordNeeded,
    WrongPassword,
    Other(String),
}

impl From<EngineError> for Problem {
    fn from(e: EngineError) -> Self {
        match e {
            EngineError::PasswordNeeded => Self::PasswordNeeded,
            EngineError::WrongPassword => Self::WrongPassword,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Result of a background query.
#[derive(Debug, Default)]
enum Load<T> {
    #[default]
    Idle,
    Loading(mpsc::Receiver<Result<T, Problem>>),
    Ready(T),
    Failed(Problem),
}

impl<T: Send + 'static> Load<T> {
    fn spawn(ctx: &Context, f: impl FnOnce() -> Result<T, Problem> + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("restore-query".into()).spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => Self::Loading(rx),
            Err(e) => Self::Failed(Problem::Other(e.to_string())),
        }
    }

    fn poll(&mut self, stopped: &str) {
        if let Self::Loading(rx) = self {
            match rx.try_recv() {
                Ok(Ok(v)) => *self = Self::Ready(v),
                Ok(Err(e)) => *self = Self::Failed(e),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    *self = Self::Failed(Problem::Other(stopped.to_owned()));
                }
            }
        }
    }

    fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(v) => Some(v),
            _ => None,
        }
    }

    /// Spinner or error for the states without a value.
    fn status_ui(&self, ui: &mut Ui, tr: &Tr, loading: &str) {
        match self {
            Self::Idle | Self::Loading(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(loading);
                });
            }
            Self::Failed(p) => {
                let text = match p {
                    Problem::PasswordNeeded => tr.t("run-password-needed"),
                    Problem::WrongPassword => tr.t("run-wrong-password"),
                    Problem::Other(e) => e.clone(),
                };
                ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {text}"));
            }
            Self::Ready(_) => {}
        }
    }
}

/// What the tab asks the window to start.
#[derive(Debug, Clone)]
pub enum Action {
    Restore(RestoreRequest),
    /// Merge the backups from `first` to `last` and everything between.
    Merge {
        first: String,
        last: String,
    },
    /// Delete one backup.
    Delete {
        archive: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Original,
    Beside,
    Folder,
}

#[derive(Debug, Default)]
pub struct RestoreView {
    profile: Option<ProfileId>,
    archives: Load<Vec<ArchiveInfo>>,
    /// Archive whose tree is shown.
    selected: Option<String>,
    /// Every file present at that date; those saved by this archive stand
    /// out, the others are greyed.
    tree: Load<Tree>,
    /// Keys of the files and folders ticked for restore.
    checked: BTreeSet<String>,
    /// File whose versions are shown.
    focused: Option<String>,
    versions: Load<Vec<FileVersion>>,
    query: String,
    query_edited: Option<Instant>,
    search: Load<Vec<SearchHit>>,
    target: Option<Target>,
    folder: Option<PathBuf>,
    overwrite: bool,
    /// Choosing backups to merge.
    merge_mode: bool,
    /// Backups ticked for the merge; everything between the oldest and the
    /// newest ticked one is merged.
    merge_marks: BTreeSet<String>,
    /// "I understand" for a merge that replaces the initial complete backup.
    merge_ack: bool,
    /// Backup whose deletion is being confirmed.
    deleting: Option<String>,
    /// "I understand" for the deletion.
    delete_ack: bool,
}

impl RestoreView {
    /// Forget everything loaded, e.g. after a backup or merge changed the
    /// archives. The restore options stay.
    pub fn invalidate(&mut self) {
        let (target, folder, overwrite) = (self.target, self.folder.take(), self.overwrite);
        *self = Self { target, folder, overwrite, ..Self::default() };
    }

    /// Draws the tab; returns a restore or merge to start.
    pub fn show(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, busy: bool) -> Option<Action> {
        if self.profile != Some(profile.id) {
            self.invalidate();
            self.profile = Some(profile.id);
        }
        let ctx = ui.ctx().clone();
        let stopped = tr.t("unexpected-stop");
        self.archives.poll(&stopped);
        self.tree.poll(&stopped);
        self.versions.poll(&stopped);
        self.search.poll(&stopped);
        if matches!(self.archives, Load::Idle) {
            let p = profile.clone();
            self.archives = Load::spawn(&ctx, move || {
                let pw = credentials::get_or_none(p.id);
                archeion_engine::list_archives(&p, pw.as_ref()).map_err(Problem::from)
            });
        }
        if self.selected.is_none()
            && let Some(newest) = self.archives.ready().and_then(|l| l.first()).map(|a| a.name.clone())
        {
            self.select_archive(&ctx, profile, newest);
        }
        self.run_search_when_idle(&ctx, profile);

        let mut action = None;
        egui::Panel::left("restore-dates")
            .resizable(true)
            .default_size(theme::SIDE_PANEL_WIDTH)
            .show(ui, |ui| action = self.dates_ui(ui, tr, profile, busy));
        egui::Panel::bottom("restore-options").show(ui, |ui| {
            if let Some(r) = self.options_ui(ui, tr, busy) {
                action = Some(Action::Restore(r));
            }
        });
        if self.focused.is_some() {
            egui::Panel::right("restore-versions").resizable(true).default_size(300.0).show(ui, |ui| {
                if let Some(r) = self.versions_ui(ui, tr, busy) {
                    action = Some(Action::Restore(r));
                }
            });
        }
        egui::CentralPanel::default().show(ui, |ui| self.files_ui(ui, tr, profile));
        action
    }

    fn select_archive(&mut self, ctx: &Context, profile: &Profile, name: String) {
        if self.selected.as_deref() == Some(name.as_str()) {
            return;
        }
        self.checked.clear();
        let p = profile.clone();
        let n = name.clone();
        self.tree = Load::spawn(ctx, move || {
            let pw = credentials::get_or_none(p.id);
            archeion_engine::archive_manifest(&p, pw.as_ref(), &n)
                .map(|m| Tree::build(&m.entries))
                .map_err(Problem::from)
        });
        self.selected = Some(name);
    }

    fn focus(&mut self, ctx: &Context, profile: &Profile, key: String) {
        if self.focused.as_deref() == Some(key.as_str()) {
            return;
        }
        let p = profile.clone();
        let k = key.clone();
        self.versions = Load::spawn(ctx, move || {
            let pw = credentials::get_or_none(p.id);
            archeion_engine::file_versions(&p, pw.as_ref(), &k).map_err(Problem::from)
        });
        self.focused = Some(key);
    }

    fn run_search_when_idle(&mut self, ctx: &Context, profile: &Profile) {
        let Some(edited) = self.query_edited else { return };
        if edited.elapsed() < SEARCH_DEBOUNCE {
            ctx.request_repaint_after(SEARCH_DEBOUNCE.saturating_sub(edited.elapsed()));
            return;
        }
        self.query_edited = None;
        let (p, q) = (profile.clone(), self.query.clone());
        self.search = if q.trim().is_empty() {
            Load::Idle
        } else {
            Load::spawn(ctx, move || {
                let pw = credentials::get_or_none(p.id);
                archeion_engine::search(&p, pw.as_ref(), &q, SEARCH_LIMIT).map_err(Problem::from)
            })
        };
    }

    fn dates_ui(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, busy: bool) -> Option<Action> {
        ui.add_space(theme::SPACE_S);
        ui.horizontal(|ui| {
            ui.strong(tr.t("dates"));
            if ui.small_button("⟳").on_hover_text(tr.t("refresh")).clicked() {
                self.invalidate();
                self.profile = Some(profile.id);
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
        }
        ui.add_space(theme::SPACE_S);

        let mut clicked = None;
        let mut toggled = None;
        egui::ScrollArea::vertical().id_salt("dates").show(ui, |ui| {
            for (i, a) in list.iter().enumerate() {
                let kind = tr.t(match a.kind {
                    ArchiveKind::Complete => "kind-complete",
                    ArchiveKind::Incremental => "kind-incremental",
                });
                ui.horizontal(|ui| {
                    if self.merge_mode {
                        let mut on = range.as_ref().is_some_and(|r| r.contains(&i));
                        if ui.checkbox(&mut on, "").on_hover_text(tr.t("merge-tick")).changed() {
                            toggled = Some(a.name.clone());
                        }
                    }
                    let selected = self.selected.as_deref() == Some(a.name.as_str());
                    let resp = ui
                        .selectable_label(selected, format!("{}  ·  {kind}", a.time_label))
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
        });
        if let Some(name) = toggled {
            self.toggle_mark(&list, name);
        }
        if let Some(name) = clicked {
            let ctx = ui.ctx().clone();
            self.select_archive(&ctx, profile, name);
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
                ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {text}"));
                ui.checkbox(&mut self.delete_ack, tr.t("merge-complete-ack"));
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
            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("merge-complete-warning")));
            ui.checkbox(&mut self.merge_ack, tr.t("merge-complete-ack"));
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
                action = Some(Action::Merge { first: o.name.clone(), last: n.name.clone() });
            }
            if ui.button(tr.t("cancel")).clicked() || action.is_some() {
                self.merge_mode = false;
                self.merge_marks.clear();
                self.merge_ack = false;
            }
        });
        action
    }

    fn files_ui(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile) {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(tr.t("search-hint"))
                .desired_width(f32::INFINITY),
        );
        if resp.changed() {
            self.query_edited = Some(Instant::now());
        }
        ui.add_space(theme::SPACE_S);
        let mut focus = None;
        if self.query.trim().is_empty() {
            match self.tree.ready() {
                None if self.selected.is_none() => {
                    ui.weak(tr.t("pick-date"));
                }
                None => self.tree.status_ui(ui, tr, &tr.t("reading-backups")),
                Some(tree) => {
                    ui.weak(tr.t_args(
                        "tree-count",
                        &[("stored", arg(tree.stored_files)), ("total", arg(tree.files))],
                    ));
                    ui.weak(tr.t("tree-stored-hint"));
                    if tree.stored_files == 0 && tree.files > 0 {
                        ui.add_space(theme::SPACE_S);
                        ui.weak(tr.t("tree-nothing-stored"));
                    }
                    egui::ScrollArea::vertical().id_salt("tree").auto_shrink(false).show(ui, |ui| {
                        for &r in &tree.roots {
                            node_ui(ui, tr, tree, r, &mut self.checked, self.focused.as_deref(), &mut focus);
                        }
                    });
                }
            }
        } else {
            match self.search.ready() {
                None if self.query_edited.is_some() => {}
                None => self.search.status_ui(ui, tr, &tr.t("searching")),
                Some(hits) if hits.is_empty() => {
                    ui.weak(tr.t("no-results"));
                }
                Some(hits) => {
                    ui.weak(tr.t_args("results", &[("count", arg(hits.len()))]));
                    egui::ScrollArea::vertical().id_salt("hits").auto_shrink(false).show(ui, |ui| {
                        for h in hits {
                            let path = keys::path_for(&h.key).display().to_string();
                            let mut text = format!(
                                "{path}  ·  {}",
                                tr.t_args("version-count", &[("count", arg(h.versions))])
                            );
                            if !h.present_now {
                                text.push_str(&format!("  ·  {}", tr.t("deleted-now")));
                            }
                            let selected = self.focused.as_deref() == Some(h.key.as_str());
                            if ui.selectable_label(selected, text).clicked() {
                                focus = Some(h.key.clone());
                            }
                        }
                    });
                }
            }
        }
        if let Some(key) = focus {
            let ctx = ui.ctx().clone();
            self.focus(&ctx, profile, key);
        }
    }

    fn versions_ui(&mut self, ui: &mut Ui, tr: &Tr, busy: bool) -> Option<RestoreRequest> {
        let key = self.focused.clone()?;
        let mut request = None;
        ui.add_space(theme::SPACE_S);
        ui.horizontal(|ui| {
            let name = key.rsplit('/').next().unwrap_or(&key).to_owned();
            ui.strong(name).on_hover_text(keys::path_for(&key).display().to_string());
            if ui.small_button("✕").on_hover_text(tr.t("close")).clicked() {
                self.focused = None;
            }
        });
        ui.weak(tr.t("versions-hint"));
        let Some(list) = self.versions.ready() else {
            self.versions.status_ui(ui, tr, &tr.t("reading-backups"));
            return None;
        };
        let target = self.restore_target();
        egui::ScrollArea::vertical().id_salt("versions").show(ui, |ui| {
            for v in list {
                ui.add_space(theme::SPACE_S);
                ui.label(
                    RichText::new(tr.t_args("version-modified", &[("date", arg(v.modified_label.clone()))]))
                        .strong(),
                );
                ui.weak(tr.t_args(
                    "version-detail",
                    &[("backup", arg(v.backup_label.clone())), ("size", arg(tr.bytes(v.size)))],
                ));
                let ready = !busy && target.is_some();
                if ui
                    .add_enabled(ready, egui::Button::new(tr.t("restore-version")))
                    .on_disabled_hover_text(if busy { tr.t("busy-hint") } else { tr.t("choose-folder-hint") })
                    .clicked()
                    && let Some(t) = target.clone()
                {
                    request = Some(RestoreRequest {
                        archive: v.archive.clone(),
                        keys: vec![key.clone()],
                        target: t,
                        overwrite: self.overwrite,
                    });
                }
            }
        });
        request
    }

    fn options_ui(&mut self, ui: &mut Ui, tr: &Tr, busy: bool) -> Option<RestoreRequest> {
        ui.add_space(theme::SPACE_S);
        let mut chosen = self.target.unwrap_or(Target::Beside);
        ui.horizontal(|ui| {
            ui.label(tr.t("restore-to"));
            ui.radio_value(&mut chosen, Target::Beside, tr.t("target-beside"))
                .on_hover_text(tr.t("target-beside-hint"));
            ui.radio_value(&mut chosen, Target::Original, tr.t("target-original"));
            ui.radio_value(&mut chosen, Target::Folder, tr.t("target-folder"));
            if chosen == Target::Folder {
                let label = self.folder.as_ref().map_or_else(|| tr.t("browse"), |f| f.display().to_string());
                if ui.button(label).clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    self.folder = Some(dir);
                }
            }
        });
        self.target = Some(chosen);
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.overwrite, tr.t("replace-existing"));
            if self.overwrite && chosen == Target::Original {
                ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("replace-warning")));
            }
        });

        let label = self
            .archives
            .ready()
            .and_then(|l| l.iter().find(|a| Some(a.name.as_str()) == self.selected.as_deref()))
            .map(|a| a.time_label.clone())
            .unwrap_or_default();
        let target = self.restore_target();
        let mut request = None;
        ui.horizontal(|ui| {
            let ready = !busy && !self.checked.is_empty() && self.selected.is_some() && target.is_some();
            let why = if busy {
                tr.t("busy-hint")
            } else if self.checked.is_empty() {
                tr.t("tick-files-hint")
            } else {
                tr.t("choose-folder-hint")
            };
            let button = egui::Button::new(RichText::new(tr.t("restore-selected")).strong())
                .fill(ui.visuals().selection.bg_fill);
            if ui.add_enabled(ready, button).on_disabled_hover_text(why).clicked()
                && let (Some(archive), Some(t)) = (self.selected.clone(), target)
            {
                request = Some(RestoreRequest {
                    archive,
                    keys: self.checked.iter().cloned().collect(),
                    target: t,
                    overwrite: self.overwrite,
                });
            }
            if !self.checked.is_empty() {
                ui.label(
                    tr.t_args("selected-count", &[("count", arg(self.checked.len())), ("date", arg(label))]),
                );
                if ui.small_button(tr.t("clear-selection")).clicked() {
                    self.checked.clear();
                }
            }
        });
        ui.add_space(theme::SPACE_S);
        request
    }

    /// `None` while "another folder" has no folder yet.
    fn restore_target(&self) -> Option<RestoreTarget> {
        match self.target.unwrap_or(Target::Beside) {
            Target::Original => Some(RestoreTarget::Original),
            Target::Beside => Some(RestoreTarget::BesideOriginal),
            Target::Folder => self.folder.clone().map(|path| RestoreTarget::Folder { path }),
        }
    }
}

/// One node of the tree, its children inside a collapsible header.
fn node_ui(
    ui: &mut Ui,
    tr: &Tr,
    tree: &Tree,
    idx: usize,
    checked: &mut BTreeSet<String>,
    focused: Option<&str>,
    focus: &mut Option<String>,
) {
    let Some(n) = tree.node(idx) else { return };
    let mut on = checked.contains(&n.key);
    // Not saved by this backup: still there to browse and restore, greyed.
    let paint = |text: String, ui: &Ui| {
        let t = RichText::new(text);
        if n.stored { t } else { t.color(ui.visuals().weak_text_color()) }
    };
    // Folders with saved files open by themselves, unless everything was
    // saved (a complete backup), which would open the whole tree.
    let open = n.stored && tree.stored_files < tree.files;
    let toggle = |on: bool, checked: &mut BTreeSet<String>| {
        if on {
            checked.insert(n.key.clone());
        } else {
            checked.remove(&n.key);
        }
    };
    if n.kind == EntryKind::Dir {
        let id = ui.make_persistent_id(("tree", &n.key));
        CollapsingState::load_with_default_open(ui.ctx(), id, open)
            .show_header(ui, |ui| {
                if ui.checkbox(&mut on, "").on_hover_text(tr.t("tick-folder")).changed() {
                    toggle(on, checked);
                }
                let label = paint(format!("📁 {}", n.label), ui);
                let resp = ui.label(label);
                if !n.stored {
                    let _ = resp.on_hover_text(tr.t("tree-not-saved-folder"));
                }
            })
            .body(|ui| {
                for &c in &n.children {
                    node_ui(ui, tr, tree, c, checked, focused, focus);
                }
            });
    } else {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut on, "").on_hover_text(tr.t("tick-file")).changed() {
                toggle(on, checked);
            }
            let text = paint(format!("{}   {}", n.label, tr.bytes(n.size)), ui);
            let hint = if n.stored { tr.t("show-versions") } else { tr.t("tree-not-saved-file") };
            if ui.selectable_label(focused == Some(n.key.as_str()), text).on_hover_text(hint).clicked() {
                *focus = Some(n.key.clone());
            }
        });
    }
}
