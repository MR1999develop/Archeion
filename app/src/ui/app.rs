//! Main window state and layout.

use super::{
    activity::phase_text,
    backup_import::{self, BackupImport},
    editor::{self, Drafts, FormatLock, PasswordStatus, RunChoice},
    help::{self, Help},
    history::HistoryView,
    restore::{Action, RestoreView},
    settings::{self, Preferences},
    symbol, theme,
    tray::{Tray, TrayCommand},
    widgets,
};
use crate::{
    activity::{Activity, Op, Outcome},
    autostart, credentials,
    i18n::{Tr, arg},
    instance::Primary,
    job::Job,
    profile_file::{self, ImportError},
    storage::Storage,
};
use archeion_core::{Destination, Profile, ProfileId, Schedule, Settings, Summary};
use archeion_engine::{EngineError, Password};
use archeion_scheduler::{Scheduler as _, SchedulerError, TaskKind, TaskSpec, WindowsTaskScheduler};
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
/// A new window size is saved once it stays the same this long.
const RESIZE_SETTLE: Duration = Duration::from_secs(1);
/// The automatic merge looks for work this long after the window opens,
/// then at this interval, and shortly after each backup.
const AUTO_MERGE_FIRST: Duration = Duration::from_secs(30);
const AUTO_MERGE_EVERY: Duration = Duration::from_secs(15 * 60);
const AUTO_MERGE_AFTER_RUN: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    Backups,
    /// The running backup, merge or restore, and the log of past runs.
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
    /// While a run started here goes on, Windows says so at shut down.
    shutdown_block: Option<archeion_win::ShutdownBlock>,
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
    /// Startup check of the scheduled tasks, running in the background:
    /// the problems it met.
    task_sync: Option<mpsc::Receiver<Vec<String>>>,
    /// Run started by the Run button of an encrypted profile, waiting for
    /// the typed password to be checked and saved.
    run_after_password: Option<(ProfileId, RunChoice)>,
    auto_merge: AutoMerge,
    /// The user closed the window while a run was going: it closes when
    /// the run ends.
    close_after_run: bool,
    /// Icon in the notification area, when Windows gave one.
    tray: Option<Tray>,
    /// Quit was chosen (menu or tray): the window really closes instead of
    /// going to the notification area.
    quitting: bool,
    /// Started with Windows: hide the window after its first frame.
    start_hidden: bool,
    /// Another launch of Archeion asked for this window.
    wake: Option<mpsc::Receiver<()>>,
    /// The Help windows: user guide and About.
    help: Help,
    /// The Preferences window: language and window behaviour.
    preferences: Preferences,
    /// Backups of a deleted profile being deleted, or the outcome.
    wipe: Option<Wipe>,
    /// Taking over a destination, on a thread of its own (a share may be
    /// slow): its result.
    adopting: Option<mpsc::Receiver<Result<(), String>>>,
    /// A profile being imported from a folder of backups.
    backup_import: BackupImport,
    /// When the window took a size not saved yet: it is saved once the
    /// resizing stops.
    resized_at: Option<Instant>,
    /// The first size was checked against the screen.
    fitted: bool,
}

/// The automatic merge, which runs while the window is open (it has no
/// scheduled task): the next look for work, and a look in progress.
#[derive(Debug)]
struct AutoMerge {
    next_check: Instant,
    /// The profile to merge, if the look found one.
    rx: Option<mpsc::Receiver<Option<ProfileId>>>,
}

impl Default for AutoMerge {
    fn default() -> Self {
        Self { next_check: Instant::now() + AUTO_MERGE_FIRST, rx: None }
    }
}

impl ArcheionApp {
    /// The window's state. `start_hidden`: wait in the notification area
    /// (when there is one). `primary`: the single-instance claim, served
    /// here so that a second launch brings this window forward.
    pub fn new(storage: Storage, ctx: &egui::Context, start_hidden: bool, primary: Option<Primary>) -> Self {
        let mut app = Self::state(storage);
        match Tray::new(ctx, &app.tr) {
            Ok(tray) => {
                tray.set_visible(app.settings.window.close_to_tray);
                app.tray = Some(tray);
            }
            Err(e) => tracing::warn!(error = %e, "no notification-area icon"),
        }
        // Hidden only where the user can get the window back.
        app.start_hidden = start_hidden && app.hides_to_tray();
        if let Some(primary) = primary {
            let (tx, rx) = mpsc::channel();
            let ctx = ctx.clone();
            primary.serve(move || {
                let _ = tx.send(());
                ctx.request_repaint();
            });
            app.wake = Some(rx);
        }
        if !app.read_only {
            crate::storage::remove_stale_requests(app.storage.dir());
            app.start_task_sync();
        }
        app
    }

    /// The window's state read from `storage`, without touching Windows
    /// (notification area, scheduled tasks): what the tests draw.
    fn state(storage: Storage) -> Self {
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
        Self {
            storage,
            settings,
            read_only,
            tr,
            selected,
            tab: Tab::Settings,
            job: None,
            shutdown_block: None,
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
            task_sync: None,
            run_after_password: None,
            auto_merge: AutoMerge::default(),
            close_after_run: false,
            tray: None,
            quitting: false,
            start_hidden: false,
            wake: None,
            help: Help::default(),
            preferences: Preferences::default(),
            resized_at: None,
            wipe: None,
            adopting: None,
            backup_import: BackupImport::default(),
            fitted: false,
        }
    }

    /// True when closing the window hides it in the notification area.
    fn hides_to_tray(&self) -> bool {
        self.tray.is_some() && self.settings.window.close_to_tray
    }

    /// Checks in the background that every automatic profile has its
    /// scheduled tasks and that they start this executable, registering
    /// them again otherwise: the program was moved, a portable copy runs
    /// from another folder, or the tasks were deleted by hand.
    fn start_task_sync(&mut self) {
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                tracing::warn!(error = %e, "executable path unknown: scheduled tasks not checked");
                return;
            }
        };
        let user = windows_user();
        let specs: Vec<TaskSpec> =
            self.settings.profiles.iter().flat_map(|p| task_specs(p, &exe, &user)).collect();
        let watch = watch_wanted(&self.settings.profiles);
        let start_with_windows = self.settings.window.start_with_windows;
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("task-sync".into()).spawn(move || {
            let mut problems = sync_tasks(&specs);
            problems.extend(sync_watch(watch, &exe, &user));
            if let Err(e) = autostart::sync(start_with_windows, &exe) {
                tracing::error!(error = %e, "updating the start with Windows");
                problems.push(e.to_string());
            }
            let _ = tx.send(problems);
        });
        match spawned {
            Ok(_) => self.task_sync = Some(rx),
            Err(e) => tracing::warn!(error = %e, "scheduled tasks not checked"),
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

    /// Saves the window's size once the user stops resizing it (at once
    /// when the window closes), so the next start opens it the same.
    fn remember_size(&mut self, ctx: &egui::Context) {
        let (inner, maximized, minimized, closing, monitor) = ctx.input(|i| {
            let v = i.viewport();
            let size = v.inner_rect.map(|r| r.size());
            (size, v.maximized == Some(true), v.minimized == Some(true), v.close_requested(), v.monitor_size)
        });
        // First start on a small screen: maximized rather than cut off.
        // The monitor may be unknown in the first frames: checked once known.
        if self.settings.window_size.is_none()
            && !self.fitted
            && let Some(m) = monitor
        {
            self.fitted = true;
            if !super::first_size_fits(m) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
        }
        let seen = super::seen_size(inner, maximized, minimized, self.settings.window_size);
        if seen.is_none() || seen == self.settings.window_size {
            self.resized_at = None;
            return;
        }
        let since = *self.resized_at.get_or_insert_with(Instant::now);
        if closing || since.elapsed() >= RESIZE_SETTLE {
            self.settings.window_size = seen;
            self.resized_at = None;
            self.save();
        } else {
            ctx.request_repaint_after(RESIZE_SETTLE.saturating_sub(since.elapsed()));
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
        let user = windows_user();
        for spec in task_specs(p, &exe, &user) {
            if let Err(e) = WindowsTaskScheduler.apply(&spec) {
                tracing::error!(error = %e, kind = ?spec.kind, "updating scheduled task");
                self.error = Some(self.tr.t_args("schedule-failed", &[("detail", arg(e.to_string()))]));
                return;
            }
        }
        // The daily "no recent backup" check exists while a profile wants it.
        let watch = watch_wanted(&self.settings.profiles);
        if let Err(e) = WindowsTaskScheduler.apply_watch(watch, &exe, &user) {
            tracing::error!(error = %e, "updating the watch task");
            self.error = Some(self.tr.t_args("schedule-failed", &[("detail", arg(e.to_string()))]));
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

    /// Adds the profile of a file chosen by the user, selects it and
    /// registers its scheduled tasks. Its password, if encrypted, is asked
    /// for by its settings, as for a new profile.
    fn import_profile(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.tr.t("import-profile"))
            .add_filter(self.tr.t("profile-file-type"), &[profile_file::EXTENSION])
            .pick_file()
        else {
            return;
        };
        let file = || arg(path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()));
        match profile_file::import(&path) {
            Ok(profile) => self.add_imported(profile, None),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "profile import");
                self.error = Some(match e {
                    ImportError::Read(e) => {
                        self.tr.t_args("import-failed-read", &[("detail", arg(e.to_string()))])
                    }
                    ImportError::NotAProfile => self.tr.t_args("import-failed-format", &[("file", file())]),
                    ImportError::Newer(_) => self.tr.t_args("import-failed-newer", &[("file", file())]),
                });
            }
        }
    }

    /// Starts importing the profile of a folder of backups the user
    /// chooses, from the copy its backups keep there.
    fn import_from_backup(&mut self, ctx: &egui::Context) {
        let Some(folder) = rfd::FileDialog::new().set_title(self.tr.t("import-backup")).pick_folder() else {
            return;
        };
        self.backup_import.start(ctx, folder);
    }

    /// Adds an imported profile, selects it and registers its scheduled
    /// tasks (paused until the user turns them on). `password` opened its
    /// backups and is saved with it; without one, a password saved for its
    /// id is forgotten: no profile here had this id, so it was left by a
    /// profile deleted before (the window closed abruptly) and must not
    /// open the backups the import names. An encrypted profile without a
    /// password asks for it in its settings, as a new one does.
    fn add_imported(&mut self, profile: Profile, password: Option<Password>) {
        let held: Vec<ProfileId> = self.removed.iter().map(|p| p.id).collect();
        let id = profile_file::add(&mut self.settings, profile, &held);
        match password {
            Some(pw) => {
                if let Err(e) = credentials::set(id, &pw) {
                    tracing::error!(error = %e, "saving the password of an imported profile");
                    self.error =
                        Some(self.tr.t_args("import-backup-password-not-saved", &[("detail", arg(e))]));
                }
            }
            None => forget_password(id),
        }
        self.password.saved = None;
        self.selected = Some(id);
        self.tab = Tab::Settings;
        self.restore.invalidate();
        self.drafts.clear_password();
        self.save();
        self.apply_schedule(id);
    }

    /// Saves the selected profile to a file the user names.
    fn export_profile(&mut self) {
        let Some(profile) = self.selected.and_then(|id| self.settings.profile(id)).cloned() else { return };
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.tr.t("export-profile"))
            .add_filter(self.tr.t("profile-file-type"), &[profile_file::EXTENSION])
            .set_file_name(profile_file::file_name(&profile.name))
            .save_file()
        else {
            return;
        };
        if let Err(e) = profile_file::export(&path, &profile) {
            tracing::error!(error = format!("{e:#}"), path = %path.display(), "profile export");
            self.error = Some(self.tr.t_args("export-failed", &[("detail", arg(format!("{e:#}")))]));
        }
    }

    fn select(&mut self, ctx: &egui::Context, id: ProfileId) {
        if self.selected != Some(id) {
            self.flush_schedule(ctx, true);
            self.selected = Some(id);
            // The Backups tab keeps what it read per profile.
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
        self.start_job(ctx, id, op, extra, true);
    }

    /// Starts a worker. `show` switches to the Activity tab (a run the user
    /// asked for); an automatic merge runs without moving the user away.
    fn start_job(
        &mut self,
        ctx: &egui::Context,
        id: ProfileId,
        op: Op,
        extra: Vec<std::ffi::OsString>,
        show: bool,
    ) {
        if self.job.is_some() {
            return;
        }
        let log = crate::history::new_log_path(self.storage.dir(), op, id);
        match Job::start(ctx, id, op, extra, log) {
            Ok(job) => {
                self.job = Some(job);
                if show {
                    self.tab = Tab::Activity;
                    self.history.show_current();
                }
            }
            Err(e) => self.error = Some(self.tr.t_args("start-failed", &[("detail", arg(e.to_string()))])),
        }
    }

    /// Merges by the rules, while the window is open, the first profile
    /// whose old backups are due for a merge. Nothing happens while this
    /// window runs anything, nor on a destination another run holds (a
    /// backup started by Task Scheduler, another window): the next look
    /// tries again. Only archive names are read to decide, without a lock.
    fn auto_merge(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.auto_merge.rx {
            match rx.try_recv() {
                Ok(found) => {
                    self.auto_merge.rx = None;
                    if let Some(id) = found
                        && self.job.is_none()
                        && !self.close_after_run
                    {
                        tracing::info!(profile = %id, "automatic merge");
                        // Logged as automatic, like the scheduled runs.
                        self.start_job(ctx, id, Op::Merge, vec!["--scheduled".into()], false);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.auto_merge.rx = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
            return;
        }
        if self.job.is_some() || self.read_only || self.close_after_run {
            return;
        }
        let now = Instant::now();
        if now < self.auto_merge.next_check {
            ctx.request_repaint_after(self.auto_merge.next_check - now);
            return;
        }
        self.auto_merge.next_check = now + AUTO_MERGE_EVERY;
        let profiles: Vec<Profile> =
            self.settings.profiles.iter().filter(|p| p.validate().is_ok()).cloned().collect();
        if profiles.is_empty() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("auto-merge".into()).spawn(move || {
            let found = profiles
                .iter()
                .find(|p| {
                    archeion_engine::merge_pending(p).unwrap_or(false)
                        && !archeion_engine::destination_busy(p)
                })
                .map(|p| p.id);
            let _ = tx.send(found);
            ctx.request_repaint();
        });
        if spawned.is_ok() {
            self.auto_merge.rx = Some(rx);
        }
    }

    /// Gives the backups of a deleted profile at the destination to `id`.
    fn adopt_destination(&mut self, ctx: &egui::Context, id: ProfileId) {
        let Some(p) = self.settings.profile(id).cloned() else { return };
        if self.adopting.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("adopt-destination".into()).spawn(move || {
            let _ = tx.send(archeion_engine::adopt_destination(&p).map_err(|e| e.to_string()));
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => self.adopting = Some(rx),
            Err(e) => {
                self.error = Some(self.tr.t_args("dest-adopt-failed", &[("detail", arg(e.to_string()))]))
            }
        }
    }

    /// The result of [`Self::adopt_destination`], once there.
    fn poll_adopting(&mut self) {
        let Some(rx) = &self.adopting else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.adopting = None;
        match result {
            Ok(()) => {
                self.format.invalidate();
                self.restore.invalidate();
                self.history.invalidate();
            }
            Err(e) => {
                tracing::error!(error = %e, "taking over the destination");
                self.error = Some(self.tr.t_args("dest-adopt-failed", &[("detail", arg(e))]));
            }
        }
    }

    /// Removes the profile and its tasks; Undo brings them back until the
    /// next deletion. With `backups`, its backups at the destination go to
    /// the Recycle Bin too, in the background, and there is no Undo.
    fn delete(&mut self, id: ProfileId, backups: bool) {
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
        if backups {
            // Final: no Undo, so the saved password goes now.
            forget_password(id);
            self.wipe = Some(Wipe::start(profile));
        } else if let Some(previous) = self.removed.replace(profile) {
            // Only the last deletion can be undone: an earlier one is final now.
            forget_password(previous.id);
        }
        self.selected = self.settings.profiles.first().map(|p| p.id);
        self.restore.invalidate();
        self.save();
    }

    /// Takes the outcome of the deletion of backups, once it ended.
    fn poll_wipe(&mut self, ctx: &egui::Context) {
        let Some(Wipe::Running { name, path, done }) = &self.wipe else { return };
        let result = match done.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(SPINNER_DELAY);
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => Err(EngineError::Io {
                context: "moving the backups to the Recycle Bin".into(),
                source: std::io::Error::other("stopped unexpectedly"),
            }),
        };
        let name = || arg(name.clone());
        let (text, warn) = match result {
            Ok(d) if d.left.is_empty() => (self.tr.t_args("delete-backups-done", &[("name", name())]), false),
            Err(EngineError::NoRecycleBin(_)) => {
                let args = [("name", name()), ("path", arg(path.clone()))];
                (self.tr.t_args("delete-backups-no-bin", &args), true)
            }
            Ok(d) => {
                let args = [("name", name()), ("path", arg(path.clone())), ("count", arg(d.left.len()))];
                (self.tr.t_args("delete-backups-left", &args), true)
            }
            Err(e) => {
                tracing::error!(error = %e, "moving the backups to the Recycle Bin");
                (
                    self.tr
                        .t_args("delete-backups-failed", &[("name", name()), ("detail", arg(e.to_string()))]),
                    true,
                )
            }
        };
        self.wipe = Some(Wipe::Done { text, warn });
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
        if let Some(rx) = &self.task_sync {
            match rx.try_recv() {
                Ok(problems) => {
                    self.task_sync = None;
                    if let Some(first) = problems.into_iter().next() {
                        self.error = Some(self.tr.t_args("schedule-failed", &[("detail", arg(first))]));
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => self.task_sync = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(job) = &mut self.job
            && job.poll()
        {
            let activity = job.activity.clone();
            self.history.run_ended(job.log_path().to_path_buf());
            self.job = None;
            // A first backup locks the format, deleting the only one frees
            // it: look at the destination again.
            self.format.invalidate();
            // A restore leaves the archives as they were: keep the browser.
            if matches!(activity.outcome, Some(Outcome::Finished(_))) && activity.op != Op::Restore {
                self.restore.invalidate();
            }
            // A new backup may leave old ones to merge.
            if activity.op == Op::Backup {
                self.auto_merge.next_check = Instant::now() + AUTO_MERGE_AFTER_RUN;
            }
            self.last_run = Some(activity);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_shortcut(&help::DOCUMENTATION)) {
            self.help.open_docs();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&NEW_PROFILE)) {
            self.new_profile();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&settings::OPEN)) {
            self.preferences.open(ctx);
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
                if ui
                    .button(self.tr.t("import-profile"))
                    .on_hover_text(self.tr.t("import-profile-hint"))
                    .clicked()
                {
                    self.import_profile();
                }
                let importing = self.backup_import.busy();
                if ui
                    .add_enabled(!importing, egui::Button::new(self.tr.t("import-backup")))
                    .on_hover_text(self.tr.t("import-backup-hint"))
                    .on_disabled_hover_text(self.tr.t("import-backup-busy"))
                    .clicked()
                {
                    let ctx = ui.ctx().clone();
                    self.import_from_backup(&ctx);
                }
                let exportable = self.selected.is_some_and(|id| self.settings.profile(id).is_some());
                if ui
                    .add_enabled(exportable, egui::Button::new(self.tr.t("export-profile")))
                    .on_hover_text(self.tr.t("export-profile-hint"))
                    .clicked()
                {
                    self.export_profile();
                }
                ui.separator();
                let preferences = egui::Button::new(self.tr.t("menu-preferences")).shortcut_text("Ctrl+,");
                if ui.add(preferences).clicked() {
                    self.preferences.open(ui.ctx());
                }
                ui.separator();
                if ui.button(self.tr.t("quit")).on_hover_text(self.tr.t("quit-hint")).clicked() {
                    let ctx = ui.ctx().clone();
                    self.quit(&ctx);
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
                    self.restore.open_files();
                }
            });
            ui.menu_button(self.tr.t("menu-help"), |ui| self.help.menu(ui, &self.tr));
        });
    }

    fn profile_list(&mut self, ui: &mut Ui) {
        ui.add_space(theme::SPACE_M);
        widgets::section_title(ui, self.tr.t("profiles"));
        let mut clicked = None;
        for p in &self.settings.profiles {
            let name = if p.name.trim().is_empty() { self.tr.t("unnamed") } else { p.name.clone() };
            let running = self.job.as_ref().is_some_and(|j| j.profile() == p.id);
            let text = if running { symbol::with(symbol::RUNNING, name) } else { name };
            if widgets::list_item(ui, self.selected == Some(p.id), egui::RichText::new(text), None).clicked()
            {
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
        let name = self.settings.profile(id).map(|p| p.name.trim().to_owned()).unwrap_or_default();
        ui.add_space(theme::SPACE_S);
        ui.heading(if name.is_empty() { self.tr.t("unnamed") } else { name });
        ui.add_space(theme::SPACE_XS);
        let activity = if self.job.is_some() {
            symbol::with(symbol::RUNNING, self.tr.t("tab-activity"))
        } else {
            self.tr.t("tab-activity")
        };
        let items = [
            (Tab::Settings, self.tr.t("tab-settings"), None),
            (Tab::Backups, self.tr.t("tab-backups"), None),
            (Tab::Activity, activity, None),
        ];
        widgets::tabs(ui, &mut self.tab, &items, false);
        ui.separator();
        ui.add_space(theme::SPACE_XS);
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
            let b = widgets::primary_button(ui, self.tr.t("new-profile"));
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
        let dest = self.format.info(&self.settings.profiles);
        let password = self.password.status(id);
        if std::mem::take(&mut self.password.just_saved) {
            self.drafts.clear_password();
            // The Run button saved the password first: run now.
            if let Some((pid, choice)) = self.run_after_password.take()
                && pid == id
            {
                let ctx = ui.ctx().clone();
                self.run(&ctx, id, choice);
            }
        }
        if password.error.is_some() {
            self.run_after_password = None;
        }
        let out = egui::ScrollArea::vertical()
            .show(ui, |ui| {
                let Some(p) = self.settings.profile_mut(id) else { return editor::Outcome::default() };
                editor::show(ui, &self.tr, p, format, &dest, &password, &mut self.drafts, busy)
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
            Some(editor::Command::SavePasswordAndRun(pw, choice)) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.save(ui.ctx(), p, pw, &self.tr);
                    self.run_after_password = Some((id, choice));
                }
            }
            Some(editor::Command::ChangePassword { current, new }) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.change(ui.ctx(), p, current, new, &self.tr);
                }
            }
            Some(editor::Command::ShowRecoveryKey(pw)) => {
                if let Some(p) = self.settings.profile(id).cloned() {
                    self.password.show_recovery(ui.ctx(), p, pw, &self.tr);
                }
            }
            Some(editor::Command::HideRecoveryKey) => self.password.hide_recovery(),
            Some(editor::Command::Verify(sample)) => {
                let ctx = ui.ctx().clone();
                self.verify(&ctx, id, sample);
            }
            Some(editor::Command::Delete { backups }) => self.delete(id, backups),
            Some(editor::Command::AdoptDestination) => self.adopt_destination(&ui.ctx().clone(), id),
            None => {}
        }
    }

    fn backups_tab(&mut self, ui: &mut Ui, id: ProfileId) {
        let Some(profile) = self.settings.profile(id).cloned() else { return };
        let busy = self.job.is_some();
        let ctx = ui.ctx().clone();
        match self.restore.show(ui, &self.tr, &profile, busy) {
            Some(Action::Restore(request)) => self.start_restore(&ctx, id, &request),
            Some(Action::Merge { first, last, keep_deleted }) => {
                let mut args = vec!["--from".into(), first.into(), "--to".into(), last.into()];
                if !keep_deleted {
                    args.push("--drop-deleted".into());
                }
                self.start_with(&ctx, id, Op::Merge, args);
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
        let running = self.job.as_ref().map(|j| &j.activity);
        let settings = &self.settings;
        let name = |id: ProfileId| settings.profile(id).map(|p| p.name.clone()).unwrap_or_default();
        let cancel = self.history.show(ui, &self.tr, self.storage.dir(), running, self.selected, &name);
        if cancel && let Some(job) = &mut self.job {
            job.cancel();
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
            } else if let Some(wipe) = &self.wipe {
                match wipe {
                    Wipe::Running { name, .. } => {
                        ui.spinner();
                        ui.label(self.tr.t_args("delete-backups-running", &[("name", arg(name.clone()))]));
                    }
                    Wipe::Done { text, warn } => {
                        widgets::status_text(ui, text.as_str(), *warn);
                        if widgets::icon_button(ui, symbol::REMOVE, self.tr.t("close")).clicked() {
                            self.wipe = None;
                        }
                    }
                }
            } else if let Some(removed) = &self.removed {
                widgets::status_text(
                    ui,
                    self.tr.t_args("profile-removed", &[("name", arg(removed.name.clone()))]),
                    false,
                );
                if ui.button(self.tr.t("undo")).clicked() {
                    self.undo_delete();
                }
                if widgets::icon_button(ui, symbol::REMOVE, self.tr.t("close")).clicked()
                    && let Some(p) = self.removed.take()
                {
                    // No Undo any more: the saved password goes too.
                    forget_password(p.id);
                }
            } else if let Some(a) = &self.last_run
                && let Some(outcome) = &a.outcome
            {
                let name = self.settings.profile(a.profile).map(|p| p.name.clone()).unwrap_or_default();
                let (text, warn) = outcome_text(&self.tr, &name, a.op, outcome);
                widgets::status_text(ui, text, warn);
                if self.tab != Tab::Activity && ui.link(self.tr.t("show-details")).clicked() {
                    self.tab = Tab::Activity;
                }
            } else {
                ui.weak(self.tr.t("ready"));
            }
        });
    }
}

/// What one look at the destination found: the format lock, the free
/// space, and the owner when it is another profile.
type DestinationLook = (FormatLock, Option<u64>, Option<ProfileId>, Option<String>);

/// Background look at a profile's destination: does it already hold backups,
/// and in which format? Never blocks the window (a share may be slow).
#[derive(Debug, Default)]
struct FormatCheck {
    key: Option<(ProfileId, Destination)>,
    state: Option<FormatLock>,
    /// Free bytes on the destination's drive, when known.
    free: Option<u64>,
    /// Owner of the destination when it is another profile.
    foreign: Option<ProfileId>,
    /// File system of the destination's drive, when known.
    file_system: Option<String>,
    rx: Option<mpsc::Receiver<DestinationLook>>,
}

impl FormatCheck {
    fn invalidate(&mut self) {
        self.key = None;
    }

    fn state(&mut self, ctx: &egui::Context, profile: &Profile) -> FormatLock {
        if profile.destination.is_empty() || profile.network_unconfirmed {
            self.free = None;
            self.foreign = None;
            self.file_system = None;
            // Looked at once confirmed.
            self.key = None;
            return if profile.network_unconfirmed { FormatLock::Unconfirmed } else { FormatLock::Free };
        }
        let key = (profile.id, profile.destination.clone());
        if self.key.as_ref() != Some(&key) {
            self.key = Some(key);
            self.state = None;
            self.free = None;
            self.foreign = None;
            self.file_system = None;
            let (tx, rx) = mpsc::channel();
            let (p, ctx) = (profile.clone(), ctx.clone());
            let spawned = std::thread::Builder::new().name("format-check".into()).spawn(move || {
                let lock = match archeion_engine::existing_format(&p) {
                    Ok(None) => FormatLock::Free,
                    Ok(Some(f)) => FormatLock::Locked { container: f.container, encrypted: f.encrypted },
                    Err(_) => FormatLock::Unreachable,
                };
                let free = archeion_engine::free_space(&p);
                let foreign = archeion_engine::destination_owner(&p).ok().flatten().filter(|o| *o != p.id);
                let file_system = archeion_engine::destination_file_system(&p);
                let _ = tx.send((lock, free, foreign, file_system));
                ctx.request_repaint();
            });
            self.rx = spawned.ok().map(|_| rx);
            if self.rx.is_none() {
                self.state = Some(FormatLock::Unreachable);
            }
        }
        if let Some(rx) = &self.rx
            && let Ok((lock, free, foreign, file_system)) = rx.try_recv()
        {
            self.state = Some(lock);
            self.free = free;
            self.foreign = foreign;
            self.file_system = file_system;
            self.rx = None;
        }
        self.state.unwrap_or(FormatLock::Checking)
    }

    /// Free space and owner found by the last look at the destination;
    /// `profiles` names the owner when it still exists.
    fn info(&self, profiles: &[Profile]) -> editor::DestinationInfo {
        let foreign = self.foreign.map(|owner| match profiles.iter().find(|p| p.id == owner) {
            Some(p) => editor::Foreign::Profile(p.name.clone()),
            None => editor::Foreign::Deleted,
        });
        editor::DestinationInfo { free: self.free, foreign, file_system: self.file_system.clone() }
    }
}

/// What a background password job produced.
#[derive(Debug)]
enum PasswordDone {
    /// A password was checked (or changed) and saved.
    Saved,
    Recovery(String),
}

/// Which part of the form a password job answers, to show its error there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasswordOp {
    /// Check and save, or change.
    Save,
    /// Read the recovery key.
    Recovery,
}

/// A password job in progress: its profile, which part of the form it
/// answers, and where its result arrives.
type PasswordJob = (ProfileId, PasswordOp, mpsc::Receiver<Result<PasswordDone, String>>);

/// The saved password of the selected profile, and a background job on it
/// (check and save, change, read the recovery key: each derives a key with
/// Argon2id and may read a slow share).
#[derive(Debug, Default)]
struct PasswordState {
    /// Profile and whether a password is saved for it.
    saved: Option<(ProfileId, bool)>,
    job: Option<PasswordJob>,
    error: Option<(ProfileId, String)>,
    /// Recovery key on show.
    recovery: Option<(ProfileId, String)>,
    /// Why the recovery key could not be read.
    recovery_error: Option<(ProfileId, String)>,
    /// A save has just succeeded: the typed password can be forgotten.
    just_saved: bool,
}

impl PasswordState {
    fn status(&mut self, id: ProfileId) -> PasswordStatus {
        if let Some((pid, op, rx)) = &self.job {
            let (pid, op) = (*pid, *op);
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
                        self.recovery_error = None;
                    }
                    Err(e) => self.fail(pid, op, e),
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
            working: self.job.as_ref().is_some_and(|(pid, _, _)| *pid == id),
            error: self.error.as_ref().filter(|(pid, _)| *pid == id).map(|(_, e)| e.clone()),
            recovery: self.recovery.as_ref().filter(|(pid, _)| *pid == id).map(|(_, k)| k.clone()),
            recovery_error: self
                .recovery_error
                .as_ref()
                .filter(|(pid, _)| *pid == id)
                .map(|(_, e)| e.clone()),
        }
    }

    fn hide_recovery(&mut self) {
        if let Some((_, key)) = &mut self.recovery {
            zeroize::Zeroize::zeroize(key);
        }
        self.recovery = None;
        self.recovery_error = None;
    }

    /// Keeps the error of a job for the part of the form that started it.
    fn fail(&mut self, id: ProfileId, op: PasswordOp, e: String) {
        match op {
            PasswordOp::Save => self.error = Some((id, e)),
            PasswordOp::Recovery => self.recovery_error = Some((id, e)),
        }
    }

    /// Checks `pw` against the destination and saves it.
    fn save(&mut self, ctx: &egui::Context, profile: Profile, pw: Password, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, PasswordOp::Save, move || match archeion_engine::check_password(&profile, &pw) {
            Ok(()) => credentials::set(id, &pw).map(|()| PasswordDone::Saved),
            Err(e) => Err(texts.explain(&e)),
        });
    }

    /// Changes the destination's password from `current`, typed by the user
    /// (not the saved one: whoever sits at the open Windows session must
    /// know it), to `new`, then saves `new`. A wrong `current` changes
    /// nothing.
    fn change(&mut self, ctx: &egui::Context, profile: Profile, current: Password, new: Password, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, PasswordOp::Save, move || {
            archeion_engine::change_password(&profile, &current, &new).map_err(|e| texts.explain(&e))?;
            credentials::set(id, &new).map(|()| PasswordDone::Saved)
        });
    }

    /// Reads the recovery key with `pw`, the current password typed by the
    /// user (never the saved one).
    fn show_recovery(&mut self, ctx: &egui::Context, profile: Profile, pw: Password, tr: &Tr) {
        let texts = Texts::new(tr);
        let id = profile.id;
        self.spawn(ctx, id, PasswordOp::Recovery, move || {
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
        op: PasswordOp,
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
                self.job = Some((id, op, rx));
                self.error = None;
                self.recovery_error = None;
            }
            Err(e) => self.fail(id, op, e.to_string()),
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
    /// Closing the window right after a schedule edit still applies it; a
    /// deleted profile can no longer be restored, so its saved password goes.
    fn drop(&mut self) {
        if let Some((id, _)) = self.schedule_pending.take() {
            self.apply_schedule(id);
        }
        if let Some(p) = self.removed.take() {
            forget_password(p.id);
        }
    }
}

/// Removes the saved password of a profile deleted for good.
fn forget_password(id: ProfileId) {
    if let Err(e) = credentials::delete(id) {
        tracing::warn!(error = %e, "removing the saved password");
    }
}

impl ArcheionApp {
    /// Shutting down or signing out during a run shows why the window is
    /// still open (Windows ends the run otherwise without a word; an
    /// unfinished backup resumes next time).
    fn hold_shutdown(&mut self, frame: &eframe::Frame) {
        use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
        let running = self.job.as_ref().is_some_and(|j| j.activity.outcome.is_none());
        if !running {
            self.shutdown_block = None;
            return;
        }
        if self.shutdown_block.is_some() {
            return;
        }
        let hwnd = frame.window_handle().ok().and_then(|h| match h.as_raw() {
            RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
            _ => None,
        });
        if let Some(hwnd) = hwnd {
            self.shutdown_block = archeion_win::ShutdownBlock::new(hwnd, &self.tr.t("shutdown-reason"));
        }
    }
}

impl eframe::App for ArcheionApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.hold_shutdown(frame);
        self.window_events(&ctx);
        self.remember_size(&ctx);
        self.poll_wipe(&ctx);
        self.poll_adopting();
        match self.backup_import.poll(&self.tr) {
            Some(backup_import::Done::Imported { profile, password }) => {
                self.add_imported(*profile, password)
            }
            Some(backup_import::Done::Failed(text)) => self.error = Some(text),
            None => {}
        }
        self.poll();
        self.update_tray();
        if self.close_after_run && self.job.is_none() {
            // The run has ended: close now, as asked.
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.shortcuts(&ctx);
        self.flush_schedule(&ctx, false);
        self.auto_merge(&ctx);

        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        if self.close_after_run {
            egui::Panel::top("closing").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        symbol::with(symbol::WAIT, self.tr.t("closing-after-run")),
                    );
                    if ui.button(self.tr.t("closing-cancel")).clicked() {
                        self.close_after_run = false;
                        self.quitting = false;
                    }
                });
            });
        }
        if let Some(text) = &self.banner {
            egui::Panel::top("banner").show(ui, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, text));
            });
        }
        if let Some(text) = self.error.clone() {
            egui::Panel::top("error").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, text));
                    if widgets::icon_button(ui, symbol::REMOVE, self.tr.t("close")).clicked() {
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
        self.help.show(&ctx, &self.tr, &self.storage);
        self.backup_import.show(&ctx, &self.tr);
        let current = settings::Values { language: self.settings.language, window: self.settings.window };
        let changed = self.preferences.show(&ctx, &self.tr, self.storage.dir(), self.tray.is_some(), current);
        if let Some(values) = changed {
            self.apply_preferences(values);
        }
    }
}

/// The scheduled tasks of `p`: backups and merges by the clock, backups
/// by events.
fn task_specs(p: &Profile, exe: &std::path::Path, user: &str) -> [TaskSpec; 3] {
    [
        (TaskKind::Backup, if p.schedule_paused { Schedule::Manual } else { p.schedule.clone() }),
        // Merges run from the window now: a merge task left by an older
        // version is removed.
        (TaskKind::Merge, Schedule::Manual),
        (TaskKind::Events, Schedule::Manual),
    ]
    .map(|(kind, schedule)| TaskSpec {
        profile_id: p.id,
        profile_name: p.name.clone(),
        kind,
        schedule,
        // Backups by events are not offered in the window: no such task.
        events: archeion_core::EventTriggers::default(),
        executable: exe.to_path_buf(),
        user: user.to_owned(),
    })
}

/// True when some profile wants to be warned after days without backups.
fn watch_wanted(profiles: &[Profile]) -> bool {
    profiles.iter().any(|p| p.alerts.stale_days > 0 && p.backs_up_automatically())
}

/// `DOMAIN\user` of the signed-in user, for logon triggers.
fn windows_user() -> String {
    let user = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    }
}

/// [`sync_tasks`] for the watch task.
fn sync_watch(wanted: bool, exe: &std::path::Path, user: &str) -> Vec<String> {
    let registered = match WindowsTaskScheduler.watch_executable() {
        Ok(r) => r,
        Err(SchedulerError::Unsupported) => return Vec::new(),
        Err(e) => {
            tracing::warn!(error = %e, "reading the watch task");
            return Vec::new();
        }
    };
    if !archeion_scheduler::needs_update_for(registered.as_deref(), wanted, exe) {
        return Vec::new();
    }
    match WindowsTaskScheduler.apply_watch(wanted, exe, user) {
        Ok(()) => Vec::new(),
        Err(e) => {
            tracing::error!(error = %e, "updating the watch task");
            vec![e.to_string()]
        }
    }
}

/// Registers again the tasks of `specs` that are missing, start another
/// executable, or should not exist (manual schedule). Returns the problems
/// met.
fn sync_tasks(specs: &[TaskSpec]) -> Vec<String> {
    let mut problems = Vec::new();
    for spec in specs {
        let registered = match WindowsTaskScheduler.registered_executable(spec.profile_id, spec.kind) {
            Ok(r) => r,
            Err(SchedulerError::Unsupported) => return problems,
            Err(e) => {
                tracing::warn!(error = %e, kind = ?spec.kind, "reading a scheduled task");
                continue;
            }
        };
        if archeion_scheduler::needs_update(registered.as_deref(), spec) {
            tracing::info!(profile = %spec.profile_id, kind = ?spec.kind, ?registered, "updating a scheduled task");
            if let Err(e) = WindowsTaskScheduler.apply(spec) {
                tracing::error!(error = %e, kind = ?spec.kind, "updating a scheduled task");
                problems.push(e.to_string());
            }
        }
    }
    problems
}

impl ArcheionApp {
    /// Requests from outside the window's widgets: the tray icon, another
    /// launch of Archeion, the start in the notification area, and closing.
    fn window_events(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.start_hidden) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        let commands = self.tray.as_ref().map(Tray::commands).unwrap_or_default();
        for cmd in commands {
            match cmd {
                TrayCommand::Show => show_window(ctx),
                TrayCommand::Quit => self.quit(ctx),
            }
        }
        if self.wake.as_ref().is_some_and(|rx| rx.try_iter().count() > 0) {
            show_window(ctx);
        }
        self.handle_close(ctx);
    }

    /// Quit from the menu or the tray: the window closes for good (after
    /// the run in progress, if any, which it then shows).
    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        if self.job.is_some() {
            show_window(ctx);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// A close request (window ✕, Alt+F4, Quit). With the notification-area
    /// icon, ✕ only hides the window: runs, scheduled tasks and the
    /// automatic merge go on. When really closing during a run, the window
    /// stays, says it will close at the end, and closes then.
    fn handle_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if !self.quitting && self.hides_to_tray() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            return;
        }
        if self.job.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.close_after_run {
                tracing::info!("close requested during a run: closing when it ends");
            }
            self.close_after_run = true;
        }
    }

    /// The tray tooltip follows the run in progress.
    fn update_tray(&mut self) {
        let running = self.job.is_some().then(|| self.tr.t("tray-running"));
        if let Some(tray) = &mut self.tray {
            tray.set_running(running.as_deref());
        }
    }

    /// Saves and applies what the user changed in the Preferences window.
    fn apply_preferences(&mut self, values: settings::Values) {
        let before = self.settings.window;
        let after = values.window;
        let language_changed = values.language != self.settings.language;
        self.settings.language = values.language;
        self.settings.window = after;
        self.save();
        // Every text is translated while drawing, so the window switches
        // at the next frame. Messages already shown (error, banner) keep
        // their language; the tray menu changes at the next start.
        if language_changed {
            self.tr = Tr::new(values.language);
        }
        if after.close_to_tray != before.close_to_tray
            && let Some(tray) = &self.tray
        {
            tray.set_visible(after.close_to_tray);
        }
        if after.start_with_windows != before.start_with_windows {
            let result =
                std::env::current_exe().and_then(|exe| autostart::sync(after.start_with_windows, &exe));
            if let Err(e) = result {
                tracing::error!(error = %e, "updating the start with Windows");
                self.error = Some(self.tr.t_args("autostart-failed", &[("detail", arg(e.to_string()))]));
            }
        }
    }
}

/// Brings the window back from the notification area (or the taskbar).
fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

/// Status line for a finished run and whether it needs attention.
fn outcome_text(tr: &Tr, name: &str, op: Op, o: &Outcome) -> (String, bool) {
    match o {
        Outcome::Finished(Summary::Merge(r)) if op == Op::Delete => {
            let text = tr.t_args("delete-finished", &[("name", arg(name.to_owned()))]);
            (super::activity::with_changed_outside(tr, text, r.changed_outside), r.changed_outside)
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
                (text, r.changed_outside)
            } else {
                let extra = tr.t_args("skipped-count", &[("count", arg(r.skipped.len()))]);
                (format!("{text} · {extra}"), true)
            }
        }
        Outcome::Finished(Summary::Merge(r)) => {
            let text = tr.t_args(
                "merge-finished",
                &[
                    ("name", arg(name.to_owned())),
                    ("before", arg(r.archives_before)),
                    ("after", arg(r.archives_after)),
                    ("freed", arg(tr.bytes(r.bytes_freed))),
                ],
            );
            (super::activity::with_changed_outside(tr, text, r.changed_outside), r.changed_outside)
        }
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
            (super::activity::no_space_text(tr, op, *needed, *available), true)
        }
        Outcome::DestinationUnavailable(loc) => {
            (tr.t_args("run-destination-missing", &[("location", arg(loc.clone()))]), true)
        }
        Outcome::Failed(msg) => (tr.t_args("run-failed", &[("detail", arg(msg.clone()))]), true),
        Outcome::Cancelled => (tr.t("run-cancelled"), false),
    }
}

#[cfg(test)]
#[path = "screens.rs"]
mod screens;

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;

/// A deleted profile's backups going to the Recycle Bin, on a thread of
/// their own.
#[derive(Debug)]
enum Wipe {
    Running {
        name: String,
        path: String,
        done: mpsc::Receiver<Result<archeion_engine::DestinationTrashed, EngineError>>,
    },
    /// What the status bar says once it ended.
    Done { text: String, warn: bool },
}

impl Wipe {
    fn start(profile: Profile) -> Self {
        let (tx, done) = mpsc::channel();
        let (name, path) = (profile.name.clone(), profile.destination.path().display().to_string());
        let spawned = std::thread::Builder::new().name("trash-backups".into()).spawn(move || {
            let _ = tx.send(archeion_engine::trash_destination(&profile));
        });
        if let Err(e) = spawned {
            tracing::error!(error = %e, "starting to move the backups to the Recycle Bin");
        }
        Self::Running { name, path, done }
    }
}
