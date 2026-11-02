//! Throttled progress of one phase: bytes, files, the current file, and
//! what was done so far (counts per outcome, the most recent files); and
//! the complete list of the files handled, sent in batches that are never
//! dropped ([`RunEvent::Files`]).

use crate::EventSink;
use archeion_core::{
    FileAction, FileDone, MergeGroup, PhaseDetail, RunEvent, Skipped, Tally, run::RECENT_FILES,
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// Minimum time between two progress events, so the stdout pipe and the
/// window are not flooded.
const EMIT_INTERVAL: Duration = Duration::from_millis(200);
/// Files per [`RunEvent::Files`] batch at most: a few tens of KB of JSON.
const FILES_BATCH: usize = 256;
/// Longest wait before the files handled so far are sent, so the log of a
/// run that is killed misses at most this much.
const FILES_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct Progress<'a> {
    sink: &'a dyn EventSink,
    done: u64,
    total: u64,
    files_done: u64,
    files_total: u64,
    current: String,
    folders: u64,
    tally: Tally,
    /// Files finished since the last event, oldest first, at most
    /// [`RECENT_FILES`].
    recent: VecDeque<FileDone>,
    group: Option<MergeGroup>,
    last: Option<Instant>,
    /// Files handled and not sent yet, in order.
    files: Vec<FileDone>,
    files_sent: Instant,
}

impl<'a> Progress<'a> {
    /// A phase with known totals (0 = unknown).
    pub(crate) fn new(sink: &'a dyn EventSink, total_bytes: u64, total_files: u64) -> Self {
        let mut p = Self {
            sink,
            done: 0,
            total: total_bytes,
            files_done: 0,
            files_total: total_files,
            current: String::new(),
            folders: 0,
            tally: Tally::default(),
            recent: VecDeque::new(),
            group: None,
            last: None,
            files: Vec::new(),
            files_sent: Instant::now(),
        };
        p.emit();
        p
    }

    /// The next file begins; `path` is shown as the current file.
    pub(crate) fn start_file(&mut self, path: impl Into<String>) {
        self.files_done = self.files_done.saturating_add(1);
        self.current = path.into();
        self.maybe_emit();
    }

    /// Bytes of the current file processed.
    pub(crate) fn add(&mut self, bytes: u64) {
        self.done = self.done.saturating_add(bytes);
        self.maybe_emit();
    }

    /// One more folder read (scanning).
    pub(crate) fn folder(&mut self) {
        self.folders = self.folders.saturating_add(1);
    }

    /// One more file found with `action`, without listing it among the
    /// recent files (scanning, where they would only scroll by).
    pub(crate) fn count(&mut self, action: FileAction) {
        self.tally.add(action);
    }

    /// The current file is finished with `action`: counted, shown among
    /// the recent files and listed.
    pub(crate) fn file_done(&mut self, action: FileAction, bytes: u64) {
        let file = FileDone::new(self.current.clone(), action, bytes);
        self.done_with(file);
    }

    /// Like [`Self::file_done`], with the details of `file` (whose path
    /// is normally the current file).
    pub(crate) fn done_with(&mut self, file: FileDone) {
        self.show(file.clone());
        self.list(file);
        self.maybe_emit();
    }

    /// The current file is finished with `action`: counted and shown
    /// among the recent files, but not listed (a full check's intact
    /// files, too many to be worth a line each).
    pub(crate) fn file_counted(&mut self, action: FileAction, bytes: u64) {
        self.show(FileDone::new(self.current.clone(), action, bytes));
        self.maybe_emit();
    }

    /// Lists a file that is not worked on but whose fate the run decides
    /// (deleted since the previous backup, an older version dropped):
    /// neither counted nor shown.
    pub(crate) fn listed(&mut self, file: FileDone) {
        self.list(file);
    }

    /// Reports a skipped file, after the files handled before it so the
    /// list keeps the order of the run.
    pub(crate) fn skipped(&mut self, skipped: Skipped) {
        self.send_files();
        self.sink.emit(RunEvent::Skipped(skipped));
    }

    /// A merge starts fusing the next group of backups.
    pub(crate) fn group(&mut self, group: MergeGroup) {
        self.group = Some(group);
        self.emit();
    }

    /// Sends the remaining files, then the final progress.
    pub(crate) fn finish(&mut self) {
        self.send_files();
        self.done = self.done.max(self.total);
        self.files_done = self.files_done.max(self.files_total);
        self.current.clear();
        self.emit();
    }

    fn show(&mut self, file: FileDone) {
        self.tally.add(file.action);
        if self.recent.len() == RECENT_FILES {
            self.recent.pop_front();
        }
        self.recent.push_back(file);
    }

    fn list(&mut self, file: FileDone) {
        self.files.push(file);
        if self.files.len() >= FILES_BATCH || self.files_sent.elapsed() >= FILES_INTERVAL {
            self.send_files();
        }
    }

    fn send_files(&mut self) {
        self.files_sent = Instant::now();
        if !self.files.is_empty() {
            let files = std::mem::replace(&mut self.files, Vec::with_capacity(FILES_BATCH));
            self.sink.emit(RunEvent::Files { files });
        }
    }

    fn maybe_emit(&mut self) {
        // A long file must not hold back the ones finished before it.
        if !self.files.is_empty() && self.files_sent.elapsed() >= FILES_INTERVAL {
            self.send_files();
        }
        if self.last.is_none_or(|t| t.elapsed() >= EMIT_INTERVAL) {
            self.emit();
        }
    }

    fn emit(&mut self) {
        self.last = Some(Instant::now());
        self.sink.emit(RunEvent::Progress {
            done: self.done,
            total: self.total,
            files_done: self.files_done,
            files_total: self.files_total,
            current: self.current.clone(),
            detail: PhaseDetail {
                folders: self.folders,
                tally: self.tally,
                recent: self.recent.drain(..).collect(),
                group: self.group.clone(),
            },
        });
    }
}

impl Drop for Progress<'_> {
    /// A run that fails or stops with an error still reports the files it
    /// handled before, ahead of the event that ends it.
    fn drop(&mut self) {
        self.send_files();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<RunEvent>>);

    impl EventSink for Recorder {
        fn emit(&self, event: RunEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    fn details(r: &Recorder) -> Vec<PhaseDetail> {
        r.0.lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                RunEvent::Progress { detail, .. } => Some(detail.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn recent_files_are_sent_once_and_capped_between_events() {
        let r = Recorder::default();
        let mut p = Progress::new(&r, 0, 0);
        for i in 0..(RECENT_FILES + 5) {
            p.start_file(format!("f{i}"));
            p.file_done(FileAction::New, 1);
        }
        p.finish();
        let sent: Vec<FileDone> = details(&r).into_iter().flat_map(|d| d.recent).collect();
        assert_eq!(sent.last().map(|f| f.path.as_str()), Some(format!("f{}", RECENT_FILES + 4).as_str()));
        let mut paths: Vec<&str> = sent.iter().map(|f| f.path.as_str()).collect();
        paths.dedup();
        assert_eq!(paths.len(), sent.len(), "no file is sent twice");
        let last = details(&r).pop().unwrap();
        assert_eq!(last.tally.get(FileAction::New), (RECENT_FILES + 5) as u64);
        assert!(last.recent.len() <= RECENT_FILES);
    }

    fn listed(r: &Recorder) -> Vec<FileDone> {
        r.0.lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                RunEvent::Files { files } => Some(files.clone()),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn every_file_is_listed_once_in_order_in_batches() {
        let r = Recorder::default();
        let n = FILES_BATCH * 2 + 7;
        {
            let mut p = Progress::new(&r, 0, 0);
            for i in 0..n {
                p.start_file(format!("f{i}"));
                p.file_done(FileAction::Changed, 1);
            }
            p.listed(FileDone::new("gone", FileAction::Deleted, 2));
            p.finish();
        }
        let files = listed(&r);
        assert_eq!(files.len(), n + 1);
        assert!(files.iter().take(n).enumerate().all(|(i, f)| f.path == format!("f{i}")));
        assert_eq!(files.last().map(|f| f.action), Some(FileAction::Deleted));
        let batches = r.0.lock().unwrap().iter().filter(|e| matches!(e, RunEvent::Files { .. })).count();
        assert!((3..=4).contains(&batches), "{batches} batches");
        let last = details(&r).pop().unwrap();
        assert_eq!((last.tally.changed, last.tally.deleted), (n as u64, 0), "listed only, not counted");
        let events = r.0.lock().unwrap();
        let files_at = events.iter().rposition(|e| matches!(e, RunEvent::Files { .. })).unwrap();
        assert!(matches!(events.get(files_at + 1), Some(RunEvent::Progress { .. })), "files before the end");
    }

    #[test]
    fn a_skipped_file_follows_the_files_before_it_and_a_dropped_progress_sends_the_rest() {
        let r = Recorder::default();
        {
            let mut p = Progress::new(&r, 0, 0);
            p.start_file("a");
            p.file_done(FileAction::New, 1);
            p.skipped(Skipped { path: "b".into(), reason: "locked".into() });
            p.start_file("c");
            p.file_counted(FileAction::Checked, 1);
            p.start_file("d");
            p.file_done(FileAction::New, 1);
        }
        let order: Vec<String> =
            r.0.lock()
                .unwrap()
                .iter()
                .filter_map(|e| match e {
                    RunEvent::Files { files } => {
                        Some(files.iter().map(|f| f.path.clone()).collect::<Vec<_>>().join(","))
                    }
                    RunEvent::Skipped(s) => Some(format!("skip {}", s.path)),
                    _ => None,
                })
                .collect();
        assert_eq!(order, ["a", "skip b", "d"], "c was only counted");
    }

    #[test]
    fn scanning_counts_folders_and_findings_without_listing_files() {
        let r = Recorder::default();
        let mut p = Progress::new(&r, 0, 0);
        p.folder();
        p.start_file("dir");
        p.count(FileAction::Unchanged);
        p.count(FileAction::Changed);
        p.finish();
        let last = details(&r).pop().unwrap();
        assert_eq!((last.folders, last.tally.unchanged, last.tally.changed), (1, 1, 1));
        assert!(details(&r).iter().all(|d| d.recent.is_empty()));
    }
}
