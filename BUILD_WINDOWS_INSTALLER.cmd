@echo off
setlocal EnableExtensions EnableDelayedExpansion
cd /d "%~dp0"
chcp 65001 >nul
title Novin Pardaz - Windows Installer Builder

rem ===========================================================================
rem  ساخت نصاب ویندوز نوین پرداز - یک کلیک
rem ---------------------------------------------------------------------------
rem  این اسکریپت هرچه لازم است را خودش بررسی و در صورت نبودن نصب می‌کند،
rem  سپس رابط کاربری و برنامه‌ی دسکتاپ را می‌سازد و فایل نصاب (.exe و .msi)
rem  را تحویل می‌دهد.
rem
rem  چرا اسکریپت به‌جای فایل آماده: کامپایل ویندوزی به زنجیره‌ی ابزار MSVC
rem  نیاز دارد که فقط روی خود ویندوز در دسترس است.
rem ===========================================================================

echo.
echo ===============================================================
echo    NOVIN PARDAZ - ساخت نصاب ویندوز
echo ===============================================================
echo.

set "MISSING="

rem --------------------------------------------------- بررسی پیش‌نیازها
where node >nul 2>&1 || set "MISSING=!MISSING! Node"
where npm  >nul 2>&1 || set "MISSING=!MISSING! npm"
where cargo >nul 2>&1 || set "MISSING=!MISSING! Rust"

if not "!MISSING!"=="" (
  echo [پیش‌نیاز] این موارد نصب نیستند: !MISSING!
  where winget >nul 2>&1
  if errorlevel 1 (
    echo.
    echo   winget روی این ویندوز موجود نیست. لطفاً دستی نصب کنید:
    echo     Node.js LTS ..... https://nodejs.org/en/download
    echo     Rust ............ https://rustup.rs
    echo     Build Tools ..... https://visualstudio.microsoft.com/visual-cpp-build-tools/
    echo.
    pause & exit /b 1
  )
  echo [پیش‌نیاز] نصب خودکار با winget آغاز شد. ممکن است چند دقیقه طول بکشد...
  echo !MISSING! | find "Node" >nul && winget install -e --id OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements
  echo !MISSING! | find "Rust" >nul && winget install -e --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
  echo.
  echo [پیش‌نیاز] نصب انجام شد. لطفاً این پنجره را ببندید و دوباره اجرا کنید
  echo            تا مسیر ابزارهای تازه شناخته شود.
  pause & exit /b 0
)

rem --------------------------------------------- زنجیره‌ی ابزار کامپایل MSVC
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
set "VSINSTALL="
if exist "%VSWHERE%" (
  for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSINSTALL=%%I"
)
if not defined VSINSTALL (
  echo [پیش‌نیاز] ابزار کامپایل C++ مایکروسافت پیدا نشد.
  where winget >nul 2>&1 && (
    echo            نصب خودکار Build Tools آغاز شد...
    winget install -e --id Microsoft.VisualStudio.2022.BuildTools --accept-package-agreements --accept-source-agreements --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    echo            پس از پایان نصب، این پنجره را ببندید و دوباره اجرا کنید.
  ) || (
    echo            از این نشانی نصب کنید:
    echo            https://visualstudio.microsoft.com/visual-cpp-build-tools/
  )
  pause & exit /b 1
)
call "%VSINSTALL%\Common7\Tools\VsDevCmd.bat" -arch=x64 >nul

rem ------------------------------------------------------------ ساخت رابط کاربری
echo [1/4] نصب وابستگی‌های رابط کاربری...
pushd apps\desktop-ui
if exist package-lock.json ( call npm ci ) else ( call npm install )
if errorlevel 1 goto fail

echo [2/4] بررسی نوع‌ها و ساخت رابط کاربری...
rem حالت نمایشی روشن است: برنامه با کاربر نمونه وارد می‌شود و داده‌ی دمو
rem را نشان می‌دهد تا همه‌ی بخش‌ها بلافاصله قابل آزمایش باشند.
set "VITE_DEMO_MODE=true"
call npx tsc --noEmit
if errorlevel 1 goto fail
call npm run build
if errorlevel 1 goto fail
popd

rem --------------------------------------------------------- ساخت برنامه‌ی دسکتاپ
echo [3/4] کامپایل برنامه‌ی دسکتاپ (نخستین بار ۱۰ تا ۲۵ دقیقه طول می‌کشد)...
rem از پوشه‌ی میزبان اجرا می‌شود تا Tauri پوشه‌ی src-tauri را پیدا کند؛
rem خودِ ابزار از node_modules رابط کاربری برداشته می‌شود تا نیازی به نصب
rem سراسری Tauri CLI نباشد.
set "TAURI_BIN=%CD%\apps\desktop-ui\node_modules\.bin\tauri.cmd"
if not exist "%TAURI_BIN%" (
  echo   ابزار Tauri در node_modules پیدا نشد.
  goto fail
)
pushd apps\desktop-host
call "%TAURI_BIN%" build
if errorlevel 1 (popd & goto fail)
popd

rem ------------------------------------------------------------------ تحویل
echo [4/4] آماده‌سازی خروجی...
set "BUNDLE=%CD%\target\release\bundle"
if not exist "%BUNDLE%" set "BUNDLE=%CD%\apps\desktop-host\src-tauri\target\release\bundle"

echo.
echo ===============================================================
echo    ساخت با موفقیت تمام شد
echo ===============================================================
echo.
echo  فایل‌های نصاب اینجا هستند:
echo     %BUNDLE%\nsis\    ..... نصاب ساده (.exe)  - پیشنهاد می‌شود
echo     %BUNDLE%\msi\     ..... نصاب سازمانی (.msi)
echo.
echo  ورود به برنامه در حالت نمایشی خودکار است (admin / demo).
echo.
if exist "%BUNDLE%" start "" "%BUNDLE%"
pause
exit /b 0

:fail
popd 2>nul
echo.
echo ===============================================================
echo    ساخت ناموفق بود
echo ===============================================================
echo  آخرین پیام خطا را در همین پنجره ببینید.
echo  رایج‌ترین علت: نصب ناقص Build Tools یا قطع بودن اینترنت هنگام
echo  دریافت وابستگی‌های Rust.
pause
exit /b 1
