//! Activity tab: the operation in progress (or the last one) and the log of
//! every operation performed, like Areca's log tab. A list of runs on the
//! left; on the right the chosen run, step by step.

use super::{
    activity::{self as activity_tab, phase_text, problem_text, summary_text},
    theme,
};
use crate::{
    activity::{Activity, Op, Outcome, format_duration},
    history::{self, KEEP_LOGS, Line, RunLog},
    i18n::{Tr, arg},
};
use archeion_core::{ProfileId, RunEvent, Summary};
use eframe::egui::{self, Color32, RichText, Ui};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

/// Which run the right side shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Selection {
    /// The run in progress, or the last one started from this window.
    #[default]
    Current,
    Past(PathBuf),
}

/// Severity of a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Info,
    Done,
    Warning,
    Error,
}

#[derive(Debug, Default)]
pub struct HistoryView {
    runs: Vec<RunLog>,
    loading: Option<mpsc::Receiver<Vec<RunLog>>>,
    loaded: bool,
    selected: Selection,
    /// Only the selected profile's runs.
    only_profile: bool,
}

impl HistoryView {
    /// Reads the logs again (a run ended).
    pub fn invalidate(&mut self) {
        self.loaded = false;
    }

    /// Shows the run that just started.
    pub fn show_current(&mut self) {
        self.selected = Selection::Current;
        self.loaded = false;
    }

    /// Draws the tab. `current` is the run in progress or the last one;
    /// `name` gives a profile's current name.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        data_dir: &Path,
        current: Option<&Activity>,
        profile: Option<ProfileId>,
        name: &dyn Fn(ProfileId) -> String,
    ) -> activity_tab::Request {
        self.load(ui.ctx(), data_dir);
        if self.selected == Selection::Current
            && current.is_none()
            && let Some(first) = self.runs.first()
        {
            self.selected = Selection::Past(first.path.clone());
        }
        let mut req = activity_tab::Request::default();
        egui::Panel::left("history-list")
            .resizable(true)
            .default_size(theme::SIDE_PANEL_WIDTH + 80.0)
            .show(ui, |ui| self.list_ui(ui, tr, current, profile, name));
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("history-detail").auto_shrink(false).show(ui, |ui| {
                match (&self.selected, current) {
                    (Selection::Current, Some(a)) => {
                        req = activity_tab::show(ui, tr, a, &name(a.profile), Instant::now());
                        ui.add_space(theme::SPACE_M);
                        ui.strong(tr.t("log-title"));
                        lines_ui(ui, tr, a.op, &a.log);
                    }
                    (Selection::Past(path), _) => match self.runs.iter().find(|r| &r.path == path) {
                        Some(run) => run_ui(ui, tr, run),
                        None => {
                            ui.weak(tr.t("log-pick"));
                        }
                    },
                    (Selection::Current, None) => idle_ui(ui, tr),
                }
            });
        });
        req
    }

    fn load(&mut self, ctx: &egui::Context, data_dir: &Path) {
        if let Some(rx) = &self.loading {
            match rx.try_recv() {
                Ok(runs) => {
                    self.runs = runs;
                    self.loading = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.loading = None,
            }
        }
        if self.loaded || self.loading.is_some() {
            return;
        }
        self.loaded = true;
        let (tx, rx) = mpsc::channel();
        let (dir, ctx) = (data_dir.to_path_buf(), ctx.clone());
        let spawned = std::thread::Builder::new().name("history".into()).spawn(move || {
            history::prune(&dir, KEEP_LOGS);
            let _ = tx.send(history::list(&dir, KEEP_LOGS));
            ctx.request_repaint();
        });
        if spawned.is_ok() {
            self.loading = Some(rx);
        }
    }

    fn list_ui(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        current: Option<&Activity>,
        profile: Option<ProfileId>,
        name: &dyn Fn(ProfileId) -> String,
    ) {
        ui.add_space(theme::SPACE_S);
        ui.horizontal(|ui| {
            ui.strong(tr.t("log-runs"));
            if ui.small_button("⟳").on_hover_text(tr.t("refresh")).clicked() {
                self.invalidate();
            }
        });
        ui.checkbox(&mut self.only_profile, tr.t("log-only-profile"));
        ui.add_space(theme::SPACE_S);

        egui::ScrollArea::vertical().id_salt("history-runs").auto_shrink(false).show(ui, |ui| {
            if let Some(a) = current {
                let label = if a.is_running() { tr.t("log-running") } else { tr.t("log-last") };
                let text = format!("{}  {}", if a.is_running() { "⟳" } else { "●" }, label);
                let on = self.selected == Selection::Current;
                if ui.selectable_label(on, RichText::new(text).strong()).clicked() {
                    self.selected = Selection::Current;
                }
                ui.weak(format!("{} · {}", op_title(tr, a.op, None, current_summary(a)), name(a.profile)));
                ui.separator();
            }
            if self.loading.is_some() && self.runs.is_empty() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr.t("reading-backups"));
                });
            }
            let mut shown = 0;
            for run in &self.runs {
                if self.only_profile && Some(run.header.profile) != profile {
                    continue;
                }
                shown += 1;
                let (level, _) = end_text(tr, run);
                let when = history::local_time(run.header.started_ms);
                let title = op_title(tr, run.header.op, run.header.detail.as_deref(), run_summary(run));
                let text =
                    RichText::new(format!("{} {}  {title}", icon(level), when.get(..16).unwrap_or(&when)))
                        .color(level_color(ui, level));
                let on = self.selected == Selection::Past(run.path.clone());
                let profile_name = if run.header.profile_name.is_empty() {
                    name(run.header.profile)
                } else {
                    run.header.profile_name.clone()
                };
                if ui.selectable_label(on, text).on_hover_text(&profile_name).clicked() {
                    self.selected = Selection::Past(run.path.clone());
                }
                ui.weak(format!("    {profile_name}"));
            }
            if shown == 0 && self.loading.is_none() {
                ui.weak(tr.t("log-empty"));
            }
        });
    }
}

fn idle_ui(ui: &mut Ui, tr: &Tr) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() / 3.0);
        ui.heading(tr.t("activity-idle-title"));
        ui.label(tr.t("activity-idle-body"));
    });
}

/// A past run: what, when, how it ended, then every logged step.
fn run_ui(ui: &mut Ui, tr: &Tr, run: &RunLog) {
    let h = &run.header;
    ui.add_space(theme::SPACE_S);
    let title = op_title(tr, h.op, h.detail.as_deref(), run_summary(run));
    ui.heading(format!("{title} · {}", h.profile_name));
    let how = if h.scheduled { tr.t("log-scheduled") } else { tr.t("log-manual") };
    let mut facts = vec![tr.t_args("log-started", &[("time", arg(history::local_time(h.started_ms)))]), how];
    if let Some(end) = run.end() {
        let ms = u64::try_from(end.at_ms.saturating_sub(h.started_ms)).unwrap_or(0);
        facts.push(tr.t_args("activity-took", &[("time", arg(format_duration(Duration::from_millis(ms))))]));
    }
    if let Some(d) = &h.detail
        && matches!(h.op, Op::Delete | Op::Merge | Op::Restore)
    {
        facts.push(tr.t_args("log-backup", &[("name", arg(d.clone()))]));
    }
    ui.weak(facts.join(" · "));
    ui.add_space(theme::SPACE_S);
    let (level, text) = end_text(tr, run);
    ui.label(RichText::new(format!("{} {text}", icon(level))).strong().color(level_color(ui, level)));
    let skipped = run.skipped();
    if skipped > 0 {
        ui.colored_label(ui.visuals().warn_fg_color, tr.t_args("skipped-count", &[("count", arg(skipped))]));
    }
    if let Some(Summary::Verify(r)) = run_summary(run) {
        activity_tab::problems_ui(ui, tr, &r.problems);
    }
    ui.add_space(theme::SPACE_M);
    ui.strong(tr.t("log-title"));
    lines_ui(ui, tr, h.op, &run.lines);
}

/// The steps of a run, one row each.
fn lines_ui(ui: &mut Ui, tr: &Tr, op: Op, lines: &[Line]) {
    if lines.is_empty() {
        ui.weak(tr.t("log-no-lines"));
        return;
    }
    egui::Grid::new("log-lines").num_columns(3).striped(true).spacing([theme::SPACE_S, 2.0]).show(ui, |ui| {
        for line in lines {
            let (level, text) = line_text(tr, op, line);
            let time = history::local_time(line.at_ms);
            ui.weak(time.get(11..).unwrap_or(&time));
            ui.label(RichText::new(icon(level)).color(level_color(ui, level)));
            let label = egui::Label::new(RichText::new(text).color(level_color(ui, level))).wrap();
            ui.add(label);
            ui.end_row();
        }
    });
}

/// Message and severity of one log line.
fn line_text(tr: &Tr, op: Op, line: &Line) -> (Level, String) {
    if line.cancelled {
        return (Level::Warning, tr.t("run-cancelled"));
    }
    let Some(ev) = &line.event else { return (Level::Info, String::new()) };
    match ev {
        RunEvent::Phase { phase } => (Level::Info, phase_text(tr, *phase)),
        RunEvent::Progress { .. } => (Level::Info, String::new()),
        RunEvent::Skipped(s) => (
            Level::Warning,
            tr.t_args("log-skipped", &[("path", arg(s.path.clone())), ("reason", arg(s.reason.clone()))]),
        ),
        RunEvent::Finished { summary } => {
            let (text, warn) = summary_text(tr, op, summary);
            (if warn { Level::Warning } else { Level::Done }, text)
        }
        other => (Level::Error, problem_text(tr, &event_outcome(other)).unwrap_or_default()),
    }
}

/// How a run ended, from its log.
fn end_text(tr: &Tr, run: &RunLog) -> (Level, String) {
    match run.end() {
        Some(line) => line_text(tr, run.header.op, line),
        None => (Level::Warning, tr.t("log-unfinished")),
    }
}

fn event_outcome(ev: &RunEvent) -> Outcome {
    match ev {
        RunEvent::DestinationUnavailable { location } => Outcome::DestinationUnavailable(location.clone()),
        RunEvent::Busy => Outcome::Busy,
        RunEvent::PasswordNeeded => Outcome::PasswordNeeded,
        RunEvent::WrongPassword => Outcome::WrongPassword,
        RunEvent::NotEnoughSpace { needed, available } => {
            Outcome::NotEnoughSpace { needed: *needed, available: *available }
        }
        RunEvent::Failed { message } => Outcome::Failed(message.clone()),
        RunEvent::Finished { summary } => Outcome::Finished(summary.clone()),
        RunEvent::Phase { .. } | RunEvent::Progress { .. } | RunEvent::Skipped(_) => {
            Outcome::Failed(String::new())
        }
    }
}

fn run_summary(run: &RunLog) -> Option<&Summary> {
    run.end().and_then(|l| match &l.event {
        Some(RunEvent::Finished { summary }) => Some(summary),
        _ => None,
    })
}

fn current_summary(a: &Activity) -> Option<&Summary> {
    match &a.outcome {
        Some(Outcome::Finished(s)) => Some(s),
        _ => None,
    }
}

/// "Complete backup", "Merge", …; a backup's kind comes from what was asked
/// or, for automatic backups, from what was done.
fn op_title(tr: &Tr, op: Op, detail: Option<&str>, summary: Option<&Summary>) -> String {
    let complete = match (detail, summary) {
        (Some("complete"), _) => Some(true),
        (Some("incremental"), _) => Some(false),
        (_, Some(Summary::Backup(r))) => Some(r.complete),
        _ => None,
    };
    tr.t(match (op, complete) {
        (Op::Backup, Some(true)) => "log-op-complete",
        (Op::Backup, Some(false)) => "log-op-incremental",
        (Op::Backup, None) => "log-op-backup",
        (Op::Merge, _) => "log-op-merge",
        (Op::Delete, _) => "log-op-delete",
        (Op::Restore, _) => "log-op-restore",
        (Op::Verify, _) if detail == Some(history::CHECK_SAMPLE) => "log-op-restore-test",
        (Op::Verify, _) => "log-op-verify",
    })
}

fn icon(level: Level) -> &'static str {
    match level {
        Level::Info => "•",
        Level::Done => "✔",
        Level::Warning => "⚠",
        Level::Error => "✖",
    }
}

fn level_color(ui: &Ui, level: Level) -> Color32 {
    match level {
        Level::Info | Level::Done => ui.visuals().text_color(),
        Level::Warning => ui.visuals().warn_fg_color,
        Level::Error => ui.visuals().error_fg_color,
    }
}
