//! Activity tab: the operation in progress and the log of every operation
//! performed, like Areca's log tab. A list of runs on the left; on the
//! right the chosen run, step by step. A run that ends simply becomes the
//! newest entry of the list.
//!
//! Above the steps of a run, buttons give its full log as plain text (in
//! the default text editor, or copied) and open the program's diagnostic
//! log or the logs folder. The full log lists every file the run handled,
//! so it can be large: it is prepared on a background thread.

use super::{
    activity::{self as activity_tab, SKIPPED_KEY, action_key, phase_text, problem_text, summary_text},
    shell, theme,
};
use crate::{
    activity::{Activity, Op, Outcome, format_duration},
    history::{self, FileEntry, FileRecord, KEEP_LOGS, Line, RunLog},
    i18n::{Tr, arg},
    storage,
};
use archeion_core::{FileAction, ProfileId, RunEvent, Summary, Tally};
use eframe::egui::{self, Color32, RichText, Ui};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

/// How long the confirmation of a log button stays (errors stay).
const NOTICE_FOR: Duration = Duration::from_secs(5);
/// A full log taking longer than this to prepare shows a spinner.
const SPINNER_DELAY: Duration = Duration::from_secs(1);
/// Folder, inside the system's temporary folder, of the text logs.
const REPORT_DIR: &str = "Archeion";

/// Which run the right side shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Selection {
    /// The run in progress.
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

/// A button above the steps of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogAction {
    /// The run's full log as a text file, opened in the default editor.
    OpenReport,
    /// The same text, to the clipboard.
    CopyReport,
    /// The program's diagnostic log (`tracing`).
    OpenProgramLog,
    OpenLogsFolder,
}

/// A full log being prepared on a background thread.
#[derive(Debug)]
struct Preparing {
    /// Log file of the run it is about.
    log: PathBuf,
    started: Instant,
    rx: mpsc::Receiver<Prepared>,
}

/// What the background thread made of a full log.
#[derive(Debug)]
enum Prepared {
    /// The text, for the clipboard.
    Text(String),
    /// Written to this file, to open.
    Written(PathBuf),
    /// What to tell the user.
    Failed(String),
}

/// What the last log button did, shown under the buttons of its run.
#[derive(Debug)]
struct Notice {
    /// Log file of the run it is about.
    log: PathBuf,
    error: bool,
    text: String,
    at: Instant,
}

#[derive(Debug, Default)]
pub struct HistoryView {
    runs: Vec<RunLog>,
    loading: Option<mpsc::Receiver<Vec<RunLog>>>,
    loaded: bool,
    selected: Selection,
    /// Only the selected profile's runs.
    only_profile: bool,
    notice: Option<Notice>,
    preparing: Option<Preparing>,
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

    /// A run ended; its log is at `log`. If it was on show, its log
    /// takes its place.
    pub fn run_ended(&mut self, log: PathBuf) {
        if self.selected == Selection::Current {
            self.selected = Selection::Past(log);
        }
        self.loaded = false;
    }

    /// Draws the tab. `running` is the run in progress; `name` gives a
    /// profile's current name. Returns true when Cancel was clicked.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tr: &Tr,
        data_dir: &Path,
        running: Option<&Activity>,
        profile: Option<ProfileId>,
        name: &dyn Fn(ProfileId) -> String,
    ) -> bool {
        self.load(ui.ctx(), data_dir);
        self.poll_report(ui.ctx(), tr);
        if self.selected == Selection::Current
            && running.is_none()
            && let Some(first) = self.runs.first()
        {
            self.selected = Selection::Past(first.path.clone());
        }
        let mut cancel = false;
        // A log button clicked, and the log file of its run.
        let mut clicked: Option<(LogAction, PathBuf)> = None;
        egui::Panel::left("history-list")
            .resizable(true)
            .default_size(theme::SIDE_PANEL_WIDTH + 80.0)
            .show(ui, |ui| self.list_ui(ui, tr, running, profile, name));
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("history-detail").auto_shrink(false).show(ui, |ui| {
                match (&self.selected, running) {
                    (Selection::Current, Some(a)) => {
                        cancel = activity_tab::show(ui, tr, a, &name(a.profile), Instant::now());
                        ui.add_space(theme::SPACE_M);
                        match &a.log_file {
                            Some(log) => {
                                let header =
                                    log_header_ui(ui, tr, log, self.notice.as_ref(), self.preparing.as_ref());
                                if let Some(action) = header {
                                    clicked = Some((action, log.clone()));
                                }
                            }
                            None => {
                                ui.strong(tr.t("log-title"));
                            }
                        }
                        lines_ui(ui, tr, a.op, &a.log);
                    }
                    (Selection::Past(path), _) => match self.runs.iter().find(|r| &r.path == path) {
                        Some(run) => {
                            let shown = run_ui(ui, tr, run, self.notice.as_ref(), self.preparing.as_ref());
                            if let Some(action) = shown {
                                clicked = Some((action, run.path.clone()));
                            }
                        }
                        None if self.loading.is_some() => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(tr.t("reading-backups"));
                            });
                        }
                        None => {
                            ui.weak(tr.t("log-pick"));
                        }
                    },
                    (Selection::Current, None) => idle_ui(ui, tr),
                }
            });
        });
        if let Some((action, log)) = clicked {
            self.notice = match action {
                LogAction::OpenReport | LogAction::CopyReport => {
                    self.prepare_report(ui.ctx(), tr, action, &log)
                }
                LogAction::OpenProgramLog | LogAction::OpenLogsFolder => act(tr, data_dir, action, &log),
            };
        }
        cancel
    }

    /// Starts preparing the full log of the run whose log file is `log`,
    /// for `action`; returns what to tell the user if it cannot start.
    fn prepare_report(
        &mut self,
        ctx: &egui::Context,
        tr: &Tr,
        action: LogAction,
        log: &Path,
    ) -> Option<Notice> {
        let (tx, rx) = mpsc::channel();
        let (thread_tr, thread_log, ctx) = (tr.clone(), log.to_path_buf(), ctx.clone());
        let spawned = std::thread::Builder::new().name("full-log".into()).spawn(move || {
            let prepared = match build_report(&thread_tr, &thread_log) {
                Ok(text) if action == LogAction::CopyReport => Prepared::Text(text),
                Ok(text) => {
                    let path = report_path(&std::env::temp_dir(), &thread_log);
                    match write_report(&path, &text) {
                        Ok(()) => Prepared::Written(path),
                        Err(e) => {
                            tracing::warn!(error = %e, path = %path.display(), "writing the text log");
                            Prepared::Failed(report_failed(&thread_tr, &e))
                        }
                    }
                }
                Err(text) => Prepared::Failed(text),
            };
            let _ = tx.send(prepared);
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => {
                self.preparing = Some(Preparing { log: log.to_path_buf(), started: Instant::now(), rx });
                None
            }
            Err(e) => Some(Notice {
                log: log.to_path_buf(),
                error: true,
                text: report_failed(tr, &e),
                at: Instant::now(),
            }),
        }
    }

    /// Takes the full log once prepared: copies or opens it.
    fn poll_report(&mut self, ctx: &egui::Context, tr: &Tr) {
        let Some(p) = &self.preparing else { return };
        let prepared = match p.rx.try_recv() {
            Ok(prepared) => prepared,
            Err(mpsc::TryRecvError::Empty) => return,
            // The thread stopped without an answer: it panicked.
            Err(mpsc::TryRecvError::Disconnected) => Prepared::Failed(tr.t("log-unreadable")),
        };
        let log = p.log.clone();
        self.preparing = None;
        let notice =
            |error: bool, text: String| Some(Notice { log: log.clone(), error, text, at: Instant::now() });
        self.notice = match prepared {
            Prepared::Text(text) => {
                ctx.copy_text(text);
                notice(false, tr.t("log-copied"))
            }
            Prepared::Written(path) => {
                shell::open(path);
                None
            }
            Prepared::Failed(text) => notice(true, text),
        };
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
        running: Option<&Activity>,
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
            if let Some(a) = running {
                let text = format!("⟳  {}", tr.t("log-running"));
                let on = self.selected == Selection::Current;
                if ui.selectable_label(on, RichText::new(text).strong()).clicked() {
                    self.selected = Selection::Current;
                }
                ui.weak(format!("    {} · {}", op_title(tr, a.op, None, None), name(a.profile)));
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

/// A past run: what, when, how it ended, then every logged step. Returns
/// the log button clicked, if any.
fn run_ui(
    ui: &mut Ui,
    tr: &Tr,
    run: &RunLog,
    notice: Option<&Notice>,
    preparing: Option<&Preparing>,
) -> Option<LogAction> {
    let h = &run.header;
    ui.add_space(theme::SPACE_S);
    ui.heading(run_title(tr, run));
    ui.weak(facts(tr, run).join(" · "));
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
    let action = log_header_ui(ui, tr, &run.path, notice, preparing);
    // The line that ended the run is the outcome shown above.
    let steps = match run.lines.iter().rposition(Line::is_terminal) {
        Some(end) => run.lines.get(..end).unwrap_or_default(),
        None => &run.lines,
    };
    lines_ui(ui, tr, h.op, steps);
    action
}

/// "Complete backup · Documents".
fn run_title(tr: &Tr, run: &RunLog) -> String {
    let h = &run.header;
    let title = op_title(tr, h.op, h.detail.as_deref(), run_summary(run));
    format!("{title} · {}", h.profile_name)
}

/// When it started, how, how long it took, which backup it was about.
fn facts(tr: &Tr, run: &RunLog) -> Vec<String> {
    let h = &run.header;
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
    facts
}

/// The "Log" title with the log buttons of the run whose log file is
/// `log`, and what the last button did for it, or that its full log is
/// being prepared. The full-log buttons wait while any full log is
/// prepared. Returns the button clicked, if any.
fn log_header_ui(
    ui: &mut Ui,
    tr: &Tr,
    log: &Path,
    notice: Option<&Notice>,
    preparing: Option<&Preparing>,
) -> Option<LogAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.strong(tr.t("log-title"));
        ui.add_space(theme::SPACE_S);
        let buttons = [
            (LogAction::OpenReport, "log-open-full", "log-open-full-hint"),
            (LogAction::CopyReport, "log-copy", "log-copy-hint"),
        ];
        for (a, label, hint) in buttons {
            let button = ui
                .add_enabled(preparing.is_none(), egui::Button::new(tr.t(label)))
                .on_hover_text(tr.t(hint))
                .on_disabled_hover_text(tr.t("log-preparing"));
            if button.clicked() {
                action = Some(a);
            }
        }
        ui.add_space(theme::SPACE_M);
        let links = [
            (LogAction::OpenProgramLog, "log-open-program", "log-open-program-hint"),
            (LogAction::OpenLogsFolder, "log-open-folder", "log-open-folder-hint"),
        ];
        for (a, label, hint) in links {
            if ui.link(tr.t(label)).on_hover_text(tr.t(hint)).clicked() {
                action = Some(a);
            }
        }
    });
    if let Some(p) = preparing.filter(|p| p.log.as_path() == log) {
        let waited = p.started.elapsed();
        if waited >= SPINNER_DELAY {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr.t("log-preparing"));
            });
        } else {
            ui.ctx().request_repaint_after(SPINNER_DELAY.saturating_sub(waited));
        }
    } else if let Some(n) = notice.filter(|n| n.log.as_path() == log) {
        let left = NOTICE_FOR.saturating_sub(n.at.elapsed());
        if n.error {
            ui.colored_label(ui.visuals().error_fg_color, format!("{} {}", icon(Level::Error), n.text));
        } else if !left.is_zero() {
            ui.label(format!("{} {}", icon(Level::Done), n.text));
            ui.ctx().request_repaint_after(left);
        }
    }
    action
}

/// The full log of the run whose log file is `log`, read again rather
/// than taken from the list (a running run has its newest lines only on
/// disk), with its list of files; or what to tell the user.
fn build_report(tr: &Tr, log: &Path) -> Result<String, String> {
    let text = fs::read_to_string(log).map_err(|e| report_failed(tr, &e))?;
    let run = RunLog::parse(log.to_path_buf(), &text).ok_or_else(|| tr.t("log-unreadable"))?;
    let files = history::read_files(log).map_err(|e| report_failed(tr, &e))?;
    Ok(report(tr, &run, files.as_deref()))
}

fn report_failed(tr: &Tr, e: &std::io::Error) -> String {
    tr.t_args("log-report-failed", &[("detail", arg(e.to_string()))])
}

/// Carries out a button that opens the program log or the logs folder;
/// returns what to tell the user about the run whose log file is `log`.
fn act(tr: &Tr, data_dir: &Path, action: LogAction, log: &Path) -> Option<Notice> {
    let notice =
        |error: bool, text: String| Some(Notice { log: log.to_path_buf(), error, text, at: Instant::now() });
    match action {
        // Prepared on a thread by `HistoryView::prepare_report`.
        LogAction::OpenReport | LogAction::CopyReport => None,
        LogAction::OpenProgramLog => {
            let path = storage::program_log_path(data_dir);
            if !path.is_file() {
                return notice(true, tr.t("log-program-missing"));
            }
            shell::open(path);
            None
        }
        LogAction::OpenLogsFolder => {
            let path = data_dir.join(history::LOG_DIR);
            if !path.is_dir() {
                return notice(true, tr.t("about-folder-missing"));
            }
            shell::open(path);
            None
        }
    }
}

/// Where the text log of the run log `log` goes: `<temp>\Archeion\<same
/// name>.txt`. A view rewritten at every click in the window's language,
/// not a second copy of the log: so it stays out of the logs folder (whose
/// `.jsonl` files are the record) and needs no pruning of its own.
fn report_path(temp: &Path, log: &Path) -> PathBuf {
    let stem = log.file_stem().map_or_else(|| "run".into(), |s| s.to_string_lossy().into_owned());
    temp.join(REPORT_DIR).join(format!("{stem}.txt"))
}

/// Writes `text` with a byte-order mark and CRLF line ends, so that every
/// Windows editor, old Notepad included, shows it right.
fn write_report(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, format!("\u{feff}{}", text.replace('\n', "\r\n")))
}

/// The whole log of a run as plain text, in the window's language: what,
/// when and how it ended (as at the top of the tab), the problems a check
/// found, every logged line (every skipped file included) with its date,
/// time and severity, then the files the run handled (`files`, from the
/// companion file; `None` for runs of older versions, which have none).
fn report(tr: &Tr, run: &RunLog, files: Option<&[FileRecord]>) -> String {
    let mut out = vec![format!("Archeion · {}", run_title(tr, run)), facts(tr, run).join(" · ")];
    let (level, text) = end_text(tr, run);
    out.push(format!("{} {text}", icon(level)));
    let skipped = run.skipped();
    if skipped > 0 {
        let count = tr.t_args("skipped-count", &[("count", arg(skipped))]);
        out.push(format!("{} {count}", icon(Level::Warning)));
    }
    if let Some(Summary::Verify(r)) = run_summary(run)
        && !r.problems.is_empty()
    {
        out.push(String::new());
        out.push(format!("{} {}", icon(Level::Error), tr.t("verify-problems")));
        out.extend(r.problems.iter().map(|p| format!("  {p}")));
        out.push(tr.t("verify-problems-hint"));
    }
    out.push(String::new());
    out.push(tr.t("log-title"));
    let words = [Level::Info, Level::Done, Level::Warning, Level::Error].map(|l| level_word(tr, l));
    let width = words.iter().map(|w| w.chars().count()).max().unwrap_or(0);
    let mut logged = 0;
    for line in &run.lines {
        let (level, text) = line_text(tr, run.header.op, line);
        if text.is_empty() {
            continue;
        }
        logged += 1;
        let head = format!("{}  {:<width$}  ", history::local_time(line.at_ms), level_word(tr, level));
        // A message on more lines goes on under its first line.
        let sep = format!("\n{}", " ".repeat(head.chars().count()));
        out.push(format!("{head}{}", text.lines().collect::<Vec<_>>().join(sep.as_str())));
    }
    if logged == 0 {
        out.push(tr.t("log-no-lines"));
    }
    out.push(String::new());
    out.push(tr.t("log-files-title"));
    match files {
        Some(files) => files_lines(tr, run, files, &mut out),
        None => out.push(tr.t("log-files-missing")),
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// The files section of the full log: how many files had each fate, then
/// one line per file in the order the run handled them (a log is read in
/// time order; the counts give the overview, and the editor's search finds
/// every file with one word). Each line: time, what happened, size, full
/// path and anything more (bytes of new chunks, kept from an interrupted
/// run, why it was skipped or found damaged).
fn files_lines(tr: &Tr, run: &RunLog, files: &[FileRecord], out: &mut Vec<String>) {
    let counts = file_counts(tr, run, files);
    if !counts.is_empty() {
        out.push(counts.join(" · "));
    }
    if files.is_empty() {
        out.push(tr.t("log-files-none"));
        return;
    }
    // Words looked up once per action, not once per file.
    let mut words: HashMap<Option<FileAction>, String> = HashMap::new();
    let mut rows: Vec<FileRow<'_>> = Vec::with_capacity(files.len());
    for r in files {
        let (action, size, path, extra) = match &r.entry {
            FileEntry::Done(f) => {
                let mut extra = Vec::new();
                if let Some(n) = f.new_chunk_bytes {
                    extra.push(tr.t_args("log-file-new-chunks", &[("size", arg(tr.bytes(n)))]));
                }
                if f.resumed {
                    extra.push(tr.t("log-file-resumed"));
                }
                if !f.note.is_empty() {
                    extra.push(f.note.clone());
                }
                (Some(f.action), tr.bytes(f.bytes), f.path.as_str(), extra.join("; "))
            }
            FileEntry::Skipped(s) => (None, String::new(), s.path.as_str(), s.reason.clone()),
        };
        words.entry(action).or_insert_with(|| tr.t(action.map_or(SKIPPED_KEY, action_key)));
        rows.push(FileRow { at_ms: r.at_ms, action, size, path, extra });
    }
    let word_width = words.values().map(|w| w.chars().count()).max().unwrap_or(0);
    let size_width = rows.iter().map(|r| r.size.chars().count()).max().unwrap_or(0);
    // Files of one batch share their time.
    let mut time: (i64, String) = (i64::MIN, String::new());
    for FileRow { at_ms, action, size, path, extra } in rows {
        if time.0 != at_ms {
            time = (at_ms, history::local_time(at_ms));
        }
        let word = words.get(&action).map_or("", String::as_str);
        let mut line = format!("{}  {word:<word_width$}  {size:>size_width$}  {path}", time.1);
        if !extra.is_empty() {
            line.push_str("  (");
            line.push_str(&extra);
            line.push(')');
        }
        out.push(line);
    }
}

/// One line of the files section, before the columns are aligned.
struct FileRow<'a> {
    at_ms: i64,
    /// `None` for a skipped file.
    action: Option<FileAction>,
    size: String,
    path: &'a str,
    extra: String,
}

/// "New: 3 · Changed: 1 · Unchanged, not listed: 1204 · …": the files
/// listed per fate, and those only counted (the unchanged files of an
/// incremental backup, the intact files of a full check).
fn file_counts(tr: &Tr, run: &RunLog, files: &[FileRecord]) -> Vec<String> {
    let mut tally = Tally::default();
    let (mut skipped, mut as_changes) = (0u64, 0u64);
    for r in files {
        match &r.entry {
            FileEntry::Done(f) => {
                tally.add(f.action);
                as_changes += u64::from(f.new_chunk_bytes.is_some());
            }
            FileEntry::Skipped(_) => skipped += 1,
        }
    }
    let unchanged = match run_summary(run) {
        Some(Summary::Backup(r)) if !r.complete => r.files_unchanged,
        _ => 0,
    };
    // A full check lists only its problems: the intact files come from
    // its summary.
    let (checked_key, checked) = match run_summary(run) {
        Some(Summary::Verify(r)) if run.header.detail.as_deref() != Some(history::CHECK_SAMPLE) => {
            ("log-count-checked-unlisted", r.files_checked.saturating_sub(tally.damaged))
        }
        _ => ("log-count-checked", tally.checked),
    };
    [
        ("log-count-new", tally.new),
        ("log-count-changed", tally.changed),
        ("log-count-as-changes", as_changes),
        ("log-count-copied-again", tally.unchanged),
        ("log-count-unchanged", unchanged),
        ("log-count-deleted", tally.deleted),
        ("log-count-moved", tally.moved),
        ("log-count-dropped", tally.dropped),
        ("log-count-restored", tally.restored),
        ("log-count-kept", tally.kept),
        (checked_key, checked),
        ("log-count-damaged", tally.damaged),
        ("log-count-skipped", skipped),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(key, n)| tr.t_args(key, &[("count", arg(n))]))
    .collect()
}

fn level_word(tr: &Tr, level: Level) -> String {
    tr.t(match level {
        Level::Info => "log-level-info",
        Level::Done => "log-level-done",
        Level::Warning => "log-level-warning",
        Level::Error => "log-level-error",
    })
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
        RunEvent::Progress { .. } | RunEvent::Files { .. } => (Level::Info, String::new()),
        RunEvent::Skipped(s) => (
            Level::Warning,
            tr.t_args("log-skipped", &[("path", arg(s.path.clone())), ("reason", arg(s.reason.clone()))]),
        ),
        RunEvent::Finished { summary } => {
            let (text, warn) = summary_text(tr, op, summary);
            (if warn { Level::Warning } else { Level::Done }, text)
        }
        other => (Level::Error, problem_text(tr, op, &event_outcome(other)).unwrap_or_default()),
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
        RunEvent::Phase { .. }
        | RunEvent::Progress { .. }
        | RunEvent::Skipped(_)
        | RunEvent::Files { .. } => Outcome::Failed(String::new()),
    }
}

fn run_summary(run: &RunLog) -> Option<&Summary> {
    run.end().and_then(|l| match &l.event {
        Some(RunEvent::Finished { summary }) => Some(summary),
        _ => None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{BackupReport, FileDone, Language, Phase, Skipped, VerifyReport};
    use history::Header;

    const T0: i64 = 1_793_000_000_000;

    fn run(op: Op, detail: Option<&str>, lines: Vec<Line>) -> RunLog {
        RunLog {
            path: "20261030-123105-123-backup-6f1c3d1e.jsonl".into(),
            header: Header {
                profile: "6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(),
                profile_name: "Documents".into(),
                op,
                detail: detail.map(Into::into),
                scheduled: true,
                started_ms: T0,
            },
            lines,
        }
    }

    fn skipped(at_ms: i64, path: &str, reason: &str) -> Line {
        Line::event(at_ms, RunEvent::Skipped(Skipped { path: path.into(), reason: reason.into() }))
    }

    #[test]
    fn the_text_log_has_the_facts_then_every_line_with_time_and_level() {
        let tr = Tr::new(Language::English);
        let r = run(
            Op::Backup,
            Some("complete"),
            vec![
                Line::event(T0 + 1_000, RunEvent::Phase { phase: Phase::Copying }),
                skipped(T0 + 2_000, r"C:\a.pst", "in use"),
                skipped(T0 + 3_000, r"C:\b.ost", "locked"),
                Line::event(
                    T0 + 60_000,
                    RunEvent::Finished { summary: Summary::Backup(BackupReport::default()) },
                ),
            ],
        );
        let text = report(&tr, &r, None);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.first(), Some(&"Archeion · Complete backup · Documents"));
        let facts = lines.get(1).copied().unwrap_or_default();
        assert!(facts.starts_with(&format!("Started {}", history::local_time(T0))), "{facts}");
        assert!(facts.contains("automatic") && facts.contains("Took"), "{facts}");
        assert!(text.contains(&tr.t_args("skipped-count", &[("count", arg(2))])), "{text}");
        let phase =
            format!("{}  Info     {}", history::local_time(T0 + 1_000), phase_text(&tr, Phase::Copying));
        assert!(lines.contains(&phase.as_str()), "{text}");
        let first = format!("{}  Warning  Skipped C:\\a.pst: in use", history::local_time(T0 + 2_000));
        assert!(lines.contains(&first.as_str()), "{text}");
        assert!(text.contains("Skipped C:\\b.ost: locked"), "{text}");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn an_unfinished_run_says_so_and_has_no_duration() {
        let tr = Tr::new(Language::English);
        let r = run(Op::Merge, None, vec![Line::event(T0 + 5, RunEvent::Phase { phase: Phase::Scanning })]);
        let text = report(&tr, &r, None);
        assert!(text.contains(&tr.t("log-unfinished")), "{text}");
        assert!(!text.contains("Took"), "{text}");
        assert!(text.contains(&phase_text(&tr, Phase::Scanning)), "{text}");
    }

    #[test]
    fn an_empty_log_and_a_cancelled_run() {
        let tr = Tr::new(Language::Italian);
        let text = report(&tr, &run(Op::Backup, None, vec![]), None);
        assert!(text.contains(&tr.t("log-no-lines")), "{text}");
        let text = report(&tr, &run(Op::Backup, None, vec![Line::cancelled(T0 + 9)]), None);
        let line = format!("{}  Avviso  {}", history::local_time(T0 + 9), tr.t("run-cancelled"));
        assert!(text.lines().any(|l| l == line), "{text}");
        assert!(!text.contains(&tr.t("log-no-lines")), "{text}");
    }

    #[test]
    fn a_check_lists_every_problem() {
        let tr = Tr::new(Language::English);
        let report_of_check =
            VerifyReport { files_checked: 3, problems: vec!["a.txt missing".into(), "b.txt differs".into()] };
        let r = run(
            Op::Verify,
            Some(history::CHECK_FULL),
            vec![Line::event(T0 + 7, RunEvent::Finished { summary: Summary::Verify(report_of_check) })],
        );
        let text = report(&tr, &r, None);
        assert!(text.contains(&tr.t("verify-problems")), "{text}");
        assert!(text.lines().any(|l| l == "  a.txt missing") && text.lines().any(|l| l == "  b.txt differs"));
    }

    #[test]
    fn a_message_on_more_lines_goes_on_under_its_first_line() {
        let tr = Tr::new(Language::English);
        let text = report(&tr, &run(Op::Backup, None, vec![skipped(T0, "p", "one\ntwo")]), None);
        let head = format!("{}  Warning  ", history::local_time(T0));
        assert!(text.contains(&format!("{head}Skipped p: one\n{}two\n", " ".repeat(head.len()))), "{text}");
    }

    fn done(at_ms: i64, file: FileDone) -> FileRecord {
        FileRecord { at_ms, entry: FileEntry::Done(file) }
    }

    fn finished_backup(report: BackupReport) -> Line {
        Line::event(T0 + 60_000, RunEvent::Finished { summary: Summary::Backup(report) })
    }

    #[test]
    fn the_files_section_counts_then_lists_every_file_in_order() {
        let tr = Tr::new(Language::English);
        let summary =
            BackupReport { files_new: 1, files_changed: 1, files_unchanged: 1204, ..Default::default() };
        let r = run(Op::Backup, Some("incremental"), vec![finished_backup(summary)]);
        let files = [
            done(T0 + 1_000, FileDone::new(r"C:\docs\new.txt", FileAction::New, 2048)),
            FileRecord {
                at_ms: T0 + 1_000,
                entry: FileEntry::Skipped(Skipped { path: r"C:\mail.pst".into(), reason: "in use".into() }),
            },
            done(
                T0 + 2_000,
                FileDone {
                    new_chunk_bytes: Some(4 << 20),
                    ..FileDone::new(r"C:\vm\disk.vhdx", FileAction::Changed, 40 << 30)
                },
            ),
            done(T0 + 2_000, FileDone::new(r"C:\old.txt", FileAction::Deleted, 5)),
        ];
        let text = report(&tr, &r, Some(&files));
        let lines: Vec<&str> = text.lines().collect();
        let title = lines.iter().position(|l| *l == "Files").expect("files section");
        assert_eq!(
            lines.get(title + 1).copied(),
            Some(
                "New: 1 · Changed: 1 · Saved as changes: 1 · Unchanged, not listed: 1204 · Deleted: 1 · \
                 Skipped: 1"
            ),
            "{text}"
        );
        let (t1, t2) = (history::local_time(T0 + 1_000), history::local_time(T0 + 2_000));
        let expected = [
            format!(r"{t1}  New       2.0 KB  C:\docs\new.txt"),
            format!(r"{t1}  Skipped           C:\mail.pst  (in use)"),
            format!(
                r"{t2}  Changed  40.0 GB  C:\vm\disk.vhdx  (saved as changes: 4.0 MB of new chunks written)"
            ),
            format!(r"{t2}  Deleted      5 B  C:\old.txt"),
        ];
        assert_eq!(
            lines.get(title + 2..title + 6),
            Some(&expected.iter().map(String::as_str).collect::<Vec<_>>()[..]),
            "{text}"
        );
        assert!(lines.iter().position(|l| *l == "Log").unwrap() < title, "files after the steps");
    }

    #[test]
    fn a_run_without_a_file_list_or_without_files_says_so() {
        let tr = Tr::new(Language::English);
        let r = run(Op::Backup, Some("incremental"), vec![finished_backup(BackupReport::default())]);
        assert!(report(&tr, &r, None).contains(&tr.t("log-files-missing")));
        let text = report(&tr, &r, Some(&[]));
        assert!(
            text.contains(&tr.t("log-files-none")) && !text.contains(&tr.t("log-files-missing")),
            "{text}"
        );
    }

    #[test]
    fn a_full_check_counts_the_intact_files_and_lists_the_damaged_ones() {
        let tr = Tr::new(Language::English);
        let summary =
            VerifyReport { files_checked: 10, problems: vec!["x: C/a.txt has different content".into()] };
        let r = run(
            Op::Verify,
            Some(history::CHECK_FULL),
            vec![Line::event(T0 + 7, RunEvent::Finished { summary: Summary::Verify(summary) })],
        );
        let damaged = FileDone {
            note: "x: C/a.txt has different content".into(),
            ..FileDone::new(r"C:\a.txt", FileAction::Damaged, 3)
        };
        let text = report(&tr, &r, Some(&[done(T0 + 5, damaged)]));
        assert!(text.contains("Read back intact, not listed: 9 · Problems: 1"), "{text}");
        assert!(text.contains(r"C:\a.txt  (x: C/a.txt has different content)"), "{text}");
    }

    #[test]
    fn the_text_log_is_named_after_the_run_log_and_readable_by_notepad() {
        let dir = tempfile::tempdir().unwrap();
        let path = report_path(dir.path(), Path::new("logs/20261030-123105-123-backup-6f1c3d1e.jsonl"));
        assert_eq!(path, dir.path().join(REPORT_DIR).join("20261030-123105-123-backup-6f1c3d1e.txt"));
        write_report(&path, "a\nb\n").unwrap();
        assert_eq!(fs::read(&path).unwrap(), "\u{feff}a\r\nb\r\n".as_bytes());
    }
}
