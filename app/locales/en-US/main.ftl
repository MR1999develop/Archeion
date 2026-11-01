# Menus
menu-file = File
menu-backup = Backup
menu-help = Help
new-profile = New profile
quit = Quit
restore = Restore files…
data-folder = Data folder

# Profile list and empty state
profiles = Profiles
unnamed = (no name)
new-profile-name = New backup
empty-title = No backups yet
empty-body = Create a profile: choose the folders to save, where to save them and when.
tab-settings = Settings
tab-backups = Backups

# Profile form
profile-name = Name
folders = Folders to back up
no-folders = No folders yet: add at least one.
add-folder = Add folder…
remove-folder = Remove this folder from the backup
exclusions = Exclusions
exclusions-hint = Patterns match the full path, for example **/node_modules or **/*.tmp.
destination = Destination
dest-local = Folder or external drive
dest-network = Network share
dest-hint = Use a folder only for this profile. The backups are not encrypted: anyone who can open it can read them.
browse = Browse…
container = Storage
container-zip-per-file = One zip per file
container-folder = Plain copies
container-zip-per-file-hint = Each file is compressed on its own (report.docx becomes report.docx.zip) in the same folders; it opens with File Explorer or 7-Zip.
container-folder-hint = Files are copied as they are, with the same names, without compression.
schedule = Automatic backup
schedule-manual = Only when I start it
schedule-daily = Every day
schedule-weekly = Every week
at = at
schedule-hint = If the computer is off at that time, it runs as soon as it is on again.
merge = Merge old backups
merge-hint = Old backups are not deleted but merged into the next one kept, so every kept date stays complete and space is freed. The first complete backup is never merged automatically.
retention-hint = A backup is kept if any rule selects it.
keep-last = Most recent
keep-daily = One per day, for days
keep-weekly = One per week, for weeks
keep-monthly = One per month, for months
keep-yearly = One per year, for years
keep-everything = All rules at 0: every backup is kept and nothing is merged.
back-up-now = Back up now
merge-now = Merge now
merge-now-hint = Apply the merge rules now; the first complete backup stays as it is.
busy-hint = A backup or merge is already running.
fix-profile-hint = Complete the profile first.
delete-profile = Delete profile
delete-profile-hint = Removes the profile and its scheduled tasks; the backups stay at the destination.

day-monday = Monday
day-tuesday = Tuesday
day-wednesday = Wednesday
day-thursday = Thursday
day-friday = Friday
day-saturday = Saturday
day-sunday = Sunday

error-empty-name = Give the profile a name.
error-no-sources = Add at least one folder to back up.
error-no-destination = Choose where to save the backups.
error-destination-in-source = The destination is inside a folder being backed up; choose another one.

# Backups tab
refresh = Refresh
reading-backups = Reading the backups…
no-backups-yet = No backups yet for this profile.
kind-complete = Complete
kind-incremental = Incremental

# Status bar
ready = Ready
cancel = Cancel
undo = Undo
phase-starting = Starting…
phase-opening = Opening the destination…
phase-scanning = Looking for changes…
phase-copying = Copying…
phase-merging = Merging old backups…
phase-restoring = Restoring…
phase-verifying = Checking…
progress-of = { $done } of { $total }
skipped-count = { $count ->
    [one] 1 file skipped (in use)
   *[other] { $count } files skipped (in use)
}
profile-removed = Profile "{ $name }" deleted.
run-finished = "{ $name }" backed up: { $new } new, { $changed } changed, { $deleted } deleted, { $added } copied.
merge-finished = "{ $name }" merged: from { $before } to { $after } backups, { $freed } freed.
restore-finished = { $count } files restored.
verify-finished = { $count } files checked, { $problems } problems.
run-destination-missing = Destination not found: { $location }. Connect the drive and try again.
run-busy = Another backup or merge of this profile is running; try again when it ends.
run-failed = The operation failed: { $detail }
run-no-space = Not enough free space on the destination: { $needed } needed, { $available } free. Free some space, or merge or delete old backups, then run it again: the files already copied are kept and not copied again.
backup-resumed = { $count ->
    [one] 1 file kept from the interrupted backup
   *[other] { $count } files kept from the interrupted backup
}
run-cancelled = Cancelled. Nothing was changed in the backups; a stopped backup goes on from where it stopped next time.

# Errors
settings-unreadable = The settings file cannot be read, so changes will not be saved: { $detail }
save-failed = The settings could not be saved: { $detail }
schedule-failed = The automatic run could not be scheduled: { $detail }
start-failed = The operation could not start: { $detail }
unexpected-stop = The operation stopped unexpectedly.

# Restore browser
dates = Backup dates
archive-tooltip = { $files } files, { $stored } saved in this backup ({ $size })
search-hint = Search a file by name in every backup…
pick-date = Choose a date on the left.
tree-hint = Tick files or folders to restore them; click a file to see its versions.
searching = Searching…
no-results = No file with that name.
results = { $count ->
    [one] 1 file found
   *[other] { $count } files found
}
version-count = { $count ->
    [one] 1 version
   *[other] { $count } versions
}
deleted-now = deleted since
close = Close
versions-hint = Each version is a different content of the file, newest first.
version-modified = Modified on { $date }
version-detail = Saved by the backup of { $backup } · { $size }
restore-version = Restore this version
restore-to = Restore to:
target-beside = Next to the original, with the date
target-beside-hint = report.docx is restored as "report (2026-11-02 21.30).docx": the current file is not touched.
target-original = Original location
target-folder = Another folder
replace-existing = Replace files that already exist
replace-warning = The current files will be replaced by the backup copy.
tick-files-hint = Tick at least one file or folder.
choose-folder-hint = Choose the folder to restore into.
restore-selected = Restore selected
selected-count = { $count ->
    [one] 1 item from the backup of { $date }
   *[other] { $count } items from the backup of { $date }
}
clear-selection = Clear selection
tick-folder = Restore this folder and everything in it
tick-file = Restore this file
show-versions = Show the versions of this file

# Manual merge
merge-choose = Choose backups to merge…
merge-choose-hint = Tick the first and the last backup: everything between them becomes one backup.
merge-tick = Tick as an end of the range to merge
merge-tick-hint = Tick the oldest and the newest backup to merge; the ones between are included.
merge-summary = { $count } backups become one, dated { $date }.
merge-complete-warning = The range includes the first complete backup: it is replaced by a new complete backup dated as the newest one chosen, and the earlier dates can no longer be restored.
merge-complete-ack = I understand
merge-two-hint = Tick at least two backups.
merge-ack-hint = Confirm that you understand the warning.
merge-selected = Merge

# Activity tab
tab-activity = Activity
show-details = Details
cancel-hint = Stop now: the backups stay as they were before this run. The next backup reuses the files already copied.
activity-backup-running = Backing up "{ $name }"
activity-merge-running = Merging the backups of "{ $name }"
activity-restore-running = Restoring from "{ $name }"
activity-backup-ended = Backup of "{ $name }"
activity-merge-ended = Merge of "{ $name }"
activity-restore-ended = Restore from "{ $name }"
activity-preparing = Preparing…
activity-file = File
activity-folder = Folder
activity-files = Files
activity-speed = Speed
activity-elapsed = Elapsed
activity-remaining = Remaining
count-of = { $done } of { $total }
files-found = { $count } found so far
about = about { $time }
activity-took = Took { $time }.
activity-backup-summary = Done: { $new } new, { $changed } changed, { $deleted } deleted, { $unchanged } unchanged; { $copied } copied.
activity-merge-summary = Done: from { $before } to { $after } backups, { $freed } freed.
activity-restore-summary = Done: { $restored } files restored, { $kept } already present and kept.
show-backups = Show backups
skipped-hint = These files could not be read, usually because another program has them open. Their previous version, if any, is still in the backups.
skipped-more = …and { $count } more (see the log).

# Exclusions and format lock
no-exclusions = Nothing excluded: every file in the folders is backed up.
remove-exclusion = Back this up again
exclusion-folder = Folder and everything in it
exclusion-file = File
exclusion-type = Every file of this type
exclusion-pattern = Pattern
exclude-folder = Exclude folder…
exclude-file = Exclude file…
exclude-type-hint = .tmp
exclude-type = Exclude type
common-types = Common:
advanced = Advanced
exclude-pattern = Exclude pattern
container-locked = The destination already holds backups in this format: it cannot change. To use the other one, choose an empty destination.
container-unreachable = The destination cannot be reached, so the format cannot be changed now.
container-checking = Checking the destination…
activity-idle-title = Nothing running
activity-idle-body = Backups, merges and restores show their progress here.

# Backup kinds and deletion
menu-run-incremental = Incremental backup now
menu-run-complete = Complete backup now
run-now = Run now:
run-incremental = Incremental
run-incremental-hint = Copies only new and changed files; quick, and the usual choice.
run-complete = Complete
run-complete-hint = Copies every file and starts a new chain: later backups no longer depend on the earlier ones. Takes longer and more space.
run-merge = Merge
run-first = Run the first backup
run-incremental-button = Run incremental backup
run-complete-button = Run complete backup
full-backup = Complete backup:
full-never = Only the first
full-every-backups = Every
full-every-days = When the last is older than
full-backups-unit = backups
full-days-unit = days
full-backup-hint = Applies to automatic backups; a complete backup starts a new chain, which makes restoring independent of older backups.
run-finished-complete = "{ $name }" backed up completely: { $new } new, { $changed } changed, { $deleted } deleted, { $added } copied.
activity-complete-summary = Complete backup done: { $new } new, { $changed } changed, { $deleted } deleted, { $unchanged } unchanged; { $copied } copied.
activity-delete-running = Deleting a backup of "{ $name }"
activity-delete-ended = Backup deleted from "{ $name }"
activity-delete-summary = Done: from { $before } to { $after } backups, { $freed } freed.
delete-finished = A backup of "{ $name }" was deleted.
delete-backup = Delete this backup…
delete-backup-hint = Removes the selected date; the backups after it stay complete.
delete-backup-confirm = Delete backup
delete-initial-hint = The first complete backup can be deleted only when it is the only one: the others depend on it.
delete-complete-hint = A complete backup starts a chain the later backups depend on: it cannot be deleted. Merge it with the next ones instead.
delete-only-warning = The backup of { $date } is the only one: the destination will be empty and the next backup will be complete. This cannot be undone.
delete-newest-warning = The backup of { $date } will be deleted, with the file versions saved only in it. This cannot be undone.
delete-middle-warning = The backup of { $date } will be deleted. Files the later backups need move into the next backup; versions saved only in this one are lost. This cannot be undone.

# Encryption
encrypt = Encrypt backups (AES-256, file names included)
encrypt-hint = Every file is an AES-256 zip with a hidden name; without the password nobody can read the content or see which files are saved. The password is kept in Windows Credential Manager.
encrypt-needs-zip = Encryption needs the "One zip per file" format.
password = Password
password-again = Repeat password
password-set = Choose the password
password-reenter = Type the password of these backups
password-saved = Password saved in Windows Credential Manager.
password-missing = No password saved on this computer: type it to run backups and browse them.
password-loss-warning = Without this password, or the recovery key shown after the first backup, the backups cannot be opened and nobody can recover them. Keep the password somewhere safe.
password-save = Save password
password-checking = Checking the password…
password-empty-hint = Type the password.
password-short-hint = Use at least { $count } characters.
password-mismatch-hint = The two passwords differ.
password-first-hint = Save the password of the encrypted backups first.
password-wrong = This password does not open the backups at the destination.
password-unreachable = The password could not be checked
run-password-needed = The backups are encrypted and no password is saved on this computer: type it in the profile settings.
run-wrong-password = The saved password does not open the backups: type it again in the profile settings.

# Backup contents
tree-count = { $stored } of { $total } files saved by this backup
tree-stored-hint = Files saved by this backup (new or changed) stand out; greyed ones were saved by earlier backups and are unchanged, and can be restored too. Tick to restore; a ticked folder comes back whole, as it was at that date. Click a file to see its versions.
tree-nothing-stored = Nothing was saved by this backup: no file had changed.
tree-not-saved-folder = Nothing in this folder changed in this backup.
tree-not-saved-file = Unchanged in this backup: its content is in an earlier one. Click to see its versions.

# Operations log
log-runs = Operations
log-only-profile = Only this profile
log-running = In progress
log-last = Last operation
log-empty = No operations yet.
log-pick = Choose an operation on the left.
log-title = Log
log-no-lines = Nothing logged yet.
log-scheduled = automatic
log-manual = started by hand
log-started = Started { $time }
log-backup = backup { $name }
log-unfinished = Did not finish: interrupted, or still running.
log-skipped = Skipped { $path }: { $reason }
log-op-backup = Backup
log-op-incremental = Incremental backup
log-op-complete = Complete backup
log-op-merge = Merge
log-op-delete = Backup deletion
log-op-restore = Restore

# Password change and recovery key
password-change = Change the password
password-new = New password
password-change-button = Change password
recovery-later = The recovery key will be available after the first backup.
recovery-show = Show the recovery key…
recovery-title = Recovery key
recovery-hint = With this key 7-Zip opens every encrypted file and manifest, even without Archeion and without the password. The manifest of each backup lists which file is stored where.
recovery-warning = Whoever has this key can read all the backups. Print it or write it down and keep it offline; it does not change when you change the password.
recovery-copy = Copy
recovery-copy-hint = Copies the key without spaces, as 7-Zip wants it.
recovery-hide = Hide

# Integrity check
verify-section = Checks
verify-hint = The full check reads back every stored file of every backup and compares it with its fingerprint; the restore test reads back some files of the newest backup at random. Due checks run after a scheduled backup.
verify-full-every = Full check every
verify-sample-every = Restore test every
verify-sample-files = on files:
verify-zero-hint = 0 days = never automatically.
verify-full-now = Check backups now
verify-full-hint = Reads back every stored file and checks it; on a large destination it takes as long as reading all the backups.
verify-sample-now = Restore test now
verify-sample-hint = Reads back some files of the newest backup at random, as a restore would, without writing them anywhere.
verify-no-backups = There are no backups to check yet.
verify-problems = Problems found
verify-problems-hint = The backups listed may not restore correctly. Make a complete backup now, and check the destination drive.
activity-verify-running = Checking the backups of "{ $name }"
activity-verify-ended = Check of the backups of "{ $name }"
log-op-verify = Integrity check
log-op-restore-test = Restore test
