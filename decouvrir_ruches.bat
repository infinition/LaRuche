@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - QUI EST SUR LE RESEAU ?
REM
REM  Liste les ruches reperees en mDNS, sans ouvrir de fenetre ni
REM  demarrer quoi que ce soit. Utile quand une ruche ne remonte
REM  pas et qu'on ne sait pas si le silence vient du pare-feu, du
REM  reseau, ou de la ruche elle-meme.
REM
REM  Chaque ruche est ensuite sondee en TCP. Une ruche « INJOIGNABLE »
REM  s'annonce sur le reseau mais n'ecoute que sur elle-meme: il faut
REM  la demarrer avec LARUCHE_BIND_LAN=1 pour qu'elle accepte les
REM  connexions venant d'ailleurs.
REM ============================================================
cd /d "%~dp0laruche"

set "LARUCHE_DECOUVRIR=1"

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
echo  RECHERCHE DES RUCHES SUR LE RESEAU LOCAL
echo  Ecoute mDNS pendant 3 secondes...
echo ============================================================
echo.

target\release\laruche.exe

echo.
pause
endlocal
