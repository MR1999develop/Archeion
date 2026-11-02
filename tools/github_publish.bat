@echo off
rem Creates the private GitHub repository MR1999develop/archeion (or links it
rem if it exists) and pushes main, the current branch and the tags. Needs
rem tools\git_init.bat first, and the GitHub CLI (https://cli.github.com)
rem signed in once with "gh auth login" unless origin is already set.
rem Safe to run again. Step by step on screen and in tools\github_publish.log;
rem the last line is "GITHUB PUBLISH DONE <code>". Options are passed to
rem tools\github_publish.ps1:
rem   -NoPause   never wait for Enter at the end
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0github_publish.ps1" %*
exit /b %errorlevel%
