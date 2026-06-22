@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - Installation globale (PROD)
REM ============================================================
echo Ce script va compiler la version finale de laruche et l'installer dans votre systeme.
echo L'installation va utiliser le dossier ~/.cargo/bin (qui est dans votre PATH).
echo.
pause

cd /d "%~dp0..\laruche"

echo.
echo === Installation de laruche-node ===
cargo install --path laruche-node

echo.
echo === Installation de laruche-cli ===
cargo install --path laruche-cli

echo.
echo === Termine ! ===
echo Vous pouvez desormais lancer "laruche" et "laruche-node" depuis n'importe ou !
echo.
pause
endlocal
