//! Form for one profile. It edits the profile in place and reports what the
//! window has to do next (save, update the scheduled tasks, start a job).

use super::{symbol, theme, widgets};
use crate::i18n::{Tr, arg};
use archeion_core::{
    Alerts, ArchiveName, Container, Destination, Exclusion, FullBackupRule, LargeFiles, Profile,
    ProfileError, ProfileId, Retention, Schedule, TimeOfDay, TypedKind, TypedRuleError, Weekday, merge_plan,
    password_policy::{self as policy, Rule, Strength},
    profile::Verification,
};
use archeion_engine::Password;
use eframe::egui::{self, RichText, Ui};

const DELETE_DIALOG_ID: &str = "delete-profile-dialog";

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
    /// An imported profile's network folders are not confirmed: not looked at.
    Unconfirmed,
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
    /// A pattern or file type being typed in the exclusions.
    pub rule: String,
    /// Why the typed rule was not added.
    pub rule_error: Option<String>,
    pub run: RunChoice,
    /// The "Delete profile" warning, while open.
    delete: Option<DeleteDialog>,
    pub password: String,
    pub password_again: String,
    /// "I understand" that a lost password means lost backups.
    pub password_ack: bool,
    /// The current password, asked before changing it.
    pub current_password: String,
    /// The current password field of the recovery key is shown.
    pub recovery_open: bool,
    /// The current password, asked before showing the recovery key.
    pub recovery_password: String,
}

impl Drafts {
    /// Opens the "Delete profile" warning for the profile `id`.
    pub fn ask_delete(&mut self, id: ProfileId) {
        self.delete = Some(DeleteDialog { profile: id, focus_cancel: true, backups: false });
    }

    /// Forgets the typed passwords (after saving, or on another profile).
    /// Forgets what was typed, overwriting the memory it used.
    pub fn clear_password(&mut self) {
        use zeroize::Zeroize as _;
        self.password.zeroize();
        self.password_again.zeroize();
        self.password_ack = false;
        self.current_password.zeroize();
        self.recovery_open = false;
        self.recovery_password.zeroize();
    }
}

/// The "Delete profile" warning, while open.
#[derive(Debug)]
struct DeleteDialog {
    /// The profile it asks about: on another profile it closes.
    profile: ProfileId,
    /// Cancel takes the keyboard focus on the first frame, so Enter
    /// cannot delete.
    focus_cancel: bool,
    /// Move the backups at the destination to the Recycle Bin too.
    backups: bool,
}

/// The saved password of an encrypted profile, as the window knows it.
#[derive(Debug, Clone, Default)]
pub struct PasswordStatus {
    /// Saved in Credential Manager; `None` while unknown.
    pub saved: Option<bool>,
    /// A typed password is being checked and saved.
    pub working: bool,
    /// Why the last save or change failed.
    pub error: Option<String>,
    /// The recovery key, while shown.
    pub recovery: Option<String>,
    /// Why the recovery key could not be read.
    pub recovery_error: Option<String>,
}

/// Which password form an encrypted profile shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasswordForm {
    /// No backups yet: a new password, typed twice.
    Choose,
    /// Backups exist but this computer has no saved password: typed once.
    Reenter,
    /// A password is saved: the current one, then the new one twice.
    Change,
}

impl PasswordForm {
    /// A new password, typed twice and with the warning acknowledged.
    fn confirm(self) -> bool {
        self != Self::Reenter
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Run(RunChoice),
    /// Check the password against the destination, then save it.
    SavePassword(Password),
    /// Save the typed password as [`Command::SavePassword`], then run: the
    /// "Run" button of an encrypted profile whose password is typed but
    /// not saved yet.
    SavePasswordAndRun(Password, RunChoice),
    /// Re-wrap the destination's key, opened with the typed current
    /// password, with the new one, then save the new one.
    ChangePassword {
        current: Password,
        new: Password,
    },
    /// Read the recovery key with this typed current password.
    ShowRecoveryKey(Password),
    /// Hide the recovery key, or stop asking the password for it.
    HideRecoveryKey,
    /// Integrity check of every stored file, or a restore test of this
    /// many files.
    Verify(Option<usize>),
    /// Give the backups of a deleted profile, found at the destination, to
    /// this profile.
    AdoptDestination,
}

/// What the window knows about the destination folder of the profile.
#[derive(Debug, Clone, Default)]
pub struct DestinationInfo {
    /// Free bytes on its drive, when known.
    pub free: Option<u64>,
    /// The folder holds the backups of another profile.
    pub foreign: Option<Foreign>,
    /// File system of its drive ("NTFS", "exFAT"…), when known.
    pub file_system: Option<String>,
    /// The backups in it, oldest first, once read.
    pub archives: Option<Vec<ArchiveName>>,
}

/// Owner of a destination that belongs to another profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Foreign {
    /// A profile that still exists, by name: the folder cannot be shared.
    Profile(String),
    /// A profile deleted since: its backups can be taken over.
    Deleted,
}

/// Draws the form, one collapsible section per topic (each remembers
/// whether it is open). `busy` disables the run buttons while a job runs.
#[expect(clippy::too_many_arguments, reason = "one call site, all parts of the form")]
pub fn show(
    ui: &mut Ui,
    tr: &Tr,
    p: &mut Profile,
    format: FormatLock,
    dest: &DestinationInfo,
    password: &PasswordStatus,
    drafts: &mut Drafts,
    busy: bool,
) -> Outcome {
    let mut out = Outcome::default();
    ui.set_max_width(theme::SETTINGS_MAX_WIDTH);

    // One field: a row, not a section.
    ui.add_space(theme::SPACE_XS);
    ui.horizontal(|ui| {
        let label = ui.label(widgets::section_text(tr.t("profile-name")));
        let resp = ui.text_edit_singleline(&mut p.name).labelled_by(label.id);
        out.changed |= resp.changed();
    });
    network_confirmation(ui, tr, p, &mut out);
    section(ui, "folders", &tr.t("folders"), true, false, |ui| folders(ui, tr, p, &mut out));
    section(ui, "exclusions", &tr.t("exclusions"), false, false, |ui| {
        exclusions(ui, tr, p, drafts, &mut out)
    });
    // A destination of another profile must be seen: kept open.
    section(ui, "destination", &tr.t("destination"), true, dest.foreign.is_some(), |ui| {
        destination(ui, tr, &mut p.destination, dest, &mut out);
    });
    // The password form must be seen while the password is missing.
    let needs_password = p.encrypted && password.saved == Some(false);
    section(ui, "container", &tr.t("container"), false, needs_password, |ui| {
        container(ui, tr, p, format, &mut out);
        encryption(ui, tr, p, format, password, drafts, &mut out);
        large_files(ui, tr, p, format, &mut out);
    });
    section(ui, "schedule", &tr.t("schedule"), true, false, |ui| {
        if p.schedule_paused {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("schedule-paused")),
            );
            if ui.button(tr.t("schedule-resume")).clicked() {
                p.schedule_paused = false;
                out.changed = true;
                out.schedule_changed = true;
            }
        }
        schedule(ui, tr, "backup", &mut p.schedule, &mut out);
        if p.schedule.is_automatic() {
            ui.weak(tr.t("schedule-hint"));
            full_backup(ui, tr, &mut p.full_backup, &mut out);
        }
    });
    section(ui, "merge", &tr.t("merge"), false, false, |ui| {
        ui.weak(tr.t("merge-hint"));
        ui.weak(tr.t("merge-automatic-hint"));
        let keep = widgets::checkbox(ui, &mut p.merge.keep_deleted, tr.t("merge-keep-deleted"));
        out.changed |= keep.on_hover_text(tr.t("merge-keep-deleted-hint")).changed();
        let mut rules = p.merge.rules;
        retention(ui, tr, &mut rules, &mut out);
        let now = jiff::Zoned::now().datetime();
        if rules != p.merge.rules {
            p.merge.set_rules(rules, dest.archives.as_deref(), now);
        }
        merge_preview(ui, tr, p, dest.archives.as_deref(), now, &mut out);
    });
    section(ui, "checks", &tr.t("verify-section"), false, false, |ui| {
        verification(ui, tr, p, format, &mut out, busy);
    });
    section(ui, "alerts", &tr.t("alerts"), false, false, |ui| alerts(ui, tr, p, &mut out));

    ui.add_space(theme::SPACE_S);
    run_now(ui, tr, p, format, password, drafts, busy, dest.foreign.is_some(), &mut out);
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
    foreign: bool,
    out: &mut Outcome,
) {
    let errors = p.validate().err().unwrap_or_default();
    for e in &errors {
        ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, error_text(tr, *e)));
    }
    let encrypted = p.encrypted || matches!(format, FormatLock::Locked { encrypted: true, .. });
    let no_password = encrypted && password.saved == Some(false);
    // A password typed in the form above is saved by the Run button itself:
    // for a new destination it must be valid and confirmed, for existing
    // backups it is checked when saved.
    let existing = matches!(format, FormatLock::Locked { encrypted: true, .. });
    let form = if existing { PasswordForm::Reenter } else { PasswordForm::Choose };
    let typed = no_password && password_problem(tr, drafts, form, &p.name).is_none();
    let password_blocks = no_password && !typed;
    let ready = errors.is_empty() && !busy && !password_blocks && !password.working && !foreign;
    let why = if busy {
        tr.t("busy-hint")
    } else if foreign {
        tr.t("dest-foreign-hint")
    } else if password_blocks {
        tr.t("password-first-hint")
    } else {
        tr.t("fix-profile-hint")
    };
    if password_blocks && errors.is_empty() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            symbol::with(symbol::WARNING, tr.t("password-first-hint")),
        );
    }

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
        let primary = widgets::primary_button(ui, label);
        if ui.add_enabled(ready, primary).on_disabled_hover_text(&why).clicked() {
            let choice = if started { drafts.run } else { RunChoice::Complete };
            out.command = Some(match Password::new(drafts.password.clone()) {
                Some(pw) if typed => Command::SavePasswordAndRun(pw, choice),
                _ => Command::Run(choice),
            });
        }
    });
}

/// The warning opened by "Delete profile…" ([`Drafts::ask_delete`]), shown
/// whatever the tab: what goes, what stays, and once confirmed whether the
/// backups go too (`Some(backups)`). The backups at the destination can go
/// to the Recycle Bin too, with a tick; the warning then says where they
/// go and that Undo no longer brings the profile back. Cancel, Esc and a
/// click outside close it; Enter does not delete. Deleting waits while a
/// job runs.
pub fn delete_dialog(ui: &Ui, tr: &Tr, p: &Profile, drafts: &mut Drafts, busy: bool) -> Option<bool> {
    let dialog = drafts.delete.as_mut()?;
    if dialog.profile != p.id {
        drafts.delete = None;
        return None;
    }
    let focus_cancel = std::mem::take(&mut dialog.focus_cancel);
    let (mut delete, mut cancel) = (false, false);
    let response = widgets::modal(ui.ctx(), egui::Id::unique(DELETE_DIALOG_ID)).show(ui.ctx(), |ui| {
        ui.set_max_width(theme::DIALOG_WIDTH);
        ui.heading(tr.t_args("delete-profile-title", &[("name", arg(p.name.clone()))]));
        ui.add_space(theme::SPACE_S);
        // With the backups, no Undo: the text says where it all goes.
        ui.label(tr.t(if dialog.backups { "delete-profile-body-backups" } else { "delete-profile-body" }));
        if p.encrypted {
            ui.label(tr.t("delete-profile-password"));
        }
        if !p.destination.is_empty() {
            let path = p.destination.path().display().to_string();
            ui.add_space(theme::SPACE_XS);
            let label = tr.t_args("delete-profile-with-backups", &[("path", arg(path.clone()))]);
            widgets::checkbox(ui, &mut dialog.backups, label);
            if dialog.backups {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    symbol::with(symbol::WARNING, tr.t("delete-profile-backups-gone")),
                );
            } else {
                ui.label(tr.t_args("delete-profile-backups", &[("path", arg(path))]));
            }
        }
        if busy {
            ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, tr.t("busy-hint")));
        }
        ui.add_space(theme::SPACE_S);
        // Inside a row, so the right-aligned layout takes one line, not
        // the whole height left. Windows order: action, then Cancel.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cancel_button = ui.button(tr.t("cancel"));
                if focus_cancel {
                    cancel_button.request_focus();
                }
                cancel = cancel_button.clicked();
                let label = if dialog.backups { "delete-profile-and-backups" } else { "delete-profile" };
                let button = widgets::danger_button(ui, tr.t(label));
                delete = ui.add_enabled(!busy, button).on_disabled_hover_text(tr.t("busy-hint")).clicked();
            });
        });
    });
    if delete {
        let backups = dialog.backups;
        drafts.delete = None;
        return Some(backups);
    }
    if cancel || response.should_close() {
        drafts.delete = None;
    }
    None
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
        .add_enabled_ui(free, |ui| widgets::checkbox(ui, &mut on, tr.t("encrypt")))
        .inner
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
            ui.label(symbol::with(symbol::KEY, tr.t("password-saved")));
        }
        Some(false) => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("password-missing")),
            );
        }
        None => {}
    }
    // Which password form: choose one (new destination), type it again (the
    // backups exist but this computer has no saved password), or change it.
    let existing = matches!(format, FormatLock::Locked { encrypted: true, .. });
    let change = existing && password.saved == Some(true);
    let form = match (existing, change) {
        (false, _) => PasswordForm::Choose,
        (true, false) => PasswordForm::Reenter,
        (true, true) => PasswordForm::Change,
    };
    let confirm = form.confirm();
    let open = password.saved == Some(false) || password.error.is_some();
    let title = match (existing, change) {
        (false, _) => tr.t("password-set"),
        (true, false) => tr.t("password-reenter"),
        (true, true) => tr.t("password-change"),
    };
    egui::CollapsingHeader::new(title).id_salt("password").open(open.then_some(true)).show(ui, |ui| {
        egui::Grid::new("password-grid").num_columns(2).show(ui, |ui| {
            if change {
                ui.label(tr.t("password-current"));
                ui.add(
                    egui::TextEdit::singleline(&mut drafts.current_password)
                        .password(true)
                        .desired_width(theme::PASSWORD_FIELD_WIDTH),
                );
                ui.end_row();
            }
            ui.label(tr.t(if change { "password-new" } else { "password" }));
            ui.add(
                egui::TextEdit::singleline(&mut drafts.password)
                    .password(true)
                    .desired_width(theme::PASSWORD_FIELD_WIDTH),
            );
            ui.end_row();
            if confirm {
                ui.label(tr.t("password-again"));
                ui.add(
                    egui::TextEdit::singleline(&mut drafts.password_again)
                        .password(true)
                        .desired_width(theme::PASSWORD_FIELD_WIDTH),
                );
                ui.end_row();
            }
        });
        let problem = password_problem(tr, drafts, form, &p.name);
        if confirm {
            password_rules(ui, tr, drafts, &p.name);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("password-loss-warning")),
            );
            widgets::checkbox(ui, &mut drafts.password_ack, tr.t("merge-complete-ack"));
        }
        if let Some(e) = &password.error {
            ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, e));
        }
        ui.horizontal(|ui| {
            let ready = problem.is_none() && !password.working;
            let label = if change { "password-change-button" } else { "password-save" };
            let resp = ui.add_enabled(ready, egui::Button::new(tr.t(label)));
            let resp = match &problem {
                Some(why) => resp.on_disabled_hover_text(why),
                None => resp,
            };
            if resp.clicked() {
                out.command = password_command(drafts, form);
            }
            // What is still missing, in plain sight rather than only in the
            // tooltip of the disabled button.
            if let Some(why) = &problem
                && !password.working
            {
                ui.weak(why);
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
        recovery_ui(ui, tr, password, drafts, out);
    }
}

/// The command of the password form's button. The typed current password
/// goes with the command and is not kept: a wrong one is typed again.
fn password_command(d: &mut Drafts, form: PasswordForm) -> Option<Command> {
    let new = Password::new(d.password.clone())?;
    match form {
        PasswordForm::Change => Password::new(std::mem::take(&mut d.current_password))
            .map(|current| Command::ChangePassword { current, new }),
        PasswordForm::Choose | PasswordForm::Reenter => Some(Command::SavePassword(new)),
    }
}

/// The recovery key: hidden until asked for, then read with the current
/// password typed by the user (never the saved one) and shown with Copy.
fn recovery_ui(ui: &mut Ui, tr: &Tr, password: &PasswordStatus, drafts: &mut Drafts, out: &mut Outcome) {
    ui.add_space(theme::SPACE_S);
    let field_id = egui::Id::unique("recovery-password");
    match &password.recovery {
        None if !drafts.recovery_open => {
            if ui
                .add_enabled(!password.working, egui::Button::new(tr.t("recovery-show")))
                .on_hover_text(tr.t("recovery-hint"))
                .clicked()
            {
                drafts.recovery_open = true;
                ui.ctx().memory_mut(|m| m.request_focus(field_id));
            }
        }
        None => {
            ui.label(tr.t("recovery-ask"));
            ui.horizontal(|ui| {
                ui.label(tr.t("password-current"));
                let field = ui.add(
                    egui::TextEdit::singleline(&mut drafts.recovery_password)
                        .id(field_id)
                        .password(true)
                        .desired_width(theme::PASSWORD_FIELD_WIDTH),
                );
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let ready = !drafts.recovery_password.is_empty() && !password.working;
                let show = ui
                    .add_enabled(ready, egui::Button::new(tr.t("recovery-show-button")))
                    .on_disabled_hover_text(tr.t("password-current-hint"))
                    .clicked();
                if (show || (entered && ready))
                    && let Some(pw) = Password::new(std::mem::take(&mut drafts.recovery_password))
                {
                    out.command = Some(Command::ShowRecoveryKey(pw));
                }
                if ui.add_enabled(!password.working, egui::Button::new(tr.t("cancel"))).clicked() {
                    drafts.recovery_open = false;
                    drafts.recovery_password.clear();
                    out.command = Some(Command::HideRecoveryKey);
                }
                if password.working {
                    ui.spinner();
                    ui.weak(tr.t("password-checking"));
                }
            });
            if let Some(e) = &password.recovery_error {
                ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, e));
            }
        }
        Some(key) => {
            drafts.recovery_open = false;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                widgets::section_title(ui, tr.t("recovery-title"));
                ui.label(RichText::new(key).monospace().size(theme::FONT_KEY));
                ui.weak(tr.t("recovery-hint"));
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    symbol::with(symbol::WARNING, tr.t("recovery-warning")),
                );
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

/// The requirements of a new password, ticked as they are met while typing
/// (a mark and a text, never colour alone), and the estimated strength.
fn password_rules(ui: &mut Ui, tr: &Tr, d: &Drafts, profile_name: &str) {
    let a = policy::assess(&d.password, profile_name);
    let matched = !d.password.is_empty() && d.password == d.password_again;
    let lines = a.checks.iter().map(|c| (c.met, rule_text(tr, c.rule, a.chars)));
    for (met, text) in lines.chain([(matched, tr.t("password-rule-match"))]) {
        if met {
            ui.label(symbol::with(symbol::DONE, text));
        } else {
            ui.weak(symbol::with(symbol::PENDING, text));
        }
    }
    if !d.password.is_empty() {
        // The level is written next to the bar: the bar's colour only repeats it.
        let fill = match a.strength {
            Strength::Weak => Some(ui.visuals().error_fg_color),
            Strength::Fair => Some(ui.visuals().warn_fg_color),
            Strength::Strong | Strength::VeryStrong => None,
        };
        ui.horizontal(|ui| {
            let bar = egui::ProgressBar::new(a.fraction()).desired_width(160.0);
            ui.add(match fill {
                Some(color) => bar.fill(color),
                None => bar,
            });
            ui.label(tr.t_args("password-strength", &[("level", arg(strength_text(tr, a.strength)))]));
        });
    }
    ui.weak(tr.t("password-rule-tip"));
}

/// The checklist line of a requirement; `typed` is the length so far.
fn rule_text(tr: &Tr, rule: Rule, typed: usize) -> String {
    match rule {
        Rule::Length => tr.t_args(
            "password-rule-length",
            &[("count", arg(policy::MIN_CHARS)), ("max", arg(policy::MAX_CHARS)), ("typed", arg(typed))],
        ),
        Rule::NotCommon => tr.t("password-rule-common"),
        Rule::NoRuns => tr.t_args("password-rule-runs", &[("count", arg(policy::MAX_RUN + 1))]),
        Rule::NotName => tr.t("password-rule-name"),
        Rule::Strength => {
            tr.t_args("password-rule-strength", &[("level", arg(strength_text(tr, policy::MIN_STRENGTH)))])
        }
    }
}

/// What to do about an unmet requirement, shown next to the Save button.
fn rule_hint(tr: &Tr, rule: Rule) -> String {
    match rule {
        Rule::Length => tr.t_args(
            "password-short-hint",
            &[("count", arg(policy::MIN_CHARS)), ("max", arg(policy::MAX_CHARS))],
        ),
        Rule::NotCommon => tr.t("password-common-hint"),
        Rule::NoRuns => tr.t("password-runs-hint"),
        Rule::NotName => tr.t("password-name-hint"),
        Rule::Strength => tr.t("password-weak-hint"),
    }
}

fn strength_text(tr: &Tr, s: Strength) -> String {
    tr.t(match s {
        Strength::Weak => "password-strength-weak",
        Strength::Fair => "password-strength-fair",
        Strength::Strong => "password-strength-strong",
        Strength::VeryStrong => "password-strength-very-strong",
    })
}

/// Why the typed password cannot be saved yet, if anything. The
/// requirements of [`policy`] apply only to a new password (typed twice),
/// never to the password of existing backups.
fn password_problem(tr: &Tr, d: &Drafts, form: PasswordForm, profile_name: &str) -> Option<String> {
    if form == PasswordForm::Change && d.current_password.is_empty() {
        return Some(tr.t("password-current-hint"));
    }
    if d.password.is_empty() {
        return Some(tr.t("password-empty-hint"));
    }
    if !form.confirm() {
        return None;
    }
    if let Some(rule) = policy::assess(&d.password, profile_name).first_unmet() {
        return Some(rule_hint(tr, rule));
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
        egui::ComboBox::from_id_salt("full-every")
            .selected_text(label(every))
            .show_ui(ui, |ui| {
                for e in [FullEvery::Never, FullEvery::Backups, FullEvery::Days] {
                    ui.selectable_value(&mut every2, e, label(e));
                }
            })
            .response
            .accessible_name(tr.t("full-backup"));
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
        FormatLock::Unreachable | FormatLock::Unconfirmed => tr.t("container-unreachable"),
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
        ui.weak(symbol::with(symbol::LOCKED, why));
    }
}

/// From which size files are saved as chunks, so a changed file stores
/// only its changed parts. On an encrypted destination every file is, and
/// only a note says so. Can change at any time, unlike the format.
fn large_files(ui: &mut Ui, tr: &Tr, p: &mut Profile, format: FormatLock, out: &mut Outcome) {
    ui.add_space(theme::SPACE_S);
    let encrypted = p.encrypted || matches!(format, FormatLock::Locked { encrypted: true, .. });
    if encrypted {
        ui.weak(tr.t("large-files-encrypted"));
        return;
    }
    let mut on = p.large_files.threshold_mb > 0;
    ui.horizontal(|ui| {
        if widgets::checkbox(ui, &mut on, tr.t("large-files"))
            .on_hover_text(tr.t("large-files-hint"))
            .changed()
        {
            p.large_files.threshold_mb = if on { LargeFiles::default().threshold_mb } else { 0 };
            out.changed = true;
        }
        if on {
            let value = egui::DragValue::new(&mut p.large_files.threshold_mb)
                .range(LargeFiles::THRESHOLD_RANGE_MB)
                .speed(10);
            out.changed |= ui.add(value).on_hover_text(tr.t("large-files-hint")).changed();
            ui.label(tr.t("large-files-unit"));
        }
    });
    ui.weak(tr.t("large-files-hint"));
    ui.weak(tr.t("large-files-change-hint"));
}

fn exclusions(ui: &mut Ui, tr: &Tr, p: &mut Profile, drafts: &mut Drafts, out: &mut Outcome) {
    if p.excludes.is_empty() {
        ui.weak(tr.t("no-exclusions"));
    }
    let mut remove = None;
    for (i, rule) in p.excludes.iter().enumerate() {
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, symbol::REMOVE, tr.t("remove-exclusion")).clicked() {
                remove = Some(i);
            }
            let (icon, kind) = match rule {
                Exclusion::Folder { .. } => (symbol::FOLDER, "exclusion-folder"),
                Exclusion::File { .. } => (symbol::FILE, "exclusion-file"),
                Exclusion::Extension { .. } => (symbol::TAG, "exclusion-type"),
                Exclusion::Pattern { .. } => (symbol::PATTERN, "exclusion-pattern"),
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
    // One field for a pattern or a file type; the button pressed says
    // which it is meant to be, and the text is checked against it.
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut drafts.rule)
            .hint_text(tr.t("exclude-rule-hint"))
            .desired_width(theme::RULE_FIELD_WIDTH);
        let resp =
            ui.add(field).accessible_name(tr.t("exclude-rule-hint")).on_hover_text(tr.t("exclusions-hint"));
        if resp.changed() {
            drafts.rule_error = None;
        }
        let kinds = [(TypedKind::Pattern, "exclude-pattern"), (TypedKind::Extension, "exclude-type")];
        for (kind, label) in kinds {
            if ui.button(tr.t(label)).on_hover_text(tr.t("exclusions-hint")).clicked() {
                match typed_rule(tr, kind, &drafts.rule) {
                    Ok(rule) if p.excludes.contains(&rule) => drafts.rule_error = Some(tr.t("rule-already")),
                    Ok(rule) => {
                        out.changed |= add_rule(p, rule);
                        drafts.rule.clear();
                        drafts.rule_error = None;
                    }
                    Err(text) => drafts.rule_error = Some(text),
                }
            }
        }
    });
    if let Some(e) = &drafts.rule_error {
        ui.colored_label(ui.visuals().error_fg_color, e.as_str());
    }
}

/// The rule typed `text` makes for the button `kind`, or the message that
/// says why not (and which button fits when the other one does).
fn typed_rule(tr: &Tr, kind: TypedKind, text: &str) -> Result<Exclusion, String> {
    let shown = || arg(text.trim().to_owned());
    let rule = Exclusion::typed(kind, text).map_err(|e| {
        let key = match e {
            TypedRuleError::Empty => return tr.t("rule-empty"),
            TypedRuleError::TypeNotPattern => "rule-type-not-pattern",
            TypedRuleError::PatternNotType => "rule-pattern-not-type",
            TypedRuleError::NameAlone => "rule-name-alone",
            TypedRuleError::BadType => "rule-bad-type",
        };
        tr.t_args(key, &[("text", shown())])
    })?;
    if let Exclusion::Pattern { glob } = &rule
        && let Some(detail) = archeion_engine::pattern_problem(glob)
    {
        return Err(tr.t_args("rule-bad-pattern", &[("text", shown()), ("detail", arg(detail))]));
    }
    Ok(rule)
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

/// A collapsible section with a bold title. `open` is its state the first
/// time it is shown; `force_open` keeps it open while it shows something
/// that must be seen (a problem to fix).
fn section(ui: &mut Ui, id: &str, title: &str, open: bool, force_open: bool, add: impl FnOnce(&mut Ui)) {
    let mut header =
        egui::CollapsingHeader::new(widgets::section_text(title)).id_salt(("section", id)).default_open(open);
    if force_open {
        header = header.open(Some(true));
    }
    header.show(ui, add);
}

fn folders(ui: &mut Ui, tr: &Tr, p: &mut Profile, out: &mut Outcome) {
    if p.sources.is_empty() {
        ui.weak(tr.t("no-folders"));
    }
    let mut remove = None;
    for (i, path) in p.sources.iter().enumerate() {
        ui.horizontal(|ui| {
            if widgets::icon_button(ui, symbol::REMOVE, tr.t("remove-folder")).clicked() {
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

/// An imported profile with folders on other computers: which computers,
/// and the button that lets Archeion contact them. Until then nothing
/// touches them (Windows would sign in to them with the user's account).
fn network_confirmation(ui: &mut Ui, tr: &Tr, p: &mut Profile, out: &mut Outcome) {
    if !p.network_unconfirmed {
        return;
    }
    let hosts = p.network_hosts().join(", ");
    egui::Frame::group(ui.style()).show(ui, |ui| {
        let text = tr.t_args("network-unconfirmed", &[("hosts", arg(hosts))]);
        ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, text));
        if ui.button(tr.t("network-confirm")).clicked() {
            p.network_unconfirmed = false;
            out.changed = true;
        }
    });
}

fn destination(ui: &mut Ui, tr: &Tr, d: &mut Destination, info: &DestinationInfo, out: &mut Outcome) {
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
        // The free space beside the folder, wrapping below it in longer
        // languages.
        Destination::Local { path } => {
            ui.horizontal_wrapped(|ui| {
                let mut text = path.display().to_string();
                let field = egui::TextEdit::singleline(&mut text)
                    .hint_text(r"E:\Backup\Documenti")
                    .desired_width(DEST_FIELD_WIDTH);
                if ui.add(field).accessible_name(tr.t("destination")).changed() {
                    *path = text.into();
                    out.changed = true;
                }
                if ui.button(tr.t("browse")).clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    *path = dir;
                    out.changed = true;
                }
                free_space(ui, tr, info.free);
            });
        }
        Destination::Network { unc } => {
            ui.horizontal_wrapped(|ui| {
                let field = egui::TextEdit::singleline(unc)
                    .hint_text(r"\\server\backup\pc")
                    .desired_width(DEST_FIELD_WIDTH);
                out.changed |= ui.add(field).accessible_name(tr.t("destination")).changed();
                free_space(ui, tr, info.free);
            });
        }
    }
    match &info.foreign {
        Some(Foreign::Profile(name)) => {
            let text = tr.t_args("dest-foreign-profile", &[("name", arg(name.clone()))]);
            ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, text));
        }
        Some(Foreign::Deleted) => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                symbol::with(symbol::WARNING, tr.t("dest-foreign-deleted")),
            );
            if ui.button(tr.t("dest-adopt")).on_hover_text(tr.t("dest-adopt-hint")).clicked() {
                out.command = Some(Command::AdoptDestination);
            }
        }
        None => {}
    }
    file_system_note(ui, tr, info.file_system.as_deref());
    ui.weak(tr.t("dest-hint"));
}

/// Width of the destination field: most folder paths without cutting.
const DEST_FIELD_WIDTH: f32 = 340.0;

/// Free space of the destination's drive, as a warning when low.
fn free_space(ui: &mut Ui, tr: &Tr, free: Option<u64>) {
    let Some(free) = free else { return };
    let text = tr.t_args("dest-free", &[("free", arg(tr.bytes(free)))]);
    if free < LOW_SPACE_BYTES {
        ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, text));
    } else {
        ui.weak(text);
    }
}

/// What a drive without a journal (exFAT, FAT) or with FAT32's 4 GiB limit
/// means for the backups, as a warning; nothing for NTFS and ReFS.
fn file_system_note(ui: &mut Ui, tr: &Tr, file_system: Option<&str>) {
    if let Some(key) = file_system_warning(file_system) {
        ui.colored_label(ui.visuals().warn_fg_color, symbol::with(symbol::WARNING, tr.t(key)));
    }
}

/// The text of [`file_system_note`].
fn file_system_warning(file_system: Option<&str>) -> Option<&'static str> {
    match file_system? {
        fs if fs.starts_with("FAT") => Some("dest-fs-fat"),
        "exFAT" => Some("dest-fs-exfat"),
        _ => None,
    }
}

/// Below this, the free space of the destination is shown as a warning.
const LOW_SPACE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Every {
    Manual,
    Days,
    Hours,
    Monthly,
}

/// Editor of a clock schedule: on chosen days at chosen times, every few
/// hours, or monthly. A schedule saved in an older form is shown as the
/// equivalent "days" one and written in that form only when changed.
fn schedule(ui: &mut Ui, tr: &Tr, id: &str, s: &mut Schedule, out: &mut Outcome) {
    let current = match &*s {
        Schedule::Daily { .. } | Schedule::Weekly { .. } => s.normalized(),
        other => other.clone(),
    };
    let every = match current {
        Schedule::Manual => Every::Manual,
        Schedule::Hours { .. } => Every::Hours,
        Schedule::Monthly { .. } => Every::Monthly,
        Schedule::Days { .. } | Schedule::Daily { .. } | Schedule::Weekly { .. } => Every::Days,
    };
    let mut chosen = every;
    let label = |e: Every| match e {
        Every::Manual => tr.t("schedule-manual"),
        Every::Days => tr.t("schedule-days"),
        Every::Hours => tr.t("schedule-hours"),
        Every::Monthly => tr.t("schedule-monthly"),
    };
    egui::ComboBox::from_id_salt(("every", id))
        .selected_text(label(every))
        .show_ui(ui, |ui| {
            for e in [Every::Manual, Every::Days, Every::Hours, Every::Monthly] {
                ui.selectable_value(&mut chosen, e, label(e));
            }
        })
        .response
        .accessible_name(tr.t("schedule"));
    let mut new = if chosen == every { current.clone() } else { first_of(chosen, &current) };
    match &mut new {
        Schedule::Manual | Schedule::Daily { .. } | Schedule::Weekly { .. } => {}
        Schedule::Days { days, times } => {
            weekdays(ui, tr, days);
            day_times(ui, tr, id, times);
        }
        Schedule::Hours { hours, from, until } => {
            ui.horizontal(|ui| {
                ui.label(tr.t("schedule-every"));
                ui.add(egui::DragValue::new(hours).range(archeion_core::schedule::HOURS_RANGE));
                ui.label(tr.t("hours-unit"));
                ui.label(tr.t("schedule-from"));
                time_edit(ui, from);
                ui.label(tr.t("schedule-until"));
                time_edit(ui, until);
            });
            ui.weak(tr.t("schedule-hours-hint"));
        }
        Schedule::Monthly { day, at } => {
            ui.horizontal(|ui| {
                ui.label(tr.t("schedule-month-day"));
                ui.add(egui::DragValue::new(day).range(archeion_core::schedule::MONTH_DAY_RANGE));
                ui.label(tr.t("at"));
                time_edit(ui, at);
            });
            ui.weak(tr.t("schedule-month-hint"));
        }
    }
    if new != current {
        *s = new;
        out.changed = true;
        out.schedule_changed = true;
    }
}

/// The schedule shown when the user switches to `every`, keeping the time
/// of `previous` where there is one.
fn first_of(every: Every, previous: &Schedule) -> Schedule {
    let at = match previous {
        Schedule::Days { times, .. } => times.first().copied().unwrap_or_default(),
        Schedule::Hours { from, .. } => *from,
        Schedule::Monthly { at, .. } | Schedule::Daily { at } | Schedule::Weekly { at, .. } => *at,
        Schedule::Manual => TimeOfDay::default(),
    };
    match every {
        Every::Manual => Schedule::Manual,
        Every::Days => Schedule::Days { days: Weekday::ALL.to_vec(), times: vec![at] },
        Every::Hours => Schedule::Hours {
            hours: 2,
            from: TimeOfDay::new(8, 0).unwrap_or_default(),
            until: TimeOfDay::new(20, 0).unwrap_or_default(),
        },
        Every::Monthly => Schedule::Monthly { day: 1, at },
    }
}

/// Seven toggles, Monday first, plus "every day".
/// The last chosen day cannot be switched off.
fn weekdays(ui: &mut Ui, tr: &Tr, days: &mut Vec<Weekday>) {
    ui.horizontal_wrapped(|ui| {
        for d in Weekday::ALL {
            let mut on = days.contains(&d);
            let short: String = day_name(tr, d).chars().take(3).collect();
            let response = ui.toggle_value(&mut on, short).on_hover_text(day_name(tr, d));
            if response.changed() {
                if on {
                    days.push(d);
                    days.sort_unstable();
                } else if days.len() > 1 {
                    days.retain(|x| *x != d);
                }
            }
        }
        ui.add_space(theme::SPACE_S);
        if ui.small_button(tr.t("schedule-all-days")).clicked() {
            *days = Weekday::ALL.to_vec();
        }
    });
}

/// The times of a "days" schedule: each editable and removable (but not
/// the last one), and a button to add one more.
fn day_times(ui: &mut Ui, tr: &Tr, id: &str, times: &mut Vec<TimeOfDay>) {
    let mut remove = None;
    let can_remove = times.len() > 1;
    for (i, t) in times.iter_mut().enumerate() {
        ui.push_id((id, "time", i), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr.t("at"));
                time_edit(ui, t);
                if can_remove
                    && widgets::icon_button(ui, symbol::REMOVE, tr.t("schedule-remove-time")).clicked()
                {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        times.remove(i);
    }
    if times.len() < archeion_core::schedule::MAX_TIMES_A_DAY
        && ui.small_button(tr.t("schedule-add-time")).clicked()
    {
        // One hour after the last time, as a starting point.
        let last = times.last().copied().unwrap_or_default();
        let next = TimeOfDay::new((last.hour() + 1) % 24, last.minute()).unwrap_or(last);
        times.push(next);
    }
}

/// Hour and minute of `t`.
fn time_edit(ui: &mut Ui, t: &mut TimeOfDay) {
    let (mut hour, mut minute) = (t.hour(), t.minute());
    ui.add(egui::DragValue::new(&mut hour).range(0..=23));
    ui.label(":");
    ui.add(egui::DragValue::new(&mut minute).range(0..=59).custom_formatter(|v, _| format!("{v:02}")));
    if let Some(new) = TimeOfDay::new(hour, minute) {
        *t = new;
    }
}

/// Windows notifications about automatic backups.
fn alerts(ui: &mut Ui, tr: &Tr, p: &mut Profile, out: &mut Outcome) {
    let before = p.alerts;
    let a: &mut Alerts = &mut p.alerts;
    widgets::checkbox(ui, &mut a.on_failure, tr.t("alert-on-failure"));
    ui.horizontal(|ui| {
        ui.label(tr.t("alert-stale"));
        ui.add(egui::DragValue::new(&mut a.stale_days).range(0..=365));
        ui.label(tr.t("full-days-unit"));
    });
    ui.weak(tr.t(if p.backs_up_automatically() { "alert-hint" } else { "alert-manual-hint" }));
    if p.alerts != before {
        out.changed = true;
        // The daily check task exists only while a profile needs it.
        out.schedule_changed = true;
    }
}

fn retention(ui: &mut Ui, tr: &Tr, r: &mut Retention, out: &mut Outcome) {
    ui.add_space(theme::SPACE_S);
    egui::Grid::new("retention").num_columns(2).show(ui, |ui| {
        for (key, value) in [
            ("keep-within", &mut r.keep_within_days),
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

/// What the automatic merge would do with the rules shown, for the backups
/// there are; when the rules would merge backups that the rules confirmed
/// before kept, a warning that the merge waits, with the user's two ways
/// out. Nothing while the backups are not read yet.
fn merge_preview(
    ui: &mut Ui,
    tr: &Tr,
    p: &mut Profile,
    archives: Option<&[ArchiveName]>,
    now: jiff::civil::DateTime,
    out: &mut Outcome,
) {
    let Some(archives) = archives.filter(|a| !a.is_empty()) else { return };
    let preview = merge_plan::preview(archives, &p.merge, now);
    if preview.waits() {
        let count = arg(preview.held_back.len());
        ui.colored_label(
            ui.visuals().warn_fg_color,
            symbol::with(symbol::WARNING, tr.t_args("merge-waits", &[("count", count)])),
        );
        ui.horizontal(|ui| {
            if ui.button(tr.t("merge-confirm")).on_hover_text(tr.t("merge-confirm-hint")).clicked() {
                p.merge.confirm();
                out.changed = true;
            }
            if ui.button(tr.t("merge-restore-rules")).clicked() {
                p.merge.restore_approved();
                out.changed = true;
            }
        });
    } else if let (Some(first), Some(last)) = (preview.merged.first(), preview.merged.last()) {
        let args =
            [("count", arg(preview.merged.len())), ("from", arg(first.label())), ("to", arg(last.label()))];
        ui.weak(tr.t_args("merge-preview", &args));
    } else {
        ui.weak(tr.t("merge-preview-none"));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::Language;

    /// A change form filled in completely: current password, new one twice,
    /// warning acknowledged.
    fn change_drafts() -> Drafts {
        Drafts {
            current_password: "old secret".into(),
            password: "a much longer new passphrase".into(),
            password_again: "a much longer new passphrase".into(),
            password_ack: true,
            ..Drafts::default()
        }
    }

    #[test]
    fn change_needs_the_current_password() {
        let tr = Tr::new(Language::English);
        let d = Drafts { current_password: String::new(), ..change_drafts() };
        assert_eq!(password_problem(&tr, &d, PasswordForm::Change, ""), Some(tr.t("password-current-hint")));
        assert_eq!(password_problem(&tr, &change_drafts(), PasswordForm::Change, ""), None);
    }

    #[test]
    fn change_still_checks_the_new_password() {
        let tr = Tr::new(Language::English);
        let d = Drafts { password_again: "something else".into(), ..change_drafts() };
        assert_eq!(password_problem(&tr, &d, PasswordForm::Change, ""), Some(tr.t("password-mismatch-hint")));
    }

    #[test]
    fn other_forms_ignore_the_current_password() {
        let tr = Tr::new(Language::English);
        let d = Drafts { current_password: String::new(), ..change_drafts() };
        assert_eq!(password_problem(&tr, &d, PasswordForm::Choose, ""), None);
        let typed_once = Drafts { password: "short".into(), ..Drafts::default() };
        assert_eq!(password_problem(&tr, &typed_once, PasswordForm::Reenter, ""), None);
    }

    #[test]
    fn a_new_password_reports_the_first_unmet_requirement() {
        let tr = Tr::new(Language::English);
        let typed = |pw: &str| Drafts { password: pw.into(), password_again: pw.into(), ..change_drafts() };
        let short = tr.t_args(
            "password-short-hint",
            &[("count", arg(policy::MIN_CHARS)), ("max", arg(policy::MAX_CHARS))],
        );
        assert_eq!(password_problem(&tr, &typed("Kx7#mQ2!vR9zLp"), PasswordForm::Choose, ""), Some(short));
        let common = Some(tr.t("password-common-hint"));
        assert_eq!(password_problem(&tr, &typed("Password2024!2025"), PasswordForm::Choose, ""), common);
        let name = Some(tr.t("password-name-hint"));
        assert_eq!(password_problem(&tr, &typed("My Photos 2024!"), PasswordForm::Change, "Photos"), name);
        assert_eq!(password_problem(&tr, &typed("Kx7#mQ2!vR9zLp5"), PasswordForm::Choose, "Photos"), None);
    }

    #[test]
    fn existing_backups_take_their_password_as_it_is() {
        let tr = Tr::new(Language::English);
        let d = Drafts { password: "password".into(), ..Drafts::default() };
        assert_eq!(password_problem(&tr, &d, PasswordForm::Reenter, "Photos"), None);
    }

    #[test]
    fn change_command_carries_and_forgets_the_current_password() {
        let mut d = change_drafts();
        let cmd = password_command(&mut d, PasswordForm::Change);
        assert!(matches!(
            &cmd,
            Some(Command::ChangePassword { current, new })
                if current.as_str() == "old secret" && new.as_str() == "a much longer new passphrase"
        ));
        assert!(d.current_password.is_empty());
    }

    #[test]
    fn change_command_needs_both_passwords() {
        let mut d = Drafts { current_password: String::new(), ..change_drafts() };
        assert!(password_command(&mut d, PasswordForm::Change).is_none());
        let mut d = Drafts { password: String::new(), ..change_drafts() };
        assert!(password_command(&mut d, PasswordForm::Change).is_none());
    }

    #[test]
    fn save_command_uses_the_typed_password() {
        let mut d = Drafts { password: "typed".into(), ..Drafts::default() };
        let cmd = password_command(&mut d, PasswordForm::Reenter);
        assert!(matches!(&cmd, Some(Command::SavePassword(pw)) if pw.as_str() == "typed"));
    }

    #[test]
    fn clearing_forgets_every_typed_password() {
        let mut d = Drafts { recovery_open: true, recovery_password: "secret".into(), ..change_drafts() };
        d.clear_password();
        assert!(d.password.is_empty() && d.password_again.is_empty() && d.current_password.is_empty());
        assert!(d.recovery_password.is_empty() && !d.recovery_open && !d.password_ack);
    }

    #[test]
    fn drives_without_a_journal_or_with_the_fat32_limit_are_pointed_out() {
        assert_eq!(file_system_warning(Some("FAT32")), Some("dest-fs-fat"));
        assert_eq!(file_system_warning(Some("FAT")), Some("dest-fs-fat"));
        assert_eq!(file_system_warning(Some("exFAT")), Some("dest-fs-exfat"));
        for quiet in [Some("NTFS"), Some("ReFS"), None] {
            assert_eq!(file_system_warning(quiet), None);
        }
    }
}
