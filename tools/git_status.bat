@echo off
rem Shows the state of the git repository and of GitHub, on screen and in
rem tools\git_status.log: branch, commits to push or pull, remotes, changed
rem files by group, last 10 commits, stashes, GitHub CLI sign-in, and a hint
rem for the next step. Changes nothing. The last line of the log is
rem "STATUS DONE <code>". Options are passed to tools\git_status.ps1:
rem   -NoPause   never wait for Enter at the end (started by a double-click,
rem              the window otherwise waits so the summary stays readable)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0git_status.ps1" %*
exit /b %errorlevel%
