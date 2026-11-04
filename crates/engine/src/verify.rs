//! Integrity check and restore test.

use crate::{
    EngineError, EventSink, Result,
    chunks::{ChunkIndex, ChunkReader, Form},
    container::{HashingWriter, Reader, copy_into},
    crypto::Password,
    progress::Progress,
    repo::Repo,
};
use archeion_core::{
    ChunkId, ChunkRef, Container, Entry, EntryKind, FileAction, FileDone, Manifest, Phase, Profile, RunEvent,
    Summary, VerifyReport, keys,
};
use std::{
    collections::HashSet,
    io::{self, Write},
    path::PathBuf,
};

/// Reads content back and compares it with the manifests.
///
/// With `sample = None` every stored file of every archive is read, every
/// file listed as "stored earlier" must really be stored in an earlier
/// archive, every chunk file is read back and checked against its name,
/// every chunk a file lists must be somewhere in the destination, and the
/// chunk list kept beside a whole copy must match that copy. With
/// `sample = Some(n)` only `n` files of the newest archive are read back,
/// from wherever they are stored, put back together from their chunks when
/// they are stored that way (the restore test).
///
/// # Errors
/// Only when the destination cannot be opened; content problems are listed
/// in the report.
pub fn verify(
    profile: &Profile,
    password: Option<&Password>,
    sample: Option<usize>,
    sink: &dyn EventSink,
) -> Result<VerifyReport> {
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let mut report = VerifyReport::default();
    let Some(repo) = Repo::open(profile, false, password)? else {
        sink.emit(RunEvent::Finished { summary: Summary::Verify(report.clone()) });
        return Ok(report);
    };
    let archives = repo.archives()?;
    if repo.list_matches()? == Some(false) {
        report.problems.push(
            "the list of backups does not match its signature: a backup was removed or put back from outside \
             Archeion"
                .to_owned(),
        );
    }
    for folder in repo.set_aside()? {
        report.problems.push(format!(
            "{folder}: a backup found unreadable after an interrupted merge, set aside with the backups it \
             replaced put back; Archeion does not remove it"
        ));
    }
    let mut manifests = Vec::with_capacity(archives.len());
    for name in &archives {
        match repo.manifest(name) {
            Ok(m) => manifests.push(Some(m)),
            Err(e) => {
                report.problems.push(e.to_string());
                manifests.push(None);
            }
        }
    }

    // Where every chunk of the destination can be read.
    let dirs: Vec<PathBuf> = archives.iter().map(|a| repo.dir(a)).collect();
    let container = dirs.last().map_or(Container::ZipPerFile, |d| crate::container::detect(d));
    let form = Form::of(container, repo.keys());
    let mut index = ChunkIndex::default();
    let mut chunk_files: Vec<(usize, ChunkId)> = Vec::new();
    for (j, (dir, m)) in dirs.iter().zip(&manifests).enumerate() {
        match form.list(dir) {
            Ok(ids) => {
                chunk_files.extend(ids.iter().map(|id| (j, *id)));
                index.add_files(j, ids);
            }
            Err(e) => report.problems.push(format!("{}: {e}", archives[j])),
        }
        if let Some(m) = m {
            index.add_whole_copies(j, m);
        }
    }

    let mut checks: Vec<Check> = Vec::new();
    match sample {
        None => {
            let mut stored_so_far: HashSet<String> = HashSet::new();
            for (j, m) in manifests.iter().enumerate() {
                let Some(m) = m else { continue };
                for e in m.entries.iter().filter(|e| e.kind == EntryKind::File) {
                    if e.chunks_only() {
                        report.files_checked += 1;
                        let list = e.chunks.iter().flat_map(|c| &c.list);
                        for missing in list.filter(|c| !index.contains(&c.id)) {
                            report.problems.push(format!(
                                "{}: chunk {} of {} is missing",
                                archives[j], missing.id, e.key
                            ));
                        }
                    } else if e.stored {
                        checks.push(Check::file(j, e));
                    } else if !stored_so_far.contains(&e.key) {
                        report
                            .problems
                            .push(format!("{}: {} is not stored in any earlier archive", archives[j], e.key));
                    }
                }
                stored_so_far.extend(m.entries.iter().filter(|e| e.stored).map(|e| e.key.clone()));
            }
            checks.extend(chunk_files.into_iter().map(|(archive, id)| Check::ChunkFile { archive, id }));
        }
        Some(n) => sample_checks(&manifests, n, &mut checks, &mut report),
    }

    sink.emit(RunEvent::Phase { phase: Phase::Verifying });
    let files = checks.iter().filter(|c| !matches!(c, Check::ChunkFile { .. })).count() as u64;
    let mut progress = Progress::new(sink, checks.iter().map(Check::size).sum(), files);
    let chunk_reader = ChunkReader { index: &index, dirs: &dirs, form: &form, keys: repo.keys() };
    let mut readers: Vec<Option<Reader>> = archives.iter().map(|_| None).collect();
    for check in checks {
        progress.check_stop()?;
        match check {
            Check::ChunkFile { archive, id } => {
                let (Some(name), Some(dir)) = (archives.get(archive), dirs.get(archive)) else { continue };
                match form.read(dir, &id, None) {
                    Ok(data) => progress.add(data.len() as u64),
                    Err(e) => report.problems.push(format!("{name}: {} ({e})", form.relative(&id))),
                }
            }
            Check::File { archive, key, size, hash, chunks_only, list } => {
                let Some(name) = archives.get(archive) else { continue };
                progress.start_file(keys::path_for(&key).display().to_string());
                let result = if chunks_only {
                    let mut out = HashingWriter::new(io::sink());
                    chunk_reader.write_all(&list, &mut out, &mut |n| progress.add(n)).map(|()| {
                        let (n, h, _) = out.finish();
                        (n, h)
                    })
                } else {
                    read_whole(&mut readers, archive, &dirs, &repo, &key, &form, &list, &mut progress)
                };
                report.files_checked += 1;
                let problem = match result {
                    Ok((n, h)) if n == size && hash.as_deref().is_none_or(|x| x == h) => None,
                    Ok(_) => Some(format!("{name}: {key} has different content")),
                    Err(e) => Some(format!("{name}: {e}")),
                };
                match problem {
                    // The few files of a restore test are worth a line
                    // each; the many of a full check only a count.
                    None if sample.is_some() => progress.file_done(FileAction::Checked, size),
                    None => progress.file_counted(FileAction::Checked, size),
                    Some(p) => {
                        progress.done_with(FileDone {
                            note: p.clone(),
                            ..FileDone::new(
                                keys::path_for(&key).display().to_string(),
                                FileAction::Damaged,
                                size,
                            )
                        });
                        report.problems.push(p);
                    }
                }
            }
        }
    }
    progress.finish();
    sink.emit(RunEvent::Finished { summary: Summary::Verify(report.clone()) });
    Ok(report)
}

/// One thing to read back.
enum Check {
    /// A stored file of archive number `archive`: its whole copy (checked
    /// against `list` too when it has one), or its chunks.
    File {
        archive: usize,
        key: String,
        size: u64,
        hash: Option<String>,
        chunks_only: bool,
        list: Vec<ChunkRef>,
    },
    /// A chunk file, checked against its name.
    ChunkFile { archive: usize, id: ChunkId },
}

impl Check {
    fn file(archive: usize, e: &Entry) -> Self {
        Self::File {
            archive,
            key: e.key.clone(),
            size: e.size,
            hash: e.hash.clone(),
            chunks_only: e.chunks_only(),
            list: e.chunks.as_ref().map(|c| c.list.clone()).unwrap_or_default(),
        }
    }

    /// Bytes to read, as far as known before reading.
    fn size(&self) -> u64 {
        match self {
            Self::File { size, .. } => *size,
            Self::ChunkFile { .. } => 0,
        }
    }
}

/// The restore test: `n` files of the newest archive, each read from the
/// archive that stores it.
fn sample_checks(
    manifests: &[Option<Manifest>],
    n: usize,
    checks: &mut Vec<Check>,
    report: &mut VerifyReport,
) {
    let Some(Some(latest)) = manifests.last() else { return };
    let files: Vec<_> = latest.entries.iter().filter(|e| e.kind == EntryKind::File).collect();
    if files.is_empty() || n == 0 {
        return;
    }
    let step = (files.len() / n).max(1);
    let start = usize::try_from(jiff::Timestamp::now().as_second().unsigned_abs()).unwrap_or(0) % step;
    for e in files.iter().skip(start).step_by(step).take(n) {
        let holder =
            manifests.iter().enumerate().rev().find_map(|(j, m)| {
                m.as_ref().and_then(|m| m.get(&e.key)).filter(|x| x.stored).map(|x| (j, x))
            });
        match holder {
            Some((j, stored)) => checks.push(Check::file(j, stored)),
            None => report.problems.push(format!("{} is not stored in any archive", e.key)),
        }
    }
}

/// Reads the whole copy of `key` in archive number `archive`: its size and
/// hash, after checking that it splits into `list` (when not empty).
#[expect(clippy::too_many_arguments, reason = "one call site, the state of the check")]
fn read_whole(
    readers: &mut [Option<Reader>],
    archive: usize,
    dirs: &[PathBuf],
    repo: &Repo,
    key: &str,
    form: &Form,
    list: &[ChunkRef],
    progress: &mut Progress<'_>,
) -> Result<(u64, String)> {
    let (Some(slot), Some(dir)) = (readers.get_mut(archive), dirs.get(archive)) else {
        return Err(EngineError::Damaged { archive: key.to_owned(), message: "unknown archive".into() });
    };
    if slot.is_none() {
        *slot = Some(Reader::open(dir, repo.keys())?);
    }
    let reader = slot.as_mut().expect("opened above");
    reader.read(key, |src| {
        let mut out = HashingWriter::new(ListCheck::new(form, list));
        copy_into(src, &mut out, &mut |n| progress.add(n))
            .map_err(EngineError::io(format!("reading {key}")))?;
        let (n, hash, check) = out.finish();
        check.finish().map_err(|message| EngineError::Damaged { archive: key.to_owned(), message })?;
        Ok((n, hash))
    })
}

/// Checks that the bytes written split into the chunks of a list (an empty
/// list checks nothing).
struct ListCheck<'a> {
    form: &'a Form,
    list: &'a [ChunkRef],
    next: usize,
    buf: Vec<u8>,
    problem: Option<String>,
}

impl<'a> ListCheck<'a> {
    fn new(form: &'a Form, list: &'a [ChunkRef]) -> Self {
        Self { form, list, next: 0, buf: Vec::new(), problem: None }
    }

    /// The first mismatch, if any.
    fn finish(self) -> std::result::Result<(), String> {
        if let Some(p) = self.problem {
            return Err(p);
        }
        if !self.list.is_empty() && (self.next != self.list.len() || !self.buf.is_empty()) {
            return Err("the chunk list does not cover the whole copy".into());
        }
        Ok(())
    }
}

impl Write for ListCheck<'_> {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        let n = data.len();
        if self.list.is_empty() || self.problem.is_some() {
            return Ok(n);
        }
        while !data.is_empty() {
            let Some(chunk) = self.list.get(self.next) else {
                self.problem = Some("the whole copy is longer than its chunk list".into());
                break;
            };
            let want = (chunk.size as usize).saturating_sub(self.buf.len());
            let (take, rest) = data.split_at(want.min(data.len()));
            self.buf.extend_from_slice(take);
            data = rest;
            if self.buf.len() == chunk.size as usize {
                if self.form.id_of(&self.buf) != chunk.id {
                    self.problem = Some(format!("chunk {} does not match the whole copy", chunk.id));
                    break;
                }
                self.buf.clear();
                self.next += 1;
            }
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(form: &Form, parts: &[&[u8]]) -> Vec<ChunkRef> {
        parts.iter().map(|p| ChunkRef { id: form.id_of(p), size: u32::try_from(p.len()).unwrap() }).collect()
    }

    #[test]
    fn a_whole_copy_must_split_into_its_chunk_list() {
        let form = Form::Plain;
        let list = refs(&form, &[b"abc", b"defg"]);
        let mut ok = ListCheck::new(&form, &list);
        for piece in [&b"ab"[..], b"cde", b"fg"] {
            ok.write_all(piece).unwrap();
        }
        assert_eq!(ok.finish(), Ok(()));

        let mut changed = ListCheck::new(&form, &list);
        changed.write_all(b"abcdeXg").unwrap();
        assert!(changed.finish().is_err());

        let mut short = ListCheck::new(&form, &list);
        short.write_all(b"abcde").unwrap();
        assert!(short.finish().is_err());

        let mut long = ListCheck::new(&form, &list);
        long.write_all(b"abcdefgh").unwrap();
        assert!(long.finish().is_err());

        let mut none = ListCheck::new(&form, &[]);
        none.write_all(b"anything").unwrap();
        assert_eq!(none.finish(), Ok(()));
    }
}
