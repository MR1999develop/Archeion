//! Form for one profile. It edits the profile in place and reports what the
//! window has to do next (save, update the scheduled tasks, start a job).

use super::theme;
use crate::i18n::{Tr, arg};
use archeion_core::{
    Container, Destination, Exclusion, FullBackupRule, Profile, ProfileError, Retention, Schedule, TimeOfDay,
    Weekday, profile::Verification,
};
use archeion_engine::Password;
use eframe::egui::{self, RichText, Ui};

/// Shortest password accepted for a new encrypted destination.
const MIN_PASSWORD_CHARS: usize = 12;

/// What the form asks the window to do after this frame.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Something changed: save the settings.
    pub changed: bool,
    /// A schedule changed: update the scheduled tasks.
    pub schedule_changed: bool,
    pub command: Option<Command>,
}

/// Whether the storage format and encryption can still be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatLock {
    /// Looking at the destination.
    Checking,
    /// No backups at the destination yet.
    Free,
    /// The destination already holds backups stored this way.
    Locked { container: Container, encrypted: bool },
    /// The destination cannot be reached: the format stays as it is.
    Unreachable,
}

/// What the user asks "Run now" to do on a profile that has backups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunChoice {
    #[default]
    Incremental,
    Complete,
    /// Apply the merge rules now.
    Merge,
}

/// Text being typed and choices made in the form, kept by the window
/// between frames; never saved.
#[derive(Debug, Default)]
pub struct Drafts {
    pub extension: String,
    pub pattern: String,
    pub run: RunChoice,
    pub password: String,
    pub password_again: String,
    /// "I understand" that a lost password means lost backups.
    pub password_ack: bool,
}

impl Drafts {
    /// Forgets the typed passwords (after saving, or on another profile).
    pub fn clear_password(&mut self) {
        self.password.clear();
        self.password_again.clear();
        self.password_ack = false;
    }
}

/// The saved password of an encrypted profile, as the window knows it.
#[derive(Debug, Clone, Default)]
pub struct PasswordStatus {
    /// Saved in Credential Manager; `None` while unknown.
    pub saved: Option<bool>,
    /// A typed password is being checked and saved.
    pub working: bool,
    /// Why the last save failed.
    pub error: Option<String>,
    /// The recovery key, while shown.
    pub recovery: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Command {
    Run(RunChoice),
    Delete,
    /// Check the password against the destination, then save it.
    SavePassword(Password),
    /// Re-wrap the destination's key with this new password, then save it.
    ChangePassword(Password),
    ShowRecoveryKey,
    HideRecoveryKey,
    /// Integrity check of every stored file, or a restore test of this
    /// many files.
    Verify(Option<usize>),
}

/// Draws the form. `busy` disables the run buttons while a job runs.
pub fn show(
    ui: &mut Ui,
    tr: &Tr,
    p: &mut Profile,
    format: FormatLock,
    password: &PasswordStatus,
    drafts: &mut Drafts,
    busy: bool,
) -> Outcome {
    let mut out = Outcome::default();
    ui.set_max_width(theme::FORM_MAX_WIDTH);

    section(ui, &tr.t("profile-name"));
    out.changed |= ui.text_edit_singleline(&mut p.name).changed();

    section(ui, &tr.t("folders"));
    folders(ui, tr, p, &mut out);

    section(ui, &tr.t("exclusions"));
    exclusions(ui, tr, p, drafts, &mut out);

    section(ui, &tr.t("destination"));
    destination(ui, tr, &mut p.destination, &mut out);

    section(ui, &tr.t("container"));
    container(ui, tr, p, format, &mut out);
    encryption(ui, tr, p, format, password, drafts, &mut out);

    section(ui, &tr.t("schedule"));
    schedule(ui, tr, "backup", &mut p.schedule, &mut out);
    if p.schedule.is_automatic() {
        ui.weak(tr.t("schedule-hint"));
        full_backup(ui, tr, &mut p.full_backup, &mut out);
    }

    section(ui, &tr.t("merge"));
    ui.weak(tr.t("merge-hint"));
    schedule(ui, tr, "merge", &mut p.merge.schedule, &mut out);
    retention(ui, tr, &mut p.merge.rules, &mut out);

    section(ui, &tr.t("verify-section"));
    verification(ui, tr, p, format, &mut out, busy);

    ui.add_space(theme::SPACE_L);
    run_now(ui, tr, p, format, password, drafts, busy, &mut out);
    out
}

/// When the backups are checked, and buttons to check them now.
fn verification(ui: &mut Ui, tr: &Tr, p: &mut Profile, format: FormatLock, out: &mut Outcome, busy: bool) {
    ui.weak(tr.t("verify-hint"));
    let v: &mut Verification = &mut p.verification;
    egui::Grid::new("verification").num_columns(2).show(ui, |ui| {
        ui.label(tr.t("verify-full-every"));
        ui.horizontal(|ui| {
            out.changed |= ui.add(egui::DragValue::new(&mut v.check_every_days).range(0..=365)).changed();
            ui.label(tr.t("full-days-unit"));
        });
        ui.end_row();
        ui.label(tr.t("verify-sample-every"));
        ui.horizontal(|ui| {
            out.changed |=
                ui.add(egui::DragValue::new(&mut v.restore_test_every_days).range(0..=365)).changed();
            ui.label(tr.t("full-days-unit"));
            ui.label(tr.t("verify-sample-files"));
            out.changed |= ui.add(egui::DragValue::new(&mut v.restore_test_files).range(1..=1000)).changed();
        });
        ui.end_row();
    });
    ui.weak(tr.t("verify-zero-hint"));
    let has_backups = matches!(format, FormatLock::Locked { .. });
    let why = if busy { tr.t("busy-hint") } else { tr.t("verify-no-backups") };
    let ready = has_backups && !busy;
    let files = usize::try_from(v.restore_test_files.max(1)).unwrap_or(1);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(ready, egui::Button::new(tr.t("verify-full-now")))
            .on_hover_text(tr.t("verify-full-hint"))
            .on_disabled_hover_text(&why)
            .clicked()
        {
            out.command = Some(Command::Verify(None));
        }
        if ui
            .add_enabled(ready, egui::Button::new(tr.t("verify-sample-now")))
            .on_hover_text(tr.t("verify-sample-hint"))
            .on_disabled_hover_text(&why)
            .clicked()
        {
            out.command = Some(Command::Verify(Some(files)));
        }
    });
}

/// The "Run now" area: on a profile with backups, a choice between an
/// incremental backup, a complete one and a merge.
#[expect(clippy::too_many_arguments, reason = "one call site, all parts of the form")]
fn run_now(
    ui: &mut Ui,
    tr: &Tr,
    p: &Profile,
    format: FormatLock,
    password: &PasswordStatus,
    drafts: &mut Drafts,
    busy: bool,
    out: &mut Outcome,
) {
    let errors = p.validate().err().unwrap_or_default();
    for e in &errors {
        ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", error_text(tr, *e)));
    }
    let encrypted = p.encrypted || matches!(format, FormatLock::Locked { encrypted: true, .. });
    let no_password = encrypted && password.saved == Some(false);
    let ready = errors.is_empty() && !busy && !no_password;
    let why = if busy {
        tr.t("busy-hint")
    } else if no_password {
        tr.t("password-first-hint")
    } else {
        tr.t("fix-profile-hint")
    };

    let started = format != FormatLock::Free;
    if started {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr.t("run-now"));
            for (choice, label, hint) in [
                (RunChoice::Incremental, "run-incremental", "run-incremental-hint"),
                (RunChoice::Complete, "run-complete", "run-complete-hint"),
                (RunChoice::Merge, "run-merge", "merge-now-hint"),
            ] {
                ui.radio_value(&mut drafts.run, choice, tr.t(label)).on_hover_text(tr.t(hint));
            }
        });
    }
    ui.horizontal(|ui| {
        let label = match (started, drafts.run) {
            (false, _) => tr.t("run-first"),
            (true, RunChoice::Incremental) => tr.t("run-incremental-button"),
            (true, RunChoice::Complete) => tr.t("run-complete-button"),
            (true, RunChoice::Merge) => tr.t("merge-now"),
        };
        let primary = egui::Button::new(RichText::new(label).strong()).fill(ui.visuals().selection.bg_fill);
        if ui.add_enabled(ready, primary).on_disabled_hover_text(&why).clicked() {
            let choice = if started { drafts.run } else { RunChoice::Complete };
            out.command = Some(Command::Run(choice));
        }
        ui.add_space(theme::SPACE_L);
        if ui.link(tr.t("delete-profile")).on_hover_text(tr.t("delete-profile-hint")).clicked() {
            out.command = Some(Command::Delete);
        }
    });
}

/// Encryption switch and password, below the format.
fn encryption(
    ui: &mut Ui,
    tr: &Tr,
    p: &mut Profile,
    format: FormatLock,
    password: &PasswordStatus,
    drafts: &mut Drafts,
    out: &mut Outcome,
) {
    // Once there are backups, the destination says whether they are encrypted.
    if let FormatLock::Locked { encrypted, .. } = format
        && p.encrypted != encrypted
    {
        p.encrypted = encrypted;
        out.changed = true;
    }
    let free = format == FormatLock::Free && p.container == Container::ZipPerFile;
    ui.add_space(theme::SPACE_S);
    let mut on = p.encrypted;
    let why =
        if p.container == Container::Folder { tr.t("encrypt-needs-zip") } else { tr.t("container-locked") };
    if ui
        .add_enabled(free, egui::Checkbox::new(&mut on, tr.t("encrypt")))
        .on_hover_text(tr.t("encrypt-hint"))
        .on_disabled_hover_text(why)
        .changed()
    {
        p.encrypted = on;
        out.changed = true;
    }
    if !p.encrypted {
        return;
    }
    ui.weak(tr.t("encrypt-hint"));

    match password.saved {
        Some(true) => {
            ui.label(format!("🔑 {}", tr.t("password-saved")));
        }
        Some(false) => {
            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("password-missing")));
        }
        None => {}
    }
    // Which password form: choose one (new destination), type it again (the
    // backups exist but this computer has no saved password), or change it.
    let existing = matches!(format, FormatLock::Locked { encrypted: true, .. });
    let change = existing && password.saved == Some(true);
    let confirm = !existing || change;
    let open = password.saved == Some(false) || password.error.is_some();
    let title = match (existing, change) {
        (false, _) => tr.t("password-set"),
        (true, false) => tr.t("password-reenter"),
        (true, true) => tr.t("password-change"),
    };
    egui::CollapsingHeader::new(title).id_salt("password").open(open.then_some(true)).show(ui, |ui| {
        egui::Grid::new("password-grid").num_columns(2).show(ui, |ui| {
            ui.label(tr.t(if change { "password-new" } else { "password" }));
            ui.add(egui::TextEdit::singleline(&mut drafts.password).password(true).desired_width(240.0));
            ui.end_row();
            if confirm {
                ui.label(tr.t("password-again"));
                ui.add(
                    egui::TextEdit::singleline(&mut drafts.password_again)
                        .password(true)
                        .desired_width(240.0),
                );
                ui.end_row();
            }
        });
        let problem = password_problem(tr, drafts, confirm);
        if confirm {
            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("password-loss-warning")));
            ui.checkbox(&mut drafts.password_ack, tr.t("merge-complete-ack"));
        }
        if let Some(e) = &password.error {
            ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {e}"));
        }
        ui.horizontal(|ui| {
            let ready = problem.is_none() && !password.working;
            let label = if change { "password-change-button" } else { "password-save" };
            let resp = ui.add_enabled(ready, egui::Button::new(tr.t(label)));
            let resp = match &problem {
                Some(why) => resp.on_disabled_hover_text(why),
                None => resp,
            };
            if resp.clicked()
                && let Some(pw) = Password::new(drafts.password.clone())
            {
                out.command =
                    Some(if change { Command::ChangePassword(pw) } else { Command::SavePassword(pw) });
            }
            if password.working {
                ui.spinner();
                ui.weak(tr.t("password-checking"));
            }
        });
    });

    if !existing {
        ui.weak(tr.t("recovery-later"));
    } else if password.saved == Some(true) {
        recovery_ui(ui, tr, password, out);
    }
}

/// The recovery key: hidden until asked for, then shown with Copy.
fn recovery_ui(ui: &mut Ui, tr: &Tr, password: &PasswordStatus, out: &mut Outcome) {
    ui.add_space(theme::SPACE_S);
    match &password.recovery {
        None => {
            if ui
                .add_enabled(!password.working, egui::Button::new(tr.t("recovery-show")))
                .on_hover_text(tr.t("recovery-hint"))
                .clicked()
            {
                out.command = Some(Command::ShowRecoveryKey);
            }
        }
        Some(key) => {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(tr.t("recovery-title"));
                ui.label(RichText::new(key).monospace().size(16.0));
                ui.weak(tr.t("recovery-hint"));
                ui.colored_label(ui.visuals().warn_fg_color, format!("⚠ {}", tr.t("recovery-warning")));
                ui.horizontal(|ui| {
                    if ui.button(tr.t("recovery-copy")).on_hover_text(tr.t("recovery-copy-hint")).clicked() {
                        ui.ctx().copy_text(key.replace(' ', ""));
                    }
                    if ui.button(tr.t("recovery-hide")).clicked() {
                        out.command = Some(Command::HideRecoveryKey);
                    }
                });
            });
        }
    }
}

/// Why the typed password cannot be saved yet, if anything. `confirm`: a
/// new password, typed twice and with the warning acknowledged.
fn password_problem(tr: &Tr, d: &Drafts, confirm: bool) -> Option<String> {
    if d.password.is_empty() {
        return Some(tr.t("password-empty-hint"));
    }
    if !confirm {
        return None;
    }
    if d.password.chars().count() < MIN_PASSWORD_CHARS {
        return Some(tr.t_args("password-short-hint", &[("count", arg(MIN_PASSWORD_CHARS))]));
    }
    if d.password != d.password_again {
        return Some(tr.t("password-mismatch-hint"));
    }
    if !d.password_ack {
        return Some(tr.t("merge-ack-hint"));
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FullEvery {
    Never,
    Backups,
    Days,
}

/// When automatic backups are complete.
fn full_backup(ui: &mut Ui, tr: &Tr, rule: &mut FullBackupRule, out: &mut Outcome) {
    let (every, mut count) = match *rule {
        FullBackupRule::Never => (FullEvery::Never, 7),
        FullBackupRule::Backups { count } => (FullEvery::Backups, count),
        FullBackupRule::Days { count } => (FullEvery::Days, count),
    };
    let mut every2 = every;
    ui.horizontal(|ui| {
        ui.label(tr.t("full-backup"));
        let label = |e: FullEvery| match e {
            FullEvery::Never => tr.t("full-never"),
            FullEvery::Backups => tr.t("full-every-backups"),
            FullEvery::Days => tr.t("full-every-days"),
        };
        egui::ComboBox::from_id_salt("full-every").selected_text(label(every)).show_ui(ui, |ui| {
            for e in [FullEvery::Never, FullEvery::Backups, FullEvery::Days] {
                ui.selectable_value(&mut every2, e, label(e));
            }
        });
        match every2 {
            FullEvery::Never => {}
            FullEvery::Backups => {
                ui.add(egui::DragValue::new(&mut count).range(2..=365));
                ui.label(tr.t("full-backups-unit"));
            }
            FullEvery::Days => {
                ui.add(egui::DragValue::new(&mut count).range(1..=365));
                ui.label(tr.t("full-days-unit"));
            }
        }
    });
    ui.weak(tr.t("full-backup-hint"));
    let new = match every2 {
        FullEvery::Never => FullBackupRule::Never,
        FullEvery::Backups => FullBackupRule::Backups { count: count.max(2) },
        FullEvery::Days => FullBackupRule::Days { count: count.max(1) },
    };
    if new != *rule {
        *rule = new;
        out.changed = true;
    }
}

fn container(ui: &mut Ui, tr: &Tr, p: &mut Profile, format: FormatLock, out: &mut Outcome) {
    // The destination's format is the one that counts once it has backups.
    if let FormatLock::Locked { container: c, .. } = format
        && p.container != c
    {
        p.container = c;
        out.changed = true;
    }
    let free = format == FormatLock::Free;
    let why = match format {
        FormatLock::Locked { .. } => tr.t("container-locked"),
        FormatLock::Unreachable => tr.t("container-unreachable"),
        FormatLock::Checking => tr.t("container-checking"),
        FormatLock::Free => String::new(),
    };
    ui.horizontal(|ui| {
        for (c, label) in
            [(Container::ZipPerFile, "container-zip-per-file"), (Container::Folder, "container-folder")]
        {
            let resp = ui.add_enabled(free, egui::RadioButton::new(p.container == c, tr.t(label)));
            if resp.clicked() && p.container != c {
                p.container = c;
                if c == Container::Folder {
                    p.encrypted = false;
                }
                out.changed = true;
            }
            resp.on_disabled_hover_text(&why);
        }
    });
    ui.weak(tr.t(match p.container {
        Container::ZipPerFile => "container-zip-per-file-hint",
        Container::Folder => "container-folder-hint",
    }));
    if !free {
        ui.weak(format!("🔒 {why}"));
    }
}

fn exclusions(ui: &mut Ui, tr: &Tr, p: &mut Profile, drafts: &mut Drafts, out: &mut Outcome) {
    if p.excludes.is_empty() {
        ui.weak(tr.t("no-exclusions"));
    }
    let mut remove = None;
    for (i, rule) in p.excludes.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.small_button("✕").on_hover_text(tr.t("remove-exclusion")).clicked() {
                remove = Some(i);
            }
            let (icon, kind) = match rule {
                Exclusion::Folder { .. } => ("📁", "exclusion-folder"),
                Exclusion::File { .. } => ("📄", "exclusion-file"),
                Exclusion::Extension { .. } => ("🏷", "exclusion-type"),
                Exclusion::Pattern { .. } => ("✱", "exclusion-pattern"),
            };
            ui.label(format!("{icon} {rule}")).on_hover_text(tr.t(kind));
        });
    }
    if let Some(i) = remove {
        p.excludes.remove(i);
        out.changed = true;
    }

    // Folder and file pickers open in the first backed-up folder.
    let start = p.sources.first().cloned();
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr.t("exclude-folder")).clicked() {
            let mut dialog = rfd::FileDialog::new().set_title(tr.t("exclude-folder"));
            if let Some(dir) = &start {
                dialog = dialog.set_directory(dir);
            }
            if let Some(dirs) = dialog.pick_folders() {
                for path in dirs {
                    out.changed |= add_rule(p, Exclusion::Folder { path });
                }
            }
        }
        if ui.button(tr.t("exclude-file")).clicked() {
            let mut dialog = rfd::FileDialog::new().set_title(tr.t("exclude-file"));
            if let Some(dir) = &start {
                dialog = dialog.set_directory(dir);
            }
            if let Some(files) = dialog.pick_files() {
                for path in files {
                    out.changed |= add_rule(p, Exclusion::File { path });
                }
            }
        }
    });
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut drafts.extension)
                .hint_text(tr.t("exclude-type-hint"))
                .desired_width(140.0),
        );
        let entered = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let rule = Exclusion::extension(&drafts.extension);
        let add = ui.add_enabled(rule.is_some(), egui::Button::new(tr.t("exclude-type")));
        if (add.clicked() || entered)
            && let Some(rule) = rule
        {
            out.changed |= add_rule(p, rule);
            drafts.extension.clear();
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.weak(tr.t("common-types"));
        for ext in ["tmp", "bak", "log", "iso", "part"] {
            let rule = Exclusion::Extension { ext: ext.into() };
            let present = p.excludes.contains(&rule);
            if ui.add_enabled(!present, egui::Button::new(format!(".{ext}")).small()).clicked() {
                out.changed |= add_rule(p, rule);
            }
        }
    });
    egui::CollapsingHeader::new(tr.t("advanced")).id_salt("exclusions-advanced").show(ui, |ui| {
        ui.weak(tr.t("exclusions-hint"));
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut drafts.pattern)
                    .hint_text("**/node_modules")
                    .desired_width(220.0),
            );
            let ready = !drafts.pattern.trim().is_empty();
            if ui.add_enabled(ready, egui::Button::new(tr.t("exclude-pattern"))).clicked() {
                let glob = drafts.pattern.trim().to_owned();
                out.changed |= add_rule(p, Exclusion::Pattern { glob });
                drafts.pattern.clear();
            }
        });
    });
}

/// Adds `rule` unless it is already there; true when added.
fn add_rule(p: &mut Profile, rule: Exclusion) -> bool {
    if p.excludes.contains(&rule) {
        false
    } else {
        p.excludes.push(rule);
        true
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(theme::SPACE_M);
    ui.strong(title);
}

fn folders(ui: &mut Ui, tr: &Tr, p: &mut Profile, out: &mut Outcome) {
    if p.sources.is_empty() {
        ui.weak(tr.t("no-folders"));
    }
    let mut remove = None;
    for (i, path) in p.sources.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.small_button("✕").on_hover_text(tr.t("remove-folder")).clicked() {
                remove = Some(i);
            }
            ui.label(path.display().to_string());
        });
    }
    if let Some(i) = remove {
        p.sources.remove(i);
        out.changed = true;
    }
    if ui.button(tr.t("add-folder")).clicked()
        && let Some(dirs) = rfd::FileDialog::new().set_title(tr.t("add-folder")).pick_folders()
    {
        for d in dirs {
            if !p.sources.contains(&d) {
                p.sources.push(d);
            }
        }
        out.changed = true;
    }
}

fn destination(ui: &mut Ui, tr: &Tr, d: &mut Destination, out: &mut Outcome) {
    let local = matches!(d, Destination::Local { .. });
    ui.horizontal(|ui| {
        if ui.radio(local, tr.t("dest-local")).clicked() && !local {
            *d = Destination::Local { path: Default::default() };
            out.changed = true;
        }
        if ui.radio(!local, tr.t("dest-network")).clicked() && local {
            *d = Destination::Network { unc: String::new() };
            out.changed = true;
        }
    });
    match d {
        Destination::Local { path } => {
            ui.horizontal(|ui| {
                let mut text = path.display().to_string();
                if ui.add(egui::TextEdit::singleline(&mut text).hint_text(r"E:\Backup\Documenti")).changed() {
                    *path = text.into();
                    out.changed = true;
                }
                if ui.button(tr.t("browse")).clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    *path = dir;
                    out.changed = true;
                }
            });
        }
        Destination::Network { unc } => {
            out.changed |= ui.add(egui::TextEdit::singleline(unc).hint_text(r"\\server\backup\pc")).changed();
        }
    }
    ui.weak(tr.t("dest-hint"));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Every {
    Manual,
    Daily,
    Weekly,
}

fn schedule(ui: &mut Ui, tr: &Tr, id: &str, s: &mut Schedule, out: &mut Outcome) {
    let (every, day, at) = match *s {
        Schedule::Manual => (Every::Manual, Weekday::Sunday, TimeOfDay::default()),
        Schedule::Daily { at } => (Every::Daily, Weekday::Sunday, at),
        Schedule::Weekly { day, at } => (Every::Weekly, day, at),
    };
    let (mut every2, mut day2) = (every, day);
    let (mut hour, mut minute) = (at.hour(), at.minute());
    ui.horizontal(|ui| {
        let label = |e: Every| match e {
            Every::Manual => tr.t("schedule-manual"),
            Every::Daily => tr.t("schedule-daily"),
            Every::Weekly => tr.t("schedule-weekly"),
        };
        egui::ComboBox::from_id_salt(("every", id)).selected_text(label(every)).show_ui(ui, |ui| {
            for e in [Every::Manual, Every::Daily, Every::Weekly] {
                ui.selectable_value(&mut every2, e, label(e));
            }
        });
        if every2 == Every::Weekly {
            egui::ComboBox::from_id_salt(("weekday", id)).selected_text(day_name(tr, day2)).show_ui(
                ui,
                |ui| {
                    for d in Weekday::ALL {
                        ui.selectable_value(&mut day2, d, day_name(tr, d));
                    }
                },
            );
        }
        if every2 != Every::Manual {
            ui.label(tr.t("at"));
            ui.add(egui::DragValue::new(&mut hour).range(0..=23));
            ui.label(":");
            ui.add(egui::DragValue::new(&mut minute).range(0..=59));
        }
    });
    let at = TimeOfDay::new(hour, minute).unwrap_or(at);
    let new = match every2 {
        Every::Manual => Schedule::Manual,
        Every::Daily => Schedule::Daily { at },
        Every::Weekly => Schedule::Weekly { day: day2, at },
    };
    if new != *s {
        *s = new;
        out.changed = true;
        out.schedule_changed = true;
    }
}

fn retention(ui: &mut Ui, tr: &Tr, r: &mut Retention, out: &mut Outcome) {
    ui.add_space(theme::SPACE_S);
    egui::Grid::new("retention").num_columns(2).show(ui, |ui| {
        for (key, value) in [
            ("keep-last", &mut r.keep_last),
            ("keep-daily", &mut r.keep_daily),
            ("keep-weekly", &mut r.keep_weekly),
            ("keep-monthly", &mut r.keep_monthly),
            ("keep-yearly", &mut r.keep_yearly),
        ] {
            ui.label(tr.t(key));
            out.changed |= ui.add(egui::DragValue::new(value).range(0..=999)).changed();
            ui.end_row();
        }
    });
    ui.weak(tr.t(if r.keeps_everything() { "keep-everything" } else { "retention-hint" }));
}

fn day_name(tr: &Tr, d: Weekday) -> String {
    tr.t(match d {
        Weekday::Monday => "day-monday",
        Weekday::Tuesday => "day-tuesday",
        Weekday::Wednesday => "day-wednesday",
        Weekday::Thursday => "day-thursday",
        Weekday::Friday => "day-friday",
        Weekday::Saturday => "day-saturday",
        Weekday::Sunday => "day-sunday",
    })
}

fn error_text(tr: &Tr, e: ProfileError) -> String {
    tr.t(match e {
        ProfileError::EmptyName => "error-empty-name",
        ProfileError::NoSources => "error-no-sources",
        ProfileError::NoDestination => "error-no-destination",
        ProfileError::DestinationInsideSource => "error-destination-in-source",
        ProfileError::EncryptionNeedsZip => "encrypt-needs-zip",
    })
}
