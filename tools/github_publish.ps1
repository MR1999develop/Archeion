# Publishes the repository on GitHub as MR1999develop/Archeion (private).
# Every step is shown on screen and written to tools\github_publish.log:
#   1. checks: a repository with commits and a local main;
#   2. remote: keeps an existing origin (stops if it points elsewhere);
#      otherwise links the GitHub repository if it already exists, or
#      creates it with the GitHub CLI (signed in once with "gh auth login");
#   3. pushes main first (so it becomes GitHub's default branch), then the
#      current branch if it is another one, then the tags; never forced;
#   4. shows the result and the next steps.
# Safe to run again: an existing repository or origin is reused, and a push
# with nothing new does nothing.
# The last line of the log is "GITHUB PUBLISH DONE <code>": 0 means done.
[CmdletBinding()]
param(
    [switch]$NoPause
)

$LogName = 'github_publish'
$DoneLabel = 'GITHUB PUBLISH'
. (Join-Path $PSScriptRoot 'git_common.ps1')

$Repo = 'MR1999develop/Archeion'
$Visibility = '--private'
$Description = 'Incremental, scheduled, optionally encrypted backups for Windows (Rust + egui)'
$BaseBranch = 'main'

Write-Start 'Archeion GitHub publication'
Write-Log "Target:    https://github.com/$Repo ($($Visibility.TrimStart('-')))"

# 1. Checks.
Write-Header '1/4 Checks'
if (-not (Get-GitValue @('rev-parse', '--verify', '--quiet', 'HEAD'))) {
    Write-Log 'The repository has no commit yet: run tools\git_init.bat first.' 'Red'
    Stop-Script 1
}
$operation = Get-OperationInProgress
if ($operation) {
    Write-Log "A $operation is in progress: finish or cancel it first." 'Red'
    Stop-Script 1
}
$branch = Get-GitValue @('branch', '--show-current')
if (-not $branch) {
    Write-Log 'You are not on a branch (detached HEAD): go back to one with "git switch <branch>".' 'Red'
    Stop-Script 1
}
Write-Log "Branch:    $branch"
if (-not (Get-GitValue @('rev-parse', '--verify', '--quiet', "refs/heads/$BaseBranch"))) {
    Write-Log "There is no local $BaseBranch branch: it is the base of the GitHub repository." 'Red'
    Write-Log "Create it from the first commit with: git branch $BaseBranch <commit>" 'Red'
    Stop-Script 1
}
Write-Log "$BaseBranch`:      $(Get-GitValue @('log', '-1', '--format=%h %s', $BaseBranch))"
if ($branch -ne $BaseBranch) {
    Write-Log "$branch is $(Get-GitValue @('rev-list', '--count', "$BaseBranch..HEAD")) commit(s) ahead of $BaseBranch; both are pushed."
}
$changes = Get-GitChanges
if ($changes.Count -gt 0) {
    Write-Log "$($changes.Count) changed file(s) are not committed: they stay on this computer." 'Yellow'
    Write-Log 'To publish them as well, run tools\git_commit.bat first, then this again.' 'Yellow'
}

# 2. Remote.
Write-Header '2/4 Remote origin'
$origin = Get-GitValue @('remote', 'get-url', 'origin')
if ($origin) {
    $slug = Get-GitHubSlug $origin
    Write-Log "origin is already set: $origin"
    if ($slug -ne $Repo) {
        Write-Log "It does not point to ${Repo}: nothing pushed, to be safe." 'Red'
        Write-Log "If it should, run: git remote set-url origin https://github.com/$Repo.git" 'Red'
        Stop-Script 1
    }
} else {
    if (-not (Test-Tool 'gh')) {
        Write-Log 'The GitHub CLI (gh) was not found. Either install it (https://cli.github.com),' 'Red'
        Write-Log 'run "gh auth login" once and run this again, or create the empty repository' 'Red'
        Write-Log "$Repo on github.com (no README, no licence) and run:" 'Red'
        Write-Log "   git remote add origin https://github.com/$Repo.git" 'Red'
        Write-Log 'then run this again.' 'Red'
        Stop-Script 1
    }
    Write-Log ("gh:        " + (@(& gh --version 2>$null) | Select-Object -First 1))
    $auth = Invoke-Tool gh @('auth', 'status')
    if ($auth.Code -ne 0) {
        Write-Log 'The GitHub CLI is not signed in: run "gh auth login" once (GitHub.com, HTTPS,' 'Red'
        Write-Log 'sign in with the browser), then run this again.' 'Red'
        Stop-Script 1
    }
    $view = Invoke-Tool gh @('repo', 'view', $Repo, '--json', 'url', '--jq', '.url')
    if ($view.Code -eq 0) {
        Write-Log "$Repo already exists on GitHub: linking it as origin."
        $add = Invoke-Tool git @('remote', 'add', 'origin', "https://github.com/$Repo.git")
        if ($add.Code -ne 0) { Stop-Script 1 }
    } else {
        Write-Log "(the error above only means that $Repo does not exist yet)" 'DarkGray'
        Write-Log "Creating $Repo."
        $create = Invoke-Tool gh @('repo', 'create', $Repo, $Visibility, '--source', '.', '--remote', 'origin', '--description', $Description)
        if ($create.Code -ne 0) {
            Write-Log 'gh repo create failed (see above): nothing pushed.' 'Red'
            Stop-Script 1
        }
    }
    $origin = Get-GitValue @('remote', 'get-url', 'origin')
    if (-not $origin) {
        Write-Log 'origin is still not set (see above): nothing pushed.' 'Red'
        Stop-Script 1
    }
    Write-Log "origin:    $origin" 'Green'
}

# 3. Push: main first, then the current branch, then the tags. Never forced.
Write-Header '3/4 Push'
$push = Invoke-Tool git @('push', '-u', 'origin', $BaseBranch)
if ($push.Code -ne 0) {
    Write-Log "Pushing $BaseBranch failed (see above). If GitHub rejected it, its $BaseBranch has" 'Red'
    Write-Log "commits this computer lacks: get them with ""git switch $BaseBranch"" and ""git pull""," 'Red'
    Write-Log 'then switch back and run this again. Nothing is ever force-pushed.' 'Red'
    Stop-Script 1
}
if ($branch -ne $BaseBranch) {
    Write-Log ''
    $push = Invoke-Tool git @('push', '-u', 'origin', $branch)
    if ($push.Code -ne 0) {
        Write-Log "Pushing $branch failed (see above); $BaseBranch is published. Pull with ""git pull""" 'Red'
        Write-Log 'if GitHub has newer commits on it, then use tools\git_push.bat.' 'Red'
        Stop-Script 1
    }
}
$tags = @(& git tag --list 2>$null)
Write-Log ''
if ($tags.Count -eq 0) {
    Write-Log 'No tags to push.'
} else {
    Write-Log "Tags: $($tags -join ', ') (a new vX.Y.Z tag starts the release workflow on GitHub)"
    $push = Invoke-Tool git @('push', 'origin', '--tags')
    if ($push.Code -ne 0) {
        Write-Log 'Pushing the tags failed (see above); the branches are published.' 'Red'
        Stop-Script 1
    }
}

# 4. Result.
Write-Header '4/4 Result'
Invoke-Tool git @('remote', '-v') | Out-Null
Invoke-Tool git @('branch', '-vv', '--no-color') | Out-Null
Write-Log ''
Write-Log "Published: https://github.com/$Repo" 'Green'
Write-Log '-> Daily work: tools\git_commit.bat, then tools\git_push.bat (-PullRequest opens the pull request).' 'Yellow'
Write-Log "-> On GitHub, Settings > Branches: protect $BaseBranch and require the CI check before merging." 'Yellow'
Stop-Script 0
