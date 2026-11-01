//! Throttled progress of one phase: bytes, files and the current file.

use crate::EventSink;
use archeion_core::RunEvent;
use std::time::{Duration, Instant};

/// Minimum time between two progress events, so the stdout pipe and the
/// window are not flooded.
const EMIT_INTERVAL: Duration = Duration::from_millis(200);

pub(crate) struct Progress<'a> {
    sink: &'a dyn EventSink,
    done: u64,
    total: u64,
    files_done: u64,
    files_total: u64,
    current: String,
    last: Option<Instant>,
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
            last: None,
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

    pub(crate) fn finish(&mut self) {
        self.done = self.done.max(self.total);
        self.files_done = self.files_done.max(self.files_total);
        self.current.clear();
        self.emit();
    }

    fn maybe_emit(&mut self) {
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
        });
    }
}
