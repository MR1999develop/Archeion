# Optimised build of archeion.exe.
# Every step is shown on screen and written to tools\release.log: the
# toolchain and version, a check that no archeion.exe is running (Windows
# locks a running executable), the cargo output as it comes, and at the end
# the executable's path, size, date and SHA-256.
# The last line of the log is "RELEASE DONE <code>": 0 means it was built.
[CmdletBinding()]
param(
    [switch]$NoPause
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$log = Join-Path $PSScriptRoot 'release.log'
Set-Content -Path $log -Value '' -Encoding utf8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:CARGO_TERM_COLOR = 'never'
$cargoVerbose = if ($VerbosePreference -eq 'Continue') { '--verbose' } else { '' }
$exe = Join-Path $root 'target\release\archeion.exe'

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

# Wait for Enter only when started by a double-click (Explorer -> cmd -> here).
function Test-StartedFromExplorer {
    try {
        $me = Get-CimInstance Win32_Process -Filter "ProcessId=$PID"
        $cmd = Get-CimInstance Win32_Process -Filter "ProcessId=$($me.ParentProcessId)"
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$($cmd.ParentProcessId)"
        return $parent.Name -eq 'explorer.exe'
    } catch { return $false }
}

function Stop-Script([int]$code) {
    Write-Log "RELEASE DONE $code"
    if (-not $NoPause -and (Test-StartedFromExplorer)) {
        Write-Host ''
        Read-Host 'Press Enter to close'
    }
    exit $code
}

$started = Get-Date
Write-Header "Archeion release build - $($started.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Log "Folder:    $root"
Write-Log "Computer:  $env:COMPUTERNAME"
foreach ($tool in 'rustc', 'cargo') {
    Write-Log ("{0,-10} {1}" -f "$tool`:", ((& $tool --version 2>&1) -join ' '))
}
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches.Groups[1].Value
Write-Log "Version:   $version"
if (Test-Path .git) {
    $head = (git log --oneline -1 2>&1) -join ' '
    $branch = (git branch --show-current 2>&1) -join ' '
    $dirty = @(git status --porcelain 2>$null).Count
    Write-Log "Git:       $branch, $head ($dirty changed files)"
} else {
    Write-Log 'Git:       no repository yet'
}

# 1. A running archeion.exe cannot be replaced.
Write-Header '1/3 Running copies'
$running = @(Get-Process -Name archeion -ErrorAction SilentlyContinue | Where-Object {
    try { $_.Path -eq $exe } catch { $false }
})
if ($running.Count -gt 0) {
    Write-Log "archeion.exe from this folder is running (process $($running.Id -join ', ')):" 'Red'
    Write-Log 'close Archeion (and wait for any backup to end), then run this again.' 'Red'
    Stop-Script 1
}
Write-Log 'No copy of target\release\archeion.exe is running.' 'Green'
$before = if (Test-Path $exe) { (Get-Item $exe).LastWriteTime } else { $null }

# 2. Build, showing cargo's output as it comes. Local folders (this one and
# cargo's) are rewritten in the paths the compiler embeds, and the linker
# names the .pdb without its folder, so the executable names no user or
# folder of this PC.
Write-Header '2/3 cargo build --release'
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
$remaps = @("--remap-path-prefix=$root=archeion", "--remap-path-prefix=$cargoHome=cargo",
    '-Clink-arg=/PDBALTPATH:%_PDB%')
$env:CARGO_ENCODED_RUSTFLAGS = $remaps -join [char]0x1f
Write-Log "Paths:     $root -> archeion, $cargoHome -> cargo"
$command = "cargo build --release --locked -p archeion $cargoVerbose".Trim()
Write-Log "> $command" 'DarkGray'
$watch = [System.Diagnostics.Stopwatch]::StartNew()
$warnings = 0
cmd /c "$command 2>&1" | ForEach-Object {
    $line = "$_"
    $color = ''
    if ($line -match '^error') { $color = 'Red' }
    elseif ($line -match '^warning') { $color = 'Yellow'; $warnings++ }
    elseif ($line -match '^\s+Finished') { $color = 'Green' }
    Write-Log $line $color
}
$code = $LASTEXITCODE
$watch.Stop()
$seconds = [math]::Round($watch.Elapsed.TotalSeconds, 1)
if ($code -ne 0) {
    Write-Log "Build FAILED (exit code $code) in $seconds s." 'Red'
    Stop-Script 1
}
Write-Log "Build OK in $seconds s, $warnings warning(s)." 'Green'

# 3. What was built.
Write-Header '3/3 Executable'
if (-not (Test-Path $exe)) {
    Write-Log "Not found: $exe" 'Red'
    Stop-Script 1
}
$item = Get-Item $exe
$hash = (Get-FileHash -Algorithm SHA256 $exe).Hash.ToLowerInvariant()
$sizeMb = [math]::Round($item.Length / 1MB, 1)
Write-Log "Path:      $exe"
Write-Log "Size:      $sizeMb MB ($($item.Length) bytes)"
Write-Log "Built:     $($item.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))"
Write-Log "SHA-256:   $hash"
if ($before -and $item.LastWriteTime -eq $before) {
    Write-Log 'Unchanged since the last build: nothing to recompile.' 'Yellow'
}
# Local folders must not appear in the executable.
$text = [System.Text.Encoding]::GetEncoding(28591).GetString([System.IO.File]::ReadAllBytes($exe))
$found = @($root, $HOME, $cargoHome) | Where-Object {
    $text.IndexOf($_, [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
    $text.IndexOf($_.Replace('\', '/'), [StringComparison]::OrdinalIgnoreCase) -ge 0
}
if ($found) {
    Write-Log "Local paths found in the executable: $($found -join ', ')" 'Red'
    Stop-Script 1
}
Write-Log 'No local path in the executable.' 'Green'
$total = [math]::Round(((Get-Date) - $started).TotalSeconds, 1)
Write-Log ''
Write-Log "Total time: $total s" 
Write-Log 'Release built.' 'Green'
Stop-Script 0
