/**
 * لایه‌ی خطای کاربردی — مطابق اصل «خطای فنی را مستقیم به کاربر نشان نده».
 *
 * بک‌اند خطاها را با قالب استاندارد `CODE-000: پیام فارسی` برمی‌گرداند.
 * اینجا آن را به یک شیء ساخت‌یافته تبدیل می‌کنیم تا:
 *  - کاربر پیام قابل فهم ببیند،
 *  - کد خطا برای پشتیبانی قابل ارجاع بماند،
 *  - خطاهای پیش‌بینی‌نشده هرگز به‌صورت stack trace خام نمایش داده نشوند.
 */

/** الگوی کد خطای استاندارد پلتفرم: AUTH-003، INV-134، ACC-006 و … */
const ERROR_CODE_PATTERN = /^([A-Z]{2,5}-\d{3}):\s*(.+)$/s

/** پیام جایگزین برای خطاهای ناشناخته. */
const FALLBACK_MESSAGE = 'عملیات انجام نشد. لطفاً دوباره تلاش کنید.'
const FALLBACK_CODE = 'APP-999'

export class AppError extends Error {
  readonly code: string
  /** متن اصلی دریافتی از بک‌اند — فقط برای لاگ و پشتیبانی. */
  readonly raw: string

  constructor(code: string, message: string, raw: string) {
    super(message)
    this.name = 'AppError'
    this.code = code
    this.raw = raw
  }

  /** متن آماده‌ی نمایش: پیام کاربرپسند به‌همراه کد پیگیری. */
  get display(): string {
    return `${this.message} (کد: ${this.code})`
  }
}

/** تبدیل هر مقدار پرتاب‌شده‌ای به یک خطای ساخت‌یافته. */
export function toAppError(error: unknown): AppError {
  if (error instanceof AppError) return error

  const raw =
    typeof error === 'string'
      ? error
      : error instanceof Error
        ? error.message
        : typeof error === 'object' && error !== null && 'message' in error
          ? String((error as { message: unknown }).message)
          : String(error)

  const match = ERROR_CODE_PATTERN.exec(raw.trim())
  if (match) return new AppError(match[1], match[2].trim(), raw)

  // خطای بدون کد استاندارد: متن فارسی خوانا حفظ می‌شود، در غیر این صورت پیام عمومی.
  const looksHumanReadable = /[\u0600-\u06FF]/.test(raw) && raw.length < 200
  return new AppError(FALLBACK_CODE, looksHumanReadable ? raw : FALLBACK_MESSAGE, raw)
}

/** متن آماده‌ی نمایش برای هر خطایی. */
export function errorText(error: unknown): string {
  return toAppError(error).display
}

/** آیا خطا به نبود مجوز مربوط است؟ (برای مخفی‌سازی هوشمند عملیات در UI) */
export function isPermissionError(error: unknown): boolean {
  return toAppError(error).code === 'AUTH-403'
}

/** آیا خطا به نبود نشست کاربر مربوط است؟ */
export function isAuthError(error: unknown): boolean {
  const code = toAppError(error).code
  return code === 'AUTH-001' || code === 'AUTH-002'
}
