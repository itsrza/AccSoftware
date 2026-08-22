# Novin Pardaz — Next-Generation Accounting Platform

**نسخه: 1.8.0** · پلتفرم حسابداری دسکتاپ نسل جدید برای بازار ایران

Tauri 2 · React 19 · TypeScript · Rust · SQLite — فارسی، راست‌به‌چپ، Offline-First

---

## معماری

```
apps/desktop-ui        رابط کاربری React (فقط لایه‌ی ارائه)
        ↓ IPC تایپ‌شده
apps/desktop-host      میزبان Tauri — مرز فرمان‌ها، مجوزها و نشست
        ↓
crates/novin-core      هسته‌ی مالی مستقل (بدون وابستگی به Tauri)
        ↓
SQLite                 پایگاه داده‌ی تراکنشی با مهاجرت نسخه‌ای
```

### چرا هسته‌ی جدا؟
`novin-core` هیچ وابستگی‌ای به Tauri، UI یا سیستم‌عامل ندارد. نتیجه:

- هر محاسبه‌ی مالی مستقل از رابط کاربری **قابل تست و قابل ممیزی** است.
- تست‌ها روی هر پلتفرمی در چند ثانیه اجرا می‌شوند.
- همین هسته در آینده برای سرویس Cloud، Sync، Mobile و ماژول AI قابل استفاده است.

| ماژول هسته | مسئولیت |
|---|---|
| `money` | مبلغ ریالی با عدد صحیح ۶۴ بیتی — بدون خطای ممیز شناور، پخش تخفیف بدون گم شدن ریال |
| `jalali` | تبدیل دوطرفه‌ی تقویم شمسی ↔ میلادی، اعتبارسنجی، کبیسه |
| `accounting` | سند دوطرفه، اعتبارسنجی تعادل، سند برگشتی، محاسبه‌ی فاکتور، سند خودکار |
| `inventory` | ارزش‌گذاری FIFO / میانگین متحرک / میانگین موزون، موجودی قابل فروش |
| `db` | اسکیما، مهاجرت نسخه‌ای، داده‌ی پایه و نمونه |

---

## 👁 دیدن نرم‌افزار

سه راه، از ساده به کامل — راهنمای کامل: [`docs/HOW_TO_RUN.md`](docs/HOW_TO_RUN.md)

| راه | نیاز | چه می‌بینید |
|---|---|---|
| **فایل تک‌نفره** — [`docs/preview/index.html`](docs/preview/index.html) را دانلود و دابل‌کلیک کنید | هیچ | ظاهر و چیدمان (داده شبیه‌سازی‌شده) |
| `npm run dev` در `apps/desktop-ui` | Node.js 22+ | همان، با به‌روزرسانی لحظه‌ای |
| `npm run tauri dev` | Node + Rust + MSVC | **نرم‌افزار واقعی** با پایگاه داده و موتور مالی |

---

## اجرا و توسعه

### پیش‌نیازها
- Node.js 22+
- Rust (stable) — برای بیلد دسکتاپ
- ویندوز: MSVC Build Tools

### رابط کاربری
```bash
cd apps/desktop-ui
npm ci
npm run dev        # توسعه
npm run build      # بیلد تولیدی
```

### برنامه‌ی دسکتاپ
```bash
cd apps/desktop-ui
npm run tauri dev
```

### تست‌های هسته
```bash
cargo test -p novin-core
```

### بیلد ویندوزی
راهنمای کامل: [`BUILD_WINDOWS.md`](BUILD_WINDOWS.md) · اسکریپت: `BUILD_AND_RUN_DEMO.cmd`

---

## حالت دمو

داده‌ی نمونه‌ی آموزشی به‌صورت **واقعی و متصل به هم** در پایگاه داده seed می‌شود
(مشتری → فاکتور → سند → دریافت → موجودی). هیچ داده‌ی ساختگی در سطح UI وجود ندارد.

حالت دمو فقط با متغیر محیطی صریح فعال می‌شود:

```bash
# apps/desktop-ui/.env
VITE_DEMO_MODE=true     # فقط بیلد توسعه
```

**پیش‌فرض خاموش است.** بیلد تجاری هرگز ورود خودکار یا ابزار حذف داده‌ی نمونه ندارد.

---

## کیفیت و CI

خط لوله‌ی CI (`ci-templates/ci.yml`) در هر push اجرا می‌شود:

| گام | تضمین |
|---|---|
| نگهبان انکودینگ | متن فارسی هیچ فایلی خراب نمی‌شود |
| نگهبان مخزن | `node_modules`، `dist` و لاگ بیلد وارد Git نمی‌شوند |
| Clippy | هشدار کامپایلر = خطا |
| تست هسته | ۱۰ تست سخت‌گیرانه‌ی یکپارچگی مالی |
| کامپایل ویندوز | میزبان Tauri روی پلتفرم هدف کامپایل می‌شود |
| TypeScript + بیلد | رابط کاربری بدون خطای نوع build می‌شود |

اگر متن فارسی فایلی خراب شد (mojibake ناشی از ابزارهای ویندوزی):

```bash
python3 tools/fix_mojibake.py apps/desktop-host/src-tauri/src/main.rs
```

---

## مستندات

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — معماری
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — نقشه‌ی راه نسخه‌ها
- [`docs/PHASE0_STABILIZATION.md`](docs/PHASE0_STABILIZATION.md) — گزارش فاز پایدارسازی
- [`docs/PROJECT_ANALYSIS.md`](docs/PROJECT_ANALYSIS.md) — تحلیل فنی وضعیت
- [`docs/CHANGELOG.md`](docs/CHANGELOG.md) — تاریخچه‌ی تغییرات
