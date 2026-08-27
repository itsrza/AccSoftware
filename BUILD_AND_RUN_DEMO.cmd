@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Novin Pardaz v1.7.1 - Windows Build

echo ===============================================
echo   NOVIN PARDAZ v1.7.1 - WINDOWS BUILD
echo ===============================================
echo.

where node >nul 2>&1 || (echo ERROR: Node.js not found.&pause&exit /b 1)
where npm >nul 2>&1 || (echo ERROR: npm not found.&pause&exit /b 1)
where cargo >nul 2>&1 || (echo ERROR: Rust Cargo not found.&pause&exit /b 1)
node -e "const [a,b]=process.versions.node.split('.').map(Number); if(a<20 || (a===20&&b<19)){process.exit(1)}" >nul 2>&1
if errorlevel 1 (echo ERROR: Node.js 20.19.0 or newer is required.&pause&exit /b 1)

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSINSTALL=%%I"
if not defined VSINSTALL (echo ERROR: Microsoft C++ Build Tools not found.&pause&exit /b 1)
call "%VSINSTALL%\Common7\Tools\VsDevCmd.bat" -arch=x64 >nul

if not exist "apps\desktop-ui\package.json" (echo ERROR: desktop-ui package.json is missing.&pause&exit /b 1)
if not exist "apps\desktop-ui\package-lock.json" (echo ERROR: desktop-ui package-lock.json is missing.&pause&exit /b 1)
if not exist "apps\desktop-host\src-tauri\tauri.conf.json" (echo ERROR: Tauri configuration is missing.&pause&exit /b 1)

echo [1/5] Installing locked frontend dependencies...
call npm --prefix apps\desktop-ui ci
if errorlevel 1 goto fail

echo [2/5] Commercial hardening tests...
call npm run test:hardening
if errorlevel 1 goto fail

echo [3/5] TypeScript and frontend build...
call npm run build
if errorlevel 1 goto fail

echo [4/5] Rust checks...
cargo fmt --manifest-path apps\desktop-host\src-tauri\Cargo.toml -- --check
if errorlevel 1 goto fail
cargo check --locked --manifest-path apps\desktop-host\src-tauri\Cargo.toml
if errorlevel 1 goto fail

echo [5/5] Tauri Windows installer build...
pushd apps\desktop-host\src-tauri
call ..\..\..\desktop-ui\node_modules\.bin\tauri.cmd build
set "TAURI_EXIT=%ERRORLEVEL%"
popd
if not "%TAURI_EXIT%"=="0" goto fail

echo.
echo BUILD COMPLETE.
echo Installer output is under apps\desktop-host\src-tauri\target\release\bundle\
for /r "apps\desktop-host\src-tauri\target\release\bundle" %%F in (*.exe) do echo EXE: %%F
for /r "apps\desktop-host\src-tauri\target\release\bundle" %%F in (*.msi) do echo MSI: %%F
pause
exit /b 0

:fail
cd /d "%~dp0"
echo.
echo BUILD FAILED. Review the error above.
pause
exit /b 1
