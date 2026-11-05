//! A backup, merge or restore started from the window: a child
//! `archeion run|merge|restore <id>` process whose JSON lines are read on a
//! background thread and applied to an [`Activity`].
//!
//! Cancel closes the child's standard input: it stops at the next file, as
//! after an interruption (an unfinished backup resumes next time). A child
//! still running [`STOP_GRACE`] later is killed. The engine works in a
//! temporary folder, so the archives are never left half-written either
//! way. The child is in a [`ChildGroup`]: it ends with the window, even
//! when the window crashes.

use crate::{
    activity::{Activity, Op, Outcome},
    history,
};
use archeion_core::{ProfileId, RunEvent};
use archeion_win::ChildGroup;
use std::{
    ffi::OsString,
    io::{BufRead as _, BufReader},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

/// How long a child asked to stop may take before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(15);

enum Msg {
    /// Boxed: a progress event carries the recent files and is large.
    Event(Box<RunEvent>),
    StreamClosed,
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Msg")
    }
}

#[derive(Debug)]
pub struct Job {
    pub activity: Activity,
    /// The run's log file, written by the worker.
    log: PathBuf,
    /// Taken by `drop`, which waits for it on a thread.
    child: Option<Child>,
    /// Closed to ask the child to stop.
    stdin: Option<ChildStdin>,
    /// Ends the child with the window.
    group: Option<ChildGroup>,
    rx: mpsc::Receiver<Msg>,
}

impl Job {
    /// Starts the worker `exe` (this program, in the window) for `profile`,
    /// with `extra` arguments; it writes its log to `log`.
    pub fn start(
        ctx: &eframe::egui::Context,
        exe: &std::path::Path,
        profile: ProfileId,
        op: Op,
        extra: Vec<OsString>,
        log: PathBuf,
    ) -> std::io::Result<Self> {
        let mut cmd = Command::new(exe);
        cmd.args([op.command(), &profile.to_string()])
            .args(extra)
            .arg("--log")
            .arg(&log)
            .arg("--stop-on-eof")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_console(&mut cmd);
        let mut child = cmd.spawn()?;
        let group = ChildGroup::new();
        if let Some(g) = &group
            && let Err(e) = g.add(&child)
        {
            tracing::warn!(error = %e, "tying the backup process to the window");
        }
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout is piped");

        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new().name("job-output".into()).spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match RunEvent::from_line(&line) {
                    Ok(ev) => {
                        if tx.send(Msg::Event(Box::new(ev))).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                    }
                    Err(e) => tracing::warn!(error = %e, "unreadable worker line"),
                }
            }
            let _ = tx.send(Msg::StreamClosed);
            ctx.request_repaint();
        })?;

        let mut activity = Activity::new(profile, op, Instant::now());
        activity.log_file = Some(log.clone());
        Ok(Self { activity, log, child: Some(child), stdin, group, rx })
    }

    pub fn profile(&self) -> ProfileId {
        self.activity.profile
    }

    /// The run's log file.
    pub fn log_path(&self) -> &std::path::Path {
        &self.log
    }

    /// Applies the events received so far; returns true once the run is
    /// over (the activity then holds the outcome). Call once per frame.
    pub fn poll(&mut self) -> bool {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Event(ev) => self.activity.apply(*ev, Instant::now()),
                Msg::StreamClosed => {
                    let status = match self.child.as_mut() {
                        Some(c) => c.wait(),
                        None => Err(std::io::Error::other("no process")),
                    };
                    self.activity.set_outcome(Outcome::Failed(match status {
                        Ok(s) => format!("the backup process stopped unexpectedly ({s})"),
                        Err(e) => format!("the backup process stopped unexpectedly: {e}"),
                    }));
                    self.activity.end(Instant::now());
                    return true;
                }
            }
        }
        false
    }

    /// Asks the child to stop at the next file; see [`STOP_GRACE`].
    pub fn cancel(&mut self) {
        self.stdin = None;
        if self.activity.outcome.is_none() {
            let line = history::Line::cancelled(history::now_ms());
            history::Writer::append_to(&self.log, &line);
            self.activity.log.push(line);
        }
        self.activity.set_outcome(Outcome::Cancelled);
    }
}

impl Drop for Job {
    /// Closing the window stops a run started from it. The child is given
    /// [`STOP_GRACE`] to stop by itself, waited for on a thread of its own
    /// so the window never blocks; its group goes with it, so it still ends
    /// if the window ends first.
    fn drop(&mut self) {
        self.stdin = None;
        let Some(mut child) = self.child.take() else { return };
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        let group = self.group.take();
        let reaped = std::thread::Builder::new().name("job-stop".into()).spawn(move || {
            let deadline = Instant::now() + STOP_GRACE;
            while matches!(child.try_wait(), Ok(None)) {
                if Instant::now() >= deadline {
                    tracing::warn!("the backup process did not stop: ending it");
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            drop(group);
        });
        if let Err(e) = reaped {
            tracing::warn!(error = %e, "waiting for the backup process");
        }
    }
}

#[cfg(windows)]
fn hide_console(cmd: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_cmd: &mut Command) {}
