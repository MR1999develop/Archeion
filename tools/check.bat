@echo off
rem Formats, lints and tests the workspace, showing every step on screen
rem and in tools\check.log. The last line of the log is "CHECK DONE <code>":
rem 0 means everything passed. Options are passed to tools\check.ps1:
rem   -NoPause   never wait for Enter at the end (started by a double-click,
rem              the window otherwise waits so the summary stays readable)
rem   -Verbose   also run cargo with --verbose (every rustc command line)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0check.ps1" %*
exit /b %errorlevel%
