//! Where settings and logs live, and how they are written.
//!
//! Installed copy: `%APPDATA%\Archeion\config\settings.toml` (moved there
//! from `%APPDATA%\Archeion\config`, the program's first name, at the first
//! launch). Portable copy:
//! a `portable.txt` file next to the executable moves everything to a `data`
//! folder beside it.

use anyhow::Context as _;
use archeion_core::Settings;
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings.toml";
const LOG_FILE: &str = "archeion.log";
/// Above this size the log is restarted at the next launch.
const LOG_LIMIT_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    /// Chooses the portable or installed data folder and creates it.
    pub fn locate() -> anyhow::Result<Self> {
        let exe = std::env::current_exe().context("finding the executable")?;
        let exe_dir = exe.parent().context("executable has no folder")?;
        let dir = if exe_dir.join("portable.txt").exists() {
            exe_dir.join("data")
        } else {
            let dirs = |name| {
                directories::ProjectDirs::from("", "", name)
                    .map(|d| d.config_dir().to_path_buf())
                    .context("no user profile folder")
            };
            let dir = dirs("Archeion")?;
            move_legacy_folder(&dirs("Archeion")?, &dir);
            dir
        };
        Self::at(dir)
    }

    /// True once, the first time it is asked: for one-off migrations that
    /// leave a marker file named `name` in the data folder.
    pub fn first_time(&self, name: &str) -> bool {
        let marker = self.dir.join(name);
        if marker.exists() {
            return false;
        }
        if let Err(e) = fs::write(&marker, b"") {
            tracing::warn!(error = %e, "writing {}", marker.display());
        }
        true
    }

    /// Storage in an explicit folder (tests).
    pub fn at(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Saved settings; a missing file gives the defaults.
    pub fn load(&self) -> anyhow::Result<Settings> {
        let path = self.dir.join(SETTINGS_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(e).with_context(|| format!("opening {}", path.display())),
        }
    }

    /// Writes the settings atomically: temporary file, flush to disk, rename.
    pub fn save(&self, settings: &Settings) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(settings).context("encoding settings")?;
        let path = self.dir.join(SETTINGS_FILE);
        let tmp = self.dir.join(format!("{SETTINGS_FILE}.tmp"));
        let mut file = fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        file.write_all(text.as_bytes()).context("writing settings")?;
        file.sync_all().context("flushing settings")?;
        drop(file);
        fs::rename(&tmp, &path).with_context(|| format!("replacing {}", path.display()))
    }

    /// Log file in append mode, restarted when it grew past the limit.
    pub fn open_log(&self) -> std::io::Result<fs::File> {
        let path = self.dir.join(LOG_FILE);
        let too_big = fs::metadata(&path).is_ok_and(|m| m.len() > LOG_LIMIT_BYTES);
        fs::OpenOptions::new().create(true).append(!too_big).write(true).truncate(too_big).open(path)
    }
}

/// Moves the data folder of Archeion to `new` when `new` does not exist
/// yet; on failure the old folder stays and is simply no longer used.
fn move_legacy_folder(old: &Path, new: &Path) {
    if new.exists() || !old.exists() {
        return;
    }
    let moved = new.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::rename(old, new));
    match moved {
        Ok(()) => {
            tracing::info!(from = %old.display(), to = %new.display(), "data folder moved");
            // `%APPDATA%\Archeion` is left empty: remove it if so.
            if let Some(parent) = old.parent() {
                let _ = fs::remove_dir(parent);
            }
        }
        Err(e) => tracing::warn!(error = %e, from = %old.display(), "moving the old data folder"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::Profile;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::at(dir.path().join("cfg")).unwrap();
        assert_eq!(s.load().unwrap(), Settings::default());
    }

    #[test]
    fn saved_settings_load_back() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::at(dir.path().to_path_buf()).unwrap();
        let mut settings = Settings::default();
        settings.profiles.push(Profile::new("Documenti"));
        s.save(&settings).unwrap();
        assert_eq!(s.load().unwrap(), settings);
        assert!(!dir.path().join("settings.toml.tmp").exists());
    }

    #[test]
    fn the_old_data_folder_moves_once_and_never_over_a_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("Archeion").join("config");
        let new = dir.path().join("Archeion").join("config");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(SETTINGS_FILE), "x").unwrap();
        move_legacy_folder(&old, &new);
        assert_eq!(fs::read_to_string(new.join(SETTINGS_FILE)).unwrap(), "x");
        assert!(!dir.path().join("Archeion").exists());

        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(SETTINGS_FILE), "older").unwrap();
        move_legacy_folder(&old, &new);
        assert_eq!(fs::read_to_string(new.join(SETTINGS_FILE)).unwrap(), "x");
    }

    #[test]
    fn a_marker_answers_true_only_the_first_time() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::at(dir.path().to_path_buf()).unwrap();
        assert!(s.first_time("done"));
        assert!(!s.first_time("done"));
    }

    #[test]
    fn damaged_file_is_an_error_not_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::at(dir.path().to_path_buf()).unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "profiles = 3").unwrap();
        assert!(s.load().is_err());
    }
}
