# Archeion

Incremental, scheduled, optionally encrypted backups for Windows with the
convenience of GNOME's Déjà Dup and the archive model of Areca: the first
backup is complete, the following ones hold only new or changed files, and
old backups are merged instead of deleted (the scheduled merge never touches
the first complete backup; a manual merge of chosen backups can, after a
warning). Each file is stored as its own zip (`report.docx.zip`), as a plain
copy, or as an AES-256 zip with a hidden name, readable without the
application (7-Zip and the recovery key for encrypted backups). Large
files (every file, on an encrypted destination) are also cut into
content-defined chunks, so an incremental stores only the parts that
changed; complete backups keep every file whole
([docs/large-files.md](docs/large-files.md)).

The name is the ancient Greek ἀρχεῖον, "archive". The program was first
called Archeion; destinations, settings, saved passwords and scheduled tasks
of that name are renamed automatically.

## Layout

| Path | Role |
| --- | --- |
| `crates/core` | Domain model: profiles, archive names, manifests, path keys, merge planning, restore requests and restored file names, run events. No I/O. |
| `crates/engine` | Archive engine: backup (with free-space check and resume), automatic and manual merge, deletion, restore of several items in one run, catalog of every file ever saved, versions, search, verify, encryption, destination lock and crash recovery. |
| `crates/scheduler` | Windows Task Scheduler tasks (XML generation + `schtasks.exe`), one for backups and one for merges per profile. |
| `app` | `archeion.exe`: egui window and headless worker (`archeion run|merge|delete|verify|restore <profile id>`, `archeion extract`). |
| `tools` | `check.bat` (fmt, clippy, tests), `release.bat` and the git scripts (`git_status`, `git_commit`, `git_push`, `github_publish`), each writing a `.log`. |

## On disk

```
E:\Backup\Documents\
  archeion.json               owning profile (and whether it is encrypted)
  archeion.lock               held while a run uses the folder
  archeion.key.json           wrapped master key (encrypted destinations)
  archeion.archives.mac       signed list of archives (encrypted destinations)
  2026-10-29_213000_C\        complete archive: every file whole
    manifest.jsonl            every file and folder at that time, with the
                              chunk list of large files (format 2)
    zipped\C\Users\...\a.txt.zip  one zip per file (or data\... plain copies,
                                  or vault\ab\<hex>.zip when encrypted)
  2026-10-30_213000_I\        incremental: only new or changed files
    chunks\3f\3f9a...c2.zip   new chunks of large files (plain file, zip,
                              or AES zip when encrypted)
  2026-10-31_213000_I.partial\  unfinished backup, resumed by the next run
    resume.jsonl              files already copied
```

Each manifest entry says whether the archive stores its content; if not,
the content is in the newest earlier archive that stores it. A large file
(at least the profile's threshold, 500 MB unless changed; every file when
encrypted) is stored whole with its chunk list in a complete archive and
as the chunks not yet in the destination in an incremental; its chunks are
read from chunk files or as byte ranges of the complete archive's whole
copy, each checked against its BLAKE3 name. Backups of earlier versions
keep their format; chunks start with the next (automatic, complete)
backup. Details in [docs/large-files.md](docs/large-files.md). A merge
replaces a group of archives by one that carries the date of the last kept
archive and the newest content of the group, so later manifests never need
rewriting. Every change happens in a `.partial` / `.merging` folder and is
committed by a rename; merges write a journal first and are completed by
the next run; an unfinished backup is resumed by the next backup.

## How a run works

"Back up now", "Merge now" and the scheduled tasks all start
`archeion run|merge <id>` (`run <id> --scheduled --event` for backups
started at logon, when idle or when a drive is connected; `archeion watch`
daily for the "no backup for N days" notification), which writes one JSON `RunEvent` per line on
stdout and a log in `logs\`: `<start>-<op>-<profile>.jsonl` (header,
steps, skipped files, outcome; read by the Activity list) and beside it
`<same name>.files.jsonl`, every file the run handled with what it did
(read only for the full log). The window reads the stdout lines to show
progress; Cancel kills the process. Exit codes: 0 success, 1 failure, 2 destination
missing, 3 busy, 4 password missing or wrong, 5 not enough free space.

Settings: `%APPDATA%\Archeion\config\settings.toml` (or `data\` next to the
executable when a `portable.txt` file is there).

### Restoring from the window

Backups > Restore files shows one tree of every file the profile's backups
ever saved (files deleted at the source are marked), with search. Ticked
files and folders become cards on the right: a file with its versions, a
folder with the date it comes back as, each with a destination (a default
for everything, changeable per version or folder). The window writes the
selection as a JSON request (`restore-<n>.json` in the data folder: a list
of items, each a backup, a file or folder, a target and whether to
replace) and starts `archeion restore <id> --request <file>`, which
restores every item in one run. Several versions of a file sent to the
same folder get their dates in the names. Backups > Manage backups lists
the dates, with the manual merge and the deletion of one backup.

### Restoring without the window

`archeion extract --destination <folder> --to <folder>` restores from a
destination folder alone, without settings, profile or saved password: the
portable executable recovers backups on any computer. Options:
`--archive <name>` (the newest by default), `--path <file or folder>`
(its original path; everything by default), `--overwrite`, and for an
encrypted destination `--stdin` (password or recovery key on the first
line of standard input), `--password <text>` or `--recovery-key <key>`, or
the `ARCHEION_PASSWORD` / `ARCHEION_RECOVERY_KEY` variables. The
destination is only read. Progress goes to stdout and to
`archeion-extract.log` in the target folder; exit codes as above.
`archeion.exe` is a window program: in a Command Prompt start it with
`start /wait archeion extract ...` so the prompt waits for it.

The window keeps running in the notification area when closed (File >
Preferences), and starts with Windows as `archeion --tray` from the per-user
`Run` registry key. Only one window runs: `window.lock` and `window.port`
in the data folder let a second launch bring the first one forward.

## Build

Rust 1.95 or newer, MSVC toolchain.

```
tools\check.bat      formats, lints and tests, step by step on screen and in
                     tools\check.log (-Verbose: cargo --verbose; -NoPause)
tools\release.bat    optimised build, step by step on screen and in
                     tools\release.log (same options)
```

The executable is `target\release\archeion.exe`.

## Development

Repository: <https://github.com/MR1999develop/archeion>.

- **Branches.** `main` always builds and passes `tools\check.bat`. Work on
  a short-lived branch (`feature/resume-copy`, `fix/merge-journal`) and
  merge it with a pull request once CI is green.
- **Commits.** English, imperative subject of at most 72 characters
  ("Resume interrupted backups"), a blank line, then why the change was
  made. One logical change per commit.
- **CI.** `.github/workflows/ci.yml` runs `cargo fmt --check`, clippy with
  warnings as errors and all tests on Windows for every push to `main`
  and every pull request.
- **Postponed work** goes in `TIDY.md`; user-visible changes go in
  `CHANGELOG.md` under "Unreleased".

### Daily work with git

Each script runs by double-click or from a terminal, shows every step and
writes `tools\<name>.log`, whose last line is `<NAME> DONE <code>` (0 = OK).
`-NoPause` never waits for Enter.

1. `tools\git_status.bat`: branch, commits to push or pull, changed files,
   last commits, stashes, GitHub CLI sign-in and the next step. Read-only.
2. Write the message in `tools\commit-message.txt` (subject of at most 72
   characters, blank line, why; `#` lines are ignored).
3. `tools\git_commit.bat`: refuses to commit on `main`
   (`-Branch feature/xyz` creates the branch and keeps the changes), runs
   `tools\check.ps1` and stops if it fails (`-SkipCheck` to skip it),
   stages everything with `git add -A`, commits with the `Co-Authored-By`
   and `Claude-Session` trailers, then moves the message to
   `commit-message.last.txt`. `-Message "Subject"` replaces the file.
4. `tools\git_push.bat`: pushes the branch (never forced; it stops if
   GitHub has commits to pull first) and says how to open the pull
   request; `-PullRequest` opens it towards `main` with the GitHub CLI.
5. Merge the pull request on GitHub once CI is green, then
   `git switch main` and `git pull`.

`tools\commit-message.txt` is ignored by git: it is a personal draft that
changes at every commit, and `git add -A` would otherwise commit it.

### First publication

1. `tools\git_init.bat`: creates the repository (`main`) and the first
   commit; log in `tools\git_init.log`.
2. `tools\github_publish.bat`: creates the private GitHub repository with
   the GitHub CLI (or links it if it exists) and pushes `main`, the current
   branch and the tags; log in `tools\github_publish.log`. Safe to run
   again.

### Releases

1. Set the new `version` in the root `Cargo.toml` and move the
   "Unreleased" notes of `CHANGELOG.md` under that version.
2. Commit, then tag and push: `git tag v0.2.0` and `git push origin v0.2.0`.
3. `.github/workflows/release.yml` checks that the tag matches the
   version, runs the tests, builds `archeion.exe` and publishes a GitHub
   release with `Archeion-v0.2.0-windows-x64-portable.zip`.

## License

Copyright (C) 2026 MR

This program is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, version 3 only ([LICENSE](LICENSE)).

This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
more details.
