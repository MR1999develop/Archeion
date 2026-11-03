//! One window at a time. The first window locks `window.lock` in the data
//! folder for as long as it runs and listens on a local TCP port, written
//! to `window.port`; starting Archeion again (from the Start menu while it
//! sits in the notification area, say) asks that window to show itself
//! and exits. Workers (`archeion run …`) are not concerned.

use std::{
    fs::{self, File},
    io::{Read as _, Write as _},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::Path,
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
    /// Another window runs and was asked to show itself.
    Secondary,
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
            // The port, then this process's id: the second launch lets it
            // come to the front.
            let text = format!(
                "{}
{}",
                listener.local_addr()?.port(),
                std::process::id()
            );
            fs::write(dir.join(PORT_FILE), text)?;
            Ok(Claim::Primary(Primary { lock, listener }))
        }
        Err(fs::TryLockError::WouldBlock) => {
            wake(dir)?;
            Ok(Claim::Secondary)
        }
        Err(fs::TryLockError::Error(e)) => Err(e),
    }
}

/// Asks the running window to show itself.
fn wake(dir: &Path) -> std::io::Result<()> {
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
            let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
            match TcpStream::connect_timeout(&addr, IO_TIMEOUT) {
                Ok(mut stream) => return stream.write_all(SHOW),
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
        assert!(matches!(claim(dir.path()).unwrap(), Claim::Secondary));
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok(), "the first window was asked to show");
    }
}
