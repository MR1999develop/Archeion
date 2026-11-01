//! A backup, merge or restore started from the window: a child
//! `archeion run|merge|restore <id>` process whose JSON lines are read on a
//! background thread and applied to an [`Activity`].
//!
//! Cancel kills the child; the engine works in a temporary folder that the
//! next run discards, so the archives are never left half-written.

use crate::{
    activity::{Activity, Op, Outcome},
    history,
};
use archeion_core::{ProfileId, RunEvent};
use std::{
    ffi::OsString,
    io::{BufRead as _, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Instant,
};

enum Msg {
    Event(RunEvent),
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
    child: Child,
    rx: mpsc::Receiver<Msg>,
}

impl Job {
    /// Starts the worker process for `profile`, with `extra` arguments; it
    /// writes its log to `log`.
    pub fn start(
        ctx: &eframe::egui::Context,
        profile: ProfileId,
        op: Op,
        extra: Vec<OsString>,
        log: PathBuf,
    ) -> std::io::Result<Self> {
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args([op.command(), &profile.to_string()])
            .args(extra)
            .arg("--log")
            .arg(&log)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_console(&mut cmd);
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().expect("stdout is piped");

        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new().name("job-output".into()).spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                match RunEvent::from_line(&line) {
                    Ok(ev) => {
                        if tx.send(Msg::Event(ev)).is_err() {
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

        Ok(Self { activity: Activity::new(profile, op, Instant::now()), log, child, rx })
    }

    pub fn profile(&self) -> ProfileId {
        self.activity.profile
    }

    /// Applies the events received so far; returns true once the run is
    /// over (the activity then holds the outcome). Call once per frame.
    pub fn poll(&mut self) -> bool {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Event(ev) => self.activity.apply(ev, Instant::now()),
                Msg::StreamClosed => {
                    let status = self.child.wait();
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

    pub fn cancel(&mut self) {
        if let Err(e) = self.child.kill() {
            tracing::warn!(error = %e, "could not stop the backup process");
        }
        if self.activity.outcome.is_none() {
            let line = history::Line::cancelled(history::now_ms());
            history::Writer::append_to(&self.log, &line);
            self.activity.log.push(line);
        }
        self.activity.set_outcome(Outcome::Cancelled);
    }
}

impl Drop for Job {
    /// Closing the window stops a run started from it.
    fn drop(&mut self) {
        if self.activity.outcome.is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
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
