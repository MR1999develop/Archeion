@echo off
rem Creates the git repository and makes the first commit.
rem Output: tools\git_init.log. Safe to run twice: it stops if .git exists.
cd /d "%~dp0.."
call :main > tools\git_init.log 2>&1
echo GIT INIT DONE %errorlevel% >> tools\git_init.log
exit /b

:main
echo == %date% %time%
if exist .git (
  echo A git repository already exists here: nothing done.
  exit /b 0
)
echo == removing obsolete files of the restic version
if exist app\src\secrets.rs del /q app\src\secrets.rs
if exist crates\engine\tests\local_repository.rs del /q crates\engine\tests\local_repository.rs
echo == git init
git init -b main || exit /b 1
git config user.email >nul 2>&1 || git config user.email "MR1999develop@users.noreply.github.com"
git config user.name >nul 2>&1 || git config user.name "MR"
echo == git add
git add -A || exit /b 1
git status --short
echo == git commit
git commit -q -F tools\first-commit.txt || exit /b 1
git log --oneline -1
exit /b 0
