@echo off
rem Optimised build of archeion.exe, step by step on screen and in
rem tools\release.log. The last line of the log is "RELEASE DONE <code>":
rem 0 means the executable was built. Options are passed to tools\release.ps1:
rem   -NoPause   never wait for Enter at the end (started by a double-click,
rem              the window otherwise waits so the summary stays readable)
rem   -Verbose   also run cargo with --verbose (every rustc command line)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0release.ps1" %*
exit /b %errorlevel%
