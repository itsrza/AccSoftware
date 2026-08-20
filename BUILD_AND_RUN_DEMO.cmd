@echo off
setlocal EnableExtensions
cd /d "%~dp0"
title Novin Pardaz v1.7.1 - Windows Build

echo ===============================================
echo   NOVIN PARDAZ v1.7.1 - WINDOWS BUILD
 echo ===============================================

where node >nul 2>&1 || (echo ERROR: Node.js not found.&pause&exit /b 1)
where npm >nul 2>&1 || (echo ERROR: npm not found.&pause&exit /b 1)
where cargo >nul 2>&1 || (echo ERROR: Cargo not found.&pause&exit /b 1)
where tauri >nul 2>&1 || (echo ERROR: Tauri CLI not found.&pause&exit /b 1)

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" (
  for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSINSTALL=%%I"
)
if not defined VSINSTALL (echo ERROR: MSVC Build Tools not found.&pause&exit /b 1)
call "%VSINSTALL%\Common7\Tools\VsDevCmd.bat" -arch=x64 >nul

if not exist "apps\desktop-ui\node_modules" (
  echo Installing dependencies...
  cd apps\desktop-ui
  call npm install
  if errorlevel 1 goto fail
  cd ..\..
)

echo [1/4] TypeScript check...
cd apps\desktop-ui
call npx tsc --noEmit
if errorlevel 1 goto fail

echo [2/4] Frontend build...
call npm run build
if errorlevel 1 goto fail
cd ..\..

echo [3/4] Rust check...
cargo check --manifest-path apps\desktop-host\src-tauri\Cargo.toml
if errorlevel 1 goto fail

echo [4/4] Tauri Windows build...
tauri build --config apps\desktop-host\src-tauri\tauri.conf.json
if errorlevel 1 goto fail

echo Build complete.
for /r "apps\desktop-host\src-tauri\target\release\bundle" %%F in (*.exe) do (
  echo Launching: %%F
  start "Novin Pardaz Demo" "%%F"
  goto launched
)
:launched
echo Demo launch requested.
pause
exit /b 0
:fail
cd /d "%~dp0"
echo.
echo BUILD FAILED. Review the error above.
pause
exit /b 1
