//! Backup profiles: what to save, where, when, and when to merge.

use crate::{Destination, Exclusion, FullBackupRule, LargeFiles, Retention, Schedule};
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf, str::FromStr};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MergePolicy {
    /// Which archives survive.
    #[serde(default)]
    pub rules: Retention,
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
            excludes: Vec::new(),
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
        }
    }

    /// True when backups start without the user: by the clock or by an
    /// event.
    #[must_use]
    pub fn backs_up_automatically(&self) -> bool {
        self.schedule.is_automatic() || self.events.any()
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
        } else if self.sources.iter().any(|s| self.destination.path().starts_with(s)) {
            errors.push(ProfileError::DestinationInsideSource);
        }
        if self.encrypted && self.container != Container::ZipPerFile {
            errors.push(ProfileError::EncryptionNeedsZip);
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }

    #[test]
    fn encryption_needs_zips() {
        let mut p = valid();
        p.encrypted = true;
        assert_eq!(p.validate(), Ok(()));
        p.container = Container::Folder;
        assert_eq!(p.validate(), Err(vec![ProfileError::EncryptionNeedsZip]));
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
