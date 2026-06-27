@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - Ajouter "bin" au PATH Windows
REM ============================================================
echo Ajout du dossier bin/ de LaRuche a votre PATH Windows...
set "BIN_PATH=%~dp0bin"

REM On utilise PowerShell pour verifier et ajouter le chemin proprement au PATH Utilisateur
powershell -NoProfile -Command "$p = [Environment]::GetEnvironmentVariable('Path', 'User'); if ($p -notmatch [regex]::Escape('%BIN_PATH%')) { [Environment]::SetEnvironmentVariable('Path', $p + ';%BIN_PATH%', 'User'); Write-Host '=> Dossier ajoute avec succes !' -ForegroundColor Green } else { Write-Host '=> Le dossier est deja dans le PATH.' -ForegroundColor Yellow }"

echo.
echo Termine. Veuillez fermer et rouvrir vos fenetres de terminal pour que les alias fonctionnent.
echo Vous pourrez ensuite taper "laruche" ou "laruche-node" depuis n'importe ou !
echo.
pause
endlocal
