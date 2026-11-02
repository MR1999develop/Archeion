# Shows the state of the git repository and of the link with GitHub, on
# screen and in tools\git_status.log: branch, upstream with commits ahead
# and behind, remotes, changed and untracked files by group, the last 10
# commits, branches, stashes, the GitHub CLI and its sign-in, and a hint for
# the next step.
# It changes nothing: not even "git fetch", so the "behind" count is as of
# the last fetch, pull or push.
# The last line of the log is "STATUS DONE <code>": 0 means it ran.
[CmdletBinding()]
param(
    [switch]$NoPause
)

$LogName = 'git_status'
$DoneLabel = 'STATUS'
. (Join-Path $PSScriptRoot 'git_common.ps1')

$hints = New-Object System.Collections.Generic.List[string]
Write-Start 'Archeion git status'
$hasGh = Test-Tool 'gh'
if ($hasGh) {
    $ghVersion = @(& gh --version 2>$null)
    Write-Log ("gh:        " + $(if ($ghVersion.Count -gt 0) { $ghVersion[0] } else { '?' }))
} else {
    Write-Log 'gh:        not installed (optional, needed to create the GitHub repository and pull requests: https://cli.github.com)'
}
$userName = Get-GitValue @('config', 'user.name')
$userEmail = Get-GitValue @('config', 'user.email')
Write-Log ("Author:    " + $(if ($userName -or $userEmail) { "$userName <$userEmail>" } else { 'not configured (tools\git_commit.bat sets it for this repository)' }))
$autocrlf = Get-GitValue @('config', 'core.autocrlf')
Write-Log ("autocrlf:  " + $(if ($autocrlf) { $autocrlf } else { 'not set (.gitattributes decides the line endings)' }))

# 1. Branch and upstream.
Write-Header '1/6 Branch'
$branch = Get-GitValue @('branch', '--show-current')
$hasCommits = [bool](Get-GitValue @('rev-parse', '--verify', '--quiet', 'HEAD'))
$ahead = 0
$behind = 0
$upstream = ''
$operation = Get-OperationInProgress
if (-not $hasCommits) {
    Write-Log "Branch:    $branch (no commits yet)" 'Yellow'
    $hints.Add('No commit yet: run tools\git_init.bat to make the first one.')
} elseif (-not $branch) {
    Write-Log "Branch:    none - detached HEAD at $(Get-GitValue @('log', '-1', '--format=%h %s'))" 'Yellow'
    $hints.Add('You are not on a branch: go back to one with "git switch main" (or your feature branch).')
} else {
    Write-Log "Branch:    $branch"
    Write-Log "HEAD:      $(Get-GitValue @('log', '-1', '--date=format:%Y-%m-%d %H:%M', '--format=%h %ad %s'))"
    $upstream = Get-GitValue @('rev-parse', '--abbrev-ref', '--symbolic-full-name', '@{u}')
    if ($upstream) {
        $counts = (Get-GitValue @('rev-list', '--left-right', '--count', '@{u}...HEAD')) -split '\s+'
        if ($counts.Count -ge 2) { $behind = [int]$counts[0]; $ahead = [int]$counts[1] }
        $color = if ($ahead -eq 0 -and $behind -eq 0) { 'Green' } else { 'Yellow' }
        Write-Log "Upstream:  $upstream - $ahead commit(s) to push, $behind commit(s) to pull" $color
        $fetchHead = Join-Path (Get-GitValue @('rev-parse', '--absolute-git-dir')) 'FETCH_HEAD'
        if (Test-Path -LiteralPath $fetchHead) {
            Write-Log "           (as of the last fetch: $((Get-Item -LiteralPath $fetchHead).LastWriteTime.ToString('yyyy-MM-dd HH:mm')))"
        } else {
            Write-Log '           (no fetch yet: the counts are as of the last push)'
        }
    } else {
        Write-Log 'Upstream:  none (the branch has never been pushed)' 'Yellow'
    }
    if ($ProtectedBranches -notcontains $branch -and (Get-GitValue @('rev-parse', '--verify', '--quiet', 'refs/heads/main'))) {
        $notInMain = Get-GitValue @('rev-list', '--count', 'main..HEAD')
        $notHere = Get-GitValue @('rev-list', '--count', 'HEAD..main')
        Write-Log "vs main:   $notInMain commit(s) not in main yet, main has $notHere commit(s) not in this branch"
    }
}
if ($operation) {
    Write-Log "In progress: a $operation" 'Red'
    $hints.Add("Finish or cancel the $operation before anything else.")
}

# 2. Remotes.
Write-Header '2/6 Remotes'
$origin = Get-GitValue @('remote', 'get-url', 'origin')
$slug = Get-GitHubSlug $origin
$remotes = @(& git remote -v 2>$null)
if ($remotes.Count -eq 0) {
    Write-Log 'No remote: the repository is only on this computer.' 'Yellow'
} else {
    $remotes | ForEach-Object { Write-Log $_ }
    if ($slug) { Write-Log "On GitHub: https://github.com/$slug" }
}

# 3. Changes, by group.
Write-Header '3/6 Changes'
$words = @{ 'M' = 'modified'; 'A' = 'added'; 'D' = 'deleted'; 'R' = 'renamed'; 'C' = 'copied'; 'T' = 'type changed'; 'U' = 'conflict' }
$groups = [ordered]@{
    'Conflicts (to resolve)'           = New-Object System.Collections.Generic.List[string]
    'Staged (in the next commit)'      = New-Object System.Collections.Generic.List[string]
    'Not staged'                       = New-Object System.Collections.Generic.List[string]
    'Untracked (new, not yet in git)'  = New-Object System.Collections.Generic.List[string]
}
$keys = @($groups.Keys)
$changes = Get-GitChanges
foreach ($line in $changes) {
    if ($line.Length -lt 4) { continue }
    $x = "$($line[0])"
    $y = "$($line[1])"
    $path = $line.Substring(3)
    if ($x -eq '?') {
        $groups[$keys[3]].Add($path)
    } elseif ($x -eq 'U' -or $y -eq 'U' -or ($x -eq 'A' -and $y -eq 'A') -or ($x -eq 'D' -and $y -eq 'D')) {
        $groups[$keys[0]].Add($path)
    } else {
        if ($x -ne ' ') { $groups[$keys[1]].Add(("{0,-13} {1}" -f $words[$x], $path)) }
        if ($y -ne ' ') { $groups[$keys[2]].Add(("{0,-13} {1}" -f $words[$y], $path)) }
    }
}
if ($changes.Count -eq 0) {
    Write-Log 'Nothing changed since the last commit.' 'Green'
} else {
    foreach ($key in $keys) {
        $list = $groups[$key]
        if ($list.Count -eq 0) { continue }
        Write-Log "$key`: $($list.Count)" $(if ($key -eq $keys[0]) { 'Red' } else { 'Yellow' })
        $list | ForEach-Object { Write-Log "   $_" }
    }
}

# 4. History and branches.
Write-Header '4/6 Last 10 commits and branches'
if ($hasCommits) {
    & git log -10 --date=short '--format=%h %ad %s%d' 2>$null | ForEach-Object { Write-Log $_ }
    Write-Log ''
    & git branch -vv --no-color 2>$null | ForEach-Object { Write-Log $_ }
} else {
    Write-Log 'No commits yet.'
}

# 5. Stashes.
Write-Header '5/6 Stashes'
$stashes = @(& git stash list 2>$null)
if ($stashes.Count -eq 0) {
    Write-Log 'None.'
} else {
    $stashes | ForEach-Object { Write-Log $_ }
    $hints.Add("There are $($stashes.Count) stash(es): work put aside with ""git stash"". Get it back with ""git stash pop"" or drop it with ""git stash drop"".")
}

# 6. GitHub CLI.
Write-Header '6/6 GitHub CLI'
$ghReady = $false
$openPr = ''
if (-not $hasGh) {
    Write-Log 'Not installed. Without it tools\git_push.bat still pushes, but cannot open pull requests.'
} else {
    $auth = Invoke-Tool gh @('auth', 'status')
    $ghReady = $auth.Code -eq 0
    if (-not $ghReady) {
        Write-Log 'Not signed in: run "gh auth login" once (GitHub.com, HTTPS, browser).' 'Yellow'
    } elseif ($slug -and $branch -and $upstream -and $ProtectedBranches -notcontains $branch) {
        Write-Log ''
        Write-Log "> gh pr list --repo $slug --head $branch --state open --json url --jq .[].url" 'DarkGray'
        $openPr = (@(& gh pr list --repo $slug --head $branch --state open --json url --jq '.[].url' 2>$null) -join ' ').Trim()
        if ($LASTEXITCODE -ne 0) {
            Write-Log 'Could not ask GitHub for pull requests (offline?).' 'Yellow'
            $openPr = '?'
        } elseif ($openPr) {
            Write-Log "Open pull request for $branch`: $openPr" 'Green'
        } else {
            Write-Log "No open pull request for $branch."
        }
    }
}

# Next step.
Write-Header 'Next step'
if (-not $operation -and $hasCommits -and $branch) {
    if ($changes.Count -gt 0) {
        if ($ProtectedBranches -contains $branch) {
            $hints.Add("You have changes on $branch, which only takes pull requests: commit them on a new branch with" +
                "`n     tools\git_commit.bat -Branch feature/<short-name>" +
                "`n   (message in tools\commit-message.txt, or -Message ""Subject"").")
        } else {
            $hints.Add("Commit the changes: tools\git_commit.bat (message in tools\commit-message.txt, or -Message ""Subject"").")
        }
    }
    if (-not $origin) {
        $hints.Add('Publish on GitHub: tools\github_publish.bat (creates the private repository and pushes main and this branch).')
    } elseif (-not $upstream) {
        $hints.Add("Push the branch: tools\git_push.bat (the first push links $branch to origin/$branch).")
    } else {
        if ($behind -gt 0) { $hints.Add("GitHub has $behind commit(s) you do not have: get them with ""git pull"" before pushing.") }
        if ($ahead -gt 0) { $hints.Add("Push the $ahead new commit(s): tools\git_push.bat") }
        if ($ProtectedBranches -notcontains $branch -and $ghReady -and -not $openPr) {
            $hints.Add('Open a pull request towards main: tools\git_push.bat -PullRequest')
        }
    }
    if (-not $hasGh) {
        $hints.Add('Optional: install the GitHub CLI (https://cli.github.com), then "gh auth login".')
    } elseif (-not $ghReady) {
        $hints.Add('Sign in to the GitHub CLI once: gh auth login')
    }
}
if ($hints.Count -eq 0) {
    Write-Log 'Nothing to do: everything is committed and pushed.' 'Green'
} else {
    foreach ($hint in $hints) { Write-Log "-> $hint" 'Yellow' }
}
Stop-Script 0
