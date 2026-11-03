# TIDY

Work deliberately postponed. Each item says why it waits and what would
trigger it. Remove an item when it is done or no longer applies.

## Before the first release

- **Publish on GitHub**: `tools\github_publish.bat` (needs the GitHub CLI signed in; creates the
  private repository `MR1999develop/archeion` and pushes). Then on GitHub:
  protect `main` (require the CI check before merging) if pull requests
  are used.
- **Try the git scripts on the PC** (written without a Windows shell to
  run them): `tools\git_status.bat`, then `tools\git_commit.bat` on
  `feature/automatic-backup`, `tools\github_publish.bat` (pushes `main`
  and the feature branch), `tools\git_push.bat -PullRequest`; read the
  four `.log` files.
- **Commands typed in a terminal print nothing** (3 November): the
  executable is a windows-subsystem program, so `archeion extract` run by
  hand in `cmd` or PowerShell does not show its output (it does through a
  pipe or redirection, as the window, Task Scheduler and the tests use
  it). Fix with `AttachConsole(ATTACH_PARENT_PROCESS)` at start, which
  needs a Windows API call (no `unsafe` in our crates: a small crate or a
  separate console helper).
- **Older versions and dates cannot be chosen in the window** (3
  November): the restore cards were removed on request, so a file comes
  back as its newest version and a folder as in its newest backup. An
  older one is still reachable with `archeion extract --archive`. If it
  is missed, a simple way back is a date chooser for the whole selection.
- **A Recycle Bin too small for the backups** (3 November): Windows may
  delete outright an item larger than the Recycle Bin allows, and the
  engine finds out only afterwards (the item is not in the Recycle Bin)
  and stops there, so that one archive is lost. Checking the size against
  the Recycle Bin's limit needs `SHQueryRecycleBinW` (no `unsafe` in our
  crates: a small crate would do).
- **Restore summary without deleted files** (3 November): the size above
  Restore counts a ticked folder's files at its date, not the deleted ones
  it now brings back too. Add them when the folder's files are read.
- **Screenshots not compared automatically** (2 November): the views are
  rendered with `cargo test -p archeion --bin archeion screens -- --ignored`
  into `target/ui-screens` and looked at, with no stored references: add
  `harness.snapshot()` with references if the look starts to drift.
- **Icons on Windows 11** use Segoe MDL2 Assets, which Windows 11 still
  ships, not the newer Segoe Fluent Icons: its glyphs differ at some code
  points and could not be checked here (Windows 10). Switch once checked
  on Windows 11 by drawing every `ui::symbol` constant.
- **Checkboxes drawn by egui then painted over** (`widgets::checkbox`,
  `partial_checkbox`): the accent fill covers egui's box. If egui's
  checkbox layout changes, the fill can be misplaced: the screenshots
  show it.
- **Accessibility of the filled box**: egui exposes a partial tick box as
  neither ticked nor unticked (no "mixed" state in `WidgetInfo`), so a
  screen reader does not say "partly ticked". Add it to the hover text or
  label if a screen-reader user needs it.
- **Try the window by hand**: the tests cover the engine, not the egui
  screens. Create a profile, back up, browse versions, merge, restore.
  Check also the Activity tab during a long backup, the exclusion list
  (folder, file, type, pattern) and the format radios locked after the
  first backup.
- **Rebuild the release executable** (`tools\release.bat`) once the
  `archeion.exe` window is closed: Windows locks a running executable.

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
- **Files of the full log** (added 2/11): the time of each file is the
  time its batch was written (at most about a second late); the merge
  lists every file carried as moved, even the ones a merged incremental
  only keeps as chunks; skipped files are written to both the run log and
  its `.files.jsonl` (small duplication, keeps the order); notes of
  damaged files are in English like the other check problems; the window
  has no "Show all files" view of its own (the full log is it); a full
  log of hundreds of thousands of files is built in memory as one string
  (tens of MB) before it is written. Revisit if a log grows too large to
  open comfortably.
- **Ticking a folder** restores the whole folder as it was at the date
  chosen on its card; a "restore only what this backup saved" option
  would need its own request field.

## Free space and resumed copies

- **Space estimate for zips** counts deflated files at half their size. A
  folder of incompressible files with unknown extensions passes the check
  and is stopped by the full-disk test during the copy instead (resumable,
  so nothing is lost but time).
- **The 64 MiB margin** refuses even a tiny incremental on a nearly full
  drive. Make it proportional if users hit it.
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

## Scheduled tasks and free space (added 01/11)

- **Task check at startup** runs once per window start, on a snapshot of
  the profiles; a schedule edited in the first second may be applied twice
  (harmless). Problems show in the error bar, only the first one.
- **Merge does not check free space first**: it mostly hard-links files,
  so it needs little room; a full disk fails with the OS message.
- **Free space is shown in the settings only**, not in the Backups tab, and
  is read again only when the destination changes or a run ends.
- **Restore space check** counts a replaced file in full (it is written
  beside the old one first) and ignores the 64 MiB margin's cost on small
  restores to a nearly full drive.

## Automatic backup: schedules, events, notifications (added 01/11)

- **Notifications come from "Windows PowerShell"**: without an installer
  there is no registered application id. The installer should create a
  Start menu shortcut with an AppUserModelID and the toast should use it;
  clicking the toast could then open the window.
- **Drive-connected trigger** listens to event 1006 of the
  Partition/Diagnostic log (on by default since Windows 10). Not tried on
  a real PC yet; if `schtasks` refuses it for a standard user, the error
  shows in the window. Network destinations never trigger it.
- **Idle trigger**: Task Scheduler's idle detection (10 minutes, wait up
  to an hour); a run is not stopped when the user comes back.
- **Event runs skipped** (too recent, destination missing) leave no line
  in the operations log, only in the diagnostic log, to avoid a line per
  idle period.
- **"No backup for N days"** ignores profiles never backed up and warns at
  most once a day per profile; it reads only the newest 500 logs.
- **Legacy texts** `schedule-daily` and `schedule-weekly` are unused in the
  locales; remove them with the next locale clean-up.

## Fixes of 01/11 afternoon

- **No test simulates a file held open by another process** during a
  merge (Rust opens files with delete sharing, so the error cannot be
  reproduced in a unit test); tried by hand only.
- **`.old` and `.deleting` folders still in use** are retried at every
  opening of the destination, up to about 5 seconds each time.

## Automatic merge from the window (added 01/11)

- **Merges only while the window is open**: a PC where Archeion is never
  opened never merges. If that matters, a scheduled `archeion merge
  --scheduled` could come back as an option.
- **One profile per look**: with several profiles due, the others are
  merged at the next looks (after each merge's end the next look comes in
  15 minutes; could be sooner).
- **The busy check is a snapshot** of the lock; a backup starting at the
  same moment makes the merge report "busy" in the operations log.

## Settings layout and event backups (added 01/11 evening)

- **Event backups are hidden, not removed**: the engine, the `--event`
  worker path, the scheduler triggers and their tests stay; the window
  registers no event task. Remove the code (and the `event-*` texts) if
  they are not coming back.
- **Section summaries**: a closed section shows only its title; a short
  summary next to it (for example "Every day at 12:30") would help.
- **Taking over a destination** is offered only when the owner profile no
  longer exists in this window's settings; a profile of another Windows
  user or PC shows up as "deleted" too.

## Git and GitHub

- **CI runs on Windows only.** The engine and core crates are portable;
  add an Ubuntu job if a Linux build is ever wanted.
- **Release zip is portable only**; the installer is still to do (see
  "Features planned"). Code signing of `archeion.exe` is not set up, so
  SmartScreen warns on first run.

## Name
- **Trademark check** was only a web search (no software or backup product
  called Archeion found; Archeion is also the Archives Association of
  Ontario's archival portal). Search TMview before a public release.

## Tray, start with Windows, merge of recent backups (added 01/11 evening)

- **Starting hidden flashes the window for one frame**: eframe shows the
  window after its first paint, and the hide command is processed right
  after. A fix needs eframe to honour `with_visible(false)`.
- **Tray tooltip and menu texts** are set once at startup: changing the
  language does not update them until the next start.
- **Disabled in Task Manager**: if the user turns off "Archeion" under
  Startup apps, Windows keeps our Run value but skips it; the Preferences
  window still shows "Start with Windows" ticked. Read
  `Explorer\StartupApproved\Run` to show the real state.
- **Single window across users**: the lock and port files are in the
  per-user data folder, so two Windows users each get their own window,
  as intended; a portable copy with its own `data\` folder is a separate
  instance too.
- **Complete backups made by hand** are merged a day later like any other
  backup, by the daily rule. If users expect them kept, add a rule "keep
  complete backups made by hand" or make the daily rule skip them.
- **Merge rules editor**: "All, for days" plus five period rules is a lot of
  numbers; consider presets ("Keep everything", "Balanced", "Save space").

## Full log of an operation (added 01/11)

- **Text logs in the temporary folder** (`%TEMP%\Archeion\<run>.txt`) are
  rewritten at every "Open full log" and never deleted by Archeion: at
  most one per run, small, and Windows Storage Sense clears the folder.
  Delete those older than a week at startup if anybody notices them.
- **Huge logs**: the text is built and written on the UI thread when the
  button is clicked; fine for thousands of lines, move it to a thread if
  a run with hundreds of thousands of skipped files makes the click lag.

## Large files as chunks, `archeion extract` (added 01/11 night)

- **Not compiled with the real crates**: written in a container without
  crates.io. The chunker and the planning logic were tested with plain
  `rustc --test`; core and engine (with their tests) were type-checked and
  linted (`cargo clippy -D warnings`) against stub crates that mirror the
  APIs used. The app changes (`extract.rs`, editor, help) were reviewed by
  hand only. First `tools\check.bat` on Windows: expect small fixes.
- **Manifests repeat chunk lists**: each archive's manifest carries the
  full chunk list of every file it stores (~80 bytes per chunk), and the
  window reads every manifest; 1 TB of chunked data in one complete
  archive is ~80 MB of manifest. If that shows, move the lists to a side
  file read only by restore, merge and verify.
- **Every chunked backup reads all manifests** to know which chunks the
  destination holds. A destination-level chunk index (rebuilt when
  missing) would avoid it for destinations with hundreds of archives.
- **`bytes_stored`** of an archive (Backups tab) counts a chunked file at
  its full size, not the chunks it added. Sum the chunk files' sizes when
  listing if users find it misleading.
- **Out-of-order ranges**: a restored file whose chunks point backwards in
  the complete archive's whole copy reopens the copy for each jump back
  (correct, slow in pathological cases). Sort by source offset and write
  at output offsets if it ever matters.
- **Unused chunk files outside a merge group** stay until their own
  archive is merged; a "compact" command could collect them.
- **Chunk files are not flushed one by one** before the commit rename,
  like whole copies. A resumed backup re-reads the chunks it reuses, but
  a power cut right after a commit could leave a chunk file of zeros:
  the integrity check finds it. A flush of the volume at commit would
  close this (it needs care on network shares).
- **`archeion extract` texts are English** and its output goes nowhere
  when started from a Command Prompt without `start /wait` or a pipe,
  because `archeion.exe` is a GUI-subsystem program (attaching to the
  parent console needs `unsafe` Win32 calls, forbidden here). The log
  `archeion-extract.log` in the target folder always has everything.
  Translate the lines with `Tr` (system language) if users ask.
- **`extract` is read-only**: it takes no lock and does not complete an
  interrupted merge; run while no backup of that destination is running.
- **Merge reads every manifest of the destination per group** to compute
  which chunks stay. Fine for the usual handful of groups.

## Restore redesign (added 02/11)

- **Try the new restore view on the PC**: tick files and folders in the
  tree and in search results, tick several versions, change a version's
  destination (✎), the folder date combo, the conflict with two
  "Overwrite the original", the Restore reason texts, and Manage backups
  (merge, delete). The egui code was type-checked against stubs only.
- **A ticked folder does not tick what it holds**: its files keep their
  own checkboxes, and a file ticked separately is restored twice (with
  dates in the names when both go to the same folder). Show the children
  as included, or warn, if it confuses.
- **Names of a folder's files** are decided by the engine: the card shows
  the name only for ticked file versions. A folder and a version of a file
  inside it sent to the same place both get dates in the names.
- **The tree reads every manifest** once per profile (kept until a backup,
  merge or deletion, or ⟳); the versions of each ticked file read them
  again. Keep the versions in the catalog if that is slow on destinations
  with many backups.
- **`archeion_engine::search` and `SearchHit`** are no longer used by the
  window (the search runs on the tree); remove them or use them for a
  command-line search.
- **A folder override to another folder never replaces** existing files
  there (the default's "Replace files that already exist" applies only to
  what follows the default). Add a checkbox to the override menu if asked.
- **Search matches file names only**, as before; folders could match too.
