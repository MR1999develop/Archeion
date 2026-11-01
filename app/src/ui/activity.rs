//! Activity tab: what the running backup, merge or restore is doing, and
//! its summary once it ends.

use super::theme;
use crate::{
    activity::{Activity, Op, Outcome, PhaseState, format_duration},
    i18n::{Tr, arg},
};
use archeion_core::{BackupReport, Phase, Summary};
use eframe::egui::{self, RichText, Ui};
use std::time::{Duration, Instant};

/// Indicator appears only for waits longer than this.
const SPINNER_DELAY: Duration = Duration::from_secs(1);
/// Path length shown in full; longer ones are shortened in the middle.
const PATH_CHARS: usize = 90;

/// What the user asked from the tab.
#[derive(Debug, Default)]
pub struct Request {
    pub cancel: bool,
    pub show_backups: bool,
}

pub fn show(ui: &mut Ui, tr: &Tr, a: &Activity, profile_name: &str, now: Instant) -> Request {
    let mut req = Request::default();
    ui.set_max_width(theme::FORM_MAX_WIDTH + 120.0);
    ui.add_space(theme::SPACE_S);

    let title = tr.t_args(
        match (a.op, a.is_running()) {
            (Op::Backup, true) => "activity-backup-running",
            (Op::Merge, true) => "activity-merge-running",
            (Op::Delete, true) => "activity-delete-running",
            (Op::Restore, true) => "activity-restore-running",
            (Op::Verify, true) => "activity-verify-running",
            (Op::Verify, false) => "activity-verify-ended",
            (Op::Backup, false) => "activity-backup-ended",
            (Op::Merge, false) => "activity-merge-ended",
            (Op::Delete, false) => "activity-delete-ended",
            (Op::Restore, false) => "activity-restore-ended",
        },
        &[("name", arg(profile_name.to_owned()))],
    );
    ui.heading(title);
    ui.add_space(theme::SPACE_M);

    phases_ui(ui, tr, a);
    ui.add_space(theme::SPACE_M);

    if a.is_running() {
        progress_ui(ui, tr, a, now);
    } else {
        outcome_ui(ui, tr, a, now, &mut req);
    }

    if a.skipped_count > 0 {
        ui.add_space(theme::SPACE_M);
        skipped_ui(ui, tr, a);
    }

    if a.is_running() {
        ui.add_space(theme::SPACE_L);
        if ui.button(tr.t("cancel")).on_hover_text(tr.t("cancel-hint")).clicked() {
            req.cancel = true;
        }
        // Elapsed time and speed change even without new events.
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
    req
}

fn phases_ui(ui: &mut Ui, tr: &Tr, a: &Activity) {
    ui.horizontal_wrapped(|ui| {
        for (i, (phase, state)) in a.phases().into_iter().enumerate() {
            if i > 0 {
                ui.weak("›");
            }
            let name = phase_text(tr, phase);
            match state {
                PhaseState::Done => {
                    ui.label(format!("✔ {name}"));
                }
                PhaseState::Current => {
                    ui.label(RichText::new(format!("● {name}")).strong());
                }
                PhaseState::Pending => {
                    ui.weak(format!("○ {name}"));
                }
            }
        }
    });
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

fn outcome_ui(ui: &mut Ui, tr: &Tr, a: &Activity, now: Instant, req: &mut Request) {
    let took = tr.t_args("activity-took", &[("time", arg(format_duration(a.elapsed(now))))]);
    let (text, warn) = match &a.outcome {
        Some(Outcome::Finished(summary)) => summary_text(tr, a.op, summary),
        Some(Outcome::Cancelled) => (tr.t("run-cancelled"), false),
        Some(o) => (problem_text(tr, o).unwrap_or_default(), true),
        None => (String::new(), false),
    };
    let icon = if warn { "⚠" } else { "✔" };
    let line = format!("{icon} {text}");
    if warn {
        ui.colored_label(ui.visuals().warn_fg_color, line);
    } else {
        ui.label(RichText::new(line).strong());
    }
    ui.weak(took);
    if let Some(Outcome::Finished(Summary::Verify(r))) = &a.outcome {
        problems_ui(ui, tr, &r.problems);
    }
    if matches!(a.outcome, Some(Outcome::Finished(_))) && a.op != Op::Restore {
        ui.add_space(theme::SPACE_S);
        if ui.button(tr.t("show-backups")).clicked() {
            req.show_backups = true;
        }
    }
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
pub fn problem_text(tr: &Tr, outcome: &Outcome) -> Option<String> {
    Some(match outcome {
        Outcome::Finished(_) => return None,
        Outcome::DestinationUnavailable(loc) => {
            tr.t_args("run-destination-missing", &[("location", arg(loc.clone()))])
        }
        Outcome::Busy => tr.t("run-busy"),
        Outcome::PasswordNeeded => tr.t("run-password-needed"),
        Outcome::WrongPassword => tr.t("run-wrong-password"),
        Outcome::NotEnoughSpace { needed, available } => no_space_text(tr, *needed, *available),
        Outcome::Failed(msg) => tr.t_args("run-failed", &[("detail", arg(msg.clone()))]),
        Outcome::Cancelled => tr.t("run-cancelled"),
    })
}

/// A full destination: how much is needed and free, and what to do.
pub fn no_space_text(tr: &Tr, needed: u64, available: u64) -> String {
    tr.t_args("run-no-space", &[("needed", arg(tr.bytes(needed))), ("available", arg(tr.bytes(available)))])
}

/// `text`, plus how many files a resumed backup kept from the interrupted run.
pub fn with_resumed(tr: &Tr, text: String, r: &BackupReport) -> String {
    if r.resumed_files == 0 {
        text
    } else {
        let extra = tr.t_args("backup-resumed", &[("count", arg(r.resumed_files))]);
        format!("{text} · {extra}")
    }
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
    let n = path.chars().count();
    if n <= PATH_CHARS {
        return path.to_owned();
    }
    let keep = PATH_CHARS / 2 - 1;
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
    }
}
