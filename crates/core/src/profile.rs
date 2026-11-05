//! Backup profiles: what to save, where, when, and when to merge.

use crate::{ArchiveName, Destination, Exclusion, FullBackupRule, LargeFiles, Retention, Schedule};
use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};
use uuid::Uuid;

/// Stable identifier of a profile; also the name of its scheduled tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(Uuid);

impl ProfileId {
    #[must_use]
    pub fn new_random() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ProfileId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// How the content of new archives is stored. Existing archives keep their
/// own container; a profile can switch at any time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// Every file compressed on its own, `report.docx` -> `report.docx.zip`,
    /// in the original folder structure. Works on any drive; each file opens
    /// with File Explorer, 7-Zip or any zip tool.
    #[default]
    ZipPerFile,
    /// Plain copies: same names, no compression.
    Folder,
}

/// How old archives are merged.
///
/// The merge is always automatic: the window applies the rules while it
/// runs, whenever they would merge something and nothing else uses the
/// destination. Fields of older versions (`schedule`, `automatic`) are
/// ignored when read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergePolicy {
    /// The automatic merge runs. Off, every backup stays until the user
    /// merges or deletes it by hand. On unless the user turns it off.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Which archives survive.
    #[serde(default)]
    pub rules: Retention,
    /// A merged backup keeps the files deleted at the source between the
    /// backups it fuses, with their last content, instead of dropping
    /// them. On unless the user turns it off.
    #[serde(default = "yes")]
    pub keep_deleted: bool,
    /// The merge last confirmed, while the current one would merge backups
    /// that one kept: the automatic merge waits for the user
    /// ([`Self::set_rules`], [`Self::set_enabled`]). `None`: the current
    /// merge is confirmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<ConfirmedMerge>,
}

/// A merge the user confirmed: on or off, and its rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmedMerge {
    /// Rules confirmed before the switch existed were on.
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(flatten)]
    pub rules: Retention,
}

fn yes() -> bool {
    true
}

impl Default for MergePolicy {
    fn default() -> Self {
        Self { enabled: true, rules: Retention::default(), keep_deleted: true, approved: None }
    }
}

impl MergePolicy {
    /// The merge as it is now, to remember as confirmed.
    fn current(&self) -> ConfirmedMerge {
        ConfirmedMerge { enabled: self.enabled, rules: self.rules }
    }

    /// New rules chosen by the user, for the backups `archives` (oldest
    /// first) at local time `now`. When they would merge backups that the
    /// merge confirmed so far keeps, that merge is remembered and the
    /// automatic merge waits for [`Self::confirm`]; otherwise the change is
    /// confirmed at once. With `archives` unknown (the destination not
    /// read yet) nothing is confirmed.
    pub fn set_rules(&mut self, rules: Retention, archives: Option<&[ArchiveName]>, now: DateTime) {
        if rules != self.rules {
            self.approved.get_or_insert(self.current());
            self.rules = rules;
            self.settle(archives, now);
        }
    }

    /// The automatic merge turned on or off, as [`Self::set_rules`]:
    /// turning it off never waits, turning it on waits when its rules would
    /// merge backups.
    pub fn set_enabled(&mut self, enabled: bool, archives: Option<&[ArchiveName]>, now: DateTime) {
        if enabled != self.enabled {
            self.approved.get_or_insert(self.current());
            self.enabled = enabled;
            self.settle(archives, now);
        }
    }

    /// Confirms the change at once when it holds nothing back.
    fn settle(&mut self, archives: Option<&[ArchiveName]>, now: DateTime) {
        if archives.is_some_and(|a| !crate::merge_plan::preview(a, self, now).waits()) {
            self.approved = None;
        }
    }

    /// The user accepts what the current merge does.
    pub fn confirm(&mut self) {
        self.approved = None;
    }

    /// Back to the merge last confirmed (rules, and on or off).
    pub fn restore_approved(&mut self) {
        if let Some(c) = self.approved.take() {
            self.enabled = c.enabled;
            self.rules = c.rules;
        }
    }
}

/// Periodic checks that the archives can really be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Verification {
    /// Full check of every archive every N days; 0 = never.
    pub check_every_days: u32,
    /// Restore test of a random sample every N days; 0 = never.
    pub restore_test_every_days: u32,
    /// Number of files read back by the restore test.
    pub restore_test_files: u32,
}

impl Default for Verification {
    fn default() -> Self {
        Self { check_every_days: 30, restore_test_every_days: 90, restore_test_files: 20 }
    }
}

/// Backups started by an event instead of the clock. They have their own
/// scheduled task, and skip the run when the last backup is more recent
/// than `min_hours_between` (idle periods and logons can come often) or
/// when the destination is not there (another drive was plugged in).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EventTriggers {
    /// A few minutes after the user signs in to Windows.
    pub at_logon: bool,
    /// When the computer has been idle for a while.
    pub when_idle: bool,
    /// When a drive is connected (the destination's, if it is removable).
    pub drive_connected: bool,
    /// Event runs closer than this to the last backup are skipped.
    pub min_hours_between: u32,
}

impl Default for EventTriggers {
    fn default() -> Self {
        Self { at_logon: false, when_idle: false, drive_connected: false, min_hours_between: 4 }
    }
}

impl EventTriggers {
    /// Range of [`Self::min_hours_between`] in the editor.
    pub const MIN_HOURS_RANGE: std::ops::RangeInclusive<u32> = 0..=168;

    /// True when at least one event starts a backup.
    #[must_use]
    pub fn any(&self) -> bool {
        self.at_logon || self.when_idle || self.drive_connected
    }
}

/// Windows notifications about automatic backups (the window shows the
/// same in its operations log).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alerts {
    /// When a scheduled or event backup fails, or cannot reach a destination
    /// it should have reached at its scheduled time.
    pub on_failure: bool,
    /// When no backup has finished for this many days; 0 = never.
    pub stale_days: u32,
}

impl Default for Alerts {
    fn default() -> Self {
        Self { on_failure: true, stale_days: 0 }
    }
}

/// One backup job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    /// Folders to back up. Empty on a new profile: the user chooses.
    pub sources: Vec<PathBuf>,
    /// Folders, files, file types and patterns left out of the backup.
    #[serde(default)]
    pub excludes: Vec<Exclusion>,
    pub destination: Destination,
    #[serde(default)]
    pub container: Container,
    /// Encrypt new destinations: AES-256 zips with hidden file names. Like
    /// the container, it counts only until the first backup; the password
    /// is kept in Windows Credential Manager, never here.
    #[serde(default)]
    pub encrypted: bool,
    /// From which size files of an unencrypted destination are saved as
    /// chunks (on an encrypted one every file is). Can change at any time.
    #[serde(default)]
    pub large_files: LargeFiles,
    /// Automatic backups.
    #[serde(default)]
    pub schedule: Schedule,
    /// The schedule is kept but does not run: an imported profile waits for
    /// the user to look at it and turn it on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub schedule_paused: bool,
    /// Imported with folders on other computers (destination or sources on
    /// a share) that the user has not confirmed yet: nothing touches them
    /// until then, so a profile file from someone else cannot make Windows
    /// sign in to the computer it names.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub network_unconfirmed: bool,
    /// Backups started by events (logon, idle, drive connected).
    #[serde(default)]
    pub events: EventTriggers,
    /// When automatic backups are complete instead of incremental.
    #[serde(default)]
    pub full_backup: FullBackupRule,
    #[serde(default)]
    pub merge: MergePolicy,
    #[serde(default)]
    pub verification: Verification,
    #[serde(default)]
    pub alerts: Alerts,
}

/// A rule a profile breaks; every broken rule is reported, not just the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("the name is empty")]
    EmptyName,
    #[error("no folder to back up")]
    NoSources,
    #[error("no destination")]
    NoDestination,
    #[error("the destination is inside a folder being backed up")]
    DestinationInsideSource,
    #[error("encryption needs the zip per file format")]
    EncryptionNeedsZip,
}

impl Profile {
    /// A new, empty profile with the default schedule and merge policy.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProfileId::new_random(),
            name: name.into(),
            sources: Vec::new(),
            excludes: crate::exclusion::defaults(),
            destination: Destination::default(),
            container: Container::default(),
            encrypted: false,
            large_files: LargeFiles::default(),
            schedule: Schedule::default(),
            events: EventTriggers::default(),
            full_backup: FullBackupRule::default(),
            merge: MergePolicy::default(),
            verification: Verification::default(),
            alerts: Alerts::default(),
            schedule_paused: false,
            network_unconfirmed: false,
        }
    }

    /// A copy of this profile named `name`, with an id of its own, for the
    /// user to adapt: every rule comes along but the destination, which
    /// belongs to this profile alone and is left for the user to choose.
    /// Its schedule waits to be turned on, and nothing waits for a
    /// confirmation of the merge (it has no backups yet).
    #[must_use]
    pub fn copy_named(&self, name: impl Into<String>) -> Self {
        let mut copy = self.clone();
        copy.id = ProfileId::new_random();
        copy.name = name.into();
        copy.destination = Destination::default();
        copy.schedule_paused = copy.schedule.is_automatic() || copy.events.any();
        copy.merge.approved = None;
        copy
    }

    /// True when backups start without the user: by the clock or by an
    /// event.
    #[must_use]
    pub fn backs_up_automatically(&self) -> bool {
        !self.schedule_paused && (self.schedule.is_automatic() || self.events.any())
    }

    /// The computers this profile's folders may be on, each once, in the
    /// order met (destination, then sources): every folder that is not on a
    /// drive of this computer (see [`remote_part`]). Empty when all are.
    #[must_use]
    pub fn network_hosts(&self) -> Vec<String> {
        let mut hosts: Vec<String> = Vec::new();
        let paths = std::iter::once(self.destination.path()).chain(self.sources.iter().map(PathBuf::as_path));
        for host in paths.filter_map(remote_part) {
            if !hosts.iter().any(|h| h.eq_ignore_ascii_case(&host)) {
                hosts.push(host);
            }
        }
        hosts
    }

    /// Checks that the profile can run. Returns all broken rules.
    pub fn validate(&self) -> Result<(), Vec<ProfileError>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push(ProfileError::EmptyName);
        }
        if self.sources.is_empty() {
            errors.push(ProfileError::NoSources);
        }
        if self.destination.is_empty() {
            errors.push(ProfileError::NoDestination);
        } else if self.sources.iter().any(|s| path_within(self.destination.path(), s)) {
            errors.push(ProfileError::DestinationInsideSource);
        }
        if self.encrypted && self.container != Container::ZipPerFile {
            errors.push(ProfileError::EncryptionNeedsZip);
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

/// True when `path` is `base` or inside it, comparing names as Windows
/// does: `D:\data\backup` is inside `D:\Data`.
#[must_use]
pub fn path_within(path: &Path, base: &Path) -> bool {
    let folded = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| matches!(c, std::path::Component::Prefix(_) | std::path::Component::Normal(_)))
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    };
    let (path, base) = (folded(path), folded(base));
    !base.is_empty() && path.len() >= base.len() && path.iter().zip(&base).all(|(a, b)| a == b)
}

/// Where Windows may go to another computer to reach `path`: `None` only
/// for a folder of this one (a drive letter, `\\?\C:\…`, `\\.\C:\…`, a
/// relative path); otherwise the computer to name: `server` for
/// `\\server\x`, `//server/x`, `\\?\UNC\server\x`, `\\.\UNC\…` or
/// `\??\UNC\…`, and the whole path for any other device path
/// (`\\?\GLOBALROOT\…`, `\\?\Volume{…}\…`), which may lead anywhere.
/// Windows reads `/` as `\`, and so does this.
#[must_use]
pub fn remote_part(path: &Path) -> Option<String> {
    let text = path.to_string_lossy().replace('/', "\\");
    let is_drive = |rest: &str| {
        let b = rest.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b.len() == 2 || b[2] == b'\\')
    };
    let device = [r"\\?\", r"\\.\", r"\??\"].into_iter().find_map(|p| text.strip_prefix(p));
    let rest = match device {
        Some(rest) if is_drive(rest) => return None,
        Some(rest) => match rest.get(..4) {
            Some(unc) if unc.eq_ignore_ascii_case(r"UNC\") => rest.get(4..).unwrap_or_default(),
            _ => return Some(text),
        },
        None => text.strip_prefix(r"\\")?,
    };
    match rest.split('\\').next() {
        Some(host) if !host.is_empty() => Some(host.to_owned()),
        _ => Some(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spelling Windows takes for a folder on another computer needs
    /// the user's confirmation; folders on this computer's drives do not.
    #[test]
    fn any_way_to_another_computer_is_found() {
        let remote = |p: &str| remote_part(Path::new(p));
        for (path, host) in [
            (r"\\server\share\x", "server"),
            (r"//server/share/x", "server"),
            (r"\/server\share", "server"),
            (r"\\?\UNC\server\share", "server"),
            (r"\\?\unc\server\share", "server"),
            (r"\\.\UNC\server\share", "server"),
            (r"\??\UNC\server\share", "server"),
            (r"//?/UNC/server/share", "server"),
        ] {
            assert_eq!(remote(path).as_deref(), Some(host), "{path}");
        }
        for path in [r"\\?\GLOBALROOT\Device\Mup\server\share", r"\\?\Volume{0000}\x", r"\\.\pipe\x", r"\\\x"]
        {
            assert_eq!(remote(path).as_deref(), Some(path.replace('/', "\\").as_str()), "{path}");
        }
        for path in [r"C:\Users\me", r"\\?\C:\long", r"\\.\D:\x", r"\\?\E:", "relative", r"\rooted", "E:"] {
            assert_eq!(remote(path), None, "{path}");
        }
    }

    #[test]
    fn the_computers_of_network_folders_are_listed_once() {
        let mut p = Profile::new("Imported");
        p.destination = Destination::Network { unc: r"\\SERVER\backup\pc".into() };
        p.sources = vec![
            r"C:\Users\me\Docs".into(),
            r"\\server\photos".into(),
            r"\\?\UNC\server2\x".into(),
            r"\\?\C:\long".into(),
        ];
        assert_eq!(p.network_hosts(), vec!["SERVER".to_owned(), "server2".to_owned()]);
        p.destination = Destination::Local { path: r"E:\Backup".into() };
        p.sources = vec![r"C:\Users\me".into()];
        assert!(p.network_hosts().is_empty());
    }

    fn valid() -> Profile {
        let mut p = Profile::new("Documents");
        p.sources.push(r"C:\Users\me\Documents".into());
        p.destination = Destination::Local { path: r"E:\Backup".into() };
        p
    }

    #[test]
    fn valid_profile_passes() {
        assert_eq!(valid().validate(), Ok(()));
    }

    #[test]
    fn new_profile_reports_every_missing_part() {
        let errors = Profile::new(" ").validate().unwrap_err();
        assert_eq!(
            errors,
            vec![ProfileError::EmptyName, ProfileError::NoSources, ProfileError::NoDestination]
        );
    }

    #[test]
    fn destination_inside_a_source_is_rejected() {
        let mut p = valid();
        p.destination = Destination::Local { path: r"C:\Users\me\Documents\Backup".into() };
        assert_eq!(p.validate(), Err(vec![ProfileError::DestinationInsideSource]));
        // Windows names ignore case.
        let mut q = valid();
        q.sources = vec![r"D:\Data".into()];
        q.destination = Destination::Local { path: r"d:\data\Backup".into() };
        assert_eq!(q.validate(), Err(vec![ProfileError::DestinationInsideSource]), "case");
        q.destination = Destination::Local { path: r"D:\Data\.\backups".into() };
        assert_eq!(q.validate(), Err(vec![ProfileError::DestinationInsideSource]), "a dot");
        q.destination = Destination::Local { path: r"D:\Database".into() };
        assert_eq!(q.validate(), Ok(()), "a name that only begins the same");
    }

    #[test]
    fn encryption_needs_zips() {
        let mut p = valid();
        p.encrypted = true;
        assert_eq!(p.validate(), Ok(()));
        p.container = Container::Folder;
        assert_eq!(p.validate(), Err(vec![ProfileError::EncryptionNeedsZip]));
    }

    /// The copy keeps every rule but the destination, has its own id, and
    /// waits before running on its own.
    #[test]
    fn a_copy_keeps_the_rules_but_not_the_destination() {
        let mut p = Profile::new("Docs");
        p.sources.push(r"C:\Users\me\Documents".into());
        p.destination = Destination::Local { path: r"E:\Backup".into() };
        p.encrypted = true;
        p.container = Container::ZipPerFile;
        p.schedule = Schedule::Monthly { day: 3, at: crate::TimeOfDay::new(21, 30).unwrap() };
        p.merge.approved = Some(crate::ConfirmedMerge { enabled: true, rules: Retention::KEEP_ALL });
        let copy = p.copy_named("Docs (copy)");
        assert_ne!(copy.id, p.id);
        assert_eq!(copy.name, "Docs (copy)");
        assert!(copy.destination.is_empty(), "a folder of its own to choose");
        assert_eq!(
            (&copy.sources, copy.encrypted, copy.container),
            (&p.sources, true, Container::ZipPerFile)
        );
        assert_eq!(copy.schedule, p.schedule, "kept for when it is turned on");
        assert!(copy.schedule_paused && !copy.backs_up_automatically());
        assert_eq!(copy.merge.approved, None);
        assert!(copy.validate().is_err(), "not valid until it has a destination");

        let manual = Profile::new("Manual").copy_named("Manual (copy)");
        assert!(!manual.schedule_paused, "nothing to wait for");
    }

    #[test]
    fn older_profiles_load_without_the_new_fields() {
        let mut p = valid();
        p.encrypted = true;
        let mut v = serde_json::to_value(&p).unwrap();
        let map = v.as_object_mut().unwrap();
        map.remove("encrypted");
        map.remove("full_backup");
        map.remove("events");
        map.remove("alerts");
        map.remove("large_files");
        let back: Profile = serde_json::from_value(v).unwrap();
        assert_eq!(back.large_files, LargeFiles::default());
        assert!(!back.encrypted);
        assert_eq!(back.full_backup, FullBackupRule::Never);
        assert_eq!(back.events, EventTriggers::default());
        assert!(back.alerts.on_failure);
        assert!(!back.backs_up_automatically());
    }

    #[test]
    fn profile_id_round_trips_through_text() {
        let id = ProfileId::new_random();
        assert_eq!(id.to_string().parse::<ProfileId>().unwrap(), id);
    }

    #[test]
    fn old_merge_settings_still_load() {
        let old: MergePolicy = serde_json::from_str(
            r#"{"schedule":{"every":"weekly","day":"sunday","at":{"hour":12,"minute":30}},"automatic":false,"rules":{"keep_last":3}}"#,
        )
        .unwrap();
        assert_eq!(old.rules.keep_last, 3);
        assert_eq!(serde_json::from_str::<MergePolicy>("{}").unwrap(), MergePolicy::default());
        let text = serde_json::to_string(&MergePolicy::default()).unwrap();
        assert!(!text.contains("schedule") && !text.contains("automatic"));
    }
}
