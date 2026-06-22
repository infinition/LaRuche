@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - DEV NODE
REM ============================================================
cd /d "%~dp0..\laruche"

REM --- Configuration du Node ---
REM Backend memoire : native | sqlite | sidecar
set "LARUCHE_MEMOIRE_BACKEND=sqlite"

REM (Optionnel) Embeddings semantiques via Ollama
REM Decommente si Ollama tourne en local avec le modele nomic-embed-text :
REM set "LARUCHE_EMBED_URL=http://localhost:11434"
REM set "LARUCHE_EMBED_MODEL=nomic-embed-text"

REM --- Demarrage du noeud serveur ---
cargo run -q -p laruche-node -- %*
endlocal
