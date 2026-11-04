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
