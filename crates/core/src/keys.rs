//! Mapping between absolute paths and archive keys.
//!
//! A key is the absolute path with `/` separators and the root spelled as a
//! plain first segment, so it can be used as a relative path inside the
//! archive and as a zip entry name:
//!
//! | Path | Key |
//! | --- | --- |
//! | `C:\Users\user\a.txt` | `C/Users/user/a.txt` |
//! | `\\server\share\a.txt` | `UNC/server/share/a.txt` |
//! | `/home/user/a.txt` | `ROOT/home/user/a.txt` |

use std::path::{Component, Path, PathBuf, Prefix};

/// Why a path has no key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error("the path is not absolute")]
    NotAbsolute,
    #[error("the path is not valid Unicode")]
    NotUnicode,
    #[error("the path contains '..'")]
    ParentDir,
    /// A key read from an archive that no path gives: it could lead a
    /// restore outside the place chosen.
    #[error("not a key Archeion writes: {0}")]
    Unsafe(String),
}

/// Key of an absolute path.
pub fn key_for(path: &Path) -> Result<String, KeyError> {
    let mut parts: Vec<String> = Vec::new();
    let mut rooted = false;
    for c in path.components() {
        match c {
            Component::Prefix(p) => {
                rooted = true;
                match p.kind() {
                    Prefix::Disk(d) | Prefix::VerbatimDisk(d) => {
                        parts.push(char::from(d).to_ascii_uppercase().to_string());
                    }
                    Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                        parts.push("UNC".to_owned());
                        parts.push(server.to_str().ok_or(KeyError::NotUnicode)?.to_owned());
                        parts.push(share.to_str().ok_or(KeyError::NotUnicode)?.to_owned());
                    }
                    Prefix::Verbatim(_) | Prefix::DeviceNS(_) => return Err(KeyError::NotAbsolute),
                }
            }
            Component::RootDir => {
                if !rooted {
                    parts.push("ROOT".to_owned());
                    rooted = true;
                }
            }
            Component::CurDir => {}
            Component::ParentDir => return Err(KeyError::ParentDir),
            Component::Normal(s) => parts.push(s.to_str().ok_or(KeyError::NotUnicode)?.to_owned()),
        }
    }
    if !rooted || !path.is_absolute() {
        return Err(KeyError::NotAbsolute);
    }
    Ok(parts.join("/"))
}

/// Checks a key read from an archive: it must be one [`key_for`] can
/// give, made of plain names, so that [`path_for`] and [`path_under`] stay
/// where the key says. A segment such as `..`, `C:\x` or `a\b` (a folder
/// separator on Windows) is refused.
///
/// # Errors
/// [`KeyError::Unsafe`] with the key.
pub fn check(key: &str) -> Result<(), KeyError> {
    let unsafe_key = || KeyError::Unsafe(key.to_owned());
    let mut segs = key.split('/');
    let first = segs.next().unwrap_or_default();
    let rooted = match first {
        "ROOT" => true,
        "UNC" => {
            // Server and share.
            for _ in 0..2 {
                segs.next().filter(|s| plain(s)).ok_or_else(unsafe_key)?;
            }
            true
        }
        d => d.len() == 1 && d.as_bytes().first().is_some_and(u8::is_ascii_uppercase),
    };
    if !rooted || !segs.all(plain) {
        return Err(unsafe_key());
    }
    Ok(())
}

/// True for one plain file or folder name on this system.
fn plain(segment: &str) -> bool {
    let mut parts = Path::new(segment).components();
    !segment.contains('\0')
        && matches!(parts.next(), Some(Component::Normal(n)) if n == segment)
        && parts.next().is_none()
}

/// Original absolute path of a key.
#[must_use]
pub fn path_for(key: &str) -> PathBuf {
    let mut segs = key.split('/');
    let first = segs.next().unwrap_or_default();
    let mut path = if first == "ROOT" {
        PathBuf::from("/")
    } else if first == "UNC" {
        let server = segs.next().unwrap_or_default();
        let share = segs.next().unwrap_or_default();
        PathBuf::from(format!(r"\\{server}\{share}\"))
    } else {
        PathBuf::from(format!(r"{first}:\"))
    };
    for s in segs {
        path.push(s);
    }
    path
}

/// Path of a key below `base` (restore to another folder, folder archives).
#[must_use]
pub fn path_under(base: &Path, key: &str) -> PathBuf {
    let mut p = base.to_path_buf();
    for s in key.split('/') {
        p.push(s);
    }
    p
}

/// True when `key` is `prefix` itself or inside it (prefix of folders).
#[must_use]
pub fn is_within(key: &str, prefix: &str) -> bool {
    key == prefix || (key.starts_with(prefix) && key.as_bytes().get(prefix.len()) == Some(&b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_paths_map_both_ways() {
        for (path, key) in [
            (r"C:\Users\user\a b\ü.txt", "C/Users/user/a b/ü.txt"),
            (r"d:\Data", "D/Data"),
            (r"\\server\share\x\y.txt", "UNC/server/share/x/y.txt"),
        ] {
            assert_eq!(key_for(Path::new(path)).unwrap(), key);
            let back = path_for(key);
            assert_eq!(key_for(&back).unwrap(), key);
        }
        assert_eq!(key_for(Path::new(r"\\?\C:\long\path")).unwrap(), "C/long/path");
    }

    #[cfg(unix)]
    #[test]
    fn unix_paths_map_both_ways() {
        assert_eq!(key_for(Path::new("/home/m/a.txt")).unwrap(), "ROOT/home/m/a.txt");
        assert_eq!(path_for("ROOT/home/m/a.txt"), PathBuf::from("/home/m/a.txt"));
    }

    #[test]
    fn relative_and_parent_paths_have_no_key() {
        assert_eq!(key_for(Path::new("docs/a.txt")), Err(KeyError::NotAbsolute));
    }

    #[test]
    fn within_matches_whole_segments_only() {
        assert!(is_within("C/Users/user/a.txt", "C/Users/user"));
        assert!(is_within("C/Users/user", "C/Users/user"));
        assert!(!is_within("C/Users/user2/a.txt", "C/Users/user"));
    }

    #[test]
    fn keys_of_paths_pass_the_check_and_doctored_ones_do_not() {
        for key in ["C/Users/user/a b/ü.txt", "D", "UNC/server/share/x", "ROOT/home/m/a.txt", "C/x/.hidden"] {
            assert_eq!(check(key), Ok(()), "{key}");
        }
        for key in [
            "",
            "C/Users/me/../../Windows/evil.dll",
            "C/Users/./a",
            "C//a",
            "C/a/",
            "c/a",
            "CD/a",
            "UNC/nas",
            "UNC//share/a",
            "../a",
            "C/a\0b",
        ] {
            assert!(matches!(check(key), Err(KeyError::Unsafe(_))), "{key:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn segments_windows_reads_as_paths_are_refused() {
        for key in [r"C/Users/C:\Windows\evil.dll", r"C/x/\\host\share\evil", r"C/a\b", "C/x/C:"] {
            assert!(matches!(check(key), Err(KeyError::Unsafe(_))), "{key:?}");
        }
    }

    #[test]
    fn path_under_nests_the_key() {
        let p = path_under(Path::new("base"), "C/x/y.txt");
        assert_eq!(p, Path::new("base").join("C").join("x").join("y.txt"));
    }
}
