@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - DEV CLI
REM ============================================================
cd /d "%~dp0..\laruche"
cargo run -q -p laruche-cli --bin laruche -- %*
endlocal
