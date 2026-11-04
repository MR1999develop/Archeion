# Parses every PowerShell script in tools with the PowerShell parser, without
# running any of them. A syntax error stops a script before its first line,
# so a script that is rarely run can carry one for a long time unnoticed.
# Prints "file:line: message" for each error; exit code 0 means none.
$errorCount = 0
$scripts = @(Get-ChildItem -Path $PSScriptRoot -Filter '*.ps1' | Sort-Object Name)
foreach ($script in $scripts) {
    $tokens = $null
    $problems = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($script.FullName, [ref]$tokens, [ref]$problems)
    foreach ($problem in $problems) {
        Write-Output ('{0}:{1}: {2}' -f $script.Name, $problem.Extent.StartLineNumber, $problem.Message)
        $errorCount++
    }
}
Write-Output "$($scripts.Count) script(s) parsed, $errorCount syntax error(s)."
if ($errorCount -gt 0) { exit 1 }
exit 0
