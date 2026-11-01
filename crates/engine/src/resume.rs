//! Journal of an unfinished backup: `<archive>.partial/resume.jsonl`.
//!
//! Each file copied into the archive being built gets one line once its
//! content is complete. When a run stops before committing (Cancel, full
//! disk, unplugged drive, power cut), the next backup reopens the same
//! unfinished archive and keeps every journalled file whose source still
//! has the same size and modification time and whose stored copy is
//! complete; only the rest is copied.
//!
//! A line is `{"id":…,"n":…,"note":…}`: `id` names the stored file (its
//! path, or its opaque name in an encrypted destination), `note` holds
//! "size mtime hash", sealed with the destination's keys when it is
//! encrypted so the journal reveals nothing a manifest would not.

use crate::{EngineError, Result, crypto::Keys};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
};

pub(crate) const FILE: &str = "resume.jsonl";

#[derive(Serialize, Deserialize)]
struct Line {
    id: String,
    /// Line number: makes every sealed note's keystream unique.
    n: u64,
    note: String,
}

/// A file already copied, as its source was when copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Done {
    pub size: u64,
    pub mtime_ns: i64,
    /// BLAKE3 of the content, hex.
    pub hash: String,
}

impl Done {
    fn to_note(&self) -> String {
        format!("{} {} {}", self.size, self.mtime_ns, self.hash)
    }

    fn parse(note: &str) -> Option<Self> {
        let mut parts = note.split(' ');
        let size = parts.next()?.parse().ok()?;
        let mtime_ns = parts.next()?.parse().ok()?;
        let hash = parts.next()?;
        let valid = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && parts.next().is_none();
        valid.then(|| Self { size, mtime_ns, hash: hash.to_owned() })
    }
}

/// The journal of the archive being built.
#[derive(Debug)]
pub(crate) struct Journal {
    path: PathBuf,
    file: File,
    keys: Option<Keys>,
    done: HashMap<String, Done>,
    lines: u64,
}

impl Journal {
    /// Opens (or starts) the journal in `staging`. Lines that cannot be
    /// read, such as one cut short by a power loss, are ignored.
    pub(crate) fn open(staging: &Path, keys: Option<&Keys>) -> Result<Self> {
        let path = staging.join(FILE);
        let ctx = || format!("opening {}", path.display());
        let mut file = File::options()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(EngineError::io(ctx()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(EngineError::io(ctx()))?;
        // A torn write may leave invalid UTF-8: that line just fails to parse.
        let text = String::from_utf8_lossy(&bytes);
        let mut done = HashMap::new();
        let mut lines = 0u64;
        for line in text.lines() {
            let Ok(l) = serde_json::from_str::<Line>(line) else { continue };
            lines = lines.max(l.n.saturating_add(1));
            let plain = match keys {
                Some(k) => k.open_note(&note_id(&l.id, l.n), &l.note),
                None => Some(l.note),
            };
            if let Some(d) = plain.as_deref().and_then(Done::parse) {
                done.insert(l.id, d);
            }
        }
        if !text.is_empty() && !text.ends_with('\n') {
            // Finish the torn line so the next one starts cleanly.
            file.write_all(b"\n").map_err(EngineError::io(ctx()))?;
        }
        Ok(Self { path, file, keys: keys.cloned(), done, lines })
    }

    /// Files journalled so far.
    pub(crate) fn len(&self) -> usize {
        self.done.len()
    }

    pub(crate) fn get(&self, id: &str) -> Option<&Done> {
        self.done.get(id)
    }

    /// Records that `id` is completely stored. Not flushed to the disk at
    /// once: a line lost in a power cut only means copying that file again,
    /// and a line whose file did not reach the disk is caught by
    /// [`crate::container::Writer::looks_complete`].
    pub(crate) fn record(&mut self, id: &str, done: Done) -> Result<()> {
        let n = self.lines;
        let plain = done.to_note();
        let note = match &self.keys {
            Some(k) => k.seal_note(&note_id(id, n), &plain),
            None => plain,
        };
        let mut line = serde_json::to_string(&Line { id: id.to_owned(), n, note }).expect("line serializes");
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .map_err(EngineError::io(format!("writing {}", self.path.display())))?;
        self.lines = n.saturating_add(1);
        self.done.insert(id.to_owned(), done);
        Ok(())
    }

    /// Deletes the journal: the archive is about to be committed.
    pub(crate) fn remove(self) -> Result<()> {
        drop(self.file);
        fs::remove_file(&self.path).map_err(EngineError::io(format!("removing {}", self.path.display())))
    }
}

fn note_id(id: &str, n: u64) -> String {
    format!("{id}\0{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done(size: u64) -> Done {
        Done { size, mtime_ns: -5, hash: "ab".repeat(32) }
    }

    #[test]
    fn lines_survive_reopening_and_the_last_one_wins() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = Journal::open(dir.path(), None).unwrap();
        j.record("C/a.txt", done(1)).unwrap();
        j.record("C/b.txt", done(2)).unwrap();
        j.record("C/a.txt", done(3)).unwrap();
        drop(j);
        let j = Journal::open(dir.path(), None).unwrap();
        assert_eq!(j.len(), 2);
        assert_eq!(j.get("C/a.txt"), Some(&done(3)));
        j.remove().unwrap();
        assert!(!dir.path().join(FILE).exists());
    }

    #[test]
    fn a_torn_line_is_ignored_and_the_next_one_starts_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let mut j = Journal::open(dir.path(), None).unwrap();
        j.record("C/a.txt", done(1)).unwrap();
        drop(j);
        let mut f = File::options().append(true).open(dir.path().join(FILE)).unwrap();
        f.write_all(br#"{"id":"C/b.txt","n":1,"no"#).unwrap();
        drop(f);
        let mut j = Journal::open(dir.path(), None).unwrap();
        assert_eq!(j.len(), 1);
        j.record("C/c.txt", done(4)).unwrap();
        drop(j);
        let j = Journal::open(dir.path(), None).unwrap();
        assert_eq!(j.get("C/c.txt"), Some(&done(4)));
        assert_eq!(j.get("C/b.txt"), None);
    }

    #[test]
    fn notes_are_parsed_strictly() {
        assert_eq!(Done::parse(&done(7).to_note()), Some(done(7)));
        assert_eq!(Done::parse("7 -5 xyz"), None);
        assert_eq!(Done::parse(&format!("{} extra", done(7).to_note())), None);
    }
}
