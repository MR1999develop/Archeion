//! Archeion executable.
//!
//! Without arguments it opens the window. `archeion run <profile id>` backs
//! up one profile without a window, `archeion merge <profile id>` merges its
//! old archives, `archeion delete <profile id> --archive <name>` deletes one
//! backup, `archeion restore <profile id> --request <file>` restores; all
//! write progress as JSON lines on stdout. Task Scheduler starts the
//! first two that way (`run --event` for backups started by an event), and
//! so does the window for its buttons, so every path runs the same code.
//! `archeion watch` warns about profiles not backed up for too long.
//! `archeion extract --destination <folder> --to <folder>` restores from a
//! destination folder alone, without settings or a profile, on any computer.
//! `archeion --tray` opens the window hidden in the notification area (how
//! Windows starts it at sign-in); a second window is never opened: the
//! running one is brought forward instead.
//!
//! It never opens a console window, development builds included; the
//! commands write to stdout, which callers (the window, Task Scheduler,
//! the end-to-end tests) read through a pipe. Unit tests keep a console.
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod activity;
mod autostart;
mod credentials;
mod extract;
mod history;
mod i18n;
mod instance;
mod job;
mod notify;
mod profile_file;
mod restore_plan;
mod seen;
mod storage;
mod ui;
mod worker;

use archeion_core::{BackupMode, ProfileId};
use clap::{Parser, Subcommand, ValueEnum};
use std::process::ExitCode;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Run log to write (the window passes it so it can find the log);
    /// without it a new one is created in the data folder.
    #[arg(long, global = true, hide = true)]
    log: Option<std::path::PathBuf>,
    /// Stop at the next file once standard input is closed: the window
    /// asks a run it started to stop this way.
    #[arg(long, global = true, hide = true)]
    stop_on_eof: bool,
    /// Open the window hidden in the notification area.
    #[arg(long)]
    tray: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Back up one profile without a window; progress as JSON lines on stdout.
    Run {
        /// Profile id, as shown in Task Scheduler.
        profile: ProfileId,
        /// Started by Task Scheduler rather than by the window.
        #[arg(long)]
        scheduled: bool,
        /// Kind of backup; without it the profile's full-backup rule decides.
        #[arg(long, value_enum)]
        kind: Option<Kind>,
        /// Started by an event (logon, idle, drive connected): skipped
        /// quietly when the last backup is recent or the destination is
        /// not there.
        #[arg(long, requires = "scheduled")]
        event: bool,
    },
    /// Show a Windows notification for every automatic profile whose last
    /// backup is older than its limit (Task Scheduler runs it daily).
    Watch,
    /// Merge the archives of one profile: by its rules, or the chosen range
    /// `--from <backup> --to <backup>` (everything between is included).
    /// Files deleted at the source inside the merged backups are kept as
    /// the profile says; `--drop-deleted` drops them from this merge.
    Merge {
        profile: ProfileId,
        #[arg(long)]
        scheduled: bool,
        #[arg(long, requires = "to")]
        from: Option<String>,
        #[arg(long, requires = "from")]
        to: Option<String>,
        #[arg(long)]
        drop_deleted: bool,
    },
    /// Check the backups of one profile: every stored file, or with
    /// `--sample N` a restore test of N files of the newest backup.
    Verify {
        profile: ProfileId,
        #[arg(long)]
        scheduled: bool,
        #[arg(long)]
        sample: Option<usize>,
    },
    /// Delete one backup of one profile (an incremental one, or the initial
    /// complete one when it is the only backup).
    Delete {
        profile: ProfileId,
        /// Backup name, e.g. 2026-10-29_213000_I.
        #[arg(long)]
        archive: String,
    },
    /// Restore files of one profile as described by a JSON request file
    /// (written by the window; deleted once read).
    Restore {
        profile: ProfileId,
        #[arg(long)]
        request: std::path::PathBuf,
    },
    /// Restore from a destination folder alone, without settings or a
    /// profile: for recovering backups on any computer.
    Extract(extract::ExtractArgs),
}

/// `--kind` values.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Kind {
    Incremental,
    Complete,
}

fn main() -> ExitCode {
    // Before anything loads a library by name (the graphics drivers).
    archeion_win::restrict_library_search();
    let cli = Cli::parse();
    if cli.stop_on_eof {
        worker::stop_when_stdin_closes();
    }
    // Needs no settings and writes none: it works on any computer.
    let command = match cli.command {
        Some(Command::Extract(args)) => return extract::run(args),
        other => other,
    };
    let storage = match storage::Storage::locate() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("archeion: {e:#}");
            return ExitCode::FAILURE;
        }
    };
    init_logging(&storage);
    match command {
        Some(Command::Run { profile, scheduled, kind, event }) => {
            let mode = match kind {
                None => BackupMode::Auto,
                Some(Kind::Incremental) => BackupMode::Incremental,
                Some(Kind::Complete) => BackupMode::Complete,
            };
            if event && !worker::event_backup_wanted(&storage, profile) {
                return ExitCode::SUCCESS;
            }
            worker::run(&storage, profile, worker::Work::Backup { mode }, scheduled, cli.log)
        }
        Some(Command::Watch) => worker::watch(&storage),
        Some(Command::Verify { profile, scheduled, sample }) => {
            worker::run(&storage, profile, worker::Work::Verify { sample }, scheduled, cli.log)
        }
        Some(Command::Delete { profile, archive }) => {
            worker::run(&storage, profile, worker::Work::Delete { archive }, false, cli.log)
        }
        Some(Command::Merge { profile, scheduled, from, to, drop_deleted }) => {
            let work = worker::Work::Merge { range: from.zip(to), drop_deleted };
            worker::run(&storage, profile, work, scheduled, cli.log)
        }
        Some(Command::Restore { profile, request }) => {
            worker::run(&storage, profile, worker::Work::Restore { request }, false, cli.log)
        }
        Some(Command::Extract(args)) => extract::run(args),
        None => {
            let outside = std::env::var_os(storage::DATA_DIR_VAR).is_some_and(|d| !d.is_empty());
            let claimed = instance::user_dir(storage.dir(), outside).and_then(|dir| instance::claim(&dir));
            let primary = match claimed {
                Ok(instance::Claim::Primary(p)) => Some(p),
                Ok(instance::Claim::Secondary { other }) => {
                    tracing::info!(?other, "Archeion is already open: brought it forward");
                    if let Some(other) = other {
                        tell_another_copy_is_open(&storage, &other);
                    }
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "single-window check failed: opening anyway");
                    None
                }
            };
            match ui::run(storage, cli.tray, primary) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    tracing::error!(error = %e, "window failed");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// Logs go to a file in the data folder (the window has no console).
/// `RUST_LOG` overrides the default `info` level.
/// The window open belongs to another copy of Archeion (another folder or
/// version): it was brought forward, and this copy says why it does not
/// open, in the user's language, with the copy to close.
fn tell_another_copy_is_open(storage: &storage::Storage, other: &std::path::Path) {
    let language = storage.load().map(|s| s.language).unwrap_or_default();
    let tr = i18n::Tr::new(language);
    let text = tr.t_args("already-open-other", &[("path", i18n::arg(other.display().to_string()))]);
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title(tr.t("already-open-title"))
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn init_logging(storage: &storage::Storage) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false);
    match storage.open_log() {
        Ok(file) => builder.with_writer(std::sync::Mutex::new(file)).init(),
        Err(_) => builder.with_writer(std::io::stderr).init(),
    }
}
