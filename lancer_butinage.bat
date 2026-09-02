@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - le noeud seul, dans un terminal
REM
REM  Pas de coque de bureau: le noeud sert la SPA, et ce script
REM  ouvre le navigateur des qu'il repond. Utilise lancer_bureau.bat
REM  pour la meme interface dans sa propre fenetre.
REM
REM  RUCHE_MOTEUR=butinage plus bas est desormais REDONDANT: butinage
REM  est le moteur par defaut, et l'ancien (brain) est deprecie. La
REM  ligne reste explicite, elle ne coute rien et elle documente.
REM ============================================================
cd /d "%~dp0laruche"

REM --- Moteur agentique : butinage (nouveau) au lieu de l'ancien brain.rs ---
set "RUCHE_MOTEUR=butinage"

REM --- Qui ouvre la page: CE script, et lui seul ---
REM   Le noeud ouvre le navigateur tout seul au demarrage, et la sonde plus bas
REM   le fait aussi: deux onglets a chaque lancement. On coupe celui du noeud et
REM   on garde la sonde, qui attend que le serveur reponde vraiment. C'est ce que
REM   fait deja la coque bureau quand elle demarre le noeud.
set "LARUCHE_NO_BROWSER=1"

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

REM --- Skills livres avec le depot: deposer ceux qui MANQUENT dans le foyer ---
REM   Le noeud lit `skills/` dans son FOYER (%APPDATA%\LaRuche par defaut, ou
REM   LARUCHE_DATA_DIR), pas dans le depot. Un skill ajoute ici restait donc
REM   invisible jusqu'a ce qu'on pense a le copier a la main, ce que personne ne
REM   fait: on cherche pendant une heure pourquoi l'agent ignore une procedure
REM   qu'on vient d'ecrire.
REM   On depose ce qui manque et ce qui a ete corrige depuis, sans jamais ecraser
REM   un skill edite sur place: xcopy /d compare les dates et tranche seul.
set "FOYER=%LARUCHE_DATA_DIR%"
if not defined FOYER set "FOYER=%APPDATA%\LaRuche"
if not exist "%FOYER%\skills" mkdir "%FOYER%\skills" >nul 2>&1
for /d %%S in ("skills\*") do (
    if exist "%%S\SKILL.md" (
        REM  /d ne recopie QUE si la version du depot est plus recente que celle du
        REM  foyer. Un skill edite sur place, ou cree par le curateur, porte une date
        REM  posterieure et survit donc; une version livree corrigee, elle, arrive
        REM  enfin. Sans le /d, web-research est reste des semaines sans mentionner
        REM  web_discover chez un utilisateur alors que le depot l'expliquait.
        xcopy /d /e /i /q /y "%%S" "%FOYER%\skills\%%~nxS\" >nul
    )
)

echo === Build de laruche-node ===
REM  --release, comme tous les autres lanceurs. En debug, l'agent tourne des fois
REM  plus lentement pour rien, et le seul target\debug de ce depot pesait 38 Go.
REM  Pour deboguer avec les symboles, retire --release ici ET plus bas.
cargo build --release -p laruche-node
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
target\release\laruche-node.exe

endlocal
