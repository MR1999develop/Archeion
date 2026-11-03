//! Hands a file or folder to the system: a folder opens in File Explorer,
//! a file in the program Windows associates with its type (Notepad for a
//! `.txt`, unless the user chose another editor).

use std::path::PathBuf;

/// The system's opener: `explorer <path>` on Windows, by its full path
/// (it opens files with their default program too, and takes the path as
/// one argument, so no shell quoting is involved), `open` on macOS,
/// `xdg-open` elsewhere.
fn opener() -> std::path::PathBuf {
    if cfg!(windows) {
        archeion_win::system_program("explorer.exe")
    } else if cfg!(target_os = "macos") {
        "open".into()
    } else {
        "xdg-open".into()
    }
}

/// Opens `path` with the system's opener. Waits for the opener on its own
/// thread, so the window never blocks and no child process is left
/// unwaited. Failures are only logged: there is nothing to retry.
pub fn open(path: PathBuf) {
    let spawned = std::thread::Builder::new().name("open-path".into()).spawn(move || {
        // explorer.exe often exits with 1 even when the path opened: only
        // a failure to start it is worth logging.
        if let Err(e) = std::process::Command::new(opener()).arg(&path).status() {
            tracing::warn!(error = %e, path = %path.display(), "opening with the system");
        }
    });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "opening with the system");
    }
}
