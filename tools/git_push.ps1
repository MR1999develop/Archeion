# Pushes the current branch to GitHub (origin). Every step is shown on
# screen and written to tools\git_push.log:
#   1. branch and remote: without an origin it says to run
#      tools\github_publish.bat first;
#   2. "git fetch", then stops if GitHub has commits this branch lacks
#      (they must be pulled first: this script never force-pushes);
#   3. pushes, linking the branch to origin the first time (-u);
#   4. for a feature branch, shows the open pull request, or how to open
#      one; with -PullRequest it opens it with the GitHub CLI towards main,
#      title and description taken from the commits (like "gh pr create
#      --fill") plus the project's footer.
# The last line of the log is "PUSH DONE <code>": 0 means pushed (or
# nothing to push), 1 means something failed (see the log).
[CmdletBinding()]
param(
    [switch]$PullRequest,
    [switch]$NoPause
)

$LogName = 'git_push'
$DoneLabel = 'PUSH'
. (Join-Path $PSScriptRoot 'git_common.ps1')

$BaseBranch = 'main'
$SessionUrl = 'https://claude.ai/code/session_01MXC7KQwLfPvfbbuZdPzBSm'
# U+1F916 (robot face), built here so the script does not depend on how
# its own file is decoded.
$PrFooter = @(
    ([char]::ConvertFromUtf32(0x1F916) + ' Generated with [Claude Code](https://claude.com/claude-code)')
    ''
    $SessionUrl
)

# Title and description of the pull request from the commits of the branch
# that are not in main, as "gh pr create --fill" does: one commit gives its
# subject and body, several give the branch name and the list of subjects.
function Get-PullRequestText([string]$branch, [string]$range) {
    $subjects = @(& git log --reverse --format=%s $range 2>$null)
    $body = New-Object System.Collections.Generic.List[string]
    if ($subjects.Count -eq 1) {
        $title = $subjects[0]
        foreach ($line in @(& git log -1 --format=%b HEAD 2>$null)) {
            if ($line -notmatch '^(Co-Authored-By|Claude-Session): ') { $body.Add($line) }
        }
    } else {
        $name = (($branch -split '/')[-1] -replace '[-_]+', ' ').Trim()
        if ($name) { $name = $name.Substring(0, 1).ToUpperInvariant() + $name.Substring(1) } else { $name = $branch }
        $title = $name
        foreach ($subject in $subjects) { $body.Add("- $subject") }
    }
    while ($body.Count -gt 0 -and $body[$body.Count - 1].Trim() -eq '') { $body.RemoveAt($body.Count - 1) }
    if ($body.Count -gt 0) { $body.Add('') }
    foreach ($line in $PrFooter) { $body.Add($line) }
    # Windows PowerShell 5.1 breaks arguments with double quotes in them.
    [pscustomobject]@{ Title = ($title -replace '"', "'"); Body = ($body -join "`n") + "`n" }
}

Write-Start 'Archeion push'
if (-not (Get-GitValue @('rev-parse', '--verify', '--quiet', 'HEAD'))) {
    Write-Log 'The repository has no commit yet: run tools\git_init.bat first.' 'Red'
    Stop-Script 1
}

# 1. Branch and remote.
Write-Header '1/4 Branch and remote'
$branch = Get-GitValue @('branch', '--show-current')
if (-not $branch) {
    Write-Log 'You are not on a branch (detached HEAD): go back to one with "git switch <branch>".' 'Red'
    Stop-Script 1
}
Write-Log "Branch:    $branch"
$origin = Get-GitValue @('remote', 'get-url', 'origin')
if (-not $origin) {
    Write-Log 'There is no remote called origin: the repository is not on GitHub yet.' 'Red'
    Write-Log 'Run tools\github_publish.bat first: it creates the GitHub repository and' 'Red'
    Write-Log 'pushes main and this branch. After that, use this script.' 'Red'
    Stop-Script 1
}
$slug = Get-GitHubSlug $origin
Write-Log "origin:    $origin"
$changes = Get-GitChanges
if ($changes.Count -gt 0) {
    Write-Log "$($changes.Count) changed file(s) are not committed and will not be pushed." 'Yellow'
    Write-Log 'If they belong to this push, run tools\git_commit.bat first.' 'Yellow'
}
if ($ProtectedBranches -contains $branch) {
    Write-Log "You are pushing $branch directly. Day-to-day work goes on a feature branch" 'Yellow'
    Write-Log "and reaches $branch through a pull request (GitHub may refuse it if $branch is protected)." 'Yellow'
}

# 2. What GitHub has.
Write-Header '2/4 Compare with GitHub'
$fetch = Invoke-Tool git @('fetch', 'origin')
if ($fetch.Code -ne 0) {
    Write-Log 'git fetch failed (offline, or not signed in to GitHub?): nothing was pushed.' 'Red'
    Stop-Script 1
}
$upstream = Get-GitValue @('rev-parse', '--abbrev-ref', '--symbolic-full-name', '@{u}')
$remoteRef = ''
if ($upstream) {
    $remoteRef = $upstream
    Write-Log "Upstream:  $upstream"
} elseif (Get-GitValue @('rev-parse', '--verify', '--quiet', "refs/remotes/origin/$branch")) {
    $remoteRef = "origin/$branch"
    Write-Log "Upstream:  none yet, but origin/$branch exists on GitHub" 'Yellow'
} else {
    Write-Log "Upstream:  none: $branch is new on GitHub; the push will create origin/$branch."
}
$ahead = 0
$behind = 0
if ($remoteRef) {
    $counts = (Get-GitValue @('rev-list', '--left-right', '--count', "$remoteRef...HEAD")) -split '\s+'
    if ($counts.Count -ge 2) { $behind = [int]$counts[0]; $ahead = [int]$counts[1] }
    Write-Log "$ahead commit(s) to push, $behind commit(s) on GitHub that are not here."
    if ($behind -gt 0) {
        Write-Log "GitHub has $behind commit(s) on $branch that this computer does not have:" 'Red'
        & git log --oneline "HEAD..$remoteRef" 2>$null | ForEach-Object { Write-Log "   $_" 'Red' }
        Write-Log 'Get them first with "git pull" (commit or stash your changes before), then' 'Red'
        Write-Log 'run this again. This script never overwrites GitHub (no force push).' 'Red'
        Stop-Script 1
    }
    $range = "$remoteRef..HEAD"
} else {
    $range = if (Get-GitValue @('rev-parse', '--verify', '--quiet', "refs/remotes/origin/$BaseBranch")) { "origin/$BaseBranch..HEAD" } else { 'HEAD' }
    $ahead = [int](Get-GitValue @('rev-list', '--count', $range))
}
if ($ahead -gt 0) {
    Write-Log 'Commits to push:'
    & git log --oneline -30 $range 2>$null | ForEach-Object { Write-Log "   $_" }
}

# 3. Push (never forced).
Write-Header '3/4 Push'
if ($upstream -and $ahead -eq 0) {
    Write-Log "Nothing to push: origin/$branch is up to date." 'Green'
} else {
    $pushArgs = @('push')
    if (-not $upstream) { $pushArgs += '-u' }
    $pushArgs += @('origin', $branch)
    $push = Invoke-Tool git $pushArgs
    if ($push.Code -ne 0) {
        Write-Log "git push failed (exit code $($push.Code), see above)." 'Red'
        Write-Log 'If GitHub "rejected" it, it has commits you do not have: "git pull", then run this again.' 'Red'
        Write-Log 'If it asked for credentials, sign in in the window Git opens (Git Credential Manager).' 'Red'
        Stop-Script 1
    }
    Write-Log "Pushed $branch to origin." 'Green'
}

# 4. Pull request.
Write-Header '4/4 Pull request'
$code = 0
if ($ProtectedBranches -contains $branch) {
    Write-Log "None for $branch."
    if ($PullRequest) { Write-Log '-PullRequest is ignored on main: switch to a feature branch.' 'Yellow' }
    Stop-Script 0
}
if (-not (Get-GitValue @('rev-parse', '--verify', '--quiet', "refs/remotes/origin/$BaseBranch"))) {
    Write-Log "$BaseBranch is not on GitHub yet, so there is nothing to open a pull request against." 'Yellow'
    Write-Log 'Run tools\github_publish.bat: it pushes main as well.' 'Yellow'
    Stop-Script $(if ($PullRequest) { 1 } else { 0 })
}
$prRange = "origin/$BaseBranch..HEAD"
$prCommits = [int](Get-GitValue @('rev-list', '--count', $prRange))
if ($prCommits -eq 0) {
    Write-Log "$branch has no commit that is not already in ${BaseBranch}: no pull request needed."
    Stop-Script 0
}
Write-Log "$prCommits commit(s) of $branch are not in $BaseBranch yet."
$compareUrl = if ($slug) { "https://github.com/$slug/compare/$BaseBranch...${branch}?expand=1" } else { '' }
$ghReady = $false
if (Test-Tool 'gh') {
    $auth = Invoke-Tool gh @('auth', 'status') -Quiet
    $ghReady = $auth.Code -eq 0
    if (-not $ghReady) { Write-Log 'The GitHub CLI is installed but not signed in: run "gh auth login" once.' 'Yellow' }
} else {
    Write-Log 'The GitHub CLI (gh) is not installed: https://cli.github.com' 'Yellow'
}
if (-not $ghReady -or -not $slug) {
    if ($compareUrl) { Write-Log "Open the pull request in the browser: $compareUrl" 'Yellow' }
    if ($PullRequest) {
        Write-Log '-PullRequest needs the GitHub CLI signed in and an origin on github.com.' 'Red'
        $code = 1
    }
    Stop-Script $code
}
Write-Log "> gh pr list --repo $slug --head $branch --state open --json url --jq .[].url" 'DarkGray'
$openPr = (@(& gh pr list --repo $slug --head $branch --state open --json url --jq '.[].url' 2>$null) -join ' ').Trim()
if ($LASTEXITCODE -ne 0) {
    Write-Log 'Could not ask GitHub for pull requests (see "gh auth status").' 'Red'
    Stop-Script 1
}
if ($openPr) {
    Write-Log "A pull request is already open; the push updated it: $openPr" 'Green'
    Stop-Script 0
}
if (-not $PullRequest) {
    Write-Log "No pull request open for $branch yet. To open one towards ${BaseBranch}:" 'Yellow'
    Write-Log '   tools\git_push.bat -PullRequest' 'Yellow'
    Write-Log "(it runs gh pr create --base $BaseBranch with title and description from the" 'Yellow'
    Write-Log ' commits, like --fill, plus the project footer), or in the browser:' 'Yellow'
    if ($compareUrl) { Write-Log "   $compareUrl" 'Yellow' }
    Stop-Script 0
}
$text = Get-PullRequestText $branch $prRange
$gitDir = Get-GitValue @('rev-parse', '--absolute-git-dir')
$bodyPath = Join-Path $gitDir 'ARCHEION_PR_BODY.md'
Write-Utf8File $bodyPath $text.Body
Write-Log "Title: $($text.Title)"
Write-Log 'Description:'
foreach ($line in ($text.Body.TrimEnd() -split "`n")) { Write-Log "   | $line" }
Write-Log ''
$pr = Invoke-Tool gh @('pr', 'create', '--repo', $slug, '--base', $BaseBranch, '--head', $branch, '--title', $text.Title, '--body-file', $bodyPath)
Remove-Item -LiteralPath $bodyPath -ErrorAction SilentlyContinue
if ($pr.Code -ne 0) {
    Write-Log "gh pr create failed (exit code $($pr.Code), see above). The branch is pushed;" 'Red'
    Write-Log 'run this again, or open the pull request in the browser.' 'Red'
    Stop-Script 1
}
Write-Log 'Pull request opened (the URL is above). CI runs on it; merge it on GitHub once green.' 'Green'
Stop-Script 0
