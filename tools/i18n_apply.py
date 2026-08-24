#!/usr/bin/env python3
"""ابزار کمکیِ توسعه برای چندزبانی‌کردن صفحه‌های رابط کاربری.

این اسکریپت **بخشی از محصول نیست**؛ فقط کار مکانیکیِ جایگزینی رشته‌های
فارسیِ سخت‌کدشده با `t('key')` را انجام می‌دهد تا خطای انسانی در صدها
جایگزینی دستی پیش نیاید. خودِ ترجمه‌ها را انسان می‌نویسد (فایل نگاشت)،
و درستی نتیجه را `tsc` و تست‌های `__tests__/i18n.test.tsx` می‌سنجند.

کاربرد:
    python3 tools/i18n_apply.py <فایل> <نگاشت.json>

نگاشت: `{"متن فارسی": "کلید.ترجمه"}`

سه الگوی جایگزینی پشتیبانی می‌شود:
  ۱. صفت JSX:      placeholder="متن"   →  placeholder={t('key')}
  ۲. متن JSX:      >متن<               →  >{t('key')}<
  ۳. رشته‌ی کد:     'متن'               →  t('key')
"""
from __future__ import annotations

import json
import re
import sys


def apply_mapping(source: str, mapping: dict[str, str]) -> tuple[str, dict[str, int]]:
    hits: dict[str, int] = {}
    # بلندترین متن‌ها اول، تا جایگزینی جزئی روی متن بلندتر اثر نگذارد.
    for text in sorted(mapping, key=len, reverse=True):
        key = mapping[text]
        count = 0
        escaped = re.escape(text)
        # متن JSX ممکن است روی چند خط شکسته باشد؛ هر فاصله با `\s+` تطبیق می‌کند.
        loose = r"\s+".join(re.escape(word) for word in text.split())

        # ۱. صفت JSX با مقدار رشته‌ای
        pattern = re.compile(r'(\s[\w:-]+)=(["\'])' + escaped + r'\2')
        source, n = pattern.subn(lambda m: f"{m.group(1)}={{t('{key}')}}", source)
        count += n

        # ۲. متن خالص داخل تگ
        pattern = re.compile(r'>(\s*)' + loose + r'(\s*)<')
        source, n = pattern.subn(lambda m: f">{m.group(1)}{{t('{key}')}}{m.group(2)}<", source)
        count += n

        # ۳. رشته‌ی معمولی در کد
        pattern = re.compile(r'(["\'])' + escaped + r'\1')
        source, n = pattern.subn(f"t('{key}')", source)
        count += n

        hits[text] = count
    return source, hits


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__)
        return 2
    path, mapping_path = sys.argv[1], sys.argv[2]
    with open(path, encoding="utf-8") as handle:
        source = handle.read()
    with open(mapping_path, encoding="utf-8") as handle:
        mapping = json.load(handle)
    result, hits = apply_mapping(source, mapping)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(result)
    missed = [text for text, count in hits.items() if count == 0]
    print(f"{path}: {sum(hits.values())} جایگزینی")
    if missed:
        print("بدون تطابق:", *missed, sep="\n  ")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
