@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - lancement avec le NOUVEAU moteur ReAct "butinage"
REM  (identique a lancer.bat + RUCHE_MOTEUR=butinage)
REM  Pour revenir a l'ancien moteur : utilise lancer.bat
REM ============================================================
cd /d "%~dp0laruche"

REM --- Moteur agentique : butinage (nouveau) au lieu de l'ancien brain.rs ---
set "RUCHE_MOTEUR=butinage"

REM --- Backend memoire : native | sqlite | sidecar ---
set "LARUCHE_MEMOIRE_BACKEND=sqlite"

REM --- (optionnel) embeddings semantiques via Ollama ---
REM set "LARUCHE_EMBED_URL=http://localhost:11434"
REM set "LARUCHE_EMBED_MODEL=nomic-embed-text"

echo.
echo ============================================================
echo  MOTEUR ACTIF : BUTINAGE  (RUCHE_MOTEUR=%RUCHE_MOTEUR%)
echo  Memoire      : %LARUCHE_MEMOIRE_BACKEND%
echo ============================================================
echo.
echo IMPORTANT : ferme toute instance LaRuche deja ouverte (sinon le port
echo 8419 est pris et le .exe est verrouille au build).
echo.

echo === Build de laruche-node ===
cargo build -p laruche-node
if errorlevel 1 (
    echo.
    echo !! Echec du build. Voir les erreurs ci-dessus.
    pause
    exit /b 1
)

echo.
echo === Ouverture de l'UI : http://localhost:8419 ===
start "" "http://localhost:8419"

echo === Demarrage du serveur ^(moteur butinage, Ctrl+C pour arreter^) ===
cargo run -p laruche-node

endlocal
