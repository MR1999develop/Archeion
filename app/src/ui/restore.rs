//! Backups tab, in two views.
//!
//! "Restore files" (the default): one folder tree of every file the
//! profile's backups ever saved, with search above it; files deleted at the
//! source since the newest backup are marked. On the right: where the
//! selection goes, its summary and the Restore button. A ticked file comes
//! back as its newest version, a ticked folder as it was in the newest
//! backup listing it, with everything it held then; the folders above a
//! ticked item show a filled box. The decisions are in
//! [`crate::restore_plan`].
//!
//! "Manage backups": the list of dates with the manual merge and the
//! deletion of one backup ([`super::backups`]).
//!
//! An encrypted destination is read with the password saved in Credential
//! Manager. Every query reads manifests on a background thread; the tree is
//! kept per profile until the archives change. The restore itself runs in
//! the worker process like a backup.

use super::{
    backups::BackupsView,
    load::{Load, Problem},
    symbol, theme,
    tree::{Node, Tick, Ticked, Tree},
    widgets,
};
use crate::{
    credentials,
    i18n::{Tr, arg},
    restore_plan::{self, Blocker, DefaultDest, DestKind, Pick, Plan, Version},
};
use archeion_core::{EntryKind, Profile, ProfileId, keys};
use archeion_engine::{FileVersion, RestoreRequest};
use eframe::egui::{self, Context, RichText, Ui, collapsing_header::CollapsingState};
use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Typing pause before a search runs.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);
const SEARCH_LIMIT: usize = 200;
/// Starting width of the selection panel.
const SELECTION_WIDTH: f32 = 380.0;

/// Key and bytes of each file of a folder at one date: its size is the sum
/// of those not unticked.
type FolderFiles = Vec<(String, u64)>;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum View {
    #[default]
    Files,
    Backups,
}

#[derive(Debug, Default)]
pub struct RestoreView {
    profile: Option<ProfileId>,
    view: View,
    /// Every file ever saved, per profile, with the destination it was
    /// read from.
    trees: HashMap<ProfileId, (PathBuf, Load<Tree>)>,
    query: String,
    query_edited: Option<Instant>,
    /// Nodes matching the query, once searched.
    hits: Option<Vec<usize>>,
    /// The selection, in the order things were ticked.
    picks: Vec<Pick>,
    /// Versions of the ticked files, by key.
    versions: HashMap<String, Load<Vec<FileVersion>>>,
    /// Files of a ticked folder at a date, by key and archive.
    folder_sizes: HashMap<(String, String), Load<FolderFiles>>,
    /// Kept when the archives change and across profiles, like a folder
    /// chosen in a file dialog.
    default: DefaultDest,
    backups: BackupsView,
}

impl RestoreView {
    /// Forget everything read, e.g. after a backup, merge or deletion
    /// changed the archives. The default destination and the view stay.
    pub fn invalidate(&mut self) {
        let (default, view) = (std::mem::take(&mut self.default), self.view);
        *self = Self { default, view, ..Self::default() };
    }

    /// Shows the "Restore files" view next time the tab is drawn.
    pub fn open_files(&mut self) {
        self.view = View::Files;
    }

    /// Shows the "Manage backups" view (tests).
    #[cfg(test)]
    pub fn open_manage(&mut self) {
        self.view = View::Backups;
    }

    /// Draws the tab; returns a restore, merge or deletion to start.
    pub fn show(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, busy: bool) -> Option<Action> {
        if self.profile != Some(profile.id) {
            self.switch_to(profile.id);
        }
        let items = [
            (View::Files, tr.t("backups-restore-files"), Some(tr.t("backups-restore-files-hint"))),
            (View::Backups, tr.t("backups-manage"), Some(tr.t("backups-manage-hint"))),
        ];
        widgets::tabs(ui, &mut self.view, &items, true);
        ui.add_space(theme::SPACE_S);
        match self.view {
            View::Files => self.files_view(ui, tr, profile, busy),
            View::Backups => {
                let mut refresh = false;
                let action = self.backups.show(ui, tr, profile, busy, &mut refresh);
                if refresh {
                    self.invalidate();
                }
                action
            }
        }
    }

    /// Another profile: its own selection. The trees read so far stay,
    /// except those that failed: a password may have been typed since.
    fn switch_to(&mut self, id: ProfileId) {
        self.profile = Some(id);
        self.trees.retain(|_, (_, t)| !matches!(t, Load::Failed(_)));
        self.picks.clear();
        self.versions.clear();
        self.folder_sizes.clear();
        self.query.clear();
        self.query_edited = None;
        self.hits = None;
        self.backups = BackupsView::default();
    }

    fn files_view(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, busy: bool) -> Option<Action> {
        let ctx = ui.ctx().clone();
        let stopped = tr.t("unexpected-stop");
        let destination = profile.destination.path();
        let (read_from, tree) = self.trees.entry(profile.id).or_default();
        if read_from != destination {
            // The destination was changed in the settings.
            *read_from = destination.to_path_buf();
            *tree = Load::Idle;
        }
        tree.poll(&stopped);
        if matches!(tree, Load::Idle) {
            let p = profile.clone();
            *tree = Load::spawn(&ctx, move || {
                let pw = credentials::get_or_none(p.id);
                archeion_engine::catalog(&p, pw.as_ref()).map(Tree::build).map_err(Problem::from)
            });
        }
        for l in self.versions.values_mut() {
            l.poll(&stopped);
        }
        for l in self.folder_sizes.values_mut() {
            l.poll(&stopped);
        }
        self.sync_picks();
        self.run_search_when_idle(&ctx, profile.id);

        let mut action = None;
        egui::Panel::right("restore-selection").resizable(true).default_size(SELECTION_WIDTH).show(
            ui,
            |ui| {
                if let Some(r) = self.selection_ui(ui, tr, busy) {
                    action = Some(Action::Restore(r));
                }
            },
        );
        egui::CentralPanel::default().show(ui, |ui| self.tree_ui(ui, tr, profile));
        action
    }

    /// Brings the selection up to date with what was read: a file takes its
    /// newest version when its versions arrive, a folder learns its size.
    fn sync_picks(&mut self) {
        for pick in &mut self.picks {
            match pick {
                Pick::File { key, loading, versions } if *loading => match self.versions.get(key.as_str()) {
                    Some(Load::Ready(list)) => {
                        *loading = false;
                        if let Some(v) = list.first() {
                            versions.push(Version { archive: v.archive.clone(), size: v.size });
                        }
                    }
                    Some(Load::Failed(_)) | None => *loading = false,
                    Some(Load::Idle | Load::Loading(_)) => {}
                },
                Pick::File { .. } => {}
                Pick::Folder { key, archive, size, exclude, .. } => {
                    let read = self.folder_sizes.get(&(key.clone(), archive.clone()));
                    *size = read.and_then(Load::ready).map(|files| {
                        files
                            .iter()
                            .filter(|(k, _)| !exclude.iter().any(|x| keys::is_within(k, x)))
                            .fold(0u64, |sum, (_, b)| sum.saturating_add(*b))
                    });
                }
            }
        }
    }

    fn run_search_when_idle(&mut self, ctx: &Context, id: ProfileId) {
        let Some(edited) = self.query_edited else { return };
        if edited.elapsed() < SEARCH_DEBOUNCE {
            ctx.request_repaint_after(SEARCH_DEBOUNCE.saturating_sub(edited.elapsed()));
            return;
        }
        // Searched once the tree is there.
        let Some(tree) = self.trees.get(&id).and_then(|(_, t)| t.ready()) else { return };
        let hits = (!self.query.trim().is_empty()).then(|| tree.search(&self.query, SEARCH_LIMIT));
        self.hits = hits;
        self.query_edited = None;
    }

    /// Ticks or unticks the node `idx`. A ticked file reads its versions,
    /// a ticked folder comes back as it was in the newest backup listing it
    /// and takes the place of what was ticked inside it. Inside a ticked
    /// folder a click leaves the item out of it, or ticks it again; a
    /// folder with something left out is ticked whole again first.
    fn toggle(&mut self, ctx: &Context, profile: &Profile, idx: usize) {
        let Some(tree) = self.trees.get(&profile.id).and_then(|(_, t)| t.ready()) else { return };
        let Some(node) = tree.node(idx) else { return };
        let key = node.key().to_owned();
        if let Some(pos) = self.picks.iter().position(|p| p.key() == key) {
            match self.picks.get_mut(pos) {
                Some(Pick::Folder { exclude, .. }) if !exclude.is_empty() => exclude.clear(),
                _ => {
                    self.picks.remove(pos);
                }
            }
            return;
        }
        let holder = self
            .picks
            .iter_mut()
            .filter(|p| matches!(p, Pick::Folder { .. }) && keys::is_within(&key, p.key()))
            .max_by_key(|p| p.key().len());
        if let Some(Pick::Folder { exclude, .. }) = holder {
            if exclude.iter().any(|x| keys::is_within(&key, x)) {
                tree.include(exclude, &key);
            } else if exclude.iter().any(|x| keys::is_within(x, &key)) {
                exclude.retain(|x| !keys::is_within(x, &key));
            } else {
                exclude.push(key);
            }
            return;
        }
        // Everything inside ticked one by one: the click unticks it all.
        let now: Vec<Ticked<'_>> = self.picks.iter().map(ticked).collect();
        if tree.tick(node, &now) == Tick::Full {
            self.picks.retain(|p| !keys::is_within(p.key(), &key));
            return;
        }
        if node.is_dir() {
            let Some(newest) = tree.dates(node).first().map(|a| a.name.clone()) else { return };
            spawn_folder_size(&mut self.folder_sizes, ctx, profile, &key, &newest);
            self.picks.retain(|p| !keys::is_within(p.key(), &key));
            self.picks.push(Pick::Folder { key, archive: newest, size: None, exclude: Vec::new() });
        } else {
            if self.versions.get(&key).is_none_or(|l| matches!(l, Load::Failed(_))) {
                let (p, k) = (profile.clone(), key.clone());
                let load = Load::spawn(ctx, move || {
                    let pw = credentials::get_or_none(p.id);
                    archeion_engine::file_versions(&p, pw.as_ref(), &k).map_err(Problem::from)
                });
                self.versions.insert(key.clone(), load);
            }
            self.picks.push(Pick::File { key, loading: true, versions: Vec::new() });
        }
    }

    fn tree_ui(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile) {
        let mut refresh = false;
        ui.horizontal(|ui| {
            refresh = widgets::icon_button(ui, symbol::REFRESH, tr.t("refresh")).clicked();
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(tr.t("search-hint"))
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                self.query_edited = Some(Instant::now());
            }
        });
        if refresh {
            // Read the backups again; the selection stays.
            self.trees.remove(&profile.id);
            self.versions.retain(|_, l| !matches!(l, Load::Failed(_)));
            self.folder_sizes.retain(|_, l| !matches!(l, Load::Failed(_)));
            self.hits = None;
            self.query_edited = Some(Instant::now());
            return;
        }
        ui.add_space(theme::SPACE_S);
        let mut toggled: Vec<usize> = Vec::new();
        match self.trees.get(&profile.id).map(|(_, t)| t) {
            Some(Load::Ready(tree)) => {
                let ticked: Vec<Ticked<'_>> = self.picks.iter().map(ticked).collect();
                if tree.files == 0 {
                    ui.weak(tr.t("no-backups-yet"));
                } else if self.query.trim().is_empty() {
                    tree_list_ui(ui, tr, tree, &ticked, &mut toggled);
                } else {
                    hits_ui(ui, tr, tree, self.hits.as_deref(), &ticked, &mut toggled);
                }
            }
            Some(other) => other.status_ui(ui, tr, &tr.t("reading-backups")),
            None => {}
        }
        let ctx = ui.ctx().clone();
        for idx in toggled {
            self.toggle(&ctx, profile, idx);
        }
    }

    /// Right panel: destination, summary and Restore.
    fn selection_ui(&mut self, ui: &mut Ui, tr: &Tr, busy: bool) -> Option<RestoreRequest> {
        let plan = restore_plan::plan(&self.picks, &self.default, busy);
        egui::Panel::top("restore-default").show(ui, |ui| default_ui(ui, tr, &mut self.default));
        let mut request = None;
        egui::Panel::bottom("restore-go").show(ui, |ui| {
            let (go, clear) = go_ui(ui, tr, &plan, !self.picks.is_empty());
            request = go;
            if clear {
                self.picks.clear();
            }
            ui.add_space(theme::SPACE_S);
        });
        egui::CentralPanel::default().show(ui, |ui| selection_note(ui, tr, self.picks.is_empty(), &plan));
        if request.is_some() {
            // Started: the Activity tab shows it; the selection is done.
            self.picks.clear();
        }
        request
    }
}

/// Reads the files of folder `key` as it was in `archive`, unless already
/// read or being read.
fn spawn_folder_size(
    sizes: &mut HashMap<(String, String), Load<FolderFiles>>,
    ctx: &Context,
    profile: &Profile,
    key: &str,
    archive: &str,
) {
    let slot = (key.to_owned(), archive.to_owned());
    if sizes.get(&slot).is_some_and(|l| !matches!(l, Load::Failed(_))) {
        return;
    }
    let (p, k, a) = (profile.clone(), key.to_owned(), archive.to_owned());
    let load = Load::spawn(ctx, move || {
        let pw = credentials::get_or_none(p.id);
        let m = archeion_engine::archive_manifest(&p, pw.as_ref(), &a).map_err(Problem::from)?;
        let files =
            m.entries.into_iter().filter(|e| e.kind == EntryKind::File && keys::is_within(&e.key, &k));
        Ok(files.map(|e| (e.key, e.size)).collect())
    });
    sizes.insert(slot, load);
}

/// What the tree needs to know of a card.
fn ticked(pick: &Pick) -> Ticked<'_> {
    match pick {
        Pick::File { key, .. } => Ticked { key, folder_archive: None, exclude: &[] },
        Pick::Folder { key, archive, exclude, .. } => Ticked { key, folder_archive: Some(archive), exclude },
    }
}

/// The whole tree, with its counts.
fn tree_list_ui(ui: &mut Ui, tr: &Tr, tree: &Tree, ticked: &[Ticked<'_>], toggled: &mut Vec<usize>) {
    ui.weak(
        tr.t_args("restore-tree-count", &[("files", arg(tree.files)), ("deleted", arg(tree.deleted_files))]),
    );
    ui.weak(tr.t_args("restore-tree-hint", &symbol::args()));
    egui::ScrollArea::vertical().id_salt("tree").auto_shrink(false).show(ui, |ui| {
        for &r in &tree.roots {
            node_ui(ui, tr, tree, r, ticked, toggled);
        }
    });
}

/// Search results: tickable like the tree, with the full path.
fn hits_ui(
    ui: &mut Ui,
    tr: &Tr,
    tree: &Tree,
    hits: Option<&[usize]>,
    ticked: &[Ticked<'_>],
    toggled: &mut Vec<usize>,
) {
    let Some(hits) = hits else { return };
    if hits.is_empty() {
        ui.weak(tr.t("no-results"));
        return;
    }
    ui.weak(tr.t_args("results", &[("count", arg(hits.len()))]));
    egui::ScrollArea::vertical().id_salt("hits").auto_shrink(false).show(ui, |ui| {
        for &idx in hits {
            let Some(n) = tree.node(idx) else { continue };
            let path = keys::path_for(n.key()).display().to_string();
            let base = if n.is_dir() {
                symbol::with(symbol::FOLDER, path)
            } else {
                format!("{}  {path}   {}", symbol::FILE, tr.bytes(n.entry.size))
            };
            let text = node_text(ui, tr, n, base);
            if tick_box(ui, tr, tree, n, tree.tick(n, ticked), text) {
                toggled.push(idx);
            }
        }
    });
}

/// One node of the tree, its children inside a collapsible header. Only
/// the roots start open.
fn node_ui(ui: &mut Ui, tr: &Tr, tree: &Tree, idx: usize, ticked: &[Ticked<'_>], toggled: &mut Vec<usize>) {
    let Some(n) = tree.node(idx) else { return };
    let tick = tree.tick(n, ticked);
    if n.is_dir() {
        let id = ui.make_persistent_id(("tree", n.key()));
        let open = tree.roots.contains(&idx);
        CollapsingState::load_with_default_open(ui.ctx(), id, open)
            .show_header(ui, |ui| {
                let text = node_text(ui, tr, n, symbol::with(symbol::FOLDER, &n.label));
                if tick_box(ui, tr, tree, n, tick, text) {
                    toggled.push(idx);
                }
            })
            .body(|ui| {
                for &c in &n.children {
                    node_ui(ui, tr, tree, c, ticked, toggled);
                }
            });
    } else {
        let text =
            node_text(ui, tr, n, format!("{}  {}   {}", symbol::FILE, n.label, tr.bytes(n.entry.size)));
        if tick_box(ui, tr, tree, n, tick, text) {
            toggled.push(idx);
        }
    }
}

/// The tick box of a node; true when clicked. Inside a ticked folder an
/// item is ticked with it and can be unticked (left out); one the folder's
/// date lacks is greyed. A folder with something ticked inside, or a
/// ticked folder with something left out, shows a filled box; clicking it
/// ticks the whole folder.
fn tick_box(ui: &mut Ui, tr: &Tr, tree: &Tree, n: &Node, tick: Tick, text: RichText) -> bool {
    let folder_path = |folder: usize| {
        let path = tree.node(folder).map(|f| keys::path_for(f.key()).display().to_string());
        arg(path.unwrap_or_default())
    };
    match tick {
        Tick::NotInFolderDate { folder } => {
            let hint = tr.t_args("tick-not-in-folder-date", &[("folder", folder_path(folder))]);
            ui.add_enabled(false, egui::Checkbox::new(&mut false, text)).on_disabled_hover_text(hint);
            false
        }
        Tick::InFolder { folder } | Tick::LeftOut { folder } => {
            let mut on = matches!(tick, Tick::InFolder { .. });
            let id = if on { "tick-in-folder" } else { "tick-left-out" };
            let hint = tr.t_args(id, &[("folder", folder_path(folder))]);
            widgets::checkbox(ui, &mut on, text).on_hover_text(hint).changed()
        }
        Tick::Partial => {
            let resp = widgets::partial_checkbox(ui, text);
            resp.on_hover_text(format!("{}\n{}", tr.t("tick-partial"), node_hint(tr, n))).changed()
        }
        Tick::Full => {
            let mut on = true;
            let hint = format!(
                "{}
{}",
                tr.t("tick-full"),
                node_hint(tr, n)
            );
            widgets::checkbox(ui, &mut on, text).on_hover_text(hint).changed()
        }
        Tick::On | Tick::Off => {
            let mut on = tick == Tick::On;
            widgets::checkbox(ui, &mut on, text).on_hover_text(node_hint(tr, n)).changed()
        }
    }
}

/// A deleted node says so in words and with an icon, and is greyed.
fn node_text(ui: &Ui, tr: &Tr, n: &Node, base: String) -> RichText {
    if n.deleted {
        RichText::new(format!("{base}   {}  {}", symbol::DELETED, tr.t("deleted-label")))
            .color(ui.visuals().weak_text_color())
    } else {
        RichText::new(base)
    }
}

fn node_hint(tr: &Tr, n: &Node) -> String {
    tr.t(match (n.is_dir(), n.deleted) {
        (true, false) => "tick-folder",
        (true, true) => "tree-deleted-folder",
        (false, false) => "tick-file",
        (false, true) => "tree-deleted-file",
    })
}

/// Under the destination: what to do when nothing is ticked, or why two
/// ticked items cannot both replace their original.
fn selection_note(ui: &mut Ui, tr: &Tr, empty: bool, plan: &Plan) {
    ui.add_space(theme::SPACE_S);
    if empty {
        ui.weak(tr.t("selection-empty"));
    } else if !plan.conflicts.is_empty() {
        ui.colored_label(
            ui.visuals().error_fg_color,
            symbol::with(symbol::WARNING, tr.t("restore-conflict")),
        );
    }
}

/// The destination of the whole selection, at its top.
fn default_ui(ui: &mut Ui, tr: &Tr, default: &mut DefaultDest) {
    ui.add_space(theme::SPACE_S);
    widgets::section_title(ui, tr.t("restore-to"));
    ui.weak(tr.t("restore-to-hint"));
    ui.radio_value(&mut default.kind, DestKind::Beside, tr.t("target-beside"))
        .on_hover_text(tr.t("target-beside-hint"));
    ui.radio_value(&mut default.kind, DestKind::Original, tr.t("target-original"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut default.kind, DestKind::Folder, tr.t("target-folder"));
        if default.kind == DestKind::Folder {
            let label = default.folder.as_ref().map_or_else(|| tr.t("browse"), |f| f.display().to_string());
            if ui.button(label).clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                default.folder = Some(dir);
            }
        }
    });
    if default.kind != DestKind::Beside {
        widgets::checkbox(ui, &mut default.replace, tr.t("replace-existing"));
        if default.replace && default.kind == DestKind::Original {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("replace-warning")),
            );
        }
    }
    ui.add_space(theme::SPACE_S);
}

/// Summary, the Restore button with "Clear selection" beside it, and the
/// reason Restore waits. Returns the request and whether to clear.
fn go_ui(ui: &mut Ui, tr: &Tr, plan: &Plan, any: bool) -> (Option<RestoreRequest>, bool) {
    ui.add_space(theme::SPACE_S);
    let s = plan.summary;
    let mut size = tr.bytes(s.bytes);
    if s.partial {
        size.push('…');
    }
    ui.label(tr.t_args(
        "restore-summary",
        &[("files", arg(s.files)), ("folders", arg(s.folders)), ("size", arg(size))],
    ));
    let why = plan.blocker.map(|b| {
        tr.t(match b {
            Blocker::Nothing => "tick-files-hint",
            Blocker::Loading => "restore-wait-versions",
            Blocker::NoFolder => "choose-folder-hint",
            Blocker::Conflict => "restore-conflict-hint",
            Blocker::Busy => "busy-hint",
        })
    });
    let (resp, clear) = ui
        .horizontal(|ui| {
            let resp = ui.add_enabled(why.is_none(), widgets::primary_button(ui, tr.t("restore-selected")));
            let clear = ui.add_enabled(any, egui::Button::new(tr.t("clear-selection"))).clicked();
            (resp, clear)
        })
        .inner;
    let mut request = None;
    match &why {
        Some(w) => {
            let _ = resp.on_disabled_hover_text(w);
            ui.weak(w);
        }
        None if resp.clicked() => request = Some(plan.request.clone()),
        None => {}
    }
    (request, clear)
}

#[cfg(test)]
#[path = "restore_tests.rs"]
mod tests;
