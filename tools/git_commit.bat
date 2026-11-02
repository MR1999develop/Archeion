@echo off
rem Commits all the current work: refuses on main, runs tools\check.ps1 and
rem stops if it fails, stages everything (git add -A) and commits with the
rem message of tools\commit-message.txt (or -Message), adding the project's
rem trailers. Step by step on screen and in tools\git_commit.log; the last
rem line is "COMMIT DONE <code>": 0 means committed (or nothing to commit).
rem Options are passed to tools\git_commit.ps1:
rem   -Message "Subject"   the message, instead of tools\commit-message.txt
rem   -Branch feature/x    create (or switch to) this branch first, keeping
rem                        the changes; needed when you are on main
rem   -SkipCheck           do not run tools\check.ps1 (work in progress only)
rem   -NoPause             never wait for Enter at the end
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0git_commit.ps1" %*
exit /b %errorlevel%
