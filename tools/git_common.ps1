# Shared part of git_status.ps1, git_commit.ps1, git_push.ps1 and
# github_publish.ps1: the log written on screen and to tools\<name>.log,
# the coloured section headers, git commands shown with their output, and
# the last line "<LABEL> DONE <code>" followed by the pause used when the
# script was started by a double-click.
# Each script sets $LogName and $DoneLabel after its param() block, then
# dot-sources this file:
#   $LogName = 'git_status'; $DoneLabel = 'STATUS'
#   . (Join-Path $PSScriptRoot 'git_common.ps1')

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
$log = Join-Path $PSScriptRoot "$LogName.log"
Set-Content -LiteralPath $log -Value '' -Encoding utf8
# git and gh write UTF-8: decode it as such (file names, messages with accents).
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ProtectedBranches = @('main', 'master')

function Write-Log([string]$text, [string]$color = '') {
    if ($color) { Write-Host $text -ForegroundColor $color } else { Write-Host $text }
    Add-Content -LiteralPath $log -Value $text -Encoding utf8
}

function Write-Header([string]$text) {
    Write-Log ''
    Write-Log ('=' * 78) 'Cyan'
    Write-Log "== $text" 'Cyan'
    Write-Log ('=' * 78) 'Cyan'
}

# Wait for Enter only when started by a double-click (Explorer -> cmd -> here),
# so the window does not vanish; from a terminal it returns at once.
function Test-StartedFromExplorer {
    try {
        $me = Get-CimInstance Win32_Process -Filter "ProcessId=$PID"
        $cmd = Get-CimInstance Win32_Process -Filter "ProcessId=$($me.ParentProcessId)"
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$($cmd.ParentProcessId)"
        return $parent.Name -eq 'explorer.exe'
    } catch { return $false }
}

function Stop-Script([int]$code) {
    Write-Log ''
    Write-Log "$DoneLabel DONE $code"
    if (-not $NoPause -and (Test-StartedFromExplorer)) {
        Write-Host ''
        Read-Host 'Press Enter to close'
    }
    exit $code
}

function Test-Tool([string]$name) {
    return [bool](Get-Command $name -CommandType Application -ErrorAction SilentlyContinue)
}

# Runs git or gh, shows the command and each output line (stderr included,
# in order) unless -Quiet, and returns the exit code and the lines.
function Invoke-Tool {
    param([string]$exe, [string[]]$arguments, [switch]$Quiet)
    if (-not $Quiet) { Write-Log ("> $exe " + ($arguments -join ' ')) 'DarkGray' }
    $lines = New-Object System.Collections.Generic.List[string]
    & $exe @arguments 2>&1 | ForEach-Object {
        $line = "$_"
        $lines.Add($line)
        if (-not $Quiet) {
            $color = ''
            if ($line -match '^(error|fatal|\s*! \[rejected\])') { $color = 'Red' }
            elseif ($line -match '^(warning|hint:)') { $color = 'Yellow' }
            Write-Log $line $color
        }
    }
    $code = $LASTEXITCODE
    [pscustomobject]@{ Code = $code; Lines = $lines.ToArray(); Text = ($lines -join "`n").Trim() }
}

# Output of a git command with nothing shown, or '' when it fails.
function Get-GitValue([string[]]$arguments) {
    $out = & git @arguments 2>$null
    if ($LASTEXITCODE -ne 0) { return '' }
    return (@($out) -join "`n").Trim()
}

# Lines of "git status --porcelain", with file names as they are (no octal
# escapes for accented letters).
function Get-GitChanges {
    return @(& git -c core.quotepath=false status --porcelain=v1 --untracked-files=all 2>$null |
        Where-Object { $_ })
}

# "owner/name" from a GitHub remote URL (https or ssh), or ''.
function Get-GitHubSlug([string]$url) {
    if ($url -match 'github\.com[:/]+([^/]+)/([^/]+?)(\.git)?/?$') {
        return "$($Matches[1])/$($Matches[2])"
    }
    return ''
}

# Name of a merge, rebase, cherry-pick or revert left half done, or ''.
function Get-OperationInProgress {
    $gitDir = Get-GitValue @('rev-parse', '--absolute-git-dir')
    if (-not $gitDir) { return '' }
    $marks = [ordered]@{
        'MERGE_HEAD'       = 'merge (git merge --abort to cancel it)'
        'rebase-merge'     = 'rebase (git rebase --abort to cancel it)'
        'rebase-apply'     = 'rebase or am (git rebase --abort to cancel it)'
        'CHERRY_PICK_HEAD' = 'cherry-pick (git cherry-pick --abort to cancel it)'
        'REVERT_HEAD'      = 'revert (git revert --abort to cancel it)'
    }
    foreach ($mark in $marks.Keys) {
        if (Test-Path -LiteralPath (Join-Path $gitDir $mark)) { return $marks[$mark] }
    }
    return ''
}

# Common start: title, folder, git present, repository present.
function Write-Start([string]$title) {
    Write-Header "$title - $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))"
    Write-Log "Folder:    $root"
    Write-Log "Computer:  $env:COMPUTERNAME"
    if (-not (Test-Tool 'git')) {
        Write-Log 'git was not found. Install Git for Windows (https://git-scm.com/download/win)' 'Red'
        Write-Log 'and open a new window so that git is on the PATH.' 'Red'
        Stop-Script 1
    }
    Write-Log ("git:       " + (Get-GitValue @('--version')))
    if (-not (Test-Path -LiteralPath (Join-Path $root '.git'))) {
        Write-Log 'No git repository here: clone it, or create it with git init.' 'Red'
        Stop-Script 1
    }
}

# Writes a file as UTF-8 without BOM (git would keep a BOM in the message).
function Write-Utf8File([string]$path, [string]$text) {
    [System.IO.File]::WriteAllText($path, $text, (New-Object System.Text.UTF8Encoding($false)))
}
