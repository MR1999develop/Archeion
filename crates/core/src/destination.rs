//! Where a profile stores its archives.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Folder holding the archives of one profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Destination {
    /// A folder on an internal or external drive.
    Local { path: PathBuf },
    /// A Windows network share, for example `\\server\backup\laptop`.
    Network { unc: String },
}

impl Default for Destination {
    fn default() -> Self {
        Self::Local { path: PathBuf::new() }
    }
}

impl Destination {
    /// Folder that holds the archives.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Local { path } => path,
            Self::Network { unc } => Path::new(unc),
        }
    }

    /// True when the user has not filled in the location yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Local { path } => path.as_os_str().is_empty(),
            Self::Network { unc } => unc.trim().is_empty(),
        }
    }

    /// Text for messages.
    #[must_use]
    pub fn display(&self) -> String {
        self.path().display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_blank_locations_are_empty() {
        assert!(Destination::default().is_empty());
        assert!(Destination::Network { unc: "  ".into() }.is_empty());
        assert!(!Destination::Local { path: "E:\\Backup".into() }.is_empty());
    }

    #[test]
    fn network_path_is_the_unc_text() {
        let d = Destination::Network { unc: r"\\server\b".into() };
        assert_eq!(d.path(), Path::new(r"\\server\b"));
    }
}
