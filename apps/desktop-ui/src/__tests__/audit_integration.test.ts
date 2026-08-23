/**
 * ممیزی یکپارچگی — قرارداد بین رابط کاربری، لایه‌ی API و میزبان.
 *
 * ## شکافی که این ممیزی می‌بندد
 *
 * سه لایه داریم که باید دقیقاً با هم بخوانند:
 *
 * ```
 *   صفحه  →  api.ts  →  فرمان Tauri در میزبان
 *                   ↘  شبیه‌ساز پیش‌نمایش (حالت توسعه)
 * ```
 *
 * اگر نام فرمان در `api.ts` با نام ثبت‌شده در میزبان یکی نباشد، دکمه در
 * نسخه‌ی نهایی کار نمی‌کند — ولی در پیش‌نمایش کار می‌کند و کسی متوجه
 * نمی‌شود. این خطرناک‌ترین نوع باگ است: **فقط برای کاربر نهایی رخ می‌دهد.**
 *
 * همین‌طور اگر صفحه‌ای در منو باشد ولی در مسیریابی نباشد، کلیک روی آن هیچ
 * کاری نمی‌کند.
 */
import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { describe, expect, it } from 'vitest'

const SRC = join(__dirname, '..')
const REPO = join(SRC, '..', '..', '..')
const HOST = join(REPO, 'apps', 'desktop-host', 'src-tauri', 'src')

const apiSource = readFileSync(join(SRC, 'api.ts'), 'utf8')
const appSource = readFileSync(join(SRC, 'App.tsx'), 'utf8')
const previewSource = readFileSync(join(SRC, 'lib', 'devPreview.ts'), 'utf8')

const hostSources = readdirSync(HOST)
  .filter((name) => name.endsWith('.rs'))
  .map((name) => ({ name, code: readFileSync(join(HOST, name), 'utf8') }))
const hostCode = hostSources.map((file) => file.code).join('\n')

const pageFiles = readdirSync(join(SRC, 'pages'))
  .filter((name) => name.endsWith('.tsx'))
  .map((name) => ({ name, code: readFileSync(join(SRC, 'pages', name), 'utf8') }))

/** نام همه‌ی فرمان‌هایی که `api.ts` صدا می‌زند. */
const apiCommands = [...apiSource.matchAll(/api<[^>]*>\(\s*'([a-z0-9_]+)'/g)].map((m) => m[1])

/** نام همه‌ی فرمان‌های اعلام‌شده در میزبان. */
const hostCommands = [...hostCode.matchAll(/#\[tauri::command\][\s\S]{0,200}?fn\s+([a-z0-9_]+)/g)].map(
  (m) => m[1],
)

/** فرمان‌های ثبت‌شده در `generate_handler!`. */
const registered = (() => {
  const main = hostSources.find((file) => file.name === 'main.rs')!.code
  const start = main.indexOf('generate_handler![')
  // فهرست تا `]`ِ متوازن ادامه دارد؛ توقف روی نخستین `]` باعث می‌شود
  // بیشتر فرمان‌ها دیده نشوند و تست دروغ بگوید.
  const listStart = main.indexOf('[', start)
  let depth = 0
  let end = start
  for (let index = listStart; index < main.length; index += 1) {
    if (main[index] === '[') depth += 1
    else if (main[index] === ']') {
      depth -= 1
      if (depth === 0) {
        end = index
        break
      }
    }
  }
  return main
    .slice(listStart + 1, end)
    .split(',')
    .map((entry) => entry.trim().split('::').pop()!.trim())
    .filter((name) => /^[a-z0-9_]+$/.test(name))
})()

/** صفحاتی که `App.tsx` می‌شناسد. */
const routedPages = [...appSource.matchAll(/case '([a-z0-9-]+)':/g)].map((m) => m[1])

/** صفحاتی که در منو آمده‌اند. */
const menuPages = [...appSource.matchAll(/page: '([a-z0-9-]+)'/g)].map((m) => m[1])

// ===========================================================================
// قرارداد نام فرمان‌ها
// ===========================================================================

describe('قرارداد رابط کاربری با میزبان', () => {
  it('ت۱۳۶ — هر فرمانی که رابط صدا می‌زند، در میزبان وجود دارد', () => {
    // نبود فرمان یعنی دکمه‌ای که فقط در پیش‌نمایش کار می‌کند.
    const missing = [...new Set(apiCommands)].filter((name) => !hostCommands.includes(name))
    expect(missing).toEqual([])
  })

  it('ت۱۳۷ — هر فرمانی که رابط صدا می‌زند، در میزبان ثبت شده است', () => {
    // اعلام‌شده ولی ثبت‌نشده = در زمان اجرا «command not found».
    const unregistered = [...new Set(apiCommands)].filter((name) => !registered.includes(name))
    expect(unregistered).toEqual([])
  })

  it('ت۱۳۸ — لایه‌ی API واقعاً همه‌ی ماژول‌های میزبان را پوشش می‌دهد', () => {
    // اگر ماژولی هیچ فرمانش صدا زده نشود، یعنی قابلیتی ساخته شده و به
    // کاربر نرسیده است.
    const modules = hostSources.filter((file) => file.name !== 'main.rs')
    const uncovered: string[] = []
    for (const file of modules) {
      const names = [...file.code.matchAll(/#\[tauri::command\][\s\S]{0,200}?fn\s+([a-z0-9_]+)/g)].map(
        (m) => m[1],
      )
      if (names.length > 0 && !names.some((name) => apiCommands.includes(name))) {
        uncovered.push(file.name)
      }
    }
    expect(uncovered).toEqual([])
  })

  it('ت۱۳۹ — شبیه‌ساز پیش‌نمایش هیچ فرمان ناموجودی را جعل نمی‌کند', () => {
    // فرمانی که فقط در شبیه‌ساز هست، توهم کارکرد می‌سازد.
    // فقط کلیدهای سطح اولِ جدول پاسخ‌ها، نه فیلدهای داخل شیء داده.
    const start = previewSource.indexOf('const responses')
    const table = previewSource.slice(start)
    const previewHandlers = [...table.matchAll(/^\s{2}([a-z0-9_]+):\s*\(/gm)].map((m) => m[1])
    expect(previewHandlers.length).toBeGreaterThan(20)
    const fake = previewHandlers.filter((name) => !hostCommands.includes(name))
    expect(fake).toEqual([])
  })

  it('ت۱۴۰ — هر فرمان مهم، هم در شبیه‌ساز پیش‌نمایش پاسخ دارد', () => {
    // بدون این، پیش‌نمایش طراحی برای آن صفحه خالی می‌ماند.
    const critical = [
      'list_products',
      'list_parties',
      'list_checks_filtered',
      'list_treasury_documents',
      'list_production_formulas',
      'list_quotes',
      'list_returns',
      'list_account_tree',
      'list_settings',
    ]
    const missing = critical.filter((name) => !previewSource.includes(`${name}:`))
    expect(missing).toEqual([])
  })
})

// ===========================================================================
// کامل بودن مسیریابی و منو
// ===========================================================================

describe('مسیریابی و منو', () => {
  it('ت۱۴۱ — هر صفحه‌ای که در منو آمده، مسیر دارد', () => {
    // کلیک روی آیتمی که مسیر ندارد، هیچ کاری نمی‌کند.
    const orphan = [...new Set(menuPages)].filter((page) => !routedPages.includes(page))
    // `invoices` صفحه‌ی پیش‌فرض است و از `default` رد می‌شود.
    const meaningful = orphan.filter((page) => !['sales', 'purchase', 'products', 'inventory'].includes(page))
    expect(meaningful).toEqual([])
  })

  it('ت۱۴۲ — هر مسیر، عنوان صفحه دارد', () => {
    // عنوان نداشتن یعنی نوار بالای برنامه خالی می‌ماند.
    const titles = [...appSource.matchAll(/^\s{2}'?([a-z0-9-]+)'?:\s*'[^']+',$/gm)].map((m) => m[1])
    const missing = [...new Set(routedPages)].filter(
      (page) => !titles.includes(page) && page !== 'default',
    )
    expect(missing).toEqual([])
  })

  it('ت۱۴۳ — همه‌ی صفحات ساخته‌شده در مسیریابی استفاده می‌شوند', () => {
    // صفحه‌ای که import نشده، کد مرده است.
    const unused = pageFiles.filter((file) => {
      const component = file.name.replace('.tsx', '')
      return !appSource.includes(`from './pages/${component}'`)
    })
    // این‌ها اجزای داخلی صفحات دیگرند، نه صفحه‌ی مستقل.
    // اجزای داخلی که از صفحه‌ی والدشان فراخوانی می‌شوند، نه از مسیریاب.
    const internal = ['PartyForm', 'ProductForm', 'DataPage', 'ProductionFormulaDialogs']
    const meaningful = unused.filter((file) => !internal.includes(file.name.replace('.tsx', '')))
    expect(meaningful.map((file) => file.name)).toEqual([])
  })

  it('ت۱۴۴ — عملیات سریع فقط به صفحات موجود اشاره می‌کنند', () => {
    const start = appSource.indexOf('const QUICK_ACTIONS')
    const block = appSource.slice(start, appSource.indexOf(']', start))
    const quickActions = [...block.matchAll(/page:\s*'([a-z0-9-]+)'/g)].map((m) => m[1])
    expect(quickActions.length).toBeGreaterThan(3)
    // شاخه‌ی `default` مسیریاب، فهرست فاکتور را با نام همان صفحه باز می‌کند.
    const handledByDefault = ['sales', 'purchase', 'products', 'inventory']
    const broken = quickActions.filter(
      (page) => !routedPages.includes(page) && !handledByDefault.includes(page),
    )
    expect(broken).toEqual([])
  })

  it('ت۱۴۵ — منوی برنامه همه‌ی ماژول‌های اصلی مرجع را دارد', () => {
    // مرجع: منوهای `dgNqWj` و `hQO24U`
    const required = [
      'invoice-form', // صدور فاکتور
      'sales-return', // برگشت از فروش
      'proforma', // پیش‌فاکتور
      'purchase-order', // سفارش خرید
      'inventory-count', // انبارگردانی
      'inventory-transfer', // انتقال بین انبارها
      'production', // تولید
      'treasury-document', // سند دریافت و پرداخت
      'banks',
      'cashboxes',
      'checks',
      'chart-of-accounts',
      'parties',
      'report-builder',
    ]
    const missing = required.filter((page) => !menuPages.includes(page))
    expect(missing).toEqual([])
  })
})

// ===========================================================================
// یکپارچگی نوع‌ها
// ===========================================================================

describe('یکپارچگی نوع‌ها', () => {
  it('ت۱۴۶ — نام پارامترها بین رابط و میزبان یکسان است', () => {
    // Tauri نام پارامتر را camelCase می‌کند؛ ناهماهنگی یعنی مقدار به
    // فرمان نمی‌رسد و بی‌صدا `null` می‌شود.
    const problems: string[] = []
    for (const match of apiSource.matchAll(/api<[^>]*>\(\s*'([a-z0-9_]+)'\s*,\s*\{([^}]*)\}/g)) {
      const [, command, args] = match
      const hostFn = hostCode.match(
        new RegExp(`#\\[tauri::command\\][\\s\\S]{0,200}?fn\\s+${command}\\s*\\(([^)]*)\\)`),
      )
      if (!hostFn) continue
      const hostParams = hostFn[1]
        .split(',')
        .map((part) => part.split(':')[0].trim())
        .filter((name) => name && name !== 'state')
      for (const pair of args.split(',')) {
        const key = pair.split(':')[0].trim()
        if (!key || key.startsWith('...')) continue
        // camelCase در رابط ↔ snake_case در میزبان
        const snake = key.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)
        if (hostParams.length > 0 && !hostParams.includes(snake) && !hostParams.includes(key)) {
          problems.push(`${command}: «${key}» در میزبان «${hostParams.join(', ')}» نیست`)
        }
      }
    }
    expect(problems).toEqual([])
  })

  it('ت۱۴۷ — هیچ فرمانی بدون نوع بازگشتی صدا زده نمی‌شود', () => {
    const untyped = [...apiSource.matchAll(/api\(\s*'([a-z0-9_]+)'/g)].map((m) => m[1])
    expect(untyped).toEqual([])
  })

  it('ت۱۴۸ — همه‌ی مبالغ در قراردادها عدد صحیح‌اند، نه رشته', () => {
    // مبلغ رشته‌ای یعنی جمع‌زدن با الحاق متن — باگ کلاسیک.
    const moneyFields = [...apiSource.matchAll(/(amount|total|subtotal|balance|price|cost)\??:\s*(\w+)/g)]
    expect(moneyFields.length).toBeGreaterThan(10)
    const wrong = moneyFields.filter((match) => match[2] !== 'number').map((match) => match[0])
    expect(wrong).toEqual([])
  })
})

// ===========================================================================
// انسجام داده‌ی پیش‌نمایش با قواعد واقعی
// ===========================================================================

describe('داده‌ی پیش‌نمایش', () => {
  it('ت۱۴۹ — وضعیت چک‌های نمونه با واژه‌نامه‌ی واقعی می‌خواند', () => {
    const statuses = [...previewSource.matchAll(/'(in_hand|deposited|collected|cashed|endorsed|bounced|returned|void|outstanding|paid|memo_in_hand|memo_returned)'/g)]
    expect(statuses.length).toBeGreaterThan(3)
    // هیچ وضعیت منسوخی نمانده باشد.
    expect(previewSource).not.toContain("'registered'")
    expect(previewSource).not.toContain("'in_progress'")
    expect(previewSource).not.toContain("'cleared'")
  })

  it('ت۱۵۰ — تاریخ‌های نمونه شمسی‌اند، نه میلادی', () => {
    const isoDates = previewSource.match(/'\d{4}-\d{2}-\d{2}'/g) ?? []
    expect(isoDates).toEqual([])
  })

  it('ت۱۵۱ — هیچ مبلغ منفی در داده‌ی نمونه نیست', () => {
    // مبلغ منفی در نمونه یعنی کاربر عدد بی‌معنا می‌بیند.
    const negatives = previewSource.match(/(amount|total|price|cost|balance):\s*-\d/g) ?? []
    expect(negatives).toEqual([])
  })

  it('ت۱۵۲ — داده‌ی نمونه همه‌ی ماژول‌ها را پوشش می‌دهد', () => {
    const modules = [
      'products',
      'contacts',
      'warehouses',
      'salesInvoices',
      'purchaseInvoices',
      'checks',
      'treasuryAccounts',
      'demoReturns',
      'demoTransfers',
      'demoQuotes',
      'demoFormulas',
      'demoProductionOrders',
      'demoSettings',
    ]
    const missing = modules.filter((name) => !previewSource.includes(`const ${name}`))
    expect(missing).toEqual([])
  })
})

// ===========================================================================
// سلامت ساختار پروژه
// ===========================================================================

describe('سلامت ساختار', () => {
  it('ت۱۵۳ — هیچ فایل صفحه‌ای غول‌پیکر نشده است', () => {
    const oversized = pageFiles
      .map((file) => ({ name: file.name, lines: file.code.split('\n').length }))
      .filter((file) => file.lines > 900)
    expect(oversized).toEqual([])
  })

  it('ت۱۵۴ — هیچ کد مرده‌ای با علامت TODO یا FIXME نمانده است', () => {
    const markers: string[] = []
    for (const { name, code } of pageFiles) {
      if (/TODO|FIXME|XXX|HACK/.test(code)) markers.push(name)
    }
    expect(markers).toEqual([])
  })

  it('ت۱۵۵ — هیچ `console.log` در کد نهایی نمانده است', () => {
    const noisy: string[] = []
    for (const { name, code } of pageFiles) {
      if (/console\.(log|debug|warn)/.test(code)) noisy.push(name)
    }
    expect(noisy).toEqual([])
  })

  it('ت۱۵۶ — هیچ متن انگلیسی به‌عنوان عنوان صفحه نمانده است', () => {
    // بازخورد کارفرما: عنوان‌های لاتین مثل «CHECKS» و «PARTIES».
    const offenders: string[] = []
    for (const { name, code } of pageFiles) {
      for (const match of code.matchAll(/className="eyebrow">([^<]+)</g)) {
        const label = match[1].trim()
        if (!/[\u0600-\u06FF]/.test(label)) offenders.push(`${name}: ${label}`)
      }
    }
    expect(offenders).toEqual([])
  })

  it('ت۱۵۷ — هیچ صفحه‌ای بدون عنوان اصلی نیست', () => {
    const missing = pageFiles.filter((file) => !file.code.includes('<h1>'))
    // اجزای داخلی (فرم مودالی) عنوان `h2` دارند نه `h1`.
    const meaningful = missing.filter((file) => !file.code.includes('<h2>'))
    expect(meaningful.map((file) => file.name)).toEqual([])
  })

  it('ت۱۵۸ — همه‌ی ماژول‌های کتابخانه توضیح سرفایل دارند', () => {
    const libDir = join(SRC, 'lib')
    const undocumented: string[] = []
    for (const name of readdirSync(libDir).filter((file) => file.endsWith('.ts'))) {
      const code = readFileSync(join(libDir, name), 'utf8')
      if (!code.trimStart().startsWith('/**')) undocumented.push(name)
    }
    expect(undocumented).toEqual([])
  })

  it('ت۱۵۹ — پوسته‌ی برنامه فقط یک منبع تم دارد', () => {
    // دو مجموعه توکن یعنی تم تیره‌ی نیمه‌کاره.
    const theme = readFileSync(join(SRC, 'theme.css'), 'utf8')
    expect(theme).not.toMatch(/^:root\s*\{/m)
    const design = readFileSync(join(SRC, 'design-system.css'), 'utf8')
    expect(design).toMatch(/^:root\s*\{/m)
  })

  it('ت۱۶۰ — هر وابستگی نصب‌شده واقعاً استفاده می‌شود', () => {
    const pkg = JSON.parse(readFileSync(join(SRC, '..', 'package.json'), 'utf8'))
    const allCode = [...pageFiles, ...readdirSync(join(SRC, 'components')).map((name) => ({
      name,
      code: readFileSync(join(SRC, 'components', name), 'utf8'),
    }))]
      .map((file) => file.code)
      .join('\n') +
      apiSource +
      appSource +
      previewSource +
      readFileSync(join(SRC, 'main.tsx'), 'utf8')
    const unused = Object.keys(pkg.dependencies ?? {}).filter((name) => {
      if (name.startsWith('@tauri-apps')) return false // در لایه‌ی api استفاده می‌شود
      return !allCode.includes(name) && !readFileSync(join(SRC, 'lib', 'cn.ts'), 'utf8').includes(name)
    })
    expect(unused).toEqual([])
  })
})
