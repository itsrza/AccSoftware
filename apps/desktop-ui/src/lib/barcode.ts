/**
 * بارکدخوان — تشخیص اسکن سخت‌افزاری از تایپ انسان.
 *
 * ## چرا این روش
 * تقریباً همه‌ی بارکدخوان‌های فروشگاهی رایج در ایران — هانیول
 * (Voyager 1450g/1250G/Eclipse 5145)، دیتالاجیک، زبرا، زبکس Z3100، میوا
 * MBS-3615، اسکار و میندئو — به‌صورت **HID Keyboard Wedge** کار می‌کنند:
 * دستگاه خودش را به ویندوز به‌عنوان صفحه‌کلید معرفی می‌کند و رقم‌های بارکد
 * را مثل تایپ می‌فرستد، معمولاً با یک Enter در انتها.
 *
 * یعنی هیچ درایور، SDK یا پورت سریالی لازم نیست — و همین است که این روش را
 * با **همه‌ی برندها** سازگار می‌کند. کاری که نرم‌افزار باید بکند فقط یک چیز
 * است: تشخیص اینکه این کاراکترها را انسان تایپ کرده یا دستگاه فرستاده.
 *
 * ## معیار تشخیص
 * ۱. **سرعت** — دستگاه کاراکترها را با فاصله‌ی کمتر از چند ده میلی‌ثانیه
 *    می‌فرستد؛ سریع‌ترین تایپیست انسانی هم به این سرعت نمی‌رسد.
 * ۲. **پایان** — کاراکتر پایان (Enter یا Tab) که در خود دستگاه تنظیم شده.
 * ۳. **طول** — رشته‌ی کوتاه‌تر از حداقل تعیین‌شده، اسکن به حساب نمی‌آید.
 *
 * هر سه پارامتر از مرکز تنظیمات خوانده می‌شوند، چون بین مدل‌ها فرق دارند.
 *
 * ## چرا روی `document` گوش می‌دهیم
 * کاربر فروشگاه نباید مجبور باشد اول روی یک کادر کلیک کند و بعد اسکن کند.
 * با گوش دادن سراسری، اسکن در هر جای فرم فاکتور کار می‌کند. ولی وقتی
 * کاربر داخل یک فیلد متنی در حال تایپ است، رویداد نادیده گرفته می‌شود مگر
 * اینکه سرعتش در حد دستگاه باشد.
 */

import { useEffect, useRef } from 'react'



export type ScannerOptions = {
  enabled: boolean
  /** حداقل طول رشته برای اینکه اسکن به حساب بیاید. */
  minLength: number
  /** بیشترین فاصله‌ی مجاز بین دو کاراکتر، بر حسب میلی‌ثانیه. */
  maxGapMs: number
  /** کاراکتری که دستگاه در انتهای بارکد می‌فرستد. */
  suffix: 'enter' | 'tab' | 'none'
}

export const DEFAULT_SCANNER: ScannerOptions = {
  enabled: true,
  minLength: 6,
  maxGapMs: 60,
  suffix: 'enter',
}

/** کاراکترهای قابل قبول در یک بارکد: رقم، حرف لاتین و چند نشانه‌ی رایج. */
const ACCEPTABLE = /^[0-9A-Za-z\-_.\/ ]$/

/**
 * آیا کانون تمرکز روی فیلدی است که کاربر واقعاً در آن تایپ می‌کند؟
 *
 * حتی در این حالت اسکن رد نمی‌شود — فقط معیار سرعت سخت‌گیرانه‌تر می‌شود،
 * چون در غیر این صورت نوشتن «۱۲۳۴۵۶» در فیلد مبلغ به اشتباه اسکن تلقی
 * می‌شد.
 */
function inTextField(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null
  if (!element) return false
  const tag = element.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || element.isContentEditable
}

export type ScanResult = { code: string; elapsedMs: number; fromField: boolean }

/**
 * موتور تشخیص، مستقل از React تا مستقیماً قابل تست باشد.
 *
 * `now` تزریق‌پذیر است تا تست بتواند زمان را کنترل کند.
 */
export function createScannerEngine(
  options: ScannerOptions,
  onScan: (result: ScanResult) => void,
  now: () => number = () => Date.now(),
) {
  let buffer = ''
  let lastAt = 0
  let startedAt = 0
  let startedInField = false

  const reset = () => {
    buffer = ''
    startedAt = 0
    startedInField = false
  }

  const flush = () => {
    const code = buffer.trim()
    const elapsed = now() - startedAt
    const fromField = startedInField
    reset()
    if (code.length < options.minLength) return false
    onScan({ code, elapsedMs: elapsed, fromField })
    return true
  }

  /** برگرداندن `true` یعنی رویداد مصرف شد و نباید به فرم برسد. */
  const handle = (event: {
    key: string
    ctrlKey?: boolean
    altKey?: boolean
    metaKey?: boolean
    target?: EventTarget | null
  }): boolean => {
    if (!options.enabled) return false
    if (event.ctrlKey || event.altKey || event.metaKey) {
      reset()
      return false
    }

    const time = now()
    const gap = lastAt === 0 ? Number.POSITIVE_INFINITY : time - lastAt
    lastAt = time

    const terminator =
      (options.suffix === 'enter' && event.key === 'Enter') ||
      (options.suffix === 'tab' && event.key === 'Tab')

    if (terminator) {
      // پایان فقط وقتی معتبر است که رشته‌ی جمع‌شده به‌اندازه‌ی کافی بلند و
      // سریع باشد؛ وگرنه Enterِ کاربر است و باید به فرم برسد.
      if (buffer.length >= options.minLength && gap <= options.maxGapMs) return flush()
      reset()
      return false
    }

    if (event.key.length !== 1 || !ACCEPTABLE.test(event.key)) {
      reset()
      return false
    }

    if (gap > options.maxGapMs) {
      // شروع تازه — کاراکتر قبلی‌ها تایپ انسان بوده‌اند.
      buffer = event.key
      startedAt = time
      startedInField = inTextField(event.target ?? null)
      return false
    }

    buffer += event.key
    // حالت «بدون کاراکتر پایان»: به‌محض رسیدن به طول کافی، با یک مهلت کوتاه
    // بسته می‌شود. اینجا فقط انباشت می‌کنیم؛ بستن با تایمر بیرونی است.
    return false
  }

  /** بستن با مهلت — برای دستگاه‌هایی که کاراکتر پایان نمی‌فرستند. */
  const flushIfIdle = () => {
    if (options.suffix !== 'none') return false
    if (!buffer) return false
    if (now() - lastAt < options.maxGapMs * 3) return false
    return flush()
  }

  return { handle, flushIfIdle, reset, peek: () => buffer }
}

/**
 * اتصال بارکدخوان به صفحه.
 *
 * `onScan` با کد خوانده‌شده صدا زده می‌شود. اگر اسکن هنگام تمرکز روی یک
 * فیلد متنی رخ داده باشد، محتوای همان فیلد پاک می‌شود تا رقم‌های بارکد
 * داخلش باقی نمانند.
 */
export function useBarcodeScanner(options: ScannerOptions, onScan: (code: string) => void) {
  const callback = useRef(onScan)
  callback.current = onScan

  useEffect(() => {
    if (!options.enabled) return
    const engine = createScannerEngine(options, (result) => {
      if (result.fromField) {
        const active = document.activeElement as HTMLInputElement | null
        if (active && 'value' in active) {
          // رقم‌های بارکد نباید داخل فیلد بمانند.
          active.value = ''
        }
      }
      callback.current(result.code)
    })

    const onKeyDown = (event: KeyboardEvent) => {
      const consumed = engine.handle(event)
      if (consumed) {
        event.preventDefault()
        event.stopPropagation()
      }
    }

    document.addEventListener('keydown', onKeyDown, true)
    const timer =
      options.suffix === 'none' ? window.setInterval(() => engine.flushIfIdle(), 80) : undefined

    return () => {
      document.removeEventListener('keydown', onKeyDown, true)
      if (timer) window.clearInterval(timer)
    }
  }, [options])
}

/** خواندن تنظیمات بارکدخوان از فهرست تنظیمات برنامه. */
export function scannerOptionsFrom(
  settings: { key: string; value: string }[],
): ScannerOptions {
  const value = (key: string, fallback: string) =>
    settings.find((item) => item.key === key)?.value ?? fallback
  const suffix = value('hardware.barcode_suffix', DEFAULT_SCANNER.suffix)
  return {
    enabled: value('hardware.barcode_enabled', 'true') === 'true',
    minLength: Number(value('hardware.barcode_min_length', '6')) || DEFAULT_SCANNER.minLength,
    maxGapMs: Number(value('hardware.barcode_max_gap_ms', '60')) || DEFAULT_SCANNER.maxGapMs,
    suffix: suffix === 'tab' || suffix === 'none' ? suffix : 'enter',
  }
}
