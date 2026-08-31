<#
.SYNOPSIS
    راه‌اندازی کامل نوین پرداز روی ویندوز — بررسی پیش‌نیازها، نصب، کلون و ساخت.

.DESCRIPTION
    این اسکریپت همه‌ی کارهای زیر را انجام می‌دهد:
      ۱. بررسی می‌کند Git، Node.js، Rust و ابزار کامپایل C++ نصب باشند.
      ۲. هرکدام نبود، با winget نصبشان می‌کند.
      ۳. مخزن را در C:\Projects\AccSoftware کلون (یا به‌روزرسانی) می‌کند.
      ۴. وابستگی‌های npm را نصب می‌کند.
      ۵. فهرست دستورهای npm را چاپ می‌کند.

    اجرای دوباره‌اش بی‌خطر است: چیزی که هست دوباره نصب نمی‌شود.

.PARAMETER Root
    پوشه‌ی ریشه‌ی پروژه‌ها. پیش‌فرض C:\Projects

.PARAMETER Branch
    شاخه‌ای که کلون می‌شود.

.PARAMETER Run
    پس از نصب، پیش‌نمایش را هم اجرا کند.

.EXAMPLE
    .\setup-windows.ps1
    .\setup-windows.ps1 -Run
#>

[CmdletBinding()]
param(
    [string]$Root = 'C:\Projects',
    [string]$Branch = 'arena/01a0242f-accsoftware',
    [switch]$Run
)

$ErrorActionPreference = 'Stop'
$RepoUrl = 'https://github.com/itsrza/AccSoftware.git'
$ProjectPath = Join-Path $Root 'AccSoftware'

# --------------------------------------------------------------------------- کمکی
function Write-Step { param([string]$Text) Write-Host "`n=== $Text" -ForegroundColor Cyan }
function Write-Ok   { param([string]$Text) Write-Host "  [OK]   $Text" -ForegroundColor Green }
function Write-Warn { param([string]$Text) Write-Host "  [!]    $Text" -ForegroundColor Yellow }
function Write-Bad  { param([string]$Text) Write-Host "  [X]    $Text" -ForegroundColor Red }

function Test-Command { param([string]$Name) return [bool](Get-Command $Name -ErrorAction SilentlyContinue) }

function Install-WithWinget {
    param([string]$Id, [string]$Label)
    if (-not (Test-Command 'winget')) {
        Write-Bad "$Label نصب نیست و winget هم موجود نیست."
        Write-Host  "         لطفاً دستی نصب کنید و دوباره اجرا کنید."
        exit 1
    }
    Write-Warn "$Label نصب نیست — نصب خودکار آغاز شد..."
    winget install -e --id $Id --accept-package-agreements --accept-source-agreements | Out-Host
    Write-Ok "$Label نصب شد."
    $script:NeedsRestart = $true
}

$script:NeedsRestart = $false

Write-Host ''
Write-Host '===============================================================' -ForegroundColor DarkCyan
Write-Host '   NOVIN PARDAZ - راه‌اندازی محیط توسعه روی ویندوز' -ForegroundColor White
Write-Host '===============================================================' -ForegroundColor DarkCyan

# --------------------------------------------------------------------- ۱. پیش‌نیازها
Write-Step 'بررسی پیش‌نیازها'

if (Test-Command 'git') { Write-Ok  "Git — $((git --version))" }
else { Install-WithWinget -Id 'Git.Git' -Label 'Git' }

if (Test-Command 'node') {
    $nodeVersion = (node --version).TrimStart('v')
    $major = [int]($nodeVersion.Split('.')[0])
    if ($major -lt 20) {
        Write-Warn "Node.js نسخه $nodeVersion قدیمی است (حداقل ۲۰ لازم است)."
        Install-WithWinget -Id 'OpenJS.NodeJS.LTS' -Label 'Node.js LTS'
    } else {
        Write-Ok "Node.js — v$nodeVersion"
    }
} else { Install-WithWinget -Id 'OpenJS.NodeJS.LTS' -Label 'Node.js LTS' }

if (Test-Command 'cargo') { Write-Ok "Rust — $((cargo --version))" }
else { Install-WithWinget -Id 'Rustlang.Rustup' -Label 'Rust' }

# ابزار کامپایل C++ فقط برای ساخت نسخه‌ی نصبی لازم است، نه برای پیش‌نمایش.
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasMsvc = $false
if (Test-Path $vsWhere) {
    $install = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($install) { $hasMsvc = $true; Write-Ok 'ابزار کامپایل C++ مایکروسافت' }
}
if (-not $hasMsvc) {
    Write-Warn 'ابزار کامپایل C++ نصب نیست — فقط برای ساخت فایل نصبی لازم است.'
    Write-Host  '         نصب: winget install -e --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
}

if ($script:NeedsRestart) {
    Write-Host ''
    Write-Host '---------------------------------------------------------------' -ForegroundColor Yellow
    Write-Host ' پیش‌نیازها نصب شدند. این پنجره را ببندید و اسکریپت را دوباره' -ForegroundColor Yellow
    Write-Host ' اجرا کنید تا مسیر ابزارهای تازه شناخته شود.' -ForegroundColor Yellow
    Write-Host '---------------------------------------------------------------' -ForegroundColor Yellow
    exit 0
}

# ------------------------------------------------------------------ ۲. کلون یا به‌روزرسانی
Write-Step "آماده‌سازی پوشه‌ی پروژه در $ProjectPath"

if (-not (Test-Path $Root)) {
    New-Item -ItemType Directory -Path $Root -Force | Out-Null
    Write-Ok "پوشه‌ی $Root ساخته شد."
}

if (Test-Path (Join-Path $ProjectPath '.git')) {
    Write-Ok 'مخزن از قبل وجود دارد — به‌روزرسانی می‌شود.'
    Push-Location $ProjectPath
    git fetch origin $Branch --quiet
    git checkout $Branch --quiet
    git pull --ff-only origin $Branch --quiet
    Pop-Location
} else {
    if (Test-Path $ProjectPath) {
        Write-Bad "پوشه‌ی $ProjectPath هست ولی مخزن Git نیست. نام دیگری بدهید یا پاکش کنید."
        exit 1
    }
    Write-Host '  در حال کلون...'
    git clone --branch $Branch $RepoUrl $ProjectPath
    Write-Ok 'کلون انجام شد.'
}

Push-Location $ProjectPath
$commit = git log -1 --pretty='%h  %s'
Write-Ok "آخرین تغییر: $commit"
Pop-Location

# ---------------------------------------------------------------- ۳. وابستگی‌های npm
Write-Step 'نصب وابستگی‌ها'
Push-Location $ProjectPath
npm run setup
if ($LASTEXITCODE -ne 0) { Write-Bad 'نصب وابستگی‌ها ناموفق بود.'; Pop-Location; exit 1 }
Write-Ok 'وابستگی‌ها نصب شدند.'

# ------------------------------------------------------------------ ۴. بررسی سلامت
Write-Step 'بررسی سلامت پروژه'
npm run typecheck
if ($LASTEXITCODE -eq 0) { Write-Ok 'بررسی نوع‌ها بدون خطا' } else { Write-Bad 'بررسی نوع‌ها خطا داشت' }
npm test
if ($LASTEXITCODE -eq 0) { Write-Ok 'همه‌ی تست‌ها سبز' } else { Write-Bad 'بعضی تست‌ها قرمزند' }
Pop-Location

# -------------------------------------------------------------------- ۵. راهنما
Write-Host ''
Write-Host '===============================================================' -ForegroundColor DarkCyan
Write-Host '   آماده است' -ForegroundColor White
Write-Host '===============================================================' -ForegroundColor DarkCyan
Write-Host ''
Write-Host "  مسیر پروژه:  $ProjectPath" -ForegroundColor Gray
Write-Host ''
Write-Host "  همه‌ی دستورها از همین پوشه اجرا می‌شوند: $ProjectPath" -ForegroundColor White
Write-Host ''
Write-Host '    npm run dev         ' -NoNewline -ForegroundColor Green
Write-Host '  اجرای پیش‌نمایش در مرورگر با داده‌ی نمونه'
Write-Host '    npm test            ' -NoNewline -ForegroundColor Green
Write-Host '  اجرای تست‌ها'
Write-Host '    npm run typecheck   ' -NoNewline -ForegroundColor Green
Write-Host '  بررسی نوع‌ها'
Write-Host '    npm run check       ' -NoNewline -ForegroundColor Green
Write-Host '  بررسی نوع‌ها + تست‌ها با هم'
Write-Host '    npm run build       ' -NoNewline -ForegroundColor Green
Write-Host '  ساخت نسخه‌ی تولیدی رابط کاربری'
Write-Host '    npm run desktop     ' -NoNewline -ForegroundColor Green
Write-Host '  اجرای برنامه‌ی دسکتاپ (نیازمند Rust)'
Write-Host '    npm run installer   ' -NoNewline -ForegroundColor Green
Write-Host '  ساخت فایل نصبی ویندوز (exe و msi)'
Write-Host '    npm run setup       ' -NoNewline -ForegroundColor Green
Write-Host '  نصب دوباره‌ی وابستگی‌ها'
Write-Host ''
Write-Host '  میان‌بر شروع:' -ForegroundColor White
Write-Host "    cd $ProjectPath ; npm run dev" -ForegroundColor Gray
Write-Host ''

if ($Run) {
    Write-Step 'اجرای پیش‌نمایش'
    Set-Location $ProjectPath
    npm run dev
}
