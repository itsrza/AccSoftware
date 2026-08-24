/**
 * بازه‌های زمانی شمسی.
 *
 * ## چرا رشته‌ی شمسی، نه شیء تاریخ
 * تمام تاریخ‌های محصول در قالب `YYYY/MM/DD` شمسی ذخیره و رد و بدل می‌شوند.
 * اگر نوار فیلتر با `Date` کار کند، در هر مرز باید تبدیل انجام شود و هر
 * تبدیل یک فرصت خطاست. اینجا واحد کار «رشته‌ی شمسی» است و تبدیل به میلادی
 * فقط برای حساب کردن فاصله‌ی روزها انجام می‌شود.
 *
 * ## چرا مقایسه با «دوره‌ی قبل» طول برابر دارد
 * مقایسه‌ی «این ماه» با «ماه گذشته» وقتی امروز پنجمِ ماه است، گمراه‌کننده
 * است: پنج روز در برابر سی‌ویک روز. دوره‌ی قبل همیشه هم‌طولِ دوره‌ی جاری
 * گرفته می‌شود و بلافاصله پیش از آن تمام می‌شود.
 */

import { toJalali } from './format'

export type PresetId =
  | 'fiscalYear'
  | 'today'
  | 'yesterday'
  | 'thisWeek'
  | 'lastWeek'
  | 'thisMonth'
  | 'lastMonth'
  | 'thisQuarter'
  | 'thisYear'
  | 'custom'

export type JalaliRange = { preset: PresetId; from: string; to: string }

export const PRESETS: { id: PresetId }[] = [
  { id: 'fiscalYear' },
  { id: 'today' },
  { id: 'yesterday' },
  { id: 'thisWeek' },
  { id: 'lastWeek' },
  { id: 'thisMonth' },
  { id: 'lastMonth' },
  { id: 'thisQuarter' },
  { id: 'thisYear' },
]

const pad = (value: number) => String(value).padStart(2, '0')

const leapCache = new Map<number, boolean>()

/**
 * آیا سال شمسی کبیسه است؟
 *
 * به‌جای بازنویسی فرمول چرخه‌ی ۲۸۲۰ ساله — که یک اشتباه کوچک در آن، سال‌ها
 * بعد خودش را نشان می‌دهد — دقیقاً همان کاری انجام می‌شود که هسته‌ی Rust
 * می‌کند: طول سال از روی خودِ تبدیل تقویم اندازه گرفته می‌شود. اگر ۳۶۶ روز
 * بود، کبیسه است. یک منبع حقیقت، بدون امکان واگرایی.
 */
export function isJalaliLeap(year: number): boolean {
  const cached = leapCache.get(year)
  if (cached !== undefined) return cached
  const start = jalaliToDate(year, 1, 1).getTime()
  const next = jalaliToDate(year + 1, 1, 1).getTime()
  const leap = Math.round((next - start) / 86_400_000) === 366
  leapCache.set(year, leap)
  return leap
}

/** تعداد روزهای یک ماه شمسی. */
export function daysInJalaliMonth(year: number, month: number): number {
  if (month <= 6) return 31
  if (month <= 11) return 30
  return isJalaliLeap(year) ? 30 : 29
}

/** رشته‌ی شمسی از اجزا. */
export const jalaliString = (year: number, month: number, day: number) =>
  `${year}/${pad(month)}/${pad(day)}`

/** تجزیه‌ی `YYYY/MM/DD`؛ در صورت نامعتبر بودن `null`. */
export function parseJalali(value: string): { year: number; month: number; day: number } | null {
  const match = /^(\d{4})\/(\d{1,2})\/(\d{1,2})$/.exec(value.trim())
  if (!match) return null
  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  if (month < 1 || month > 12) return null
  if (day < 1 || day > daysInJalaliMonth(year, month)) return null
  return { year, month, day }
}

/**
 * شمسی → میلادی.
 *
 * به‌جای پیاده‌سازی دوباره‌ی الگوریتم تبدیل (که خطر واگرایی با هسته دارد)،
 * از خودِ `toJalali` به‌عنوان مرجع استفاده می‌شود: یک تخمین اولیه زده و بعد
 * روز‌به‌روز تصحیح می‌شود. تعداد گام‌ها همیشه کمتر از چند ده است و این تابع
 * فقط چند بار در هر تعامل کاربر صدا زده می‌شود.
 */
export function jalaliToDate(year: number, month: number, day: number): Date {
  const cursor = new Date(year + 621, 2, 21)
  cursor.setHours(0, 0, 0, 0)
  const target = year * 10_000 + month * 100 + day
  for (let guard = 0; guard < 800; guard += 1) {
    const parts = toJalali(cursor)
    const current = parts.year * 10_000 + parts.month * 100 + parts.day
    if (current === target) return cursor
    // فاصله‌ی تقریبی بر حسب روز؛ گام بزرگ اول، بعد گام یک‌روزه.
    const deltaYears = target > current ? Math.floor((target - current) / 10_000) : -Math.floor((current - target) / 10_000)
    const step = deltaYears !== 0 ? deltaYears * 365 : target > current ? 1 : -1
    cursor.setDate(cursor.getDate() + step)
  }
  return cursor
}

/** میلادی → رشته‌ی شمسی. */
export function dateToJalali(date: Date): string {
  const { year, month, day } = toJalali(date)
  return jalaliString(year, month, day)
}

/** جابه‌جایی یک تاریخ شمسی به اندازه‌ی چند روز. */
export function shiftJalali(value: string, days: number): string {
  const parts = parseJalali(value)
  if (!parts) return value
  const date = jalaliToDate(parts.year, parts.month, parts.day)
  date.setDate(date.getDate() + days)
  return dateToJalali(date)
}

/** فاصله‌ی دو تاریخ شمسی بر حسب روز (پایان − شروع). */
export function jalaliDayDiff(from: string, to: string): number {
  const a = parseJalali(from)
  const b = parseJalali(to)
  if (!a || !b) return 0
  const start = jalaliToDate(a.year, a.month, a.day).getTime()
  const end = jalaliToDate(b.year, b.month, b.day).getTime()
  return Math.round((end - start) / 86_400_000)
}

/** امروز به شمسی. */
export const todayJalaliString = () => dateToJalali(new Date())

/**
 * محاسبه‌ی بازه از روی پیش‌تنظیم.
 *
 * هفته در ایران از **شنبه** شروع می‌شود: `Date.getDay()` یکشنبه را صفر
 * می‌دهد، پس با `(getDay() + 1) % 7` به شنبه‌مبنا تبدیل می‌شود.
 */
export function resolveRange(
  preset: PresetId,
  custom?: { from: string; to: string },
  today = todayJalaliString(),
): JalaliRange {
  const parts = parseJalali(today)
  if (!parts) return { preset, from: today, to: today }
  const { year, month, day } = parts

  switch (preset) {
    case 'today':
      return { preset, from: today, to: today }
    case 'yesterday': {
      const previous = shiftJalali(today, -1)
      return { preset, from: previous, to: previous }
    }
    case 'thisWeek': {
      const gregorian = jalaliToDate(year, month, day)
      const offset = (gregorian.getDay() + 1) % 7
      return { preset, from: shiftJalali(today, -offset), to: today }
    }
    case 'lastWeek': {
      const gregorian = jalaliToDate(year, month, day)
      const offset = (gregorian.getDay() + 1) % 7
      const saturday = shiftJalali(today, -offset)
      return { preset, from: shiftJalali(saturday, -7), to: shiftJalali(saturday, -1) }
    }
    case 'thisMonth':
      return { preset, from: jalaliString(year, month, 1), to: today }
    case 'lastMonth': {
      const previousMonth = month === 1 ? 12 : month - 1
      const previousYear = month === 1 ? year - 1 : year
      return {
        preset,
        from: jalaliString(previousYear, previousMonth, 1),
        to: jalaliString(previousYear, previousMonth, daysInJalaliMonth(previousYear, previousMonth)),
      }
    }
    case 'thisQuarter': {
      const quarterStart = month - ((month - 1) % 3)
      return { preset, from: jalaliString(year, quarterStart, 1), to: today }
    }
    case 'thisYear':
      return { preset, from: jalaliString(year, 1, 1), to: today }
    // سال مالی از خود پایگاه داده می‌آید، نه از تقویم؛ پس بازه‌اش را
    // فراخوان می‌دهد. اگر نداد، به سال شمسی جاری برمی‌گردیم.
    case 'fiscalYear':
      return custom
        ? { preset, from: custom.from, to: custom.to }
        : { preset, from: jalaliString(year, 1, 1), to: jalaliString(year, 12, daysInJalaliMonth(year, 12)) }
    case 'custom':
      return custom
        ? { preset, from: custom.from, to: custom.to }
        : { preset, from: jalaliString(year, month, 1), to: today }
  }
}

/** دوره‌ی قبلی هم‌طول، بلافاصله پیش از دوره‌ی جاری. */
export function previousRange(range: JalaliRange): { from: string; to: string } {
  const length = Math.max(1, jalaliDayDiff(range.from, range.to) + 1)
  const to = shiftJalali(range.from, -1)
  const from = shiftJalali(range.from, -length)
  return { from, to }
}

/** آیا تاریخ داخل بازه است؟ مقایسه‌ی متنی چون قالب صفرپیشرو دارد. */
export const inRange = (date: string, from: string, to: string) => date >= from && date <= to
