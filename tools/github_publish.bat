@echo off
rem Creates the GitHub repository MR1999develop/archeion and pushes main.
rem Needs tools\git_init.bat first, and the GitHub CLI (https://cli.github.com)
rem signed in once with "gh auth login". Output: tools\github_publish.log
rem Safe to run again: if the remote exists it only pushes.
cd /d "%~dp0.."
set REPO=MR1999develop/archeion
set VISIBILITY=--private
call :main > tools\github_publish.log 2>&1
echo GITHUB PUBLISH DONE %errorlevel% >> tools\github_publish.log
exit /b

:main
echo == %date% %time%
if not exist .git (
  echo No git repository yet: run tools\git_init.bat first.
  exit /b 1
)
git remote get-url origin >nul 2>&1 && goto push
where gh >nul 2>&1
if errorlevel 1 (
  echo GitHub CLI not found. Either install it and run "gh auth login",
  echo or create the empty repository %REPO% on github.com and run:
  echo   git remote add origin https://github.com/%REPO%.git
  echo then run this script again.
  exit /b 1
)
gh auth status || exit /b 1
echo == creating %REPO%
gh repo create %REPO% %VISIBILITY% --source . --remote origin --description "Incremental, scheduled, optionally encrypted backups for Windows (Rust + egui)" || exit /b 1

:push
echo == pushing
git push -u origin main || exit /b 1
git push origin --tags || exit /b 1
git remote -v
exit /b 0
