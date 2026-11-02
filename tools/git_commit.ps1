# Commits all the current work, safely. Every step is shown on screen and
# written to tools\git_commit.log:
#   1. stops if nothing changed, or a merge or rebase is half done;
#   2. reads the message from -Message or tools\commit-message.txt (lines
#      starting with # are ignored), checks it (subject of 1 to 72
#      characters, then a blank line) and adds the project's trailers;
#   3. refuses to commit on main or master: -Branch <name> creates (or
#      switches to) that branch first, keeping the changes;
#   4. runs tools\check.ps1 (fmt, clippy, tests) and stops if it fails,
#      unless -SkipCheck;
#   5. stages everything ("git add -A", .gitignore respected) and shows it;
#   6. commits, setting the author for this repository only if git has none.
# After a commit taken from tools\commit-message.txt, that file is moved to
# tools\commit-message.last.txt and an empty template is written again.
# Nothing is pushed: that is tools\git_push.bat.
# The last line of the log is "COMMIT DONE <code>": 0 means committed (or
# nothing to commit), 1 means nothing was committed (see the log).
[CmdletBinding()]
param(
    [string]$Message = '',
    [string]$Branch = '',
    [switch]$SkipCheck,
    [switch]$NoPause
)

$LogName = 'git_commit'
$DoneLabel = 'COMMIT'
. (Join-Path $PSScriptRoot 'git_common.ps1')

# Trailers the project adds to every commit (only those not already there).
$Trailers = @(
    'Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>'
    'Claude-Session: https://claude.ai/code/session_01MXC7KQwLfPvfbbuZdPzBSm'
)
$DefaultName = 'MR'
$DefaultEmail = 'MR1999develop@users.noreply.github.com'
$MaxSubject = 72
$MaxFileMb = 100
$WarnFileMb = 50
$messageFile = Join-Path $PSScriptRoot 'commit-message.txt'
$lastMessageFile = Join-Path $PSScriptRoot 'commit-message.last.txt'

function New-MessageTemplate {
    $template = @(
        '# Commit message for tools\git_commit.bat. Lines starting with # are ignored.'
        '#'
        '# Line 1: the subject, in English, imperative, at most 72 characters'
        '#         ("Add scheduled merge of old backups").'
        '# Line 2: empty.'
        '# Line 3 and on: why the change was made, wrapped at about 72 characters.'
        '#'
        '# The Co-Authored-By and Claude-Session trailers are added by the script.'
        '# After a successful commit this file is moved to commit-message.last.txt'
        '# and this template is written again, so a message is never used twice.'
        '#'
        '# The | below is column 72:'
        ('#' + ('-' * 70) + '|')
        ''
    )
    Write-Utf8File $messageFile (($template -join "`r`n") + "`r`n")
}

# Message lines without comments, trailing spaces, and blank lines at the
# start and the end.
function ConvertTo-MessageLines([string]$raw) {
    $list = New-Object System.Collections.Generic.List[string]
    foreach ($line in (($raw -replace "`r`n", "`n") -replace "`r", "`n") -split "`n") {
        if ($line.StartsWith('#')) { continue }
        $list.Add($line.TrimEnd())
    }
    while ($list.Count -gt 0 -and $list[0].Trim() -eq '') { $list.RemoveAt(0) }
    while ($list.Count -gt 0 -and $list[$list.Count - 1] -eq '') { $list.RemoveAt($list.Count - 1) }
    if ($list.Count -gt 0) { $list[0] = $list[0].Trim() }
    return , $list
}

# "feature/add-scheduled-merge" from "Add scheduled merge of old backups".
function Get-BranchSuggestion([string]$subject) {
    $prefix = if ($subject -match '^(fix|correct|repair)') { 'fix' } else { 'feature' }
    $words = @(($subject.ToLowerInvariant() -replace '[^a-z0-9]+', ' ').Trim() -split ' ' |
        Where-Object { $_ -and $_ -notin @('a', 'an', 'the', 'of', 'to', 'and', 'for', 'in', 'on') } |
        Select-Object -First 4)
    if ($words.Count -eq 0) { return "$prefix/<short-name>" }
    return "$prefix/" + ($words -join '-')
}

Write-Start 'Archeion commit'
$operation = Get-OperationInProgress
if ($operation) {
    Write-Log "A $operation is in progress: finish or cancel it first." 'Red'
    Stop-Script 1
}
if (-not (Get-GitValue @('rev-parse', '--verify', '--quiet', 'HEAD'))) {
    Write-Log 'The repository has no commit yet: run tools\git_init.bat for the first one.' 'Red'
    Stop-Script 1
}

# 1. Anything to commit?
Write-Header '1/6 Changes'
$changes = Get-GitChanges
if ($changes.Count -eq 0) {
    Write-Log 'Nothing changed since the last commit: nothing to do.' 'Green'
    Stop-Script 0
}
Write-Log "$($changes.Count) changed or new file(s)."

# 2. Message.
Write-Header '2/6 Message'
$fromFile = $false
if ($Message.Trim()) {
    Write-Log 'Taken from -Message.'
    $raw = $Message
} elseif (Test-Path -LiteralPath $messageFile) {
    Write-Log "Taken from $messageFile"
    $raw = Get-Content -LiteralPath $messageFile -Raw -Encoding UTF8
    if ($null -eq $raw) { $raw = '' }
    $fromFile = $true
} else {
    New-MessageTemplate
    $raw = ''
}
$lines = ConvertTo-MessageLines $raw
if ($lines.Count -eq 0) {
    Write-Log 'The message is empty. Write it in tools\commit-message.txt (subject on the first' 'Red'
    Write-Log 'line, a blank line, then the details) or pass it with -Message "Subject".' 'Red'
    Stop-Script 1
}
$subject = $lines[0]
if ($subject.Length -gt $MaxSubject) {
    Write-Log "The subject has $($subject.Length) characters, at most $MaxSubject are allowed:" 'Red'
    Write-Log "   $subject" 'Red'
    Write-Log "   $(' ' * $MaxSubject)^ cut here" 'Red'
    Write-Log 'Shorten it and move the details to the body (after a blank line).' 'Red'
    Stop-Script 1
}
if ($lines.Count -gt 1 -and $lines[1] -ne '') {
    Write-Log 'The second line must be blank: the subject is one line, the details follow' 'Red'
    Write-Log 'after an empty line.' 'Red'
    Stop-Script 1
}
$missing = @($Trailers | Where-Object { $lines -notcontains $_ })
if ($missing.Count -gt 0) {
    $last = $lines[$lines.Count - 1]
    if (-not ($lines.Count -ge 3 -and $last -match '^(Co-Authored-By|Claude-Session|Signed-off-by): ')) {
        $lines.Add('')
    }
    foreach ($trailer in $missing) { $lines.Add($trailer) }
}
Write-Log 'Message:'
foreach ($line in $lines) { Write-Log "   | $line" }

# 3. Branch: never commit on main directly.
Write-Header '3/6 Branch'
$current = Get-GitValue @('branch', '--show-current')
if (-not $current) {
    Write-Log 'You are not on a branch (detached HEAD): go back to one with "git switch <branch>".' 'Red'
    Stop-Script 1
}
Write-Log "Current branch: $current"
if ($Branch) {
    $Branch = $Branch.Trim()
    $valid = $Branch -match '^[A-Za-z0-9][A-Za-z0-9._/-]*$'
    if ($valid) {
        & git check-ref-format --branch $Branch 2>$null | Out-Null
        $valid = $LASTEXITCODE -eq 0
    }
    if (-not $valid) {
        Write-Log "'$Branch' is not a usable branch name: use letters, digits, - _ . and /," 'Red'
        Write-Log 'for example feature/automatic-backup or fix/merge-journal.' 'Red'
        Stop-Script 1
    }
    if ($ProtectedBranches -contains $Branch) {
        Write-Log "-Branch ${Branch}: commits go on a feature branch, not on $Branch." 'Red'
        Stop-Script 1
    }
    if ($Branch -ne $current) {
        & git show-ref --verify --quiet "refs/heads/$Branch" 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Switching to the existing branch $Branch, keeping the changes."
            $switch = Invoke-Tool git @('switch', $Branch)
        } else {
            Write-Log "Creating the branch $Branch from $current, keeping the changes."
            $switch = Invoke-Tool git @('switch', '-c', $Branch)
        }
        if ($switch.Code -ne 0) {
            Write-Log "Could not switch to $Branch (see above): nothing was committed." 'Red'
            Stop-Script 1
        }
        $current = $Branch
    }
} elseif ($ProtectedBranches -contains $current) {
    $suggestion = Get-BranchSuggestion $subject
    Write-Log "You are on $current, which only receives work through pull requests." 'Red'
    Write-Log 'Run again naming a new branch; the changes move with you, for example:' 'Red'
    if ($fromFile -or -not $Message) {
        Write-Log "   tools\git_commit.bat -Branch $suggestion" 'Yellow'
    } else {
        Write-Log "   tools\git_commit.bat -Branch $suggestion -Message ""$($Message.Trim())""" 'Yellow'
    }
    Stop-Script 1
}
Write-Log "Committing on: $current" 'Green'

# 4. Check: fmt, clippy, tests.
Write-Header '4/6 Check (tools\check.ps1)'
if ($SkipCheck) {
    Write-Log 'Skipped (-SkipCheck): the code was not formatted, linted or tested.' 'Yellow'
} else {
    $checkScript = Join-Path $PSScriptRoot 'check.ps1'
    $checkLog = Join-Path $PSScriptRoot 'check.log'
    Write-Log '> powershell -File tools\check.ps1 -NoPause (its output: tools\check.log)' 'DarkGray'
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $checkScript -NoPause
    $checkCode = $LASTEXITCODE
    $watch.Stop()
    Set-Location -LiteralPath $root
    Write-Log ''
    Write-Log "Check summary (from tools\check.log, $([math]::Round($watch.Elapsed.TotalSeconds, 1)) s):"
    if (Test-Path -LiteralPath $checkLog) {
        $checkLines = @(Get-Content -LiteralPath $checkLog -Encoding UTF8)
        $start = -1
        for ($i = 0; $i -lt $checkLines.Count; $i++) {
            if ($checkLines[$i] -eq '== Summary') { $start = $i + 2 }
        }
        if ($start -ge 0) {
            for ($i = $start; $i -lt $checkLines.Count; $i++) {
                $line = $checkLines[$i]
                $color = ''
                if ($line -match '^FAILED|Something failed|^\s+\S') { $color = 'Red' }
                if ($line -match '^OK|Everything passed') { $color = 'Green' }
                Write-Log "   $line" $color
            }
        }
    }
    if ($checkCode -ne 0) {
        Write-Log "The check failed (exit code $checkCode): nothing was committed." 'Red'
        Write-Log 'Fix what tools\check.log reports and run again. Only if the commit must be' 'Red'
        Write-Log 'made anyway (work in progress on a feature branch), add -SkipCheck.' 'Red'
        Stop-Script 1
    }
    Write-Log 'Check passed.' 'Green'
}

# 5. Stage everything.
Write-Header '5/6 Stage'
Invoke-Tool git @('-c', 'core.quotepath=false', 'status', '--short', '--branch') | Out-Null
$add = Invoke-Tool git @('add', '-A')
if ($add.Code -ne 0) {
    Write-Log 'git add failed (see above): nothing was committed.' 'Red'
    Stop-Script 1
}
& git diff --cached --quiet 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Log 'Nothing is staged after git add (only ignored files changed?): nothing to commit.' 'Green'
    Stop-Script 0
}
Write-Log ''
Invoke-Tool git @('-c', 'core.quotepath=false', 'diff', '--cached', '--stat=100') | Out-Null
$tooBig = New-Object System.Collections.Generic.List[string]
foreach ($path in @(& git -c core.quotepath=false diff --cached --name-only --diff-filter=AM 2>$null)) {
    $full = Join-Path $root $path
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
    $mb = [math]::Round((Get-Item -LiteralPath $full).Length / 1MB, 1)
    if ($mb -ge $MaxFileMb) { $tooBig.Add("$path ($mb MB)") }
    elseif ($mb -ge $WarnFileMb) { Write-Log "Large file: $path ($mb MB). GitHub warns above $WarnFileMb MB." 'Yellow' }
}
if ($tooBig.Count -gt 0) {
    Write-Log "GitHub refuses files of $MaxFileMb MB or more, so the push would fail:" 'Red'
    $tooBig | ForEach-Object { Write-Log "   $_" 'Red' }
    Write-Log 'Add them to .gitignore (or move them out of the folder) and run again.' 'Red'
    Write-Log 'Nothing was committed; the files stay staged until then.' 'Red'
    Stop-Script 1
}

# 6. Commit.
Write-Header '6/6 Commit'
$name = Get-GitValue @('config', 'user.name')
$email = Get-GitValue @('config', 'user.email')
if (-not $name) {
    Invoke-Tool git @('config', 'user.name', $DefaultName) | Out-Null
    $name = $DefaultName
}
if (-not $email) {
    Invoke-Tool git @('config', 'user.email', $DefaultEmail) | Out-Null
    $email = $DefaultEmail
}
Write-Log "Author: $name <$email>"
$gitDir = Get-GitValue @('rev-parse', '--absolute-git-dir')
$messagePath = Join-Path $gitDir 'ARCHEION_COMMIT_MSG'
Write-Utf8File $messagePath (($lines -join "`n") + "`n")
$commit = Invoke-Tool git @('commit', '--cleanup=whitespace', '-F', $messagePath)
Remove-Item -LiteralPath $messagePath -ErrorAction SilentlyContinue
if ($commit.Code -ne 0) {
    Write-Log "git commit failed (exit code $($commit.Code), see above). The changes are staged;" 'Red'
    Write-Log 'nothing is lost.' 'Red'
    Stop-Script 1
}
if ($fromFile) {
    Move-Item -LiteralPath $messageFile -Destination $lastMessageFile -Force
    New-MessageTemplate
    Write-Log 'tools\commit-message.txt moved to commit-message.last.txt; the template is back.'
}
Write-Log ''
Invoke-Tool git @('-c', 'core.quotepath=false', 'show', '--stat=100', '--format=fuller', 'HEAD') | Out-Null
Write-Log ''
Write-Log "Committed on $current`: $(Get-GitValue @('log', '-1', '--format=%h %s'))" 'Green'
Write-Log '-> Next: tools\git_push.bat sends it to GitHub (-PullRequest also opens the pull request).' 'Yellow'
Write-Log '-> To undo this commit and keep the changes: git reset --soft HEAD~1' 'Yellow'
Stop-Script 0
