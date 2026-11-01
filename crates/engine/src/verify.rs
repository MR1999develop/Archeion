//! Integrity check and restore test.

use crate::{
    EventSink, Result,
    container::{Reader, hash_stream},
    crypto::Password,
    progress::Progress,
    repo::Repo,
};
use archeion_core::{EntryKind, Phase, Profile, RunEvent, Summary, VerifyReport};
use std::collections::HashSet;

/// Reads content back and compares it with the manifests.
///
/// With `sample = None` every stored file of every archive is read, and
/// every file listed as "stored earlier" must really be stored in an earlier
/// archive. With `sample = Some(n)` only `n` files of the newest archive are
/// read back, from wherever they are stored (the restore test).
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

    // (archive index, key, size, hash) to read back.
    let mut checks: Vec<(usize, String, u64, Option<String>)> = Vec::new();
    match sample {
        None => {
            let mut stored_so_far: HashSet<String> = HashSet::new();
            for (j, m) in manifests.iter().enumerate() {
                let Some(m) = m else { continue };
                for e in m.entries.iter().filter(|e| e.kind == EntryKind::File) {
                    if e.stored {
                        checks.push((j, e.key.clone(), e.size, e.hash.clone()));
                    } else if !stored_so_far.contains(&e.key) {
                        report
                            .problems
                            .push(format!("{}: {} is not stored in any earlier archive", archives[j], e.key));
                    }
                }
                stored_so_far.extend(m.entries.iter().filter(|e| e.stored).map(|e| e.key.clone()));
            }
        }
        Some(n) => {
            if let Some(Some(latest)) = manifests.last() {
                let files: Vec<_> = latest.entries.iter().filter(|e| e.kind == EntryKind::File).collect();
                if !files.is_empty() && n > 0 {
                    let step = (files.len() / n).max(1);
                    let start = usize::try_from(jiff::Timestamp::now().as_second().unsigned_abs())
                        .unwrap_or(0)
                        % step;
                    for e in files.iter().skip(start).step_by(step).take(n) {
                        let holder = manifests
                            .iter()
                            .enumerate()
                            .rev()
                            .find(|(_, m)| m.as_ref().and_then(|m| m.get(&e.key)).is_some_and(|x| x.stored))
                            .map(|(j, _)| j);
                        match holder {
                            Some(j) => checks.push((j, e.key.clone(), e.size, e.hash.clone())),
                            None => report.problems.push(format!("{} is not stored in any archive", e.key)),
                        }
                    }
                }
            }
        }
    }

    sink.emit(RunEvent::Phase { phase: Phase::Verifying });
    let mut progress = Progress::new(sink, checks.iter().map(|c| c.2).sum(), checks.len() as u64);
    let mut readers: Vec<Option<Reader>> = archives.iter().map(|_| None).collect();
    for (j, key, size, hash) in checks {
        let Some(name) = archives.get(j) else { continue };
        let slot = readers.get_mut(j).expect("one slot per archive");
        if slot.is_none() {
            match Reader::open(&repo.dir(name), repo.keys()) {
                Ok(r) => *slot = Some(r),
                Err(e) => {
                    report.problems.push(format!("{name}: {e}"));
                    continue;
                }
            }
        }
        let Some(reader) = slot.as_mut() else { continue };
        progress.start_file(archeion_core::keys::path_for(&key).display().to_string());
        let result = reader.read(&key, |src| {
            hash_stream(src, &mut |n| progress.add(n))
                .map_err(crate::EngineError::io(format!("reading {key}")))
        });
        report.files_checked += 1;
        match result {
            Ok((n, h)) if n == size && hash.as_deref().is_none_or(|x| x == h) => {}
            Ok(_) => report.problems.push(format!("{name}: {key} has different content")),
            Err(e) => report.problems.push(format!("{name}: {e}")),
        }
    }
    progress.finish();
    sink.emit(RunEvent::Finished { summary: Summary::Verify(report.clone()) });
    Ok(report)
}
