@echo off
setlocal
REM ============================================================
REM  LaRuche v2 - demo POC memoire (auto-recall + auto-curation)
REM  Necessite un LLM OpenAI-compatible sur http://localhost:8001
REM  (ex. llama.cpp). Voir le haut de examples/poc_memoire.rs.
REM ============================================================
cd /d "%~dp0laruche"
echo === Lancement du POC memoire sur llama.cpp:8001 ===
cargo run -p laruche-essaim --example poc_memoire
pause
endlocal
