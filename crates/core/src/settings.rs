//! Everything the application saves between sessions.

use crate::{Profile, ProfileId};
use serde::{Deserialize, Serialize};

/// Interface language. Saved by variant name, so variants may be added but
/// never renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    /// Follow the Windows display language, English when it is none of the
    /// languages below.
    #[default]
    System,
    English,
    Italian,
    /// Mandarin Chinese, simplified characters.
    Chinese,
    Hindi,
    Spanish,
    French,
    Japanese,
    /// Brazilian Portuguese.
    Portuguese,
    Russian,
}

impl Language {
    /// The languages the user can pick, in display order; `System` is
    /// left out, since it is offered apart, before them.
    pub const ALL: [Self; 9] = [
        Self::English,
        Self::Italian,
        Self::Chinese,
        Self::Hindi,
        Self::Spanish,
        Self::French,
        Self::Japanese,
        Self::Portuguese,
        Self::Russian,
    ];

    /// The language's name in that language ("Italiano"), so that anyone
    /// finds their own whatever the current one; `None` for `System`,
    /// which the interface names in the current language.
    #[must_use]
    pub const fn native_name(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::English => Some("English"),
            Self::Italian => Some("Italiano"),
            Self::Chinese => Some("中文（简体）"),
            Self::Hindi => Some("हिन्दी"),
            Self::Spanish => Some("Español"),
            Self::French => Some("Français"),
            Self::Japanese => Some("日本語"),
            Self::Portuguese => Some("Português (Brasil)"),
            Self::Russian => Some("Русский"),
        }
    }
}

/// Saved application state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub profiles: Vec<Profile>,
    pub window: WindowOptions,
    /// The size the window had when last resized, opened again at start;
    /// `None` until the user resizes it.
    pub window_size: Option<WindowSize>,
}

/// Size of the main window, in logical pixels (as Windows scales them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u16,
    pub height: u16,
    /// Maximized: `width` and `height` are the size it comes back to.
    #[serde(default)]
    pub maximized: bool,
}

/// How the window behaves, chosen in its Preferences window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowOptions {
    /// Closing the window hides it in the notification area, where
    /// Archeion keeps merging old backups and showing scheduled runs.
    pub close_to_tray: bool,
    /// Archeion starts, hidden in the notification area, when the user
    /// signs in to Windows.
    pub start_with_windows: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self { close_to_tray: true, start_with_windows: true }
    }
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
    fn every_selectable_language_has_a_name() {
        assert_eq!(Language::System.native_name(), None);
        for l in Language::ALL {
            assert_ne!(l, Language::System);
            assert!(l.native_name().is_some(), "{l:?} has no name");
        }
    }

    #[test]
    fn every_language_round_trips_through_toml() {
        for language in std::iter::once(Language::System).chain(Language::ALL) {
            let s = Settings { language, ..Settings::default() };
            let text = toml::to_string_pretty(&s).unwrap();
            assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s, "{text}");
        }
    }

    #[test]
    fn languages_saved_before_keep_their_names() {
        for (text, language) in [
            ("language = \"System\"", Language::System),
            ("language = \"English\"", Language::English),
            ("language = \"Italian\"", Language::Italian),
        ] {
            assert_eq!(toml::from_str::<Settings>(text).unwrap().language, language);
        }
    }

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(toml::from_str::<Settings>("").unwrap(), Settings::default());
    }
}
