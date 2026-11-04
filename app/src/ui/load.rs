//! A query of the Backups tab run on a background thread: reading the
//! manifests never blocks the window.

use super::symbol;
use crate::i18n::Tr;
use archeion_engine::EngineError;
use eframe::egui::{Context, Ui};
use std::sync::mpsc;

/// Why a query failed, as the tab explains it.
#[derive(Debug)]
pub enum Problem {
    PasswordNeeded,
    WrongPassword,
    /// An imported profile's network folders are not confirmed yet.
    NetworkUnconfirmed,
    Other(String),
}

impl From<EngineError> for Problem {
    fn from(e: EngineError) -> Self {
        match e {
            EngineError::PasswordNeeded => Self::PasswordNeeded,
            EngineError::WrongPassword => Self::WrongPassword,
            EngineError::NetworkUnconfirmed => Self::NetworkUnconfirmed,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Result of a background query.
#[derive(Debug, Default)]
pub enum Load<T> {
    #[default]
    Idle,
    Loading(mpsc::Receiver<Result<T, Problem>>),
    Ready(T),
    Failed(Problem),
}

impl<T: Send + 'static> Load<T> {
    /// Runs `f` on its own thread; the window repaints when it is done.
    pub fn spawn(ctx: &Context, f: impl FnOnce() -> Result<T, Problem> + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new().name("restore-query".into()).spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => Self::Loading(rx),
            Err(e) => Self::Failed(Problem::Other(e.to_string())),
        }
    }

    /// Takes the result when it has arrived; `stopped` explains a thread
    /// that ended without one.
    pub fn poll(&mut self, stopped: &str) {
        if let Self::Loading(rx) = self {
            match rx.try_recv() {
                Ok(Ok(v)) => *self = Self::Ready(v),
                Ok(Err(e)) => *self = Self::Failed(e),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    *self = Self::Failed(Problem::Other(stopped.to_owned()));
                }
            }
        }
    }

    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(v) => Some(v),
            _ => None,
        }
    }

    /// Spinner or error for the states without a value.
    pub fn status_ui(&self, ui: &mut Ui, tr: &Tr, loading: &str) {
        match self {
            Self::Idle | Self::Loading(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(loading);
                });
            }
            Self::Failed(p) => {
                let text = match p {
                    Problem::PasswordNeeded => tr.t("run-password-needed"),
                    Problem::WrongPassword => tr.t("run-wrong-password"),
                    Problem::NetworkUnconfirmed => tr.t("run-network-unconfirmed"),
                    Problem::Other(e) => e.clone(),
                };
                ui.colored_label(ui.visuals().error_fg_color, symbol::with(symbol::WARNING, text));
            }
            Self::Ready(_) => {}
        }
    }
}
