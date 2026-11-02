//! What a run is doing, built from its [`RunEvent`]s: shown by the
//! Activity tab while it runs and kept afterwards as its summary.
//!
//! Pure state: time comes in as `now`, so speed, remaining time and step
//! durations are testable without a clock.

use crate::history::{self, Line};
use archeion_core::{
    FileAction, MergeGroup, Phase, PhaseDetail, ProfileId, RunEvent, Skipped, Summary, Tally,
    run::RECENT_FILES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

/// Window over which the speed is averaged.
const SPEED_WINDOW: Duration = Duration::from_secs(5);
/// Remaining time is shown only after this much copying, so the first
/// unstable estimates are not displayed.
const ETA_AFTER: Duration = Duration::from_secs(3);
/// Skipped files kept for display; the count goes on beyond it.
const SKIPPED_SHOWN: usize = 1000;

/// What a run does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Backup,
    Merge,
    /// Deleting one backup (a merge with the next one, or a removal).
    Delete,
    Restore,
    /// Integrity check or restore test.
    Verify,
}

impl Op {
    /// Worker subcommand.
    pub fn command(self) -> &'static str {
        match self {
            Self::Backup => "run",
            Self::Merge => "merge",
            Self::Delete => "delete",
            Self::Restore => "restore",
            Self::Verify => "verify",
        }
    }

    /// The phases this operation goes through, in order.
    pub fn phases(self) -> &'static [Phase] {
        match self {
            Self::Backup => &[Phase::Opening, Phase::Scanning, Phase::Copying],
            Self::Merge | Self::Delete => &[Phase::Opening, Phase::Merging],
            Self::Restore => &[Phase::Opening, Phase::Restoring],
            Self::Verify => &[Phase::Opening, Phase::Verifying],
        }
    }
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Finished(Summary),
    DestinationUnavailable(String),
    Busy,
    /// Encrypted destination without a saved password.
    PasswordNeeded,
    /// The saved password does not open the destination.
    WrongPassword,
    /// The destination is too full (bytes needed, bytes free).
    NotEnoughSpace {
        needed: u64,
        available: u64,
    },
    Failed(String),
    Cancelled,
}

/// State of a phase in the phase list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseState {
    Done,
    Current,
    Pending,
}

/// One count shown next to a step, such as "12 new".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Count {
    /// Files found while scanning.
    Found,
    /// Folders read while scanning.
    Folders,
    New,
    Changed,
    /// Same as in the previous backup (scanning).
    Unchanged,
    /// Unchanged but copied by a complete backup.
    CopiedAgain,
    Moved,
    Restored,
    Kept,
    Checked,
    Damaged,
    Skipped,
}

/// One step of the run as listed in the Activity tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub phase: Phase,
    pub state: PhaseState,
    /// When it started, in milliseconds since the Unix epoch.
    pub started_ms: Option<i64>,
    /// How long it took, or has been running.
    pub took: Option<Duration>,
    /// What it did so far, only the counts that are not zero.
    pub counts: Vec<(Count, u64)>,
}

/// What happened to a recently handled file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handled {
    Done { action: FileAction, bytes: u64 },
    Skipped { reason: String },
}

/// A file in the list of the most recent ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    pub handled: Handled,
}

/// A phase that started, with what it did.
#[derive(Debug, Clone)]
struct StepRecord {
    phase: Phase,
    started: Instant,
    started_ms: i64,
    ended: Option<Instant>,
    files_done: u64,
    folders: u64,
    tally: Tally,
    skipped: u64,
}

impl StepRecord {
    /// Counts worth showing, in reading order.
    fn counts(&self) -> Vec<(Count, u64)> {
        let mut out = Vec::new();
        if self.phase == Phase::Scanning {
            out.push((Count::Found, self.files_done));
            out.push((Count::Folders, self.folders));
        }
        let unchanged = if self.phase == Phase::Copying { Count::CopiedAgain } else { Count::Unchanged };
        let t = &self.tally;
        out.extend([
            (Count::New, t.new),
            (Count::Changed, t.changed),
            (unchanged, t.unchanged),
            (Count::Moved, t.moved),
            (Count::Restored, t.restored),
            (Count::Kept, t.kept),
            (Count::Checked, t.checked),
            (Count::Damaged, t.damaged),
            (Count::Skipped, self.skipped),
        ]);
        out.retain(|(_, n)| *n > 0);
        out
    }
}

#[derive(Debug, Clone)]
pub struct Activity {
    pub profile: ProfileId,
    pub op: Op,
    pub started: Instant,
    /// Set when the run is over.
    pub ended: Option<Instant>,
    pub phase: Option<Phase>,
    phase_started: Instant,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub files_done: u64,
    pub files_total: u64,
    /// File or folder being worked on.
    pub current: String,
    pub skipped: Vec<Skipped>,
    pub skipped_count: usize,
    pub outcome: Option<Outcome>,
    /// The run's log so far, as written to its log file.
    pub log: Vec<Line>,
    /// That log file, once known (the worker writes it; the Activity tab
    /// reads it back for the full log).
    pub log_file: Option<PathBuf>,
    /// Group of backups a merge is fusing now.
    pub merge_group: Option<MergeGroup>,
    /// (time, bytes done) samples for the speed.
    samples: VecDeque<(Instant, u64)>,
    /// Phases started so far, in order.
    steps: Vec<StepRecord>,
    /// Files handled in the current phase, newest first, at most
    /// [`RECENT_FILES`].
    recent: VecDeque<RecentFile>,
}

impl Activity {
    pub fn new(profile: ProfileId, op: Op, now: Instant) -> Self {
        Self {
            profile,
            op,
            started: now,
            ended: None,
            phase: None,
            phase_started: now,
            done_bytes: 0,
            total_bytes: 0,
            files_done: 0,
            files_total: 0,
            current: String::new(),
            skipped: Vec::new(),
            skipped_count: 0,
            outcome: None,
            log: Vec::new(),
            log_file: None,
            merge_group: None,
            samples: VecDeque::new(),
            steps: Vec::new(),
            recent: VecDeque::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.ended.is_none()
    }

    /// Applies one event received at `now`.
    pub fn apply(&mut self, ev: RunEvent, now: Instant) {
        if history::is_logged(&ev) {
            self.log.push(Line::event(history::now_ms(), ev.clone()));
        }
        match ev {
            RunEvent::Phase { phase } => {
                self.phase = Some(phase);
                self.phase_started = now;
                self.done_bytes = 0;
                self.total_bytes = 0;
                self.files_done = 0;
                self.files_total = 0;
                self.current.clear();
                self.samples.clear();
                self.recent.clear();
                self.merge_group = None;
                self.close_step(now);
                self.steps.push(StepRecord {
                    phase,
                    started: now,
                    started_ms: history::now_ms(),
                    ended: None,
                    files_done: 0,
                    folders: 0,
                    tally: Tally::default(),
                    skipped: 0,
                });
            }
            RunEvent::Progress { done, total, files_done, files_total, current, detail } => {
                self.done_bytes = done;
                self.total_bytes = total;
                self.files_done = files_done;
                self.files_total = files_total;
                if !current.is_empty() {
                    self.current = current;
                }
                self.samples.push_back((now, done));
                while self.samples.front().is_some_and(|(t, _)| now.duration_since(*t) > SPEED_WINDOW) {
                    self.samples.pop_front();
                }
                self.apply_detail(files_done, detail);
            }
            RunEvent::Skipped(s) => {
                self.skipped_count += 1;
                if let Some(step) = self.steps.last_mut() {
                    step.skipped += 1;
                }
                self.push_recent(RecentFile {
                    path: s.path.clone(),
                    handled: Handled::Skipped { reason: s.reason.clone() },
                });
                if self.skipped.len() < SKIPPED_SHOWN {
                    self.skipped.push(s);
                }
            }
            // The complete list goes to the run's companion file; the
            // window shows the recent files of the progress events.
            RunEvent::Files { .. } => {}
            RunEvent::Finished { summary } => self.set_outcome(Outcome::Finished(summary)),
            RunEvent::Busy => self.set_outcome(Outcome::Busy),
            RunEvent::PasswordNeeded => self.set_outcome(Outcome::PasswordNeeded),
            RunEvent::WrongPassword => self.set_outcome(Outcome::WrongPassword),
            RunEvent::NotEnoughSpace { needed, available } => {
                self.set_outcome(Outcome::NotEnoughSpace { needed, available });
            }
            RunEvent::DestinationUnavailable { location } => {
                self.set_outcome(Outcome::DestinationUnavailable(location));
            }
            RunEvent::Failed { message } => self.set_outcome(Outcome::Failed(message)),
        }
    }

    /// Counts and recent files of a progress event.
    fn apply_detail(&mut self, files_done: u64, detail: PhaseDetail) {
        let PhaseDetail { folders, tally, recent, group } = detail;
        if let Some(step) = self.steps.last_mut() {
            step.files_done = files_done;
            step.folders = folders;
            step.tally = tally;
        }
        for f in recent {
            self.push_recent(RecentFile {
                path: f.path,
                handled: Handled::Done { action: f.action, bytes: f.bytes },
            });
        }
        self.merge_group = group;
    }

    fn push_recent(&mut self, f: RecentFile) {
        self.recent.push_front(f);
        self.recent.truncate(RECENT_FILES);
    }

    /// The current phase ends at `now`.
    fn close_step(&mut self, now: Instant) {
        if let Some(step) = self.steps.last_mut() {
            step.ended.get_or_insert(now);
        }
    }

    /// Files handled in the current phase, newest first.
    pub fn recent(&self) -> impl Iterator<Item = &RecentFile> {
        self.recent.iter()
    }

    /// A cancel that raced with the last event keeps "cancelled".
    pub fn set_outcome(&mut self, o: Outcome) {
        if self.outcome.is_none() {
            self.outcome = Some(o);
        }
    }

    /// The run is over (process ended) at `now`.
    pub fn end(&mut self, now: Instant) {
        self.ended.get_or_insert(now);
        self.close_step(now);
        self.current.clear();
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.ended.unwrap_or(now).duration_since(self.started)
    }

    /// 0.0–1.0, or `None` while the total is unknown.
    pub fn fraction(&self) -> Option<f32> {
        (self.total_bytes > 0).then(|| {
            #[expect(clippy::cast_precision_loss, reason = "display only")]
            let f = self.done_bytes as f64 / self.total_bytes as f64;
            #[expect(clippy::cast_possible_truncation, reason = "clamped to 0..=1")]
            let f = f.clamp(0.0, 1.0) as f32;
            f
        })
    }

    /// Bytes per second over the last few seconds, if measurable.
    pub fn speed(&self) -> Option<f64> {
        let (t0, b0) = *self.samples.front()?;
        let (t1, b1) = *self.samples.back()?;
        let secs = t1.duration_since(t0).as_secs_f64();
        #[expect(clippy::cast_precision_loss, reason = "display only")]
        let bytes = b1.saturating_sub(b0) as f64;
        (secs >= 1.0 && bytes > 0.0).then(|| bytes / secs)
    }

    /// Estimated time left in the current phase.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        if !self.is_running() || self.total_bytes == 0 || now.duration_since(self.phase_started) < ETA_AFTER {
            return None;
        }
        let speed = self.speed()?;
        #[expect(clippy::cast_precision_loss, reason = "display only")]
        let left = self.total_bytes.saturating_sub(self.done_bytes) as f64;
        Some(Duration::from_secs_f64(left / speed))
    }

    /// Steps of this operation: state, start, duration and what each did.
    pub fn steps(&self, now: Instant) -> Vec<Step> {
        let all = self.op.phases();
        let current = self.phase.and_then(|p| all.iter().position(|x| *x == p));
        let finished = matches!(self.outcome, Some(Outcome::Finished(_)));
        all.iter()
            .enumerate()
            .map(|(i, p)| {
                let state = match current {
                    _ if finished => PhaseState::Done,
                    Some(c) if i < c => PhaseState::Done,
                    Some(c) if i == c && self.is_running() => PhaseState::Current,
                    Some(c) if i == c => PhaseState::Done,
                    _ => PhaseState::Pending,
                };
                let record = self.steps.iter().rev().find(|r| r.phase == *p);
                Step {
                    phase: *p,
                    state,
                    started_ms: record.map(|r| r.started_ms),
                    took: record.map(|r| r.ended.unwrap_or(now).saturating_duration_since(r.started)),
                    counts: record.map(StepRecord::counts).unwrap_or_default(),
                }
            })
            .collect()
    }
}

/// `1 h 05 min`, `3 min 20 s`, `45 s`.
pub fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h} h {m:02} min")
    } else if m > 0 {
        format!("{m} min {sec:02} s")
    } else {
        format!("{sec} s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(done: u64, total: u64, files_done: u64, files_total: u64, current: &str) -> RunEvent {
        RunEvent::Progress {
            done,
            total,
            files_done,
            files_total,
            current: current.into(),
            detail: PhaseDetail::default(),
        }
    }

    fn with_detail(files_done: u64, detail: PhaseDetail) -> RunEvent {
        RunEvent::Progress { done: 0, total: 0, files_done, files_total: 0, current: String::new(), detail }
    }

    fn done(path: &str, action: FileAction) -> archeion_core::FileDone {
        archeion_core::FileDone::new(path, action, 10)
    }

    fn act(now: Instant) -> Activity {
        Activity::new("6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(), Op::Backup, now)
    }

    #[test]
    fn progress_fills_counters_and_keeps_the_last_current_file() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Copying }, t);
        a.apply(progress(50, 200, 3, 10, r"C:\a.txt"), t);
        a.apply(progress(60, 200, 3, 10, ""), t);
        assert_eq!((a.done_bytes, a.files_done, a.files_total), (60, 3, 10));
        assert_eq!(a.current, r"C:\a.txt");
        assert_eq!(a.fraction(), Some(0.3));
    }

    #[test]
    fn a_new_phase_resets_the_counters() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(progress(10, 0, 500, 0, r"C:\x"), t);
        a.apply(RunEvent::Phase { phase: Phase::Copying }, t);
        assert_eq!((a.files_done, a.done_bytes), (0, 0));
        assert!(a.current.is_empty());
        assert_eq!(a.fraction(), None);
    }

    #[test]
    fn speed_and_remaining_time_follow_the_samples() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Copying }, t);
        a.apply(progress(0, 1000, 0, 1, ""), t);
        a.apply(progress(100, 1000, 1, 1, ""), t + Duration::from_secs(2));
        assert_eq!(a.speed(), Some(50.0));
        assert_eq!(a.remaining(t + Duration::from_secs(2)), None, "too early to estimate");
        a.apply(progress(200, 1000, 1, 1, ""), t + Duration::from_secs(4));
        assert_eq!(a.remaining(t + Duration::from_secs(4)), Some(Duration::from_secs(16)));
    }

    #[test]
    fn old_samples_leave_the_speed_window() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(progress(0, 1000, 0, 0, ""), t);
        a.apply(progress(500, 1000, 0, 0, ""), t + Duration::from_secs(1));
        a.apply(progress(510, 1000, 0, 0, ""), t + Duration::from_secs(10));
        assert_eq!(a.speed(), None, "only one sample in the last 5 s");
    }

    #[test]
    fn phases_show_done_current_pending_and_all_done_at_the_end() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Scanning }, t);
        let states: Vec<PhaseState> = a.steps(t).into_iter().map(|s| s.state).collect();
        assert_eq!(states, vec![PhaseState::Done, PhaseState::Current, PhaseState::Pending]);
        a.apply(RunEvent::Finished { summary: Summary::Backup(archeion_core::BackupReport::default()) }, t);
        a.end(t);
        assert!(a.steps(t).iter().all(|s| s.state == PhaseState::Done));
    }

    #[test]
    fn each_step_keeps_its_start_and_duration() {
        let t = Instant::now();
        let sec = Duration::from_secs;
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Opening }, t);
        a.apply(RunEvent::Phase { phase: Phase::Scanning }, t + sec(2));
        let steps = a.steps(t + sec(10));
        assert_eq!(steps.iter().map(|s| s.took).collect::<Vec<_>>(), vec![Some(sec(2)), Some(sec(8)), None]);
        assert!(steps[0].started_ms.is_some() && steps[2].started_ms.is_none());
        a.end(t + sec(12));
        assert_eq!(a.steps(t + sec(60))[1].took, Some(sec(10)), "the last step stops when the run ends");
    }

    #[test]
    fn scanning_counts_show_found_folders_and_changes_without_zeros() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Scanning }, t);
        let tally = Tally { new: 2, unchanged: 7, ..Tally::default() };
        a.apply(with_detail(9, PhaseDetail { folders: 3, tally, ..PhaseDetail::default() }), t);
        a.apply(RunEvent::Skipped(Skipped { path: "x".into(), reason: "denied".into() }), t);
        let scanning = a.steps(t).into_iter().find(|s| s.phase == Phase::Scanning).unwrap();
        assert_eq!(
            scanning.counts,
            vec![
                (Count::Found, 9),
                (Count::Folders, 3),
                (Count::New, 2),
                (Count::Unchanged, 7),
                (Count::Skipped, 1)
            ]
        );
        // Counts of a finished step stay once the next one begins.
        a.apply(RunEvent::Phase { phase: Phase::Copying }, t);
        a.apply(
            with_detail(
                1,
                PhaseDetail { tally: Tally { unchanged: 1, ..Tally::default() }, ..PhaseDetail::default() },
            ),
            t,
        );
        let steps = a.steps(t);
        assert_eq!(steps[1].counts.len(), 5);
        assert_eq!(steps[2].counts, vec![(Count::CopiedAgain, 1)]);
    }

    #[test]
    fn recent_files_are_newest_first_capped_and_reset_by_a_new_phase() {
        let t = Instant::now();
        let mut a = act(t);
        a.apply(RunEvent::Phase { phase: Phase::Copying }, t);
        a.apply(
            with_detail(
                2,
                PhaseDetail {
                    recent: vec![done("a", FileAction::New), done("b", FileAction::Changed)],
                    ..PhaseDetail::default()
                },
            ),
            t,
        );
        a.apply(RunEvent::Skipped(Skipped { path: "c".into(), reason: "in use".into() }), t);
        let paths: Vec<&str> = a.recent().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["c", "b", "a"]);
        assert_eq!(
            a.recent().nth(1).unwrap().handled,
            Handled::Done { action: FileAction::Changed, bytes: 10 }
        );

        let many = (0..RECENT_FILES + 7).map(|i| done(&format!("f{i}"), FileAction::New)).collect();
        a.apply(with_detail(40, PhaseDetail { recent: many, ..PhaseDetail::default() }), t);
        assert_eq!(a.recent().count(), RECENT_FILES);
        assert_eq!(a.recent().next().unwrap().path, format!("f{}", RECENT_FILES + 6));

        a.apply(RunEvent::Phase { phase: Phase::Merging }, t);
        assert_eq!(a.recent().count(), 0);
    }

    #[test]
    fn the_merge_group_follows_the_progress_events() {
        let t = Instant::now();
        let mut a = Activity::new("6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(), Op::Merge, t);
        a.apply(RunEvent::Phase { phase: Phase::Merging }, t);
        let group = MergeGroup { index: 2, of: 3, archives: 4, first: "a".into(), last: "b".into() };
        a.apply(with_detail(0, PhaseDetail { group: Some(group.clone()), ..PhaseDetail::default() }), t);
        assert_eq!(a.merge_group, Some(group));
        a.apply(RunEvent::Phase { phase: Phase::Verifying }, t);
        assert_eq!(a.merge_group, None);
    }

    #[test]
    fn skipped_files_are_counted_beyond_the_shown_list() {
        let t = Instant::now();
        let mut a = act(t);
        for i in 0..(SKIPPED_SHOWN + 5) {
            a.apply(RunEvent::Skipped(Skipped { path: format!("f{i}"), reason: "in use".into() }), t);
        }
        assert_eq!(a.skipped.len(), SKIPPED_SHOWN);
        assert_eq!(a.skipped_count, SKIPPED_SHOWN + 5);
    }

    #[test]
    fn cancel_wins_over_a_late_finish() {
        let t = Instant::now();
        let mut a = act(t);
        a.set_outcome(Outcome::Cancelled);
        a.apply(RunEvent::Failed { message: "killed".into() }, t);
        assert_eq!(a.outcome, Some(Outcome::Cancelled));
    }

    #[test]
    fn durations_read_naturally() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45 s");
        assert_eq!(format_duration(Duration::from_secs(200)), "3 min 20 s");
        assert_eq!(format_duration(Duration::from_secs(3900)), "1 h 05 min");
    }
}
