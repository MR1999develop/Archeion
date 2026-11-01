@echo off
rem Optimised build of the executable. Output: tools\release.log
cd /d "%~dp0.."
set CARGO_TERM_COLOR=never
cargo build --release -p archeion > tools\release.log 2>&1
echo RELEASE DONE %errorlevel% >> tools\release.log
