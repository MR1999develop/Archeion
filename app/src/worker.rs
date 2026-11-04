//! Headless run of one profile (`archeion run|merge|restore <id>`).
//!
//! Exit codes: 0 success, 1 failure, 2 destination not available (drive
//! unplugged, share offline), 3 destination busy (another run of the same
//! profile), 4 password missing or wrong (encrypted destination).
//!
//! The password of an encrypted profile comes from Windows Credential
//! Manager or, when none is saved, from the `ARCHEION_PASSWORD`
//! environment variable (the end-to-end tests). Every run writes its log (see [`crate::history`]). A scheduled
//! run that fails shows a Windows notification when the profile asks for
//! it (see [`crate::notify`]).

use crate::{activity::Op, credentials, history, i18n::Tr, notify, storage::Storage};
use archeion_core::{BackupMode, ProfileId, RunEvent};
use archeion_engine::{EngineError, EventSink, Password, RestoreRequest};
use std::{
    io::Write as _,
    path::PathBuf,
    process::ExitCode,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

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
    /// name; `drop_deleted`: the files deleted inside go, whatever the
    /// profile says.
    Merge {
        range: Option<(String, String)>,
        drop_deleted: bool,
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

pub(crate) const EXIT_FAILED: u8 = 1;
pub(crate) const EXIT_DESTINATION_UNAVAILABLE: u8 = 2;
const EXIT_BUSY: u8 = 3;
const EXIT_CANCELLED: u8 = 6;

/// Set once the window that started this run asked it to stop.
static STOP: AtomicBool = AtomicBool::new(false);

/// Watches standard input on a thread of its own: when the window closes
/// it (Cancel), the run stops at the next file, like an interruption.
pub fn stop_when_stdin_closes() {
    let spawned = std::thread::Builder::new().name("stop-watch".into()).spawn(|| {
        let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
        tracing::info!("asked to stop");
        STOP.store(true, Ordering::Relaxed);
    });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "watching for a stop request");
    }
}
pub(crate) const EXIT_PASSWORD: u8 = 4;
pub(crate) const EXIT_NO_SPACE: u8 = 5;

/// Writes each event as one JSON line on stdout, every event except
/// progress and files to the run log, and the files handled (skipped ones
/// included) to its companion file. The files do not go to stdout: the
/// window shows the recent files of the progress events. Write errors are
/// ignored: the window may have been closed while the run continues.
struct Sink {
    out: Mutex<std::io::Stdout>,
    log: history::Writer,
    files: history::Writer,
}

impl EventSink for Sink {
    fn stop_requested(&self) -> bool {
        STOP.load(Ordering::Relaxed)
    }

    fn emit(&self, event: RunEvent) {
        let now = history::now_ms();
        if matches!(event, RunEvent::Files { .. }) {
            self.files.write(&history::Line::event(now, event));
            return;
        }
        if history::is_file_event(&event) {
            self.files.write(&history::Line::event(now, event.clone()));
        }
        if history::is_logged(&event) {
            self.log.write(&history::Line::event(now, event.clone()));
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

/// Whether a backup started by an event should run: the profile still has
/// events, its destination is there, and its last backup is older than
/// `min_hours_between`. When not, nothing is logged in the operations log
/// (idle periods and other drives come often); the reason goes to the
/// diagnostic log.
pub fn event_backup_wanted(storage: &Storage, id: ProfileId) -> bool {
    let Some(profile) = storage.load().ok().and_then(|s| s.profile(id).cloned()) else {
        tracing::info!(profile = %id, "event backup: profile not found");
        return false;
    };
    if !profile.events.any() {
        tracing::info!(profile = %id, "event backup: no event chosen any more");
        return false;
    }
    if !archeion_engine::destination_reachable(&profile) {
        tracing::info!(profile = %id, "event backup: destination not connected, skipped");
        return false;
    }
    let runs = history::list(storage.dir(), history::KEEP_LOGS);
    let last = history::last_backup(&runs, id);
    let gap_ms = i64::from(profile.events.min_hours_between).saturating_mul(60 * 60 * 1000);
    let recent = last.is_some_and(|t| history::now_ms().saturating_sub(t) < gap_ms);
    if recent {
        tracing::info!(profile = %id, "event backup: last backup too recent, skipped");
    }
    !recent
}

/// `archeion watch`: notifications for profiles not backed up for too long.
pub fn watch(storage: &Storage) -> ExitCode {
    match storage.load() {
        Ok(settings) => {
            let tr = Tr::new(settings.language);
            notify::watch(storage, &tr, &settings.profiles, history::now_ms());
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!(error = format!("{e:#}"), "watch: settings unreadable");
            ExitCode::from(EXIT_FAILED)
        }
    }
}

/// Runs `work` on profile `id`, writing its log to `log` (or a new file in
/// the data folder).
pub fn run(storage: &Storage, id: ProfileId, work: Work, scheduled: bool, log: Option<PathBuf>) -> ExitCode {
    let _span = tracing::info_span!("run", profile = %id, ?work, scheduled).entered();
    if scheduled {
        // Nobody is waiting for it: low CPU, disk, memory and energy use.
        archeion_win::enter_background_mode();
    }
    let settings = storage.load();
    let profile = settings.as_ref().ok().and_then(|s| s.profile(id).cloned());

    // A restore request is read first: the restored backups go in the log.
    // Request files written by earlier versions are read too.
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
        (Work::Merge { range: Some((a, b)), .. }, _) => Some(format!("{a} → {b}")),
        (Work::Merge { range: None, .. }, _) => None,
        (Work::Restore { .. }, Some(Ok(req))) => Some(req.archives().join(", ")),
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
    let sink = Sink {
        out: Mutex::new(std::io::stdout()),
        log: history::Writer::create(&log_path, &header),
        files: history::Writer::create_plain(&history::files_path(&log_path)),
    };

    let tr =
        settings.as_ref().map_or_else(|_| Tr::new(archeion_core::Language::System), |s| Tr::new(s.language));
    let profile = match (profile, settings) {
        (Some(p), _) => p,
        (None, Ok(_)) => return fail(&sink, &format!("profile {id} not found")),
        (None, Err(e)) => return fail(&sink, &format!("{e:#}")),
    };
    // A failed automatic run tells the user, who is not watching.
    let alert = |problem: String| {
        if scheduled && profile.alerts.on_failure {
            let (title, body) = notify::failure_text(&tr, &profile, &problem);
            notify::toast(&title, &body);
        }
    };
    // Read even for profiles not set to encrypt: the destination decides.
    let password = credentials::get_or_none(id).or_else(|| {
        std::env::var(crate::extract::PASSWORD_VAR).ok().filter(|p| !p.is_empty()).and_then(Password::new)
    });
    let pw = password.as_ref();
    let changes_archives = matches!(work, Work::Backup { .. } | Work::Merge { .. } | Work::Delete { .. });
    if matches!(work, Work::Backup { .. } | Work::Verify { .. }) {
        warn_if_went_back(storage, &tr, &profile, pw);
    }
    // The engine reports progress and the final summary itself.
    let result = match work {
        Work::Backup { mode } => archeion_engine::backup(&profile, pw, mode, &sink).map(|report| {
            warn_if_changed_outside(&tr, &profile, report.changed_outside);
            warn_if_unusual(storage, &tr, &profile, &log_path, &report);
        }),
        Work::Merge { range, drop_deleted } => {
            let mut merging = profile.clone();
            merging.merge.keep_deleted &= !drop_deleted;
            match range {
                None => archeion_engine::merge(&merging, pw, &sink),
                Some((first, last)) => archeion_engine::merge_range(&merging, pw, &first, &last, &sink),
            }
            .map(|report| warn_if_changed_outside(&tr, &profile, report.changed_outside))
        }
        Work::Delete { archive } => archeion_engine::delete_archive(&profile, pw, &archive, &sink)
            .map(|report| warn_if_changed_outside(&tr, &profile, report.changed_outside)),
        Work::Restore { .. } => match restore {
            Some(Ok(req)) => archeion_engine::restore(&profile, pw, &req, &sink).map(|_| ()),
            Some(Err(e)) => return fail(&sink, &e),
            None => return fail(&sink, "no restore request"),
        },
        Work::Verify { sample } => archeion_engine::verify(&profile, pw, sample, &sink).map(|_| ()),
    };
    if changes_archives
        && result.is_ok()
        && let Some(newest) = newest_backup(&profile, pw)
    {
        crate::seen::record(storage.dir(), profile.id, newest.as_deref());
    }
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
            alert(tr.t_args("run-destination-missing", &[("location", crate::i18n::arg(location.clone()))]));
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
            alert(tr.t("run-password-needed"));
            sink.emit(RunEvent::PasswordNeeded);
            ExitCode::from(EXIT_PASSWORD)
        }
        Err(EngineError::WrongPassword) => {
            tracing::warn!("the saved password does not open the destination");
            alert(tr.t("run-wrong-password"));
            sink.emit(RunEvent::WrongPassword);
            ExitCode::from(EXIT_PASSWORD)
        }
        Err(EngineError::NotEnoughSpace { needed, available }) => {
            tracing::warn!(needed, available, "not enough free space on the destination");
            alert(tr.t_args(
                "run-no-space",
                &[
                    ("needed", crate::i18n::arg(tr.bytes(needed))),
                    ("available", crate::i18n::arg(tr.bytes(available))),
                ],
            ));
            sink.emit(RunEvent::NotEnoughSpace { needed, available });
            ExitCode::from(EXIT_NO_SPACE)
        }
        Err(EngineError::NetworkUnconfirmed) => {
            let text = tr.t("run-network-unconfirmed");
            alert(text.clone());
            fail(&sink, &text)
        }
        Err(EngineError::EncryptionMissing) => {
            let text = tr.t("run-encryption-missing");
            alert(text.clone());
            fail(&sink, &text)
        }
        Err(EngineError::Cancelled) => {
            // The window has written the cancellation in the log.
            tracing::info!("stopped on request");
            ExitCode::from(EXIT_CANCELLED)
        }
        Err(EngineError::ForeignDestination) => {
            let text = tr.t("run-foreign-destination");
            alert(text.clone());
            fail(&sink, &text)
        }
        Err(e) => {
            alert(tr.t_args("run-failed", &[("detail", crate::i18n::arg(e.to_string()))]));
            fail(&sink, &e.to_string())
        }
    }
}

/// The newest backup at the destination: `Some(None)` when there is none,
/// `None` when the destination cannot be read now.
fn newest_backup(profile: &archeion_core::Profile, pw: Option<&Password>) -> Option<Option<String>> {
    archeion_engine::list_archives(profile, pw).ok().map(|list| list.into_iter().next().map(|a| a.name))
}

/// Warns when the destination holds older backups than this computer last
/// saw there ([`crate::seen`]), then remembers what it holds now.
fn warn_if_went_back(storage: &Storage, tr: &Tr, profile: &archeion_core::Profile, pw: Option<&Password>) {
    let Some(newest) = newest_backup(profile, pw) else { return };
    let seen = crate::seen::load(storage.dir());
    let remembered = seen.get(&profile.id).map(String::as_str);
    if !crate::seen::went_back(remembered, newest.as_deref()) {
        return;
    }
    tracing::warn!(?remembered, ?newest, "the destination holds older backups than last seen");
    let title = tr.t_args("alert-went-back-title", &[("name", crate::i18n::arg(profile.name.clone()))]);
    notify::toast(&title, &tr.t("alert-went-back-body"));
    crate::seen::record(storage.dir(), profile.id, newest.as_deref());
}

/// When the run found backups removed or put back at the encrypted
/// destination from outside Archeion (the signed list did not match): a
/// notification, whoever started it. The run itself went on.
fn warn_if_changed_outside(tr: &Tr, profile: &archeion_core::Profile, changed: bool) {
    if !changed {
        return;
    }
    tracing::warn!("backups were removed or put back at the destination outside Archeion");
    let title = tr.t_args("alert-changed-outside-title", &[("name", crate::i18n::arg(profile.name.clone()))]);
    notify::toast(&title, &tr.t("alert-changed-outside-body"));
}

/// After an incremental backup that touched far more files than usual
/// ([`history::unusual_changes`]): a notification, whoever started it.
fn warn_if_unusual(
    storage: &Storage,
    tr: &Tr,
    profile: &archeion_core::Profile,
    log: &std::path::Path,
    report: &archeion_core::BackupReport,
) {
    if report.complete {
        return;
    }
    let current = report.files_new.saturating_add(report.files_changed).saturating_add(report.files_deleted);
    let runs = history::list(storage.dir(), history::KEEP_LOGS);
    if !history::unusual_changes(current, &history::incremental_changes(&runs, profile.id, log)) {
        return;
    }
    tracing::warn!(files = current, "far more files changed than usual");
    let title = tr.t_args("alert-unusual-title", &[("name", crate::i18n::arg(profile.name.clone()))]);
    let body = tr.t_args("alert-unusual-body", &[("count", crate::i18n::arg(current))]);
    notify::toast(&title, &body);
}

fn fail(sink: &Sink, message: &str) -> ExitCode {
    tracing::error!(error = message, "run failed");
    sink.emit(RunEvent::Failed { message: message.to_owned() });
    ExitCode::from(EXIT_FAILED)
}
