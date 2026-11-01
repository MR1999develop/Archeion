//! Everything the application saves between sessions.

use crate::{Profile, ProfileId};
use serde::{Deserialize, Serialize};

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    /// Follow the Windows display language (Italian if it is Italian,
    /// otherwise English).
    #[default]
    System,
    English,
    Italian,
}

/// Saved application state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub profiles: Vec<Profile>,
}

impl Settings {
    #[must_use]
    pub fn profile(&self, id: ProfileId) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile_mut(&mut self, id: ProfileId) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_toml() {
        let mut s = Settings::default();
        let mut p = Profile::new("Photos");
        p.sources.push(r"C:\Users\me\Pictures".into());
        s.profiles.push(p);
        let text = toml::to_string_pretty(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
    }

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(toml::from_str::<Settings>("").unwrap(), Settings::default());
    }
}
