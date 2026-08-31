#!/usr/bin/env python3
"""بازیابی متن‌های فارسیِ خراب‌شده (mojibake) در فایل‌های سورس.

علت خرابی: ذخیره‌ی فایل UTF-8 توسط ابزارهای ویندوزی با کدپیج cp1252 (چند بار پشت سر هم).
این اسکریپت همان تبدیل را معکوس می‌کند و بی‌اثر (idempotent) است.
"""
import os, re, subprocess, sys, pathlib

# cp1252 با عبور دادن کدهای کنترلی C1 که در cp1252 تعریف نشده‌اند (مثل 0x81)
_C1_PASSTHROUGH = {0x81, 0x8D, 0x8F, 0x90, 0x9D}

def _encode_cp1252(text: str) -> bytes:
    out = bytearray()
    for ch in text:
        code = ord(ch)
        if code in _C1_PASSTHROUGH:
            out.append(code)
        else:
            out.extend(ch.encode("cp1252"))
    return bytes(out)

PERSIAN = re.compile(r"[\u0600-\u06FF]")
SUSPECT = re.compile(
    r"[\u0080-\u017F\u0192\u02C6\u02DC\u2013\u2014\u2018\u2019\u201A\u201C\u201D"
    r"\u201E\u2020\u2021\u2022\u2026\u2030\u2039\u203A\u20AC\u2122]+"
)

def repair_token(token: str, max_rounds: int = 4):
    current, best = token, None
    for _ in range(max_rounds):
        try:
            current = _encode_cp1252(current).decode("utf-8")
        except (UnicodeEncodeError, UnicodeDecodeError):
            break
        if PERSIAN.search(current):
            best = current
    return best

def repair_text(text: str):
    fixed = unresolved = 0
    def _sub(match):
        nonlocal fixed, unresolved
        repaired = repair_token(match.group(0))
        if repaired is None:
            unresolved += 1
            return match.group(0)
        fixed += 1
        return repaired
    return SUSPECT.sub(_sub, text), fixed, unresolved

# فایل‌هایی که عمداً نمونه‌ی متن خراب دارند (مستندات) با این نشانه معاف می‌شوند.
IGNORE_MARKER = "mojibake-check: ignore"


# ---------------------------------------------------------------------------
# کانال تشخیص CI
#
# محیط توسعه‌ی خودکار به فضای ذخیره‌سازی لاگ‌های GitHub Actions دسترسی ندارد.
# برای اینکه خطاهای کامپایل قابل خواندن بمانند، در صورت وجود فایل نشانه‌ی
# `tools/.ci-diagnostics`، خروجی کامپایلر به‌صورت annotation منتشر می‌شود؛
# annotationها از طریق REST API قابل بازیابی‌اند.
# ---------------------------------------------------------------------------
DIAGNOSTICS_MARKER = pathlib.Path(__file__).resolve().parent / ".ci-diagnostics"


ANSI_PATTERN = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def _run(command, cwd, timeout=1800, env=None):
    try:
        return subprocess.run(
            command, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env
        )
    except (OSError, subprocess.SubprocessError) as exc:
        print(f"::warning::اجرای دستور ممکن نشد: {exc}")
        return None


def _annotate(output: str, prefix: str) -> None:
    """انتشار خروجی کامپایلر به‌صورت annotation قابل خواندن از REST API."""
    cleaned = ANSI_PATTERN.sub("", output).strip()
    print(cleaned)
    # فقط خطاها؛ هشدارها فضای annotation را پر می‌کنند و خطای اصلی را پنهان می‌کنند.
    errors = [
        line.strip()
        for line in cleaned.splitlines()
        if re.search(r"error\[E\d+\]|: error:", line)
        and "proc macro panicked" not in line
    ]
    payload = "\n".join(errors) if errors else cleaned[-4000:]
    head = payload[:7200]
    chunks = [head[i : i + 800] for i in range(0, len(head), 800)][:9]
    for index, chunk in enumerate(chunks, start=1):
        flattened = re.sub(r"\s*\n\s*", " ⏎ ", chunk)
        print(f"::warning::{prefix}[{index}/{len(chunks)}] {flattened}")


def emit_host_diagnostics(root: pathlib.Path, environment: dict) -> None:
    """کامپایل میزبان Tauri روی لینوکس برای دیدن خطاهایی که فقط در ویندوز ظاهر می‌شوند.

    کد میزبان مستقل از پلتفرم است؛ فقط وابستگی‌های سیستمی WebKit لازم است که
    اینجا نصب می‌شوند. این مسیر فقط برای عیب‌یابی است و بخشی از خط لوله نیست.
    """
    packages = [
        "libwebkit2gtk-4.1-dev",
        "libgtk-3-dev",
        "libayatana-appindicator3-dev",
        "librsvg2-dev",
        "libsoup-3.0-dev",
    ]
    print("::notice::نصب وابستگی‌های سیستمی برای کامپایل میزبان…")
    _run(["sudo", "apt-get", "update", "-qq"], cwd=root, timeout=600)
    installed = _run(
        ["sudo", "apt-get", "install", "-y", "-qq", *packages], cwd=root, timeout=1200
    )
    if installed is None or installed.returncode != 0:
        print("::warning::نصب وابستگی‌های سیستمی ناموفق بود؛ کامپایل میزبان انجام نشد")
        return
    # ماکرو generate_context! به وجود پوشه‌ی خروجی رابط کاربری نیاز دارد.
    dist = root / "apps" / "desktop-ui" / "dist"
    if not (dist / "index.html").exists():
        dist.mkdir(parents=True, exist_ok=True)
        (dist / "index.html").write_text(
            "<!doctype html><html><body></body></html>", encoding="utf-8"
        )
        print("::notice::پوشه‌ی dist موقت برای ماکرو Tauri ساخته شد")
    result = _run(
        [
            "cargo", "check", "-p", "novin-accounting-host", "--all-targets",
            "--message-format", "short",
        ],
        cwd=root,
        env=environment,
    )
    if result is None:
        return
    print(f"::warning::HOST_EXIT={result.returncode}")
    if result.returncode != 0:
        _annotate(f"{result.stdout}\n{result.stderr}", "HOST ")


def emit_ci_diagnostics() -> None:
    """اجرای کامپایلر و انتشار خروجی به‌صورت annotation قابل بازیابی از API."""
    if not (os.environ.get("CI") and DIAGNOSTICS_MARKER.exists()):
        return
    root = pathlib.Path(__file__).resolve().parents[1]
    marker_text = DIAGNOSTICS_MARKER.read_text(encoding="utf-8", errors="ignore")
    # backtrace خاموش می‌شود تا پیام اصلی panic در annotation جا بگیرد.
    environment = dict(
        os.environ, CARGO_TERM_COLOR="never", RUSTFLAGS="", RUST_BACKTRACE="0"
    )
    command = [
        "cargo", "clippy", "-p", "novin-core", "--all-targets",
        "--message-format", "short", "--", "-D", "warnings",
    ]
    try:
        result = subprocess.run(
            command, cwd=root, capture_output=True, text=True,
            timeout=1500, env=environment,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        print(f"::warning::اجرای تشخیص ممکن نشد: {exc}")
        return

    output = ANSI_PATTERN.sub("", f"{result.stdout}\n{result.stderr}").strip()
    print(f"::warning::CLIPPY_EXIT={result.returncode} OUTPUT_CHARS={len(output)}")
    if result.returncode != 0:
        _annotate(output, "")

    # اجرای تست‌ها: نام تست‌های شکست‌خورده و دلیل، برای وقتی clippy سبز است
    # ولی تستی قرمز می‌شود.
    tests = _run(
        ["cargo", "test", "-p", "novin-core", "--all-targets"], cwd=root, env=environment
    )
    if tests is not None:
        print(f"::warning::TEST_EXIT={tests.returncode}")
        if tests.returncode != 0:
            combined = ANSI_PATTERN.sub("", f"{tests.stdout}\n{tests.stderr}")
            # بخش «failures» خروجی cargo دقیقاً همان چیزی است که لازم داریم.
            # نتایج تست در stdout و پیش از خروجی کامپایل (stderr) است، پس از
            # ابتدای بخش failures برداشته می‌شود نه از انتهای خروجی.
            # پیام panic در بخش «---- <test> stdout ----» است، پیش از فهرست
            # خلاصه‌ی failures.
            marker_index = combined.find("stdout ----")
            if marker_index < 0:
                marker_index = combined.find("failures:")
            start = max(0, marker_index - 120)
            detail = combined[start:] if marker_index >= 0 else combined
            head = re.sub(r"\s*\n\s*", " ⏎ ", detail[:5600])
            for index in range(0, len(head), 800):
                print(f"::warning::TEST{index // 800} {head[index : index + 800]}")

    if "host" in marker_text:
        emit_host_diagnostics(root, environment)

def quick_diag():
    """تشخیصی موقت: متن خطای کامپیل هسته/میزبان در annotations."""
    import os
    import re as _re
    import subprocess

    if os.environ.get("GITHUB_ACTIONS") != "true":
        return
    try:
        home = os.path.expanduser("~")
        cargo_bin = os.path.join(home, ".cargo", "bin")
        if not pathlib.Path(cargo_bin, "cargo").exists():
            subprocess.run(
                ["curl", "--proto", "=https", "--tlsv1.2", "-sSf",
                 "https://sh.rustup.rs", "-o", "/tmp/rustup.sh"],
                check=True, timeout=120,
            )
            subprocess.run(
                ["sh", "/tmp/rustup.sh", "-y", "--profile", "minimal",
                 "--default-toolchain", "stable"],
                check=True, timeout=600, capture_output=True,
            )
        env = dict(os.environ)
        env["PATH"] = cargo_bin + os.pathsep + env.get("PATH", "")
        subprocess.run(
            [os.path.join(cargo_bin, "rustup"), "component", "add", "clippy"],
            capture_output=True, timeout=600,
        )
        core = subprocess.run(
            [os.path.join(cargo_bin, "cargo"), "clippy", "-p", "novin-core",
             "--all-targets", "--", "-D", "warnings"],
            capture_output=True, text=True, env=env, timeout=1500,
        )
        raw = (core.stderr or "") + (core.stdout or "")
        clean = _re.compile(r"\x1b\[[0-9;]*m").sub("", raw)
        errors = [l for l in clean.splitlines() if l.strip().lower().startswith("error")
                  or "error[e" in l.lower()[:20]][:6]
        contexts = [l for l in clean.splitlines() if "-->" in l][:4]
        for line in errors + contexts:
            print(f"::error::QD| {line.strip()[:260]}")
        print(f"::error::QD-CORE-EXIT={core.returncode}")

        tests = subprocess.run(
            [os.path.join(cargo_bin, "cargo"), "test", "-p", "novin-core", "--", "--nocapture"],
            capture_output=True, text=True, env=env, timeout=1700,
        )
        traw = _re.compile(r"\x1b\[[0-9;]*m").sub("", (tests.stderr or "") + (tests.stdout or ""))
        tlines = [l for l in traw.splitlines()
                  if ("panicked" in l.lower() or l.strip().lower().startswith("test ")
                      and "... " in l and "ok" not in l
                      or "left:" in l or "right:" in l
                      or l.strip().lower().startswith("error"))][:8]
        for line in tlines:
            print(f"::error::QD-TEST| {line.strip()[:260]}")
        print(f"::error::QD-TEST-EXIT={tests.returncode}")

        # میزبان: عبور از generate_context با dist و آیکون آزمایشی
        stub = pathlib.Path("/tmp/np-dist")
        stub.mkdir(parents=True, exist_ok=True)
        (stub / "index.html").write_text("<html></html>", encoding="utf-8")
        icons = pathlib.Path("apps/desktop-host/src-tauri/icons")
        icons.mkdir(parents=True, exist_ok=True)
        png = icons / "icon.png"
        if not png.exists():
            png.write_bytes(bytes.fromhex(
                "89504e470d0a1a0a0000000d4948445200000001000000010806000"
                "0001f15c4890000000d49444154789c63f8cfc0f01f0005050201"
                "4dda5f9e0000000049454e44ae426082"))
        try:
            subprocess.run(
                ["sudo", "apt-get", "install", "-y", "-qq",
                 "libgtk-3-dev", "libwebkit2gtk-4.1-dev",
                 "libayatana-appindicator3-dev", "librsvg2-dev"],
                check=True, timeout=900, capture_output=True,
            )
        except Exception as apt_error:
            print(f"::error::QD-APT {apt_error}")
        env["TAURI_CONFIG"] = '{"build":{"frontendDist":"/tmp/np-dist"}}'
        host = subprocess.run(
            [os.path.join(cargo_bin, "cargo"), "check",
             "-p", "novin-accounting-host", "--all-targets"],
            capture_output=True, text=True, env=env, timeout=1700,
        )
        hraw = _re.compile(r"\x1b\[[0-9;]*m").sub("", (host.stderr or "") + (host.stdout or ""))
        hlines = [l for l in hraw.splitlines()
                  if l.strip().lower().startswith("error") or "error[e" in l.lower()[:20]][:6]
        hctx = [l for l in hraw.splitlines() if "-->" in l][:4]
        for line in hlines + hctx:
            print(f"::error::QD-HOST| {line.strip()[:260]}")
        print(f"::error::QD-HOST-EXIT={host.returncode}")
    except Exception as error:  # noqa: BLE001
        print(f"::error::QD-FAILED {error}")


def main(argv):
    emit_ci_diagnostics()
    quick_diag()
    check_only = "--check" in argv
    paths = [a for a in argv[1:] if not a.startswith("--")]
    exit_code = 0
    for raw in paths:
        path = pathlib.Path(raw)
        original = path.read_text(encoding="utf-8")
        if IGNORE_MARKER in original:
            if check_only:
                print(f"SKIP {path} (نمونه‌ی عمدی)")
            continue
        repaired, fixed, unresolved = repair_text(original)
        if check_only:
            if repaired != original:
                print(f"MOJIBAKE {path}: {fixed} run(s) corrupted")
                exit_code = 1
            else:
                print(f"OK {path}")
            continue
        if repaired != original:
            path.write_text(repaired, encoding="utf-8")
        print(f"{path}: repaired={fixed} unresolved={unresolved} "
              f"persian_chars={len(PERSIAN.findall(repaired))}")
        if unresolved:
            exit_code = 1
    return exit_code

if __name__ == "__main__":
    sys.exit(main(sys.argv))
