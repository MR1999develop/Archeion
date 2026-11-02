//! Automatic runs through Windows Task Scheduler.
//!
//! Each profile has up to two tasks in the `\Archeion\` folder of Task
//! Scheduler: `<id>-backup` starts `archeion.exe run <id> --scheduled` by
//! the clock (chosen days and times, every few hours, monthly), and
//! `<id>-events` starts
//! `archeion.exe run <id> --scheduled --event` at logon, when the computer
//! is idle or when a drive is connected. One more task, `watch`, warns about
//! profiles not backed up for too long. Tasks run with the user's rights (no
//! elevation) and "Start the task as soon as possible after a scheduled
//! start is missed", so a run skipped while the PC was off happens at the
//! next boot. Battery and metered networks are deliberately ignored.
//!
//! [`task_xml`] is pure and tested on every platform; [`WindowsTaskScheduler`]
//! hands it to `schtasks.exe`.

use archeion_core::{EventTriggers, ProfileId, Schedule, TimeOfDay, Weekday};
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

/// The jobs a profile can schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// Backups by the clock.
    Backup,
    /// Merges by the clock: no longer registered (the window merges while
    /// open); kept so the tasks of older versions are removed.
    Merge,
    /// Backups started by events: logon, idle, drive connected.
    Events,
}

impl TaskKind {
    pub const ALL: [Self; 3] = [Self::Backup, Self::Merge, Self::Events];

    fn suffix(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::Merge => "merge",
            Self::Events => "events",
        }
    }

    /// Worker arguments after the profile id.
    fn arguments(self, id: ProfileId) -> String {
        match self {
            Self::Backup => format!("run {id} --scheduled"),
            Self::Merge => format!("merge {id} --scheduled"),
            Self::Events => format!("run {id} --scheduled --event"),
        }
    }
}

/// What to register for one profile and job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpec {
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub kind: TaskKind,
    /// Used by [`TaskKind::Backup`] and [`TaskKind::Merge`].
    pub schedule: Schedule,
    /// Used by [`TaskKind::Events`].
    pub events: EventTriggers,
    /// Absolute path of `archeion.exe`.
    pub executable: PathBuf,
    /// `DOMAIN\user` whose logon starts an event backup (a logon trigger
    /// for any user would need administrator rights).
    pub user: String,
}

impl TaskSpec {
    /// True when the task should exist.
    #[must_use]
    pub fn is_wanted(&self) -> bool {
        match self.kind {
            TaskKind::Backup | TaskKind::Merge => self.schedule.is_automatic(),
            TaskKind::Events => self.events.any(),
        }
    }
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

/// Task path of the daily check that warns about profiles not backed up
/// for too long (one for all profiles).
pub const WATCH_TASK: &str = r"\Archeion\watch";

/// Task Scheduler XML for `spec`, or `None` when the task should not exist
/// (manual schedule, no event chosen).
#[must_use]
pub fn task_xml(spec: &TaskSpec) -> Option<String> {
    if !spec.is_wanted() {
        return None;
    }
    let (triggers, idle) = match spec.kind {
        TaskKind::Backup | TaskKind::Merge => (schedule_triggers(&spec.schedule), false),
        TaskKind::Events => (event_triggers(&spec.events, &spec.user), spec.events.when_idle),
    };
    let what = format!("{}: {}", spec.kind.suffix(), spec.profile_name);
    Some(document(&what, &triggers, idle, &spec.executable, &spec.kind.arguments(spec.profile_id)))
}

/// XML of [`WATCH_TASK`]: every day at 12:00 (or as soon as the computer
/// is on) and a quarter of an hour after `user` signs in.
#[must_use]
pub fn watch_task_xml(executable: &std::path::Path, user: &str) -> String {
    let noon = TimeOfDay::new(12, 0).unwrap_or_default();
    let triggers = format!(
        "{}{}",
        calendar(noon, "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>", ""),
        logon(user, "PT15M")
    );
    document("watch", &triggers, false, executable, "watch")
}

/// The whole task document around `triggers`.
fn document(what: &str, triggers: &str, idle: bool, exe: &std::path::Path, arguments: &str) -> String {
    let idle_settings = if idle {
        "\n    <IdleSettings>\n      <Duration>PT10M</Duration>\n      <WaitTimeout>PT1H</WaitTimeout>\n      \
         <StopOnIdleEnd>false</StopOnIdleEnd>\n      <RestartOnIdle>false</RestartOnIdle>\n    </IdleSettings>"
    } else {
        ""
    };
    let mut xml = String::new();
    // Infallible: writing to a String.
    let _ = write!(
        xml,
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Archeion {what}</Description>
  </RegistrationInfo>
  <Triggers>
{triggers}  </Triggers>
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
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>{idle_settings}
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        what = escape(what),
        exe = escape(&exe.display().to_string()),
        arguments = escape(arguments),
    );
    xml
}

/// One `<CalendarTrigger>` at `at`, repeating as `schedule_xml` says, with
/// an optional `<Repetition>` block.
fn calendar(at: TimeOfDay, schedule_xml: &str, repetition: &str) -> String {
    // Same element order as Task Scheduler's own exports.
    format!(
        "    <CalendarTrigger>{repetition}\n      <StartBoundary>2026-01-01T{at}:00</StartBoundary>\n      \
         <Enabled>true</Enabled>\n      {schedule_xml}\n    </CalendarTrigger>\n"
    )
}

fn logon(user: &str, delay: &str) -> String {
    format!(
        "    <LogonTrigger>\n      <Enabled>true</Enabled>\n      <UserId>{}</UserId>\n      \
         <Delay>{delay}</Delay>\n    </LogonTrigger>\n",
        escape(user)
    )
}

/// Triggers of a clock schedule (empty for a manual one).
fn schedule_triggers(schedule: &Schedule) -> String {
    const EVERY_DAY: &str = "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>";
    match schedule.normalized() {
        Schedule::Manual | Schedule::Daily { .. } | Schedule::Weekly { .. } => String::new(),
        Schedule::Days { days, times } => {
            let by = if days.len() == Weekday::ALL.len() {
                EVERY_DAY.to_owned()
            } else {
                let list = days.iter().fold(String::new(), |mut acc, d| {
                    let _ = write!(acc, "<{}/>", day_element(*d));
                    acc
                });
                format!(
                    "<ScheduleByWeek><DaysOfWeek>{list}</DaysOfWeek><WeeksInterval>1</WeeksInterval></ScheduleByWeek>"
                )
            };
            times.iter().map(|t| calendar(*t, &by, "")).collect()
        }
        Schedule::Hours { hours, from, until } => {
            let span = repetition_minutes(from, until);
            let repetition = format!(
                "\n      <Repetition>\n        <Interval>PT{hours}H</Interval>\n        \
                 <Duration>PT{span}M</Duration>\n        <StopAtDurationEnd>false</StopAtDurationEnd>\n      \
                 </Repetition>"
            );
            calendar(from, EVERY_DAY, &repetition)
        }
        Schedule::Monthly { day, at } => {
            let months = MONTHS.iter().fold(String::new(), |mut acc, m| {
                let _ = write!(acc, "<{m}/>");
                acc
            });
            let by = format!(
                "<ScheduleByMonth><DaysOfMonth><Day>{day}</Day></DaysOfMonth><Months>{months}</Months></ScheduleByMonth>"
            );
            calendar(at, &by, "")
        }
    }
}

/// Minutes from `from` to `until` (the next day when `until` is earlier);
/// a whole day when they are equal. Repetitions start strictly before the
/// end, so a run at `until` itself does not happen.
fn repetition_minutes(from: TimeOfDay, until: TimeOfDay) -> u16 {
    const DAY: u16 = 24 * 60;
    let (a, b) = (from.minutes(), until.minutes());
    if b > a { b - a } else { DAY - a + b }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The Partition/Diagnostic log records event 1006 whenever a disk is
/// connected; it is enabled by default since Windows 10.
const DRIVE_CONNECTED_QUERY: &str = r#"<QueryList><Query Id="0" Path="Microsoft-Windows-Partition/Diagnostic"><Select Path="Microsoft-Windows-Partition/Diagnostic">*[System[(EventID=1006)]]</Select></Query></QueryList>"#;

/// Triggers of the event task.
fn event_triggers(events: &EventTriggers, user: &str) -> String {
    let mut out = String::new();
    if events.at_logon {
        out.push_str(&logon(user, "PT5M"));
    }
    if events.when_idle {
        out.push_str("    <IdleTrigger>\n      <Enabled>true</Enabled>\n    </IdleTrigger>\n");
    }
    if events.drive_connected {
        // The drive needs a moment to get its letter.
        let _ = write!(
            out,
            "    <EventTrigger>\n      <Enabled>true</Enabled>\n      <Subscription>{}</Subscription>\n      \
             <Delay>PT1M</Delay>\n    </EventTrigger>\n",
            escape(DRIVE_CONNECTED_QUERY)
        );
    }
    out
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
        match task_xml(spec) {
            Some(xml) => create_task(&task_name(spec.profile_id, spec.kind), &xml),
            None => self.remove(spec.profile_id, spec.kind),
        }
    }

    fn remove(&self, id: ProfileId, kind: TaskKind) -> Result<(), SchedulerError> {
        delete_task(&task_name(id, kind)).map(|_| ())
    }
}

impl WindowsTaskScheduler {
    /// Executable started by the registered task of `id` and `kind`;
    /// `Ok(None)` when there is no such task.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    pub fn registered_executable(
        &self,
        id: ProfileId,
        kind: TaskKind,
    ) -> Result<Option<PathBuf>, SchedulerError> {
        registered_executable_of(&task_name(id, kind))
    }

    /// Executable started by [`WATCH_TASK`]; `Ok(None)` when it does not exist.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    pub fn watch_executable(&self) -> Result<Option<PathBuf>, SchedulerError> {
        registered_executable_of(WATCH_TASK)
    }

    /// Creates or replaces [`WATCH_TASK`] when `wanted`, removes it otherwise.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    pub fn apply_watch(&self, wanted: bool, executable: &Path, user: &str) -> Result<(), SchedulerError> {
        if wanted {
            create_task(WATCH_TASK, &watch_task_xml(executable, user))
        } else {
            delete_task(WATCH_TASK).map(|_| ())
        }
    }

    /// Removes the task Archeion registered for `id`, which starts an
    /// executable that no longer exists. Returns true when there was one.
    ///
    /// # Errors
    /// See [`SchedulerError`].
    pub fn remove_legacy(&self, id: ProfileId, kind: TaskKind) -> Result<bool, SchedulerError> {
        delete_task(&legacy_task_name(id, kind))
    }
}

/// True when the task of `spec` must be registered again: a wanted task
/// that is missing or starts another executable (the program was moved,
/// or a portable copy runs from elsewhere), or an unwanted one that exists.
#[must_use]
pub fn needs_update(registered: Option<&Path>, spec: &TaskSpec) -> bool {
    needs_update_for(registered, spec.is_wanted(), &spec.executable)
}

/// [`needs_update`] for any task: `wanted` or not, starting `executable`.
#[must_use]
pub fn needs_update_for(registered: Option<&Path>, wanted: bool, executable: &Path) -> bool {
    match (wanted, registered) {
        (false, found) => found.is_some(),
        (true, None) => true,
        // Windows paths ignore case.
        (true, Some(path)) => !path.to_string_lossy().eq_ignore_ascii_case(&executable.to_string_lossy()),
    }
}

/// Text printed by schtasks: UTF-16 for `/XML` on most systems, else the
/// console code page (read as UTF-8, which covers the ASCII of the XML).
fn decode_output(bytes: &[u8]) -> String {
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || bytes.get(1) == Some(&0);
    if utf16 {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| u16::from_le_bytes(pair))
            .skip_while(|&u| u == 0xFEFF)
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// The program of the first `<Command>` of a task's XML, unquoted.
fn command_from_xml(xml: &str) -> Option<String> {
    let start = xml.find("<Command>")? + "<Command>".len();
    let len = xml.get(start..)?.find("</Command>")?;
    let raw = xml.get(start..start + len)?.trim();
    let text = raw
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    let text = text.trim().trim_matches('"').trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// Registers (or replaces) the task `name` from its XML.
fn create_task(name: &str, xml: &str) -> Result<(), SchedulerError> {
    // schtasks wants a file; UTF-16 LE with BOM matches the declaration.
    let mut file = tempfile::Builder::new().suffix(".xml").tempfile().map_err(SchedulerError::Launch)?;
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    std::io::Write::write_all(&mut file, &bytes).map_err(SchedulerError::Launch)?;
    // Closed before schtasks reads it: Windows refuses to open a file that
    // another process holds ("the file is used by another process"). The
    // path deletes the file when dropped, after schtasks has finished.
    let path = file.into_temp_path();
    schtasks(&["/Create", "/TN", name, "/XML", &path.display().to_string(), "/F"])
}

fn registered_executable_of(name: &str) -> Result<Option<PathBuf>, SchedulerError> {
    match schtasks_output(&["/Query", "/TN", name, "/XML"]) {
        Ok(bytes) => Ok(command_from_xml(&decode_output(&bytes)).map(PathBuf::from)),
        Err(SchedulerError::Rejected(_)) => Ok(None), // no such task
        Err(e) => Err(e),
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

fn schtasks(args: &[&str]) -> Result<(), SchedulerError> {
    schtasks_output(args).map(|_| ())
}

/// Runs schtasks.exe without a console window; its standard output.
#[cfg(windows)]
fn schtasks_output(args: &[&str]) -> Result<Vec<u8>, SchedulerError> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(SchedulerError::Launch)?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(SchedulerError::Rejected(String::from_utf8_lossy(&out.stderr).trim().to_owned()))
    }
}

#[cfg(not(windows))]
fn schtasks_output(_args: &[&str]) -> Result<Vec<u8>, SchedulerError> {
    Err(SchedulerError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(schedule: Schedule) -> TaskSpec {
        TaskSpec {
            profile_id: "6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f".parse().unwrap(),
            profile_name: "Docs & <Photos>".into(),
            kind: TaskKind::Backup,
            schedule,
            events: EventTriggers::default(),
            executable: r"C:\Program Files\Archeion\archeion.exe".into(),
            user: r"PC\user".into(),
        }
    }

    fn t(h: u8, m: u8) -> TimeOfDay {
        TimeOfDay::new(h, m).unwrap()
    }

    #[test]
    fn several_times_on_chosen_days_give_one_trigger_per_time() {
        let s =
            Schedule::Days { days: vec![Weekday::Friday, Weekday::Monday], times: vec![t(18, 0), t(9, 30)] };
        let xml = task_xml(&spec(s)).unwrap();
        assert_eq!(xml.matches("<CalendarTrigger>").count(), 2);
        assert!(xml.contains("<StartBoundary>2026-01-01T09:30:00</StartBoundary>"));
        assert!(xml.contains("<StartBoundary>2026-01-01T18:00:00</StartBoundary>"));
        assert!(xml.contains("<DaysOfWeek><Monday/><Friday/></DaysOfWeek>"));
        let every = Schedule::Days { days: Weekday::ALL.to_vec(), times: vec![t(9, 0)] };
        assert!(task_xml(&spec(every)).unwrap().contains("<ScheduleByDay>"));
    }

    #[test]
    fn every_few_hours_repeats_within_the_window() {
        let s = Schedule::Hours { hours: 3, from: t(8, 0), until: t(20, 0) };
        let xml = task_xml(&spec(s)).unwrap();
        assert!(xml.contains("<Interval>PT3H</Interval>"));
        assert!(xml.contains("<Duration>PT720M</Duration>"));
        assert!(xml.contains("<StartBoundary>2026-01-01T08:00:00</StartBoundary>"));
        assert_eq!(repetition_minutes(t(22, 0), t(6, 0)), 480);
        assert_eq!(repetition_minutes(t(0, 0), t(0, 0)), 1440);
    }

    #[test]
    fn monthly_runs_on_the_chosen_day_of_every_month() {
        let xml = task_xml(&spec(Schedule::Monthly { day: 15, at: t(12, 0) })).unwrap();
        assert!(xml.contains("<DaysOfMonth><Day>15</Day></DaysOfMonth>"));
        assert!(xml.contains("<January/>") && xml.contains("<December/>"));
    }

    #[test]
    fn the_event_task_has_one_trigger_per_chosen_event() {
        let mut s = spec(Schedule::Manual);
        s.kind = TaskKind::Events;
        assert!(!s.is_wanted());
        assert_eq!(task_xml(&s), None);
        s.events =
            EventTriggers { at_logon: true, when_idle: true, drive_connected: true, min_hours_between: 4 };
        let xml = task_xml(&s).unwrap();
        assert!(xml.contains(r"<UserId>PC\user</UserId>"));
        assert!(xml.contains("<IdleTrigger>") && xml.contains("<IdleSettings>"));
        assert!(xml.contains("<EventTrigger>"));
        assert!(xml.contains("EventID=1006"));
        assert!(!xml.contains("<Select Path"), "the query is escaped inside the subscription");
        assert!(
            xml.contains(
                "<Arguments>run 6f1c3d1e-8a2b-4c3d-9e4f-0a1b2c3d4e5f --scheduled --event</Arguments>"
            )
        );
        assert!(!xml.contains("<CalendarTrigger>"));
        s.events.when_idle = false;
        assert!(!task_xml(&s).unwrap().contains("<IdleSettings>"));
    }

    #[test]
    fn the_watch_task_runs_daily_and_after_logon() {
        let xml = watch_task_xml(Path::new(r"D:\Archeion\archeion.exe"), r"PC\user");
        assert!(xml.contains("<Arguments>watch</Arguments>"));
        assert!(xml.contains("<StartBoundary>2026-01-01T12:00:00</StartBoundary>"));
        assert!(xml.contains("<Delay>PT15M</Delay>"));
        assert_eq!(command_from_xml(&xml).as_deref(), Some(r"D:\Archeion\archeion.exe"));
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
        assert_eq!(task_name(id, TaskKind::Events), format!(r"\Archeion\{id}-events"));
        assert_eq!(legacy_task_name(id, TaskKind::Merge), format!(r"\Archeion\{id}-merge"));
    }

    #[test]
    fn the_command_is_read_back_from_the_task_xml_in_either_encoding() {
        let xml = task_xml(&spec(Schedule::Daily { at: TimeOfDay::default() })).unwrap();
        let exe = r"C:\Program Files\Archeion\archeion.exe";
        assert_eq!(command_from_xml(&decode_output(xml.as_bytes())).as_deref(), Some(exe));
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(command_from_xml(&decode_output(&utf16)).as_deref(), Some(exe));
        let amp = "<Exec><Command>\"C:\\A &amp; B\\x.exe\"</Command></Exec>";
        assert_eq!(command_from_xml(amp).as_deref(), Some(r"C:\A & B\x.exe"));
        assert_eq!(command_from_xml("<Task/>"), None);
    }

    #[test]
    fn a_task_is_registered_again_when_missing_or_pointing_elsewhere() {
        let daily = spec(Schedule::Daily { at: TimeOfDay::default() });
        assert!(needs_update(None, &daily));
        assert!(!needs_update(Some(Path::new(r"c:\program files\ARCHEION\archeion.exe")), &daily));
        assert!(needs_update(Some(Path::new(r"D:\Portable\archeion.exe")), &daily));
        let manual = spec(Schedule::Manual);
        assert!(!needs_update(None, &manual));
        assert!(needs_update(Some(Path::new(r"D:\x.exe")), &manual));
        assert!(needs_update_for(None, true, Path::new(r"D:\x.exe")));
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
