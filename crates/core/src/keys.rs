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
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error("the path is not absolute")]
    NotAbsolute,
    #[error("the path is not valid Unicode")]
    NotUnicode,
    #[error("the path contains '..'")]
    ParentDir,
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
    fn path_under_nests_the_key() {
        let p = path_under(Path::new("base"), "C/x/y.txt");
        assert_eq!(p, Path::new("base").join("C").join("x").join("y.txt"));
    }
}
