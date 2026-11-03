//! What this computer last saw at each profile's destination: the name of
//! the newest backup. A destination whose newest backup is older than that
//! was put back from an old copy, or had backups removed from outside: the
//! user is warned (the backups at the destination cannot tell, since an old
//! copy carries its own, matching signature).
//!
//! Kept in `destinations.json` in the data folder; losing it only loses the
//! warning.

use archeion_core::ProfileId;
use std::{collections::BTreeMap, fs, path::Path};

const FILE: &str = "destinations.json";

/// Newest backup name seen, by profile.
pub type Seen = BTreeMap<ProfileId, String>;

/// The memory in `dir`; empty when missing or unreadable.
pub fn load(dir: &Path) -> Seen {
    fs::read_to_string(dir.join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Remembers `newest` for `profile` (`None`: no backup left).
pub fn record(dir: &Path, profile: ProfileId, newest: Option<&str>) {
    let mut seen = load(dir);
    match newest {
        Some(name) => seen.insert(profile, name.to_owned()),
        None => seen.remove(&profile),
    };
    let text = serde_json::to_string_pretty(&seen).unwrap_or_default();
    let tmp = dir.join(format!("{FILE}.tmp"));
    let written = fs::write(&tmp, text).and_then(|()| archeion_win::rename_durably(&tmp, &dir.join(FILE)));
    if let Err(e) = written {
        tracing::warn!(error = %e, "remembering the newest backup");
    }
}

/// True when the destination's `newest` backup is older than the one
/// `remembered`, or gone while one was remembered. Names begin with their
/// date and time; the kind letter after them may change (a merge makes an
/// incremental complete).
pub fn went_back(remembered: Option<&str>, newest: Option<&str>) -> bool {
    let time = |name: &str| name.rsplit_once('_').map_or(name, |(t, _)| t).to_owned();
    match (remembered, newest) {
        (Some(r), Some(n)) => time(n) < time(r),
        (Some(_), None) => true,
        (None, _) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_newest_backup_or_none_at_all_is_going_back() {
        let (old, new) = ("2026-11-01_100000_I", "2026-11-03_100000_I");
        assert!(went_back(Some(new), Some(old)));
        assert!(went_back(Some(new), None), "all gone");
        assert!(!went_back(Some(old), Some(new)), "a newer backup");
        assert!(!went_back(Some(new), Some(new)));
        assert!(!went_back(Some(new), Some("2026-11-03_100000_C")), "merged into a complete");
        assert!(!went_back(None, Some(old)), "first time");
    }

    #[test]
    fn the_memory_survives_a_new_start() {
        let dir = tempfile::tempdir().unwrap();
        let id = ProfileId::new_random();
        assert!(load(dir.path()).is_empty());
        record(dir.path(), id, Some("2026-11-03_100000_I"));
        assert_eq!(load(dir.path()).get(&id).map(String::as_str), Some("2026-11-03_100000_I"));
        record(dir.path(), id, None);
        assert!(load(dir.path()).is_empty());
    }
}
