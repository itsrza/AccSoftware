@echo off
rem ============================================================
rem  نوین‌پرداز — ساخت و اجرای دمو روی ویندوز
rem  پیش‌نیاز: Node 20.19+ و Rust stable (gnu یا msvc)
rem ============================================================
setlocal
cd /d "%~dp0"

echo [1/3] نصب وابستگي‌هاي رابط کاربري...
cd apps\desktop-ui
call npm ci || goto :err

echo [2/3] ساخت خروجي رابط کاربري...
call npm run build || goto :err

echo [3/3] اجراي ميزبان Tauri (حالت دمو)...
set VITE_DEMO_MODE=true
cd ..\desktop-host\src-tauri
cargo run -p novin-accounting-host
goto :eof

:err
echo ساخت با خطا متوقف شد.
exit /b 1
