# Changelog

All notable changes to Archeion are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

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
