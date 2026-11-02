//! Backups tab, in two views.
//!
//! "Restore files" (the default): one folder tree of every file the
//! profile's backups ever saved, with search above it; files deleted at the
//! source since the newest backup are marked. On the right, the selection:
//! one card per ticked file (its versions, the newest ticked) or folder
//! (the date it comes back as), each version or folder with its
//! destination, the default destination at the top and the Restore button
//! at the bottom. A ticked folder ticks everything it held at its date;
//! the folders above a ticked item show a filled box. With several cards,
//! only the highlighted one (the last ticked, or the last clicked) shows
//! its options; the others show one line. The decisions are in
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
    theme,
    tree::{Node, Tick, Ticked, Tree},
};
use crate::{
    credentials,
    i18n::{Tr, arg},
    restore_plan::{self, Blocker, DefaultDest, Dest, DestKind, Pick, Plan, Version},
};
use archeion_core::{EntryKind, Profile, ProfileId, keys};
use archeion_engine::{FileVersion, RestoreRequest};
use eframe::egui::{self, Context, RichText, Ui, collapsing_header::CollapsingState};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
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
    /// Key of the highlighted card, the one showing its options.
    focused: Option<String>,
    /// Versions of the ticked files, by key.
    versions: HashMap<String, Load<Vec<FileVersion>>>,
    /// Files of a ticked folder at a date, by key and archive.
    folder_sizes: HashMap<(String, String), Load<FolderFiles>>,
    /// Kept when the archives change and across profiles, like a folder
    /// chosen in a file dialog.
    default: DefaultDest,
    backups: BackupsView,
}

/// What a click in a card asks for, applied once the cards are drawn.
#[derive(Debug)]
enum CardEdit {
    Remove(usize),
    /// Highlight this card.
    Focus(usize),
    /// Tick or untick the version at this position of the file's list.
    Version {
        card: usize,
        index: usize,
        on: bool,
    },
    /// A folder comes back as it was in this archive.
    Date {
        card: usize,
        archive: String,
    },
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

    /// Draws the tab; returns a restore, merge or deletion to start.
    pub fn show(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, busy: bool) -> Option<Action> {
        if self.profile != Some(profile.id) {
            self.switch_to(profile.id);
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.view, View::Files, tr.t("backups-restore-files"))
                .on_hover_text(tr.t("backups-restore-files-hint"));
            ui.selectable_value(&mut self.view, View::Backups, tr.t("backups-manage"))
                .on_hover_text(tr.t("backups-manage-hint"));
        });
        ui.separator();
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
        self.focused = None;
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
                if let Some(r) = self.selection_ui(ui, tr, profile, busy) {
                    action = Some(Action::Restore(r));
                }
            },
        );
        egui::CentralPanel::default().show(ui, |ui| self.tree_ui(ui, tr, profile));
        action
    }

    /// Brings the cards up to date with what was read: the newest version
    /// of a file is ticked when its versions arrive, a folder learns its
    /// size.
    fn sync_picks(&mut self) {
        for pick in &mut self.picks {
            match pick {
                Pick::File { key, loading, versions } if *loading => match self.versions.get(key.as_str()) {
                    Some(Load::Ready(list)) => {
                        *loading = false;
                        if let Some(v) = list.first() {
                            versions.push(Version { archive: v.archive.clone(), size: v.size, dest: None });
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
    /// folder with something left out is ticked whole again first. What is
    /// ticked becomes the highlighted card.
    fn toggle(&mut self, ctx: &Context, profile: &Profile, idx: usize) {
        let Some(tree) = self.trees.get(&profile.id).and_then(|(_, t)| t.ready()) else { return };
        let Some(node) = tree.node(idx) else { return };
        let key = node.key().to_owned();
        if let Some(pos) = self.picks.iter().position(|p| p.key() == key) {
            match self.picks.get_mut(pos) {
                Some(Pick::Folder { exclude, .. }) if !exclude.is_empty() => exclude.clear(),
                _ => {
                    self.picks.remove(pos);
                    if self.focused.as_deref() == Some(key.as_str()) {
                        self.focused = None;
                    }
                }
            }
            return;
        }
        let holder = self
            .picks
            .iter_mut()
            .filter(|p| matches!(p, Pick::Folder { .. }) && keys::is_within(&key, p.key()))
            .max_by_key(|p| p.key().len());
        if let Some(Pick::Folder { key: folder, exclude, .. }) = holder {
            if exclude.iter().any(|x| keys::is_within(&key, x)) {
                tree.include(exclude, &key);
            } else if exclude.iter().any(|x| keys::is_within(x, &key)) {
                exclude.retain(|x| !keys::is_within(x, &key));
            } else {
                exclude.push(key);
            }
            self.focused = Some(folder.clone());
            return;
        }
        self.focused = Some(key.clone());
        if node.is_dir() {
            let Some(newest) = tree.dates(node).first().map(|a| a.name.clone()) else { return };
            spawn_folder_size(&mut self.folder_sizes, ctx, profile, &key, &newest);
            self.picks.retain(|p| !keys::is_within(p.key(), &key));
            self.picks.push(Pick::Folder {
                key,
                archive: newest,
                dest: None,
                size: None,
                exclude: Vec::new(),
            });
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
            refresh = ui.small_button("⟳").on_hover_text(tr.t("refresh")).clicked();
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

    /// Right panel: default destination, cards, summary and Restore.
    fn selection_ui(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        profile: &Profile,
        busy: bool,
    ) -> Option<RestoreRequest> {
        let plan = restore_plan::plan(&self.picks, &self.default, busy);
        egui::Panel::top("restore-default").show(ui, |ui| default_ui(ui, tr, &mut self.default));
        let mut request = None;
        egui::Panel::bottom("restore-go").show(ui, |ui| {
            request = go_ui(ui, tr, &plan);
            if !self.picks.is_empty() && ui.link(tr.t("clear-selection")).clicked() {
                self.picks.clear();
                self.focused = None;
            }
            ui.add_space(theme::SPACE_S);
        });
        egui::CentralPanel::default().show(ui, |ui| self.cards_ui(ui, tr, profile, &plan));
        if request.is_some() {
            // Started: the Activity tab shows it; the selection is done.
            self.picks.clear();
            self.focused = None;
        }
        request
    }

    fn cards_ui(&mut self, ui: &mut Ui, tr: &Tr, profile: &Profile, plan: &Plan) {
        if self.picks.is_empty() {
            ui.add_space(theme::SPACE_S);
            ui.weak(tr.t("selection-empty"));
            return;
        }
        let tree = self.trees.get(&profile.id).and_then(|(_, t)| t.ready());
        let several = self.picks.len() > 1;
        let mut edits: Vec<CardEdit> = Vec::new();
        egui::ScrollArea::vertical().id_salt("cards").auto_shrink(false).show(ui, |ui| {
            for (card, pick) in self.picks.iter_mut().enumerate() {
                let focused = !several || self.focused.as_deref() == Some(pick.key());
                let mut frame = egui::Frame::new().inner_margin(theme::SPACE_S).corner_radius(theme::RADIUS);
                if several && focused {
                    frame = frame.fill(ui.visuals().faint_bg_color).stroke(ui.visuals().selection.stroke);
                }
                frame.show(ui, |ui| {
                    ui.push_id(("card", pick.key().to_owned()), |ui| {
                        let node = tree.and_then(|t| t.find(pick.key()));
                        match card_header(ui, tr, pick.key(), node, several.then_some(focused)) {
                            HeaderClick::Remove => edits.push(CardEdit::Remove(card)),
                            HeaderClick::Focus => edits.push(CardEdit::Focus(card)),
                            HeaderClick::None => {}
                        }
                        let dates = match (tree, node) {
                            (Some(t), Some(n)) => t.dates(n),
                            _ => Vec::new(),
                        };
                        if focused {
                            card_body(
                                ui,
                                tr,
                                &self.default,
                                plan,
                                card,
                                pick,
                                &self.versions,
                                &self.folder_sizes,
                                &dates,
                                &mut edits,
                            );
                        } else {
                            let line = RichText::new(card_summary(tr, &self.default, pick, &dates)).weak();
                            let resp = ui.add(egui::Label::new(line).sense(egui::Sense::click()));
                            if resp.on_hover_text(tr.t("card-show-options")).clicked() {
                                edits.push(CardEdit::Focus(card));
                            }
                        }
                        if plan.conflicts.contains(&card) {
                            ui.colored_label(
                                ui.visuals().error_fg_color,
                                format!("⚠ {}", tr.t("restore-conflict")),
                            );
                        }
                    });
                });
                ui.add_space(theme::SPACE_XS);
            }
        });
        self.apply(ui.ctx(), profile, edits);
    }

    fn apply(&mut self, ctx: &Context, profile: &Profile, edits: Vec<CardEdit>) {
        let mut removed: Vec<usize> = Vec::new();
        for edit in edits {
            match edit {
                CardEdit::Remove(card) => removed.push(card),
                CardEdit::Focus(card) => self.focused = self.picks.get(card).map(|p| p.key().to_owned()),
                CardEdit::Version { card, index, on } => {
                    let Some(Pick::File { key, versions, .. }) = self.picks.get_mut(card) else { continue };
                    let Some(list) = self.versions.get(key.as_str()).and_then(Load::ready) else { continue };
                    let Some(v) = list.get(index) else { continue };
                    if on {
                        // Ticked versions stay newest first, like the list.
                        let position = |archive: &str| list.iter().position(|l| l.archive == archive);
                        let at = versions.iter().filter(|t| position(&t.archive) < Some(index)).count();
                        versions.insert(at, Version { archive: v.archive.clone(), size: v.size, dest: None });
                    } else {
                        versions.retain(|t| t.archive != v.archive);
                    }
                }
                CardEdit::Date { card, archive } => {
                    let Some(Pick::Folder { key, archive: current, size, .. }) = self.picks.get_mut(card)
                    else {
                        continue;
                    };
                    spawn_folder_size(&mut self.folder_sizes, ctx, profile, key, &archive);
                    *current = archive;
                    *size = None;
                }
            }
        }
        removed.sort_unstable();
        for card in removed.into_iter().rev() {
            if card < self.picks.len() {
                let pick = self.picks.remove(card);
                if self.focused.as_deref() == Some(pick.key()) {
                    self.focused = None;
                }
            }
        }
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
    ui.weak(tr.t("restore-tree-hint"));
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
            let text = node_text(ui, tr, n, format!("📄 {path}   {}", tr.bytes(n.entry.size)));
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
                let text = node_text(ui, tr, n, format!("📁 {}", n.label));
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
        let text = node_text(ui, tr, n, format!("📄 {}   {}", n.label, tr.bytes(n.entry.size)));
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
            ui.checkbox(&mut on, text).on_hover_text(hint).changed()
        }
        Tick::Partial => {
            let mut on = false;
            let resp = ui.add(egui::Checkbox::new(&mut on, text).indeterminate(true));
            fill_box(ui, &resp);
            resp.on_hover_text(format!("{}\n{}", tr.t("tick-partial"), node_hint(tr, n))).changed()
        }
        Tick::On | Tick::Off => {
            let mut on = tick == Tick::On;
            ui.checkbox(&mut on, text).on_hover_text(node_hint(tr, n)).changed()
        }
    }
}

/// Fills the box of a checkbox, over egui's dash for "indeterminate".
fn fill_box(ui: &Ui, resp: &egui::Response) {
    let s = ui.spacing();
    let center = egui::pos2(resp.rect.left() + s.icon_width * 0.5, resp.rect.center().y);
    let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(s.icon_width_inner));
    ui.painter().rect_filled(rect, 2.0, ui.style().interact(resp).fg_stroke.color);
}

/// A deleted node says so in words and with an icon, and is greyed.
fn node_text(ui: &Ui, tr: &Tr, n: &Node, base: String) -> RichText {
    if n.deleted {
        RichText::new(format!("{base}   🗑 {}", tr.t("deleted-label"))).color(ui.visuals().weak_text_color())
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

/// What a click in a card header asks for.
enum HeaderClick {
    None,
    Remove,
    Focus,
}

/// Name, folder and ✕ of a card. With several cards (`focused` is `Some`)
/// the name is selectable: clicking it highlights the card.
fn card_header(ui: &mut Ui, tr: &Tr, key: &str, node: Option<&Node>, focused: Option<bool>) -> HeaderClick {
    let dir = node.is_some_and(Node::is_dir);
    let name = key.rsplit('/').next().unwrap_or(key);
    let path = keys::path_for(key);
    let mut click = HeaderClick::None;
    ui.horizontal(|ui| {
        let icon = if dir { "📁" } else { "📄" };
        let title = RichText::new(format!("{icon} {name}")).strong();
        let hover = path.display().to_string();
        match focused {
            Some(on) => {
                let hover = format!("{hover}\n{}", tr.t("card-show-options"));
                if ui.selectable_label(on, title).on_hover_text(hover).clicked() {
                    click = HeaderClick::Focus;
                }
            }
            None => {
                ui.label(title).on_hover_text(hover);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = tr.t("remove-from-selection");
            let resp = ui.small_button("✕").on_hover_text(&label);
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            if resp.clicked() {
                click = HeaderClick::Remove;
            }
        });
    });
    if let Some(parent) = path.parent() {
        ui.weak(parent.display().to_string());
    }
    if node.is_some_and(|n| n.deleted) {
        ui.weak(format!("🗑 {}", tr.t("deleted-since")));
    }
    click
}

/// The options of the highlighted card: versions or date, and destination.
#[expect(clippy::too_many_arguments, reason = "one card: its parts are drawn together")]
fn card_body(
    ui: &mut Ui,
    tr: &Tr,
    default: &DefaultDest,
    plan: &Plan,
    card: usize,
    pick: &mut Pick,
    versions: &HashMap<String, Load<Vec<FileVersion>>>,
    folder_sizes: &HashMap<(String, String), Load<FolderFiles>>,
    dates: &[&archeion_engine::CatalogArchive],
    edits: &mut Vec<CardEdit>,
) {
    match pick {
        Pick::File { key, versions: ticked, .. } => {
            file_card(ui, tr, default, plan, card, versions.get(key.as_str()), ticked, edits);
        }
        Pick::Folder { key, archive, dest, size, exclude } => {
            folder_card(ui, tr, card, archive, dates, edits);
            match folder_sizes.get(&(key.clone(), archive.clone())) {
                Some(Load::Ready(_)) => {
                    let bytes = tr.bytes(size.unwrap_or_default());
                    ui.weak(tr.t_args("folder-size", &[("size", arg(bytes))]));
                }
                Some(other) => other.status_ui(ui, tr, &tr.t("reading-backups")),
                None => {}
            }
            if !exclude.is_empty() {
                ui.weak(tr.t_args("folder-left-out", &[("count", arg(exclude.len()))]));
            }
            dest_ui(ui, tr, dest, default, None);
        }
    }
}

/// One line for a card that is not highlighted: what it restores and where.
fn card_summary(
    tr: &Tr,
    default: &DefaultDest,
    pick: &Pick,
    dates: &[&archeion_engine::CatalogArchive],
) -> String {
    let dest_of = |dest: Option<&Dest>| match dest {
        None => default_text(tr, default),
        Some(d) => format!("✎ {}", dest_text(tr, d)),
    };
    match pick {
        Pick::File { versions, .. } => {
            let dest = match versions.split_first() {
                None => default_text(tr, default),
                Some((first, rest)) if rest.iter().all(|v| v.dest == first.dest) => {
                    dest_of(first.dest.as_ref())
                }
                Some(_) => format!("✎ {}", tr.t("dest-several")),
            };
            tr.t_args("card-summary-file", &[("versions", arg(versions.len())), ("dest", arg(dest))])
        }
        Pick::Folder { archive, dest, .. } => {
            let date = dates
                .iter()
                .find(|a| a.name == *archive)
                .map_or_else(|| archive.clone(), |a| a.label.clone());
            tr.t_args("card-summary-folder", &[("date", arg(date)), ("dest", arg(dest_of(dest.as_ref())))])
        }
    }
}

/// The versions of a ticked file, each tickable; under a ticked one, its
/// destination and the name it will take.
#[expect(clippy::too_many_arguments, reason = "one card: its parts are drawn together")]
fn file_card(
    ui: &mut Ui,
    tr: &Tr,
    default: &DefaultDest,
    plan: &Plan,
    card: usize,
    read: Option<&Load<Vec<FileVersion>>>,
    ticked: &mut [Version],
    edits: &mut Vec<CardEdit>,
) {
    let Some(read) = read else { return };
    let Some(list) = read.ready() else {
        read.status_ui(ui, tr, &tr.t("reading-backups"));
        return;
    };
    ui.weak(tr.t("versions-hint"));
    for (index, v) in list.iter().enumerate() {
        let position = ticked.iter().position(|t| t.archive == v.archive);
        let mut on = position.is_some();
        let label = tr.t_args(
            "version-detail",
            &[("backup", arg(v.backup_label.clone())), ("size", arg(tr.bytes(v.size)))],
        );
        let hint = tr.t_args("version-modified", &[("date", arg(v.modified_label.clone()))]);
        if ui.checkbox(&mut on, label).on_hover_text(hint).changed() {
            edits.push(CardEdit::Version { card, index, on });
        }
        if let Some(n) = position
            && let Some(t) = ticked.get_mut(n)
        {
            ui.indent(("dest", index), |ui| {
                dest_ui(ui, tr, &mut t.dest, default, plan.output(card, n));
            });
        }
    }
}

/// The date a ticked folder comes back as.
fn folder_card(
    ui: &mut Ui,
    tr: &Tr,
    card: usize,
    archive: &str,
    dates: &[&archeion_engine::CatalogArchive],
    edits: &mut Vec<CardEdit>,
) {
    let current =
        dates.iter().find(|a| a.name == archive).map_or_else(|| archive.to_owned(), |a| a.label.clone());
    ui.horizontal(|ui| {
        ui.label(tr.t("folder-as-of"));
        let mut chosen = archive.to_owned();
        egui::ComboBox::from_id_salt("folder-date")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for a in dates {
                    ui.selectable_value(&mut chosen, a.name.clone(), a.label.clone());
                }
            })
            .response
            .on_hover_text(tr.t("folder-as-of-hint"));
        if chosen != archive {
            edits.push(CardEdit::Date { card, archive: chosen });
        }
    });
}

/// Where a version or folder goes: the file name it takes (for a version)
/// and a menu to change the destination; a changed one is marked ✎.
fn dest_ui(ui: &mut Ui, tr: &Tr, dest: &mut Option<Dest>, default: &DefaultDest, output: Option<&Path>) {
    if let Some(path) = output {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        ui.label(format!("→ {name}")).on_hover_text(path.display().to_string());
    }
    let text = match dest {
        None => tr.t_args("dest-default", &[("dest", arg(default_text(tr, default)))]),
        Some(d) => format!("✎ {}", dest_text(tr, d)),
    };
    ui.menu_button(text, |ui| {
        if ui.button(tr.t("dest-follow-default")).clicked() {
            *dest = None;
            ui.close();
        }
        if ui.button(tr.t("target-beside")).clicked() {
            *dest = Some(Dest::Beside);
            ui.close();
        }
        if ui.button(tr.t("dest-overwrite")).on_hover_text(tr.t("replace-warning")).clicked() {
            *dest = Some(Dest::Original { replace: true });
            ui.close();
        }
        if ui.button(tr.t("dest-folder")).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                *dest = Some(Dest::Folder { path, replace: false });
            }
            ui.close();
        }
    })
    .response
    .on_hover_text(tr.t("dest-change-hint"));
    if dest.as_ref().is_some_and(Dest::overwrites_original) {
        ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("replace-warning")));
    }
}

fn default_text(tr: &Tr, default: &DefaultDest) -> String {
    match default.resolve() {
        Some(d) => dest_text(tr, &d),
        None => tr.t("dest-folder-missing"),
    }
}

fn dest_text(tr: &Tr, dest: &Dest) -> String {
    match dest {
        Dest::Beside => tr.t("target-beside"),
        Dest::Original { replace: true } => tr.t("dest-overwrite"),
        Dest::Original { replace: false } => tr.t("target-original"),
        Dest::Folder { path, .. } => path.display().to_string(),
    }
}

/// The default destination, at the top of the selection.
fn default_ui(ui: &mut Ui, tr: &Tr, default: &mut DefaultDest) {
    ui.add_space(theme::SPACE_S);
    ui.strong(tr.t("restore-to"));
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
        ui.checkbox(&mut default.replace, tr.t("replace-existing"));
        if default.replace && default.kind == DestKind::Original {
            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("replace-warning")));
        }
    }
    ui.add_space(theme::SPACE_S);
}

/// Summary and the Restore button, with the reason when it waits.
fn go_ui(ui: &mut Ui, tr: &Tr, plan: &Plan) -> Option<RestoreRequest> {
    ui.add_space(theme::SPACE_S);
    let s = plan.summary;
    let mut size = tr.bytes(s.bytes);
    if s.partial {
        size.push('…');
    }
    ui.label(tr.t_args(
        "restore-summary",
        &[
            ("files", arg(s.files)),
            ("versions", arg(s.versions)),
            ("folders", arg(s.folders)),
            ("size", arg(size)),
        ],
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
    let button = egui::Button::new(RichText::new(tr.t("restore-selected")).strong())
        .fill(ui.visuals().selection.bg_fill);
    let resp = ui.add_enabled(why.is_none(), button);
    let mut request = None;
    match &why {
        Some(w) => {
            let _ = resp.on_disabled_hover_text(w);
            ui.weak(w);
        }
        None if resp.clicked() => request = Some(plan.request.clone()),
        None => {}
    }
    request
}
