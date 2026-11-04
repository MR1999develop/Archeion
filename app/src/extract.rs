//! `archeion extract`: restore from a destination folder alone, without
//! settings, profile or saved password, so that the portable executable
//! can recover backups on any computer.
//!
//! Progress and the result are written as plain text on stdout and in
//! `archeion-extract.log` in the target folder. The password or recovery
//! key of an encrypted destination comes from the first line of standard
//! input (`--stdin`) or the `ARCHEION_RECOVERY_KEY` / `ARCHEION_PASSWORD`
//! environment variables, never from the command line, which other
//! programs can read.
//!
//! Exit codes as for the other commands: 0 everything restored, 1 failure
//! or some files skipped, 2 destination missing, 4 password missing or
//! wrong, 5 not enough free space on the target drive.

use crate::worker::{EXIT_DESTINATION_UNAVAILABLE, EXIT_FAILED, EXIT_NO_SPACE, EXIT_PASSWORD};
use archeion_core::{Phase, RunEvent, Summary};
use archeion_engine::{EngineError, EventSink, ExtractRequest, Password, Secret};
use std::{
    fs::File,
    io::{BufRead as _, Write as _},
    path::PathBuf,
    process::ExitCode,
    sync::Mutex,
};

/// Environment variable holding the recovery key.
const RECOVERY_KEY_VAR: &str = "ARCHEION_RECOVERY_KEY";
/// Environment variable holding the password (also read by the other
/// commands when no password is saved).
pub(crate) const PASSWORD_VAR: &str = "ARCHEION_PASSWORD";
/// Name of the log written in the target folder.
const LOG_NAME: &str = "archeion-extract.log";

/// Arguments of `archeion extract`.
#[derive(Debug, clap::Args)]
pub struct ExtractArgs {
    /// Destination folder holding the backups (the folder with archeion.json
    /// and the dated backup folders).
    #[arg(long)]
    destination: PathBuf,
    /// Backup to restore from, e.g. 2026-10-29_213000_I; the newest when
    /// left out.
    #[arg(long)]
    archive: Option<String>,
    /// File or folder to restore, as its original path
    /// (C:\Users\me\Documents); everything when left out.
    #[arg(long)]
    path: Option<String>,
    /// Folder that receives the files, below their full path
    /// (<folder>\C\Users\me\...).
    #[arg(long)]
    to: PathBuf,
    /// Replace files that already exist in the target folder.
    #[arg(long)]
    overwrite: bool,
    /// Read the password or the recovery key (64 hex digits, spaces
    /// allowed) of an encrypted destination from the first line of standard
    /// input. Without it, the ARCHEION_PASSWORD or ARCHEION_RECOVERY_KEY
    /// environment variable gives it.
    #[arg(long)]
    stdin: bool,
}

/// Runs `archeion extract`.
pub fn run(args: ExtractArgs) -> ExitCode {
    let out = Output::new(&args.to);
    let secret = match secret(&args) {
        Ok(s) => s,
        Err(message) => {
            out.line(&message);
            return ExitCode::from(EXIT_PASSWORD);
        }
    };
    out.line(&format!("Restoring from {} to {}", args.destination.display(), args.to.display()));
    let request = ExtractRequest {
        destination: args.destination,
        archive: args.archive,
        path: args.path,
        to: args.to,
        overwrite: args.overwrite,
        secret,
    };
    match archeion_engine::extract(&request, &out) {
        Ok(report) if report.skipped.is_empty() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(EXIT_FAILED),
        Err(e) => {
            out.line(&format!("Failed: {e}"));
            ExitCode::from(match e {
                EngineError::DestinationUnavailable(_) => EXIT_DESTINATION_UNAVAILABLE,
                EngineError::PasswordNeeded | EngineError::WrongPassword => EXIT_PASSWORD,
                EngineError::NotEnoughSpace { .. } => EXIT_NO_SPACE,
                _ => EXIT_FAILED,
            })
        }
    }
}

/// The secret from the arguments, standard input or the environment;
/// `None` when there is none (fine for a destination that is not
/// encrypted).
fn secret(args: &ExtractArgs) -> Result<Option<Secret>, String> {
    let non_empty = |s: String| (!s.trim().is_empty()).then_some(s);
    if args.stdin {
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).map_err(|e| format!("reading standard input: {e}"))?;
        let line = line.trim_end_matches(['\r', '\n']).to_owned();
        return non_empty(line)
            .map(|l| Some(Secret::Either(l)))
            .ok_or_else(|| "standard input is empty".into());
    }
    if let Some(k) = std::env::var(RECOVERY_KEY_VAR).ok().and_then(non_empty) {
        return Ok(Some(Secret::RecoveryKey(k)));
    }
    Ok(std::env::var(PASSWORD_VAR).ok().and_then(non_empty).and_then(Password::new).map(Secret::Password))
}

/// Plain-text progress on stdout and in the log in the target folder.
struct Output {
    log: Mutex<Option<File>>,
    /// Last tenth of the work reported, so progress prints a line per 10%.
    last_tenth: Mutex<u64>,
}

impl Output {
    fn new(to: &std::path::Path) -> Self {
        let log = std::fs::create_dir_all(to).and_then(|()| File::create(to.join(LOG_NAME))).ok();
        Self { log: Mutex::new(log), last_tenth: Mutex::new(0) }
    }

    /// Writes one line to stdout and to the log. Write errors are ignored:
    /// with no console the output goes nowhere, and the log is a help.
    fn line(&self, text: &str) {
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{text}");
        let _ = stdout.flush();
        if let Some(file) = self.log.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_mut() {
            let _ = writeln!(file, "{text}");
        }
    }
}

impl EventSink for Output {
    fn emit(&self, event: RunEvent) {
        match event {
            RunEvent::Phase { phase: Phase::Restoring } => self.line("Restoring files…"),
            RunEvent::Progress { done, total, files_done, files_total, .. } if total > 0 => {
                let tenth = done.saturating_mul(10) / total;
                let mut last = self.last_tenth.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if tenth > *last {
                    *last = tenth;
                    drop(last);
                    self.line(&format!("{}% ({files_done} of {files_total} files)", tenth.min(10) * 10));
                }
            }
            RunEvent::Skipped(s) => self.line(&format!("Skipped {}: {}", s.path, s.reason)),
            RunEvent::Finished { summary: Summary::Restore(r) } => self.line(&format!(
                "Done: {} files restored, {} already there and kept, {} skipped",
                r.files_restored,
                r.files_kept,
                r.skipped.len()
            )),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    #[derive(Debug, clap::Parser)]
    struct Test {
        #[command(flatten)]
        args: ExtractArgs,
    }

    #[test]
    fn arguments_parse_and_no_secret_comes_from_them() {
        let t = Test::try_parse_from(["x", "--destination", r"E:\B", "--to", "out", "--stdin"]).unwrap();
        assert!(t.args.stdin);
        for flag in ["--password", "--recovery-key"] {
            let parsed = Test::try_parse_from(["x", "--destination", "d", "--to", "o", flag, "secret"]);
            assert!(parsed.is_err(), "{flag} is not an option");
        }
        assert!(Test::try_parse_from(["x", "--to", "o"]).is_err(), "the destination is required");
    }
}
