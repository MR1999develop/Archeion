//! A profile as text: every rule of a profile (folders, exclusions,
//! destination, format, schedule, merge, checks, notifications) in readable
//! TOML. It is the content of a profile file exported by the user
//! (`Documents.archeion-profile`) and of the copy each backup keeps at its
//! destination. Never the password: profiles do not hold it.

use crate::Profile;
use serde::{Deserialize, Serialize};

/// Version of the text layout; a text of a later one is refused.
pub const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct ProfileText {
    format: u32,
    profile: Profile,
}

/// Why a text is not a profile this version can read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("the text is not an Archeion profile")]
    NotAProfile,
    #[error("the profile was written by a later version of Archeion (format {0})")]
    Newer(u32),
}

/// The text of `profile`, starting with a comment that names it.
///
/// # Errors
/// When the profile cannot be written as TOML (none of its fields should
/// make it fail).
pub fn encode(profile: &Profile) -> Result<String, toml::ser::Error> {
    let body = toml::to_string_pretty(&ProfileText { format: FORMAT, profile: profile.clone() })?;
    Ok(format!("# Archeion profile \"{}\". The password is not in this file.\n\n{body}", profile.name))
}

/// The profile in `text`, as [`encode`] wrote it.
///
/// # Errors
/// [`DecodeError::Newer`] for a text of a later format, otherwise
/// [`DecodeError::NotAProfile`] for anything that is not a profile.
pub fn decode(text: &str) -> Result<Profile, DecodeError> {
    #[derive(Deserialize)]
    struct Head {
        format: Option<u32>,
    }
    let head: Head = toml::from_str(text).map_err(|_| DecodeError::NotAProfile)?;
    match head.format {
        None => Err(DecodeError::NotAProfile),
        Some(f) if f > FORMAT => Err(DecodeError::Newer(f)),
        Some(_) => {
            let parsed: ProfileText = toml::from_str(text).map_err(|_| DecodeError::NotAProfile)?;
            Ok(parsed.profile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Destination, Exclusion, Schedule, TimeOfDay};

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
    fn every_rule_comes_back_from_the_text() {
        let p = sample();
        let text = encode(&p).unwrap();
        assert!(text.starts_with("# Archeion profile \"Documenti\""), "{text}");
        assert_eq!(decode(&text).unwrap(), p);
    }

    #[test]
    fn other_texts_and_later_formats_are_refused() {
        assert_eq!(decode("not = [toml"), Err(DecodeError::NotAProfile));
        assert_eq!(decode("language = \"Italian\"\n"), Err(DecodeError::NotAProfile), "settings file");
        assert_eq!(decode("format = 1\n"), Err(DecodeError::NotAProfile), "no profile");
        let later = encode(&sample()).unwrap().replace("format = 1", "format = 2");
        assert_eq!(decode(&later), Err(DecodeError::Newer(2)));
    }
}
