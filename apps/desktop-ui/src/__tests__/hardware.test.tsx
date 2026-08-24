/**
 * @vitest-environment jsdom
 *
 * سخت‌افزار فروشگاهی: بارکدخوان و چاپ.
 *
 * این دو قابلیت بیش از بقیه در معرض «کار می‌کند روی دستگاه من» هستند، پس
 * منطقشان از رابط کاربری جدا شده تا بدون سخت‌افزار هم قابل آزمودن باشد:
 * موتور تشخیص اسکن زمان را از بیرون می‌گیرد و موتور چاپ فقط رشته‌ی HTML
 * برمی‌گرداند.
 */
import { translate } from '../lib/i18n'
import { describe, expect, it, vi, afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { createScannerEngine, scannerOptionsFrom, DEFAULT_SCANNER } from '../lib/barcode'
import {
  COLUMN_LABEL,
  PAPER_WIDTH_MM,
  amountInWords,
  defaultDesign,
  parseDesign,
  renderBody,
  renderDocument,
  printStyles,
  type CompanyIdentity,
  type PrintDocument,
} from '../lib/printTemplate'
import { companyFrom } from '../lib/printing'

const src = (path: string) => readFileSync(resolve(__dirname, '..', path), 'utf8')

afterEach(cleanup)

// ---------------------------------------------------------------------------
// بارکدخوان
// ---------------------------------------------------------------------------
describe('بارکدخوان (HID Keyboard Wedge)', () => {
  /** شبیه‌سازی تایپ با فاصله‌ی زمانی دلخواه بین کاراکترها. */
  function feed(
    text: string,
    gapMs: number,
    options = DEFAULT_SCANNER,
    terminator: string | null = 'Enter',
  ) {
    const scans: string[] = []
    let clock = 1000
    const engine = createScannerEngine(options, (result) => scans.push(result.code), () => clock)
    for (const character of text) {
      clock += gapMs
      engine.handle({ key: character })
    }
    if (terminator) {
      clock += gapMs
      engine.handle({ key: terminator })
    }
    return { scans, engine }
  }

  it('س۱ — بارکد سریع با Enter پایانی تشخیص داده می‌شود', () => {
    const { scans } = feed('6260100100015', 8)
    expect(scans).toEqual(['6260100100015'])
  })

  it('س۲ — تایپ آهسته‌ی انسان اسکن به حساب نمی‌آید', () => {
    // ۲۰۰ میلی‌ثانیه بین کاراکترها = تایپ انسانی
    const { scans } = feed('6260100100015', 200)
    expect(scans).toEqual([])
  })

  it('س۳ — رشته‌ی کوتاه‌تر از حداقل رد می‌شود', () => {
    const { scans } = feed('123', 8)
    expect(scans).toEqual([])
  })

  it('س۴ — Enterِ کاربر به فرم می‌رسد و مصرف نمی‌شود', () => {
    let clock = 1000
    const engine = createScannerEngine(DEFAULT_SCANNER, () => undefined, () => clock)
    clock += 500
    const consumed = engine.handle({ key: 'Enter' })
    expect(consumed).toBe(false)
  })

  it('س۵ — دستگاه با پایان Tab پشتیبانی می‌شود', () => {
    const options = { ...DEFAULT_SCANNER, suffix: 'tab' as const }
    const { scans } = feed('6260100100015', 8, options, 'Tab')
    expect(scans).toEqual(['6260100100015'])
  })

  it('س۶ — بارکد حرفی-عددی هم خوانده می‌شود', () => {
    const { scans } = feed('ABC-1234-XY', 6)
    expect(scans).toEqual(['ABC-1234-XY'])
  })

  it('س۷ — کلید ترکیبی بافر را پاک می‌کند (میانبر برنامه نباید اسکن شود)', () => {
    let clock = 1000
    const scans: string[] = []
    const engine = createScannerEngine(DEFAULT_SCANNER, (r) => scans.push(r.code), () => clock)
    for (const character of '626010') {
      clock += 8
      engine.handle({ key: character })
    }
    clock += 8
    engine.handle({ key: 's', ctrlKey: true })
    clock += 8
    engine.handle({ key: 'Enter' })
    expect(scans).toEqual([])
  })

  it('س۸ — دستگاه بدون کاراکتر پایان با مهلت زمانی بسته می‌شود', () => {
    const options = { ...DEFAULT_SCANNER, suffix: 'none' as const }
    let clock = 1000
    const scans: string[] = []
    const engine = createScannerEngine(options, (r) => scans.push(r.code), () => clock)
    for (const character of '6260100100015') {
      clock += 8
      engine.handle({ key: character })
    }
    expect(engine.flushIfIdle()).toBe(false) // هنوز زود است
    clock += options.maxGapMs * 4
    expect(engine.flushIfIdle()).toBe(true)
    expect(scans).toEqual(['6260100100015'])
  })

  it('س۹ — بارکدخوان خاموش هیچ رویدادی نمی‌سازد', () => {
    const options = { ...DEFAULT_SCANNER, enabled: false }
    const { scans } = feed('6260100100015', 8, options)
    expect(scans).toEqual([])
  })

  it('س۱۰ — تنظیمات از مرکز تنظیمات خوانده می‌شود', () => {
    const options = scannerOptionsFrom([
      { key: 'hardware.barcode_enabled', value: 'false' },
      { key: 'hardware.barcode_min_length', value: '8' },
      { key: 'hardware.barcode_max_gap_ms', value: '40' },
      { key: 'hardware.barcode_suffix', value: 'tab' },
    ])
    expect(options).toEqual({ enabled: false, minLength: 8, maxGapMs: 40, suffix: 'tab' })
  })

  it('س۱۱ — فرم فاکتور اسکن را به افزودن کالا وصل کرده است', () => {
    const form = src('pages/InvoiceForm.tsx')
    expect(form).toContain('useBarcodeScanner')
    // جستجو باید هم بارکد و هم کد کالا را بپذیرد.
    expect(form).toMatch(/item\.barcode === normalized \|\| item\.sku === normalized/)
    // اسکن دوباره‌ی یک کالا باید مقدار را زیاد کند نه سطر تکراری بسازد.
    expect(form).toMatch(/quantity: line\.quantity \+ 1/)
  })
})

// ---------------------------------------------------------------------------
// چاپ
// ---------------------------------------------------------------------------
describe('موتور چاپ', () => {
  const company: CompanyIdentity = {
    name: 'فروشگاه نمونه',
    phone: '021-88776655',
    address: 'تهران، ولیعصر',
    economicCode: '411111111111',
    logo: 'data:image/png;base64,AAAA',
  }

  const document_: PrintDocument = {
    number: '1042',
    date: '1405/05/30',
    partyName: 'شرکت آریا',
    partyPhone: '09121234567',
    lines: [
      {
        code: 'P-1',
        name: 'روغن موتور',
        quantity: 2,
        unit: 'عدد',
        unit_price: 1_000_000,
        discount: 0,
        vat: 180_000,
        line_total: 2_180_000,
      },
    ],
    subtotal: 2_000_000,
    discount: 0,
    vat: 180_000,
    total: 2_180_000,
  }

  it('چ۱ — سربرگ نام، تلفن و لوگوی مجموعه را از تنظیمات می‌گیرد', () => {
    const html = renderBody(defaultDesign('invoice'), company, document_)
    expect(html).toContain('فروشگاه نمونه')
    expect(html).toContain('021-88776655')
    expect(html).toContain('data:image/png;base64,AAAA')
    expect(html).toContain('411111111111')
  })

  it('چ۲ — خاموش کردن هر بخش، آن را از خروجی حذف می‌کند', () => {
    const design = { ...defaultDesign('invoice'), showLogo: false, showPhone: false }
    const html = renderBody(design, company, document_)
    expect(html).not.toContain('data:image/png')
    expect(html).not.toContain('021-88776655')
    expect(html).toContain('فروشگاه نمونه')
  })

  it('چ۳ — فقط ستون‌های انتخاب‌شده چاپ می‌شوند و به همان ترتیب', () => {
    const design = { ...defaultDesign('receipt'), columns: ['name', 'line_total'] as const }
    const html = renderBody({ ...design, columns: [...design.columns] }, company, document_)
    expect(html).toContain(COLUMN_LABEL.name)
    expect(html).toContain(COLUMN_LABEL.line_total)
    expect(html).not.toContain(COLUMN_LABEL.unit_price)
    expect(html.indexOf(COLUMN_LABEL.name)).toBeLessThan(html.indexOf(COLUMN_LABEL.line_total))
  })

  it('چ۴ — اندازه‌ی کاغذ در CSS چاپ درست می‌نشیند', () => {
    const receipt = printStyles({ ...defaultDesign('receipt'), paper: '80mm' })
    expect(receipt).toContain('size: 80mm auto')
    expect(receipt).toContain(`width: ${PAPER_WIDTH_MM['80mm']}mm`)

    const invoice = printStyles({ ...defaultDesign('invoice'), paper: 'A4' })
    expect(invoice).toContain('size: A4')
  })

  it('چ۵ — مبلغ به حروف فارسی درست است', () => {
    expect(amountInWords(0)).toBe('صفر ریال')
    expect(amountInWords(1)).toBe('یک ریال')
    expect(amountInWords(15)).toBe('پانزده ریال')
    expect(amountInWords(120)).toBe('یکصد و بیست ریال')
    expect(amountInWords(1_000)).toBe('یک هزار ریال')
    expect(amountInWords(2_180_000)).toBe('دو میلیون و یکصد و هشتاد هزار ریال')
    expect(amountInWords(1_500_000_000)).toBe('یک میلیارد و پانصد میلیون ریال')
  })

  it('چ۶ — مبلغ به حروف در فاکتور رسمی پیش‌فرض روشن است', () => {
    // در صورتحساب رسمی ایرانی درج مبلغ به حروف الزامی است.
    expect(defaultDesign('invoice').showAmountInWords).toBe(true)
    const html = renderBody(defaultDesign('invoice'), company, document_)
    expect(html).toContain('به حروف:')
  })

  it('چ۷ — چند نسخه با شکست صفحه چاپ می‌شود', () => {
    const html = renderDocument(defaultDesign('receipt'), company, document_, 3)
    expect(html.match(/page-break-before:always/g)?.length).toBe(2)
  })

  it('چ۸ — ورودی کاربر در HTML خنثی می‌شود (بدون تزریق)', () => {
    const dirty = { ...document_, partyName: '<script>alert(1)</script>' }
    const html = renderBody(defaultDesign('invoice'), company, dirty)
    expect(html).not.toContain('<script>')
    expect(html).toContain('&lt;script&gt;')
  })

  it('چ۹ — قالب ذخیره‌شده به‌صورت JSON خوانده می‌شود و HTML قدیمی رد می‌شود', () => {
    const design = { ...defaultDesign('receipt'), paper: '58mm' as const }
    expect(parseDesign(JSON.stringify(design), 'receipt')?.paper).toBe('58mm')
    expect(parseDesign('<section>قدیمی</section>', 'receipt')).toBeNull()
    expect(parseDesign('{بد}', 'receipt')).toBeNull()
  })

  it('چ۱۰ — هویت مجموعه از تنظیمات خوانده می‌شود و «—» خالی حساب می‌شود', () => {
    const identity = companyFrom(
      [
        { key: 'company.display_name', value: 'فروشگاه الف' },
        { key: 'company.phone', value: '021-1' },
        { key: 'company.address', value: '—' },
      ],
      'پیش‌فرض',
    )
    expect(identity.name).toBe('فروشگاه الف')
    expect(identity.phone).toBe('021-1')
    expect(identity.address).toBe('')

    const fallback = companyFrom([], 'پیش‌فرض')
    expect(fallback.name).toBe('پیش‌فرض')
  })

  it('چ۱۱ — سند کامل، معتبر و خودبسنده است', () => {
    const html = renderDocument(defaultDesign('invoice'), company, document_)
    expect(html.startsWith('<!doctype html>')).toBe(true)
    expect(html).toContain('dir="rtl"')
    // استایل باید داخل خود سند باشد، وگرنه خروجی چاپ بدون قالب می‌شود.
    expect(html).toContain('<style>')
    expect(html).toContain('@page')
  })
})

// ---------------------------------------------------------------------------
// طراح قالب
// ---------------------------------------------------------------------------
describe('طراح بصری قالب چاپ', () => {
  const page = src('pages/PrintTemplates.tsx')

  it('ط۱ — دیگر ویرایشگر HTML خام نیست', () => {
    expect(page).not.toContain('template-editor')
    expect(page).not.toMatch(/<textarea/)
  })

  it('ط۲ — پیش‌نمایش با همان موتور چاپ رسم می‌شود', () => {
    // اگر پیش‌نمایش کد جدا داشته باشد، دیر یا زود با چاپگر فرق می‌کند.
    expect(page).toContain('renderBody(design, company, sampleDocument(t))')
    expect(page).toContain('printStyles(design)')
  })

  it('ط۳ — قالب به‌صورت JSON ذخیره می‌شود', () => {
    expect(page).toContain('JSON.stringify(design)')
  })

  it('ط۴ — بارگذاری لوگو در تنظیمات ذخیره می‌شود', () => {
    expect(page).toContain("setSetting('company.logo'")
    expect(page).toContain('readAsDataURL')
  })

  it('ط۵ — چاپ آزمایشی وجود دارد', () => {
    expect(page).toContain("t('print.testPrint')")
    expect(translate('fa', 'print.testPrint')).toBe('چاپ آزمایشی')
    expect(page).toContain('renderDocument(design, company, sampleDocument(t), 1)')
  })

  it('ط۶ — پیش‌نمایش در عرض واقعی کاغذ رسم می‌شود', () => {
    expect(page).toContain('PAPER_WIDTH_MM[design.paper]}mm')
  })
})

// ---------------------------------------------------------------------------
// چاپ در مرورگر
// ---------------------------------------------------------------------------
describe('ارسال به چاپگر', () => {
  it('پ۱ — چاپ با iframe پنهان انجام می‌شود و بعدش پاک می‌شود', async () => {
    const { printHtml } = await import('../lib/printing')
    const print = vi.fn()
    // jsdom تابع print ندارد؛ تزریقش می‌کنیم تا رفتار واقعی سنجیده شود.
    Object.defineProperty(window.HTMLIFrameElement.prototype, 'contentWindow', {
      configurable: true,
      get() {
        return { focus: () => undefined, print }
      },
    })
    const promise = printHtml('<!doctype html><html><body>سلام</body></html>')
    // onload در jsdom پس از نوشتن سند اجرا می‌شود.
    const frame = document.querySelector('iframe')
    expect(frame).not.toBeNull()
    frame?.dispatchEvent(new Event('load'))
    ;(frame as HTMLIFrameElement & { onload?: () => void }).onload?.()
    await promise
    expect(print).toHaveBeenCalled()
    expect(document.querySelector('iframe')).toBeNull()
  })
})
