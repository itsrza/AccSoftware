/**
 * @vitest-environment jsdom
 *
 * ممیزی دور ۱۲ — تقویم سه‌گانه شمسی/میلادی/قمری و مناسبت‌ها.
 *
 * مرجع: نمایش سه‌گانه‌ی تاریخ در نرم‌افزارهای حسابداری ایران. دو چیز
 * سنجیده می‌شود:
 *  ۱. **آینه‌ی TS** الگوریتم قمری (برای پیش‌نمایش) با همان لنگرهای هسته.
 *  ۲. **پنل تقویم** — رفتار واقعی با ماک API و همگامی جدول مناسبت‌های
 *     پیش‌نمایش با جدول هسته (نگهبان واگرایی).
 */
import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { CalendarMenu } from '../components/CalendarPopover'
import { I18nProvider } from '../lib/i18n'
import { getCalendarOverview, type CalendarOverview } from '../api'
import { toHijri, hijriToGregorian } from '../lib/hijri'
import { calendarOverviewResponse, PREVIEW_OCCASIONS } from '../lib/preview/calendar'

vi.mock('../api', () => ({
  getCalendarOverview: vi.fn(),
}))

const SRC = resolve(__dirname, '..')
const ROOT = resolve(SRC, '../../..')
const read = (path: string) => readFileSync(resolve(SRC, path), 'utf8')
const readRoot = (path: string) => readFileSync(resolve(ROOT, path), 'utf8')

const OVERVIEW: CalendarOverview = {
  today: {
    iso: '2026-08-24',
    jalali: '1405/06/02',
    jalali_year: 1405,
    jalali_month: 6,
    jalali_day: 2,
    hijri: '1448/03/10',
    hijri_year: 1448,
    hijri_month: 3,
    hijri_day: 10,
    gregorian: '2026-08-24',
    weekday: 2,
    occasions: [
      {
        date: '2026-08-24',
        jalali: '1405/06/02',
        hijri: '1448/03/10',
        title: 'عید سعید فطر',
        calendar: 'hijri',
        holiday: true,
      },
    ],
  },
  occasions: [
    {
      date: '2026-08-24',
      jalali: '1405/06/02',
      hijri: '1448/03/10',
      title: 'عید سعید فطر',
      calendar: 'hijri',
      holiday: true,
    },
    {
      date: '2026-09-06',
      jalali: '1405/06/15',
      hijri: '1448/03/23',
      title: 'شهادت امام حسن عسکری (ع)',
      calendar: 'hijri',
      holiday: false,
    },
  ],
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(getCalendarOverview).mockResolvedValue(OVERVIEW)
})

afterEach(cleanup)

// ---------------------------------------------------------------------------
// آینه‌ی TypeScript الگوریتم قمری
// ---------------------------------------------------------------------------

describe('م۱۲ · آینه‌ی TS تقویم قمری', () => {
  it('ت۱ — همان لنگرهای تأییدشده‌ی هسته را می‌دهد', () => {
    // عید فطر ۱۴۴۷ = 2026-03-20 (اُم‌القری)
    expect(toHijri(new Date(2026, 2, 20))).toEqual({year: 1447, month: 10, day: 1})
    // عید قربان ۱۴۴۷ = 2026-05-27
    expect(toHijri(new Date(2026, 4, 27))).toEqual({year: 1447, month: 12, day: 10})
    // عاشورای ۱۴۴۷ = 2025-07-06
    expect(toHijri(new Date(2025, 6, 6))).toEqual({year: 1447, month: 1, day: 10})
  })

  it('ت۲ — رفت‌وبرگشت ۲۰۰۰ روز پیاپی بدون خطا', () => {
    // مقایسه «روز تقویمی محلی» است نه لحظه‌ی زمانی — تبدیل‌ها روی
    // اجزای تاریخ محلی کار می‌کنند و روی ماشین با هر منطقه‌زمانی
    // (مثلاً UTC+3:30 در ویندوز کاربر) باید یک روز برگردند.
    const start = new Date(2020, 0, 1).getTime()
    for (let index = 0; index < 2000; index += 1) {
      const date = new Date(start + index * 86_400_000)
      const back = hijriToGregorian(toHijri(date))
      expect(
        [back.getFullYear(), back.getMonth(), back.getDate()],
        date.toLocaleDateString('en-CA'),
      ).toEqual([date.getFullYear(), date.getMonth(), date.getDate()])
    }
  })

  it('ت۳ — تاریخ نامعتبر هیچ تاریخی نمی‌دهد', () => {
    expect(hijriToGregorian({year: 1447, month: 13, day: 1})).toBeNull()
    expect(hijriToGregorian({year: 1447, month: 1, day: 31})).toBeNull()
  })
})

// ---------------------------------------------------------------------------
// پنل تقویم — رفتار
// ---------------------------------------------------------------------------

describe('م۱۲ · پنل تقویم سه‌گانه', () => {
  async function openMenu() {
    render(
      <I18nProvider initialLocale="fa">
        <CalendarMenu />
      </I18nProvider>,
    )
    fireEvent.click(screen.getByRole('button', {name: /تقویم سه‌گانه/}))
    await screen.findByRole('dialog')
  }

  it('ت۴ — امروز در هر سه تقویم با نام روز هفته‌ی ایرانی', async () => {
    await openMenu()
    // سال با جداکننده‌ی هزارگان فارسی نمایش داده می‌شود: ۱۴۰۵ → ۱٬۴۰۵
    expect(screen.getByText(/دوشنبه ۲ شهریور ۱٬۴۰۵/)).toBeTruthy()
    // «قمری:» داخل <b> است و بقیه خواهرش؛ پس روی کل متن می‌سنچیم
    expect(document.body.textContent).toContain('قمری:')
    expect(document.body.textContent).toContain('۱۰ ربیع‌الاول ۱٬۴۴۸')
    expect(screen.getByText(/2026-08-24/)).toBeTruthy()
    expect(getCalendarOverview).toHaveBeenCalledTimes(1)
  })

  it('ت۵ — مناسبت امروز با نشان تعطیل دیده می‌شود', async () => {
    await openMenu()
    expect(screen.getAllByText('عید سعید فطر').length).toBeGreaterThanOrEqual(2)
    expect(screen.getAllByText('تعطیل').length).toBeGreaterThanOrEqual(1)
  })

  it('ت۶ — تقویم ماه: ۳۱ روز شهریور و نقطه روی روز مناسبت‌دار', async () => {
    await openMenu()
    // سرستون‌های هفته‌ی ایرانی
    for (const letter of ['ش', 'ی', 'د', 'س', 'چ', 'پ', 'ج']) {
      expect(screen.getByText(letter)).toBeTruthy()
    }
    // ۱۴۰۵ شهریور ۳۱ روزه است و روز ۲ (امروز) هایلایت شده
    expect(screen.getByText('۲').className).toContain('size-7')
    const dotDay = screen.getByTitle('شهادت امام حسن عسکری (ع)')
    expect(dotDay.textContent).toBe('۱۵')
  })

  it('ت۷ — فهرست مناسبت‌های ماه تاریخ شمسی و پرچم تعطیل دارد', async () => {
    await openMenu()
    expect(screen.getByText('مناسبت‌های این ماه')).toBeTruthy()
    expect(screen.getByText('1405/06/15')).toBeTruthy()
  })

  it('ت۸ — یادداشت دقت قمری نشان داده می‌شود', async () => {
    await openMenu()
    expect(screen.getByText(/±۱ روز اختلاف/)).toBeTruthy()
  })

  it('ت۹ — روز بدون مناسبت پیام صریح دارد', async () => {
    vi.mocked(getCalendarOverview).mockResolvedValue({
      ...OVERVIEW,
      today: {...OVERVIEW.today, occasions: []},
      occasions: [],
    })
    await openMenu()
    expect(screen.getByText('امروز مناسبتی ثبت نشده است.')).toBeTruthy()
  })

  it('ت۱۰ — خطای میزبان در جعبه‌ی خطا می‌آید', async () => {
    vi.mocked(getCalendarOverview).mockRejectedValue('CAL-002: بازه‌ی مناسبت‌ها نامعتبر است')
    await openMenu()
    const box = await screen.findByText(/بازه‌ی مناسبت‌ها نامعتبر است/)
    expect(box.textContent).toContain('CAL-002')
  })
})

// ---------------------------------------------------------------------------
// قرارداد و همگامی
// ---------------------------------------------------------------------------

describe('م۱۲ · قرارداد و همگامی', () => {
  it('ت۱۱ — تقویم از نوار بالا باز می‌شود و در api/میزبان ثبت است', () => {
    expect(read('components/Topbar.tsx')).toContain('<CalendarMenu />')
    expect(read('api.ts')).toContain("'calendar_overview'")
    const main = readRoot('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).toContain('mod calendar;')
    expect(main).toContain('calendar::calendar_overview')
    expect(readRoot('apps/desktop-host/src-tauri/src/calendar.rs')).toContain('occasions_between')
  })

  it('ت۱۲ — کلیدهای تقویم در هر سه زبان موجودند', () => {
    for (const file of ['fa.ts', 'en.ts', 'ar.ts']) {
      const dict = readFileSync(resolve(SRC, 'lib/i18n', file), 'utf8')
      for (const key of [
        'calendar.open',
        'calendar.today',
        'calendar.lunar',
        'calendar.holiday',
        'weekday.0',
        'weekday.6',
        'weekdayShort.0',
        'hijriMonth.1',
        'hijriMonth.12',
      ]) {
        expect(dict, `${file}: ${key}`).toContain(`'${key}'`)
      }
    }
  })

  it('ت۱۳ — جدول مناسبت‌های پیش‌نمایش آینه‌ی یک‌به‌یک جدول هسته است', () => {
    const core = readRoot('crates/novin-core/src/occasions.rs')
    const pattern =
      /Occasion\s*\{[^}]*?calendar:\s*"(jalali|hijri)",[^}]*?month:\s*(\d+),[^}]*?day:\s*(\d+),[^}]*?title:\s*"([^"]*)",[^}]*?holiday:\s*(true|false)[^}]*?\}/gs
    const coreRows = [...core.matchAll(pattern)].map((match) => ({
      calendar: match[1],
      month: Number(match[2]),
      day: Number(match[3]),
      title: match[4],
      holiday: match[5] === 'true',
    }))
    expect(coreRows.length, 'جدول هسته خوانده شود').toBeGreaterThanOrEqual(26)

    const key = (row: {calendar: string; month: number; day: number}) =>
      `${row.calendar}:${row.month}:${row.day}`
    const coreMap = new Map(coreRows.map((row) => [key(row), row]))
    for (const row of PREVIEW_OCCASIONS) {
      const mirror = coreMap.get(key(row))
      expect(mirror, `مناسبت ${key(row)} باید در هسته باشد`).toBeTruthy()
      expect(mirror?.title).toBe(row.title)
      expect(mirror?.holiday).toBe(row.holiday)
    }
    expect(PREVIEW_OCCASIONS.length).toBe(coreRows.length)
  })

  it('ت۱۴ — پاسخ پیش‌نمایش همان قرارداد میزبان را دارد', () => {
    const response = calendarOverviewResponse({})
    expect(response.today.weekday).toBeGreaterThanOrEqual(0)
    expect(response.today.weekday).toBeLessThanOrEqual(6)
    expect(response.today.jalali).toMatch(/^14\d{2}\/\d{2}\/\d{2}$/)
    expect(response.today.hijri).toMatch(/^14\d{2}\/\d{2}\/\d{2}$/)
    expect(Array.isArray(response.occasions)).toBe(true)

    // بازه‌ی شامل عید فطر ۱۴۴۷ (۱۴۰۴/۱۲/۲۹) باید آن را برگرداند
    const withRange = calendarOverviewResponse({
      fromJalali: '1404/12/01',
      toJalali: '1405/01/10',
    })
    expect(
      withRange.occasions.some((occasion) => occasion.title === 'عید سعید فطر'),
    ).toBe(true)
  })
})
