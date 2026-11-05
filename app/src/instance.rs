//! One window per Windows user, whichever copy of Archeion is started. The
//! first window locks `window.lock` in a folder of the user's
//! ([`user_dir`]: not the data folder, which a portable copy keeps beside
//! itself) for as long as it runs and listens on a local TCP port, written
//! to `window.port` with its process id and executable; starting Archeion
//! again (from the Start menu while it sits in the notification area, say)
//! asks that window to show itself and exits, saying so when the window
//! open is another copy. Workers (`archeion run …`) are not concerned.

use std::{
    fs::{self, File},
    io::{Read as _, Write as _},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Duration,
};

const LOCK_FILE: &str = "window.lock";
const PORT_FILE: &str = "window.port";
const SHOW: &[u8] = b"show";
/// How long a second launch waits for the first window to publish its port
/// (it may be starting at the same moment).
const PORT_WAIT: Duration = Duration::from_secs(3);
const IO_TIMEOUT: Duration = Duration::from_secs(2);

/// The running window: holds the lock and the listening socket.
#[derive(Debug)]
pub struct Primary {
    lock: File,
    listener: TcpListener,
}

/// The outcome of [`claim`].
#[derive(Debug)]
pub enum Claim {
    /// This is the only window.
    Primary(Primary),
    /// Another window runs and was asked to show itself; `other` is its
    /// executable when it is another copy of Archeion than this one.
    Secondary { other: Option<PathBuf> },
}

/// The folder of the window lock: the same for every copy of Archeion run
/// by this Windows user (`%LOCALAPPDATA%\Archeion`), so that a portable
/// copy and an installed one never both open; `data_dir` when the data
/// folder was chosen from outside (`ARCHEION_DATA_DIR`: the tests, which
/// must not meet the user's window).
///
/// # Errors
/// The user has no local application folder, or it cannot be created.
pub fn user_dir(data_dir: &Path, chosen_from_outside: bool) -> std::io::Result<PathBuf> {
    let dir = if chosen_from_outside {
        data_dir.to_path_buf()
    } else {
        directories::BaseDirs::new()
            .map(|d| d.data_local_dir().join("Archeion"))
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no local application folder"))?
    };
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Becomes the window of this user, or wakes the one already running.
///
/// # Errors
/// The lock or port file could not be used; the caller then opens the
/// window anyway.
pub fn claim(dir: &Path) -> std::io::Result<Claim> {
    let lock = File::options().create(true).truncate(false).write(true).open(dir.join(LOCK_FILE))?;
    match lock.try_lock() {
        Ok(()) => {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
            // The port, this process's id (the second launch lets it come
            // to the front) and its executable (another copy is told).
            let exe = std::env::current_exe().map(|e| e.display().to_string()).unwrap_or_default();
            let text = format!("{}\n{}\n{exe}", listener.local_addr()?.port(), std::process::id());
            fs::write(dir.join(PORT_FILE), text)?;
            Ok(Claim::Primary(Primary { lock, listener }))
        }
        Err(fs::TryLockError::WouldBlock) => {
            let running = wake(dir)?;
            let this = std::env::current_exe().ok();
            let other = running.filter(|r| !this.as_deref().is_some_and(|t| same_file(r, t)));
            Ok(Claim::Secondary { other })
        }
        Err(fs::TryLockError::Error(e)) => Err(e),
    }
}

/// True when `a` and `b` name the same executable (Windows paths ignore
/// case; a path that cannot be resolved is compared as written).
fn same_file(a: &Path, b: &Path) -> bool {
    let canon =
        |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()).to_string_lossy().to_lowercase();
    canon(a) == canon(b)
}

/// Asks the running window to show itself; returns its executable, when
/// its port file names one.
fn wake(dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let port_file = dir.join(PORT_FILE);
    let started = std::time::Instant::now();
    loop {
        let text = fs::read_to_string(&port_file).unwrap_or_default();
        let mut lines = text.lines().map(str::trim);
        if let Some(port) = lines.next().and_then(|t| t.parse::<u16>().ok()) {
            // Windows lets only the program in front give the focus away.
            if let Some(pid) = lines.next().and_then(|t| t.parse::<u32>().ok()) {
                archeion_win::allow_foreground(pid);
            }
            let exe = lines.next().filter(|t| !t.is_empty()).map(PathBuf::from);
            let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
            match TcpStream::connect_timeout(&addr, IO_TIMEOUT) {
                Ok(mut stream) => return stream.write_all(SHOW).map(|()| exe),
                Err(e) if started.elapsed() >= PORT_WAIT => return Err(e),
                Err(_) => {}
            }
        } else if started.elapsed() >= PORT_WAIT {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "the running window has no port"));
        }
        // The first window is still starting: its port is not written yet.
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl Primary {
    /// Calls `on_show` each time another launch asks for the window, on a
    /// background thread that lives as long as the program (and keeps the
    /// lock).
    pub fn serve(self, on_show: impl Fn() + Send + 'static) {
        let Self { lock, listener } = self;
        let spawned = std::thread::Builder::new().name("single-instance".into()).spawn(move || {
            let _lock = lock;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
                let mut buf = [0u8; 16];
                let n = stream.read(&mut buf).unwrap_or(0);
                if buf.get(..n).is_some_and(|b| b.starts_with(SHOW)) {
                    on_show();
                }
            }
        });
        if let Err(e) = spawned {
            tracing::warn!(error = %e, "a second launch will not find this window");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn a_second_launch_wakes_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let Claim::Primary(primary) = claim(dir.path()).unwrap() else { panic!("first launch") };
        let published = fs::read_to_string(dir.path().join(PORT_FILE)).unwrap();
        let pid = std::process::id().to_string();
        assert_eq!(published.lines().nth(1), Some(pid.as_str()), "the id that may take the focus");
        let (tx, rx) = mpsc::channel();
        primary.serve(move || {
            let _ = tx.send(());
        });
        assert!(matches!(claim(dir.path()).unwrap(), Claim::Secondary { other: None }), "the same copy");
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok(), "the first window was asked to show");
    }

    /// A window of another copy (another folder, another version) is still
    /// the one window: the second launch wakes it and names it.
    #[test]
    fn a_launch_of_another_copy_wakes_the_window_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        let Claim::Primary(primary) = claim(dir.path()).unwrap() else { panic!("first launch") };
        let other = dir.path().join("Other copy").join("archeion.exe");
        let text = fs::read_to_string(dir.path().join(PORT_FILE)).unwrap();
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        lines[2] = other.display().to_string();
        fs::write(dir.path().join(PORT_FILE), lines.join("\n")).unwrap();
        let (tx, rx) = mpsc::channel();
        primary.serve(move || {
            let _ = tx.send(());
        });
        match claim(dir.path()).unwrap() {
            Claim::Secondary { other: Some(named) } => assert_eq!(named, other),
            c => panic!("{c:?}"),
        }
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok(), "the window was asked to show");
    }

    /// Copies whose data folders differ meet at the same lock: the user's
    /// folder, unless the data folder was chosen from outside (tests).
    #[test]
    fn every_copy_of_a_user_meets_at_the_same_lock() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let ua = user_dir(a.path(), false).unwrap();
        assert_eq!(ua, user_dir(b.path(), false).unwrap(), "portable copies in two folders");
        assert!(ua.ends_with("Archeion"));
        assert_eq!(user_dir(a.path(), true).unwrap(), a.path(), "a sandbox keeps its own");
    }
}
