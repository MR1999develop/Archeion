//! The few Windows calls Archeion needs that the standard library does not
//! make, each behind a safe function. Every other crate forbids `unsafe`;
//! this one keeps it to a line per call, with the reason it is sound next
//! to it. On other systems the functions do the nearest portable thing, or
//! nothing.

use std::{io, path::Path};

/// Renames `from` to `to` (replacing a file `to`) so that the new name is
/// on the disk when the call returns, not seconds later: the rename that
/// commits a backup must survive a power cut. On Windows `MoveFileExW` with
/// `MOVEFILE_WRITE_THROUGH`; elsewhere a rename and a flush of the folder.
///
/// # Errors
/// As `std::fs::rename`.
pub fn rename_durably(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_durably(from, to)
}

/// The file system of the volume holding `path` ("NTFS", "ReFS", "exFAT",
/// "FAT32", "FAT"); `None` when Windows does not say, and elsewhere.
#[must_use]
pub fn file_system(path: &Path) -> Option<String> {
    imp::file_system(path)
}

/// A program that comes with Windows, by its path below the Windows folder
/// (`System32\schtasks.exe`, `explorer.exe`): started by full path, never
/// looked up by name in the program's folder or the `PATH`, where a file of
/// the same name could be waiting.
#[must_use]
pub fn system_program(relative: &str) -> std::path::PathBuf {
    imp::windows_folder().join(relative)
}

/// Libraries loaded by name after this call come only from the Windows
/// system folder or the program's own, never from the current folder or
/// the `PATH`. Called first thing in `main`.
pub fn restrict_library_search() {
    imp::restrict_library_search();
}

/// Lets process `pid` bring its window to the front (a second launch hands
/// the focus to the window already open).
pub fn allow_foreground(pid: u32) {
    imp::allow_foreground(pid);
}

/// Lowers the priority of this process for work nobody is waiting for: CPU,
/// disk and memory (background mode) and energy (EcoQoS).
pub fn enter_background_mode() {
    imp::enter_background_mode();
}

/// A group of child processes that end when it is dropped or when this
/// process ends in any way, a crash included (a job object with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
#[derive(Debug)]
pub struct ChildGroup(imp::Job);

impl ChildGroup {
    /// A new, empty group; `None` when Windows refuses one, and elsewhere.
    #[must_use]
    pub fn new() -> Option<Self> {
        imp::Job::new().map(Self)
    }

    /// Puts `child` in the group.
    ///
    /// # Errors
    /// When Windows refuses (the child already ended, say).
    pub fn add(&self, child: &std::process::Child) -> io::Result<()> {
        self.0.add(child)
    }
}

/// While it lives, Windows shows `reason` when the user shuts down or signs
/// out with the window open, instead of closing it unasked.
#[derive(Debug)]
pub struct ShutdownBlock {
    _held: imp::Block,
}

impl ShutdownBlock {
    /// `hwnd` is the window's handle as a number; `None` when Windows
    /// refuses, and elsewhere.
    #[must_use]
    pub fn new(hwnd: isize, reason: &str) -> Option<Self> {
        imp::Block::new(hwnd, reason).map(|held| Self { _held: held })
    }
}

#[cfg(windows)]
mod imp {
    use std::{
        ffi::OsStr,
        io,
        os::windows::{ffi::OsStrExt as _, io::AsRawHandle as _},
        path::Path,
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Storage::FileSystem::{
            GetVolumeInformationW, GetVolumePathNameW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            MoveFileExW,
        },
        System::{
            JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            LibraryLoader::{
                LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
            },
            Shutdown::{ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy},
            SystemInformation::GetSystemWindowsDirectoryW,
            Threading::{
                GetCurrentProcess, PROCESS_MODE_BACKGROUND_BEGIN, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
                ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
            },
        },
        UI::WindowsAndMessaging::AllowSetForegroundWindow,
    };

    /// `text` as a NUL-terminated UTF-16 string.
    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(Some(0)).collect()
    }

    /// The form Windows APIs take: absolute, with `\\?\` when long (the
    /// standard library's own conversion is private).
    fn api_path(path: &Path) -> Vec<u16> {
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let text = absolute.to_string_lossy();
        if text.len() < 248 || text.starts_with(r"\\?\") {
            return wide(absolute.as_os_str());
        }
        let prefixed = match text.strip_prefix(r"\\") {
            Some(unc) => format!(r"\\?\UNC\{unc}"),
            None => format!(r"\\?\{text}"),
        };
        wide(OsStr::new(&prefixed))
    }

    /// A buffer length as the `u32` the APIs take.
    fn len32(buffer: &[u16]) -> u32 {
        u32::try_from(buffer.len()).unwrap_or(u32::MAX)
    }

    pub(super) fn rename_durably(from: &Path, to: &Path) -> io::Result<()> {
        let (from, to) = (api_path(from), api_path(to));
        // SAFETY: both pointers are NUL-terminated UTF-16 strings that live
        // until the call returns.
        let ok = unsafe {
            MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH | MOVEFILE_REPLACE_EXISTING)
        };
        if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }

    pub(super) fn file_system(path: &Path) -> Option<String> {
        let target = api_path(path);
        let mut root = [0u16; 1024];
        // SAFETY: `target` is NUL-terminated; `root` is writable for the
        // length passed.
        let ok = unsafe { GetVolumePathNameW(target.as_ptr(), root.as_mut_ptr(), len32(&root)) };
        if ok == 0 {
            return None;
        }
        let mut name = [0u16; 64];
        // SAFETY: `root` was NUL-terminated by the call above; `name` is
        // writable for the length passed; the outputs left null are
        // optional.
        let ok = unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                name.as_mut_ptr(),
                len32(&name),
            )
        };
        if ok == 0 {
            return None;
        }
        let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Some(String::from_utf16_lossy(name.get(..len)?))
    }

    pub(super) fn windows_folder() -> std::path::PathBuf {
        let mut buffer = [0u16; 512];
        // SAFETY: `buffer` is writable for the length passed.
        let len = unsafe { GetSystemWindowsDirectoryW(buffer.as_mut_ptr(), len32(&buffer)) };
        match buffer.get(..len as usize) {
            Some(text) if len > 0 => std::path::PathBuf::from(String::from_utf16_lossy(text)),
            _ => std::path::PathBuf::from(r"C:\Windows"),
        }
    }

    pub(super) fn restrict_library_search() {
        // SAFETY: takes flags only.
        let ok = unsafe {
            SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)
        };
        if ok == 0 {
            tracing::warn!(error = %io::Error::last_os_error(), "restricting the library search");
        }
    }

    pub(super) fn allow_foreground(pid: u32) {
        // SAFETY: takes a number only; a refusal changes nothing.
        unsafe { AllowSetForegroundWindow(pid) };
    }

    pub(super) fn enter_background_mode() {
        // SAFETY: returns the pseudo-handle of this process, which needs no
        // closing.
        let me = unsafe { GetCurrentProcess() };
        // SAFETY: `me` is valid for the life of the process.
        if unsafe { SetPriorityClass(me, PROCESS_MODE_BACKGROUND_BEGIN) } == 0 {
            tracing::warn!(error = %io::Error::last_os_error(), "entering background mode");
        }
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        };
        let size = u32::try_from(size_of::<PROCESS_POWER_THROTTLING_STATE>()).unwrap_or(0);
        // SAFETY: `state` lives until the call returns and `size` is its
        // size.
        let ok =
            unsafe { SetProcessInformation(me, ProcessPowerThrottling, (&raw const state).cast(), size) };
        if ok == 0 {
            // Older Windows without EcoQoS: nothing more to do.
            tracing::debug!(error = %io::Error::last_os_error(), "asking for EcoQoS");
        }
    }

    #[derive(Debug)]
    pub(super) struct Job(HANDLE);

    // SAFETY: a job handle can be used and closed from any thread.
    unsafe impl Send for Job {}
    // SAFETY: the handle is only read after creation.
    unsafe impl Sync for Job {}

    impl Job {
        pub(super) fn new() -> Option<Self> {
            // SAFETY: no attributes and no name: an unnamed job of this
            // process.
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return None;
            }
            let job = Self(handle);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let size = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()).unwrap_or(0);
            // SAFETY: `limits` lives until the call returns and `size` is
            // its size; `job.0` is a valid job handle.
            let ok = unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const limits).cast(),
                    size,
                )
            };
            (ok != 0).then_some(job)
        }

        pub(super) fn add(&self, child: &std::process::Child) -> io::Result<()> {
            // SAFETY: both handles are valid: the job's is ours, the
            // child's is kept open by `child`.
            let ok = unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle()) };
            if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the handle is ours and closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    #[derive(Debug)]
    pub(super) struct Block(HWND);

    // SAFETY: the window handle is a number; the calls work from any thread
    // of the process that owns the window.
    unsafe impl Send for Block {}
    // SAFETY: as above.
    unsafe impl Sync for Block {}

    impl Block {
        pub(super) fn new(hwnd: isize, reason: &str) -> Option<Self> {
            let hwnd = hwnd as HWND;
            let text = wide(OsStr::new(reason));
            // SAFETY: `text` is NUL-terminated and lives until the call
            // returns; a stale `hwnd` only makes the call fail.
            let ok = unsafe { ShutdownBlockReasonCreate(hwnd, text.as_ptr()) };
            (ok != 0).then_some(Self(hwnd))
        }
    }

    impl Drop for Block {
        fn drop(&mut self) {
            // SAFETY: as in `new`.
            unsafe { ShutdownBlockReasonDestroy(self.0) };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::{io, path::Path};

    pub(super) fn rename_durably(from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)?;
        if let Some(parent) = to.parent() {
            std::fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    }

    pub(super) fn file_system(_path: &Path) -> Option<String> {
        None
    }

    pub(super) fn windows_folder() -> std::path::PathBuf {
        std::path::PathBuf::from("/")
    }

    pub(super) fn restrict_library_search() {}

    pub(super) fn allow_foreground(_pid: u32) {}

    pub(super) fn enter_background_mode() {}

    #[derive(Debug)]
    pub(super) struct Job;

    impl Job {
        pub(super) fn new() -> Option<Self> {
            None
        }

        pub(super) fn add(&self, _child: &std::process::Child) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Debug)]
    pub(super) struct Block;

    impl Block {
        pub(super) fn new(_hwnd: isize, _reason: &str) -> Option<Self> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_durable_rename_moves_files_and_folders_and_replaces_files() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        std::fs::write(&a, "new").unwrap();
        std::fs::write(&b, "old").unwrap();
        rename_durably(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "new");

        let (d1, d2) = (dir.path().join("d.partial"), dir.path().join("d"));
        std::fs::create_dir(&d1).unwrap();
        std::fs::write(d1.join("x"), "x").unwrap();
        rename_durably(&d1, &d2).unwrap();
        assert!(d2.join("x").exists() && !d1.exists());
        assert!(rename_durably(&dir.path().join("missing"), &d2.join("y")).is_err());
    }

    #[test]
    fn a_long_path_is_renamed_too() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a".repeat(120)).join("b".repeat(120));
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(deep.join("f.partial"), "x").unwrap();
        rename_durably(&deep.join("f.partial"), &deep.join("f")).unwrap();
        assert!(deep.join("f").exists());
    }

    #[cfg(windows)]
    #[test]
    fn programs_of_windows_are_found_by_full_path() {
        for p in [r"System32\schtasks.exe", r"System32\WindowsPowerShell\v1.0\powershell.exe", "explorer.exe"]
        {
            let path = system_program(p);
            assert!(path.is_absolute() && path.is_file(), "{}", path.display());
        }
    }

    #[cfg(windows)]
    #[test]
    fn the_file_system_of_the_temporary_folder_is_known() {
        let dir = tempfile::tempdir().unwrap();
        let fs = file_system(dir.path()).unwrap();
        assert!(["NTFS", "ReFS", "exFAT", "FAT32", "FAT"].contains(&fs.as_str()), "{fs}");
    }

    #[cfg(windows)]
    #[test]
    fn a_child_in_a_group_ends_with_it() {
        let group = ChildGroup::new().unwrap();
        let system =
            std::env::var_os("SystemRoot").map_or_else(|| r"C:\Windows".into(), std::path::PathBuf::from);
        let mut child = std::process::Command::new(system.join(r"System32\PING.EXE"))
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        group.add(&child).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        drop(group);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            assert!(std::time::Instant::now() < deadline, "the child outlived its group");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
