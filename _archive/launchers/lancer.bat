@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - lancement du serveur (Web UI + API + agent)
REM ============================================================
cd /d "%~dp0laruche"

REM --- Backend memoire : native | sqlite | sidecar ---
set "LARUCHE_MEMOIRE_BACKEND=sqlite"

REM --- (optionnel) embeddings semantiques via Ollama ---
REM   Decommente si Ollama tourne et a le modele nomic-embed-text :
REM set "LARUCHE_EMBED_URL=http://localhost:11434"
REM set "LARUCHE_EMBED_MODEL=nomic-embed-text"

echo.
echo === Build de laruche-node (backend memoire = %LARUCHE_MEMOIRE_BACKEND%) ===
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

echo === Demarrage du serveur (Ctrl+C pour arreter) ===
cargo run -p laruche-node

endlocal
