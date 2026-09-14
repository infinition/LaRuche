@echo off
setlocal
pushd "%~dp0..\..\apps-library"
if errorlevel 1 exit /b 1

py -3 -c "import sys; sys.exit(sys.version_info < (3, 9))" >nul 2>&1
if not errorlevel 1 (
    set "PYTHON_APPS=py -3"
    goto compiler
)
python -c "import sys; sys.exit(sys.version_info < (3, 9))" >nul 2>&1
if not errorlevel 1 (
    set "PYTHON_APPS=python"
    goto compiler
)
echo Python 3.9 ou plus recent est introuvable. Installe Python puis relance.
goto erreur

:compiler
for /d %%D in (*) do (
    if exist "%%D\build.py" (
        echo Compilation : %%D
        %PYTHON_APPS% "%%D\build.py"
        if errorlevel 1 goto erreur
    )
)
%PYTHON_APPS% check_dist.py
if errorlevel 1 goto erreur
echo.
echo Apps compilees dans apps-library/APP/dist/.
echo Dans LaRuche, ouvre Apps puis Installer et selectionne une archive .laruche-app.
popd
pause
exit /b 0

:erreur
echo.
echo La compilation a echoue. Voir les erreurs ci-dessus.
popd
pause
exit /b 1
