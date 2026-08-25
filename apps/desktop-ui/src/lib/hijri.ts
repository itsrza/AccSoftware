/**
 * تقویم قمری — آینه‌ی TypeScript الگوریتم هسته.
 *
 * ## چرا نسخه‌ی TS هم هست
 * این ماژول **فقط برای داده‌ی نمونه‌ی پیش‌نمایش مرورگر** است؛ مسیر واقعی
 * برنامه از دستور `calendar_overview` میزبان می‌گذرد که همان محاسبه را در
 * `novin_core::hijri` انجام می‌دهد. دو پیاده‌سازی با لنگرهای یکسان در
 * تست‌ها (`audit12_calendar`) به هم قفل شده‌اند تا واگرا نشوند.
 *
 * الگوریتم: تقویم قمری حسابی (کویتی/مدنی) روی عدد روز جولیَن — رفت‌وبرگشتش
 * همیشه دقیق است؛ با رؤیت هلال واقعی ممکن است ±۱ روز اختلاف داشته باشد.
 */

export type Hijri = {year: number; month: number; day: number}

/** عدد روز جولیَن از تاریخ میلادی (تقویم پرولپتیک). */
function julianDay(date: Date): number {
  return Math.floor(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / 86_400_000) + 2_440_588
}

/** تاریخ میلادی از عدد روز جولیَن. */
function fromJulianDay(jdn: number): Date {
  return new Date((jdn - 2_440_588) * 86_400_000)
}

export function toHijri(date: Date): Hijri {
  let l = julianDay(date) - 1_948_440 + 10_632
  const n = Math.floor((l - 1) / 10_631)
  l = l - 10_631 * n + 354
  const j =
    Math.floor((10_985 - l) / 5_316) * Math.floor((50 * l) / 17_719) +
    Math.floor(l / 5_670) * Math.floor((43 * l) / 15_238)
  l =
    l -
    Math.floor((30 - j) / 15) * Math.floor((17_719 * j) / 50) -
    Math.floor(j / 16) * Math.floor((15_238 * j) / 43) +
    29
  const month = Math.floor((24 * l) / 709)
  const day = l - Math.floor((709 * month) / 24)
  const year = 30 * n + j - 30
  return {year, month, day}
}

export function hijriToGregorian(hijri: Hijri): Date | null {
  if (hijri.month < 1 || hijri.month > 12 || hijri.day < 1 || hijri.day > 30) return null
  const {year, month, day} = hijri
  const jdn =
    Math.floor((11 * year + 3) / 30) +
    354 * year +
    30 * month -
    Math.floor((month - 1) / 2) +
    day +
    1_948_440 -
    385
  return fromJulianDay(jdn)
}

/** نام ماه‌های قمری — همان فهرست هسته. */
export const HIJRI_MONTHS = [
  'محرم',
  'صفر',
  'ربیع‌الاول',
  'ربیع‌الثانی',
  'جمادی‌الاول',
  'جمادی‌الثانی',
  'رجب',
  'شعبان',
  'رمضان',
  'شوال',
  'ذی‌القعده',
  'ذی‌الحجه',
] as const
