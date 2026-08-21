/**
 * تست‌های سخت‌گیرانه‌ی لایه‌ی ارائه.
 *
 * دو ادعای حیاتی سنجیده می‌شود:
 *  ۱. هیچ متن فنی خامی به کاربر نشان داده نمی‌شود و کد خطا هرگز گم نمی‌شود.
 *  ۲. تبدیل تاریخ در رابط کاربری **دقیقاً** با هسته‌ی Rust یکسان است؛
 *     واگرایی این دو یعنی کاربر تاریخی متفاوت از آنچه ذخیره شده می‌بیند.
 */
import { describe, expect, it } from 'vitest'
import { AppError, errorText, isAuthError, isPermissionError, toAppError } from '../lib/errors'
import {
  formatJalali,
  formatNumber,
  formatRials,
  formatRialsWithUnit,
  formatTomans,
  normalizeDigits,
  parseAmount,
  toJalali,
  tomansToRials,
} from '../lib/format'

describe('لایه‌ی خطا', () => {
  it('کد و پیام خطای بک‌اند را جدا می‌کند', () => {
    const error = toAppError('INV-134: سطر انبارگردانی معتبر نیست')
    expect(error).toBeInstanceOf(AppError)
    expect(error.code).toBe('INV-134')
    expect(error.message).toBe('سطر انبارگردانی معتبر نیست')
    expect(error.display).toBe('سطر انبارگردانی معتبر نیست (کد: INV-134)')
  })

  it('خطای فنی خام را به پیام قابل فهم تبدیل می‌کند', () => {
    const technical = new Error(
      'called `Option::unwrap()` on a `None` value at src/main.rs:1234:56 ... stack backtrace ...',
    )
    const error = toAppError(technical)
    expect(error.code).toBe('APP-999')
    expect(error.message).toBe('عملیات انجام نشد. لطفاً دوباره تلاش کنید.')
    expect(error.message).not.toContain('unwrap')
    expect(error.raw).toContain('unwrap') // متن اصلی فقط برای پشتیبانی حفظ می‌شود
  })

  it('پیام فارسی بدون کد را حفظ می‌کند و خطای ساخت‌یافته را دوباره نمی‌پیچد', () => {
    expect(toAppError('فایل باید حداقل یک ردیف داده داشته باشد.').message).toBe(
      'فایل باید حداقل یک ردیف داده داشته باشد.',
    )
    const original = new AppError('ACC-006', 'سند نامتعادل است', 'ACC-006: سند نامتعادل است')
    expect(toAppError(original)).toBe(original)
    expect(errorText(original)).toBe('سند نامتعادل است (کد: ACC-006)')
  })

  it('خطاهای مجوز و نشست را تشخیص می‌دهد', () => {
    expect(isPermissionError('AUTH-403: مجوز لازم وجود ندارد: sales.invoice.post')).toBe(true)
    expect(isAuthError('AUTH-002: ابتدا وارد حساب کاربری شوید')).toBe(true)
    expect(isPermissionError('INV-100: شرکت فعال یافت نشد')).toBe(false)
    expect(isAuthError(null)).toBe(false)
  })
})

describe('قالب‌بندی مبلغ و عدد', () => {
  it('مبلغ ریالی را با ارقام فارسی و جداکننده نمایش می‌دهد', () => {
    expect(formatRials(12500000)).toBe('۱۲٬۵۰۰٬۰۰۰')
    expect(formatRialsWithUnit(12500000)).toBe('۱۲٬۵۰۰٬۰۰۰ ریال')
    expect(formatTomans(12500000)).toBe('۱٬۲۵۰٬۰۰۰ تومان')
    expect(formatRials(0)).toBe('۰')
    expect(formatRials(-45000)).toContain('۴۵٬۰۰۰')
  })

  it('تبدیل تومان به ریال دقیق است', () => {
    expect(tomansToRials(1250000)).toBe(12500000)
    expect(tomansToRials(0)).toBe(0)
  })

  it('ورودی کاربر با ارقام فارسی را درست می‌خواند', () => {
    expect(normalizeDigits('۱۲۳۴۵۶۷۸۹۰')).toBe('1234567890')
    expect(normalizeDigits('٠١٢٣')).toBe('0123')
    expect(parseAmount('۱۲٬۵۰۰,۰۰۰')).toBe(12500000)
    expect(parseAmount('  ۹۵۰۰ ')).toBe(9500)
    expect(parseAmount('')).toBeNull()
    expect(parseAmount('ن')).toBeNull()
  })

  it('تعداد کسری را با حداکثر سه رقم اعشار نمایش می‌دهد', () => {
    expect(formatNumber(2.5)).toBe('۲٫۵')
    expect(formatNumber(1000)).toBe('۱٬۰۰۰')
  })
})

describe('تقویم شمسی رابط کاربری', () => {
  it('با نقاط مرجع تاریخی مطابقت دارد', () => {
    expect(toJalali(new Date(2025, 7, 21))).toEqual({ year: 1404, month: 5, day: 30 })
    expect(toJalali(new Date(2026, 2, 21))).toEqual({ year: 1405, month: 1, day: 1 })
    expect(toJalali(new Date(2024, 2, 20))).toEqual({ year: 1403, month: 1, day: 1 })
    expect(toJalali(new Date(1979, 1, 11))).toEqual({ year: 1357, month: 11, day: 22 })
  })

  it('هیچ روزی در ۵۰ سال با هسته‌ی Rust واگرا نمی‌شود', () => {
    // پورت مستقیم الگوریتم هسته به‌عنوان مرجع مقایسه (crates/novin-core/src/jalali.rs)
    const reference = (date: Date) => {
      const offsets = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]
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
        offsets[gm - 1]
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

    const cursor = new Date(2000, 0, 1)
    const end = new Date(2050, 0, 1)
    let checked = 0
    while (cursor < end) {
      expect(toJalali(cursor)).toEqual(reference(cursor))
      cursor.setDate(cursor.getDate() + 1)
      checked += 1
    }
    expect(checked).toBeGreaterThan(18000)
  })

  it('تاریخ‌های شمسی موجود را دست‌نخورده و ورودی نامعتبر را ایمن برمی‌گرداند', () => {
    expect(formatJalali('1405/05/10')).toBe('1405/05/10')
    expect(formatJalali('2025-08-21')).toBe('1404/05/30')
    expect(formatJalali('—')).toBe('—')
    expect(formatJalali('')).toBe('')
  })
})
