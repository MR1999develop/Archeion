//! What a run is doing, built from its [`RunEvent`]s: shown by the
//! Activity tab while it runs and kept afterwards as its summary.
//!
//! Pure state: time comes in as `now`, so speed and remaining time are
//! testable without a clock.

use crate::history::{self, Line};
use archeion_core::{Phase, ProfileId, RunEvent, Skipped, Summary};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
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
    /// (time, bytes done) samples for the speed.
    samples: VecDeque<(Instant, u64)>,
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
            samples: VecDeque::new(),
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
            }
            RunEvent::Progress { done, total, files_done, files_total, current } => {
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
            }
            RunEvent::Skipped(s) => {
                self.skipped_count += 1;
                if self.skipped.len() < SKIPPED_SHOWN {
                    self.skipped.push(s);
                }
            }
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

    /// A cancel that raced with the last event keeps "cancelled".
    pub fn set_outcome(&mut self, o: Outcome) {
        if self.outcome.is_none() {
            self.outcome = Some(o);
        }
    }

    /// The run is over (process ended) at `now`.
    pub fn end(&mut self, now: Instant) {
        self.ended.get_or_insert(now);
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

    /// Phases of this operation with their state.
    pub fn phases(&self) -> Vec<(Phase, PhaseState)> {
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
                (*p, state)
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
        RunEvent::Progress { done, total, files_done, files_total, current: current.into() }
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
        let states: Vec<PhaseState> = a.phases().into_iter().map(|(_, s)| s).collect();
        assert_eq!(states, vec![PhaseState::Done, PhaseState::Current, PhaseState::Pending]);
        a.apply(RunEvent::Finished { summary: Summary::Backup(archeion_core::BackupReport::default()) }, t);
        a.end(t);
        assert!(a.phases().iter().all(|(_, s)| *s == PhaseState::Done));
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
