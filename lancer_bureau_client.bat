@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - APPLICATION DE BUREAU, MODE CLIENT
REM
REM  Ouvre la fenetre SANS demarrer de noeud local: elle cherche
REM  les ruches du reseau en mDNS et te laisse choisir. C'est le
REM  mode pour une machine qui consulte une ruche hebergee
REM  ailleurs (le PC du salon, un serveur, plus tard un telephone).
REM
REM  Une seule ruche trouvee: on y va directement.
REM  Plusieurs: un selecteur s'affiche.
REM  Aucune: la fenetre explique quoi faire.
REM
REM  A SAVOIR: la ruche visee doit avoir ete demarree avec
REM  LARUCHE_BIND_LAN=1. Sans ca elle s'annonce sur le reseau mais
REM  n'ecoute que sur elle-meme, et le selecteur l'affichera comme
REM  « injoignable ». Utilise decouvrir_ruches.bat pour verifier.
REM ============================================================
cd /d "%~dp0laruche"

REM C'est CETTE variable qui fait le mode client: la coque ne cherche meme pas de
REM noeud a demarrer et passe directement a la decouverte reseau. Une ruse de
REM repertoire ne suffirait pas - le chemin target\release est compile dans le
REM binaire, donc la coque retrouverait le noeud du depot depuis n'importe ou.
set "LARUCHE_SANS_NOEUD=1"

REM --- Adresse fixe, si tu ne veux pas passer par la decouverte ---
REM set "LARUCHE_URL=http://192.168.1.20:8419"

REM  On rebatit A CHAQUE FOIS, et non "seulement si l'exe est absent".
REM  La version paresseuse lancait une coque perimee des qu'on avait touche au
REM  code: le binaire existait, donc rien n'etait recompile, et on testait sans
REM  le savoir la version d'avant. Un build incremental sans changement ne coute
REM  qu'une poignee de secondes; ce mode ne demande que la coque, pas le noeud,
REM  puisqu'il n'en demarre aucun.
echo === Build de la coque ===
cargo build --release -p laruche-bureau
if errorlevel 1 (
    echo.
    echo !! Echec du build. Voir les erreurs ci-dessus.
    pause
    exit /b 1
)

echo.
echo ============================================================
echo  LARUCHE - MODE CLIENT  ^(aucun noeud local demarre^)
if defined LARUCHE_URL (
    echo  Ruche visee : %LARUCHE_URL%
) else (
    echo  Ruche visee : celle qu'on trouvera sur le reseau
)
echo ============================================================
echo.

target\release\laruche.exe

endlocal
