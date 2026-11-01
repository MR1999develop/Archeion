//! Main window state and layout.

use super::{
    activity::phase_text,
    editor::{self, Drafts, FormatLock, PasswordStatus, RunChoice},
    history::HistoryView,
    restore::{Action, RestoreView},
    theme,
};
use crate::{
    activity::{Activity, Op, Outcome},
    credentials,
    i18n::{Tr, arg},
    job::Job,
    storage::Storage,
};
use archeion_core::{Destination, Profile, ProfileId, Settings, Summary};
use archeion_engine::{EngineError, Password};
use archeion_scheduler::{Scheduler as _, TaskKind, TaskSpec, WindowsTaskScheduler};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, Ui};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

const NEW_PROFILE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const BACK_UP_NOW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::B);
/// Indicator appears only for waits longer than this.
const SPINNER_DELAY: Duration = Duration::from_secs(1);
/// Scheduled tasks are updated this long after the last schedule edit, so
/// dragging a time does not start `schtasks.exe` at every step.
const SCHEDULE_DEBOUNCE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    Backups,
    /// The running (or last) backup, merge or restore.
    Activity,
}

#[derive(Debug)]
pub struct ArcheionApp {
    storage: Storage,
    settings: Settings,
    /// Settings could not be read: never overwrite the user's file.
    read_only: bool,
    tr: Tr,
    selected: Option<ProfileId>,
    tab: Tab,
    job: Option<Job>,
    /// The last run once it has ended, for its summary.
    last_run: Option<Activity>,
    restore: RestoreView,
    /// Log of the operations performed.
    history: HistoryView,
    /// Text being typed in the profile form.
    drafts: Drafts,
    format: FormatCheck,
    /// Saved password of the selected profile, and a save in progress.
    password: PasswordState,
    /// Persistent problem shown in a banner until fixed.
    banner: Option<String>,
    /// Last failed action, dismissible.
    error: Option<String>,
    /// Profile removed by the last "Delete profile", for Undo.
    removed: Option<Profile>,
    /// Profile whose scheduled tasks must be updated, and when it changed.
    schedule_pending: Option<(ProfileId, Instant)>,
}

impl ArcheionApp {
    pub fn new(storage: Storage) -> Self {
        let (settings, read_only, banner) = match storage.load() {
            Ok(s) => (s, false, None),
            Err(e) => {
                tracing::error!(error = format!("{e:#}"), "settings unreadable");
                (Settings::default(), true, Some(format!("{e:#}")))
            }
        };
        let tr = Tr::new(settings.language);
        let banner = banner.map(|detail| tr.t_args("settings-unreadable", &[("detail", arg(detail))]));
        let selected = settings.profiles.first().map(|p| p.id);
        let mut app = Self {
            storage,
            settings,
            read_only,
            tr,
            selected,
            tab: Tab::Settings,
            job: None,
            last_run: None,
            restore: RestoreView::default(),
            history: HistoryView::default(),
            drafts: Drafts::default(),
            format: FormatCheck::default(),
            password: PasswordState::default(),
            banner,
            error: None,
            removed: None,
            schedule_pending: None,
        };
        app.move_legacy_tasks();
        app
    }

    /// Once: replaces the scheduled tasks of Archeion (the program's first
    /// name, whose executable no longer exists) with Archeion's.
    fn move_legacy_tasks(&mut self) {
        if self.read_only || !self.storage.first_time("tasks-renamed") {
            return;
        }
        let ids: Vec<ProfileId> = self.settings.profiles.iter().map(|p| p.id).collect();
        for id in ids {
            let mut had = false;
            for kind in TaskKind::ALL {
                match WindowsTaskScheduler.remove_legacy(id, kind) {
                    Ok(found) => had |= found,
                    Err(e) => tracing::warn!(error = %e, ?kind, "removing a Archeion task"),
                }
            }
            if had {
                self.apply_schedule(id);
            }
        }
    }

    /// Updates the scheduled tasks once edits have settled (or `now`).
    fn flush_schedule(&mut self, ctx: &egui::Context, force: bool) {
        let Some((id, since)) = self.schedule_pending else { return };
        if force || since.elapsed() >= SCHEDULE_DEBOUNCE {
            self.schedule_pending = None;
            self.apply_schedule(id);
        } else {
            ctx.request_repaint_after(SCHEDULE_DEBOUNCE.saturating_sub(since.elapsed()));
        }
    }

    fn save(&mut self) {
        if self.read_only {
            return;
        }
        if let Err(e) = self.storage.save(&self.settings) {
            tracing::error!(error = format!("{e:#}"), "saving settings");
            self.error = Some(self.tr.t_args("save-failed", &[("detail", arg(format!("{e:#}")))]));
        }
    }

    /// Registers (or removes) the backup and merge tasks of a profile.
    fn apply_schedule(&mut self, id: ProfileId) {
        let Some(p) = self.settings.profile(id) else { return };
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        for (kind, schedule) in [(TaskKind::Backup, p.schedule), (TaskKind::Merge, p.merge.schedule)] {
            let spec = TaskSpec {
                profile_id: id,
                profile_name: p.name.clone(),
                kind,
                schedule,
                executable: exe.clone(),
            };
            if let Err(e) = WindowsTaskScheduler.apply(&spec) {
                tracing::error!(error = %e, ?kind, "updating scheduled task");
                self.error = Some(self.tr.t_args("schedule-failed", &[("detail", arg(e.to_string()))]));
                return;
            }
        }
    }

    fn new_profile(&mut self) {
        let p = Profile::new(self.tr.t("new-profile-name"));
        self.selected = Some(p.id);
        self.settings.profiles.push(p);
        self.tab = Tab::Settings;
        self.restore.invalidate();
        self.save();
    }

    fn select(&mut self, ctx: &egui::Context, id: ProfileId) {
        if self.selected != Some(id) {
            self.flush_schedule(ctx, true);
            self.selected = Some(id);
            self.restore.invalidate();
            self.drafts.clear_password();
            self.password.hide_recovery();
        }
    }

    /// Starts the integrity check, or a restore test of `sample` files.
    fn verify(&mut self, ctx: &egui::Context, id: ProfileId, sample: Option<usize>) {
        let extra = sample.map(|n| vec!["--sample".into(), n.to_string().into()]).unwrap_or_default();
        self.start_with(ctx, id, Op::Verify, extra);
    }

    /// Starts a backup of the chosen kind, or the merge.
    fn run(&mut self, ctx: &egui::Context, id: ProfileId, choice: RunChoice) {
        match choice {
            RunChoice::Incremental => {
                self.start_with(ctx, id, Op::Backup, vec!["--kind".into(), "incremental".into()]);
            }
            RunChoice::Complete => {
                self.start_with(ctx, id, Op::Backup, vec!["--kind".into(), "complete".into()])
            }
            RunChoice::Merge => self.start(ctx, id, Op::Merge),
        }
    }

    fn start(&mut self, ctx: &egui::Context, id: ProfileId, op: Op) {
        self.start_with(ctx, id, op, Vec::new());
    }

    fn start_with(&mut self, ctx: &egui::Context, id: ProfileId, op: Op, extra: Vec<std::ffi::OsString>) {
        if self.job.is_some() {
            return;
        }
        let log = crate::history::new_log_path(self.storage.dir(), op, id);
        match Job::start(ctx, id, op, extra, log) {
            Ok(job) => {
                self.job = Some(job);
                self.tab = Tab::Activity;
                self.history.show_current();
            }
            Err(e) => self.error = Some(self.tr.t_args("start-failed", &[("detail", arg(e.to_string()))])),
        }
    }

    fn delete(&mut self, id: ProfileId) {
        let Some(pos) = self.settings.profiles.iter().position(|p| p.id == id) else { return };
        let profile = self.settings.profiles.remove(pos);
        if self.schedule_pending.is_some_and(|(p, _)| p == id) {
            self.schedule_pending = None;
        }
        // The tasks go; the archives stay, so Undo works.
        for kind in TaskKind::ALL {
            if let Err(e) = WindowsTaskScheduler.remove(id, kind) {
                tracing::warn!(error = %e, ?kind, "removing scheduled task");
            }
        }
        self.removed = Some(profile);
        self.selected = self.settings.profiles.first().map(|p| p.id);
        self.restore.invalidate();
        self.save();
    }

    fn undo_delete(&mut self) {
        if let Some(p) = self.removed.take() {
            let id = p.id;
            self.settings.profiles.push(p);
            self.selected = Some(id);
            self.save();
            self.apply_schedule(id);
        }
    }

    fn poll(&mut self) {
        if let Some(job) = &mut self.job
            && job.poll()
        {
            let activity = job.activity.clone();
            self.job = None;
            self.history.invalidate();
            // A first backup locks the format, deleting the only one frees
            // it: look at the destination again.
            self.format.invalidate();
            // A restore leaves the archives as they were: keep the browser.
            if matches!(activity.outcome, Some(Outcome::Finished(_))) && activity.op != Op::Restore {
                self.restore.invalidate();
            }
            self.last_run = Some(activity);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_shortcut(&NEW_PROFILE)) {
            self.new_profile();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&BACK_UP_NOW))
            && let Some(id) = self.selected
            && self.settings.profile(id).is_some_and(|p| p.validate().is_ok())
        {
            self.run(ctx, id, RunChoice::Incremental);
        }
    }

    fn menu(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(self.tr.t("menu-file"), |ui| {
                if ui.add(egui::Button::new(self.tr.t("new-profile")).shortcut_text("Ctrl+N")).clicked() {
                    self.new_profile();
                }
                ui.separator();
                if ui.button(self.tr.t("quit")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(self.tr.t("menu-backup"), |ui| {
                let can = self.job.is_none()
                    && self
                        .selected
                        .and_then(|id| self.settings.profile(id))
                        .is_some_and(|p| p.validate().is_ok());
                for (choice, label, shortcut) in [
                    (RunChoice::Incremental, "menu-run-incremental", Some("Ctrl+B")),
                    (RunChoice::Complete, "menu-run-complete", None),
                    (RunChoice::Merge, "merge-now", None),
                ] {
                    let mut button = egui::Button::new(self.tr.t(label));
                    if let Some(s) = shortcut {
                        button = button.shortcut_text(s);
                    }
                    if ui
                        .add_enabled(can, button)
                        .on_disabled_hover_text(self.tr.t("fix-profile-hint"))
                        .clicked()
                        && let Some(id) = self.selected
                    {
                        let ctx = ui.ctx().clone();
                        self.run(&ctx, id, choice);
                    }
                }
                if ui
                    .add_enabled(can, egui::Button::new(self.tr.t("verify-full-now")))
                    .on_hover_text(self.tr.t("verify-full-hint"))
                    .on_disabled_hover_text(self.tr.t("fix-profile-hint"))
                    .clicked()
                    && let Some(id) = self.selected
                {
                    let ctx = ui.ctx().clone();
                    self.verify(&ctx, id, None);
                }
                ui.separator();
                let has_profile = self.selected.is_some();
                if ui
                    .add_enabled(has_profile, egui::Button::new(self.tr.t("restore")))
                    .on_disabled_hover_text(self.tr.t("fix-profile-hint"))
                    .clicked()
                {
                    self.tab = Tab::Backups;
                }
            });
            ui.menu_button(self.tr.t("menu-help"), |ui| {
                ui.label(format!("Archeion {}", env!("CARGO_PKG_VERSION")));
                ui.label(format!("{}: {}", self.tr.t("data-folder"), self.storage.dir().display()));
            });
        });
    }

    fn profile_list(&mut self, ui: &mut Ui) {
        ui.add_space(theme::SPACE_S);
        ui.strong(self.tr.t("profiles"));
        let mut clicked = None;
        for p in &self.settings.profiles {
            let name = if p.name.trim().is_empty() { self.tr.t("unnamed") } else { p.name.clone() };
            let running = self.job.as_ref().is_some_and(|j| j.profile() == p.id);
            let text = if running { format!("⟳ {name}") } else { name };
            if ui.selectable_label(self.selected == Some(p.id), text).clicked() {
                clicked = Some(p.id);
            }
        }
        if let Some(id) = clicked {
            let ctx = ui.ctx().clone();
            self.select(&ctx, id);
        }
        ui.add_space(theme::SPACE_S);
        if ui.button(format!("+ {}", self.tr.t("new-profile"))).clicked() {
            self.new_profile();
        }
    }

    fn central(&mut self, ui: &mut Ui) {
        let Some(id) = self.selected.filter(|id| self.settings.profile(*id).is_some()) else {
            self.empty_state(ui);
            return;
        };
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Settings, self.tr.t("tab-settings"));
            ui.selectable_value(&mut self.tab, Tab::Backups, self.tr.t("tab-backups"));
            let label = if self.job.is_some() {
                format!("⟳ {}", self.tr.t("tab-activity"))
            } else {
                self.tr.t("tab-activity")
            };
            ui.selectable_value(&mut self.tab, Tab::Activity, label);
        });
        ui.separator();
        match self.tab {
            Tab::Settings => self.settings_tab(ui, id),
            Tab::Backups => self.backups_tab(ui, id),
            Tab::Activity => self.activity_tab(ui),
        }
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.heading(self.tr.t("empty-title"));
            ui.label(self.tr.t("empty-body"));
            ui.add_space(theme::SPACE_M);
            let b = egui::Button::new(self.tr.t("new-profile")).fill(ui.visuals().selection.bg_fill);
            if ui.add(b).clicked() {
                self.new_profile();
            }
        });
    }

    fn settings_tab(&mut self, ui: &mut Ui, id: ProfileId) {
        let busy = self.job.is_some();
        let format = match self.settings.profile(id) {
            Some(p) => self.format.state(ui.ctx(), p),
            None => FormatLock::Checking,
        };
        let password = self.password.status(id);
        if std::mem::take(&mut self.password.just_saved) {
            self.drafts.clear_password();
        }
        let out = egui::ScrollArea::vertical()
            .show(ui, |ui| {
                let Some(p) = self.settings.profile_mut(id) else { return editor::Outcome::default() };
                editor::show(ui, &self.tr, p, format, &password, &mut self.drafts, busy)
            })
            .inner;
        if out.changed {
            self.save();
        }
        if out.schedule_changed {
            self.schedule_pending = Some((id, Instant::now()));
        }
        match out.command {
            Some(editor::Command::Run(choice)) => {
                let ctx = ui.ctx().clone();
                self.run(&ctx, id, choice);
            }
            Some(editor::Command::SavePassword(pw)) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.save(ui.ctx(), p, pw, &self.tr);
                }
            }
            Some(editor::Command::ChangePassword(pw)) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.change(ui.ctx(), p, pw, &self.tr);
                }
            }
            Some(editor::Command::ShowRecoveryKey) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.show_recovery(ui.ctx(), p, &self.tr);
                }
            }
            Some(editor::Command::HideRecoveryKey) => self.password.hide_recovery(),
            Some(editor::Command::Verify(sample)) => {
                let ctx = ui.ctx().clone();
                self.verify(&ctx, id, sample);
            }
            Some(editor::Command::Delete) => self.delete(id),
            None => {}
        }
    }

    fn backups_tab(&mut self, ui: &mut Ui, id: ProfileId) {
        let Some(profile) = self.settings.profile(id).cloned() else { return };
        let busy = self.job.is_some();
        let ctx = ui.ctx().clone();
        match self.restore.show(ui, &self.tr, &profile, busy) {
            Some(Action::Restore(request)) => self.start_restore(&ctx, id, &request),
            Some(Action::Merge { first, last }) => {
                self.start_with(
                    &ctx,
                    id,
                    Op::Merge,
                    vec!["--from".into(), first.into(), "--to".into(), last.into()],
                );
            }
            Some(Action::Delete { archive }) => {
                self.start_with(&ctx, id, Op::Delete, vec!["--archive".into(), archive.into()]);
            }
            None => {}
        }
    }

    /// Hands the request to the worker through a file in the data folder.
    fn start_restore(
        &mut self,
        ctx: &egui::Context,
        id: ProfileId,
        request: &archeion_engine::RestoreRequest,
    ) {
        if self.job.is_some() {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let path = self.storage.dir().join(format!("restore-{stamp}.json"));
        let written = serde_json::to_string(request)
            .map_err(|e| e.to_string())
            .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()));
        if let Err(e) = written {
            self.error = Some(self.tr.t_args("start-failed", &[("detail", arg(e))]));
            return;
        }
        let log = crate::history::new_log_path(self.storage.dir(), Op::Restore, id);
        match Job::start(ctx, id, Op::Restore, vec!["--request".into(), path.into_os_string()], log) {
            Ok(job) => {
                self.job = Some(job);
                self.tab = Tab::Activity;
                self.history.show_current();
            }
            Err(e) => self.error = Some(self.tr.t_args("start-failed", &[("detail", arg(e.to_string()))])),
        }
    }

    fn activity_tab(&mut self, ui: &mut Ui) {
        let current = self.job.as_ref().map(|j| &j.activity).or(self.last_run.as_ref());
        let settings = &self.settings;
        let name = |id: ProfileId| settings.profile(id).map(|p| p.name.clone()).unwrap_or_default();
        let req = self.history.show(ui, &self.tr, self.storage.dir(), current, self.selected, &name);
        let profile = current.map(|a| a.profile);
        if req.cancel
            && let Some(job) = &mut self.job
        {
            job.cancel();
        }
        if req.show_backups
            && let Some(id) = profile
        {
            let ctx = ui.ctx().clone();
            self.select(&ctx, id);
            self.tab = Tab::Backups;
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if let Some(job) = &mut self.job {
                let a = &job.activity;
                let phase = a.phase.map_or_else(|| self.tr.t("phase-starting"), |p| phase_text(&self.tr, p));
                ui.label(phase);
                match a.fraction() {
                    Some(f) => {
                        ui.add(egui::ProgressBar::new(f).desired_width(240.0).show_percentage());
                    }
                    None if a.elapsed(Instant::now()) > SPINNER_DELAY => {
                        ui.spinner();
                    }
                    None => ui.ctx().request_repaint_after(SPINNER_DELAY),
                }
                if a.skipped_count > 0 {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        self.tr.t_args("skipped-count", &[("count", arg(a.skipped_count))]),
                    );
                }
                if self.tab != Tab::Activity && ui.link(self.tr.t("show-details")).clicked() {
                    self.tab = Tab::Activity;
                }
                if ui.button(self.tr.t("cancel")).clicked() {
                    job.cancel();
                }
            } else if let Some(removed) = &self.removed {
                ui.label(self.tr.t_args("profile-removed", &[("name", arg(removed.name.clone()))]));
                if ui.button(self.tr.t("undo")).clicked() {
                    self.undo_delete();
                }
                if ui.small_button("✕").clicked()
                    && let Some(p) = self.removed.take()
                {
                    // No Undo any more: the saved password goes too.
                    if let Err(e) = credentials::delete(p.id) {
                        tracing::warn!(error = %e, "removing the saved password");
                    }
                }
            } else if let Some(a) = &self.last_run
                && let Some(outcome) = &a.outcome
            {
                let name = self.settings.profile(a.profile).map(|p| p.name.clone()).unwrap_or_default();
                let (text, warn) = outcome_text(&self.tr, &name, a.op, outcome);
                if warn {
                    ui.colored_label(ui.visuals().warn_fg_color, text);
                } else {
                    ui.label(text);
                }
                if self.tab != Tab::Activity && ui.link(self.tr.t("show-details")).clicked() {
                    self.tab = Tab::Activity;
                }
            } else {
                ui.weak(self.tr.t("ready"));
            }
        });
    }
}

/// Background look at a profile's destination: does it already hold backups,
/// and in which format? Never blocks the window (a share may be slow).
#[derive(Debug, Default)]
struct FormatCheck {
    key: Option<(ProfileId, Destination)>,
    state: Option<FormatLock>,
    rx: Option<mpsc::Receiver<FormatLock>>,
}

impl FormatCheck {
    fn invalidate(&mut self) {
        self.key = None;
    }

    fn state(&mut self, ctx: &egui::Context, profile: &Profile) -> FormatLock {
        if profile.destination.is_empty() {
            return FormatLock::Free;
        }
        let key = (profile.id, profile.destination.clone());
        if self.key.as_ref() != Some(&key) {
            self.key = Some(key);
            self.state = None;
            let (tx, rx) = mpsc::channel();
            let (p, ctx) = (profile.clone(), ctx.clone());
            let spawned = std::thread::Builder::new().name("format-check".into()).spawn(move || {
                let lock = match archeion_engine::existing_format(&p) {
                    Ok(None) => FormatLock::Free,
                    Ok(Some(f)) => FormatLock::Locked { container: f.container, encrypted: f.encrypted },
                    Err(_) => FormatLock::Unreachable,
                };
                let _ = tx.send(lock);
                ctx.request_repaint();
            });
            self.rx = spawned.ok().map(|_| rx);
            if self.rx.is_none() {
                self.state = Some(FormatLock::Unreachable);
            }
        }
        if let Some(rx) = &self.rx
            && let Ok(lock) = rx.try_recv()
        {
            self.state = Some(lock);
            self.rx = None;
        }
        self.state.unwrap_or(FormatLock::Checking)
    }
}

/// What a background password job produced.
#[derive(Debug)]
enum PasswordDone {
    /// A password was checked (or changed) and saved.
    Saved,
    Recovery(String),
}

/// The saved password of the selected profile, and a background job on it
/// (check and save, change, read the recovery key: each derives a key with
/// Argon2id and may read a slow share).
#[derive(Debug, Default)]
struct PasswordState {
    /// Profile and whether a password is saved for it.
    saved: Option<(ProfileId, bool)>,
    job: Option<(ProfileId, mpsc::Receiver<Result<PasswordDone, String>>)>,
    error: Option<(ProfileId, String)>,
    /// Recovery key on show.
    recovery: Option<(ProfileId, String)>,
    /// A save has just succeeded: the typed password can be forgotten.
    just_saved: bool,
}

impl PasswordState {
    fn status(&mut self, id: ProfileId) -> PasswordStatus {
        if let Some((pid, rx)) = &self.job {
            let pid = *pid;
            let result = match rx.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("stopped unexpectedly".to_owned())),
            };
            if let Some(result) = result {
                self.job = None;
                match result {
                    Ok(PasswordDone::Saved) => {
                        self.saved = Some((pid, true));
                        self.error = None;
                        self.just_saved = true;
                    }
                    Ok(PasswordDone::Recovery(key)) => {
                        self.recovery = Some((pid, key));
                        self.error = None;
                    }
                    Err(e) => self.error = Some((pid, e)),
                }
            }
        }
        if self.saved.is_none_or(|(pid, _)| pid != id) {
            // Credential Manager answers locally and at once.
            let saved = match credentials::get(id) {
                Ok(p) => p.is_some(),
                Err(e) => {
                    tracing::warn!(error = %e, "reading Credential Manager");
                    false
                }
            };
            self.saved = Some((id, saved));
        }
        PasswordStatus {
            saved: self.saved.filter(|(pid, _)| *pid == id).map(|(_, s)| s),
            working: self.job.as_ref().is_some_and(|(pid, _)| *pid == id),
            error: self.error.as_ref().filter(|(pid, _)| *pid == id).map(|(_, e)| e.clone()),
            recovery: self.recovery.as_ref().filter(|(pid, _)| *pid == id).map(|(_, k)| k.clone()),
        }
    }

    fn hide_recovery(&mut self) {
        self.recovery = None;
    }

    /// Checks `pw` against the destination and saves it.
    fn save(&mut self, ctx: &egui::Context, profile: Profile, pw: Password, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, move || match archeion_engine::check_password(&profile, &pw) {
            Ok(()) => credentials::set(id, &pw).map(|()| PasswordDone::Saved),
            Err(e) => Err(texts.explain(&e)),
        });
    }

    /// Changes the destination's password from the saved one to `new`, then
    /// saves `new`.
    fn change(&mut self, ctx: &egui::Context, profile: Profile, new: Password, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, move || {
            let old = credentials::get(id)?.ok_or_else(|| texts.missing.clone())?;
            archeion_engine::change_password(&profile, &old, &new).map_err(|e| texts.explain(&e))?;
            credentials::set(id, &new).map(|()| PasswordDone::Saved)
        });
    }

    /// Reads the recovery key with the saved password.
    fn show_recovery(&mut self, ctx: &egui::Context, profile: Profile, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, move || {
            let pw = credentials::get(id)?.ok_or_else(|| texts.missing.clone())?;
            match archeion_engine::recovery_key(&profile, &pw) {
                Ok(Some(key)) => Ok(PasswordDone::Recovery(key)),
                Ok(None) => Err(texts.missing.clone()),
                Err(e) => Err(texts.explain(&e)),
            }
        });
    }

    fn spawn(
        &mut self,
        ctx: &egui::Context,
        id: ProfileId,
        work: impl FnOnce() -> Result<PasswordDone, String> + Send + 'static,
    ) {
        if self.job.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("password".into()).spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => {
                self.job = Some((id, rx));
                self.error = None;
            }
            Err(e) => self.error = Some((id, e.to_string())),
        }
    }
}

/// Messages a password job may need, taken on the window's thread.
#[derive(Debug, Clone)]
struct Texts {
    wrong: String,
    busy: String,
    missing: String,
    unreachable: String,
}

impl Texts {
    fn new(tr: &Tr) -> Self {
        Self {
            wrong: tr.t("password-wrong"),
            busy: tr.t("run-busy"),
            missing: tr.t("password-missing"),
            unreachable: tr.t("password-unreachable"),
        }
    }

    fn explain(&self, e: &EngineError) -> String {
        match e {
            EngineError::WrongPassword => self.wrong.clone(),
            EngineError::Busy => self.busy.clone(),
            other => format!("{}: {other}", self.unreachable),
        }
    }
}

impl Drop for ArcheionApp {
    /// Closing the window right after a schedule edit still applies it.
    fn drop(&mut self) {
        if let Some((id, _)) = self.schedule_pending.take() {
            self.apply_schedule(id);
        }
    }
}

impl eframe::App for ArcheionApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.poll();
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        self.flush_schedule(&ctx, false);

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        if let Some(text) = &self.banner {
            egui::Panel::top("banner").show(ui, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {text}"));
            });
        }
        if let Some(text) = self.error.clone() {
            egui::Panel::top("error").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {text}"));
                    if ui.small_button("✕").clicked() {
                        self.error = None;
                    }
                });
            });
        }
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("profiles")
            .resizable(true)
            .default_size(theme::SIDE_PANEL_WIDTH)
            .show(ui, |ui| self.profile_list(ui));
        egui::CentralPanel::default().show(ui, |ui| self.central(ui));
    }
}

/// Status line for a finished run and whether it needs attention.
fn outcome_text(tr: &Tr, name: &str, op: Op, o: &Outcome) -> (String, bool) {
    match o {
        Outcome::Finished(Summary::Merge(_)) if op == Op::Delete => {
            (tr.t_args("delete-finished", &[("name", arg(name.to_owned()))]), false)
        }
        Outcome::Finished(Summary::Backup(r)) => {
            let text = tr.t_args(
                if r.complete { "run-finished-complete" } else { "run-finished" },
                &[
                    ("name", arg(name.to_owned())),
                    ("new", arg(r.files_new)),
                    ("changed", arg(r.files_changed)),
                    ("deleted", arg(r.files_deleted)),
                    ("added", arg(tr.bytes(r.bytes_copied))),
                ],
            );
            let text = super::activity::with_resumed(tr, text, r);
            if r.skipped.is_empty() {
                (text, false)
            } else {
                let extra = tr.t_args("skipped-count", &[("count", arg(r.skipped.len()))]);
                (format!("{text} · {extra}"), true)
            }
        }
        Outcome::Finished(Summary::Merge(r)) => (
            tr.t_args(
                "merge-finished",
                &[
                    ("name", arg(name.to_owned())),
                    ("before", arg(r.archives_before)),
                    ("after", arg(r.archives_after)),
                    ("freed", arg(tr.bytes(r.bytes_freed))),
                ],
            ),
            false,
        ),
        Outcome::Finished(Summary::Restore(r)) => {
            (tr.t_args("restore-finished", &[("count", arg(r.files_restored))]), !r.skipped.is_empty())
        }
        Outcome::Finished(Summary::Verify(r)) => (
            tr.t_args(
                "verify-finished",
                &[("count", arg(r.files_checked)), ("problems", arg(r.problems.len()))],
            ),
            !r.problems.is_empty(),
        ),
        Outcome::Busy => (tr.t("run-busy"), true),
        Outcome::PasswordNeeded => (tr.t("run-password-needed"), true),
        Outcome::WrongPassword => (tr.t("run-wrong-password"), true),
        Outcome::NotEnoughSpace { needed, available } => {
            (super::activity::no_space_text(tr, *needed, *available), true)
        }
        Outcome::DestinationUnavailable(loc) => {
            (tr.t_args("run-destination-missing", &[("location", arg(loc.clone()))]), true)
        }
        Outcome::Failed(msg) => (tr.t_args("run-failed", &[("detail", arg(msg.clone()))]), true),
        Outcome::Cancelled => (tr.t("run-cancelled"), false),
    }
}
