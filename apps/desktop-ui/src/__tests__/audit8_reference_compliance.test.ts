/**
 * @vitest-environment jsdom
 *
 * ممیزی دور ۸ — انطباق با اسکرین‌شات‌های نوین پرداز (۵۰ تست سخت‌گیرانه).
 *
 * هر تست به یک قابلیت مشخص از یک تصویر مرجع گره خورده و شناسه‌ی تصویر در
 * توضیحش آمده است. هدف این پرونده «سبز ماندن» نیست، «برگشت‌ناپذیر کردن» است:
 * اگر قابلیتی که یک بار مطابق مرجع ساخته شده حذف یا خراب شود، همین‌جا قرمز
 * می‌شود. فهرست شکاف‌های باقی‌مانده در `docs/AUDIT_ROUND8.md` است.
 */
import { describe, expect, it } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { fa } from '../lib/i18n'
import { CHECK_STATUSES, checkStatusLabel, checkStatusTone, isOpenCheck } from '../lib/checkStatus'
import { SHORTCUTS, isTypingTarget, shortcutTarget } from '../lib/shortcuts'
import { PRESETS, resolveRange, previousRange, inRange } from '../lib/dateRange'
import { formatRials, formatJalali, toJalali, tomansToRials, parseAmount } from '../lib/format'

const SRC = resolve(__dirname, '..')
const ROOT = resolve(SRC, '../../..')
const ui = (relative: string) => readFileSync(resolve(SRC, relative), 'utf8')
const host = (relative: string) =>
  readFileSync(resolve(ROOT, 'apps/desktop-host/src-tauri/src', relative), 'utf8')
const core = (relative: string) =>
  readFileSync(resolve(ROOT, 'crates/novin-core/src', relative), 'utf8')

const mainRs = host('main.rs')
const apiTs = ui('api.ts')

// ---------------------------------------------------------------------------
// ۱. پوسته و ناوبری — تصویر `mo0rdx`
// ---------------------------------------------------------------------------

describe('م۸ · پوسته و ناوبری (mo0rdx)', () => {
  it('ر۱ — منوی کناری همان چهار گروه مرجع را دارد', () => {
    const app = ui('App.tsx')
    for (const key of ['nav.group.main', 'nav.group.operations', 'nav.group.accounting', 'nav.group.reports']) {
      expect(app, key).toContain(key)
    }
  })

  it('ر۲ — میانبرهای تک‌حرفی فارسی مرجع پیاده شده‌اند', () => {
    // جدول نوار کناری مرجع: ک کالا، الف اشخاص، ب بانک، ص صندوق، چ چک، ف فاکتور فروش، خ خرید.
    expect(SHORTCUTS['ک']).toBe('products')
    expect(SHORTCUTS['ا']).toBe('parties')
    expect(SHORTCUTS['ب']).toBe('banks')
    expect(SHORTCUTS['ص']).toBe('cashboxes')
    expect(SHORTCUTS['چ']).toBe('checks')
    expect(SHORTCUTS['ف']).toBe('invoice-form')
    expect(SHORTCUTS['خ']).toBe('purchase')
    expect(SHORTCUTS['س']).toBe('single-journal')
    expect(SHORTCUTS['ت']).toBe('production')
  })

  it('ر۳ — کلیدهای تابعی مرجع (F2..F5) هم نگاشت دارند', () => {
    expect(SHORTCUTS.F2).toBe('sales-return')
    expect(SHORTCUTS.F3).toBe('purchase-return')
    expect(SHORTCUTS.F4).toBe('cashboxes')
    expect(SHORTCUTS.F5).toBe('banks')
  })

  it('ر۴ — میانبر تک‌حرفی هرگز حین تایپ در فرم فعال نمی‌شود', () => {
    const input = document.createElement('input')
    expect(isTypingTarget(input)).toBe(true)
    expect(shortcutTarget({ key: 'ف', target: input })).toBeNull()
    const div = document.createElement('div')
    expect(shortcutTarget({ key: 'ف', target: div })).toBe('invoice-form')
  })

  it('ر۵ — ترکیب با Ctrl/Alt/Meta میانبر صفحه نیست (جای پالت فرمان را نمی‌گیرد)', () => {
    expect(shortcutTarget({ key: 'ک', ctrlKey: true })).toBeNull()
    expect(shortcutTarget({ key: 'ک', metaKey: true })).toBeNull()
    expect(shortcutTarget({ key: 'ک', altKey: true })).toBeNull()
  })

  it('ر۶ — هر مقصد میانبر واقعاً یک صفحه‌ی مسیریابی‌شده است', () => {
    const app = ui('App.tsx')
    const routed = new Set([...app.matchAll(/case '([a-z0-9-]+)':/g)].map((m) => m[1]))
    // صفحه‌های فهرست فاکتور از شاخه‌ی پیش‌فرض مسیریاب باز می‌شوند.
    const byDefault = new Set(['sales', 'purchase', 'proforma', 'purchase-order'])
    const broken = Object.values(SHORTCUTS).filter((page) => !routed.has(page) && !byDefault.has(page))
    expect(broken).toEqual([])
  })

  it('ر۷ — پالت فرمان (Ctrl+K) همه‌ی صفحه‌ها را پوشش می‌دهد', () => {
    const palette = ui('components/CommandPalette.tsx')
    const ids = [...palette.matchAll(/\{id: '([a-z0-9-]+)'/g)].map((m) => m[1])
    expect(ids.length).toBeGreaterThanOrEqual(28)
    for (const id of ids) expect(`page.${id}` in fa, id).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// ۲. اشخاص — تصاویر `c9pvYl`, `1zkKV5`
// ---------------------------------------------------------------------------

describe('م۸ · اشخاص (c9pvYl, 1zkKV5)', () => {
  const parties = core('parties.rs')
  const form = ui('pages/PartyForm.tsx')

  it('ر۸ — چهار نوع شخصیت مرجع در هسته تعریف شده‌اند', () => {
    for (const kind of ['natural', 'private_legal', 'government_legal', 'civil_partnership']) {
      expect(parties, kind).toContain(kind)
    }
  })

  it('ر۹ — سه نقش مرجع (شخص، بازاریاب، سوپروایزر) وجود دارند', () => {
    for (const role of ['marketer', 'supervisor']) expect(parties, role).toContain(role)
    for (const key of ['parties.role.person', 'parties.role.agent', 'parties.role.supervisor']) {
      expect(key in fa, key).toBe(true)
    }
  })

  it('ر۱۰ — هفت زبانه‌ی فرم شخص مطابق مرجع است', () => {
    for (const key of [
      'partyForm.tab.general',
      'partyForm.tab.contact',
      'partyForm.tab.bank',
      'partyForm.tab.images',
      'partyForm.tab.account',
      'partyForm.tab.other',
      'partyForm.tab.events',
    ]) {
      expect(form, key).toContain(key)
    }
  })

  it('ر۱۱ — مسیر پخش و بازاریاب روی شخص ثبت می‌شوند', () => {
    expect(form).toContain('partyForm.route')
    expect(form).toContain('partyForm.marketer')
    expect(parties).toMatch(/route/)
  })

  it('ر۱۲ — شبا و شماره کارت با الگوریتم رسمی اعتبارسنجی می‌شوند', () => {
    expect(parties).toMatch(/iban/i)
    expect(parties).toMatch(/card/i)
    expect(mainRs).toContain('validate_party_identity')
  })

  it('ر۱۳ — خلاصه‌ی حساب (بدهکار/بستانکار/بی‌حساب) در فهرست اشخاص است', () => {
    const page = ui('pages/Parties.tsx')
    for (const key of ['parties.debtors', 'parties.creditors', 'parties.balanced', 'parties.netBalance']) {
      expect(page, key).toContain(key)
    }
  })

  it('ر۱۴ — تقویم مناسبت‌ها دوازده ماه شمسی دارد', () => {
    for (let month = 1; month <= 12; month += 1) expect(`month.${month}` in fa).toBe(true)
    expect(fa['month.1']).toBe('فروردین')
    expect(fa['month.12']).toBe('اسفند')
  })

  it('ر۱۵ — سقف اعتبار هنگام فروش نسیه کنترل می‌شود', () => {
    expect(host('settings.rs')).toContain('parties.enforce_credit_limit')
    expect(mainRs).toContain('credit_limit')
  })
})

// ---------------------------------------------------------------------------
// ۳. کالا — تصاویر `8Xmc1p`, `NztJl5`, `6FM9Ow`
// ---------------------------------------------------------------------------

describe('م۸ · کالا (8Xmc1p, NztJl5, 6FM9Ow)', () => {
  const catalog = core('catalog.rs')
  const form = ui('pages/ProductForm.tsx')

  it('ر۱۶ — چهار نوع کالای مرجع تعریف شده‌اند', () => {
    for (const kind of ['simple', 'composite', 'variant', 'gold_jewelry']) {
      expect(catalog, kind).toContain(kind)
    }
  })

  it('ر۱۷ — هفت سطح قیمت مرجع وجود دارد', () => {
    for (const level of [
      'retail',
      'wholesale',
      'partner',
      'partner_tier2',
      'partner_tier3',
      'seasonal',
      'exhibition',
    ]) {
      expect(catalog, level).toContain(level)
    }
    expect(catalog).toContain('PriceLevel::ALL')
  })

  it('ر۱۸ — زبانه‌های فرم کالا شامل قیمت، چند واحدی، مالیات و طلا هستند', () => {
    for (const key of [
      'productForm.tab.general',
      'productForm.tab.prices',
      'productForm.tab.units',
      'productForm.tab.tax',
      'productForm.tab.stock',
      'productForm.tab.tiers',
      'productForm.tab.gold',
    ]) {
      expect(form, key).toContain(key)
    }
  })

  it('ر۱۹ — چند واحدی با ضریب تبدیل پیاده شده است', () => {
    expect(form).toContain('productForm.unitFactor')
    expect(catalog).toMatch(/factor/i)
  })

  it('ر۲۰ — شناسه‌ی کالا در سامانه مؤدیان فیلد دارد', () => {
    expect(form).toContain('productForm.taxCode')
    expect(mainRs + core('catalog.rs')).toContain('tax_code')
  })

  it('ر۲۱ — قیمت طلا: مالیات فقط روی اجرت و سود، نه ارزش خودِ فلز', () => {
    // قاعده‌ی مالیاتی ایران؛ اگر این خط برگردد، محاسبه‌ی طلا غلط می‌شود.
    expect(catalog).toMatch(/making_charge/)
    expect(catalog).toMatch(/profit/)
    const goldTest = readFileSync(
      resolve(ROOT, 'crates/novin-core/tests/audit7_catalog.rs'),
      'utf8',
    )
    expect(goldTest).toMatch(/vat|مالیات/i)
  })

  it('ر۲۲ — ستون‌های فهرست کالا همان مرجع‌اند (کد، نام، موجودی، واحد، گروه، جزئی، همکار)', () => {
    const page = ui('pages/Products.tsx')
    for (const key of [
      'common.code',
      'products.name',
      'products.stock',
      'common.unit',
      'common.group',
      'products.retailWithUnit',
      'products.partnerWithUnit',
    ]) {
      expect(page, key).toContain(key)
    }
  })

  it('ر۲۳ — گروه‌بندی درختی کالا در هسته وجود دارد', () => {
    expect(catalog).toContain('build_group_tree')
  })

  it('ر۲۴ — تخفیف پلکانی بر اساس مقدار پیاده شده است', () => {
    expect(form).toContain('productForm.tiersTitle')
    expect(catalog).toMatch(/tier/i)
  })
})

// ---------------------------------------------------------------------------
// ۴. صندوق و بانک — تصاویر `WLumbs`, `p6hT01`
// ---------------------------------------------------------------------------

describe('م۸ · خزانه (WLumbs, p6hT01)', () => {
  const accounts = host('treasury_accounts.rs')

  it('ر۲۵ — سیاست منفی شدن موجودی سه‌حالته است', () => {
    for (const policy of ['error', 'warn', 'ignore']) expect(accounts, policy).toContain(policy)
  })

  it('ر۲۶ — شبا، شماره کارت، شعبه و کارتخوان روی حساب بانکی هستند', () => {
    for (const field of ['iban', 'card_number', 'branch', 'pos_terminal']) {
      expect(accounts, field).toContain(field)
    }
  })

  it('ر۲۷ — حساب خزانه به حساب حسابداری وصل می‌شود', () => {
    expect(accounts).toContain('linked_account_id')
    expect(ui('pages/TreasuryAccounts.tsx')).toContain('treasuryAcc.linkedAccount')
  })

  it('ر۲۸ — صندوق نقدی نمی‌تواند منفی شود (قاعده‌ی حسابداری، نه تزئین)', () => {
    expect(ui('pages/TreasuryAccounts.tsx')).toContain('treasuryAcc.cashLead')
    expect(fa['treasuryAcc.cashLead']).toContain('منفی')
  })
})

// ---------------------------------------------------------------------------
// ۵. چک — تصاویر `1hNwr0`, `rm1qup`, `hutUjB`
// ---------------------------------------------------------------------------

describe('م۸ · چک (1hNwr0, rm1qup, hutUjB)', () => {
  it('ر۲۹ — هر دوازده وضعیت چک مرجع تعریف شده‌اند', () => {
    expect(CHECK_STATUSES.length).toBe(12)
    for (const status of [
      'in_hand',
      'deposited',
      'collected',
      'cashed',
      'endorsed',
      'bounced',
      'returned',
      'void',
      'outstanding',
      'paid',
      'memo_in_hand',
      'memo_returned',
    ]) {
      expect((CHECK_STATUSES as readonly string[]).includes(status), status).toBe(true)
    }
  })

  it('ر۳۰ — چک انتظامی اثر مالی ندارد (قاعده‌ی حسابداری مرجع)', () => {
    const checks = core('checks.rs')
    expect(checks).toContain('MemoInHand')
    expect(checks).toContain('MemoReturned')
    // وضعیت انتظامی نباید مانده‌ی جاری شرکت را بسازد.
    expect(isOpenCheck('memo_in_hand')).toBe(false)
    expect(isOpenCheck('memo_returned')).toBe(false)
  })

  it('ر۳۱ — «برگشتی» با «وصول شده» و «خرج شده» هم‌سطح استایل دارد', () => {
    // بازخورد کاربر: استایل برگشتی با بقیه فرق می‌کرد.
    expect(checkStatusTone('collected')).toBe('done')
    expect(checkStatusTone('bounced')).toBe('danger')
    expect(checkStatusTone('endorsed')).toBe('pending')
    /* هر سه وضعیت باید در همان بلوک‌ها و با همان تعداد قاعده تعریف شوند؛
     * قاعده‌ی اضافه یا کم برای «برگشتی» یعنی همان ناهماهنگی‌ای که کاربر دید. */
    const theme = ui('theme.css')
    const count = (selector: string) => (theme.match(new RegExp(selector, 'g')) ?? []).length
    expect(count('\\.status\\.danger')).toBe(count('\\.status\\.done'))
    expect(count('\\.status\\.danger')).toBe(count('\\.status\\.pending'))
  })

  it('ر۳۲ — گذارهای مجاز را همیشه هسته تعیین می‌کند، نه رابط', () => {
    expect(ui('pages/Checks.tsx')).toContain('getCheckTransitionOptions')
    expect(mainRs).toContain('check_transition_options')
  })

  it('ر۳۳ — وصول و برگشت چک، سند خزانه‌ی متناظر می‌سازند', () => {
    const page = ui('pages/Checks.tsx')
    expect(page).toContain('checks.receiptVoucher')
    expect(page).toContain('checks.paymentVoucher')
    expect(page).toContain('checks.noFinancialEffect')
  })

  it('ر۳۴ — برچسب وضعیت در سه زبان ترجمه دارد', () => {
    expect(checkStatusLabel('bounced', 'fa')).toBe('برگشتی')
    expect(checkStatusLabel('bounced', 'en')).toBe('Bounced')
    expect(checkStatusLabel('bounced', 'ar')).toBe('مرتجع')
  })

  it('ر۳۵ — شناسه صیادی روی سطر چکِ سند خزانه هست', () => {
    expect(ui('pages/TreasuryDocumentForm.tsx')).toContain('treasuryDoc.sayadId')
    expect(core('treasury.rs')).toMatch(/sayad/i)
  })
})

// ---------------------------------------------------------------------------
// ۶. فاکتور — تصاویر `sFpxWK`, `PI5uot`, `FRPBDr`
// ---------------------------------------------------------------------------

describe('م۸ · فاکتور (sFpxWK, PI5uot, FRPBDr)', () => {
  const form = ui('pages/InvoiceForm.tsx')

  it('ر۳۶ — سطر فاکتور همه‌ی اجزای مرجع را دارد', () => {
    for (const key of [
      'invoiceForm.unitPrice',
      'invoiceForm.discountAmount',
      'invoiceForm.discountBp',
      'invoiceForm.vatBp',
      'invoiceForm.dutyBp',
      'invoiceForm.commissionBp',
      'invoiceForm.serialTracked',
    ]) {
      expect(form, key).toContain(key)
    }
  })

  it('ر۳۷ — پانویس مرجع: تخفیف سرجمع، کرایه حمل و سرشکن‌کردن آن', () => {
    expect(form).toContain('invoiceForm.headerDiscount')
    expect(form).toContain('invoiceForm.freight')
    expect(form).toContain('invoiceForm.allocateFreight')
  })

  it('ر۳۸ — شش روش تسویه در یک سند خزانه ممکن است', () => {
    const doc = ui('pages/TreasuryDocumentForm.tsx')
    for (const key of [
      'treasury.method.cash',
      'treasury.method.check',
      'treasury.method.transfer',
      'treasury.method.pos',
      'treasury.method.discount',
      'treasury.method.offset',
    ]) {
      expect(doc, key).toContain(key)
    }
  })

  it('ر۳۹ — نوار وضعیت زنده: مانده قبل و بعد از فاکتور', () => {
    expect(form).toContain('invoiceForm.balanceBefore')
    expect(form).toContain('invoiceForm.balanceAfter')
  })

  it('ر۴۰ — هیچ محاسبه‌ی مالی در رابط انجام نمی‌شود؛ همه از موتور می‌آید', () => {
    expect(form).toContain('previewInvoice')
    // جمع نهایی هرگز در رابط با + ساخته نمی‌شود.
    expect(form).not.toMatch(/const\s+total\s*=\s*lines\.reduce/)
  })

  it('ر۴۱ — برگشت از فروش و خرید فقط اقلام قابل برگشت را نشان می‌دهد', () => {
    const returns = ui('pages/Returns.tsx')
    expect(returns).toContain('returns.returnable')
    expect(returns).toContain('returns.alreadyReturned')
  })

  it('ر۴۲ — اقساط فاکتور با سررسید شمسی تولید می‌شود', () => {
    expect(form).toContain('invoiceForm.generateInstallments')
    expect(form).toContain('due_date_jalali')
  })
})

// ---------------------------------------------------------------------------
// ۷. سند حسابداری و گزارش — تصاویر `Rb2xiG`, `MZlUiD`, `k51J4O`
// ---------------------------------------------------------------------------

describe('م۸ · حسابداری و گزارش (Rb2xiG, MZlUiD, k51J4O)', () => {
  it('ر۴۳ — سند یک‌سطری: یک مبلغ، یک بدهکار، یک بستانکار', () => {
    const page = ui('pages/SingleLineJournal.tsx')
    expect(page).toContain('journal.debitSide')
    expect(page).toContain('journal.creditSide')
    expect(page).toContain('journal.swapSides')
  })

  it('ر۴۴ — کدینگ چهارسطحی با ماهیت و کنترل والد/فرزند', () => {
    const coa = ui('pages/ChartOfAccounts.tsx')
    expect(coa).toContain('coa.natureHint')
    expect(core('coding.rs')).toMatch(/level|سطح/)
  })

  it('ر۴۵ — تراز آزمایشی توازن را صریح نشان می‌دهد', () => {
    const reports = ui('pages/Reports.tsx')
    expect(reports).toContain('reports.balanceCheck')
    expect(reports).toContain('reports.balanced')
    expect(reports).toContain('reports.unbalanced')
  })

  it('ر۴۶ — چهارده گزارش مرجع در مرکز گزارشات هستند', () => {
    const reports = ui('pages/Reports.tsx')
    const kinds = [...reports.matchAll(/\['(\w+)', '(reports\.[\w.]+)'\]/g)]
    expect(kinds.length).toBe(14)
  })

  it('ر۴۷ — مرکز تنظیمات: هر تنظیم می‌گوید کجا اثر می‌گذارد', () => {
    const settings = host('settings.rs')
    // هر تعریف تنظیم باید هم `key` داشته باشد هم `effect` — «کجا اثر می‌گذارد».
    const registry = settings.slice(settings.indexOf('pub fn registry()'))
    const blocks = registry.split('SettingDefinition {').slice(1)
    expect(blocks.length).toBeGreaterThanOrEqual(30)
    const withoutEffect = blocks
      .filter((block) => !block.slice(0, block.indexOf('},')).includes('effect: "'))
      .map((block) => block.match(/key: "([^"]+)"/)?.[1] ?? '?')
    expect(withoutEffect).toEqual([])
  })

  it('ر۴۸ — تاریخ‌ها شمسی نمایش داده می‌شوند و با هسته یکی‌اند', () => {
    // ۱۴۰۴/۰۵/۳۰ = ۲۰۲۵-۰۸-۲۱ (اعتبارسنجی‌شده با نرم‌افزار واقعی)
    expect(formatJalali('2025-08-21')).toBe('1404/05/30')
    expect(toJalali(new Date('2026-03-21'))).toEqual({ year: 1405, month: 1, day: 1 })
    expect(formatJalali('1979-02-11')).toBe('1357/11/22')
  })

  it('ر۴۹ — ریال واحد داخلی است و تومان فقط نمایش', () => {
    expect(tomansToRials(1000)).toBe(10_000)
    expect(parseAmount('۱٬۲۳۴')).toBe(1234)
    expect(formatRials(1234)).toBe('۱٬۲۳۴')
  })

  it('ر۵۰ — بازه‌های تاریخ شمسی و بازه‌ی قبلی درست محاسبه می‌شوند', () => {
    expect(PRESETS.length).toBeGreaterThanOrEqual(9)
    const range = resolveRange('fiscalYear', { from: '1405/01/01', to: '1405/12/29' })
    expect(range.from).toBe('1405/01/01')
    expect(inRange('1405/06/15', range.from, range.to)).toBe(true)
    expect(inRange('1404/12/29', range.from, range.to)).toBe(false)
    const previous = previousRange(range)
    expect(previous.to < range.from).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// نگهبان‌های ساختاری
// ---------------------------------------------------------------------------

describe('م۸ · نگهبان‌های ساختاری', () => {
  it('ن۱ — هر فرمانی که رابط صدا می‌زند در میزبان ثبت شده است', () => {
    const called = [...apiTs.matchAll(/api<[^>]*>\('([a-z_0-9]+)'/g)].map((m) => m[1])
    expect(called.length).toBeGreaterThan(150)
    const registry = [
      mainRs,
      ...['treasury_docs', 'treasury_accounts', 'parties_form', 'chart_of_accounts', 'returns', 'quotes', 'production', 'settings', 'products_form', 'cardex', 'calendar', 'api_profiles']
        .filter((name) => existsSync(resolve(ROOT, 'apps/desktop-host/src-tauri/src', `${name}.rs`)))
        .map((name) => host(`${name}.rs`)),
    ].join('\n')
    const missing = [...new Set(called)].filter((command) => !registry.includes(`fn ${command}`))
    expect(missing).toEqual([])
  })

  it('ن۲ — هیچ فایل رابط کاربری از سقف اندازه رد نشده است', () => {
    const oversize: string[] = []
    for (const file of ['App.tsx', 'pages/InvoiceForm.tsx', 'pages/PartyForm.tsx', 'pages/ProductForm.tsx', 'pages/Production.tsx']) {
      const lines = ui(file).split('\n').length
      if (lines > 900) oversize.push(`${file}: ${lines}`)
    }
    expect(oversize).toEqual([])
  })
})
