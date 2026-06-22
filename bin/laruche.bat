@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - DEV CLI
REM ============================================================
cd /d "%~dp0..\laruche"

REM --- Configuration du client CLI ---
REM Port de l'API de laruche-node (par defaut 8419)
REM set "LARUCHE_PORT=8419"

REM URL complete de l'API (si le noeud est sur une autre machine)
REM set "LARUCHE_URL=http://127.0.0.1:8419"

REM Modele par defaut utilise par le CLI
REM set "LARUCHE_MODEL=gemma4:e4b"

REM --- Lancement du client ---
cargo run -q -p laruche-cli --bin laruche -- %*
endlocal
