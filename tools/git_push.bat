@echo off
rem Pushes the current branch to GitHub (never forced), linking it to origin
rem the first time, and shows how to open the pull request. Step by step on
rem screen and in tools\git_push.log; the last line is "PUSH DONE <code>".
rem Options are passed to tools\git_push.ps1:
rem   -PullRequest   also open the pull request towards main with the
rem                  GitHub CLI (gh), if none is open yet
rem   -NoPause       never wait for Enter at the end
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0git_push.ps1" %*
exit /b %errorlevel%
