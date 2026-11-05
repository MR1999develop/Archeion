//! What the engine's test files share.

use std::path::PathBuf;

/// Where a test keeps its backups: `local`, or a new folder below
/// `ARCHEION_TEST_DEST` when it is set, to run the tests against another
/// drive or a network share (`$env:ARCHEION_TEST_DEST = 'E:\ArcheionTest'`).
/// The guard removes that folder when the test ends.
pub fn backups_folder(local: PathBuf) -> (PathBuf, Option<tempfile::TempDir>) {
    match std::env::var_os("ARCHEION_TEST_DEST").filter(|d| !d.is_empty()) {
        Some(root) => {
            let dir = tempfile::Builder::new()
                .prefix("archeion-test-")
                .tempdir_in(&root)
                .unwrap_or_else(|e| panic!("a folder in ARCHEION_TEST_DEST {}: {e}", root.to_string_lossy()));
            (dir.path().join("backups"), Some(dir))
        }
        None => (local, None),
    }
}

/// True when the backups go to `ARCHEION_TEST_DEST`.
#[allow(dead_code, reason = "not every test file asks")]
pub fn elsewhere() -> bool {
    std::env::var_os("ARCHEION_TEST_DEST").is_some_and(|d| !d.is_empty())
}

/// True when this Windows account has a Recycle Bin that works: a probe
/// file sent there is found in it, then removed for good. The accounts of
/// CI runners may have none; Archeion then refuses to move backups.
#[allow(dead_code, reason = "not every test file asks")]
pub fn recycle_bin_works() -> bool {
    let Ok(dir) = tempfile::tempdir() else { return false };
    let name = "archeion-test-recycle-bin-probe.txt";
    let probe = dir.path().join(name);
    if std::fs::write(&probe, b"probe").is_err() || trash::delete(&probe).is_err() {
        return false;
    }
    let parent = dir.path().to_string_lossy().to_lowercase();
    let found: Vec<_> = trash::os_limited::list()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| {
            i.name.to_string_lossy() == name && i.original_parent.to_string_lossy().to_lowercase() == parent
        })
        .collect();
    let works = !found.is_empty();
    let _ = trash::os_limited::purge_all(found);
    works
}
