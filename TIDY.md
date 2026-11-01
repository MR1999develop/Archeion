# TIDY

Work deliberately postponed. Each item says why it waits and what would
trigger it. Remove an item when it is done or no longer applies.

## Before the first release

- **Delete two obsolete files** left from the restic version, now one-line
  placeholders: `app/src/secrets.rs` and
  `crates/engine/tests/local_repository.rs`.
- **Initialise git and publish on GitHub**: `tools\git_init.bat` (deletes
  the two placeholders, creates `main`, first commit), then
  `tools\github_publish.bat` (needs the GitHub CLI signed in; creates the
  private repository `MR1999develop/archeion` and pushes). Then on GitHub:
  protect `main` (require the CI check before merging) if pull requests
  are used.
- **Try the window by hand**: the tests cover the engine, not the egui
  screens. Create a profile, back up, browse versions, merge, restore.
  Check also the Activity tab during a long backup, the exclusion list
  (folder, file, type, pattern) and the format radios locked after the
  first backup.
- **Rebuild the release executable** (`tools\release.bat`) once the
  `archeion.exe` window is closed: Windows locks a running executable.
- **Old `target\release\archeion.exe`** stays after the rename to
  Archeion; delete it by hand (or with `cargo clean`) so nobody starts it.
- **Repository folder** on the PC is
  `the repository folder` (since 1
  November; before, `another folder` on another PC).
- **`portable.txt` and scheduled tasks.** Tasks store the executable path;
  a moved portable copy breaks them. At startup compare the registered path
  with `current_exe()` and offer to re-register the tasks.
- **Restore request file cleanup.** The window writes `restore-<n>.json` in
  the data folder and the worker deletes it; if the worker never starts, the
  file stays. Remove leftovers older than a day at startup.

## Encryption, backup kinds and deletion (added 30/10)

- **Integrity checks run only after scheduled backups**: a profile backed
  up by hand gets its checks only from the buttons. A separate scheduled
  task would cover it.
- **Check problems are in English** (they come from the engine as text);
  give them a kind and translate them.
- **No notification** when a scheduled check finds problems: it shows in
  the operations log only. Comes with the tray icon and notifications.
- **Sizes of encrypted files** stay visible (each zip is about as large as
  its file). Padding would hide them at a cost in space; not done.
- **Argon2id cost** is fixed at 64 MiB / 3 passes; measure on slow PCs and
  make it adaptive if unlocking takes more than a second.
- **Deleting a profile keeps its saved password** in Credential Manager
  while Undo is offered; it is removed when the Undo bar is closed with ✕,
  but not when the window is closed with the bar still showing.
- **Deleting a later complete backup** is refused; the user must merge it.
  Allow it by merging it with the next archive if that is ever asked for.
- **Skipped files in a complete backup** keep pointing to the older
  archive, so that "complete" archive is not fully self-contained.

## Operations log and backup contents (added 30/10)

- **Long logs**: a run that skips thousands of files draws every line in
  a grid; switch to `ScrollArea::show_rows` if it gets slow.
- **Log of a run killed with the window closed**: it stays "did not
  finish"; Task Scheduler runs that are still going look the same.
- **Ticking a folder** restores the whole folder as it was at that date,
  greyed (unchanged) files included; a "restore only what this backup
  saved" option would need its own request field.

## Free space and resumed copies

- **Space estimate for zips** counts deflated files at half their size. A
  folder of incompressible files with unknown extensions passes the check
  and is stopped by the full-disk test during the copy instead (resumable,
  so nothing is lost but time).
- **The 64 MiB margin** refuses even a tiny incremental on a nearly full
  drive. Make it proportional if users hit it.
- **Merge and restore** do not check free space first: a full disk during a
  merge fails with the OS message ("Failed"), not "not enough space".
- **Free space is not shown in the window** (profile settings or Backups
  tab); add it next to the destination when convenient.
- **A resumed backup keeps the name (date) of the interrupted one**: the
  encrypted stored names depend on it. Its content is as of the resumed
  run, a few hours or days later; the Backups tab shows the older date.
- **A file larger than the free space** is restarted from zero every run:
  resumption works per file, not inside a file.
- **The journal is not flushed per line**: after a power cut a few
  completed files may be copied again.

## Features planned in later milestones

- **Restore browser extras**: compare two versions of a file, preview of
  text files and images, restore a whole date in one click.
- **Background run**: tray icon with progress and Cancel, Windows
  notifications on errors, hourly retry when the destination is missing.
- **Remote copy (cloud, SFTP)**: undecided. Proposal: mirror the per-file
  zips with rclone after each backup and merge.
- **Installer and portable zip**, Segoe UI font, application icon (`.ico`
  with 16/24/32/48/256 px).
- **7z per file** as an alternative to zip, if the Rust 7z crates prove
  reliable enough.

## Design debts

- **Large files changed often** (for example Outlook `.pst`) are copied
  whole at every backup. Measure real cases before adding a delta mode.
- **Change detection uses size + modification time only.** A file rewritten
  with the same size and timestamp is missed. A "deep check" option hashing
  every file would close the gap at a cost.
- **Search reads every manifest** on each query. Fine for hundreds of
  archives; cache the manifests in memory if it becomes slow.
- **`Writer::finish` does nothing** for both remaining containers; remove it
  or give it a purpose (for example flushing the folder on Linux).
- **Format lock check reads the destination** in a background thread each
  time a profile is opened; on a slow or offline share the radios show
  "checking" until it answers. Acceptable; cache per profile if annoying.
- **Cancel kills the worker process**: a cancelled backup is resumed by
  the next one, but a cancelled restore may leave the files it
  already wrote. Document it in the restore dialog or restore into a
  staging folder first.
- **`app/src/ui/app.rs` is the hotspot**: split status bar and menus into
  their own modules when it next grows.

## Git and GitHub

- **CI runs on Windows only.** The engine and core crates are portable;
  add an Ubuntu job if a Linux build is ever wanted.
- **Release zip is portable only**; the installer is still to do (see
  "Features planned"). Code signing of `archeion.exe` is not set up, so
  SmartScreen warns on first run.
- **`tools\probe.bat`** is a leftover of the restic experiments; delete it
  together with its log when convenient.

## Rename to Archeion

- **Migration code** for Archeion names (destination files, data folder,
  Credential Manager entries, scheduled tasks) can go once no copy of the
  old name is in use; keep the key derivation contexts, which still say
  "Archeion" on purpose: changing them would make encrypted backups
  unreadable.
- **Trademark check** was only a web search (no software or backup product
  called Archeion found; Archeion is also the Archives Association of
  Ontario's archival portal). Search TMview before a public release.
