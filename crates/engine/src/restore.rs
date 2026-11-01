//! Restore files as they were in a given archive.

use crate::{
    EngineError, EventSink, Result,
    container::{Reader, copy_hashing, system_time},
    crypto::Password,
    progress::Progress,
    repo::Repo,
};
use archeion_core::{
    ArchiveName, EntryKind, Phase, Profile, RestoreReport, RunEvent, Skipped, Summary, keys,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Where restored files go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum RestoreTarget {
    /// Their original location.
    Original,
    /// Next to the original, with the backup date in the name:
    /// `report (2026-11-02 21.30).docx`. The current file is never touched.
    BesideOriginal,
    /// Below this folder, keeping the full path (`<folder>\C\Users\...`).
    Folder { path: PathBuf },
}

/// What to restore; the window writes it to a file for the worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreRequest {
    /// Archive name, as listed.
    pub archive: String,
    /// Keys of the files or folders to restore; empty = everything.
    pub keys: Vec<String>,
    pub target: RestoreTarget,
    /// Replace files that already exist.
    pub overwrite: bool,
}

/// Restores the selection of `request`.
///
/// Each file's content comes from the newest archive, up to the chosen one,
/// that stores it. Files are written to a temporary name and renamed, so an
/// interruption never leaves a half-written file under the real name.
///
/// # Errors
/// See [`EngineError`]. Files that cannot be written are reported as
/// skipped, not as errors.
pub fn restore(
    profile: &Profile,
    password: Option<&Password>,
    request: &RestoreRequest,
    sink: &dyn EventSink,
) -> Result<RestoreReport> {
    sink.emit(RunEvent::Phase { phase: Phase::Opening });
    let missing =
        || EngineError::Damaged { archive: request.archive.clone(), message: "archive not found".into() };
    let repo = Repo::open(profile, false, password)?.ok_or_else(missing)?;
    let wanted: ArchiveName = request.archive.parse().map_err(|_| missing())?;
    let archives = repo.archives()?;
    let k = archives.iter().position(|a| *a == wanted).ok_or_else(missing)?;
    let manifest = repo.manifest(&wanted)?;
    let selected: Vec<_> = manifest
        .entries
        .iter()
        .filter(|e| request.keys.is_empty() || request.keys.iter().any(|p| keys::is_within(&e.key, p)))
        .collect();
    let selected_keys: HashSet<&str> = selected.iter().map(|e| e.key.as_str()).collect();

    // Newest archive up to `k` holding each selected file.
    let mut holder: HashMap<String, usize> = HashMap::new();
    for (j, name) in archives.iter().enumerate().take(k + 1) {
        let m = if j == k { manifest.clone() } else { repo.manifest(name)? };
        for e in m.entries.into_iter().filter(|e| e.stored && selected_keys.contains(e.key.as_str())) {
            holder.insert(e.key, j);
        }
    }

    let date_tag = wanted.label().replace(':', ".");

    sink.emit(RunEvent::Phase { phase: Phase::Restoring });
    let files = selected.iter().filter(|e| e.kind == EntryKind::File);
    let (total, count) = files.fold((0u64, 0u64), |(b, n), e| (b.saturating_add(e.size), n + 1));
    let mut progress = Progress::new(sink, total, count);
    let mut report = RestoreReport::default();
    let mut readers: Vec<Option<Reader>> = archives.iter().map(|_| None).collect();
    let skip = |report: &mut RestoreReport, path: &Path, reason: String| {
        let s = Skipped { path: path.display().to_string(), reason };
        sink.emit(RunEvent::Skipped(s.clone()));
        report.skipped.push(s);
    };

    for e in &selected {
        let dest = match &request.target {
            RestoreTarget::Original => keys::path_for(&e.key),
            RestoreTarget::BesideOriginal if e.kind == EntryKind::File => {
                with_date_tag(&keys::path_for(&e.key), &date_tag)
            }
            RestoreTarget::BesideOriginal => keys::path_for(&e.key),
            RestoreTarget::Folder { path } => keys::path_under(path, &e.key),
        };
        if e.kind == EntryKind::Dir {
            if let Err(err) = fs::create_dir_all(&dest) {
                skip(&mut report, &dest, err.to_string());
            }
            continue;
        }
        progress.start_file(dest.display().to_string());
        if dest.exists() && !request.overwrite {
            report.files_kept += 1;
            progress.add(e.size);
            continue;
        }
        let Some((j, name)) = holder.get(&e.key).and_then(|&j| archives.get(j).map(|n| (j, n))) else {
            skip(&mut report, &dest, "no archive holds this file".into());
            continue;
        };
        let slot = readers.get_mut(j).expect("one slot per archive");
        if slot.is_none() {
            *slot = Some(Reader::open(&repo.dir(name), repo.keys())?);
        }
        let reader = slot.as_mut().expect("opened above");
        match write_file(reader, &e.key, &dest, e.mtime_ns, e.hash.as_deref(), &mut progress) {
            Ok(()) => report.files_restored += 1,
            Err(err) => skip(&mut report, &dest, err.to_string()),
        }
    }
    progress.finish();
    sink.emit(RunEvent::Finished { summary: Summary::Restore(report.clone()) });
    Ok(report)
}

/// `C:\a\report.docx` -> `C:\a\report (2026-11-02 21.30).docx`.
fn with_date_tag(path: &Path, tag: &str) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem} ({tag}).{}", ext.to_string_lossy()),
        None => format!("{stem} ({tag})"),
    };
    path.with_file_name(name)
}

fn write_file(
    reader: &mut Reader,
    key: &str,
    dest: &Path,
    mtime_ns: i64,
    expected_hash: Option<&str>,
    progress: &mut Progress<'_>,
) -> Result<()> {
    let ctx = format!("restoring {}", dest.display());
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(EngineError::io(ctx.clone()))?;
    }
    let tmp = dest.with_file_name(format!(
        "{}.archeion-tmp",
        dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    let result = reader.read(key, |src| {
        let mut out = fs::File::create(&tmp).map_err(EngineError::io(ctx.clone()))?;
        let (_, hash) =
            copy_hashing(src, &mut out, &mut |n| progress.add(n)).map_err(EngineError::io(ctx.clone()))?;
        if expected_hash.is_some_and(|h| h != hash) {
            return Err(EngineError::Damaged {
                archive: key.to_owned(),
                message: "content does not match".into(),
            });
        }
        out.sync_all().map_err(EngineError::io(ctx.clone()))?;
        let _ = out.set_modified(system_time(mtime_ns));
        Ok(())
    });
    match result {
        Ok(()) => fs::rename(&tmp, dest).map_err(EngineError::io(ctx)),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_tag_goes_before_the_extension() {
        let p = Path::new("dir").join("report.final.docx");
        assert_eq!(
            with_date_tag(&p, "2026-11-02 21.30"),
            Path::new("dir").join("report.final (2026-11-02 21.30).docx")
        );
        let p = Path::new("dir").join("README");
        assert_eq!(with_date_tag(&p, "x"), Path::new("dir").join("README (x)"));
    }

    #[test]
    fn request_round_trips_through_json() {
        let r = RestoreRequest {
            archive: "2026-10-29_213000_C".into(),
            keys: vec!["C/Users/a.txt".into()],
            target: RestoreTarget::Folder { path: PathBuf::from("out") },
            overwrite: false,
        };
        let text = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<RestoreRequest>(&text).unwrap(), r);
    }
}
