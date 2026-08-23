/**
 * @vitest-environment jsdom
 *
 * انطباق با سیستم طراحی مرجع (`References/UI-BY-AI`) و درستی بازه‌ی شمسی.
 *
 * دو چیز سنجیده می‌شود:
 *  ۱. **تقویم** — تبدیل شمسی/میلادی، کبیسه، و پیش‌تنظیم‌های بازه. یک خطای
 *     یک‌روزه در بازه یعنی گزارش فروش یک روز کم یا زیاد می‌آورد.
 *  ۲. **چیدمان** — داشبورد همان ترتیب و همان اجزای مرجع را داشته باشد.
 */
import { describe, expect, it, afterEach, vi } from 'vitest'
import { render, screen, fireEvent, cleanup, within } from '@testing-library/react'
import { useState } from 'react'
import { readFileSync, readdirSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  PRESETS,
  daysInJalaliMonth,
  dateToJalali,
  isJalaliLeap,
  jalaliDayDiff,
  jalaliToDate,
  parseJalali,
  previousRange,
  rangeLabel,
  resolveRange,
  shiftJalali,
  type JalaliRange,
} from '../lib/dateRange'
import { toJalali } from '../lib/format'
import { FilterBar } from '../components/FilterBar'
import { toBuckets } from '../components/DashboardPanels'
import type { PartyAging } from '../api'

const src = (path: string) => readFileSync(resolve(__dirname, '..', path), 'utf8')

afterEach(cleanup)

// ---------------------------------------------------------------------------
// تقویم شمسی
// ---------------------------------------------------------------------------
describe('بازه‌ی زمانی شمسی', () => {
  it('ب۱ — تبدیل شمسی↔میلادی برای ۱۳ سال رفت‌وبرگشت دقیق است', () => {
    let checked = 0
    for (let year = 1398; year <= 1410; year += 1) {
      for (let month = 1; month <= 12; month += 1) {
        for (const day of [1, 15, daysInJalaliMonth(year, month)]) {
          const back = toJalali(jalaliToDate(year, month, day))
          expect([back.year, back.month, back.day], `${year}/${month}/${day}`).toEqual([
            year,
            month,
            day,
          ])
          checked += 1
        }
      }
    }
    expect(checked).toBeGreaterThan(450)
  })

  it('ب۲ — نقاط مرجع تأییدشده با نرم‌افزار واقعی می‌خوانند', () => {
    expect(dateToJalali(new Date(2025, 7, 21))).toBe('1404/05/30')
    expect(dateToJalali(new Date(2026, 2, 21))).toBe('1405/01/01')
    expect(dateToJalali(new Date(2024, 2, 20))).toBe('1403/01/01')
  })

  it('ب۳ — کبیسه از خود تقویم مشتق می‌شود، نه از فرمول جدا', () => {
    expect(isJalaliLeap(1403)).toBe(true)
    expect(isJalaliLeap(1404)).toBe(false)
    expect(daysInJalaliMonth(1403, 12)).toBe(30)
    expect(daysInJalaliMonth(1404, 12)).toBe(29)
    expect(parseJalali('1404/12/30')).toBeNull()
    expect(parseJalali('1403/12/30')).not.toBeNull()
  })

  it('ب۴ — مرز ماه‌های ۳۱ و ۳۰ روزه درست جابه‌جا می‌شود', () => {
    expect(shiftJalali('1405/01/31', 1)).toBe('1405/02/01')
    expect(shiftJalali('1405/07/01', -1)).toBe('1405/06/31')
    expect(shiftJalali('1405/01/01', -1)).toBe('1404/12/29')
    expect(jalaliDayDiff('1405/01/01', '1405/02/01')).toBe(31)
  })

  it('ب۵ — پیش‌تنظیم‌ها بازه‌ی درست می‌دهند', () => {
    const month = resolveRange('thisMonth', undefined, '1405/05/15')
    expect([month.from, month.to]).toEqual(['1405/05/01', '1405/05/15'])

    const last = resolveRange('lastMonth', undefined, '1405/05/15')
    expect([last.from, last.to]).toEqual(['1405/04/01', '1405/04/31'])

    const quarter = resolveRange('thisQuarter', undefined, '1405/05/15')
    expect(quarter.from).toBe('1405/04/01')

    const year = resolveRange('thisYear', undefined, '1405/05/15')
    expect(year.from).toBe('1405/01/01')

    const yesterday = resolveRange('yesterday', undefined, '1405/01/01')
    expect(yesterday.from).toBe('1404/12/29')
  })

  it('ب۶ — هفته از شنبه شروع می‌شود', () => {
    // ۱۴۰۵/۰۱/۰۷ برابر ۲۰۲۶-۰۳-۲۷ و روز جمعه است؛ پس شنبه‌ی همان هفته ۰۱/۰۱ است.
    const friday = jalaliToDate(1405, 1, 7)
    expect(friday.getDay()).toBe(5)
    const week = resolveRange('thisWeek', undefined, '1405/01/07')
    expect(week.from).toBe('1405/01/01')
    expect(week.to).toBe('1405/01/07')
  })

  it('ب۷ — دوره‌ی قبل هم‌طول است و بلافاصله پیش از دوره‌ی جاری تمام می‌شود', () => {
    const range = resolveRange('thisMonth', undefined, '1405/05/15')
    const previous = previousRange(range)
    expect(previous.to).toBe('1405/04/31')
    expect(jalaliDayDiff(previous.from, previous.to)).toBe(
      jalaliDayDiff(range.from, range.to),
    )
  })

  it('ب۸ — پیش‌تنظیم «سال مالی» بازه‌اش را از بیرون می‌گیرد', () => {
    const range = resolveRange('fiscalYear', { from: '1405/01/01', to: '1405/12/29' })
    expect([range.from, range.to]).toEqual(['1405/01/01', '1405/12/29'])
    expect(rangeLabel(range)).toBe('سال مالی')
  })

  it('ب۹ — تاریخ نامعتبر رد می‌شود', () => {
    expect(parseJalali('1405/13/01')).toBeNull()
    expect(parseJalali('1405/07/31')).toBeNull()
    expect(parseJalali('۱۴۰۵/۰۱/۰۱')).toBeNull()
    expect(parseJalali('1405/01/01')).toEqual({ year: 1405, month: 1, day: 1 })
  })
})

// ---------------------------------------------------------------------------
// نوار فیلتر
// ---------------------------------------------------------------------------
describe('نوار فیلتر مرجع', () => {
  const Harness = ({ onChange }: { onChange?: (r: JalaliRange) => void }) => {
    const [range, setRange] = useState<JalaliRange>(
      resolveRange('fiscalYear', { from: '1405/01/01', to: '1405/12/29' }),
    )
    const [payment, setPayment] = useState('all')
    return (
      <FilterBar
        range={range}
        onRange={(next) => {
          setRange(next)
          onChange?.(next)
        }}
        fiscalRange={{ from: '1405/01/01', to: '1405/12/29' }}
        filters={[
          {
            key: 'payment',
            label: 'وضعیت تسویه',
            value: payment,
            onChange: setPayment,
            options: [
              { value: 'all', label: 'همه‌ی وضعیت‌ها' },
              { value: 'paid', label: 'تسویه‌شده' },
            ],
          },
        ]}
        onReset={() => setRange(resolveRange('fiscalYear', { from: '1405/01/01', to: '1405/12/29' }))}
        isDefault={range.preset === 'fiscalYear'}
      />
    )
  }

  it('ف۱ — همه‌ی پیش‌تنظیم‌های مرجع به‌صورت قرص نمایش داده می‌شوند', () => {
    render(<Harness />)
    for (const preset of PRESETS) {
      expect(screen.getByRole('button', { name: preset.label })).toBeTruthy()
    }
    expect(screen.getByRole('button', { name: 'بازه سفارشی' })).toBeTruthy()
  })

  it('ف۲ — کلیک روی پیش‌تنظیم بازه را عوض می‌کند و وضعیت فشرده می‌گیرد', () => {
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)
    fireEvent.click(screen.getByRole('button', { name: 'امروز' }))
    expect(onChange).toHaveBeenCalled()
    expect(onChange.mock.calls[0][0].preset).toBe('today')
    expect(screen.getByRole('button', { name: 'امروز' }).getAttribute('aria-pressed')).toBe('true')
  })

  it('ف۳ — «سال مالی» بازه‌ی واقعی پایگاه داده را می‌گیرد، نه تقویم', () => {
    const onChange = vi.fn()
    render(<Harness onChange={onChange} />)
    fireEvent.click(screen.getByRole('button', { name: 'این ماه' }))
    fireEvent.click(screen.getByRole('button', { name: 'سال مالی' }))
    const last = onChange.mock.calls.at(-1)![0]
    expect([last.from, last.to]).toEqual(['1405/01/01', '1405/12/29'])
  })

  it('ف۴ — دکمه‌ی بازنشانی در حالت پیش‌فرض غیرفعال است', () => {
    render(<Harness />)
    const reset = screen.getByRole('button', { name: /بازنشانی/ })
    expect((reset as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'دیروز' }))
    expect((screen.getByRole('button', { name: /بازنشانی/ }) as HTMLButtonElement).disabled).toBe(
      false,
    )
  })

  it('ف۵ — بازه‌ی سفارشی شمسی است و ورودی نامعتبر را رد می‌کند', () => {
    render(<Harness />)
    fireEvent.click(screen.getByRole('button', { name: 'بازه سفارشی' }))
    const menu = screen.getByRole('menu')
    const inputs = within(menu).getAllByRole('textbox') as HTMLInputElement[]
    expect(inputs[0].placeholder).toBe('1405/01/01')
    fireEvent.change(inputs[0], { target: { value: '1405/13/01' } })
    fireEvent.click(within(menu).getByRole('button', { name: /اعمال بازه/ }))
    expect(within(menu).getByText(/قالب ۱۴۰۵/)).toBeTruthy()
  })

  it('ف۶ — هیچ ورودی تاریخ میلادی در نوار فیلتر نیست', () => {
    expect(src('components/FilterBar.tsx')).not.toContain('type="date"')
  })
})

// ---------------------------------------------------------------------------
// سنی‌سازی
// ---------------------------------------------------------------------------
describe('سنی‌سازی مطالبات', () => {
  const rows: PartyAging[] = [
    {
      contact_id: 'a',
      contact_name: 'الف',
      current: 100,
      days_1_30: 50,
      days_31_60: 0,
      days_61_90: 0,
      over_90: 10,
      total: 160,
    },
    {
      contact_id: 'b',
      contact_name: 'ب',
      current: 0,
      days_1_30: 20,
      days_31_60: 30,
      days_61_90: 0,
      over_90: 0,
      total: 50,
    },
  ]

  it('س۱ — پنج سطل مرجع ساخته می‌شود و جمعشان با کل می‌خواند', () => {
    const { buckets, total } = toBuckets(rows)
    expect(buckets.map((bucket) => bucket.label)).toEqual([
      'سررسید نشده',
      '۱ تا ۳۰ روز',
      '۳۱ تا ۶۰ روز',
      '۶۱ تا ۹۰ روز',
      'بیش از ۹۰ روز',
    ])
    expect(total).toBe(210)
    expect(buckets.reduce((sum, bucket) => sum + bucket.amount, 0)).toBe(total)
  })

  it('س۲ — «تعداد» یعنی تعداد طرف حساب دارای مبلغ در آن سطل', () => {
    const { buckets } = toBuckets(rows)
    expect(buckets[0].count).toBe(1)
    expect(buckets[1].count).toBe(2)
    expect(buckets[3].count).toBe(0)
  })

  it('س۳ — سطل‌های معوق لحن هشدار دارند', () => {
    const { buckets } = toBuckets(rows)
    expect(buckets[0].tone).toBe('ok')
    expect(buckets[4].tone).toBe('bad')
  })
})

// ---------------------------------------------------------------------------
// چیدمان داشبورد
// ---------------------------------------------------------------------------
describe('انطباق چیدمان داشبورد با مرجع', () => {
  const dashboard = src('pages/Dashboard.tsx')
  const panels = src('components/DashboardPanels.tsx')
  const data = src('pages/dashboardData.ts')

  it('چ۱ — ترتیب بخش‌ها همان ترتیب مرجع است', () => {
    const order = ['<FilterBar', 'شاخص‌های کلیدی', 'روند فروش و خرید', '<AgingPanel', '<TopParties']
    let cursor = -1
    for (const marker of order) {
      const index = dashboard.indexOf(marker)
      expect(index, `«${marker}» پیدا نشد`).toBeGreaterThan(-1)
      expect(index, `«${marker}» جای اشتباهی است`).toBeGreaterThan(cursor)
      cursor = index
    }
  })

  it('چ۲ — هشت کارت شاخص مثل مرجع', () => {
    const defs = dashboard.slice(dashboard.indexOf('const KPI_DEFS'), dashboard.indexOf('const AXIS'))
    expect(defs.match(/key: '/g)?.length).toBe(8)
  })

  it('چ۳ — شاخص‌ها به دو دسته‌ی «دوره» و «در لحظه» تفکیک شده‌اند', () => {
    const defs = dashboard.slice(dashboard.indexOf('const KPI_DEFS'), dashboard.indexOf('const AXIS'))
    expect(defs.match(/periodic: true/g)?.length).toBe(4)
    expect(defs.match(/periodic: false/g)?.length).toBe(4)
    expect(dashboard).toContain('مانده در لحظه — مستقل از بازه')
  })

  it('چ۴ — اجزای بصری مرجع (اسپارک‌لاین، نشان روند، اسکلت) استفاده شده‌اند', () => {
    for (const piece of ['Sparkline', 'TrendChip', 'Skeleton', 'EmptyState', 'ErrorState']) {
      expect(dashboard, piece).toContain(piece)
    }
  })

  it('چ۵ — پنل سنی‌سازی نوار سهم و نوار هر سطل دارد', () => {
    expect(panels).toContain('overflow-hidden rounded-full bg-bg-soft')
    expect(panels).toMatch(/style=\{\{\s*width: `\$\{\(bucket\.amount \/ share\)/)
  })

  it('چ۶ — داده‌ی دوره از گزارش همان بازه می‌آید، نه از عدد ثابت', () => {
    expect(data).toContain('getSalesReport(range.from, range.to)')
    expect(data).toContain('getPurchaseReport(range.from, range.to)')
    expect(data).toContain('previousRange(range)')
  })

  it('چ۷ — پیش‌فرض داشبورد سال مالی است', () => {
    expect(data).toContain("resolveRange('fiscalYear')")
    expect(dashboard).toContain('useFiscalPeriod')
  })

  it('چ۸ — هیچ فایلی از سقف اندازه رد نشده است', () => {
    expect(dashboard.split('\n').length).toBeLessThan(900)
    expect(panels.split('\n').length).toBeLessThan(900)
    expect(data.split('\n').length).toBeLessThan(900)
    expect(src('components/FilterBar.tsx').split('\n').length).toBeLessThan(900)
  })
})

// ---------------------------------------------------------------------------
// فهرست فاکتورها
// ---------------------------------------------------------------------------
describe('فهرست فاکتور — تصویر مرجع sFpxWK', () => {
  const invoices = src('pages/Invoices.tsx')

  it('ل۱ — ستون‌های ضروری فهرست فاکتور وجود دارند', () => {
    // بدون نام طرف حساب و انبار، کاربر نمی‌تواند فاکتور را تشخیص بدهد.
    for (const column of ['شماره', 'تاریخ', 'طرف حساب', 'انبار', 'جمع کل', 'وضعیت', 'تسویه']) {
      expect(invoices, `ستون «${column}»`).toContain(column)
    }
  })

  it('ل۲ — همه‌ی ستون‌ها قابل مرتب‌سازی‌اند', () => {
    const headers = invoices.match(/sortProps\('([a-z_]+)'\)/g) ?? []
    expect(headers.length).toBeGreaterThanOrEqual(9)
  })

  it('ل۳ — نوار فیلتر و سطر جمع دارد', () => {
    expect(invoices).toContain('<FilterBar')
    expect(invoices).toContain('total-row')
    expect(invoices).toContain('جمع سطرهای نمایش‌داده‌شده')
  })

  it('ل۴ — خروجی CSV واقعی می‌سازد، نه دکمه‌ی تزئینی', () => {
    expect(invoices).toContain('new Blob')
    expect(invoices).toContain("'\\ufeff'")
    expect(invoices).toContain('link.download')
  })

  it('ل۵ — دکمه‌ی «فاکتور جدید» به فرم صدور می‌رود', () => {
    expect(invoices).toContain("onNavigate?.('invoice-form')")
    expect(src('App.tsx')).toContain('<Invoices page={page} onNavigate={go} />')
  })
})

// ---------------------------------------------------------------------------
// قاعده‌ی «هیچ رابط کاربری ساختگی»
// ---------------------------------------------------------------------------
describe('نبود رابط کاربری ساختگی', () => {
  const files = ['pages', 'components'].flatMap((dir) =>
    readdirSync(resolve(__dirname, '..', dir))
      .filter((name) => name.endsWith('.tsx'))
      .map((name) => ({ name: `${dir}/${name}`, code: src(`${dir}/${name}`) })),
  )

  it('ک۱ — هیچ دکمه‌ای برای همیشه غیرفعال نیست', () => {
    // `disabled` بدون شرط یعنی دکمه‌ای که هرگز کار نمی‌کند.
    const offenders = files
      .filter((file) => /<button(?![^>]*disabled=\{)[^>]*\sdisabled[\s>]/.test(file.code))
      .map((file) => file.name)
    expect(offenders).toEqual([])
  })

  it('ک۲ — هیچ دکمه‌ای بدون کنش نیست', () => {
    // ویژگی‌های دکمه ممکن است چندخطی باشند و شامل `=>` هم بشوند؛ پس به‌جای
    // تطبیق تا نخستین `>`، یک بلوک از ابتدای تگ تا محتوای دکمه بررسی می‌شود.
    const offenders: string[] = []
    for (const file of files) {
      for (const match of file.code.matchAll(/<button\b/g)) {
        const block = file.code.slice(match.index, match.index + 600)
        const head = block.slice(0, block.indexOf('</button>') + 1 || 600)
        if (/onClick|onMouseDown|onPointerDown|type="submit"/.test(head)) continue
        // دکمه‌ی بدون `type` داخل فرم، پیش‌فرض submit است.
        if (!head.includes('type=')) continue
        offenders.push(`${file.name}: ${head.replace(/\s+/g, ' ').slice(0, 70)}`)
      }
    }
    expect(offenders).toEqual([])
  })
})

// ---------------------------------------------------------------------------
// دستورهای ریشه‌ی مخزن
// ---------------------------------------------------------------------------
describe('دستورهای ریشه‌ی مخزن', () => {
  /**
   * چرا این تست وجود دارد: اولین کاری که هر کسی پس از کلون می‌کند تایپ
   * `npm run dev` در ریشه است. اگر آنجا package.json نباشد، با خطای
   * ENOENT روبه‌رو می‌شود — و این بدترین اولین برخورد ممکن است.
   */
  const rootPkg = JSON.parse(
    readFileSync(resolve(__dirname, '..', '..', '..', '..', 'package.json'), 'utf8'),
  ) as { scripts: Record<string, string>; private: boolean }

  const uiPkg = JSON.parse(
    readFileSync(resolve(__dirname, '..', '..', 'package.json'), 'utf8'),
  ) as { scripts: Record<string, string> }

  it('ر۱ — ریشه‌ی مخزن package.json دارد', () => {
    expect(rootPkg.private).toBe(true)
    expect(Object.keys(rootPkg.scripts).length).toBeGreaterThan(6)
  })

  it('ر۲ — دستورهای پرکاربرد از ریشه در دسترس‌اند', () => {
    for (const script of ['setup', 'dev', 'build', 'test', 'typecheck', 'check', 'installer']) {
      expect(rootPkg.scripts[script], `دستور ${script}`).toBeTruthy()
    }
  })

  it('ر۳ — هر دستور واگذارشده، در بسته‌ی مقصد واقعاً وجود دارد', () => {
    const delegated = Object.values(rootPkg.scripts)
      .map((command) => /npm --prefix apps\/desktop-ui run ([a-z:]+)/.exec(command)?.[1])
      .filter((name): name is string => Boolean(name))
    expect(delegated.length).toBeGreaterThan(2)
    for (const name of delegated) {
      expect(uiPkg.scripts[name], `apps/desktop-ui فاقد دستور ${name}`).toBeTruthy()
    }
  })

  it('ر۴ — ابزار Tauri با مسیر مستقل از سیستم‌عامل صدا زده می‌شود', () => {
    // اسلش رو به جلو را node روی ویندوز هم می‌فهمد؛ cmd.exe نه.
    for (const key of ['desktop', 'installer', 'tauri:info']) {
      expect(rootPkg.scripts[key]).toContain(
        'node apps/desktop-ui/node_modules/@tauri-apps/cli/tauri.js',
      )
    }
  })

  it('ر۵ — ریشه هیچ وابستگی ندارد تا نیازی به نصب جداگانه نباشد', () => {
    const raw = rootPkg as unknown as Record<string, unknown>
    expect(raw.dependencies).toBeUndefined()
    expect(raw.devDependencies).toBeUndefined()
  })

  it('ر۶ — راهنما همان دستورهای موجود را معرفی می‌کند', () => {
    const guide = readFileSync(resolve(__dirname, '..', '..', '..', '..', 'BUILD_WINDOWS.md'), 'utf8')
    for (const script of ['setup', 'dev', 'build', 'installer']) {
      expect(guide, `راهنما به npm run ${script} اشاره نکرده`).toContain(`npm run ${script}`)
    }
    // `npm test` اصطلاح رسمی npm است و نیازی به `run` ندارد.
    expect(guide).toContain('npm test')
    // راهنما نباید کاربر را به پوشه‌ی دیگری بفرستد.
    expect(guide).not.toContain('cd apps\\desktop-ui')
  })
})
