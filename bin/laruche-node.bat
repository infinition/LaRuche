@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - DEV NODE
REM ============================================================
cd /d "%~dp0..\laruche"
set "LARUCHE_MEMOIRE_BACKEND=sqlite"
cargo run -q -p laruche-node -- %*
endlocal
