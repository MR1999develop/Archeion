//! Activity tab: what the running backup, merge or restore is doing. Once
//! it ends, the tab shows its log like any past run.

use super::{theme, widgets};
use crate::{
    activity::{Activity, Count, Handled, Op, Outcome, PhaseState, RecentFile, Step, format_duration},
    history,
    i18n::{Tr, arg},
};
use archeion_core::{BackupReport, FileAction, MergeGroup, Phase, Summary};
use eframe::egui::{self, RichText, Ui};
use std::time::{Duration, Instant};

/// Indicator appears only for waits longer than this.
const SPINNER_DELAY: Duration = Duration::from_secs(1);
/// Path length shown in full; longer ones are shortened in the middle.
const PATH_CHARS: usize = 90;
/// Same, in the list of recent files, which has two more columns.
const RECENT_PATH_CHARS: usize = 64;
/// Height of the list of recent files: it scrolls instead of pushing the
/// rest of the page down.
const RECENT_HEIGHT: f32 = 200.0;

/// The run in progress: its steps with what each did, progress, the most
/// recent files, skipped files, and Cancel. Returns true when Cancel was
/// clicked.
pub fn show(ui: &mut Ui, tr: &Tr, a: &Activity, profile_name: &str, now: Instant) -> bool {
    ui.set_max_width(theme::FORM_MAX_WIDTH + 120.0);
    ui.add_space(theme::SPACE_S);

    let title = tr.t_args(
        match a.op {
            Op::Backup => "activity-backup-running",
            Op::Merge => "activity-merge-running",
            Op::Delete => "activity-delete-running",
            Op::Restore => "activity-restore-running",
            Op::Verify => "activity-verify-running",
        },
        &[("name", arg(profile_name.to_owned()))],
    );
    ui.heading(title);
    ui.add_space(theme::SPACE_M);

    steps_ui(ui, tr, a, now);
    ui.add_space(theme::SPACE_M);
    progress_ui(ui, tr, a, now);

    if a.recent().next().is_some() {
        ui.add_space(theme::SPACE_M);
        recent_ui(ui, tr, a);
    }

    if a.skipped_count > 0 {
        ui.add_space(theme::SPACE_M);
        skipped_ui(ui, tr, a);
    }

    ui.add_space(theme::SPACE_L);
    let cancel = ui.button(tr.t("cancel")).on_hover_text(tr.t("cancel-hint")).clicked();
    // Elapsed time and speed change even without new events.
    ui.ctx().request_repaint_after(Duration::from_millis(500));
    cancel
}

/// One line per step: its state, name, start time and duration, then
/// what it did so far.
fn steps_ui(ui: &mut Ui, tr: &Tr, a: &Activity, now: Instant) {
    for (i, step) in a.steps(now).into_iter().enumerate() {
        let name = phase_text(tr, step.phase);
        ui.horizontal(|ui| {
            match step.state {
                PhaseState::Done => ui.label(format!("✔ {name}")),
                PhaseState::Current => ui.label(RichText::new(format!("● {name}")).strong()),
                PhaseState::Pending => ui.weak(format!("○ {name}")),
            };
            if let Some(timing) = step_timing(&step) {
                ui.weak(timing);
            }
        });
        let counts = counts_text(tr, &step.counts);
        let group = a.merge_group.as_ref().filter(|_| step.state == PhaseState::Current);
        if counts.is_empty() && group.is_none() {
            continue;
        }
        ui.indent(("activity-step", i), |ui| {
            if let Some(g) = group {
                ui.label(group_text(tr, g));
            }
            if !counts.is_empty() {
                let text = RichText::new(counts);
                ui.add(
                    egui::Label::new(if step.state == PhaseState::Current { text } else { text.weak() })
                        .wrap(),
                );
            }
        });
    }
}

/// `21:30:04 · 45 s`: when a step started and how long it took.
fn step_timing(step: &Step) -> Option<String> {
    let started = history::local_time(step.started_ms?);
    let took = step.took.map(format_duration).unwrap_or_default();
    Some(format!("{} · {took}", started.get(11..).unwrap_or(&started)))
}

/// `1.234 files found, 56 folders read, 3 new`.
fn counts_text(tr: &Tr, counts: &[(Count, u64)]) -> String {
    let parts: Vec<String> = counts
        .iter()
        .map(|(c, n)| {
            let key = match c {
                Count::Found => "step-found",
                Count::Folders => "step-folders",
                Count::New => "step-new",
                Count::Changed => "step-changed",
                Count::Unchanged => "step-unchanged",
                Count::CopiedAgain => "step-copied-again",
                Count::Moved => "step-moved",
                Count::Restored => "step-restored",
                Count::Kept => "step-kept",
                Count::Checked => "step-checked",
                Count::Damaged => "step-damaged",
                Count::Skipped => "step-skipped",
            };
            tr.t_args(key, &[("count", arg(*n))])
        })
        .collect();
    parts.join(", ")
}

fn group_text(tr: &Tr, g: &MergeGroup) -> String {
    let args = [
        ("index", arg(g.index)),
        ("of", arg(g.of)),
        ("count", arg(g.archives)),
        ("first", arg(g.first.clone())),
        ("last", arg(g.last.clone())),
    ];
    tr.t_args(if g.of > 1 { "merge-group-of" } else { "merge-group" }, &args)
}

/// The files handled last, newest on top, in a fixed-height list.
fn recent_ui(ui: &mut Ui, tr: &Tr, a: &Activity) {
    widgets::section_title(ui, tr.t("recent-title"));
    egui::ScrollArea::vertical()
        .id_salt("recent-files")
        .max_height(RECENT_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Grid::new("recent-grid").num_columns(3).striped(true).spacing([theme::SPACE_M, 2.0]).show(
                ui,
                |ui| {
                    for f in a.recent() {
                        recent_row(ui, tr, f);
                        ui.end_row();
                    }
                },
            );
        });
}

/// Text id of the word for what happened to a file, as in the list of
/// the latest files and in the full log.
pub(super) fn action_key(action: FileAction) -> &'static str {
    match action {
        FileAction::New => "recent-new",
        FileAction::Changed => "recent-changed",
        // Only a complete backup copies unchanged files.
        FileAction::Unchanged => "recent-copied-again",
        FileAction::Moved => "recent-moved",
        FileAction::Restored => "recent-restored",
        FileAction::Kept => "recent-kept",
        FileAction::Checked => "recent-checked",
        FileAction::Damaged => "recent-damaged",
        FileAction::Deleted => "recent-deleted",
        FileAction::Dropped => "recent-dropped",
    }
}

/// Text id of the word for a skipped file, beside [`action_key`].
pub(super) const SKIPPED_KEY: &str = "recent-skipped";

fn recent_row(ui: &mut Ui, tr: &Tr, f: &RecentFile) {
    let (key, color, extra) = match &f.handled {
        Handled::Done { action, bytes } => {
            let color = (*action == FileAction::Damaged).then(|| ui.visuals().error_fg_color);
            (action_key(*action), color, tr.bytes(*bytes))
        }
        Handled::Skipped { reason } => (SKIPPED_KEY, Some(ui.visuals().warn_fg_color), reason.clone()),
    };
    let what = RichText::new(tr.t(key));
    ui.label(match color {
        Some(c) => what.color(c),
        None => what.weak(),
    });
    ui.label(shorten_to(&f.path, RECENT_PATH_CHARS)).on_hover_text(&f.path);
    ui.weak(extra);
}

fn progress_ui(ui: &mut Ui, tr: &Tr, a: &Activity, now: Instant) {
    match a.fraction() {
        Some(f) => {
            let text = tr.t_args(
                "progress-of",
                &[("done", arg(tr.bytes(a.done_bytes))), ("total", arg(tr.bytes(a.total_bytes)))],
            );
            ui.add(egui::ProgressBar::new(f).show_percentage().text(text));
        }
        None if a.elapsed(now) > SPINNER_DELAY => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr.t("activity-preparing"));
            });
        }
        None => ui.ctx().request_repaint_after(SPINNER_DELAY),
    }
    ui.add_space(theme::SPACE_S);

    egui::Grid::new("activity-details").num_columns(2).spacing([theme::SPACE_M, theme::SPACE_S]).show(
        ui,
        |ui| {
            if !a.current.is_empty() {
                let label =
                    if a.phase == Some(Phase::Scanning) { "activity-folder" } else { "activity-file" };
                ui.weak(tr.t(label));
                ui.label(shorten(&a.current)).on_hover_text(&a.current);
                ui.end_row();
            }
            if a.files_done > 0 || a.files_total > 0 {
                ui.weak(tr.t("activity-files"));
                let text = if a.files_total > 0 {
                    tr.t_args("count-of", &[("done", arg(a.files_done)), ("total", arg(a.files_total))])
                } else {
                    tr.t_args("files-found", &[("count", arg(a.files_done))])
                };
                ui.label(text);
                ui.end_row();
            }
            if let Some(speed) = a.speed() {
                ui.weak(tr.t("activity-speed"));
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "display only")]
                let per_second = speed as u64;
                ui.label(format!("{}/s", tr.bytes(per_second)));
                ui.end_row();
            }
            ui.weak(tr.t("activity-elapsed"));
            ui.label(format_duration(a.elapsed(now)));
            ui.end_row();
            if let Some(left) = a.remaining(now) {
                ui.weak(tr.t("activity-remaining"));
                ui.label(tr.t_args("about", &[("time", arg(format_duration(left)))]));
                ui.end_row();
            }
        },
    );
}

/// What an integrity check found, one line per problem.
pub fn problems_ui(ui: &mut Ui, tr: &Tr, problems: &[String]) {
    if problems.is_empty() {
        return;
    }
    ui.add_space(theme::SPACE_S);
    ui.colored_label(ui.visuals().error_fg_color, format!("✖ {}", tr.t("verify-problems")));
    egui::ScrollArea::vertical().id_salt("verify-problems").max_height(220.0).show(ui, |ui| {
        for p in problems {
            ui.add(egui::Label::new(RichText::new(p).color(ui.visuals().error_fg_color)).wrap());
        }
    });
    ui.weak(tr.t("verify-problems-hint"));
}

fn skipped_ui(ui: &mut Ui, tr: &Tr, a: &Activity) {
    let title = tr.t_args("skipped-count", &[("count", arg(a.skipped_count))]);
    egui::CollapsingHeader::new(RichText::new(format!("⚠ {title}")).color(ui.visuals().warn_fg_color))
        .id_salt("activity-skipped")
        .default_open(false)
        .show(ui, |ui| {
            ui.weak(tr.t("skipped-hint"));
            egui::ScrollArea::vertical().id_salt("skipped-list").max_height(220.0).show(ui, |ui| {
                egui::Grid::new("skipped-grid").num_columns(2).striped(true).show(ui, |ui| {
                    for s in &a.skipped {
                        ui.label(shorten(&s.path)).on_hover_text(&s.path);
                        ui.weak(&s.reason);
                        ui.end_row();
                    }
                });
                if a.skipped_count > a.skipped.len() {
                    ui.weak(tr.t_args("skipped-more", &[("count", arg(a.skipped_count - a.skipped.len()))]));
                }
            });
        });
}

/// What a finished run did, and whether it needs attention.
pub fn summary_text(tr: &Tr, op: Op, summary: &Summary) -> (String, bool) {
    match summary {
        Summary::Backup(r) => {
            let text = tr.t_args(
                if r.complete { "activity-complete-summary" } else { "activity-backup-summary" },
                &[
                    ("new", arg(r.files_new)),
                    ("changed", arg(r.files_changed)),
                    ("deleted", arg(r.files_deleted)),
                    ("unchanged", arg(r.files_unchanged)),
                    ("copied", arg(tr.bytes(r.bytes_copied))),
                ],
            );
            (with_resumed(tr, text, r), !r.skipped.is_empty())
        }
        Summary::Merge(r) => (
            tr.t_args(
                if op == Op::Delete { "activity-delete-summary" } else { "activity-merge-summary" },
                &[
                    ("before", arg(r.archives_before)),
                    ("after", arg(r.archives_after)),
                    ("freed", arg(tr.bytes(r.bytes_freed))),
                ],
            ),
            false,
        ),
        Summary::Restore(r) => (
            tr.t_args(
                "activity-restore-summary",
                &[("restored", arg(r.files_restored)), ("kept", arg(r.files_kept))],
            ),
            !r.skipped.is_empty(),
        ),
        Summary::Verify(r) => (
            tr.t_args(
                "verify-finished",
                &[("count", arg(r.files_checked)), ("problems", arg(r.problems.len()))],
            ),
            !r.problems.is_empty(),
        ),
    }
}

/// Text of a run that ended without a summary.
pub fn problem_text(tr: &Tr, op: Op, outcome: &Outcome) -> Option<String> {
    Some(match outcome {
        Outcome::Finished(_) => return None,
        Outcome::DestinationUnavailable(loc) => {
            tr.t_args("run-destination-missing", &[("location", arg(loc.clone()))])
        }
        Outcome::Busy => tr.t("run-busy"),
        Outcome::PasswordNeeded => tr.t("run-password-needed"),
        Outcome::WrongPassword => tr.t("run-wrong-password"),
        Outcome::NotEnoughSpace { needed, available } => no_space_text(tr, op, *needed, *available),
        Outcome::Failed(msg) => tr.t_args("run-failed", &[("detail", arg(msg.clone()))]),
        Outcome::Cancelled => tr.t("run-cancelled"),
    })
}

/// A full drive: how much is needed and free, and what to do (a restore
/// fills the drive it restores to, the other operations the destination).
pub fn no_space_text(tr: &Tr, op: Op, needed: u64, available: u64) -> String {
    let key = if op == Op::Restore { "restore-no-space" } else { "run-no-space" };
    tr.t_args(key, &[("needed", arg(tr.bytes(needed))), ("available", arg(tr.bytes(available)))])
}

/// `text`, plus how many files a resumed backup kept from the interrupted
/// run and how many were saved as changes only.
pub fn with_resumed(tr: &Tr, text: String, r: &BackupReport) -> String {
    let mut text = text;
    if r.resumed_files > 0 {
        let extra = tr.t_args("backup-resumed", &[("count", arg(r.resumed_files))]);
        text = format!("{text} · {extra}");
    }
    if r.files_as_changes > 0 {
        let extra = tr.t_args("backup-as-changes", &[("count", arg(r.files_as_changes))]);
        text = format!("{text} · {extra}");
    }
    text
}

pub fn phase_text(tr: &Tr, p: Phase) -> String {
    tr.t(match p {
        Phase::Opening => "phase-opening",
        Phase::Scanning => "phase-scanning",
        Phase::Copying => "phase-copying",
        Phase::Merging => "phase-merging",
        Phase::Restoring => "phase-restoring",
        Phase::Verifying => "phase-verifying",
    })
}

/// Keeps the start and the end of a long path: `C:\Users\…\file.txt`.
fn shorten(path: &str) -> String {
    shorten_to(path, PATH_CHARS)
}

/// [`shorten`] to at most `max` characters.
fn shorten_to(path: &str, max: usize) -> String {
    let n = path.chars().count();
    if n <= max {
        return path.to_owned();
    }
    let keep = max / 2 - 1;
    let head: String = path.chars().take(keep).collect();
    let tail: String = path.chars().skip(n - keep).collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_paths_keep_both_ends() {
        let long = format!(r"C:\{}\report.docx", "a".repeat(200));
        let s = shorten(&long);
        assert!(s.chars().count() <= PATH_CHARS);
        assert!(s.starts_with(r"C:\aa") && s.ends_with("report.docx") && s.contains('…'));
        assert_eq!(shorten(r"C:\short.txt"), r"C:\short.txt");
        assert!(shorten_to(&long, RECENT_PATH_CHARS).chars().count() <= RECENT_PATH_CHARS);
    }
}
