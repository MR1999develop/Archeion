//! A profile as a file of its own (`Documents.archeion-profile`): every
//! rule of the profile (folders, exclusions, destination, format,
//! schedule, merge, checks, notifications) in readable TOML, to keep it or
//! take it to another computer. Never the password: it stays in Windows
//! Credential Manager and is asked for again after an import.

use archeion_core::{Profile, ProfileId, Settings};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read as _, path::Path};

/// Extension of profile files, without the dot.
pub const EXTENSION: &str = "archeion-profile";
/// Version of the file layout; a file of a later one is refused.
const FORMAT: u32 = 1;
/// A profile file is a few kilobytes; anything far larger is not one.
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct ProfileFile {
    format: u32,
    profile: Profile,
}

/// Why a file could not be imported.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("reading the file: {0}")]
    Read(#[from] std::io::Error),
    #[error("the file is not an Archeion profile")]
    NotAProfile,
    #[error("the file was made by a later version of Archeion (format {0})")]
    Newer(u32),
}

/// The text of the file for `profile`.
pub fn encode(profile: &Profile) -> anyhow::Result<String> {
    let file = ProfileFile { format: FORMAT, profile: profile.clone() };
    let body = toml::to_string_pretty(&file)?;
    Ok(format!("# Archeion profile \"{}\". The password is not in this file.\n\n{body}", profile.name))
}

/// The profile in `text`.
pub fn decode(text: &str) -> Result<Profile, ImportError> {
    #[derive(Deserialize)]
    struct Head {
        format: Option<u32>,
    }
    let head: Head = toml::from_str(text).map_err(|_| ImportError::NotAProfile)?;
    match head.format {
        None => Err(ImportError::NotAProfile),
        Some(f) if f > FORMAT => Err(ImportError::Newer(f)),
        Some(_) => {
            let file: ProfileFile = toml::from_str(text).map_err(|_| ImportError::NotAProfile)?;
            Ok(file.profile)
        }
    }
}

/// Writes `profile` to `path`, replacing it.
pub fn export(path: &Path, profile: &Profile) -> anyhow::Result<()> {
    fs::write(path, encode(profile)?)?;
    Ok(())
}

/// Reads the profile in `path`.
pub fn import(path: &Path) -> Result<Profile, ImportError> {
    let mut text = String::new();
    fs::File::open(path)?.take(MAX_BYTES + 1).read_to_string(&mut text).map_err(|e| {
        // Not UTF-8: not a profile file.
        if e.kind() == std::io::ErrorKind::InvalidData { ImportError::NotAProfile } else { e.into() }
    })?;
    if text.len() as u64 > MAX_BYTES {
        return Err(ImportError::NotAProfile);
    }
    decode(&text)
}

/// Adds an imported profile to `settings` and returns its id. It keeps its
/// id, so on another computer it finds its own backups at the destination;
/// a profile already here with that id (the same file imported twice)
/// makes it a copy, with a new id and a name of its own.
///
/// Its schedule does not run until the user turns it on: a file from
/// elsewhere decides what is backed up and where.
pub fn add(settings: &mut Settings, mut profile: Profile) -> ProfileId {
    profile.schedule_paused = profile.schedule.is_automatic() || profile.events.any();
    // Folders on other computers wait for the user to confirm them.
    profile.network_unconfirmed = !profile.network_hosts().is_empty();
    if settings.profile(profile.id).is_some() {
        profile.id = ProfileId::new_random();
    }
    profile.name = free_name(settings, &profile.name);
    let id = profile.id;
    settings.profiles.push(profile);
    id
}

/// `name`, or `name (2)`, `name (3)`… when a profile already has it.
fn free_name(settings: &Settings, name: &str) -> String {
    let taken = |n: &str| settings.profiles.iter().any(|p| p.name == n);
    if !taken(name) {
        return name.to_owned();
    }
    (2..).map(|i| format!("{name} ({i})")).find(|n| !taken(n)).unwrap_or_else(|| name.to_owned())
}

/// A file name for `profile_name`: characters Windows refuses replaced.
pub fn file_name(profile_name: &str) -> String {
    let clean: String = profile_name
        .trim()
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let stem = clean.trim_end_matches(['.', ' ']);
    format!("{}.{EXTENSION}", if stem.is_empty() { "profile" } else { stem })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{Destination, Exclusion, Schedule, TimeOfDay};

    fn sample() -> Profile {
        let mut p = Profile::new("Documenti");
        p.sources.push(r"C:\Users\someone\Documents".into());
        p.excludes.push(Exclusion::Extension { ext: "tmp".into() });
        p.destination = Destination::Network { unc: r"\\server\backup\pc".into() };
        p.encrypted = true;
        p.schedule = Schedule::Monthly { day: 3, at: TimeOfDay::new(21, 30).unwrap() };
        p
    }

    #[test]
    fn every_rule_comes_back_from_the_file() {
        let p = sample();
        let text = encode(&p).unwrap();
        assert!(text.starts_with("# Archeion profile"), "{text}");
        assert_eq!(decode(&text).unwrap(), p);
    }

    #[test]
    fn the_file_round_trips_through_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(file_name("Documenti"));
        let p = sample();
        export(&path, &p).unwrap();
        assert_eq!(import(&path).unwrap(), p);
    }

    #[test]
    fn other_files_and_later_formats_are_refused() {
        assert!(matches!(decode("not = [toml"), Err(ImportError::NotAProfile)));
        assert!(matches!(decode("language = \"Italian\"\n"), Err(ImportError::NotAProfile)), "settings file");
        let later = encode(&sample()).unwrap().replace("format = 1", "format = 2");
        assert!(matches!(decode(&later), Err(ImportError::Newer(2))));
        let dir = tempfile::tempdir().unwrap();
        let big = dir.path().join("big.archeion-profile");
        fs::write(&big, "#".repeat(2 * 1024 * 1024)).unwrap();
        assert!(matches!(import(&big), Err(ImportError::NotAProfile)));
        let binary = dir.path().join("photo.archeion-profile");
        fs::write(&binary, [0xFF, 0xD8, 0xFF, 0xE0]).unwrap();
        assert!(matches!(import(&binary), Err(ImportError::NotAProfile)));
    }

    #[test]
    fn an_import_keeps_its_id_unless_the_profile_is_already_here() {
        let mut settings = Settings::default();
        let p = sample();
        assert_eq!(add(&mut settings, p.clone()), p.id, "another computer: the same profile");
        let copy = add(&mut settings, p.clone());
        assert_ne!(copy, p.id, "imported twice: a copy");
        assert_eq!(settings.profile(copy).unwrap().name, "Documenti (2)");
        let third = add(&mut settings, p);
        assert_eq!(settings.profile(third).unwrap().name, "Documenti (3)");
    }

    #[test]
    fn an_imported_schedule_waits_to_be_turned_on() {
        let mut settings = Settings::default();
        let p = sample();
        assert!(p.schedule.is_automatic());
        let id = add(&mut settings, p);
        let imported = settings.profile(id).unwrap();
        assert!(imported.schedule_paused && !imported.backs_up_automatically());
        assert!(imported.schedule.is_automatic(), "kept for when it is turned on");

        let mut manual = sample();
        manual.schedule = Schedule::Manual;
        let id = add(&mut settings, manual);
        assert!(!settings.profile(id).unwrap().schedule_paused, "nothing to wait for");
    }

    #[test]
    fn an_imported_profile_with_network_folders_waits_for_their_confirmation() {
        let mut settings = Settings::default();
        let mut p = sample();
        p.destination = archeion_core::Destination::Network { unc: r"\\server\backup".into() };
        let id = add(&mut settings, p);
        assert!(settings.profile(id).unwrap().network_unconfirmed);

        let mut local = sample();
        local.destination = archeion_core::Destination::Local { path: r"E:\Backup".into() };
        local.sources = vec![r"C:\Users\me".into()];
        let id = add(&mut settings, local);
        assert!(!settings.profile(id).unwrap().network_unconfirmed, "nothing on other computers");
    }

    #[test]
    fn file_names_have_no_character_windows_refuses() {
        assert_eq!(file_name("Documenti"), "Documenti.archeion-profile");
        assert_eq!(file_name(r#"A/B:C*?"<>|"#), "A_B_C______.archeion-profile");
        assert_eq!(file_name("  "), "profile.archeion-profile");
        assert_eq!(file_name("Foto. "), "Foto.archeion-profile");
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        /// The text of a real profile file with some bytes changed.
        fn damaged_file() -> impl Strategy<Value = String> {
            let text = encode(&sample()).unwrap();
            prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..6).prop_map(move |cuts| {
                let mut bytes = text.clone().into_bytes();
                for (at, b) in cuts {
                    let i = at.index(bytes.len());
                    bytes[i] = b;
                }
                String::from_utf8_lossy(&bytes).into_owned()
            })
        }

        proptest! {
            #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

            /// Whatever a file holds, an imported profile never runs on its
            /// own and never reaches another computer before the user says so.
            #[test]
            fn imported_profiles_wait_for_the_user(text in prop_oneof![damaged_file(), "\\PC{0,300}"]) {
                if let Ok(profile) = decode(&text) {
                    let mut settings = Settings::default();
                    let id = add(&mut settings, profile);
                    let p = settings.profile(id).unwrap();
                    prop_assert!(!p.backs_up_automatically(), "{p:?}");
                    prop_assert_eq!(p.network_unconfirmed, !p.network_hosts().is_empty());
                }
            }

            /// A file name never holds a character Windows refuses, nor
            /// ends its name with a dot or a space.
            #[test]
            fn file_names_are_valid(name in "\\PC{0,40}|[ .<>:\"/\\\\|?*a]{0,12}|[\\x00-\\x1f .a]{1,8}") {
                let file = file_name(&name);
                let stem = file.strip_suffix(&format!(".{EXTENSION}")).unwrap();
                prop_assert!(!stem.is_empty() && !stem.ends_with(['.', ' ']), "{file:?}");
                prop_assert!(!stem.chars().any(|c| c.is_control() || r#"<>:"/\|?*"#.contains(c)), "{file:?}");
            }
        }
    }
}
