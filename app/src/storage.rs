//! Where settings and logs live, and how they are written.
//!
//! Installed copy: `%APPDATA%\Archeion\config\settings.toml`. Portable
//! copy: a `portable.txt` file next to the executable moves everything to a
//! `data` folder beside it.

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
            directories::ProjectDirs::from("", "", "Archeion")
                .map(|d| d.config_dir().to_path_buf())
                .context("no user profile folder")?
        };
        Self::at(dir)
    }

    /// Storage in an explicit folder (tests).
    pub fn at(dir: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Folder of the run logs (it exists once the first run wrote one).
    pub fn logs_dir(&self) -> PathBuf {
        self.dir.join(crate::history::LOG_DIR)
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
        let path = program_log_path(&self.dir);
        let too_big = fs::metadata(&path).is_ok_and(|m| m.len() > LOG_LIMIT_BYTES);
        fs::OpenOptions::new().create(true).append(!too_big).write(true).truncate(too_big).open(path)
    }
}

/// The program's diagnostic log in the data folder `dir` (written through
/// `tracing` by the window and by every worker).
pub fn program_log_path(dir: &Path) -> PathBuf {
    dir.join(LOG_FILE)
}

/// Restore requests older than this are leftovers of a worker that never
/// started (or crashed before reading its request).
const STALE_REQUEST: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Deletes `restore-*.json` files of `dir` older than a day. The window
/// writes one per restore and the worker deletes it once read.
pub fn remove_stale_requests(dir: &Path) {
    remove_requests_older_than(dir, STALE_REQUEST);
}

fn remove_requests_older_than(dir: &Path, age: std::time::Duration) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("restore-") && name.ends_with(".json")) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| now.duration_since(t).is_ok_and(|d| d >= age));
        if old {
            match fs::remove_file(entry.path()) {
                Ok(()) => tracing::info!(file = %name, "old restore request removed"),
                Err(e) => tracing::warn!(file = %name, error = %e, "removing an old restore request"),
            }
        }
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
    fn only_old_restore_requests_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["restore-1.json", "settings.toml", "restore-notes.txt"] {
            fs::write(dir.path().join(name), "x").unwrap();
        }
        remove_requests_older_than(dir.path(), std::time::Duration::from_secs(3600));
        assert!(dir.path().join("restore-1.json").exists(), "a fresh request stays");
        remove_requests_older_than(dir.path(), std::time::Duration::ZERO);
        assert!(!dir.path().join("restore-1.json").exists());
        assert!(dir.path().join("settings.toml").exists());
        assert!(dir.path().join("restore-notes.txt").exists());
    }

    #[test]
    fn damaged_file_is_an_error_not_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Storage::at(dir.path().to_path_buf()).unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "profiles = 3").unwrap();
        assert!(s.load().is_err());
    }
}
