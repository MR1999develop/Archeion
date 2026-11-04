//! "Import profile from backups…": the profile of a folder of backups, read
//! from the copy each backup keeps there
//! ([`archeion_engine::read_profile_copy`]), for a new computer or one whose
//! settings were lost. The folder is read on a thread (a share may be
//! slow); an encrypted one asks for its password first, in a dialog, and
//! the password that opened it comes back with the profile, to be saved
//! with it as for any encrypted profile.

use super::{symbol, theme, widgets};
use crate::i18n::{Tr, arg};
use archeion_core::Profile;
use archeion_engine::{EngineError, Password};
use eframe::egui;
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
use zeroize::Zeroize as _;

const DIALOG_ID: &str = "import-from-backup";

/// How an import ended.
#[derive(Debug)]
pub enum Done {
    /// The profile read, and the password that opened its backups.
    Imported { profile: Box<Profile>, password: Option<Password> },
    /// Why nothing was imported, for the error bar.
    Failed(String),
}

/// A copy being read: the password it was given, and where the result
/// arrives.
type Reading = (Option<Password>, mpsc::Receiver<Result<Profile, EngineError>>);

#[derive(Debug)]
struct Import {
    folder: PathBuf,
    /// The folder is encrypted: the dialog asks for its password.
    asking: bool,
    /// The password being typed.
    typed: String,
    /// Why the last password did not open the backups.
    error: Option<String>,
    /// The password field takes the keyboard at the next frame.
    focus: bool,
    reading: Option<Reading>,
}

impl Drop for Import {
    fn drop(&mut self) {
        self.typed.zeroize();
    }
}

impl Import {
    /// Reads the copy in the background, with `password` for an encrypted
    /// folder. A thread that cannot start shows as a failed read.
    fn read(&mut self, ctx: &egui::Context, password: Option<Password>) {
        let (tx, rx) = mpsc::channel();
        let (folder, given, ctx) = (self.folder.clone(), password.clone(), ctx.clone());
        let spawned = std::thread::Builder::new().name("import-from-backup".into()).spawn(move || {
            let _ = tx.send(archeion_engine::read_profile_copy(&folder, given.as_ref()));
            ctx.request_repaint();
        });
        if let Err(e) = spawned {
            tracing::error!(error = %e, "starting the import from backups");
        }
        self.reading = Some((password, rx));
    }
}

/// The import in progress, if any: at most one at a time.
#[derive(Debug, Default)]
pub struct BackupImport {
    current: Option<Import>,
}

impl BackupImport {
    /// Starts reading the profile of the backups in `folder`, chosen by the
    /// user; does nothing while another import is under way.
    pub fn start(&mut self, ctx: &egui::Context, folder: PathBuf) {
        if self.current.is_some() {
            return;
        }
        let mut import =
            Import { folder, asking: false, typed: String::new(), error: None, focus: false, reading: None };
        import.read(ctx, None);
        self.current = Some(import);
    }

    /// True while an import is under way (reading, or asking a password).
    pub fn busy(&self) -> bool {
        self.current.is_some()
    }

    /// How the import ended, once it has: an encrypted folder opens the
    /// password dialog instead, and a wrong password shows there.
    pub fn poll(&mut self, tr: &Tr) -> Option<Done> {
        let import = self.current.as_mut()?;
        let result = match &import.reading {
            None => return None,
            Some((_, rx)) => match rx.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return None,
                Err(mpsc::TryRecvError::Disconnected) => Err(EngineError::Io {
                    context: "reading the backups".into(),
                    source: std::io::Error::other("stopped unexpectedly"),
                }),
            },
        };
        let (password, _) = import.reading.take()?;
        match result {
            Ok(profile) => {
                self.current = None;
                Some(Done::Imported { profile: Box::new(profile), password })
            }
            Err(EngineError::PasswordNeeded) if password.is_none() => {
                import.asking = true;
                import.focus = true;
                None
            }
            Err(EngineError::WrongPassword) => {
                import.asking = true;
                import.error = Some(tr.t("password-wrong"));
                import.focus = true;
                None
            }
            Err(e) => {
                tracing::warn!(error = %e, folder = %import.folder.display(), "importing a profile from backups");
                let text = failure(tr, &import.folder, &e);
                self.current = None;
                Some(Done::Failed(text))
            }
        }
    }

    /// The password dialog of an encrypted folder. Import (or Enter) reads
    /// the copy with the password typed; Cancel, Esc and a click outside
    /// end the import.
    pub fn show(&mut self, ctx: &egui::Context, tr: &Tr) {
        let Some(import) = self.current.as_mut() else { return };
        if !import.asking {
            return;
        }
        let reading = import.reading.is_some();
        let (mut submit, mut cancel) = (false, false);
        let response = widgets::modal(ctx, egui::Id::unique(DIALOG_ID)).show(ctx, |ui| {
            ui.set_max_width(theme::DIALOG_WIDTH);
            ui.heading(tr.t("import-backup-password-title"));
            ui.add_space(theme::SPACE_S);
            // The folder on a line of its own: a long path inside the
            // sentence breaks it apart.
            ui.label(symbol::with(symbol::FOLDER, import.folder.display().to_string()));
            ui.add_space(theme::SPACE_XS);
            ui.label(tr.t("import-backup-password-body"));
            ui.add_space(theme::SPACE_S);
            ui.horizontal(|ui| {
                let label = ui.label(tr.t("password"));
                let field = egui::TextEdit::singleline(&mut import.typed)
                    .password(true)
                    .desired_width(theme::PASSWORD_FIELD_WIDTH);
                let field = ui.add_enabled(!reading, field).labelled_by(label.id);
                if std::mem::take(&mut import.focus) {
                    field.request_focus();
                }
                submit |= field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
            if let Some(error) = &import.error {
                ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, error));
            }
            ui.add_space(theme::SPACE_S);
            // Inside a row, so the right-aligned layout takes one line, not
            // the whole height left. Windows order: action, then Cancel.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    cancel = ui.button(tr.t("cancel")).clicked();
                    let button = widgets::primary_button(ui, tr.t("import-backup-import"));
                    submit |= ui.add_enabled(!reading && !import.typed.is_empty(), button).clicked();
                    if reading {
                        ui.spinner();
                    }
                });
            });
        });
        if cancel || response.should_close() {
            self.current = None;
        } else if submit
            && !reading
            && let Some(password) = Password::new(std::mem::take(&mut import.typed))
        {
            import.error = None;
            import.read(ctx, Some(password));
        }
    }
}

/// Why the backups in `folder` gave no profile, for the user.
fn failure(tr: &Tr, folder: &Path, e: &EngineError) -> String {
    let folder = || arg(folder.display().to_string());
    match e {
        EngineError::NotADestination => tr.t_args("import-backup-none", &[("folder", folder())]),
        EngineError::NoProfileCopy => tr.t_args("import-backup-no-copy", &[("folder", folder())]),
        EngineError::Damaged { message, .. } => {
            tr.t_args("import-backup-damaged", &[("folder", folder()), ("detail", arg(message.clone()))])
        }
        other => {
            tr.t_args("import-backup-failed", &[("folder", folder()), ("detail", arg(other.to_string()))])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{BackupMode, Container, Destination, Language, RunEvent};
    use archeion_engine::EventSink;
    use eframe::egui::accesskit::Role;
    use egui_kittest::{Harness, kittest::Queryable as _};
    use std::time::{Duration, Instant};

    struct Quiet;

    impl EventSink for Quiet {
        fn emit(&self, _: RunEvent) {}
    }

    const PASSWORD: &str = "a long enough test password";

    /// A folder with one backup of a profile, encrypted or not.
    fn backed_up(encrypted: bool) -> (tempfile::TempDir, Profile) {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), "alpha").unwrap();
        let mut profile = Profile::new("Docs");
        profile.sources.push(source);
        profile.destination = Destination::Local { path: dir.path().join("backups") };
        profile.container = Container::ZipPerFile;
        profile.encrypted = encrypted;
        let password = encrypted.then(|| Password::new(PASSWORD.into()).unwrap());
        archeion_engine::backup(&profile, password.as_ref(), BackupMode::Auto, &Quiet).unwrap();
        (dir, profile)
    }

    #[derive(Default)]
    struct State {
        import: BackupImport,
        start: Option<PathBuf>,
        done: Option<Done>,
    }

    fn fonts(ctx: &egui::Context) {
        ctx.add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
        theme::apply(ctx);
    }

    /// The import of `folder` as the window drives it: started, polled and
    /// drawn at every frame.
    fn harness(folder: PathBuf) -> Harness<'static, State> {
        let tr = Tr::new(Language::English);
        let builder = Harness::builder().with_size(egui::vec2(800.0, 600.0)).with_context_setup(fonts);
        builder.build_ui_state(
            move |ui, s: &mut State| {
                let ctx = ui.ctx().clone();
                if let Some(folder) = s.start.take() {
                    s.import.start(&ctx, folder);
                }
                if let Some(done) = s.import.poll(&tr) {
                    s.done = Some(done);
                }
                s.import.show(&ctx, &tr);
            },
            State { start: Some(folder), ..State::default() },
        )
    }

    /// Steps until `until` holds: the folder is read on a thread.
    fn wait(h: &mut Harness<'_, State>, what: &str, until: impl Fn(&Harness<'_, State>) -> bool) {
        let started = Instant::now();
        while !until(h) {
            assert!(started.elapsed() < Duration::from_secs(20), "{what}");
            std::thread::sleep(Duration::from_millis(20));
            h.step();
        }
    }

    fn dialog_open(h: &Harness<'_, State>) -> bool {
        h.query_all_by(|n| n.role() == Role::PasswordInput).next().is_some()
    }

    fn button<'h>(h: &'h Harness<'_, State>, label: &'h str) -> egui_kittest::Node<'h> {
        h.query_all_by(move |n| n.role() == Role::Button && n.label().as_deref() == Some(label))
            .next()
            .unwrap_or_else(|| panic!("no button {label}"))
    }

    fn type_password(h: &mut Harness<'_, State>, text: &str) {
        h.query_all_by(|n| n.role() == Role::PasswordInput).next().expect("the field").focus();
        h.run_steps(2);
        let field = h.query_all_by(|n| n.role() == Role::PasswordInput).next().unwrap();
        assert!(field.is_focused(), "the field takes the keyboard");
        field.type_text(text);
        h.run_steps(2);
        button(h, "Import").click();
        h.run_steps(2);
    }

    #[test]
    fn plain_backups_give_their_profile_at_once() {
        let (_dir, profile) = backed_up(false);
        let mut h = harness(profile.destination.path().to_path_buf());
        wait(&mut h, "the profile was not read", |h| h.state().done.is_some());
        assert!(!dialog_open(&h), "no password asked");
        match h.state_mut().done.take() {
            Some(Done::Imported { profile: read, password: None }) => assert_eq!(*read, profile),
            other => panic!("{other:?}"),
        }
        assert!(!h.state().import.busy());
    }

    #[test]
    fn encrypted_backups_ask_their_password_first() {
        let (_dir, profile) = backed_up(true);
        let mut h = harness(profile.destination.path().to_path_buf());
        wait(&mut h, "the password was not asked", dialog_open);
        assert!(h.query_by_label_contains("are encrypted").is_some(), "the dialog says why");

        type_password(&mut h, "not the password at all");
        wait(&mut h, "the wrong password was not told", |h| {
            h.query_by_label_contains("does not open the backups").is_some()
        });
        assert!(h.state().done.is_none(), "nothing imported");

        type_password(&mut h, PASSWORD);
        wait(&mut h, "the profile was not read", |h| h.state().done.is_some());
        match h.state_mut().done.take() {
            Some(Done::Imported { profile: read, password: Some(pw) }) => {
                assert_eq!(*read, profile);
                assert_eq!(pw.as_str(), PASSWORD, "the password that opened them, to be saved");
            }
            other => panic!("{other:?}"),
        }
        h.run_steps(2);
        assert!(!dialog_open(&h), "the dialog closed");
    }

    #[test]
    fn cancel_ends_the_import() {
        let (_dir, profile) = backed_up(true);
        let mut h = harness(profile.destination.path().to_path_buf());
        wait(&mut h, "the password was not asked", dialog_open);
        button(&h, "Cancel").click();
        h.run_steps(2);
        assert!(!dialog_open(&h));
        assert!(h.state().done.is_none() && !h.state().import.busy());
    }

    #[test]
    fn a_folder_without_backups_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let mut h = harness(dir.path().to_path_buf());
        wait(&mut h, "the folder was not read", |h| h.state().done.is_some());
        match h.state_mut().done.take() {
            Some(Done::Failed(text)) => assert!(text.contains("holds no Archeion backups"), "{text}"),
            other => panic!("{other:?}"),
        }
    }
}
