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

REM --- CURATEUR (auto-creation de skills/tools, OPT-IN, conservateur) ---
REM   Active/desactive depuis l'UI : Settings > General > "Curateur - Butinage".
REM   (Le reglage est persistant.) Cette variable d'env le FORCE en plus, si besoin :
REM set "RUCHE_CURATEUR=1"

REM --- RECHERCHE WEB : decommente UNE ligne pour une vraie API (gros boost qualite) ---
REM   Tavily (pense pour les agents, free 1000/mois) : https://tavily.com
REM set "LARUCHE_TAVILY_KEY=tvly-xxxxxxxxxxxxxxxx"
REM   Brave Search (free 2000/mois) : https://brave.com/search/api/
REM set "LARUCHE_BRAVE_KEY=BSA-xxxxxxxxxxxxxxxx"
REM   ou un SearXNG auto-heberge :
REM set "LARUCHE_SEARXNG_URL=http://localhost:8888"
REM   Sans cle : scrapers gratuits (Yahoo+DDG) interroges en parallele et fusionnes.

REM --- Embeddings semantiques de la MEMOIRE (recall par sens, pas par mots) ---
REM   Par defaut le node tente Ollama local (nomic-embed-text, ~270 Mo :
REM   `ollama pull nomic-embed-text`). Serveur absent = disjoncteur, recall FTS5.
set "LARUCHE_EMBED_URL=http://localhost:11434"
set "LARUCHE_EMBED_MODEL=nomic-embed-text"
REM   Alternative llama.cpp : lance `lancer_embeddings.bat llamacpp` (port 8002,
REM   telechargement auto du GGUF nomic) puis :
REM set "LARUCHE_EMBED_URL=http://localhost:8002"

REM --- LLM via llama.cpp (tes .bat C:\DEV\_Local_AI\*, port 8001) ---
REM   Settings > Providers : provider "llamacpp" (base par defaut
REM   http://127.0.0.1:8001, pas de cle) ou provider "openai" + api_base.

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
echo === L'UI s'ouvrira automatiquement des que le serveur repond ===
REM Ouvre le navigateur SEULEMENT quand le node ecoute : sinon la page se charge
REM pendant le demarrage et le service worker sert un shell en cache (potentiellement
REM perime) avec des appels API qui echouent. Sonde en fenetre reduite, 5 min max.
start "" /min powershell -NoProfile -Command "$ok=$false; for($i=0;$i -lt 600;$i++){ try{ $null=Invoke-WebRequest -UseBasicParsing -TimeoutSec 1 'http://127.0.0.1:8419/'; $ok=$true; break } catch { Start-Sleep -Milliseconds 500 } }; if($ok){ Start-Process 'http://localhost:8419' }"

echo === Demarrage du serveur ^(moteur butinage, Ctrl+C pour arreter^) ===
cargo run -p laruche-node

endlocal
