//! Archeion executable.
//!
//! Without arguments it opens the window. `archeion run <profile id>` backs
//! up one profile without a window, `archeion merge <profile id>` merges its
//! old archives, `archeion delete <profile id> --archive <name>` deletes one
//! backup, `archeion restore <profile id> --request <file>` restores; all
//! write progress as JSON lines on stdout. Task Scheduler starts the
//! first two that way, and so does the window for its buttons, so every
//! path runs the same code.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod credentials;
mod history;
mod i18n;
mod job;
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
    },
    /// Merge the archives of one profile: by its rules, or the chosen range
    /// `--from <backup> --to <backup>` (everything between is included).
    Merge {
        profile: ProfileId,
        #[arg(long)]
        scheduled: bool,
        #[arg(long, requires = "to")]
        from: Option<String>,
        #[arg(long, requires = "from")]
        to: Option<String>,
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
}

/// `--kind` values.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Kind {
    Incremental,
    Complete,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let storage = match storage::Storage::locate() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("archeion: {e:#}");
            return ExitCode::FAILURE;
        }
    };
    init_logging(&storage);
    match cli.command {
        Some(Command::Run { profile, scheduled, kind }) => {
            let mode = match kind {
                None => BackupMode::Auto,
                Some(Kind::Incremental) => BackupMode::Incremental,
                Some(Kind::Complete) => BackupMode::Complete,
            };
            worker::run(&storage, profile, worker::Work::Backup { mode }, scheduled, cli.log)
        }
        Some(Command::Verify { profile, scheduled, sample }) => {
            worker::run(&storage, profile, worker::Work::Verify { sample }, scheduled, cli.log)
        }
        Some(Command::Delete { profile, archive }) => {
            worker::run(&storage, profile, worker::Work::Delete { archive }, false, cli.log)
        }
        Some(Command::Merge { profile, scheduled, from, to }) => {
            let work = worker::Work::Merge { range: from.zip(to) };
            worker::run(&storage, profile, work, scheduled, cli.log)
        }
        Some(Command::Restore { profile, request }) => {
            worker::run(&storage, profile, worker::Work::Restore { request }, false, cli.log)
        }
        None => match ui::run(storage) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                tracing::error!(error = %e, "window failed");
                ExitCode::FAILURE
            }
        },
    }
}

/// Logs go to a file in the data folder (the window has no console).
/// `RUST_LOG` overrides the default `info` level.
fn init_logging(storage: &storage::Storage) {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false);
    match storage.open_log() {
        Ok(file) => builder.with_writer(std::sync::Mutex::new(file)).init(),
        Err(_) => builder.with_writer(std::io::stderr).init(),
    }
}
