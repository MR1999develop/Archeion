# Archeion

Incremental, scheduled, optionally encrypted backups for Windows with the
convenience of GNOME's Déjà Dup and the archive model of Areca: the first
backup is complete, the following ones hold only new or changed files, and
old backups are merged instead of deleted (the scheduled merge never touches
the first complete backup; a manual merge of chosen backups can, after a
warning). Each file is stored as its own zip (`report.docx.zip`), as a plain
copy, or as an AES-256 zip with a hidden name, readable without the
application (7-Zip and the recovery key for encrypted backups).

The name is the ancient Greek ἀρχεῖον, "archive". The program was first
called Archeion; destinations, settings, saved passwords and scheduled tasks
of that name are renamed automatically.

## Layout

| Path | Role |
| --- | --- |
| `crates/core` | Domain model: profiles, archive names, manifests, path keys, merge planning, run events. No I/O. |
| `crates/engine` | Archive engine: backup (with free-space check and resume), automatic and manual merge, deletion, restore, versions, search, verify, encryption, destination lock and crash recovery. |
| `crates/scheduler` | Windows Task Scheduler tasks (XML generation + `schtasks.exe`), one for backups and one for merges per profile. |
| `app` | `archeion.exe`: egui window and headless worker (`archeion run|merge|delete|verify|restore <profile id>`). |
| `tools` | `check.bat` (fmt, clippy, tests) and `release.bat`, each writing a `.log`. |

## On disk

```
E:\Backup\Documents\
  archeion.json               owning profile (and whether it is encrypted)
  archeion.lock               held while a run uses the folder
  archeion.key.json           wrapped master key (encrypted destinations)
  archeion.archives.mac       signed list of archives (encrypted destinations)
  2026-10-29_213000_C\        complete archive
    manifest.jsonl            every file and folder at that time
    zipped\C\Users\...\a.txt.zip  one zip per file (or data\... plain copies,
                                  or vault\ab\<hex>.zip when encrypted)
  2026-10-30_213000_I\        incremental: only new or changed files
  2026-10-31_213000_I.partial\  unfinished backup, resumed by the next run
    resume.jsonl              files already copied
```

Each manifest entry says whether the archive stores its content; if not,
the content is in the newest earlier archive that stores it. A merge
replaces a group of archives by one that carries the date of the last kept
archive and the newest content of the group, so later manifests never need
rewriting. Every change happens in a `.partial` / `.merging` folder and is
committed by a rename; merges write a journal first and are completed by
the next run; an unfinished backup is resumed by the next backup.

## How a run works

"Back up now", "Merge now" and the scheduled tasks all start
`archeion run|merge <id>`, which writes one JSON `RunEvent` per line on
stdout and a log in `logs\`. The window reads those lines to show progress;
Cancel kills the process. Exit codes: 0 success, 1 failure, 2 destination
missing, 3 busy, 4 password missing or wrong, 5 not enough free space.

Settings: `%APPDATA%\Archeion\config\settings.toml` (or `data\` next to the
executable when a `portable.txt` file is there).

## Build

Rust 1.95 or newer, MSVC toolchain.

```
tools\check.bat      formats, lints and tests, step by step on screen and in
                     tools\check.log (-Verbose: cargo --verbose; -NoPause)
tools\release.bat    optimised build; log in tools\release.log
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

### First publication

1. `tools\git_init.bat`: creates the repository (`main`) and the first
   commit; log in `tools\git_init.log`.
2. `tools\github_publish.bat`: creates the private GitHub repository with
   the GitHub CLI and pushes; log in `tools\github_publish.log`.

### Releases

1. Set the new `version` in the root `Cargo.toml` and move the
   "Unreleased" notes of `CHANGELOG.md` under that version.
2. Commit, then tag and push: `git tag v0.2.0` and `git push origin v0.2.0`.
3. `.github/workflows/release.yml` checks that the tag matches the
   version, runs the tests, builds `archeion.exe` and publishes a GitHub
   release with `Archeion-v0.2.0-windows-x64-portable.zip`.

## License

Dual-licensed under the MIT license ([LICENSE-MIT](LICENSE-MIT)) or the
Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), at your option.
