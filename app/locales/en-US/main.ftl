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
exclusions-hint = A pattern matches the full path, ignoring case: * any text in a name, ** any folders, ? one character. For example **/node_modules, **/cache/**, **/~$*.
destination = Destination
dest-local = Folder or external drive
dest-network = Network share
dest-hint = Use a folder only for this profile. Unless encryption is on (below), anyone who can open it can read the backups.
dest-free = Free space on the destination: { $free }
dest-foreign-profile = This folder holds the backups of the profile "{ $name }": choose another folder for this profile.
dest-foreign-deleted = This folder holds the backups of a profile that no longer exists.
dest-adopt = Use these backups for this profile
dest-adopt-hint = The existing backups become this profile's: the next backup continues them, and they can be restored and merged from here.
dest-adopt-failed = The backups could not be taken over: { $detail }
dest-foreign-hint = The destination holds the backups of another profile: see the Destination section.
run-foreign-destination = The destination holds the backups of another profile. Open the profile settings: if that profile was deleted, "Use these backups for this profile" takes them over.
browse = Browse…
container = Storage
container-zip-per-file = One zip per file
container-folder = Plain copies
container-zip-per-file-hint = Each file is compressed on its own (report.docx becomes report.docx.zip) in the same folders; it opens with File Explorer or 7-Zip.
container-folder-hint = Files are copied as they are, with the same names, without compression.
large-files = Save large files as changes only, from
large-files-unit = MB
large-files-hint = Files of at least this size are saved in pieces: after a change, a backup stores only the pieces that changed. Complete backups also keep them whole, so they still open with 7-Zip. Smaller files are saved as above.
large-files-change-hint = You can change it at any time: the next backups follow the new limit, the ones already made stay as they are.
large-files-encrypted = Every file is saved in pieces, so a changed file stores only what changed; complete backups also keep whole copies.
schedule = Automatic backup
schedule-manual = Only when I start it
schedule-daily = Every day
schedule-weekly = Every week
at = at
schedule-hint = If the computer is off at that time, it runs as soon as it is on again.
merge = Merge old backups
merge-hint = Old backups are not deleted but merged into the next one kept, so every kept date stays complete and space is freed. The first complete backup is never merged automatically.
merge-automatic-hint = The merge is automatic: while Archeion is running it merges old backups by the rules below, never while a backup or another operation uses them.
retention-hint = A backup is kept if any rule selects it. The backups of the days set in "All, for days" are never merged.
keep-within = All, for days
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
delete-profile-hint = Asks first, then removes the profile and its scheduled tasks; the backups stay at the destination.
delete-profile-title = Delete profile "{ $name }"?
delete-profile-body = The profile and its scheduled tasks are removed. Until you close the message in the status bar, Undo brings them back.
delete-profile-password = The saved password is removed from Windows Credential Manager when Undo is no longer possible: when you close that message, delete another profile or quit Archeion.
delete-profile-backups = The backups in { $path } are not deleted: a new profile that uses that folder can take them over with "Use these backups for this profile".

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
restore-no-space = Not enough free space where the files are restored: { $needed } needed, { $available } free. Nothing was written: free some space, or restore fewer files or to another drive.
backup-resumed = { $count ->
    [one] 1 file kept from the interrupted backup
   *[other] { $count } files kept from the interrupted backup
}
backup-as-changes = { $count ->
    [one] 1 file saved as changes only
   *[other] { $count } files saved as changes only
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
no-results = No file with that name.
results = { $count ->
    [one] 1 file found
   *[other] { $count } files found
}
close = Close
versions-hint = Each version is a different content of the file, newest first.
version-modified = Modified on { $date }
version-detail = Saved by the backup of { $backup } · { $size }
restore-to = Restore to:
target-beside = Next to the original, with the date
target-beside-hint = report.docx is restored as "report (2026-11-02 21.30).docx": the current file is not touched.
target-original = Original location
target-folder = Another folder
replace-existing = Replace files that already exist
replace-warning = The current files will be replaced by the backup copy.
tick-files-hint = Tick at least one file or folder.
choose-folder-hint = Choose the folder to restore into.
restore-selected = Restore
clear-selection = Clear selection
tick-folder = Restore this folder and everything in it
tick-file = Restore this file
backups-restore-files = Restore files
backups-restore-files-hint = Every file ever saved: tick what to bring back, choose the versions or the date, and where it goes.
backups-manage = Manage backups
backups-manage-hint = The list of backup dates: merge several into one, or delete one.
restore-tree-count = { $files ->
        [one] 1 file
       *[other] { $files } files
    } in the backups · deleted at the source: { $deleted }
restore-tree-hint = Tick files or folders to restore them; the search finds a file by name. Files marked { $deleted-icon } deleted are no longer at the source and can still be restored.
deleted-label = deleted
deleted-since = Not in the newest backup: deleted at the source.
tree-deleted-file = Not in the newest backup: deleted at the source. Tick it to restore one of its versions.
tree-deleted-folder = Not in the newest backup: deleted at the source. Tick it to restore it as it was on a date.
remove-from-selection = Remove from the selection
tick-in-folder = Restored with the ticked folder { $folder }. Untick it to leave it out.
tick-not-in-folder-date = Not in the backup that the ticked folder { $folder } comes back as: it is not restored. Untick that folder to choose this item on its own.
tick-partial = Something inside is ticked, or left out of the ticked folder. Click to tick the whole folder.
tick-left-out = Left out of the ticked folder { $folder }: not restored. Tick it to restore it with the folder.
folder-left-out = { $count ->
    [one] { $count } item unticked inside: not restored.
   *[other] { $count } items unticked inside: not restored.
}
card-show-options = Click to see and change its versions, date and destination.
card-summary-file = { $versions ->
    [one] 1 version → { $dest }
   *[other] { $versions } versions → { $dest }
}
card-summary-folder = As it was on { $date } → { $dest }
dest-several = several destinations
selection-empty = Nothing to restore yet. Tick files or folders in the tree: each one appears here with its versions or its date, and where it will go.
folder-as-of = As it was on:
folder-as-of-hint = The folder comes back whole, with everything it held on that date.
folder-size = { $size } at that date
dest-default = Default: { $dest }
dest-follow-default = Use the default destination
dest-overwrite = Overwrite the original
dest-folder = Another folder…
dest-folder-missing = another folder, not chosen yet
dest-change-hint = Where this goes. { $edited-icon } marks a destination changed from the default.
restore-to-hint = For everything ticked, unless changed on a version or folder below.
restore-conflict = Another version overwrites the same original: keep "Overwrite the original" for one version only.
restore-conflict-hint = Two versions would overwrite the same original file: change the destination of one of them.
restore-wait-versions = Reading the versions of the ticked files…
restore-summary = { $files ->
        [one] 1 file
       *[other] { $files } files
    }, { $versions ->
        [one] 1 version
       *[other] { $versions } versions
    }, { $folders ->
        [one] 1 folder
       *[other] { $folders } folders
    } — { $size }

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
exclude-pattern = Exclude patterns
exclude-pattern-add = Add
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
password-rule-length = At least { $count } characters (now { $typed })
password-rule-match = Both passwords match
password-rule-variety = At least { $kinds } kinds of characters among lowercase, uppercase, digits and symbols (not needed from { $passphrase } characters)
password-rule-common = Not a common password, like "Password2024" or "qwerty"
password-rule-runs = No { $count } repeated or consecutive characters, like "aaaa", "1234" or "abcd"
password-rule-name = Does not contain the profile name
password-rule-strength = Hard enough to guess: strength at least { $level }
password-rule-tip = Any character works, spaces included: a sentence of four or five words is easy to remember and hard to guess, and from { $passphrase } characters it needs no digits, capitals or symbols.
password-strength = Strength: { $level }
password-strength-weak = Weak
password-strength-fair = Fair
password-strength-strong = Strong
password-strength-very-strong = Very strong
password-variety-hint = Mix lowercase, uppercase, digits and symbols, or write a sentence of at least { $passphrase } characters.
password-common-hint = This is one of the most used passwords: choose another.
password-runs-hint = Avoid runs like "aaaa" or "1234".
password-name-hint = Do not put the profile name in the password.
password-weak-hint = Too easy to guess: make it longer, for instance with a few more words.
password-first-hint = Type the password of the encrypted backups above (twice for new backups, and tick "I understand"): the button saves it and starts the backup.
password-wrong = This password does not open the backups at the destination.
password-unreachable = The password could not be checked
run-password-needed = The backups are encrypted and no password is saved on this computer: type it in the profile settings.
run-wrong-password = The saved password does not open the backups: type it again in the profile settings.

# Operations log
log-runs = Operations
log-only-profile = Only this profile
log-running = In progress
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
log-open-full = Open full log
log-open-full-hint = Writes the whole log of this operation to a text file and opens it in your text editor: every step, then every file it worked on and what it did with it, skipped files included.
log-copy = Copy log
log-copy-hint = Copies the whole log of this operation as text, to paste it in an email or a problem report.
log-copied = Log copied.
log-open-program = Program log
log-open-program-hint = Opens archeion.log, the program's technical log: useful to look into a problem or to report it.
log-open-folder = Logs folder
log-open-folder-hint = Opens the folder of the operation logs in File Explorer.
log-program-missing = The program log does not exist yet.
log-unreadable = The log file cannot be read.
log-report-failed = Cannot show the full log: { $detail }
log-level-info = Info
log-level-done = OK
log-level-warning = Warning
log-level-error = Error
log-preparing = Preparing the full log…
log-files-title = Files
log-files-missing = The list of files was not recorded for this operation: it was made by an older version of Archeion.
log-files-none = No files were handled.
log-file-new-chunks = saved as changes: { $size } of new chunks written
log-file-resumed = already copied by the interrupted backup, kept
log-count-new = New: { $count }
log-count-changed = Changed: { $count }
log-count-as-changes = Saved as changes: { $count }
log-count-copied-again = Copied again: { $count }
log-count-unchanged = Unchanged, not listed: { $count }
log-count-deleted = Deleted: { $count }
log-count-moved = Moved: { $count }
log-count-dropped = Versions removed: { $count }
log-count-restored = Restored: { $count }
log-count-kept = Kept: { $count }
log-count-checked = Read back intact: { $count }
log-count-checked-unlisted = Read back intact, not listed: { $count }
log-count-damaged = Problems: { $count }
log-count-skipped = Skipped: { $count }

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
password-current = Current password
password-current-hint = Type the current password.
recovery-ask = To show the recovery key, type the current password of these backups.
recovery-show-button = Show

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
log-op-verify = Integrity check
log-op-restore-test = Restore test

# Automatic backup: schedules, events, notifications
schedule-days = On chosen days
schedule-hours = Every few hours
schedule-monthly = Once a month
schedule-all-days = Every day
schedule-add-time = + Add a time
schedule-remove-time = Remove this time
schedule-every = Every
hours-unit = hours
schedule-from = from
schedule-until = until
schedule-hours-hint = The last run starts before the end time; the same start and end time means around the clock.
schedule-month-day = On day
schedule-month-hint = Days 1 to 28 only, so that every month has the day.
events-title = Also back up:
event-logon = A few minutes after I sign in to Windows
event-idle = When the computer has been idle for 10 minutes
event-drive = When a drive is connected
event-drive-hint = Useful for an external drive as destination: the backup starts when it is plugged in, and nothing happens when another drive is.
event-min-hours = Not more often than every
event-hint = A backup started by one of these events is skipped when the last backup is more recent, or when the destination is not connected.
alerts = Notifications
alert-on-failure = Notify me when an automatic backup fails
alert-stale = Warn me when there has been no backup for
alert-hint = Windows notifications; 0 days = no warning. The check runs every day and after signing in.
alert-manual-hint = Only for automatic backups: choose a schedule in "Automatic backup".
alert-failed-title = Archeion: backup of "{ $name }" failed
alert-stale-title = Archeion: "{ $name }" not backed up
alert-stale-body = No backup for { $days } days (last on { $when }). Check that the destination is connected, or start a backup from Archeion.
closing-after-run = An operation is running: Archeion will close by itself when it ends.
closing-cancel = Do not close

# Notification area and Preferences window
menu-preferences = Preferences…
preferences-title = Preferences
preferences-window = Window
language = Language
language-system = System language
language-tray-restart = The menu of the notification-area icon changes language the next time Archeion starts.
quit-hint = Close Archeion; scheduled backups still run from Windows Task Scheduler.
tray-open = Open Archeion
tray-quit = Quit
tray-running = operation in progress
option-tray = Keep running in the notification area when the window is closed
option-tray-hint = Closing the window hides it next to the clock: Archeion keeps merging old backups and finishing the operation in progress. Click its icon to open it again, or choose Quit.
option-tray-unavailable = Windows did not provide a notification-area icon.
option-autostart = Start Archeion with Windows
option-autostart-hint = When you sign in, Archeion starts hidden in the notification area. You can also turn this off in Task Manager under Startup apps.
autostart-failed = Could not change the start with Windows: { $detail }

# Help menu: user guide and About
menu-documentation = Documentation
menu-about = About Archeion
about-version = Version { $version }
about-name = The name is the ancient Greek ἀρχεῖον, "archive".
about-author = Author: MR
about-license = License: GPL-3.0
about-repository = Source code on GitHub
about-logs-folder = Logs folder
about-open-folder = Open folder
about-folder-missing = The folder does not exist yet.
doc-title = Archeion documentation
doc-intro = Archeion keeps copies of your folders on another drive, a network share or any other folder, and gives you back any file as it was on a past date. Open a section to read it.
doc-what-title = What Archeion does
doc-what-backups = The first backup of a profile is complete: it copies every file. The following ones are incremental: they copy only new and changed files, so they are quick and small, yet every date can be restored whole. Under "Complete backup" in the profile settings you can also ask for a new complete backup every few backups, or when the last one is older than some days; a complete backup starts a new chain that no longer depends on the older backups.
doc-what-storage = Each file is stored on its own, in the same folders as the original: as a zip of that file ("One zip per file": report.docx becomes report.docx.zip), as a plain copy, or as an AES-256 encrypted zip. The backups can be read without Archeion: File Explorer and 7-Zip open the zips, and 7-Zip opens the encrypted ones with the recovery key.
doc-what-safe = An interrupted backup (computer turned off, drive removed, Cancel) leaves the existing backups as they were; the next backup goes on from where it stopped and does not copy again the files already copied.
doc-profiles-title = Profiles
doc-profiles-body = A profile says what to back up, where and when. Create one with File > New profile (Ctrl+N) and fill in its Settings tab: a name, the folders to back up, and the destination, a folder or external drive or a network share. Use each destination folder for one profile only.
doc-profiles-exclusions = Exclusions leave out what you do not need: a folder and everything in it, a file, or every file of a type such as .tmp. With "Exclude patterns" you can add patterns, matched against the full path ignoring case: * is any text in a name, ** any folders, ? one character. For example **/node_modules or **/cache/**.
doc-profiles-format = The storage format (one zip per file or plain copies) and encryption must be chosen before the first backup: once the destination holds backups they cannot change. To switch, choose an empty destination.
doc-profiles-delete = Delete profile asks for confirmation, then removes the profile and its scheduled tasks; the backups stay at the destination. Right after, Undo in the status bar brings the profile back. A profile that uses that folder later can take them over with "Use these backups for this profile".
doc-auto-title = Automatic backups
doc-auto-schedule = Under "Automatic backup" choose when backups run by themselves: on chosen days at one or more times, every few hours between two times, or once a month on a day from 1 to 28. Archeion registers the schedule in Windows Task Scheduler, so backups run even when Archeion is not open.
doc-auto-missed = If the computer is off at the scheduled time, the backup runs as soon as it is on again.
doc-auto-alerts = Under "Notifications" Archeion can show a Windows notification when an automatic backup fails, and a warning when a profile has had no backup for a number of days (checked every day and after you sign in).
doc-auto-now = You can always start a backup yourself: Back up now in the profile settings, or Backup > Incremental backup now (Ctrl+B) and Complete backup now.
doc-merge-title = Merging old backups
doc-merge-body = Old backups are not deleted but merged into the next backup that is kept: every date kept stays complete, and the space of the versions no longer needed is freed. The merge is automatic: it runs while Archeion is running, never while a backup or another operation uses the destination.
doc-merge-rules = The rules in "Merge old backups" say which backups to keep, and a backup is kept if any rule selects it: all the backups of the last days, the most recent ones, one per day, one per week, one per month and one per year, each for the number you set. With every rule at 0 all backups are kept and nothing is merged.
doc-merge-first = The first complete backup is never merged automatically.
doc-merge-manual = In Backups > Manage backups, "Choose backups to merge…" merges by hand: tick the oldest and the newest backup, and everything between them becomes one backup. If the range includes the first complete backup, Archeion warns you first, because the earlier dates can no longer be restored.
doc-merge-delete = "Delete this backup…" in Backups > Manage backups removes one date; the backups after it stay complete. A complete backup that later backups depend on cannot be deleted (merge it instead). A deletion cannot be undone.
doc-encrypt-title = Encryption
doc-encrypt-body = With "Encrypt backups" every file becomes an AES-256 zip with a hidden name: without the password nobody can read the content or see which files are saved. Encryption needs the "One zip per file" format and must be turned on before the first backup; it cannot be changed afterwards.
doc-encrypt-password = The password is saved in Windows Credential Manager, so automatic backups need no typing. You can change it later in the profile settings, typing the current one. On another computer, type it once in the profile settings.
doc-encrypt-recovery = After the first backup, "Show the recovery key…" shows a key with which 7-Zip opens every encrypted file, even without Archeion and without the password. Print it or write it down and keep it offline: without the password and without the key nobody can open the backups. The key does not change when you change the password. Archeion asks for the current password before showing it.
doc-large-title = Large files
doc-large-body = A large file that changes a little (a mailbox, a virtual machine disk, a database) is not copied again in full: it is saved in pieces of about 1 MB, and each backup stores only the pieces that are new. Identical pieces are stored once. This applies to files of at least the size chosen in the Storage section of the profile (500 MB unless changed; it can be turned off), and to every file on an encrypted destination.
doc-large-complete = Complete backups still keep every file whole, as a zip, a plain copy or an encrypted zip, so they can be restored with 7-Zip alone; only the incremental backups after them hold pieces. Restoring puts the pieces back together and checks each one; the integrity check reads every piece.
doc-large-upgrade = Backups made by earlier versions stay as they are and remain readable. Pieces start with the next complete backup: on such a destination the next automatic backup is complete.
doc-restore-title = Restoring files
doc-restore-body = Open the Backups tab of the profile (or Backup > Restore files…): "Restore files" shows one tree of every file and folder the backups ever saved; files no longer at the source are marked { $deleted-icon } deleted and can still be restored. Tick files or folders in the tree or in the search results (the search box finds a file by name). Each ticked item appears on the right; { $remove-icon } removes it.
doc-restore-versions = A ticked file lists its versions, newest first, each a different content: the newest is ticked, and you can tick more. A ticked folder comes back whole, as it was on the date chosen in "As it was on" (the newest unless changed).
doc-restore-where = At the top of the right panel choose where everything goes: next to the original with the date in the name (report.docx comes back as "report (2026-11-02 21.30).docx" and the current file is not touched), the original location, or another folder; existing files are replaced only if you tick "Replace files that already exist". A version or folder can go elsewhere with its own menu, marked { $edited-icon }. Several versions of a file sent to the same folder each get their date in the name; only one version of a file can overwrite the original. Restore then brings everything back in one run.
doc-verify-title = Integrity check and restore test
doc-verify-body = The full check reads back every stored file of every backup and compares it with its fingerprint; the restore test reads back some files of the newest backup at random, as a restore would, without writing them anywhere. Start them from the Checks section of the profile settings (or Backup > Check backups now), or have them run every few days after a scheduled backup. If problems are found, make a complete backup and check the destination drive.
doc-activity-title = Activity tab
doc-activity-body = The Activity tab shows the progress of the running backup, merge, restore or check, with a Cancel button, and the log of every past operation, automatic or started by hand: what it did, and which files were skipped because another program had them open. Above the steps of an operation, Open full log shows its whole log as a text file in your editor: the steps, then every file it worked on with the time, what happened to it (new, changed, saved as changes, copied again, deleted, moved, version removed, restored, kept, skipped, problem), its size and its full path; unchanged files of an incremental backup and intact files of a full check are only counted. Copy log copies the same text; Program log opens Archeion's technical log, for deeper problems.
doc-window-title = Closing the window
doc-window-body = Closing the window does not stop Archeion: it moves to the notification area, next to the clock, and keeps working, so old backups are still merged; open the window again from its icon there. Archeion also starts with Windows, hidden there. File > Quit, or Quit in the icon's menu, really closes it. Both can be turned off in File > Preferences, where you also choose the language. Scheduled backups run in any case, even when Archeion is not running.
doc-cli-title = Command line
doc-cli-body = archeion.exe without arguments opens the window. The commands below run one operation without the window, as the scheduled tasks do. The profile id is the id of the profile in settings.toml.
doc-cli-run = Backs up the profile (--kind complete for a complete backup).
doc-cli-merge = Merges old backups by the rules of the profile.
doc-cli-verify = Checks the backups (--sample N: restore test of N files).
doc-cli-delete = Deletes one backup.
doc-cli-restore = Restores the files listed in a request file written by the window.
doc-cli-watch = Warns about profiles with no recent backup; Task Scheduler runs it every day.
doc-cli-extract = Restores from a backup folder alone, without settings or a profile: for example with the portable Archeion on another computer. The files go below the target folder with their full path.
doc-cli-extract-options = Options of extract: --archive <name> chooses the backup (the newest when left out); --path <file or folder> restores only that, given as its original path; --overwrite replaces existing files. For encrypted backups give the password or the recovery key with --stdin (first line of standard input), with the ARCHEION_PASSWORD or ARCHEION_RECOVERY_KEY variable, or with --password or --recovery-key. Progress is also written to archeion-extract.log in the target folder. In a Command Prompt start it as start /wait archeion extract … so that the prompt waits for it.
doc-cli-exit = Exit codes: 0 success, 1 failure, 2 destination missing, 3 busy (another operation is using the destination), 4 password missing or wrong, 5 not enough free space.
doc-data-title = Where data and logs are
doc-data-body = Settings are in the folder %APPDATA%\Archeion\config (settings.toml), and the log of each operation is in its logs subfolder. A portable copy (with a portable.txt file next to archeion.exe) keeps everything in a data folder next to the executable instead. Help > About Archeion opens these folders.
doc-data-backups = The backups themselves are only at the destination: one folder per backup, named with its date and time, holding the stored files and a list of every file at that time.

# Activity tab: steps and recent files of the running operation
step-found = { $count ->
    [one] 1 file found
   *[other] { $count } files found
}
step-folders = { $count ->
    [one] 1 folder read
   *[other] { $count } folders read
}
step-new = { $count } new
step-changed = { $count } changed
step-unchanged = { $count } unchanged
step-copied-again = { $count } unchanged copied again
step-moved = { $count ->
    [one] 1 file moved
   *[other] { $count } files moved
}
step-restored = { $count } restored
step-kept = { $count } already present, kept
step-checked = { $count } checked
step-damaged = { $count ->
    [one] 1 problem
   *[other] { $count } problems
}
step-skipped = { $count } skipped
merge-group = { $count } backups, from { $first } to { $last }, into one
merge-group-of = Group { $index } of { $of }: { $count } backups, from { $first } to { $last }, into one
recent-title = Latest files
recent-new = New
recent-changed = Changed
recent-copied-again = Copied again
recent-moved = Moved
recent-restored = Restored
recent-kept = Kept
recent-checked = OK
recent-damaged = Problem
recent-skipped = Skipped
recent-deleted = Deleted
recent-dropped = Version removed
