/**
 * میانبرهای تک‌حرفی فارسی — مطابق نوار کناری نرم‌افزار مرجع (تصویر `mo0rdx`).
 *
 * ## چرا این لایه اهمیت دارد
 * حسابدار ایرانی روزی ده‌ها فاکتور می‌زند و دستش روی صفحه‌کلید است، نه روی
 * ماوس. در نرم‌افزار فعلی نوین پرداز، «ف» یعنی فاکتور فروش و «د» یعنی سند
 * دریافت. کاربری که مهاجرت می‌کند، همان عادت را با خودش می‌آورد.
 *
 * ## دو قاعده‌ای که رعایت شده
 * ۱. **هرگز حین تایپ فعال نمی‌شود.** اگر تمرکز روی `input`, `textarea`,
 *    `select` یا ناحیه‌ی قابل ویرایش باشد، حرف باید تایپ شود نه اینکه صفحه
 *    عوض کند. این تنها راهِ داشتنِ میانبر تک‌حرفی بدون خراب‌کردن فرم‌هاست.
 * ۲. **با Ctrl/Alt/Meta فعال نمی‌شود.** آن ترکیب‌ها میانبرهای مرورگر و
 *    سیستم‌عامل‌اند (`Ctrl+K` پالت فرمان است).
 */

/** نگاشت مرجع: حرف/کلید → صفحه‌ی مقصد. */
export const SHORTCUTS: Record<string, string> = {
  // حروف فارسی — دقیقاً مطابق جدول نوار کناری مرجع
  ک: 'products', // مدیریت کالاها
  ا: 'parties', // مدیریت اشخاص («الف»)
  ب: 'banks', // حساب‌های بانکی
  ص: 'cashboxes', // صندوق‌ها
  چ: 'checks', // چک‌ها
  ف: 'invoice-form', // فاکتور فروش
  خ: 'purchase', // فاکتور خرید
  پ: 'treasury-document', // صدور سند پرداخت
  د: 'treasury-document', // صدور سند دریافت
  س: 'single-journal', // سند حسابداری یک‌سطری
  ر: 'inventory-transfer', // رسید/حواله‌ی تحویل کالا
  ت: 'production', // تولید
  گ: 'reports', // گزارش گردش تفصیلی
  // کلیدهای تابعی مرجع
  F2: 'sales-return', // برگشت از فروش
  F3: 'purchase-return', // برگشت از خرید
  F4: 'cashboxes', // گزارش گردش صندوق
  F5: 'banks', // گزارش گردش بانک
}

/** آیا تمرکز روی جایی است که کاربر دارد متن می‌نویسد؟ */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  const tag = target.tagName.toLowerCase()
  if (tag === 'input' || tag === 'textarea' || tag === 'select') return true
  if (target.isContentEditable) return true
  // فهرست بازشوی جزء `Select` سفارشی هم با حرف جستجو می‌کند.
  return target.getAttribute('role') === 'listbox' || target.getAttribute('role') === 'combobox'
}

/**
 * صفحه‌ی مقصد یک رویداد صفحه‌کلید — یا `null` اگر میانبری نیست.
 *
 * جدا از React نوشته شده تا بشود بدون رندر کامل برنامه تستش کرد.
 */
export function shortcutTarget(event: {
  key: string
  ctrlKey?: boolean
  altKey?: boolean
  metaKey?: boolean
  target?: EventTarget | null
}): string | null {
  if (event.ctrlKey || event.altKey || event.metaKey) return null
  if (isTypingTarget(event.target ?? null)) return null
  return SHORTCUTS[event.key] ?? null
}
