//! Log of the operations performed, like Areca's log tab.
//!
//! Every run (backup, merge, deletion, restore), started from the window or
//! by Task Scheduler, writes one file in the `logs` folder of the data
//! folder: a header line, then one line per event except progress. Lines
//! are appended as they happen, so a run that is stopped or crashes still
//! leaves its log, just without the final line. File names start with the
//! start time, so they sort in time order.

use crate::activity::Op;
use archeion_core::{ProfileId, RunEvent, profile::Verification};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub const LOG_DIR: &str = "logs";
/// Logs kept; older ones are deleted when the window opens.
pub const KEEP_LOGS: usize = 500;

/// First line of a run log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub profile: ProfileId,
    /// Name at the time of the run (the profile may be renamed or deleted).
    pub profile_name: String,
    pub op: Op,
    /// Backup kind asked for (`incremental`, `complete`), merged range or
    /// deleted backup, restored backup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub scheduled: bool,
    /// Milliseconds since the Unix epoch.
    pub started_ms: i64,
}

/// One logged moment of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// Milliseconds since the Unix epoch.
    pub at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<RunEvent>,
    /// The user stopped the run from the window.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cancelled: bool,
}

impl Line {
    pub fn event(at_ms: i64, event: RunEvent) -> Self {
        Self { at_ms, event: Some(event), cancelled: false }
    }

    pub fn cancelled(at_ms: i64) -> Self {
        Self { at_ms, event: None, cancelled: true }
    }

    /// True for the line that ends a run.
    pub fn is_terminal(&self) -> bool {
        self.cancelled || self.event.as_ref().is_some_and(RunEvent::is_terminal)
    }
}

/// Progress is shown live, not logged.
pub fn is_logged(event: &RunEvent) -> bool {
    !matches!(event, RunEvent::Progress { .. })
}

/// Now, in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

/// `2026-10-30 12:31:05`, local time.
pub fn local_time(ms: i64) -> String {
    jiff::Timestamp::from_millisecond(ms)
        .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// Path of a new run log in `dir`: `20261030-123105-123-backup-6f1c3d1e.jsonl`.
pub fn new_log_path(dir: &Path, op: Op, profile: ProfileId) -> PathBuf {
    let now = jiff::Zoned::now();
    let id = profile.to_string();
    let short = id.get(..8).unwrap_or(&id);
    let stamp = format!("{}-{:03}", now.strftime("%Y%m%d-%H%M%S"), now.millisecond());
    dir.join(LOG_DIR).join(format!("{stamp}-{}-{short}.jsonl", op.command()))
}

/// Appends lines to one run log. Write errors are logged and otherwise
/// ignored: the run matters more than its log.
#[derive(Debug)]
pub struct Writer {
    file: Mutex<Option<File>>,
}

impl Writer {
    /// Creates the log at `path` (and its folder) with `header`.
    pub fn create(path: &Path, header: &Header) -> Self {
        let file = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| OpenOptions::new().create(true).append(true).open(path));
        let file = match file {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "cannot create the run log");
                None
            }
        };
        let w = Self { file: Mutex::new(file) };
        w.write_json(header);
        w
    }

    /// Appends to an existing log (the window adding "cancelled").
    pub fn append_to(path: &Path, line: &Line) {
        let w = Self { file: Mutex::new(OpenOptions::new().append(true).open(path).ok()) };
        w.write(line);
    }

    pub fn write(&self, line: &Line) {
        self.write_json(line);
    }

    fn write_json<T: Serialize>(&self, value: &T) {
        let Ok(mut text) = serde_json::to_string(value) else { return };
        text.push('\n');
        let mut guard = self.file.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(f) = guard.as_mut()
            && let Err(e) = f.write_all(text.as_bytes())
        {
            tracing::warn!(error = %e, "writing the run log");
        }
    }
}

/// A run log read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLog {
    pub path: PathBuf,
    pub header: Header,
    pub lines: Vec<Line>,
}

impl RunLog {
    /// Parses a log file's text; unreadable lines are skipped (a run killed
    /// in the middle of a write leaves half a line).
    pub fn parse(path: PathBuf, text: &str) -> Option<Self> {
        let mut it = text.lines().filter(|l| !l.trim().is_empty());
        let header: Header = serde_json::from_str(it.next()?).ok()?;
        let lines = it.filter_map(|l| serde_json::from_str(l).ok()).collect();
        Some(Self { path, header, lines })
    }

    /// The line that ended the run; `None` while it runs or if it was
    /// interrupted.
    pub fn end(&self) -> Option<&Line> {
        self.lines.iter().rev().find(|l| l.is_terminal())
    }

    pub fn skipped(&self) -> usize {
        self.lines.iter().filter(|l| matches!(l.event, Some(RunEvent::Skipped(_)))).count()
    }
}

/// Logs in `dir`, newest first, at most `limit`.
pub fn list(dir: &Path, limit: usize) -> Vec<RunLog> {
    let mut paths = log_files(dir);
    paths.reverse();
    paths
        .into_iter()
        .take(limit)
        .filter_map(|p| fs::read_to_string(&p).ok().and_then(|t| RunLog::parse(p, &t)))
        .collect()
}

/// `detail` of a full integrity check and of a restore test.
pub const CHECK_FULL: &str = "full";
pub const CHECK_SAMPLE: &str = "sample";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Start of the newest run of `profile` with `op` and `detail` that
/// finished (its last line is a summary).
pub fn last_finished(runs: &[RunLog], profile: ProfileId, op: Op, detail: &str) -> Option<i64> {
    runs.iter()
        .filter(|r| {
            r.header.profile == profile && r.header.op == op && r.header.detail.as_deref() == Some(detail)
        })
        .filter(|r| matches!(r.end().and_then(|l| l.event.as_ref()), Some(RunEvent::Finished { .. })))
        .map(|r| r.header.started_ms)
        .max()
}

/// Which check is due now: `Some(None)` the full check, `Some(Some(n))` a
/// restore test of `n` files, `None` nothing. A full check also counts as
/// a restore test. Every-0-days means never.
pub fn check_due(
    v: &Verification,
    last_full: Option<i64>,
    last_sample: Option<i64>,
    now_ms: i64,
) -> Option<Option<usize>> {
    let due = |days: u32, last: Option<i64>| {
        days > 0 && last.is_none_or(|t| now_ms.saturating_sub(t) >= i64::from(days).saturating_mul(DAY_MS))
    };
    if due(v.check_every_days, last_full) {
        Some(None)
    } else if due(v.restore_test_every_days, last_sample.max(last_full)) {
        Some(Some(usize::try_from(v.restore_test_files.max(1)).unwrap_or(1)))
    } else {
        None
    }
}

/// Deletes all but the newest `keep` logs.
pub fn prune(dir: &Path, keep: usize) {
    let paths = log_files(dir);
    let excess = paths.len().saturating_sub(keep);
    for p in paths.into_iter().take(excess) {
        if let Err(e) = fs::remove_file(&p) {
            tracing::warn!(error = %e, path = %p.display(), "removing an old run log");
        }
    }
}

/// Log files, oldest first.
fn log_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir.join(LOG_DIR)) else { return Vec::new() };
    let mut paths: Vec<PathBuf> =
        entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jsonl")).collect();
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{Phase, Skipped, Summary};

    fn header() -> Header {
        Header {
            profile: "6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(),
            profile_name: "Documenti".into(),
            op: Op::Backup,
            detail: Some("complete".into()),
            scheduled: true,
            started_ms: 1_793_000_000_000,
        }
    }

    #[test]
    fn a_log_is_written_and_read_back_without_progress() {
        let dir = tempfile::tempdir().unwrap();
        let h = header();
        let path = new_log_path(dir.path(), h.op, h.profile);
        let w = Writer::create(&path, &h);
        for ev in [
            RunEvent::Phase { phase: Phase::Copying },
            RunEvent::Progress { done: 1, total: 2, files_done: 1, files_total: 2, current: String::new() },
            RunEvent::Skipped(Skipped { path: r"C:\a.pst".into(), reason: "in use".into() }),
            RunEvent::Finished { summary: Summary::Backup(archeion_core::BackupReport::default()) },
        ] {
            if is_logged(&ev) {
                w.write(&Line::event(1, ev));
            }
        }
        let logs = list(dir.path(), 10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].header, h);
        assert_eq!(logs[0].lines.len(), 3);
        assert_eq!(logs[0].skipped(), 1);
        assert!(logs[0].end().is_some());
    }

    #[test]
    fn an_interrupted_run_has_no_end_and_a_cut_line_is_skipped() {
        let text = format!(
            "{}\n{}\n{{\"at_ms\":3,\"ev",
            serde_json::to_string(&header()).unwrap(),
            serde_json::to_string(&Line::event(2, RunEvent::Phase { phase: Phase::Scanning })).unwrap()
        );
        let log = RunLog::parse("x.jsonl".into(), &text).unwrap();
        assert_eq!(log.lines.len(), 1);
        assert!(log.end().is_none());
        let mut cancelled = log.clone();
        cancelled.lines.push(Line::cancelled(4));
        assert!(cancelled.end().is_some_and(|l| l.cancelled));
    }

    #[test]
    fn newest_first_and_old_logs_pruned() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(LOG_DIR)).unwrap();
        for name in [
            "20261001-000000-000-run-a.jsonl",
            "20261002-000000-000-run-a.jsonl",
            "20261003-000000-000-run-a.jsonl",
        ] {
            Writer::create(&dir.path().join(LOG_DIR).join(name), &header());
        }
        prune(dir.path(), 2);
        let logs = list(dir.path(), 10);
        let names: Vec<_> =
            logs.iter().map(|l| l.path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["20261003-000000-000-run-a.jsonl", "20261002-000000-000-run-a.jsonl"]);
    }

    #[test]
    fn checks_fall_due_by_their_own_period() {
        let v = Verification { check_every_days: 30, restore_test_every_days: 7, restore_test_files: 20 };
        let day = DAY_MS;
        assert_eq!(check_due(&v, None, None, 100 * day), Some(None), "never checked: full check");
        assert_eq!(check_due(&v, Some(90 * day), None, 100 * day), Some(Some(20)), "test overdue");
        assert_eq!(check_due(&v, Some(95 * day), Some(80 * day), 100 * day), None, "full counts as a test");
        assert_eq!(check_due(&v, Some(70 * day), Some(99 * day), 100 * day), Some(None));
        let never = Verification { check_every_days: 0, restore_test_every_days: 0, restore_test_files: 5 };
        assert_eq!(check_due(&never, None, None, 100 * day), None);
    }

    #[test]
    fn the_last_finished_check_is_found() {
        let mut h = header();
        h.op = Op::Verify;
        h.detail = Some(CHECK_FULL.into());
        let done = RunLog {
            path: "a".into(),
            header: h.clone(),
            lines: vec![Line::event(
                5,
                RunEvent::Finished { summary: archeion_core::Summary::Verify(Default::default()) },
            )],
        };
        let unfinished = RunLog {
            path: "b".into(),
            header: Header { started_ms: h.started_ms + 10, ..h.clone() },
            lines: vec![],
        };
        let runs = vec![unfinished, done];
        assert_eq!(last_finished(&runs, h.profile, Op::Verify, CHECK_FULL), Some(h.started_ms));
        assert_eq!(last_finished(&runs, h.profile, Op::Verify, CHECK_SAMPLE), None);
    }

    #[test]
    fn times_are_shown_locally_to_the_second() {
        assert_eq!(local_time(now_ms()).len(), "2026-10-30 12:31:05".len());
    }
}
