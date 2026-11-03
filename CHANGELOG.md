# Changelog

All notable changes to Archeion are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- File > Export profile… saves every rule of the selected profile to a
  readable `.archeion-profile` file (never the password); File > Import
  profile… adds it, with its scheduled tasks. On another computer the
  imported profile keeps its id and so finds its backups at the
  destination; imported twice on the same one it becomes a copy with
  "(2)" in its name. Files of another kind, too large or of a later
  format are refused with a message. In the guide.
- The window opens at the size it last had, maximized if it was; the
  size is saved a second after resizing stops, or when the window closes.
  The first time it opens at 1200 × 1000, enough for every setting of a
  profile without scrolling, or maximized on a screen too short for it
  (Full HD included). Controls
  and rows are a little closer together.
- The profile settings are more compact: the name is a row ("Name" and
  its field) instead of a section, sections sit closer, the free space
  of the destination is beside its folder, and the form is wider so its
  hints fit on one line.
- `ARCHEION_DATA_DIR` names another data folder (settings and logs); the
  commands without a window read the password of an encrypted profile from
  `ARCHEION_PASSWORD` when none is saved. Both serve the end-to-end tests,
  which run the executable on temporary folders, and the tests that drive
  the Restore files view and check that every character of the interface
  can be drawn.

### Changed

- Built with Rust 1.99.1, pinned in `rust-toolchain.toml` and in CI (the
  minimum version is 1.99). Dependencies updated, including toml 1,
  argon2 0.6 (same keys: checked against the reference implementation)
  and getrandom 0.4; fs4 replaces the unmaintained fs2 for the free
  space. CI actions updated to their Node 24 versions.

- A cleaner, more uniform window: Segoe UI (regular and semibold) from
  Windows instead of egui's thin font; Windows 11 greys with one blue
  accent for what can be clicked, in light and dark; the profile name as
  page title; tabs as text with an accent bar, so they no longer look
  like selected list items; semibold section titles everywhere; one
  primary button per view and a red one only to delete; Preferences, Help
  and About open centred, with window margins. Text, links and field
  borders keep the WCAG contrasts in both themes (tested). Fields and
  menus without a label beside them have an accessible name.
- Icons come from the Windows icon font (Segoe MDL2 Assets) instead of
  emoji, so they match the system and each other. Ticked boxes are filled
  with the accent; a partly ticked folder shows an accent square. Lists
  (profiles, operations in Activity) highlight the whole item with an
  accent mark, and each operation shows its date and profile on a second
  line of the same item. Remove, close and refresh are quiet icon buttons.
  In Activity the log links sit beside the "Log" title instead of
  wrapping under the buttons, and the log rows no longer have stripes.
- Encrypted destinations use new key derivation contexts (key format 3):
  those created before cannot be opened any more and must be created
  again. Settings, saved passwords and scheduled tasks of earlier test
  builds under another name are no longer taken over.
- Restore files: ticking a folder ticks everything it held at the date it
  comes back as (files missing from that backup stay unticked and greyed)
  and replaces what was ticked inside it. Unticking a file or folder
  inside it leaves it out of the restore; the ticked folder and the
  folders between then show a filled box, and clicking a filled box ticks
  the whole folder again. The folders above a ticked file or folder show
  a filled box too. The folder's card shows how many items are left out
  and its size without them. With several
  items in the selection, only the highlighted card (the last ticked, or
  the one clicked) shows its versions, date and destination; the others
  show one line with what goes where.
- Archeion is now licensed under the GNU General Public License,
  version 3 only (`LICENSE`), instead of MIT or Apache-2.0. The release
  build leaves no local folder in `archeion.exe`.

- Restoring is redesigned. Backups > "Restore files" shows one tree of
  every file and folder the profile's backups ever saved, with the search
  above it; files no longer at the source are marked "deleted" (icon and
  text). Files and folders are ticked in the tree or in the search
  results, and each appears as a card on the right: a file with its
  versions (the newest ticked, more can be ticked), a folder with the date
  it comes back as. A default destination at the top (next to the
  original with the date, original location, another folder; "Replace
  files that already exist") applies to everything, and each version or
  folder can have its own (marked ✎). Several versions of a file sent to
  the same folder each get their date in the name; two versions
  overwriting the same original are refused until one changes. A summary
  ("3 files, 5 versions, 1 folder — 1.2 GB") sits above the Restore
  button, which says why it waits. Everything restores in one run, with
  one free-space check, one progress and one log.
- The list of backup dates with the manual merge and the deletion of one
  backup moved to Backups > "Manage backups", unchanged.
- Restore requests hold a list of items (backup, file or folder, target,
  replace); request files written by the previous version are still read.

- New backups are written in format 2 (manifests may list chunks); older
  versions of Archeion cannot read them. Backups of earlier versions stay
  readable as they are; on such a destination the next automatic backup
  is complete when large files would be chunked, and chunks start there.

- "Delete profile" in the profile settings is a button at the far end of
  the Run row and opens a warning first: it says what is removed (the
  profile, its scheduled tasks, the saved password of an encrypted profile)
  and that the backups stay at the destination. Enter does not confirm it.

- The merge rules keep every backup of the last N days ("All, for days",
  1 by default), so backups made minutes apart are no longer merged right
  away; a day later the other rules apply as before.
- Closing the window hides Archeion in the notification area, where it
  keeps working; File > Quit or the icon's Quit closes it. Archeion starts
  with Windows, hidden. Both are in the new Preferences window
  (File > Preferences…, Ctrl+,), which also chooses the interface language
  (or follows the Windows one) and shows the data folder.
- Starting Archeion while it already runs brings the open window forward
  instead of opening a second one.
- The Activity tab shows the run in progress while it runs; once it ends
  it becomes the newest entry of the log. The "Last operation" entry, the
  "Show backups" button and the repeated outcome line are gone.
- While a run is in progress the Activity tab lists each step with its
  start time, duration and what it did so far (files and folders found,
  new, changed and unchanged files, files moved, restored, kept, checked,
  problems, skipped), the group of backups a merge is fusing, and the
  latest files handled with what happened to each.
- The password of an encrypted profile shows its requirements while it is
  typed (at least 12 characters, both entries equal) and what is still
  missing next to the Save button.
- A new password must also mix three kinds of characters (not needed for a
  passphrase of 20 characters or more), not be a common password, have no
  runs like "1234", not contain the profile name and be at least "Fair" on
  the new strength bar.
- The "Monday to Friday" shortcut of the schedule is gone.

- The profile settings are in collapsible sections, each remembering
  whether it is open; a section with something to fix (a destination of
  another profile, a missing password) stays open.
- Backups started by events (logon, idle, drive connected) are no longer
  offered in the window.

- The merge of old backups is always automatic and has no days, times or
  switch: it runs by its rules while Archeion is running (shortly after a
  backup and every 15 minutes), only when no backup or other operation
  uses the destination. The merge tasks of earlier versions are removed
  from Task Scheduler.
- "Exclude patterns" sits next to "Exclude folders" and "Exclude files"
  instead of under "Advanced", with the wildcards explained.
- Closing the window while a backup, merge or other operation runs no
  longer cuts it short: a message says Archeion will close when it ends,
  and it does (or the user chooses "Do not close").

### Added

- Large files are saved as changes only: files of at least a size chosen
  in the profile's Storage section (500 MB unless changed, or never) are
  cut into content-defined chunks of about 1 MB, and an incremental
  backup stores only the chunks the destination does not hold yet (one
  copy of identical chunks). On an encrypted destination every file is
  saved this way, each chunk an AES-256 zip that 7-Zip opens with the
  recovery key. Complete backups still keep every file whole, so they open
  with 7-Zip alone. Restores, versions, merges, deletion, the integrity
  check (which reads every chunk), the restore test, resuming an
  interrupted backup and the free-space estimate (new chunks only) all
  handle chunked files. The backup summary says how many files were saved
  as changes only. Design in `docs/large-files.md`.
- `archeion extract --destination <folder> --to <folder>` restores from a
  backup folder alone, without settings or a profile (also `--archive`,
  `--path`, `--overwrite`, and the password or recovery key from
  `--stdin`, `--password`, `--recovery-key` or `ARCHEION_PASSWORD` /
  `ARCHEION_RECOVERY_KEY`), so the portable executable recovers backups on
  any computer. In the guide and the README.
- Developer scripts for git and GitHub: `tools\git_status.bat` (overview,
  read-only), `tools\git_commit.bat` (check, then commit on a feature
  branch with the project's trailers) and `tools\git_push.bat` (push and
  pull request); `tools\github_publish.bat` now logs every step, also
  pushes the current branch and can be run again.
- The Activity tab shows an operation's full log: "Open full log" writes
  it as readable text (date and time, severity, every step, every skipped
  file with its reason, every problem a check found) and opens it in the
  default text editor; "Copy log" copies the same text. "Program log" and
  "Logs folder" open Archeion's technical log and the logs folder.
- The full log of an operation lists every file it worked on and what it
  did with it, after the steps: time, what happened (new, changed, saved
  as changes with the bytes of new chunks written, copied again by a
  complete backup, deleted since the previous backup, moved into a merged
  backup, older version removed by a merge or deletion, restored, kept,
  checked by a restore test, problem, skipped with the reason), size and
  full path, in the order of the run, with the count of each at the top.
  Unchanged files of an incremental backup and intact files of a full
  check are only counted. The list is kept beside each run log in
  `logs\<run>.files.jsonl` and removed with it; operations made by earlier
  versions say they have no list. The full log is prepared in the
  background, with a spinner when it takes more than a second.
- Automatic backups on chosen days of the week at up to six times a day,
  every 1 to 12 hours within a time window, or once a month; schedules
  saved before keep working.
- Backups started by events: a few minutes after signing in to Windows,
  when the computer is idle, when a drive is connected; skipped when the
  last backup is recent or the destination is not connected.
- Windows notifications when an automatic backup fails, and when a profile
  has had no backup for a chosen number of days (`archeion watch`, run
  daily and after signing in).

- Free space of the destination shown under it in the profile settings,
  as a warning below 2 GB.
- A restore that does not fit on its drive stops before writing anything,
  with a message saying how much is needed and free.
- At startup the scheduled tasks are checked in the background and
  registered again when missing or pointing to another `archeion.exe`
  (program moved, portable copy started from another folder).

### Fixed

- A profile re-created with the destination of a deleted profile failed
  with "the destination is used by another profile": the settings now say
  so and offer "Use these backups for this profile".

- Merging (or deleting) backups failed with "access denied" when Windows
  still held a file of the backup open for a moment (antivirus, indexer,
  thumbnails): renames and removals are now retried for a few seconds, and
  an old folder still in use is removed by the next run instead of failing
  a merge that is already complete.
- Scheduled tasks were never registered: `schtasks` could not read the
  task file while Archeion still held it open.
- The backup button of a new encrypted profile stayed disabled until the
  password was saved with its own button; it now saves the typed password
  and starts the backup, and says what is missing.

- Restore requests left behind by a worker that never started are deleted
  after a day.
- The destination hint no longer says that backups are never encrypted.

## [0.1.0] - 2026-11-01

First version, not yet released.

### Added

- Backup profiles with sources, exclusions (folder, file, file type,
  pattern), destination on a local, external or network drive, and a
  daily or weekly schedule run by Windows Task Scheduler.
- Archive format in the style of Areca: a complete backup followed by
  incrementals, each with a full manifest; one zip per file or plain
  copies, readable without the program.
- Incremental, complete or merge on demand; a rule for automatic complete
  backups; automatic merge by retention rules that never touches the first
  complete backup; manual merge of a chosen range; deletion of a single
  backup.
- Optional encryption: AES-256 zip per file with hidden names, random master
  key wrapped with Argon2id, recovery key that opens the zips with 7-Zip,
  password kept in Windows Credential Manager.
- Restore of files or folders at any date, file versions, search; Backups
  tab with the folder tree of each date.
- Integrity check and restore test, by hand or after scheduled backups.
- Activity tab with a log of every operation.
- Free-space check before copying; an interrupted backup resumes where it
  stopped.
- English and Italian interface.

[Unreleased]: https://github.com/MR1999develop/archeion/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/MR1999develop/archeion/releases/tag/v0.1.0
