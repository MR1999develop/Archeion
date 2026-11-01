@echo off
rem Writes the toolchain versions to tools\probe.log (read by the assistant).
cd /d "%~dp0"
(
  echo == rustc
  rustc -vV
  echo == cargo
  cargo -V
  echo == rustup
  rustup show
  echo == git
  git --version
  echo == rclone
  where rclone
  echo == cl
  where cl
  where link
) > probe.log 2>&1
