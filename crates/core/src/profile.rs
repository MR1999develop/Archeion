//! Backup profiles: what to save, where, when, and when to merge.

use crate::{Destination, Exclusion, FullBackupRule, Retention, Schedule, TimeOfDay, Weekday};
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

/// When old archives are merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MergePolicy {
    /// Automatic merge; `Manual` = only with "Merge now".
    pub schedule: Schedule,
    /// Which archives survive.
    pub rules: Retention,
}

impl Default for MergePolicy {
    /// Every Sunday, keeping the default rules.
    fn default() -> Self {
        Self {
            schedule: Schedule::Weekly { day: Weekday::Sunday, at: TimeOfDay::default() },
            rules: Retention::default(),
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
    /// Automatic backups.
    #[serde(default)]
    pub schedule: Schedule,
    /// When automatic backups are complete instead of incremental.
    #[serde(default)]
    pub full_backup: FullBackupRule,
    #[serde(default)]
    pub merge: MergePolicy,
    #[serde(default)]
    pub verification: Verification,
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
            schedule: Schedule::default(),
            full_backup: FullBackupRule::default(),
            merge: MergePolicy::default(),
            verification: Verification::default(),
        }
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
        let back: Profile = serde_json::from_value(v).unwrap();
        assert!(!back.encrypted);
        assert_eq!(back.full_backup, FullBackupRule::Never);
    }

    #[test]
    fn profile_id_round_trips_through_text() {
        let id = ProfileId::new_random();
        assert_eq!(id.to_string().parse::<ProfileId>().unwrap(), id);
    }

    #[test]
    fn default_merge_runs_weekly() {
        assert!(MergePolicy::default().schedule.is_automatic());
    }
}
