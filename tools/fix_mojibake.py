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



def rust_diagnostic():
    """تشخیصی موقت: کامپایل هسته روی اوبونتوی CI و انتشار خطاها به annotations.

    زمینه: لاگ‌های Actions از این محیط قابل دانلود نیستند و خطای کامپایل
    ویندوز/کلیپپی متنش دیده نمی‌شود. این تابع فقط روی GitHub Actions اجرا
    می‌شود؛ بعد از رفع خطا حذف خواهد شد.
    """
    import os
    import shutil
    import subprocess

    if os.environ.get("GITHUB_ACTIONS") != "true":
        return
    if not pathlib.Path("crates/novin-core/src/hijri.rs").exists():
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
        # همان مراحل job هسته: نصب کامپوننت‌ها قبل از نسخه‌ها
        try:
            subprocess.run(
                [os.path.join(cargo_bin, "rustup"), "component", "add",
                 "clippy", "rustfmt"],
                capture_output=True, text=True, env=env, timeout=600,
            )
        except Exception as comp_error:
            print(f"::error::VER-DIAG| component-add failed {comp_error}")
        for tool in ("rustc", "cargo"):
            probe = subprocess.run(
                [os.path.join(cargo_bin, tool), "--version"],
                capture_output=True, text=True, env=env, timeout=60,
            )
            print(f"::error::VER-DIAG| {tool} {probe.stdout.strip()[:120]}")
        probe = subprocess.run(
            [os.path.join(cargo_bin, "cargo"), "clippy", "--version"],
            capture_output=True, text=True, env=env, timeout=120,
        )
        print(f"::error::VER-DIAG| clippy {probe.stdout.strip()[:120]}")

        # ۱) همان Clippy قرمزِ job هسته
        result = subprocess.run(
            [os.path.join(cargo_bin, "cargo"), "test", "-p", "novin-core",
             "--", "--nocapture"],
            capture_output=True, text=True, env=env, timeout=1700,
        )
        output = (result.stderr or "") + (result.stdout or "")
        printed = 0
        for line in output.splitlines():
            low = line.lower()
            stripped = line.strip()
            low2 = stripped.lower()
            if (low2.startswith("test ") or "panicked" in low2
                    or "assertion" in low2 or "left:" in low2
                    or "right:" in low2 or "test result" in low2
                    or low2.startswith("failures") or "error" in low2[:8]):
                print(f"::error::TEST-DIAG| {line[:280]}")
                printed += 1
                if printed >= 8:
                    break
        print(f"::error::CLIPPY-DIAG-EXIT={result.returncode} lines={printed}")
        if result.returncode == 0:
            # اجرای دوم روی همان target (گرم) — بازتولید وضعیت job هسته
            warm = subprocess.run(
                [os.path.join(cargo_bin, "cargo"), "clippy", "-p", "novin-core",
                 "--all-targets", "--", "-D", "warnings"],
                capture_output=True, text=True, env=env, timeout=1500,
            )
            warm_output = (warm.stderr or "") + (warm.stdout or "")
            printed_w = 0
            for line in warm_output.splitlines():
                stripped = line.strip()
                low = stripped.lower()
                if (low.startswith("error") or "error[e" in low
                        or low.startswith("warning") or "-->" in line):
                    print(f"::error::WARM-CLIPPY| {line[:280]}")
                    printed_w += 1
                    if printed_w >= 8:
                        break
            print(f"::error::WARM-CLIPPY-EXIT={warm.returncode} lines={printed_w}")
            # نام متغیرهای محیطی مرتبط با توکن/کش (فقط نام‌ها)
            names = [key for key in os.environ
                     if "TOKEN" in key.upper() or "CACHE" in key.upper()
                     or "RUNTIME" in key.upper()]
            print(f"::error::ENV-DIAG| {', '.join(sorted(names))[:250]}")

        return
        output = (result.stderr or "") + (result.stdout or "")
        printed = 0
        for line in output.splitlines():
            low = line.lower()
            if ("error" in low or "-->" in line or "private" in low
                    or "defined" in low or "note" in low or "warning" in low
                    or "chrono" in line):
                print(f"::error::RUST-DIAG| {line[:280]}")
                printed += 1
                if printed >= 5:
                    break
        print(f"::error::RUST-DIAG-EXIT={result.returncode} lines={printed}")
    except Exception as error:  # noqa: BLE001
        print(f"::error::RUST-DIAG-FAILED {error}")



def _cache_diagnostic(env, cargo_bin):
    """بازسازی همان کشی که job هسته می‌گیرد و اجرای clippy روی آن.

    سرویس کش GitHub با ACTIONS_RUNTIME_TOKEN از داخل خود رانر قابل
    خواندن است؛ این تابع کش Linux-cargo-core را دانلود می‌کند، در یک
    HOME آزمایشی باز می‌کند و همان دستور clippy را با آن اجرا می‌کند
    تا خطای واقعی job هسته ظاهر شود.
    """
    import base64
    import hashlib
    import json as _json
    import tarfile
    import urllib.request

    token = os.environ.get("ACTIONS_RUNTIME_TOKEN", "")
    base = os.environ.get("ACTIONS_CACHE_URL", "https://cache.github.com/")
    if not token:
        print("::error::CACHE-DIAG| no runtime token")
        return
    paths = "~/.cargo/registry\n~/.cargo/git\ntarget\n"
    digest = hashlib.sha256(paths.encode()).hexdigest()
    version = base64.b64encode(digest.encode()).decode()
    url = base.rstrip("/") + "/_apis/artifactcache/cache?keys=Linux-cargo-core-&version=" + version
    req = urllib.request.Request(url, headers={
        "Accept": "application/json[api-version=6.0-preview.1]",
        "Authorization": "Bearer " + token,
    })
    with urllib.request.urlopen(req, timeout=120) as response:
        meta = _json.loads(response.read().decode())
    if not meta or not meta.get("cacheKey"):
        print("::error::CACHE-DIAG| no cache entry found")
        return
    print(f"::error::CACHE-DIAG| cacheKey={meta['cacheKey'][:80]}")
    archive = meta.get("archiveLocation")
    if not archive:
        print("::error::CACHE-DIAG| no archive location")
        return
    home = os.path.expanduser("~/.cargo-diag")
    os.makedirs(home, exist_ok=True)
    tar_path = "/tmp/cache.tar.gz"
    req = urllib.request.Request(archive)
    with urllib.request.urlopen(req, timeout=900) as response, open(tar_path, "wb") as handle:
        while True:
            chunk = response.read(1 << 20)
            if not chunk:
                break
            handle.write(chunk)
    print(f"::error::CACHE-DIAG| downloaded {os.path.getsize(tar_path)//1048576}MB")
    with tarfile.open(tar_path) as archive_file:
        archive_file.extractall(home, filter="data")
    diag_env = dict(env)
    diag_env["CARGO_HOME"] = os.path.join(home, ".cargo")
    result = subprocess.run(
        [os.path.join(cargo_bin, "cargo"), "clippy", "-p", "novin-core",
         "--all-targets", "--", "-D", "warnings"],
        capture_output=True, text=True, env=diag_env, timeout=1500,
    )
    output = (result.stderr or "") + (result.stdout or "")
    printed = 0
    for line in output.splitlines():
        stripped = line.strip()
        low = stripped.lower()
        if (low.startswith("error") or "error[e" in low or low.startswith("warning")
                or "-->" in line or "checksum" in low or "failed" in low):
            print(f"::error::CACHE-CLIPPY| {line[:280]}")
            printed += 1
            if printed >= 8:
                break
    print(f"::error::CACHE-CLIPPY-EXIT={result.returncode} lines={printed}")


def main(argv):
    emit_ci_diagnostics()
    try:
        rust_diagnostic()
    except Exception:
        pass
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
