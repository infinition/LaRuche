@echo off
setlocal
TITLE LaRuche - Embeddings memoire (Ollama ou llama.cpp)
REM ============================================================
REM  Lance le serveur d'EMBEDDINGS de la memoire semantique.
REM  Usage :
REM    lancer_embeddings.bat            (auto : Ollama si installe, sinon llama.cpp)
REM    lancer_embeddings.bat ollama     (force Ollama, port 11434)
REM    lancer_embeddings.bat llamacpp   (force llama.cpp, port 8002)
REM  Le modele (nomic-embed-text, ~140-270 Mo) est TELECHARGE
REM  automatiquement s'il est absent.
REM ============================================================

set "MODE=%~1"
REM Dossier des binaires/modeles llama.cpp locaux (surchager via LOCAL_AI_DIR)
set "LOCAL_AI=%LOCAL_AI_DIR%"
if "%LOCAL_AI%"=="" set "LOCAL_AI=C:\DEV\_Local_AI"

if /i "%MODE%"=="ollama"   goto :ollama
if /i "%MODE%"=="llamacpp" goto :llamacpp
where ollama >nul 2>nul
if not errorlevel 1 goto :ollama
goto :llamacpp

REM ============================ OLLAMA ============================
:ollama
echo [MODE] Ollama - LARUCHE_EMBED_URL=http://localhost:11434 (defaut LaRuche, rien a changer)
curl -s -o nul --max-time 3 http://127.0.0.1:11434/api/tags
if not errorlevel 1 goto :ollama_up
echo [INFO] Ollama ne repond pas : demarrage...
start "" /min ollama serve
for /l %%i in (1,1,30) do (
    curl -s -o nul --max-time 2 http://127.0.0.1:11434/api/tags
    if not errorlevel 1 goto :ollama_up
    timeout /t 1 /nobreak >nul
)
echo [ERREUR] Ollama ne repond toujours pas apres 30 s.
pause
exit /b 1

:ollama_up
echo [OK] Serveur Ollama actif.
ollama list 2>nul | findstr /i "nomic-embed-text" >nul
if not errorlevel 1 goto :ollama_ready
echo [INFO] Modele nomic-embed-text absent : telechargement (~270 Mo)...
ollama pull nomic-embed-text
if errorlevel 1 (
    echo [ERREUR] Le pull du modele a echoue.
    pause
    exit /b 1
)

:ollama_ready
echo [TEST] Embedding de controle...
curl -s --max-time 30 -X POST http://127.0.0.1:11434/api/embed -H "Content-Type: application/json" -d "{\"model\":\"nomic-embed-text\",\"input\":\"test\"}" | findstr /i "embeddings" >nul
if errorlevel 1 (
    echo [ATTENTION] Le test d'embedding n'a pas renvoye de vecteur. Verifie `ollama list`.
) else (
    echo [OK] Embeddings semantiques PRETS. La memoire de LaRuche est branchee.
)
echo.
echo (Ollama tourne en tache de fond ; cette fenetre peut etre fermee.)
pause
exit /b 0

REM ============================ LLAMA.CPP ============================
:llamacpp
echo [MODE] llama.cpp - port 8002 (le chat reste sur 8001)
set "MODEL_DIR=%LOCAL_AI%\.models"
set "MODEL_PATH=%MODEL_DIR%\nomic-embed-text-v1.5.Q8_0.gguf"
set "MODEL_URL=https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q8_0.gguf"

REM Localiser llama-server.exe (dernier dossier llama-* en date)
set "LLAMA_EXE="
for /d %%d in ("%LOCAL_AI%\llama-*") do if exist "%%d\llama-server.exe" set "LLAMA_EXE=%%d\llama-server.exe"
if "%LLAMA_EXE%"=="" (
    echo [ERREUR] llama-server.exe introuvable sous %LOCAL_AI%\llama-*
    echo          Ajuste LOCAL_AI_DIR ou installe les binaires llama.cpp.
    pause
    exit /b 1
)
REM DLL CUDA au PATH si presentes (meme convention que tes .bat modeles)
for /d %%d in ("%LOCAL_AI%\cudart-*") do set "PATH=%%d;%PATH%"

if not exist "%MODEL_DIR%" mkdir "%MODEL_DIR%"
if not exist "%MODEL_PATH%" (
    echo [INFO] Modele d'embeddings absent : telechargement (~140 Mo)...
    echo        %MODEL_URL%
    curl -L -o "%MODEL_PATH%" "%MODEL_URL%"
    if errorlevel 1 (
        echo [ERREUR] Telechargement echoue.
        del "%MODEL_PATH%" 2>nul
        pause
        exit /b 1
    )
)

echo.
echo [IMPORTANT] Dans lancer_butinage.bat, pointe la memoire sur ce serveur :
echo             set "LARUCHE_EMBED_URL=http://localhost:8002"
echo.
echo [RUN] %LLAMA_EXE%
"%LLAMA_EXE%" -m "%MODEL_PATH%" --embeddings --pooling mean -c 4096 -b 4096 -ngl 99 --host 127.0.0.1 --port 8002
pause
exit /b 0
