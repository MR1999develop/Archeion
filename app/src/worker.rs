//! Headless run of one profile (`archeion run|merge|restore <id>`).
//!
//! Exit codes: 0 success, 1 failure, 2 destination not available (drive
//! unplugged, share offline), 3 destination busy (another run of the same
//! profile), 4 password missing or wrong (encrypted destination).
//!
//! The password of an encrypted profile comes from Windows Credential
//! Manager. Every run writes its log (see [`crate::history`]).

use crate::{activity::Op, credentials, history, storage::Storage};
use archeion_core::{BackupMode, ProfileId, RunEvent};
use archeion_engine::{EngineError, EventSink, RestoreRequest};
use std::{io::Write as _, path::PathBuf, process::ExitCode, sync::Mutex};

/// What the worker does.
#[derive(Debug)]
pub enum Work {
    Backup {
        mode: BackupMode,
    },
    /// Delete the named backup.
    Delete {
        archive: String,
    },
    /// By the profile's rules, or the archives from the first to the second
    /// name.
    Merge {
        range: Option<(String, String)>,
    },
    /// JSON [`RestoreRequest`] in a file written by the window.
    Restore {
        request: PathBuf,
    },
    /// Full integrity check, or a restore test of this many files.
    Verify {
        sample: Option<usize>,
    },
}

const EXIT_FAILED: u8 = 1;
const EXIT_DESTINATION_UNAVAILABLE: u8 = 2;
const EXIT_BUSY: u8 = 3;
const EXIT_PASSWORD: u8 = 4;
const EXIT_NO_SPACE: u8 = 5;

/// Writes each event as one JSON line on stdout, and every event except
/// progress to the run log. Write errors are ignored: the window may have
/// been closed while the run continues.
struct Sink {
    out: Mutex<std::io::Stdout>,
    log: history::Writer,
}

impl EventSink for Sink {
    fn emit(&self, event: RunEvent) {
        if history::is_logged(&event) {
            self.log.write(&history::Line::event(history::now_ms(), event.clone()));
        }
        let mut out = self.out.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = writeln!(out, "{}", event.to_line());
        let _ = out.flush();
    }
}

impl Work {
    fn op(&self) -> Op {
        match self {
            Self::Backup { .. } => Op::Backup,
            Self::Delete { .. } => Op::Delete,
            Self::Merge { .. } => Op::Merge,
            Self::Restore { .. } => Op::Restore,
            Self::Verify { .. } => Op::Verify,
        }
    }
}

/// Runs `work` on profile `id`, writing its log to `log` (or a new file in
/// the data folder).
pub fn run(storage: &Storage, id: ProfileId, work: Work, scheduled: bool, log: Option<PathBuf>) -> ExitCode {
    let _span = tracing::info_span!("run", profile = %id, ?work, scheduled).entered();
    let settings = storage.load();
    let profile = settings.as_ref().ok().and_then(|s| s.profile(id).cloned());

    // A restore request is read first: the restored backup goes in the log.
    let restore = match &work {
        Work::Restore { request } => {
            let parsed = std::fs::read_to_string(request)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<RestoreRequest>(&text).map_err(|e| e.to_string()))
                .map_err(|e| format!("restore request {}: {e}", request.display()));
            let _ = std::fs::remove_file(request);
            Some(parsed)
        }
        _ => None,
    };
    let detail = match (&work, &restore) {
        (Work::Backup { mode: BackupMode::Incremental }, _) => Some("incremental".to_owned()),
        (Work::Backup { mode: BackupMode::Complete }, _) => Some("complete".to_owned()),
        (Work::Backup { mode: BackupMode::Auto }, _) => None,
        (Work::Delete { archive }, _) => Some(archive.clone()),
        (Work::Merge { range: Some((a, b)) }, _) => Some(format!("{a} → {b}")),
        (Work::Merge { range: None }, _) => None,
        (Work::Restore { .. }, Some(Ok(req))) => Some(req.archive.clone()),
        (Work::Restore { .. }, _) => None,
        (Work::Verify { sample: None }, _) => Some(history::CHECK_FULL.to_owned()),
        (Work::Verify { sample: Some(_) }, _) => Some(history::CHECK_SAMPLE.to_owned()),
    };
    let after_scheduled_backup = scheduled && matches!(work, Work::Backup { .. });
    let header = history::Header {
        profile: id,
        profile_name: profile.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
        op: work.op(),
        detail,
        scheduled,
        started_ms: history::now_ms(),
    };
    let log_path = log.unwrap_or_else(|| history::new_log_path(storage.dir(), header.op, id));
    let sink = Sink { out: Mutex::new(std::io::stdout()), log: history::Writer::create(&log_path, &header) };

    let profile = match (profile, settings) {
        (Some(p), _) => p,
        (None, Ok(_)) => return fail(&sink, &format!("profile {id} not found")),
        (None, Err(e)) => return fail(&sink, &format!("{e:#}")),
    };
    // Read even for profiles not set to encrypt: the destination decides.
    let password = credentials::get_or_none(id);
    let pw = password.as_ref();
    // The engine reports progress and the final summary itself.
    let result = match work {
        Work::Backup { mode } => archeion_engine::backup(&profile, pw, mode, &sink).map(|_| ()),
        Work::Merge { range: None } => archeion_engine::merge(&profile, pw, &sink).map(|_| ()),
        Work::Merge { range: Some((first, last)) } => {
            archeion_engine::merge_range(&profile, pw, &first, &last, &sink).map(|_| ())
        }
        Work::Delete { archive } => {
            archeion_engine::delete_archive(&profile, pw, &archive, &sink).map(|_| ())
        }
        Work::Restore { .. } => match restore {
            Some(Ok(req)) => archeion_engine::restore(&profile, pw, &req, &sink).map(|_| ()),
            Some(Err(e)) => return fail(&sink, &e),
            None => return fail(&sink, "no restore request"),
        },
        Work::Verify { sample } => archeion_engine::verify(&profile, pw, sample, &sink).map(|_| ()),
    };
    // After a scheduled backup, the checks that are due run in the same
    // process, each with its own log.
    if after_scheduled_backup && result.is_ok() {
        let runs = history::list(storage.dir(), history::KEEP_LOGS);
        let last = |detail| history::last_finished(&runs, id, Op::Verify, detail);
        let due = history::check_due(
            &profile.verification,
            last(history::CHECK_FULL),
            last(history::CHECK_SAMPLE),
            history::now_ms(),
        );
        if let Some(sample) = due {
            tracing::info!(?sample, "integrity check due");
            drop(sink);
            let _ = run(storage, id, Work::Verify { sample }, true, None);
            return ExitCode::SUCCESS;
        }
    }
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(EngineError::DestinationUnavailable(location)) => {
            tracing::warn!(%location, "destination not available");
            sink.emit(RunEvent::DestinationUnavailable { location });
            ExitCode::from(EXIT_DESTINATION_UNAVAILABLE)
        }
        Err(EngineError::Busy) => {
            tracing::warn!("destination busy");
            sink.emit(RunEvent::Busy);
            ExitCode::from(EXIT_BUSY)
        }
        Err(EngineError::PasswordNeeded) => {
            tracing::warn!("no saved password for an encrypted destination");
            sink.emit(RunEvent::PasswordNeeded);
            ExitCode::from(EXIT_PASSWORD)
        }
        Err(EngineError::WrongPassword) => {
            tracing::warn!("the saved password does not open the destination");
            sink.emit(RunEvent::WrongPassword);
            ExitCode::from(EXIT_PASSWORD)
        }
        Err(EngineError::NotEnoughSpace { needed, available }) => {
            tracing::warn!(needed, available, "not enough free space on the destination");
            sink.emit(RunEvent::NotEnoughSpace { needed, available });
            ExitCode::from(EXIT_NO_SPACE)
        }
        Err(e) => fail(&sink, &e.to_string()),
    }
}

fn fail(sink: &Sink, message: &str) -> ExitCode {
    tracing::error!(error = message, "run failed");
    sink.emit(RunEvent::Failed { message: message.to_owned() });
    ExitCode::from(EXIT_FAILED)
}
