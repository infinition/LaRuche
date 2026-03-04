@echo off
echo --- PATH --- > %~dp0env_debug.txt
echo %PATH% >> %~dp0env_debug.txt
echo --- RUSTUP TOOLCHAIN --- >> %~dp0env_debug.txt
rustup toolchain list >> %~dp0env_debug.txt
echo --- RUSTC VERBOSE --- >> %~dp0env_debug.txt
rustc --version --verbose >> %~dp0env_debug.txt
echo --- VSWHERE --- >> %~dp0env_debug.txt
where vswhere.exe >> %~dp0env_debug.txt
echo --- LINK --- >> %~dp0env_debug.txt
where link.exe >> %~dp0env_debug.txt
