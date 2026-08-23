/**
 * ممیزی رابط کاربری — دراپ‌داون‌ها، سیستم طراحی و قراردادهای فرم.
 *
 * ## چرا این تست‌ها روی فایل منبع کار می‌کنند
 *
 * ادعاهایی مثل «همه‌ی دراپ‌داون‌ها با استایل قالب طراحی شده‌اند» یا «هیچ
 * فرمی گزینه‌ی ثابت hardcode ندارد» را نمی‌شود با رندر یک صفحه سنجید —
 * باید **کل کد** بررسی شود. پس این ممیزی فایل‌های منبع را می‌خواند و
 * الگوهای ممنوع را پیدا می‌کند.
 *
 * ## قاعده‌ی ثابت
 *
 * هر تست یک **قانون سراسری** را می‌سنجد، نه یک نمونه. نقض قانون در حتی یک
 * فایل، یعنی کاربر در همان یک صفحه تجربه‌ی متفاوت می‌بیند — و همان‌جاست که
 * اعتمادش را از دست می‌دهد.
 */
import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { CHECK_STATUS_LABELS, checkStatusLabel, checkStatusTone, isOpenCheck } from '../lib/checkStatus'
import { allowedAggregations } from '../lib/reportEngine'
import {
  formatJalali,
  formatNumber,
  formatPercent,
  formatRials,
  formatTomans,
  normalizeDigits,
  parseAmount,
  todayJalali,
  tomansToRials,
} from '../lib/format'
import { cn } from '../lib/cn'

const SRC = join(__dirname, '..')
const PAGES = join(SRC, 'pages')
const COMPONENTS = join(SRC, 'components')

const readAll = (dir: string) =>
  readdirSync(dir)
    .filter((name) => name.endsWith('.tsx'))
    .map((name) => ({ name, code: readFileSync(join(dir, name), 'utf8') }))

const pageFiles = readAll(PAGES)
const componentFiles = readAll(COMPONENTS)
const allFiles = [...pageFiles, ...componentFiles]

const designSystem = readFileSync(join(SRC, 'design-system.css'), 'utf8')
const theme = readFileSync(join(SRC, 'theme.css'), 'utf8')
const styles = readFileSync(join(SRC, 'styles.css'), 'utf8')
const allCss = `${designSystem}\n${theme}\n${styles}`

// ===========================================================================
// دراپ‌داون‌ها
// ===========================================================================

describe('دراپ‌داون‌ها', () => {
  it('ت۷۶ — استایل دراپ‌داون یک بار و سراسری تعریف شده است', () => {
    // ظاهر پیش‌فرض سیستم‌عامل باید خاموش شده باشد.
    expect(allCss).toMatch(/select\s*\{[^}]*appearance:\s*none/s)
    // و فلش سفارشی داشته باشد.
    expect(allCss).toContain('background-image: url("data:image/svg+xml')
  })

  it('ت۷۷ — فلش دراپ‌داون در تم تیره رنگ جدا دارد', () => {
    // بدون این، فلش تیره روی پس‌زمینه‌ی تیره دیده نمی‌شود.
    expect(allCss).toMatch(/\.dark\s+select\s*\{[^}]*background-image/s)
  })

  it('ت۷۸ — گزینه‌های دراپ‌داون در تم تیره خوانا هستند', () => {
    // مرورگرها به `option` رنگ سیستمی می‌دهند؛ باید صریحاً تنظیم شود.
    expect(allCss).toMatch(/select\s+option\s*\{[^}]*color:\s*var\(--text\)/s)
  })

  it('ت۷۹ — دراپ‌داون غیرفعال ظاهر متمایز دارد', () => {
    expect(allCss).toMatch(/select:disabled\s*\{[^}]*cursor:\s*not-allowed/s)
  })

  it('ت۸۰ — دراپ‌داون انتخاب موجودیت، گزینه‌ی راهنما دارد', () => {
    // تفکیک مهم:
    //  · انتخاب **موجودیت** (کالا، انبار، شخص، حساب) باید با گزینه‌ی خالی
    //    شروع شود؛ وگرنه کاربر ناخواسته اولین رکورد فهرست را ثبت می‌کند و
    //    فاکتور به انبار اشتباه می‌خورد.
    //  · انتخاب **حالت** (ماهیت حساب، نوع شخصیت، ماه، روش تخصیص) پیش‌فرض
    //    منطقی دارد و گزینه‌ی خالی در آن فقط مزاحم است.
    const entitySources = /\b(products|warehouses|parties|accounts|ledger|rows|groups|routes|marketers|invoices|formulas|contacts)\.(map|filter)/
    const offenders: string[] = []
    for (const { name, code } of allFiles) {
      for (const match of code.matchAll(/<select/g)) {
        const end = code.indexOf('</select>', match.index!)
        if (end === -1) continue
        const block = code.slice(match.index!, end + 9)
        if (!entitySources.test(block)) continue
        if (!/<option value=""/.test(block)) {
          offenders.push(`${name}: ${block.split(/\s+/).slice(0, 6).join(' ')}`)
        }
      }
    }
    expect(offenders).toEqual([])
  })

  it('ت۸۰ب — هیچ گزینه‌ی دراپ‌داونی برچسب لاتین خام ندارد', () => {
    // رابط فارسی است؛ «Batch» و «Weighted Average» برای کاربر ایرانی
    // معنا ندارند. اصطلاح فنی مثل «API Key» استثناست چون ترجمه‌اش گمراه‌کننده‌تر است.
    const allowed = /API|Key|OAuth|Bearer|Basic|Token|CSV|Excel|PDF|JSON|XML|URL|SMS|Webhook/
    const offenders: string[] = []
    for (const { name, code } of allFiles) {
      const options = code.match(/<option[^>]*>([^<{]+)<\/option>/g) ?? []
      for (const option of options) {
        const label = option.replace(/<[^>]+>/g, '').trim()
        if (!label || allowed.test(label)) continue
        // برچسب باید دست‌کم یک حرف فارسی داشته باشد (عدد و درصد مجاز است).
        const hasPersian = /[\u0600-\u06FF]/.test(label)
        const isNumeric = /^[\d۰-۹٪%.,\s-]+$/.test(label)
        if (!hasPersian && !isNumeric) offenders.push(`${name}: ${label}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('ت۸۱ — فهرست وضعیت چک از واژه‌نامه‌ی مشترک می‌آید، نه رشته‌ی خام', () => {
    // هیچ صفحه‌ای نباید وضعیت انگلیسی چک را مستقیم چاپ کند.
    const checksPage = pageFiles.find((file) => file.name === 'Checks.tsx')!
    expect(checksPage.code).toContain('checkStatusLabel')
    expect(checksPage.code).not.toMatch(/>\s*(in_hand|deposited|collected|bounced)\s*</)
  })

  it('ت۸۲ — گزینه‌های روش تسویه و تخصیص بها از backend می‌آیند', () => {
    // اگر فهرست در UI ثابت شود، با تغییر موتور ناهماهنگ می‌شود.
    const treasuryForm = pageFiles.find((f) => f.name === 'TreasuryDocumentForm.tsx')!
    expect(treasuryForm.code).toContain('getPaymentMethods')
    const production = pageFiles.find((f) => f.name === 'Production.tsx')!
    expect(production.code).toContain('getCostAllocations')
  })
})

// ===========================================================================
// سیستم طراحی
// ===========================================================================

describe('سیستم طراحی', () => {
  it('ت۸۳ — توکن‌های برند دقیقاً مطابق مرجع تعریف شده‌اند', () => {
    expect(designSystem).toContain('--primary: #21254e')
    expect(designSystem).toContain('--accent: #dca757')
    expect(designSystem).toMatch(/--sidebar-from:/)
    expect(designSystem).toMatch(/--sidebar-to:/)
  })

  it('ت۸۴ — تم تیره همه‌ی توکن‌های تم روشن را بازتعریف می‌کند', () => {
    const light = [...designSystem.matchAll(/^\s{2}(--[a-z0-9-]+):/gm)]
      .map((match) => match[1])
      .filter((token) => !token.startsWith('--color-'))
    const darkBlock = designSystem.slice(designSystem.indexOf('.dark {'))
    const missing = light.filter((token) => {
      // توکن‌های ابعادی (شعاع، فاصله) در تم تیره تغییر نمی‌کنند و لازم نیست تکرار شوند.
      if (/radius|sidebar-w|topbar/.test(token)) return false
      // توکن‌های سازگاری، از توکن‌های اصلی مشتق می‌شوند.
      if (darkBlock.includes(`${token}:`)) return false
      return /bg|card|border|text|muted|faint|primary|accent|success|danger|warning|info|chart|grid|shadow|sidebar/.test(
        token,
      )
    })
    expect(missing).toEqual([])
  })

  it('ت۸۵ — هیچ رنگ خام hex در صفحات نمانده است', () => {
    // رنگ خام یعنی آن نقطه در تم تیره خراب می‌شود.
    const offenders: string[] = []
    for (const { name, code } of pageFiles) {
      const hexes = code.match(/#[0-9a-fA-F]{3,8}\b/g) ?? []
      if (hexes.length > 0) offenders.push(`${name}: ${hexes.join(', ')}`)
    }
    expect(offenders).toEqual([])
  })

  it('ت۸۶ — هیچ متغیر CSS ناقص یا خرابی در شیوه‌نامه‌ها نیست', () => {
    // الگوی `var(--x)7e3` که از پاک‌سازی ناقص رنگ‌ها می‌ماند.
    const broken = allCss.match(/var\(--[a-z0-9-]+\)[0-9a-f]{2,}/g) ?? []
    expect(broken).toEqual([])
  })

  it('ت۸۷ — اسکرول‌بار سفارشی با توکن تم تعریف شده است', () => {
    expect(designSystem).toContain('::-webkit-scrollbar')
    expect(designSystem).toMatch(/scrollbar-thumb\s*\{[^}]*var\(--border-strong\)/s)
  })

  it('ت۸۸ — تم تیره با کلاس `dark` روی ریشه کار می‌کند، نه `app.dark`', () => {
    expect(allCss).not.toContain('.app.dark')
    expect(designSystem).toContain('.dark {')
  })

  it('ت۸۹ — حرکت‌ها برای کاربران حساس به حرکت خاموش می‌شوند', () => {
    expect(designSystem).toContain('prefers-reduced-motion: reduce')
  })
})

// ===========================================================================
// چیدمان و دسترس‌پذیری
// ===========================================================================

describe('چیدمان و دسترس‌پذیری', () => {
  it('ت۹۰ — منوی جمع‌شده برای هر آیتم تولتیپ یا منوی شناور دارد', () => {
    const sidebar = componentFiles.find((file) => file.name === 'Sidebar.tsx')!
    expect(sidebar.code).toContain('role="tooltip"')
    // و آیتم دارای زیرمنو در حالت جمع‌شده، منوی شناور می‌دهد.
    expect(sidebar.code).toMatch(/collapsed && item\.children/)
  })

  it('ت۹۱ — ناحیه‌ی پیمایش منو واقعاً اسکرول می‌شود', () => {
    const sidebar = componentFiles.find((file) => file.name === 'Sidebar.tsx')!
    // بدون `min-h-0` ناحیه‌ی flex هرگز اسکرول نمی‌شود.
    expect(sidebar.code).toMatch(/min-h-0[^"]*overflow-y-auto|overflow-y-auto[^"]*min-h-0/)
  })

  it('ت۹۲ — همه‌ی دکمه‌های فقط-آیکنی برچسب دسترس‌پذیری دارند', () => {
    const offenders: string[] = []
    for (const { name, code } of allFiles) {
      const iconButtons = code.match(/<button[^>]*className="[^"]*icon-btn[^"]*"[^>]*>/g) ?? []
      for (const button of iconButtons) {
        if (!button.includes('aria-label')) offenders.push(`${name}: ${button.slice(0, 70)}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('ت۹۳ — جدول‌های بلند سرآیند چسبان دارند', () => {
    // بدون سرآیند چسبان، در جدول ۵۰ ردیفی معلوم نیست کدام ستون کدام است.
    expect(allCss).toMatch(/\.large-table thead th\s*\{[^}]*position:\s*sticky/s)
  })

  it('ت۹۴ — هیچ صفحه‌ای کلاس «در دست ساخت» ندارد', () => {
    const offenders = pageFiles.filter(
      (file) => file.code.includes('UnderConstruction') || file.code.includes('آماده توسعه'),
    )
    expect(offenders.map((file) => file.name)).toEqual([])
  })
})

// ===========================================================================
// قراردادهای فرم
// ===========================================================================

describe('قراردادهای فرم', () => {
  it('ت۹۵ — هیچ فرمی مبلغ را در مرورگر محاسبه و ثبت نمی‌کند', () => {
    // فرم‌های مالی باید جمع را از موتور بگیرند. وجود `preview*` نشانه‌ی
    // این است که محاسبه سمت موتور انجام می‌شود.
    const financialForms = ['InvoiceForm.tsx', 'TreasuryDocumentForm.tsx', 'Quotes.tsx', 'Production.tsx']
    for (const name of financialForms) {
      const file = pageFiles.find((item) => item.name === name)
      expect(file, `فایل ${name} پیدا نشد`).toBeTruthy()
      expect(file!.code, `${name} پیش‌نمایش موتور ندارد`).toMatch(/preview[A-Z]\w+|preview_/)
    }
  })

  it('ت۹۶ — همه‌ی صفحات خطا را با مترجم مشترک نمایش می‌دهند', () => {
    // پیام خام بک‌اند برای کاربر بی‌معناست و کد خطا را گم می‌کند.
    const offenders: string[] = []
    for (const { name, code } of pageFiles) {
      const catchesErrors = /catch\s*\((\w+)\)/.test(code)
      if (catchesErrors && !code.includes('errorText')) offenders.push(name)
    }
    expect(offenders).toEqual([])
  })

  it('ت۹۷ — هیچ فرمی تاریخ میلادی از کاربر نمی‌گیرد', () => {
    // کل سیستم شمسی است؛ ورودی میلادی یعنی فیلتر و ذخیره‌ی خراب.
    const offenders: string[] = []
    for (const { name, code } of pageFiles) {
      // توضیحات کد ممکن است از باگ قبلی نام ببرد؛ فقط JSX واقعی سنجیده می‌شود.
      const withoutComments = code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '')
      if (/<input[^>]*type="date"/.test(withoutComments)) offenders.push(name)
    }
    expect(offenders).toEqual([])
  })

  it('ت۹۸ — هر فرم مالی دکمه‌اش را تا معتبر شدن ورودی غیرفعال نگه می‌دارد', () => {
    const financialForms = ['TreasuryDocumentForm.tsx', 'Returns.tsx', 'Quotes.tsx', 'Production.tsx']
    for (const name of financialForms) {
      const file = pageFiles.find((item) => item.name === name)!
      expect(file.code, `${name} دکمه‌ی بدون کنترل دارد`).toMatch(/disabled=\{[^}]*canSubmit|disabled=\{!canSubmit/)
    }
  })

  it('ت۹۹ — مبالغ همه‌جا با قالب‌کننده‌ی مشترک نمایش داده می‌شوند', () => {
    // نمایش خام عدد یعنی بدون جداکننده‌ی هزارگان — ناخوانا و خطاساز.
    const offenders: string[] = []
    for (const { name, code } of pageFiles) {
      const showsMoney = /ریال|مبلغ|بهای/.test(code)
      if (showsMoney && !/formatRials|money\(/.test(code)) offenders.push(name)
    }
    expect(offenders).toEqual([])
  })
})

// ===========================================================================
// قالب‌بندی عدد، پول و تاریخ
// ===========================================================================

describe('قالب‌بندی', () => {
  it('ت۱۰۰ — تبدیل تومان و ریال دوطرفه و بدون خطاست', () => {
    expect(tomansToRials(1)).toBe(10)
    expect(tomansToRials(1_234_567)).toBe(12_345_670)
    // نمایش ریال به تومان باید تقسیم بر ده باشد.
    expect(formatTomans(10_000)).toContain('تومان')
    // و مبلغ صفر هم درست نمایش داده شود، نه خالی.
    expect(formatRials(0)).not.toBe('')
  })

  it('ت۱۰۱ — ارقام فارسی و عربی ورودی کاربر پذیرفته می‌شوند', () => {
    expect(normalizeDigits('۱۲۳۴')).toBe('1234')
    expect(normalizeDigits('١٢٣٤')).toBe('1234')
    expect(parseAmount('۱٬۲۳۴٬۵۶۷')).toBe(1234567)
    expect(parseAmount('1,234,567')).toBe(1234567)
    // ورودی بی‌معنا باید null بدهد، نه صفر — صفر یعنی «کاربر صفر زد».
    expect(parseAmount('abc')).toBeNull()
    expect(parseAmount('')).toBeNull()
  })

  it('ت۱۰۲ — درصد با علامت نمایش داده می‌شود تا رشد و افت اشتباه نشوند', () => {
    expect(formatPercent(12)).toContain('+')
    expect(formatPercent(-12)).toContain('−')
    expect(formatPercent(0)).not.toContain('+')
    expect(formatPercent(0)).not.toContain('−')
  })

  it('ت۱۰۳ — تاریخ شمسیِ از پیش شمسی، دوباره تبدیل نمی‌شود', () => {
    // اگر دوباره تبدیل شود، تاریخ ۶۲۱ سال جابه‌جا می‌شود.
    expect(formatJalali('1405/05/10')).toBe('1405/05/10')
    expect(formatJalali('1405/5/1')).toBe('1405/5/1')
    // و تاریخ میلادی درست تبدیل می‌شود.
    expect(formatJalali('2025-08-21')).toBe('1404/05/30')
    // امروز باید قالب معتبر شمسی بدهد.
    expect(todayJalali()).toMatch(/^1[34]\d{2}\/\d{2}\/\d{2}$/)
  })

  it('ت۱۰۴ — عدد اعشاری و صحیح تفکیک نمایش دارند', () => {
    // مبلغ گرد می‌شود (ریال اعشار ندارد) ولی مقدار کالا نه.
    expect(formatRials(1234.7)).toBe(formatRials(1235))
    expect(formatNumber(2.5)).not.toBe(formatNumber(3))
  })
})

// ===========================================================================
// یکپارچگی منطق مشترک
// ===========================================================================

describe('منطق مشترک', () => {
  it('ت۱۰۵ — هر وضعیت چک برچسب فارسی و لحن رنگی دارد', () => {
    const statuses = Object.keys(CHECK_STATUS_LABELS)
    expect(statuses).toHaveLength(12)
    for (const status of statuses) {
      const label = checkStatusLabel(status)
      expect(label).not.toBe(status)
      // برچسب باید فارسی باشد، نه لاتین.
      expect(label).toMatch(/[\u0600-\u06FF]/)
      expect(['done', 'danger', 'pending', 'neutral']).toContain(checkStatusTone(status))
    }
  })

  it('ت۱۰۶ — وضعیت باز چک دقیقاً همان‌هایی است که در مانده می‌آیند', () => {
    // پایانی و انتظامی نباید در مانده‌ی جاری بیایند.
    expect(isOpenCheck('in_hand')).toBe(true)
    expect(isOpenCheck('outstanding')).toBe(true)
    expect(isOpenCheck('collected')).toBe(false)
    expect(isOpenCheck('paid')).toBe(false)
    expect(isOpenCheck('void')).toBe(false)
    expect(isOpenCheck('memo_in_hand')).toBe(false)
  })

  it('ت۱۰۷ — جمع‌بندی بی‌معنا برای هیچ نوع ستونی پیشنهاد نمی‌شود', () => {
    // جمع نام‌ها و جمع تاریخ‌ها عدد بی‌معنایی می‌سازد.
    expect(allowedAggregations('text')).not.toContain('sum')
    expect(allowedAggregations('text')).not.toContain('average')
    expect(allowedAggregations('date')).not.toContain('sum')
    expect(allowedAggregations('date')).not.toContain('average')
    expect(allowedAggregations('money')).toContain('sum')
  })

  it('ت۱۰۸ — ادغام کلاس‌ها تضاد Tailwind را درست حل می‌کند', () => {
    // بدون این، کلاس شرطی روی کلاس پایه اثر نمی‌گذارد.
    expect(cn('p-2', 'p-4')).toBe('p-4')
    expect(cn('text-muted', false && 'text-danger')).toBe('text-muted')
    expect(cn('text-muted', true && 'text-danger')).toBe('text-danger')
  })

  it('ت۱۰۹ — شبیه‌ساز پیش‌نمایش فقط در حالت توسعه فعال می‌شود', () => {
    // اگر در نسخه‌ی نهایی فعال بماند، کاربر داده‌ی ساختگی می‌بیند.
    const preview = readFileSync(join(SRC, 'lib', 'devPreview.ts'), 'utf8')
    expect(preview).toMatch(/import\.meta\.env\.DEV/)
  })

  it('ت۱۱۰ — هیچ صفحه‌ای مستقیماً با Tauri حرف نمی‌زند', () => {
    // همه باید از لایه‌ی `api` رد شوند تا خطاها یکسان ترجمه شوند.
    const offenders = pageFiles.filter((file) => file.code.includes('@tauri-apps/api'))
    expect(offenders.map((file) => file.name)).toEqual([])
  })
})
