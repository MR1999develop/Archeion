# Project status

Updated at the end of every block of work (see `CLAUDE.md`).

## Last checked commit

`main` with the branches of the security review (`fix/security-review-2`:
network folders to be confirmed, restore limited by size, password
rules), of the supply chain (`chore/supply-chain-2`: daily check, SBOM,
optional signing), of the property tests (`test/property-parsers`:
proptest on the readers of manifests, keys, worker lines, typed
exclusions, hex and sealed notes, profile files) and of the third
security review (`fix/security-review-3`, November 4: merge journal with
archive names only, list signature checked at every opening, network
paths in every spelling, links and junctions not followed, small files
limited to 1 MiB, password left by a deleted profile, constant-time
comparisons, release in two jobs; tests in `crates/engine/tests/hostile.rs`
and in `app/tests/cli.rs`) and of the merge recovery
(`fix/merge-recovery-copy`: an unreadable merged archive is set aside as
`.damaged` instead of causing the newest member to be deleted, a case
reported by the OSTEP review) and of the fix to `tools\github_publish.ps1`
(`fix/publish-script-parse`: syntax error at line 69, `$Repo:`; new step
6/6 of `tools\check.bat` and of the CI, `tools\script_check.ps1`, which
parses every PowerShell script without running it) and of the exact
repository name (`chore/repository-name`: `MR1999develop/Archeion` with
the capital letter, as on GitHub, in `origin`, `Cargo.toml`, README,
SECURITY, CHANGELOG, Help window and scripts) and of the profile copy at
the destination (`feature/profile-copy`: every backup writes
`archeion.profile.toml`, or a sealed `archeion.profile.zip` if encrypted;
File > Import profile from backups… reads it; text format moved to
`archeion_core::profile_text`; tests in
`crates/engine/tests/profile_copy.rs`, `app/src/ui/backup_import.rs` and
`app/tests/cli.rs`), of the full import test (`test/full-import`) and of
the profile list (`feature/profile-list`, November 5: order by dragging
and with Alt+Up/Down, right-click menu and File menu with Rename in place
and Delete profile…, no more button below the settings; module
`ui/profile_list.rs`, tests in `app/src/ui/app_tests.rs`) and of the
confirmation of the merge rules (`feature/merge-preview`:
`MergePolicy::approved`, `merge_plan::preview`, `archive_names`; tests in
`merge_plan.rs`, `crates/engine/tests/merge_wait.rs` and `app_tests.rs`)
and of the automatic merge switch (`feature/merge-switch`:
`MergePolicy::enabled`, `ConfirmedMerge` with on/off and rules; the
window also receives the worker program, `worker_exe`, so tests never
start themselves; test of "Merge now" counting as confirmation);
`tools\check.bat` green.
On GitHub (November 4) `main` is at `542a215`; the rest goes out with the
next push, which the user runs.

## Work in progress

- None.

## Open points

- Try by hand: Cancel during a backup (stops at the next file), shutdown
  with a backup running (Windows message), second launch bringing the
  window forward, imported profile with the schedule paused, warning on
  FAT32/exFAT drives.
- Measure on a slow USB disk the cost of the per-file flush (TIDY).
- Release 0.2.0 published on GitHub on November 5 by the workflow
  (Release #1 and CI #7 green; public repository): portable zip,
  SHA256SUMS, SBOM. The downloaded zip matches its hash, starts and has no
  local paths. Not signed (no certificate); attestations to verify with
  `gh attestation verify` (TIDY).
- Measure on a NAS the cost of checking the real path of files read from
  plain backups (TIDY).

## Decisions

- The destination is untrusted also for where things are read and written
  on the PC (November 4): the merge journal holds only `ArchiveName`
  (serde in `archeion_core::archive`), recovery touches only
  `<archive>.old/.merging/.deleting` folders, keys are opened before
  recovery. Links and junctions: an interrupted backup that contains any
  is not resumed; plain files are read only if their real path
  (`archeion_win::real_path`) is inside the destination. The list
  signature is checked at every opening: backup, merge and delete warn
  (`changed_outside` in the reports, notification and note in the
  summary), then sign the list they found; recovery does not re-sign a
  list that does not match.
- GitHub with a fine-grained token for Archeion only (November 4), like
  the other project: Git Credential Manager keeps it for the repository
  URL (`useHttpPath` in `.git/config`), `gh` keeps the other project's
  token and is used for Archeion only to read workflow runs (CI status,
  failed-step logs; since November 7), never to push, publish or change
  the repository; with `origin` set, `tools\github_publish.bat` does not
  call `gh`. The repository is named
  `Archeion` with the capital letter: every reference uses that exact
  name.
- Profile settings live on the PC, the source of truth (November 4); every
  backup leaves a copy at the destination (written at opening, after
  recovery: temporary file, flush, rename), sealed with the key if the
  destination is encrypted, and removes the copy of the other kind. The
  copy is read only with File > Import profile from backups…, on a folder
  the user chooses: the id is the owner's in `archeion.json`, the
  destination is the chosen folder, the encryption is the destination's;
  then the same protections as profile files (`profile_file::add`). The
  password that opens the backups is saved with the profile. Rejected:
  settings only at the destination (untrusted, unreachable when the disk
  is detached, anti-rollback memory that must stay on the PC) and a copy
  per archive (more complex with merges and deletions, with no current
  need).
- Merge rules (November 5): new rules that would merge backups kept by the
  confirmed ones wait (`merge.approved` holds the confirmed ones) and the
  automatic merge does not start until the user chooses "Merge with the
  new rules" or "Restore the previous rules"; "Merge now" counts as a
  confirmation. Wider rules, or narrower ones with no effect on the
  existing backups, confirm themselves; with backups not yet read a change
  waits. The automatic merge can be turned off (`merge.enabled`, on for
  existing profiles): off, it merges nothing, not even `engine::merge`,
  and "Merge now" is not offered; turning it back on is like narrowing
  the rules.
- One window per Windows user, whichever copy is started (November 5):
  `window.lock` and `window.port` are in `%LOCALAPPDATA%\Archeion`
  (`instance::user_dir`) and also in the data folder, where versions up to
  0.2 look for them (tried with the real executables: 0.2.0 and the new
  one in both orders, a single window); with `ARCHEION_DATA_DIR` they stay
  in the test folder. A second, different copy brings the window forward
  and says so with a message (path of the open copy, File > Exit).
- Duplicate profile (November 5): `Profile::copy_named` keeps every rule
  except the destination (one folder per profile), pauses the schedule
  and clears the merge confirmations; the copy goes right below the
  original with the next free name ("(copy)", "(copy 2)"…).
- The window receives the Task Scheduler as a dependency (`Tasks`, trait
  `archeion_scheduler::Scheduler` with `apply_watch`): window tests use a
  recorder and never touch the user's tasks. The full test of the
  encrypted import
  (`an_encrypted_profile_comes_back_from_its_backups_with_its_password`)
  writes to the real Credential Manager with the random id of the test
  profile and deletes it at the end (accepted by the user on November 5).
- Recovery never deletes the only copy (OSTEP): an `.old` is removed only
  if the archive in its place can be read; an unreadable archive goes to
  `<archive>.damaged`, which Archeion never deletes (the check names it,
  deleting the profile sends it to the Recycle Bin).
- A path of an imported profile is local only if it is on a drive of this
  PC (`X:\…`, `\\?\X:\…`); every other spelling asks for confirmation
  (`archeion_core::profile::remote_part`).
- License `GPL-3.0-only`, copyright and authors `MR`. History rewritten
  (November 2) without name, email or local paths, and again on November
  4 before publication: network examples and the test folder are generic
  (`\\server\…`, `E:\ArcheionTest`), no proper names in the test lists.
  Examples of paths and computers are always invented, never those of the
  development PC. Commit author
  `MR <MR1999develop@users.noreply.github.com>`, also set in the
  repository's git configuration.
- `tools\release.ps1` passes `--remap-path-prefix` and `/PDBALTPATH` through
  `CARGO_ENCODED_RUSTFLAGS` instead of `.cargo/config.toml`: the paths to
  rewrite are local and must not end up in a versioned file. The script
  then looks for the project folder, the user folder and the cargo folder
  in the exe and fails if it finds them.
- The program's first name is removed everywhere, history included: no
  migration from earlier installations; keys derived with contexts
  "Archeion 2026-11-02 … v3" and `KEY_FORMAT` 3, so encrypted destinations
  created before no longer open (message "test version no longer
  supported") and must be recreated.
- Git scripts no longer set a default author: without
  `user.name`/`user.email` they stop.
- Restore tree: a ticked folder ticks everything it contained at its date
  and replaces the choices made inside it; what is unticked inside becomes
  an exclusion (`RestoreItem::exclude`, read by the engine with
  `RestoreItem::wants`). Ticking again an item inside an excluded subfolder
  replaces that exclusion with the siblings along the path
  (`Tree::include`). The state is computed in `Tree::tick`, tested
  without egui.
  The partial state is egui's "indeterminate" checkbox, filled
  (`fill_box`). With several tabs, only the highlighted one shows the
  options.
- Autonomous tests: `app/tests/cli.rs` runs `archeion.exe` on temporary
  folders (`ARCHEION_DATA_DIR`, `ARCHEION_PASSWORD`), the egui_kittest
  tests drive the restore view with the system fonts like eframe
  (`egui_system_font_provider`), a test checks that every character of the
  interface and of the 9 languages can be drawn. Permissions for
  repetitive commands in `.claude/settings.json` (push and publish
  denied).
- Look: all colors, sizes and fonts in `ui/theme.rs` (light/dark tokens in
  Windows 11 style, contrasts tested), the common components in
  `ui/widgets.rs` (section titles, tabs, primary and destructive button,
  windows and modals), the system font in `ui/platform.rs` (Segoe UI read
  from Windows, never shipped). Screenshots of every view are generated by
  the ignored test `screens` (egui_kittest with wgpu) and pass kittest's
  accessibility check.
- Icons from the system font Segoe MDL2 Assets (read from `%WINDIR%\Fonts`,
  never included), constants in `ui/symbol.rs` with the codes verified by
  drawing them; translated texts that name an icon receive it as a
  variable (`{ $deleted-icon }`, `{ $edited-icon }`, `{ $remove-icon }`).
  Ticked boxes filled with the accent, lists with the whole selected item
  and the accent mark, icon buttons without border (`widgets::checkbox`,
  `partial_checkbox`, `list_item`, `icon_button`).
- Window size in `Settings::window_size` (logical pixels, `u16`, plus
  "maximized"): saved after 1 s without resizing or at closing, applied
  at startup never below the 760×480 minimum. The position is not saved:
  Windows chooses it (no windows off a detached screen).
- First size 1200×1000 (all the settings of a profile without scrolling),
  maximized if the screen is lower than 1100 logical px (Full HD
  included);
  controls 24 high and rows 6 px apart (`theme::CONTROL_HEIGHT`, `ROW_GAP`).
  The test screenshots use the same first size.
- Toolchain pinned to 1.99.1 (`rust-toolchain.toml`, CI, MSRV 1.99): the
  "1.99" channel installed locally has an incomplete standard library.
  `fs4` instead of `fs2` (abandoned) for free space; a test with a known
  value (argon2-cffi) guarantees that keys derived from passwords do not
  change when `argon2` is updated.
- Profile settings `theme::SETTINGS_MAX_WIDTH` wide (760; the Guide and the
  other forms stay at `FORM_MAX_WIDTH` 600 for reading), the name as a row
  and not as a section, free space next to the destination: with the
  1200×1000 window they end at about 655 px.
- Profiles as `.archeion-profile` files (`app/src/profile_file.rs`): TOML
  with `format = 1` and the whole profile, never the password; at most
  1 MiB; a later format is rejected. Import keeps the id (on another PC the
  profile finds its backups again) unless it already exists: then a new
  id and the name with "(2)".
- The executable is always a windowed program (`windows_subsystem`,
  except unit tests): no console next to the window; commands write to
  stdout read through a pipe. Commands typed by hand in the terminal have
  no output: in `TIDY.md`.
- Restore tree: an unticked folder whose content is all ticked item by
  item is `Tick::Full` (ticked box); a click removes all the ticks inside
  instead of turning them into the ticked folder (the chosen file versions
  do not change until the user asks).
- Restore: one destination for the whole selection (the 3 options at the
  top, plus "Replace files that already exist"); removed the destination
  menu of each version or folder and the ✎ icon. The versions of a file
  and the date of a folder remain. "Clear selection" is a button next to
  Restore.
- Restore without tabs (user's request, November 3): a file comes back in
  its newest version, a folder as it was in the newest backup that
  contains it. An older version or date can no longer be chosen from the
  window.
- Backups of the deleted profile to the Recycle Bin
  (`archeion_engine::trash_destination`, crate `trash`): only if the
  descriptor belongs to that profile; the whole folder if it contains
  only what Archeion wrote, otherwise every Archeion name (descriptor
  last). Windows silently deletes what it cannot move to the Recycle Bin
  (network shares, removable drives): a test file goes to the Recycle Bin
  first and must be there, and every moved item is looked for in the
  Recycle Bin; at the first one missing it stops. No Undo for the
  profile; the saved password is removed at once. Tests empty from the
  Recycle Bin what they put there.
- Merge: `MergePolicy::keep_deleted` (true by default, also for profiles
  saved before) brings into the merged manifest the entries listed by a
  member of the group but not by the last one, as they were in the last
  one that listed them. It applies to the automatic merge and to the
  manual one (its own checkbox, starting from the profile's value;
  `--drop-deleted` turns it off for that merge). Deleting a single backup
  always discards what only it contained.
- Restore tree: the checkbox has no text (accessible name for screen
  readers) and only ticks; a file's name is a button that shows its
  versions on the right (`RestoreView::shown`), the choice is in
  `RestoreView::chosen` and holds even if made before ticking. In tests,
  a click on a button just enabled must be made one frame later (egui
  checks clicks on the previous frame).
- Version chosen for a file inside a ticked folder: the file enters the
  folder's exclusions and becomes a choice of its own with that version;
  `Tree::tick` does not count as missing an exclusion that has its own
  choice, so the folder stays wholly ticked.
  Up and down arrows: among the drawn files (tree or search), only when
  no text field has the keyboard.
- A file's name in the tree: `widgets::chosen_name` (clickable label with
  the background and accent mark of the selected item, gives up focus so
  the arrows stay with the tree). A file missing from the date of a
  ticked folder: can be ticked on its own; it enters the folder's
  exclusions (no conflict when the original is replaced) and leaves them
  when it is unticked.
- `RestoreItem::with_deleted` (the window always sets it, the command line
  does not): a folder adds what the backups before its date listed and it
  does not, each from the newest that lists it; every file missing from
  the newest backup gets the prefix `restore::DELETED_PREFIX`
  ("_deleted_"). In the tree a file saved before the folder's date shows
  as ticked with it. The total above Restore does not count the deleted
  files yet (in `TIDY.md`).
- Icon drawn in code (`ui/icon.rs`): window, notification area and,
  through `app/build.rs` (crate `winresource`, which uses the Windows
  SDK's `rc.exe`), the executable too, with name, description and
  copyright MR in Properties. Preview: `cargo test -p archeion --bin
  archeion icon_preview -- --ignored` writes `target/ui-screens/archeion.ico`.
- Checkbox "Include deleted files in ticked folders" above the tree
  (`RestoreView::skip_deleted`, on by default, kept like the destination):
  it applies to all ticked folders, immediately; it reaches the engine as
  the folders' `with_deleted` (the files stay marked).
- Removed `tools\probe.bat` (early experiments), `tools\git_init.bat` and
  `tools\first-commit.txt` (the repository exists), the translation keys
  of removed features. `tools\github_publish.bat` remains: the repository
  has no remote yet.
- Review with all the skills (November 3): the destination is untrusted.
  With keys only manifests and encrypted files are read; an encrypted
  profile or archives with encrypted manifests never proceed in plain
  (`EncryptionMissing`); manifest keys are checked on reading
  (`keys::check`).
- `unsafe` code lives only in `crates/win` (its own lints:
  `undocumented_unsafe_blocks`); the rest of the workspace forbids it.
- Durability: flush of every file before the commit and write-through
  renames at commit points; merge recovery never deletes an `.old` if the
  merged archive is not in its place.
- Cancel closes the worker's stdin (`--stop-on-eof`): the engine stops at
  the next file (`EventSink::stop_requested`); kill after 15 s. The
  window's workers are in a job object.
- An imported profile has its schedule paused (`schedule_paused`) until
  the user turns it on.
- `destinations.json` in the data folder remembers the last backup seen
  per profile: if the destination goes back in time a notification
  arrives (not a block). An incremental with many more changed files than
  usual is also notified.
- `ROW_GAP` stays 6 (not a multiple of 4) so that the settings do not
  scroll.
- Deep tests (`crates/engine/tests/deep.rs`, November 3): random histories
  with a printed seed, interruptions at every file, every crash state of
  a merge, manifest fuzzing, damaged data; key properties in `keys.rs`.
  Fuzz escapes land in the temporary folder: a first run with the key
  check removed on purpose had created empty folders in `C:\` (removed at
  once).
