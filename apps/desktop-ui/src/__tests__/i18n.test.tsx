/**
 * @vitest-environment jsdom
 */
import { describe, expect, it, afterEach } from 'vitest'
import { render, act, cleanup } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  I18nProvider,
  LOCALES,
  ar,
  directionOf,
  en,
  fa,
  interpolate,
  translate,
  useI18n,
  type TranslationKey,
} from '../lib/i18n'
import {
  formatCount,
  formatNumber,
  formatRialsWithUnit,
  numberLocale,
  percentSign,
  rialUnit,
  setNumberLocale,
} from '../lib/format'

/**
 * ممیزی چندزبانی — ده تست سخت‌گیرانه.
 *
 * قاعده‌ای که این پرونده نگهبانش است: **زبان تازه نباید متن جامانده بسازد.**
 * دیکشنری‌ها باید هم‌شکل باشند، متغیرها باید در هر سه زبان یکی باشند و
 * ارقام باید با زبان عوض شوند — چون «۱۲۳» در یک صفحه‌ی انگلیسی، ایراد
 * ترجمه نیست، ایراد محصول است.
 */

const SRC = resolve(__dirname, '..')
const read = (relative: string) => readFileSync(resolve(SRC, relative), 'utf8')

afterEach(() => {
  cleanup()
  setNumberLocale('fa')
})

const keys = Object.keys(fa) as TranslationKey[]

describe('چندزبانی — ساختار دیکشنری', () => {
  it('ز۱ — هر سه زبان دقیقاً یک مجموعه کلید دارند', () => {
    expect(Object.keys(en).sort()).toEqual(keys.slice().sort())
    expect(Object.keys(ar).sort()).toEqual(keys.slice().sort())
  })

  it('ز۲ — هیچ ترجمه‌ای خالی یا تکرار عین کلید نیست', () => {
    const broken: string[] = []
    for (const key of keys) {
      for (const [name, dictionary] of [
        ['fa', fa],
        ['en', en],
        ['ar', ar],
      ] as const) {
        const value = dictionary[key]
        if (!value.trim() || value === key) broken.push(`${name}:${key}`)
      }
    }
    expect(broken).toEqual([])
  })

  it('ز۳ — متغیرهای {name} در هر سه زبان یکسان‌اند', () => {
    const placeholders = (text: string) =>
      [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort()
    const mismatched = keys.filter((key) => {
      const base = JSON.stringify(placeholders(fa[key]))
      return (
        JSON.stringify(placeholders(en[key])) !== base ||
        JSON.stringify(placeholders(ar[key])) !== base
      )
    })
    expect(mismatched).toEqual([])
  })

  it('ز۴ — ترجمه‌ی انگلیسی واقعاً انگلیسی است (نه فارسی کپی‌شده)', () => {
    // استثناها: نشانه‌های صفحه‌کلید و نویسه‌های تزئینی، حرف عربی/فارسی ندارند.
    const persian = keys.filter((key) => /[\u0600-\u06FF]/.test(en[key]))
    expect(persian).toEqual([])
  })

  it('ز۵ — ترجمه‌ی عربی از واژگان حسابداری عربی استفاده می‌کند', () => {
    // چند اصطلاح کلیدی که ترجمه‌ی تحت‌اللفظی‌شان غلط حرفه‌ای است.
    expect(ar['dashboard.kpi.receivables']).toBe('الذمم المدينة')
    expect(ar['dashboard.kpi.payables']).toBe('الذمم الدائنة')
    expect(ar['nav.journals']).toBe('قيود اليومية')
    expect(ar['page.chart-of-accounts']).toBe('دليل الحسابات')
    expect(en['nav.journals']).toBe('Journal vouchers')
    expect(en['page.chart-of-accounts']).toBe('Chart of accounts')
  })
})

describe('چندزبانی — رفتار زمان اجرا', () => {
  it('ز۶ — ارقام هر زبان به خط همان زبان نوشته می‌شوند', () => {
    setNumberLocale('fa')
    expect(formatCount(1234)).toBe('۱٬۲۳۴')
    setNumberLocale('ar')
    expect(formatCount(1234)).toBe('١٬٢٣٤')
    setNumberLocale('en')
    expect(formatCount(1234)).toBe('1,234')
    expect(numberLocale()).toBe('en')
    // اعشار هم باید همان خط را بگیرد، نه فقط اعداد صحیح.
    setNumberLocale('fa')
    expect(formatNumber(12.5)).toContain('۱۲')
  })

  it('ز۷ — واحد پول و نشانه‌ی درصد با زبان عوض می‌شوند', () => {
    setNumberLocale('fa')
    expect(rialUnit()).toBe('ریال')
    expect(percentSign()).toBe('٪')
    expect(formatRialsWithUnit(1000)).toBe('۱٬۰۰۰ ریال')
    setNumberLocale('ar')
    expect(rialUnit()).toBe('ريال')
    setNumberLocale('en')
    expect(rialUnit()).toBe('IRR')
    expect(percentSign()).toBe('%')
    expect(formatRialsWithUnit(1000)).toBe('1,000 IRR')
  })

  it('ز۸ — جهت صفحه از زبان مشتق می‌شود و روی <html> می‌نشیند', () => {
    expect(directionOf('fa')).toBe('rtl')
    expect(directionOf('ar')).toBe('rtl')
    expect(directionOf('en')).toBe('ltr')
    expect(LOCALES.map((item) => item.code)).toEqual(['fa', 'en', 'ar'])

    function Probe() {
      const { locale, setLocale, t } = useI18n()
      return (
        <button onClick={() => setLocale('en')}>
          {locale}:{t('page.dashboard')}
        </button>
      )
    }
    const view = render(
      <I18nProvider initialLocale="fa">
        <Probe />
      </I18nProvider>,
    )
    expect(document.documentElement.dir).toBe('rtl')
    expect(view.getByRole('button').textContent).toBe('fa:داشبورد')
    act(() => {
      view.getByRole('button').click()
    })
    expect(document.documentElement.dir).toBe('ltr')
    expect(document.documentElement.lang).toBe('en')
    expect(view.getByRole('button').textContent).toBe('en:Dashboard')
  })

  it('ز۹ — جای‌گذاری متغیر و بازگشت امن به فارسی', () => {
    expect(interpolate('{a} از {b}', { a: 2, b: 5 })).toBe('2 از 5')
    // متغیر ناشناخته دست‌نخورده می‌ماند تا خطا در چشم بیاید، نه اینکه بی‌صدا حذف شود.
    expect(interpolate('{a} و {c}', { a: 1 })).toBe('1 و {c}')
    expect(translate('en', 'alert.overdueChecks', { count: 3 })).toBe('3 overdue cheques')
    // کلید ناموجود هرگز به کاربر نشان داده نمی‌شود مگر اینکه هیچ زبانی نداشته باشد.
    expect(translate('en', 'nothing.here' as TranslationKey)).toBe('nothing.here')
  })

  it('ز۱۰ — پوسته‌ی برنامه هیچ متن سخت‌کدشده‌ای ندارد و هر صفحه کلید عنوان دارد', () => {
    const app = read('App.tsx')
    const routed = [...app.matchAll(/case '([a-z0-9-]+)':/g)].map((match) => match[1])
    const missing = routed.filter((page) => !(`page.${page}` in fa))
    expect(missing).toEqual([])

    // منو، عنوان صفحه و عملیات سریع باید فقط کلید ترجمه داشته باشند.
    const menu = app.slice(app.indexOf('const MENU'), app.indexOf('function pageTitleKey'))
    expect(menu).not.toMatch(/label: '[^']*[\u0600-\u06FF]/)
    const quick = app.slice(app.indexOf('const QUICK_ACTIONS'), app.indexOf('\n]', app.indexOf('const QUICK_ACTIONS')))
    expect(quick).not.toMatch(/label: '[^']*[\u0600-\u06FF]/)

    // نوار بالا، منوی کناری و پالت فرمان از `useI18n` استفاده می‌کنند.
    for (const file of ['components/Topbar.tsx', 'components/Sidebar.tsx', 'components/CommandPalette.tsx']) {
      expect(read(file), file).toContain('useI18n')
    }
  })
})
