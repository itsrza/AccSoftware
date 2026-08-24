/**
 * قالب‌بندی واحد اعداد، مبالغ و تاریخ برای کاربر ایرانی.
 *
 * تنها منبع حقیقت نمایش در رابط کاربری؛ پیش از این نُه نسخه‌ی تکراری از تابع
 * `money` در صفحات مختلف وجود داشت.
 *
 * قواعد محصول:
 *  - واحد داخلی همیشه **ریال** است؛ تومان فقط واحد نمایش/ورودی.
 *  - تاریخ در پایگاه داده میلادی/ISO است و فقط در لحظه‌ی نمایش شمسی می‌شود.
 */

const RIALS_PER_TOMAN = 10

/**
 * زبان فعال نمایش اعداد.
 *
 * چرا اینجا و نه در هر صفحه: بیش از هزار فراخوانی `formatRials`/`formatNumber`
 * در برنامه هست. اگر هر کدام می‌خواستند زبان را بگیرند، افزودن زبان دوم به
 * معنی دست‌زدن به همه‌ی آن‌ها بود. با یک «حالت ماژول» که لایه‌ی i18n هنگام
 * تغییر زبان به‌روزش می‌کند، همه‌ی محل‌های فراخوانی دست‌نخورده می‌مانند.
 */
export type NumberLocale = 'fa' | 'en' | 'ar'

/** ارقام هر زبان: فارسی «۱۲۳»، عربی «١٢٣»، انگلیسی «123». */
const INTL_TAG: Record<NumberLocale, string> = {
  fa: 'fa-IR',
  ar: 'ar-EG',
  en: 'en-US',
}

/** واحد پول در هر زبان — «ریال» واحد داخلی است و «تومان» واحد نمایشی. */
const CURRENCY_WORDS: Record<NumberLocale, { rial: string; toman: string }> = {
  fa: { rial: 'ریال', toman: 'تومان' },
  ar: { rial: 'ريال', toman: 'تومان' },
  en: { rial: 'IRR', toman: 'Toman' },
}

let activeLocale: NumberLocale = 'fa'
let numberFormatter = new Intl.NumberFormat(INTL_TAG.fa, { maximumFractionDigits: 0 })
let decimalFormatter = new Intl.NumberFormat(INTL_TAG.fa, { maximumFractionDigits: 3 })
const latinFormatter = new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 })

/**
 * تعیین زبان ارقام و واحد پول برای کل رابط کاربری.
 *
 * فقط لایه‌ی i18n این را صدا می‌زند؛ صفحه‌ها نباید مستقیم استفاده کنند.
 */
export function setNumberLocale(locale: NumberLocale): void {
  activeLocale = locale
  numberFormatter = new Intl.NumberFormat(INTL_TAG[locale], { maximumFractionDigits: 0 })
  decimalFormatter = new Intl.NumberFormat(INTL_TAG[locale], { maximumFractionDigits: 3 })
}

/** زبان فعال ارقام. */
export function numberLocale(): NumberLocale {
  return activeLocale
}

/** واحد «ریال» به زبان فعال — برای برچسب ستون‌ها و محورها. */
export function rialUnit(): string {
  return CURRENCY_WORDS[activeLocale].rial
}

/** واحد «تومان» به زبان فعال. */
export function tomanUnit(): string {
  return CURRENCY_WORDS[activeLocale].toman
}

/** عدد صحیح به زبان فعال — جایگزین `x.toLocaleString('fa-IR')`. */
export function formatCount(value: number): string {
  return numberFormatter.format(Math.round(value))
}

/** مبلغ ریالی با جداکننده‌ی هزارگان فارسی (بدون واحد). */
export function formatRials(amount: number): string {
  return numberFormatter.format(Math.round(amount))
}

/** مبلغ ریالی همراه با واحد. */
export function formatRialsWithUnit(amount: number): string {
  return `${formatRials(amount)} ${rialUnit()}`
}

/** نمایش مبلغ به تومان (واحد آشناتر برای کاربر). */
export function formatTomans(rials: number): string {
  return `${numberFormatter.format(Math.round(rials / RIALS_PER_TOMAN))} ${tomanUnit()}`
}

/** تبدیل ورودی تومان کاربر به ریال برای ارسال به بک‌اند. */
export function tomansToRials(tomans: number): number {
  return Math.round(tomans * RIALS_PER_TOMAN)
}

/** عدد عمومی (تعداد، درصد و…) با حداکثر سه رقم اعشار. */
export function formatNumber(value: number): string {
  return decimalFormatter.format(value)
}

/** عدد با ارقام لاتین — برای فیلدهای ورودی و کد. */
export function formatLatin(value: number): string {
  return latinFormatter.format(value)
}

/** نشانه‌ی درصد به زبان فعال — «٪» در فارسی/عربی و «%» در انگلیسی. */
export function percentSign(): string {
  return activeLocale === 'en' ? '%' : '٪'
}

/**
 * درصد با تعداد رقم اعشار مشخص، به ارقام و نشانه‌ی زبان فعال.
 *
 * `toFixed` ارقام لاتین می‌دهد؛ در صفحه‌ی فارسی «12.50٪» کنار «۱۲٬۵۰۰ ریال»
 * ناهماهنگ دیده می‌شود.
 */
export function percentText(value: number, digits = 2): string {
  const formatter = new Intl.NumberFormat(INTL_TAG[activeLocale], {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })
  return `${formatter.format(value)}${percentSign()}`
}

/** تبدیل ارقام فارسی/عربی ورودی کاربر به لاتین. */
export function normalizeDigits(input: string): string {
  return input.replace(/[۰-۹٠-٩]/g, (digit) => {
    const code = digit.charCodeAt(0)
    const base = code >= 0x06f0 ? 0x06f0 : 0x0660
    return String(code - base)
  })
}

/** خواندن مبلغ از ورودی کاربر (ارقام فارسی و جداکننده مجاز است). */
export function parseAmount(input: string): number | null {
  const cleaned = normalizeDigits(input).replace(/[,٬\s_]/g, '')
  if (!cleaned) return null
  const value = Number(cleaned)
  return Number.isFinite(value) ? value : null
}

const JALALI_MONTH_OFFSETS = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]

/**
 * تبدیل تاریخ میلادی به شمسی — هم‌الگوریتم با هسته‌ی Rust
 * (`crates/novin-core/src/jalali.rs`) تا نمایش و ذخیره هرگز واگرا نشوند.
 */
export function toJalali(date: Date): { year: number; month: number; day: number } {
  const gy = date.getFullYear()
  const gm = date.getMonth() + 1
  const gd = date.getDate()
  let jy = gy >= 1600 ? 979 : 0
  const gy0 = gy >= 1600 ? gy - 1600 : gy - 621
  const gy2 = gm > 2 ? gy0 + 1 : gy0
  let days =
    365 * gy0 +
    Math.floor((gy2 + 3) / 4) -
    Math.floor((gy2 + 99) / 100) +
    Math.floor((gy2 + 399) / 400) -
    80 +
    gd +
    JALALI_MONTH_OFFSETS[gm - 1]
  jy += 33 * Math.floor(days / 12053)
  days %= 12053
  jy += 4 * Math.floor(days / 1461)
  days %= 1461
  if (days > 365) {
    jy += Math.floor((days - 1) / 365)
    days = (days - 1) % 365
  }
  const month = days < 186 ? Math.floor(days / 31) + 1 : Math.floor((days - 186) / 30) + 7
  const day = days < 186 ? (days % 31) + 1 : ((days - 186) % 30) + 1
  return { year: jy, month, day }
}

const pad = (value: number) => String(value).padStart(2, '0')

/** نمایش شمسی یک تاریخ میلادی/ISO. */
export function formatJalali(input: string | Date): string {
  if (typeof input === 'string') {
    // مقادیری که از قبل شمسی‌اند (مانند `1405/05/10`) بدون تغییر نمایش داده می‌شوند.
    if (/^\d{4}\/\d{1,2}\/\d{1,2}$/.test(input)) return input
    const parsed = new Date(input)
    if (Number.isNaN(parsed.getTime())) return input
    input = parsed
  }
  const { year, month, day } = toJalali(input)
  return `${year}/${pad(month)}/${pad(day)}`
}

/** تاریخ امروز به شمسی. */
export function todayJalali(): string {
  return formatJalali(new Date())
}

/**
 * نمایش درصد با علامت — برای نشانگرهای رشد و افت.
 *
 * علامت مثبت عمداً نمایش داده می‌شود: «۱۲٪+» و «۱۲٪−» در یک نگاه از هم
 * تفکیک می‌شوند، ولی «۱۲٪» و «۱۲٪−» نه.
 */
export function formatPercent(value: number): string {
  const sign = value > 0 ? '+' : value < 0 ? '−' : ''
  return `${sign}${decimalFormatter.format(Math.abs(value))}${percentSign()}`
}
