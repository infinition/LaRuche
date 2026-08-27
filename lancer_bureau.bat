@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - APPLICATION DE BUREAU
REM
REM  La meme interface web, dans sa propre fenetre. La coque ne
REM  contient aucun front: elle ouvre une fenetre sur le noeud
REM  local, qui sert deja la SPA. Rien n'est duplique.
REM
REM  Difference avec lancer_butinage.bat: c'est la coque qui
REM  demarre le noeud, et qui l'arrete en se fermant. Si un
REM  noeud tourne deja (ce .bat-la, ou le service Windows), elle
REM  s'y raccroche et le laisse vivre apres elle.
REM ============================================================
cd /d "%~dp0laruche"

REM --- Moteur agentique : butinage ---
set "RUCHE_MOTEUR=butinage"

REM --- Backend memoire ---
REM   Plus besoin de le poser: SQLite est desormais le defaut. La ligne reste
REM   commentee pour memoire, et pour le jour ou tu voudrais du volatile:
REM set "LARUCHE_MEMOIRE_BACKEND=memory"

REM --- Foyer de la ruche (memoire, sessions, skills, secrets) ---
REM   Par defaut: %%APPDATA%%\LaRuche, le meme quel que soit le lanceur.
REM   Decommente pour travailler SUR LaRuche: l'agent voit alors le code source
REM   dans son repertoire de travail, ce qui n'est pas le cas depuis AppData.
REM set "LARUCHE_DATA_DIR=%~dp0laruche"

REM --- Viser une ruche du reseau au lieu de la machine locale ---
REM   Avec cette variable, la coque n'en demarre aucune et se connecte a celle-la.
REM   La ruche visee doit avoir ete lancee avec LARUCHE_BIND_LAN=1, sinon elle
REM   s'annonce sur le reseau sans y repondre.
REM set "LARUCHE_URL=http://192.168.1.20:8419"

REM --- Recherche web : decommente UNE ligne pour une vraie API ---
REM set "LARUCHE_TAVILY_KEY=tvly-xxxxxxxxxxxxxxxx"
REM set "LARUCHE_BRAVE_KEY=BSA-xxxxxxxxxxxxxxxx"
REM set "LARUCHE_SEARXNG_URL=http://localhost:8888"

REM --- Embeddings semantiques de la memoire (recall par sens) ---
set "LARUCHE_EMBED_URL=http://localhost:11434"
set "LARUCHE_EMBED_MODEL=nomic-embed-text"

echo.
echo ============================================================
echo  LARUCHE - APPLICATION DE BUREAU
echo  Moteur  : %RUCHE_MOTEUR%
if defined LARUCHE_DATA_DIR (
    echo  Foyer   : %LARUCHE_DATA_DIR%
) else (
    echo  Foyer   : %APPDATA%\LaRuche
)
if defined LARUCHE_URL (
    echo  Ruche   : %LARUCHE_URL%  ^(distante, aucun noeud local demarre^)
) else (
    echo  Ruche   : locale, demarree par la fenetre
)
echo ============================================================
echo.
echo IMPORTANT : ferme toute fenetre LaRuche deja ouverte, sinon le .exe
echo est verrouille au build.
echo.

echo === Build du noeud et de la coque ===
REM --- Skills livres avec le depot: deposer ceux qui MANQUENT dans le foyer ---
REM   Le noeud lit `skills/` dans son FOYER, pas dans le depot: sans ce depot
REM   automatique, un skill ajoute au depot reste invisible pour l'agent.
REM   On ne remplace jamais un skill deja present: une edition sur place, ou un
REM   skill cree par le curateur, doit survivre.
set "FOYER_SKILLS=%LARUCHE_DATA_DIR%"
if not defined FOYER_SKILLS set "FOYER_SKILLS=%APPDATA%\LaRuche"
if not exist "%FOYER_SKILLS%\skills" mkdir "%FOYER_SKILLS%\skills" >nul 2>&1
for /d %%S in ("skills\*") do (
    if not exist "%FOYER_SKILLS%\skills\%%~nxS\SKILL.md" (
        if exist "%%S\SKILL.md" (
            echo   + skill depose dans le foyer : %%~nxS
            xcopy /e /i /q /y "%%S" "%FOYER_SKILLS%\skills\%%~nxS\" >nul
        )
    )
)

cargo build --release -p laruche-node -p laruche-bureau
if errorlevel 1 (
    echo.
    echo !! Echec du build. Voir les erreurs ci-dessus.
    pause
    exit /b 1
)

echo.
echo === Ouverture de la fenetre ===
REM Pas de sonde ni de navigateur a lancer ici: la coque attend elle-meme une vraie
REM reponse HTTP du noeud avant d'afficher la page. C'est ce qui evite d'ouvrir sur
REM une interface a moitie chargee, qui reclamait un F5.
target\release\laruche-bureau.exe

endlocal
