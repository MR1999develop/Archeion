//! Starting with Windows: the value "Archeion" under
//! `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`, which
//! starts `archeion.exe --tray` (the window opens hidden in the
//! notification area). It is the standard per-user startup entry, listed
//! in Task Manager under "Startup apps", where the user can also disable
//! it; no administrator rights are needed.

use std::path::Path;

/// Argument that starts the window hidden in the notification area.
pub const TRAY_ARG: &str = "--tray";

const VALUE_NAME: &str = "Archeion";

/// The command line registered for `exe`.
#[must_use]
pub fn command_line(exe: &Path) -> String {
    format!("\"{}\" {TRAY_ARG}", exe.display())
}

/// What to do with the registered entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Nothing,
    Write,
    Remove,
}

/// Compares the registered command (if any) with what is wanted. A copy
/// moved elsewhere registers itself again.
fn change(registered: Option<&str>, wanted: bool, command: &str) -> Change {
    match (registered, wanted) {
        (Some(r), true) if r.eq_ignore_ascii_case(command) => Change::Nothing,
        (_, true) => Change::Write,
        (Some(_), false) => Change::Remove,
        (None, false) => Change::Nothing,
    }
}

/// Registers or removes the startup entry so that it matches `wanted` and
/// starts `exe`.
///
/// # Errors
/// The registry could not be read or written.
pub fn sync(wanted: bool, exe: &Path) -> std::io::Result<()> {
    let command = command_line(exe);
    let registered = registry::read(VALUE_NAME)?;
    match change(registered.as_deref(), wanted, &command) {
        Change::Nothing => Ok(()),
        Change::Write => {
            tracing::info!(command, "registering the start with Windows");
            registry::write(VALUE_NAME, &command)
        }
        Change::Remove => {
            tracing::info!("removing the start with Windows");
            registry::remove(VALUE_NAME)
        }
    }
}

#[cfg(windows)]
mod registry {
    use std::io;
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE},
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn run_key(flags: u32) -> io::Result<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN, flags)
    }

    pub fn read(name: &str) -> io::Result<Option<String>> {
        match run_key(KEY_QUERY_VALUE)?.get_value::<String, _>(name) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn write(name: &str, command: &str) -> io::Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN)?;
        key.set_value(name, &command.to_owned())
    }

    pub fn remove(name: &str) -> io::Result<()> {
        match run_key(KEY_SET_VALUE)?.delete_value(name) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Elsewhere there is no startup entry to manage.
#[cfg(not(windows))]
mod registry {
    pub fn read(_name: &str) -> std::io::Result<Option<String>> {
        Ok(None)
    }

    pub fn write(_name: &str, _command: &str) -> std::io::Result<()> {
        Ok(())
    }

    pub fn remove(_name: &str) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_quotes_the_path_and_starts_in_the_tray() {
        let cmd = command_line(Path::new(r"C:\Program Files\Archeion\archeion.exe"));
        assert_eq!(cmd, r#""C:\Program Files\Archeion\archeion.exe" --tray"#);
    }

    #[test]
    fn the_entry_follows_the_option_and_the_executable() {
        let cmd = r#""C:\A\archeion.exe" --tray"#;
        assert_eq!(change(None, true, cmd), Change::Write);
        assert_eq!(change(Some(cmd), true, cmd), Change::Nothing);
        assert_eq!(change(Some(&cmd.to_uppercase()), true, cmd), Change::Nothing, "paths ignore case");
        assert_eq!(change(Some(r#""D:\B\archeion.exe" --tray"#), true, cmd), Change::Write);
        assert_eq!(change(Some(cmd), false, cmd), Change::Remove);
        assert_eq!(change(None, false, cmd), Change::Nothing);
    }
}
