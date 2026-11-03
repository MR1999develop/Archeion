# Formats, lints and tests the Archeion workspace.
# Every step is shown on screen and written to tools\check.log, with the
# toolchain, the time each step took, the files rustfmt changed, every
# warning and failed test, and a summary at the end.
# The last line of the log is "CHECK DONE <code>": 0 means everything passed.
[CmdletBinding()]
param(
    [switch]$NoPause
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$log = Join-Path $PSScriptRoot 'check.log'
Set-Content -Path $log -Value '' -Encoding utf8
# cargo writes UTF-8: decode it as such (test names with accents).
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:CARGO_TERM_COLOR = 'never'
$cargoVerbose = if ($VerbosePreference -eq 'Continue') { @('--verbose') } else { @() }

function Write-Log([string]$text, [string]$color = '') {
    if ($color) { Write-Host $text -ForegroundColor $color } else { Write-Host $text }
    Add-Content -Path $log -Value $text -Encoding utf8
}

function Write-Header([string]$text) {
    Write-Log ''
    Write-Log ('=' * 78) 'Cyan'
    Write-Log "== $text" 'Cyan'
    Write-Log ('=' * 78) 'Cyan'
}

# Runs a command through cmd so stderr and stdout stay in order, shows each
# line as it comes and keeps them all. Returns exit code, lines and seconds.
function Invoke-Step([string]$command) {
    Write-Log "> $command" 'DarkGray'
    $lines = New-Object System.Collections.Generic.List[string]
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    cmd /c "$command 2>&1" | ForEach-Object {
        $line = "$_"
        $lines.Add($line)
        $color = ''
        if ($line -match '^(error|test .* FAILED|failures:)') { $color = 'Red' }
        elseif ($line -match '^warning') { $color = 'Yellow' }
        elseif ($line -match '^test result: ok') { $color = 'Green' }
        Write-Log $line $color
    }
    $code = $LASTEXITCODE
    $watch.Stop()
    [pscustomobject]@{ Code = $code; Lines = $lines; Seconds = [math]::Round($watch.Elapsed.TotalSeconds, 1) }
}

$results = New-Object System.Collections.Generic.List[object]
function Add-Result([string]$step, $run, [string]$detail) {
    $ok = $run.Code -eq 0
    $results.Add([pscustomobject]@{ Step = $step; Ok = $ok; Seconds = $run.Seconds; Detail = $detail })
    $status = if ($ok) { 'OK' } else { "FAILED (exit code $($run.Code))" }
    Write-Log "-- $step`: $status in $($run.Seconds) s $detail" $(if ($ok) { 'Green' } else { 'Red' })
}

$started = Get-Date
Write-Header "Archeion check - $($started.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Log "Folder:    $root"
Write-Log "Computer:  $env:COMPUTERNAME"
foreach ($tool in 'rustc', 'cargo') {
    $version = (& $tool --version 2>&1) -join ' '
    Write-Log ("{0,-10} {1}" -f "$tool`:", $version)
}
if (Test-Path .git) {
    $head = (git log --oneline -1 2>&1) -join ' '
    $dirty = @(git status --porcelain 2>$null).Count
    Write-Log "Git:       $head ($dirty changed files)"
} else {
    Write-Log 'Git:       no repository yet'
}

# 1. Formatting: list what rustfmt would change, then format.
Write-Header '1/4 cargo fmt'
$fmtCheck = Invoke-Step 'cargo fmt --all -- --check -l'
$changed = @($fmtCheck.Lines | Where-Object { $_ -match '\.rs$' })
if ($changed.Count -gt 0) {
    Write-Log "rustfmt reformats $($changed.Count) file(s):" 'Yellow'
    $changed | ForEach-Object { Write-Log "   $_" 'Yellow' }
} else {
    Write-Log 'All files were already formatted.'
}
$fmt = Invoke-Step 'cargo fmt --all'
Add-Result 'fmt' $fmt "($($changed.Count) file(s) reformatted)"

# 2. Lints: warnings are errors.
Write-Header '2/4 cargo clippy'
$clippy = Invoke-Step "cargo clippy --workspace --all-targets $($cargoVerbose -join ' ') -- -D warnings"
$problems = @($clippy.Lines | Where-Object { $_ -match '^(warning|error)(\[|:)' -and $_ -notmatch 'generated \d+ warning' })
Add-Result 'clippy' $clippy "($($problems.Count) warning(s) or error(s))"

# 3. Tests: every test name is printed by cargo; failures summed up below.
Write-Header '3/4 cargo test'
$test = Invoke-Step "cargo test --workspace --no-fail-fast $($cargoVerbose -join ' ')"
$passed = 0; $failed = 0; $ignored = 0
foreach ($line in $test.Lines) {
    if ($line -match '^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored') {
        $passed += [int]$Matches[1]; $failed += [int]$Matches[2]; $ignored += [int]$Matches[3]
    }
}
$failedTests = @($test.Lines | Where-Object { $_ -match '^test (.+) \.\.\. FAILED$' } | ForEach-Object { $_ -replace '^test (.+) \.\.\. FAILED$', '$1' })
Add-Result 'test' $test "($passed passed, $failed failed, $ignored ignored)"

# 4. Dependencies: known vulnerabilities, licences, sources (deny.toml).
# Skipped, and said, when cargo-deny is not installed.
Write-Header '4/4 cargo deny'
if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    $deny = Invoke-Step 'cargo deny check'
    Add-Result 'deny' $deny '(advisories, licences, sources)'
} else {
    Write-Log 'cargo-deny is not installed (cargo install cargo-deny): skipped; CI runs it.' 'Yellow'
}

# Summary.
Write-Header 'Summary'
foreach ($r in $results) {
    $mark = if ($r.Ok) { 'OK    ' } else { 'FAILED' }
    Write-Log ("{0}  {1,-7} {2,7} s  {3}" -f $mark, $r.Step, $r.Seconds, $r.Detail) $(if ($r.Ok) { 'Green' } else { 'Red' })
}
if ($problems.Count -gt 0) {
    Write-Log ''
    Write-Log 'Clippy:' 'Yellow'
    $problems | Select-Object -Unique | ForEach-Object { Write-Log "   $_" 'Yellow' }
}
if ($failedTests.Count -gt 0) {
    Write-Log ''
    Write-Log 'Failed tests:' 'Red'
    $failedTests | ForEach-Object { Write-Log "   $_" 'Red' }
}
$total = [math]::Round(((Get-Date) - $started).TotalSeconds, 1)
$exit = if (@($results | Where-Object { -not $_.Ok }).Count -eq 0) { 0 } else { 1 }
Write-Log ''
Write-Log "Total time: $total s" 
if ($exit -eq 0) { Write-Log 'Everything passed.' 'Green' } else { Write-Log 'Something failed: see above.' 'Red' }
Write-Log "CHECK DONE $exit"

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
if (-not $NoPause -and (Test-StartedFromExplorer)) {
    Write-Host ''
    Read-Host 'Press Enter to close'
}
exit $exit
