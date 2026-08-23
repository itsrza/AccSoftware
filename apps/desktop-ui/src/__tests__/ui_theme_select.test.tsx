/**
 * @vitest-environment jsdom
 *
 * ممیزی رابط کاربری — تم، دراپ‌داون و ناوبری.
 *
 * این فایل چیزهایی را می‌سنجد که تست‌های متنی نمی‌توانند: رفتار واقعی DOM.
 * هر تست به یکی از ایرادهای گزارش‌شده‌ی کاربر گره خورده تا اگر دوباره
 * برگشت، همین‌جا قرمز شود.
 */
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, fireEvent, cleanup, within } from '@testing-library/react'
import { useState } from 'react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { Select } from '../components/Select'
import { CommandPalette } from '../components/CommandPalette'

const src = (p: string) => readFileSync(resolve(__dirname, '..', p), 'utf8')

afterEach(cleanup)

// ---------------------------------------------------------------------------
// دراپ‌داون
// ---------------------------------------------------------------------------
describe('دراپ‌داون سیستم طراحی', () => {
  const Basic = ({ onPick }: { onPick?: (v: string) => void }) => {
    const [value, setValue] = useState('')
    return (
      <Select
        value={value}
        aria-label="نوع چک"
        onChange={(e) => {
          setValue(e.target.value)
          onPick?.(e.target.value)
        }}
      >
        <option value="">همه</option>
        <option value="received">دریافتی</option>
        <option value="issued">پرداختی</option>
      </Select>
    )
  }

  it('د۱ — به‌جای عنصر بومی select، دکمه‌ی combobox رندر می‌کند', () => {
    const { container } = render(<Basic />)
    expect(container.querySelector('select')).toBeNull()
    expect(screen.getByRole('combobox', { name: 'نوع چک' })).toBeTruthy()
  })

  it('د۲ — فهرست فقط پس از کلیک باز می‌شود', () => {
    render(<Basic />)
    expect(screen.queryByRole('listbox')).toBeNull()
    fireEvent.click(screen.getByRole('combobox'))
    expect(screen.getByRole('listbox')).toBeTruthy()
  })

  it('د۳ — انتخاب گزینه مقدار را برمی‌گرداند و فهرست را می‌بندد', () => {
    const onPick = vi.fn()
    render(<Basic onPick={onPick} />)
    fireEvent.click(screen.getByRole('combobox'))
    fireEvent.click(screen.getByRole('option', { name: /پرداختی/ }))
    expect(onPick).toHaveBeenCalledWith('issued')
    expect(screen.queryByRole('listbox')).toBeNull()
    expect(screen.getByRole('combobox').textContent).toContain('پرداختی')
  })

  it('د۴ — پیمایش با صفحه‌کلید کار می‌کند (پایین + Enter)', () => {
    const onPick = vi.fn()
    render(<Basic onPick={onPick} />)
    const button = screen.getByRole('combobox')
    fireEvent.keyDown(button, { key: 'ArrowDown' })
    fireEvent.keyDown(button, { key: 'ArrowDown' })
    fireEvent.keyDown(button, { key: 'Enter' })
    expect(onPick).toHaveBeenCalledWith('received')
  })

  it('د۵ — Escape بدون تغییر مقدار می‌بندد', () => {
    const onPick = vi.fn()
    render(<Basic onPick={onPick} />)
    fireEvent.click(screen.getByRole('combobox'))
    fireEvent.keyDown(screen.getByRole('combobox'), { key: 'Escape' })
    expect(screen.queryByRole('listbox')).toBeNull()
    expect(onPick).not.toHaveBeenCalled()
  })

  it('د۶ — مقدار برای FormData با input مخفی حفظ می‌شود', () => {
    const { container } = render(
      <form>
        <Select name="kind" defaultValue="received">
          <option value="received">دریافتی</option>
          <option value="issued">پرداختی</option>
        </Select>
      </form>,
    )
    const form = container.querySelector('form')!
    expect(new FormData(form).get('kind')).toBe('received')
    fireEvent.click(screen.getByRole('combobox'))
    fireEvent.click(screen.getByRole('option', { name: /پرداختی/ }))
    expect(new FormData(form).get('kind')).toBe('issued')
  })

  it('د۷ — گزینه‌ی غیرفعال («انتخاب کنید») قابل انتخاب نیست', () => {
    const onPick = vi.fn()
    render(
      <Select onChange={(e) => onPick(e.target.value)}>
        <option value="" disabled>
          انتخاب کنید…
        </option>
        <option value="a">الف</option>
      </Select>,
    )
    fireEvent.click(screen.getByRole('combobox'))
    fireEvent.click(screen.getByRole('option', { name: /انتخاب کنید/ }))
    expect(onPick).not.toHaveBeenCalled()
  })

  it('د۸ — گروه‌بندی optgroup عنوان گروه را نشان می‌دهد', () => {
    render(
      <Select defaultValue="a">
        <optgroup label="دارایی">
          <option value="a">موجودی نقد</option>
        </optgroup>
        <optgroup label="بدهی">
          <option value="b">حساب‌های پرداختنی</option>
        </optgroup>
      </Select>,
    )
    fireEvent.click(screen.getByRole('combobox'))
    const list = screen.getByRole('listbox')
    expect(within(list).getByText('دارایی')).toBeTruthy()
    expect(within(list).getByText('بدهی')).toBeTruthy()
  })

  it('د۹ — مقدار عددی گزینه‌ها پشتیبانی می‌شود (نرخ مالیات)', () => {
    const onPick = vi.fn()
    render(
      <Select value={0} onChange={(e) => onPick(e.target.value)}>
        <option value={0}>معاف</option>
        <option value={900}>۹٪</option>
      </Select>,
    )
    expect(screen.getByRole('combobox').textContent).toContain('معاف')
    fireEvent.click(screen.getByRole('combobox'))
    fireEvent.click(screen.getByRole('option', { name: /۹٪/ }))
    expect(onPick).toHaveBeenCalledWith('900')
  })

  it('د۱۰ — حالت غیرفعال باز نمی‌شود', () => {
    render(
      <Select disabled defaultValue="a">
        <option value="a">الف</option>
      </Select>,
    )
    fireEvent.click(screen.getByRole('combobox'))
    expect(screen.queryByRole('listbox')).toBeNull()
  })

  it('د۱۰.الف — فهرست با پورتال روی body باز می‌شود تا هیچ overflow آن را نبُرد', () => {
    // دراپ‌داون‌های داخل پاپ‌آپ و جدول‌های اسکرول‌دار قبلاً بریده می‌شدند.
    const { container } = render(
      <div style={{ overflow: 'hidden', height: 20 }}>
        <Select defaultValue="a">
          <option value="a">الف</option>
          <option value="b">ب</option>
        </Select>
      </div>,
    )
    fireEvent.click(screen.getByRole('combobox'))
    const list = screen.getByRole('listbox')
    expect(container.contains(list)).toBe(false)
    expect(document.body.contains(list)).toBe(true)
    expect(list.style.position).toBe('fixed')
  })

  it('د۱۱ — هیچ صفحه‌ای دیگر select بومی ندارد', () => {
    const files = [
      'pages/Checks.tsx',
      'pages/InvoiceForm.tsx',
      'pages/PartyForm.tsx',
      'pages/ReportBuilder.tsx',
      'pages/TreasuryDocumentForm.tsx',
      'components/SettingsCenter.tsx',
    ]
    for (const file of files) {
      expect(src(file), `${file} هنوز select بومی دارد`).not.toMatch(/<select[\s>]/)
    }
  })
})

// ---------------------------------------------------------------------------
// تم روشن و تیره
// ---------------------------------------------------------------------------
describe('تم روشن و تیره', () => {
  const design = src('design-system.css')
  const styles = src('styles.css')
  const theme = src('theme.css')

  it('ت۱ — ترتیب لایه‌ها CSS قدیمی را زیر کلاس‌های کمکی می‌گذارد', () => {
    const order = design.match(/@layer\s+([a-z,\s]+);/)![1]
    const names = order.split(',').map((s) => s.trim())
    expect(names.indexOf('legacy')).toBeGreaterThan(names.indexOf('base'))
    expect(names.indexOf('legacy')).toBeLessThan(names.indexOf('utilities'))
  })

  it('ت۲ — CSS قدیمی داخل لایه‌ی legacy وارد می‌شود', () => {
    expect(design).toMatch(/@import\s+"\.\/styles\.css"\s+layer\(legacy\)/)
    expect(design).toMatch(/@import\s+"\.\/theme\.css"\s+layer\(legacy\)/)
  })

  it('ت۳ — هیچ چرخه‌ای بین توکن‌های رنگ نیست', () => {
    // نگاشتی از نام توکن به مقدار خام، فقط از تعریف‌های :root.
    const defs = new Map<string, string>()
    const strip = (css: string) => css.replace(/\/\*[\s\S]*?\*\//g, '')
    for (const file of [design, styles, theme].map(strip)) {
      for (const m of file.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/g)) {
        defs.set(m[1], m[2].trim())
      }
    }
    const seen = new Set<string>()
    const resolveToken = (name: string, stack: string[]): void => {
      if (stack.includes(name)) throw new Error(`چرخه: ${[...stack, name].join(' → ')}`)
      const value = defs.get(name)
      if (!value) return
      for (const ref of value.matchAll(/var\(--([a-z0-9-]+)/g)) {
        resolveToken(ref[1], [...stack, name])
      }
      seen.add(name)
    }
    expect(() => defs.forEach((_, key) => resolveToken(key, []))).not.toThrow()
    expect(seen.size).toBeGreaterThan(20)
  })

  it('ت۴ — کلاس dark روی ریشه‌ی سند اعمال می‌شود، نه یک div داخلی', () => {
    const app = src('App.tsx')
    expect(app).toMatch(/document\.documentElement[\s\S]{0,120}classList\.toggle\('dark'/)
    expect(app).not.toMatch(/cn\('min-h-screen', dark && 'dark'\)/)
  })

  it('ت۵ — color-scheme هم با تم عوض می‌شود تا کنترل‌های بومی هماهنگ بمانند', () => {
    expect(src('App.tsx')).toMatch(/colorScheme = dark \? 'dark' : 'light'/)
  })

  it('ت۶ — styles.css دیگر رنگ ثابت ندارد', () => {
    expect(styles.match(/#[0-9a-fA-F]{3,8}/g)).toBeNull()
  })

  it('ت۷ — هر توکن روشن معادل تیره دارد', () => {
    const grab = (block: string) =>
      new Set(Array.from(block.matchAll(/--([a-z0-9-]+)\s*:/g)).map((m) => m[1]))
    const light = design.slice(design.indexOf(':root {'), design.indexOf('.dark {'))
    const dark = design.slice(design.indexOf('.dark {'), design.indexOf('@theme inline'))
    const lightTokens = grab(light)
    const darkTokens = grab(dark)
    const missing = [...lightTokens].filter((t) => !darkTokens.has(t) && !t.startsWith('radius'))
    expect(missing).toEqual([])
  })

  it('ت۸ — پوسته‌ی پاپ‌آپ پس‌زمینه‌ی مات دارد (باگ پاپ‌آپ سرمه‌ای فاکتور)', () => {
    const modal = theme.slice(theme.indexOf('.modal {'), theme.indexOf('.modal {') + 400)
    expect(modal).toMatch(/background:\s*var\(--surface\)/)
    expect(modal).toMatch(/color:\s*var\(--text\)/)
  })

  it('ت۹.الف — متن روی سطح رنگی توکن اختصاصی دارد', () => {
    // بدون --on-primary، دکمه‌ی اصلی در تم تیره سفیدِ روی زمینه‌ی روشن می‌شد.
    for (const token of ['--on-primary', '--on-accent', '--on-danger']) {
      const light = design.slice(design.indexOf(':root {'), design.indexOf('.dark {'))
      const dark = design.slice(design.indexOf('.dark {'), design.indexOf('@theme inline'))
      expect(light, `${token} در تم روشن`).toContain(token)
      expect(dark, `${token} در تم تیره`).toContain(token)
    }
    // و هیچ سطح رنگی‌ای رنگ متنش را از --surface نمی‌گیرد.
    expect(styles).not.toMatch(/background:var\(--brand\);color:var\(--surface\)/)
  })

  it('ت۹.ب — CSS منوی کناری قدیمی حذف شده است', () => {
    const rules = styles.replace(/\/\*[\s\S]*?\*\//g, '')
    for (const dead of ['.nav-item', '.subnav ', '.company-switch', '.storage ']) {
      expect(rules, `${dead} کد مرده است`).not.toContain(dead)
    }
  })

  it('ت۹ — پوشش مرکز تنظیمات مات است تا صفحه‌ی زیرین دیده نشود', () => {
    expect(theme).toMatch(/\.settings-overlay\s*\{[^}]*background:\s*var\(--bg\)/)
    expect(styles).toMatch(/\.settings-overlay\{[^}]*background:var\(--bg\)/)
  })
})

// ---------------------------------------------------------------------------
// ناوبری
// ---------------------------------------------------------------------------
describe('ناوبری', () => {
  const app = src('App.tsx')

  it('ن۱ — قالب‌های چاپ و اتصالات از منوی کناری حذف شده‌اند', () => {
    const menu = app.slice(app.indexOf('const MENU'), app.indexOf('const PAGE_TITLES'))
    expect(menu).not.toContain("page: 'print-templates'")
    expect(menu).not.toContain("page: 'integrations'")
  })

  it('ن۲ — اما هر دو صفحه هنوز مسیر و عنوان دارند', () => {
    expect(app).toContain("case 'print-templates'")
    expect(app).toContain("case 'integrations'")
    expect(app).toContain("'print-templates': 'قالب‌های چاپ'")
  })

  it('ن۳ — مرکز تنظیمات به آن دو صفحه پل می‌زند', () => {
    const settings = src('components/SettingsCenter.tsx')
    expect(settings).toContain("navigate('print-templates')")
    expect(settings).toContain("navigate('integrations')")
  })

  it('ن۴ — پالت فرمان فقط شناسه‌ی صفحه‌های واقعی دارد', () => {
    const palette = src('components/CommandPalette.tsx')
    const ids = Array.from(palette.matchAll(/\{id: '([a-z-]+)'/g)).map((m) => m[1])
    const routes = new Set(Array.from(app.matchAll(/case '([a-z-]+)':/g)).map((m) => m[1]))
    routes.add('sales')
    routes.add('purchase')
    routes.add('proforma')
    const orphans = ids.filter((id) => !routes.has(id))
    expect(orphans).toEqual([])
  })

  it('ن۵ — پالت فرمان با جهت‌نما و Enter انتخاب می‌کند', () => {
    const onSelect = vi.fn()
    render(<CommandPalette open onClose={() => undefined} onSelect={onSelect} />)
    fireEvent.keyDown(window, { key: 'ArrowDown' })
    fireEvent.keyDown(window, { key: 'Enter' })
    expect(onSelect).toHaveBeenCalledWith('invoice-form')
  })

  it('ن۶ — زیرمنوهای سایدبار متن دارند و رنگ‌شان از توکن سایدبار می‌آید', () => {
    const sidebar = src('components/Sidebar.tsx')
    expect(sidebar).toMatch(/\{child\.label\}/)
    expect(sidebar).toMatch(/text-\[var\(--sidebar-text\)\]/)
  })
})

// ---------------------------------------------------------------------------
// ورود و خروج اطلاعات
// ---------------------------------------------------------------------------
describe('ورود و خروج اطلاعات', () => {
  const page = src('pages/DataTools.tsx')

  it('و۱ — نمونه‌ی نمایشی پیش از انتخاب فایل وجود دارد', () => {
    expect(page).toContain('نمونه‌ی نمایشی')
    expect(page).toMatch(/sample: \[/)
  })

  it('و۲ — نمونه صریحاً «ثبت نمی‌شود» علامت خورده است', () => {
    expect(page).toContain('هرگز ثبت نمی‌شوند')
  })

  it('و۳ — دریافت فایل CSV نمونه ممکن است', () => {
    expect(page).toContain('دریافت فایل نمونه')
    expect(page).toMatch(/function sampleCsv/)
  })

  it('و۴ — ستون اجباری غایب، ورود را مسدود می‌کند', () => {
    expect(page).toMatch(/disabled=\{busy \|\| missing\.length > 0\}/)
  })

  it('و۵ — نمونه با BOM تولید می‌شود تا اکسل فارسی را درست بخواند', () => {
    expect(page).toContain("'\\ufeff'")
  })
})

// ---------------------------------------------------------------------------
// پاپ‌آپ‌ها
// ---------------------------------------------------------------------------
describe('رفتار بستن پاپ‌آپ‌ها', () => {
  /** فرم‌های ورود داده: کلیک بیرون نباید کار نیمه‌تمام را دور بیندازد. */
  const DATA_ENTRY = [
    'pages/InvoiceForm.tsx',
    'pages/PartyForm.tsx',
    'pages/Quotes.tsx',
    'pages/Stocktaking.tsx',
    'pages/ChartOfAccounts.tsx',
    'pages/ProductionFormulaDialogs.tsx',
    'pages/DataPage.tsx',
    'pages/Operations.tsx',
    'pages/AdvancedInventory.tsx',
    'pages/Treasury.tsx',
    'pages/TreasuryAccounts.tsx',
  ]

  it('پ۱ — فرم افزودن کالای فاکتور فقط با دکمه بسته می‌شود', () => {
    const code = src('pages/InvoiceForm.tsx')
    expect(code).toContain('<div className="modal-backdrop" role="presentation">')
    expect(code).not.toMatch(/modal-backdrop"\s+onClick/)
    // ولی راه بستن باید وجود داشته باشد: هم دکمه‌ی بستن، هم انصراف.
    expect(code).toContain('aria-label="بستن"')
    expect(code).toMatch(/>\s*انصراف\s*</)
  })

  it('پ۲ — هیچ فرم ورود داده‌ای با کلیک روی پس‌زمینه بسته نمی‌شود', () => {
    // تنها پاپ‌آپ‌هایی حق دارند با کلیک بیرون بسته شوند که «نمایش جزئیات»
    // باشند؛ نام state آن‌ها این را نشان می‌دهد.
    const readOnlyClosers = /setDetail\(|setSelected\(|setFormulaDetail\(/
    const offenders: string[] = []
    for (const file of DATA_ENTRY) {
      for (const match of src(file).matchAll(/modal-backdrop"\s+onClick=\{([^}]*)\}/g)) {
        if (!readOnlyClosers.test(match[1])) offenders.push(`${file}: ${match[1]}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('پ۲.ب — هر فایل حداقل یک پاپ‌آپ غیرقابل‌بستن با کلیک بیرون دارد', () => {
    const offenders = DATA_ENTRY.filter(
      (file) => !src(file).includes('modal-backdrop" role="presentation"'),
    )
    expect(offenders).toEqual([])
  })

  it('پ۳ — هر فرم ورود داده راه بستن صریح دارد', () => {
    const offenders = DATA_ENTRY.filter((file) => {
      const code = src(file)
      return !/انصراف/.test(code) && !/aria-label="بستن"/.test(code)
    })
    expect(offenders).toEqual([])
  })

  it('پ۴ — پاپ‌آپ‌های فقط-خواندنی همچنان با کلیک بیرون بسته می‌شوند', () => {
    // بستن تصادفی یک «نمایش جزئیات» چیزی را از بین نمی‌برد؛ اینجا سرعت مهم‌تر است.
    for (const file of ['pages/Checks.tsx', 'pages/Returns.tsx']) {
      expect(src(file), file).toMatch(/modal-backdrop"\s+onClick/)
    }
  })
})

// ---------------------------------------------------------------------------
// کامل بودن داده‌ی نمونه‌ی پیش‌نمایش
// ---------------------------------------------------------------------------
describe('داده‌ی نمونه‌ی پیش‌نمایش', () => {
  const apiSource = src('api.ts')
  const preview = src('lib/devPreview.ts')
  const extras = src('lib/preview/extras.ts')

  const commands = Array.from(
    new Set([
      ...Array.from(apiSource.matchAll(/api<[^>]*>\('([a-z0-9_]+)'/g)).map((m) => m[1]),
      ...Array.from(apiSource.matchAll(/\bapi\('([a-z0-9_]+)'/g)).map((m) => m[1]),
    ]),
  )

  const simulated = new Set([
    ...Array.from(preview.matchAll(/^ {2}([a-z0-9_]+):/gm)).map((m) => m[1]),
    ...Array.from(extras.matchAll(/^ {4}([a-z0-9_]+):/gm)).map((m) => m[1]),
  ])

  it('ن۱ — هر فرمانی که رابط کاربری صدا می‌زند در پیش‌نمایش پاسخ دارد', () => {
    const missing = commands.filter((command) => !simulated.has(command))
    expect(missing, 'فرمان بدون پاسخ باعث می‌شود صفحه خالی بماند').toEqual([])
    expect(commands.length).toBeGreaterThan(150)
  })

  it('ن۲ — بخش‌های ابزار خالی برنمی‌گردند', () => {
    for (const marker of [
      'list_print_templates',
      'list_custom_reports',
      'list_api_profiles',
      'list_plugins',
      'list_backups',
      'list_permissions',
    ]) {
      expect(extras, `${marker} باید محتوای نمونه بدهد`).toContain(marker)
    }
    expect(extras).not.toMatch(/list_print_templates: \(\) => \[\]/)
  })

  it('ن۳ — گزارش‌ها از همان فاکتورهای دمو ساخته می‌شوند، نه عدد ثابت', () => {
    expect(extras).toMatch(/get_sales_report:[\s\S]{0,200}salesInvoices/)
    expect(extras).toMatch(/get_purchase_report:[\s\S]{0,200}purchaseInvoices/)
  })

  it('ن۴ — شبیه‌ساز از ماژول جدا می‌آید تا فایل غول‌پیکر نشود', () => {
    expect(preview).toContain("from './preview/extras'")
    expect(preview.split('\n').length).toBeLessThan(1500)
    expect(extras.split('\n').length).toBeLessThan(700)
  })
})
