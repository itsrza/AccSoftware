#!/usr/bin/env python3
"""بازیابی متن‌های فارسیِ خراب‌شده (mojibake) در فایل‌های سورس.

علت خرابی: ذخیره‌ی فایل UTF-8 توسط ابزارهای ویندوزی با کدپیج cp1252 (چند بار پشت سر هم).
این اسکریپت همان تبدیل را معکوس می‌کند و بی‌اثر (idempotent) است.
"""
import re, sys, pathlib

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

def main(argv):
    check_only = "--check" in argv
    paths = [a for a in argv[1:] if not a.startswith("--")]
    exit_code = 0
    for raw in paths:
        path = pathlib.Path(raw)
        original = path.read_text(encoding="utf-8")
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
