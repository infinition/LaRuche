@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - lancement du client CLI (Terminal)
REM ============================================================
cd /d "%~dp0laruche"

echo.
echo === Build de laruche-cli ===
cargo build -p laruche-cli
if errorlevel 1 (
    echo.
    echo !! Echec du build. Voir les erreurs ci-dessus.
    pause
    exit /b 1
)

echo.
echo === Demarrage du client CLI ===
cargo run -p laruche-cli -- %*

endlocal
