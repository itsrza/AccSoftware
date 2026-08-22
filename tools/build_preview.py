#!/usr/bin/env python3
"""ساخت فایل تک‌نفره‌ی پیش‌نمایش طراحی.

خروجی `docs/preview/index.html` یک فایل کاملاً خودکفاست (HTML + CSS + JS در یک
فایل) که فقط با باز کردن در مرورگر اجرا می‌شود — بدون نصب Node یا Rust.

کاربرد: بازبینی سریع چیدمان و ظاهر توسط کارفرما، پیش از بیلد کامل دسکتاپ.

⚠️ این فایل محصول نیست: داده‌ها شبیه‌سازی‌شده‌اند و موتور مالی Rust در آن اجرا
نمی‌شود. برنامه‌ی واقعی با `npm run tauri dev` اجرا می‌شود.

اجرا:
    python3 tools/build_preview.py
"""

import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
UI = ROOT / "apps" / "desktop-ui"
OUTPUT = ROOT / "docs" / "preview" / "index.html"

TEMPLATE = """<!doctype html>
<html lang="fa" dir="rtl">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>نوین پرداز — پیش‌نمایش طراحی</title>
<!--
  پیش‌نمایش طراحی نرم‌افزار حسابداری نوین پرداز نسل جدید.
  این فایل کاملاً خودکفاست: فقط آن را در مرورگر باز کنید.
  توجه: داده‌ها شبیه‌سازی‌شده‌اند و موتور مالی واقعی (Rust) در آن اجرا نمی‌شود.
-->
<style>{css}</style>
</head>
<body>
<div id="root"></div>
<script type="module">{js}</script>
</body>
</html>
"""


def main() -> int:
    if not (UI / "node_modules").is_dir():
        print("نصب وابستگی‌ها…")
        subprocess.run(["npm", "ci", "--no-audit", "--no-fund"], cwd=UI, check=True)

    print("ساخت بسته‌ی رابط کاربری…")
    dist = UI / "dist"
    if dist.exists():
        shutil.rmtree(dist)
    # حالت development تا شبیه‌ساز پیش‌نمایش فعال بماند.
    subprocess.run(
        ["node", "node_modules/vite/bin/vite.js", "build", "--mode", "development"],
        cwd=UI,
        check=True,
    )

    css = next(dist.glob("assets/*.css")).read_text(encoding="utf-8")
    js = next(dist.glob("assets/*.js")).read_text(encoding="utf-8")
    if "پیش‌نمایش طراحی" not in js:
        print("خطا: شبیه‌ساز پیش‌نمایش در بسته فعال نیست.", file=sys.stderr)
        return 1

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(
        TEMPLATE.format(css=css, js=js.replace("</script", "<\\/script")),
        encoding="utf-8",
    )
    size_kb = OUTPUT.stat().st_size / 1024
    print(f"آماده شد: {OUTPUT.relative_to(ROOT)} ({size_kb:.0f} کیلوبایت)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
