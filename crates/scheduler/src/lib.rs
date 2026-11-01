//! Automatic runs through Windows Task Scheduler.
//!
//! Each profile has up to two tasks in the `\Archeion\` folder of Task
//! Scheduler: `<id>-backup` starts `archeion.exe run <id> --scheduled`,
//! `<id>-merge` starts `archeion.exe merge <id> --scheduled`. Tasks run with
//! the user's rights (no elevation) and "Start the task as soon as possible
//! after a scheduled start is missed", so a run skipped while the PC was off
//! happens at the next boot. Battery and metered networks are deliberately
//! ignored.
//!
//! [`task_xml`] is pure and tested on every platform; [`WindowsTaskScheduler`]
//! hands it to `schtasks.exe`.

use archeion_core::{ProfileId, Schedule, Weekday};
use std::{fmt::Write as _, path::PathBuf};

/// The two jobs a profile can schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Backup,
    Merge,
}

impl TaskKind {
    pub const ALL: [Self; 2] = [Self::Backup, Self::Merge];

    fn suffix(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Merge => "merge",
        }
    }

    /// Worker subcommand.
    fn command(self) -> &'static str {
        match self {
            Self::Backup => "run",
            Self::Merge => "merge",
        }
    }
}

/// What to register for one profile and job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpec {
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub kind: TaskKind,
    pub schedule: Schedule,
    /// Absolute path of `archeion.exe`.
    pub executable: PathBuf,
}

/// Failure while talking to Task Scheduler.
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("Task Scheduler is only available on Windows")]
    Unsupported,
    #[error("could not run schtasks.exe")]
    Launch(#[source] std::io::Error),
    #[error("schtasks.exe failed: {0}")]
    Rejected(String),
}

/// Role used by the application: install or remove a profile's task.
pub trait Scheduler {
    /// Creates or replaces the task. A [`Schedule::Manual`] profile has its
    /// task removed instead.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    fn apply(&self, spec: &TaskSpec) -> Result<(), SchedulerError>;

    /// Removes one task; removing a missing task is not an error.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    fn remove(&self, id: ProfileId, kind: TaskKind) -> Result<(), SchedulerError>;
}

/// Task path inside Task Scheduler.
#[must_use]
pub fn task_name(id: ProfileId, kind: TaskKind) -> String {
    format!(r"\Archeion\{id}-{}", kind.suffix())
}

/// Task path used by Archeion, the program's first name.
#[must_use]
pub fn legacy_task_name(id: ProfileId, kind: TaskKind) -> String {
    format!(r"\Archeion\{id}-{}", kind.suffix())
}

/// Task Scheduler XML for `spec`, or `None` for a manual profile.
#[must_use]
pub fn task_xml(spec: &TaskSpec) -> Option<String> {
    let (at, schedule_xml) = match spec.schedule {
        Schedule::Manual => return None,
        Schedule::Daily { at } => {
            (at, "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>".to_owned())
        }
        Schedule::Weekly { day, at } => (
            at,
            format!(
                "<ScheduleByWeek><DaysOfWeek><{d}/></DaysOfWeek><WeeksInterval>1</WeeksInterval></ScheduleByWeek>",
                d = day_element(day)
            ),
        ),
    };
    let mut xml = String::new();
    // Infallible: writing to a String.
    let _ = write!(
        xml,
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Archeion {what}: {name}</Description>
  </RegistrationInfo>
  <Triggers>
    <CalendarTrigger>
      <StartBoundary>2026-01-01T{at}:00</StartBoundary>
      <Enabled>true</Enabled>
      {schedule_xml}
    </CalendarTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
      <Arguments>{command} {id} --scheduled</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        name = escape(&spec.profile_name),
        exe = escape(&spec.executable.display().to_string()),
        id = spec.profile_id,
        command = spec.kind.command(),
        what = spec.kind.suffix(),
    );
    Some(xml)
}

fn day_element(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// [`Scheduler`] backed by `schtasks.exe`.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsTaskScheduler;

impl Scheduler for WindowsTaskScheduler {
    fn apply(&self, spec: &TaskSpec) -> Result<(), SchedulerError> {
        let Some(xml) = task_xml(spec) else {
            return self.remove(spec.profile_id, spec.kind);
        };
        // schtasks wants a file; UTF-16 LE with BOM matches the declaration.
        let mut file = tempfile::Builder::new().suffix(".xml").tempfile().map_err(SchedulerError::Launch)?;
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
        std::io::Write::write_all(&mut file, &bytes).map_err(SchedulerError::Launch)?;
        let path = file.path().display().to_string();
        schtasks(&["/Create", "/TN", &task_name(spec.profile_id, spec.kind), "/XML", &path, "/F"])
    }

    fn remove(&self, id: ProfileId, kind: TaskKind) -> Result<(), SchedulerError> {
        delete_task(&task_name(id, kind)).map(|_| ())
    }
}

impl WindowsTaskScheduler {
    /// Removes the task Archeion registered for `id`, which starts an
    /// executable that no longer exists. Returns true when there was one.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    pub fn remove_legacy(&self, id: ProfileId, kind: TaskKind) -> Result<bool, SchedulerError> {
        delete_task(&legacy_task_name(id, kind))
    }
}

/// Deletes the task `name`; `Ok(false)` when there is none.
fn delete_task(name: &str) -> Result<bool, SchedulerError> {
    match schtasks(&["/Query", "/TN", name]) {
        Ok(()) => schtasks(&["/Delete", "/TN", name, "/F"]).map(|()| true),
        Err(SchedulerError::Rejected(_)) => Ok(false), // no such task
        Err(e) => Err(e),
    }
}

#[cfg(windows)]
fn schtasks(args: &[&str]) -> Result<(), SchedulerError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(SchedulerError::Launch)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(SchedulerError::Rejected(String::from_utf8_lossy(&out.stderr).trim().to_owned()))
    }
}

#[cfg(not(windows))]
fn schtasks(_args: &[&str]) -> Result<(), SchedulerError> {
    Err(SchedulerError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::TimeOfDay;

    fn spec(schedule: Schedule) -> TaskSpec {
        TaskSpec {
            profile_id: "6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(),
            profile_name: "Docs & <Photos>".into(),
            kind: TaskKind::Backup,
            schedule,
            executable: r"C:\Program Files\Archeion\archeion.exe".into(),
        }
    }

    #[test]
    fn manual_profile_has_no_task() {
        assert_eq!(task_xml(&spec(Schedule::Manual)), None);
    }

    #[test]
    fn daily_task_runs_at_the_chosen_time_and_catches_up() {
        let xml = task_xml(&spec(Schedule::Daily { at: TimeOfDay::new(8, 5).unwrap() })).unwrap();
        assert!(xml.contains("<StartBoundary>2026-01-01T08:05:00</StartBoundary>"));
        assert!(xml.contains("<DaysInterval>1</DaysInterval>"));
        assert!(xml.contains("<StartWhenAvailable>true</StartWhenAvailable>"));
        assert!(xml.contains("<RunLevel>LeastPrivilege</RunLevel>"));
    }

    #[test]
    fn weekly_task_names_the_day() {
        let s = Schedule::Weekly { day: Weekday::Sunday, at: TimeOfDay::default() };
        assert!(task_xml(&spec(s)).unwrap().contains("<DaysOfWeek><Sunday/></DaysOfWeek>"));
    }

    #[test]
    fn names_and_paths_are_escaped_and_quoted() {
        let xml = task_xml(&spec(Schedule::Daily { at: TimeOfDay::default() })).unwrap();
        assert!(xml.contains("Archeion backup: Docs &amp; &lt;Photos&gt;"));
        assert!(xml.contains(r#"<Command>"C:\Program Files\Archeion\archeion.exe"</Command>"#));
        assert!(xml.contains("run 6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f --scheduled"));
    }

    #[test]
    fn tasks_live_in_their_own_folder() {
        let id = spec(Schedule::Manual).profile_id;
        assert_eq!(task_name(id, TaskKind::Backup), format!(r"\Archeion\{id}-backup"));
        assert_eq!(task_name(id, TaskKind::Merge), format!(r"\Archeion\{id}-merge"));
        assert_eq!(legacy_task_name(id, TaskKind::Merge), format!(r"\Archeion\{id}-merge"));
    }

    #[test]
    fn merge_task_starts_the_merge_command() {
        let mut s = spec(Schedule::Daily { at: TimeOfDay::default() });
        s.kind = TaskKind::Merge;
        assert!(
            task_xml(&s)
                .unwrap()
                .contains("<Arguments>merge 6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f --scheduled</Arguments>")
        );
    }
}
