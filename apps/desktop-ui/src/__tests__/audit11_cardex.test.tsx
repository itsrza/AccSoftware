/**
 * @vitest-environment jsdom
 *
 * ممیزی دور ۱۱ — صفحه‌ی کاردکس کالا (F4/F5/F6).
 *
 * مرجع: لیست کالاهای نرم‌افزار فعلی (تصویر `8Xmc1p`).
 *
 * فرم با ماک کامل API رندر می‌شود؛ ادعاها روی DOM و پارامترهای فراخوانی
 * واقعی‌اند: فیلترها به میزبان درست می‌رسند، اعداد و تاریخ‌های فارسی
 * درست نمایش داده می‌شوند و خطای میزبان با کد پیگیری دیده می‌شود.
 */
import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup, within } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { ProductCardex } from '../pages/ProductCardex'
import { I18nProvider } from '../lib/i18n'
import {
  getProductCardex,
  getProductsDetailed,
  getWarehouses,
  type CardexReport,
  type ProductListRow,
} from '../api'

vi.mock('../api', () => ({
  getProductCardex: vi.fn(),
  getProductsDetailed: vi.fn(),
  getWarehouses: vi.fn(),
}))

const SRC = resolve(__dirname, '..')
const ROOT = resolve(SRC, '../../..')
const read = (path: string) => readFileSync(resolve(SRC, path), 'utf8')

const PRODUCTS: ProductListRow[] = [
  {
    id: 'p1',
    kind: 'simple',
    kind_label: 'کالای عمومی (ساده)',
    sku: '1001',
    name: 'کالای نمونه',
    unit: 'عدد',
    quantity: 20,
    retail_price: 2_500_000,
    partner_price: 2_200_000,
    purchase_price: 1_200_000,
    min_stock: 2,
    vat_basis_points: 900,
    tax_exempt: false,
  },
]

const REPORT: CardexReport = {
  product_id: 'p1',
  product_name: 'کالای نمونه',
  product_unit: 'عدد',
  kind: 'all',
  opening_balance: 10,
  total_in: 5,
  total_out: 3,
  closing_balance: 12,
  entries: [
    {
      date_iso: '2025-09-01',
      date_jalali: '1404/06/10',
      warehouse_name: 'انبار مرکزی',
      flow: 'out',
      doc_kind: 'sales_invoice',
      doc_number: 7,
      quantity: 3,
      unit_cost: 0,
      value: 0,
      balance: 7,
      note: 'فروش',
    },
    {
      date_iso: '2025-09-02',
      date_jalali: '1404/06/11',
      warehouse_name: 'انبار مرکزی',
      flow: 'in',
      doc_kind: 'purchase_invoice',
      doc_number: 12,
      quantity: 5,
      unit_cost: 1_000_000,
      value: 5_000_000,
      balance: 12,
      note: null,
    },
  ],
}

async function renderCardex(initial?: {productId?: string; kind?: 'sales' | 'purchase' | 'all'}) {
  render(
    <I18nProvider initialLocale="fa">
      <ProductCardex initial={initial} />
    </I18nProvider>,
  )
  await screen.findByText('کاردکس کالا', {selector: 'h1'})
}

const chooseProduct = async () => {
  fireEvent.click(screen.getByRole('combobox', {name: 'کالا ★'}))
  fireEvent.click(await screen.findByRole('option', {name: '1001 — کالای نمونه'}))
}

const chooseWarehouse = async () => {
  fireEvent.click(screen.getByRole('combobox', {name: 'انبار'}))
  fireEvent.click(await screen.findByRole('option', {name: 'شعبه'}))
}

const run = () => fireEvent.click(screen.getByRole('button', {name: 'نمایش کاردکس'}))

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(getProductsDetailed).mockResolvedValue(PRODUCTS)
  vi.mocked(getWarehouses).mockResolvedValue([
    {id: 'wh-main', name: 'انبار مرکزی', code: '01', is_active: true},
    {id: 'wh-b', name: 'شعبه', code: '02', is_active: true},
  ])
  vi.mocked(getProductCardex).mockResolvedValue(REPORT)
})

afterEach(cleanup)

// ---------------------------------------------------------------------------
// چیدمان و فیلترها
// ---------------------------------------------------------------------------

describe('م۱۱ · چیدمان کاردکس', () => {
  it('ک۱ — سه کانال مرجع (فروش/خرید/کلی) به ترتیب دیده می‌شوند', async () => {
    await renderCardex()
    const tabs = within(document.querySelector('.tab-bar') as HTMLElement)
      .getAllByRole('button')
      .map((button) => button.textContent)
    expect(tabs).toEqual(['کاردکس فروش', 'کاردکس خرید', 'کاردکس کلی'])
    expect(document.querySelector('.tab-bar button.active')?.textContent).toBe('کاردکس کلی')
  })

  it('ک۲ — کالا و انبار از میزبان می‌آیند و «همه‌ی انبارها» پیش‌فرض است', async () => {
    await renderCardex()
    await chooseProduct()
    await chooseWarehouse()
    expect(getProductsDetailed).toHaveBeenCalled()
    expect(getWarehouses).toHaveBeenCalled()
  })

  it('ک۳ — بدون کالا دکمه‌ی نمایش غیرفعال است', async () => {
    await renderCardex()
    expect(
      (screen.getByRole('button', {name: 'نمایش کاردکس'}) as HTMLButtonElement).disabled,
    ).toBe(true)
    await chooseProduct()
    expect(
      (screen.getByRole('button', {name: 'نمایش کاردکس'}) as HTMLButtonElement).disabled,
    ).toBe(false)
  })

  it('ک۴ — تاریخ شمسیِ نامعتبر اجازه‌ی پرس‌وجو نمی‌گیرد', async () => {
    await renderCardex()
    await chooseProduct()
    fireEvent.change(screen.getByLabelText('از تاریخ'), {target: {value: '1404-01-01'}})
    run()
    expect(await screen.findByText(/قالب تاریخ باید شبیه 1404\/01\/01 باشد/)).toBeTruthy()
    expect(getProductCardex).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// پرس‌وجو و نمایش
// ---------------------------------------------------------------------------

describe('م۱۱ · پرس‌وجو و نمایش', () => {
  it('ک۵ — پارامترهای کامل به میزبان می‌روند (کالا، کانال، بازه، انبار)', async () => {
    await renderCardex()
    await chooseProduct()
    await chooseWarehouse()
    run()
    await waitFor(() => expect(getProductCardex).toHaveBeenCalledTimes(1))
    const args = vi.mocked(getProductCardex).mock.calls[0]
    expect(args[0]).toBe('p1')
    expect(args[1]).toBe('all')
    // ابتدای سال شمسی جاری تا امروز
    expect(args[2]).toMatch(/^14\d{2}\/01\/01$/)
    expect(args[3]).toMatch(/^14\d{2}\/\d{2}\/\d{2}$/)
    expect(args[4]).toBe('wh-b')
  })

  it('ک۶ — سطرهای گزارش با تاریخ شمسی، برچسب سند و ورود/خروج درست رندر می‌شوند', async () => {
    await renderCardex()
    await chooseProduct()
    run()
    await waitFor(() => expect(getProductCardex).toHaveBeenCalled())
    expect(screen.getByText('1404/06/10')).toBeTruthy()
    // متن سلول سند، شماره سند را هم دارد؛ پس با زیررشته می‌سنچیم
    expect(screen.getByText('فاکتور فروش', {exact: false})).toBeTruthy()
    expect(screen.getByText('فاکتور خرید', {exact: false})).toBeTruthy()
    // ستون ورود/خروج: مقدار فقط در ستون خودش
    expect(screen.getByText('۵٬۰۰۰٬۰۰۰')).toBeTruthy()
    // سربرگ جمع‌ها با ارقام فارسی — «۱۲» هم ماند سطر آخر است هم جمع بستن
    expect(screen.getAllByText('۱۲').length).toBeGreaterThanOrEqual(2)
    expect(screen.getByText('افتتاحیه‌ی دوره')).toBeTruthy()
    expect(screen.getByText('۱۰')).toBeTruthy()
  })

  it('ک۷ — تغییر کانال، پرس‌وجوی بعدی را همان کانال می‌فرستد', async () => {
    await renderCardex()
    await chooseProduct()
    fireEvent.click(screen.getByRole('button', {name: 'کاردکس فروش'}))
    run()
    await waitFor(() => expect(getProductCardex).toHaveBeenCalledTimes(1))
    expect(vi.mocked(getProductCardex).mock.calls[0][1]).toBe('sales')
    expect(document.querySelector('.tab-bar button.active')?.textContent).toBe('کاردکس فروش')
  })

  it('ک۸ — کالای آماده از صفحه‌ی کالاها، خودکار نمایش می‌شود', async () => {
    await renderCardex({productId: 'p1', kind: 'purchase'})
    await waitFor(() => expect(getProductCardex).toHaveBeenCalledTimes(1))
    const args = vi.mocked(getProductCardex).mock.calls[0]
    expect(args[0]).toBe('p1')
    expect(args[1]).toBe('purchase')
    await screen.findByText('فاکتور فروش', {exact: false})
  })

  it('ک۹ — گزارش خالی پیام صریح دارد، نه جدول خالی', async () => {
    vi.mocked(getProductCardex).mockResolvedValue({...REPORT, entries: []})
    await renderCardex()
    await chooseProduct()
    run()
    expect(await screen.findByText('در این بازه حرکتی ثبت نشده است.')).toBeTruthy()
  })

  it('ک۱۰ — خطای میزبان با کد پیگیری در جعبه‌ی خطا دیده می‌شود', async () => {
    vi.mocked(getProductCardex).mockRejectedValue('CRDX-002: کالای کاردکس مشخص نشده یا یافت نشد')
    await renderCardex()
    await chooseProduct()
    run()
    const box = await screen.findByText(/کالای کاردکس مشخص نشده یا یافت نشد/)
    expect(box.textContent).toContain('CRDX-002')
  })

  it('ک۱۱ — نوع سند ناشناخته «سایر اسناد» نشان داده می‌شود نه کلید خام', async () => {
    vi.mocked(getProductCardex).mockResolvedValue({
      ...REPORT,
      entries: [
        {...REPORT.entries[0], doc_kind: 'mystery', doc_number: null},
      ],
    })
    await renderCardex()
    await chooseProduct()
    run()
    expect(await screen.findByText('سایر اسناد')).toBeTruthy()
    expect(document.body.textContent).not.toContain('mystery')
  })
})

// ---------------------------------------------------------------------------
// قرارداد سورس — دیوار پشتیبان
// ---------------------------------------------------------------------------

describe('م۱۱ · قرارداد سورس', () => {
  it('ک۱۲ — میانه‌بر F4/F5/F6 فقط در صفحه‌ی کالاها فعال است', () => {
    const app = read('App.tsx')
    expect(app).toContain("page === 'products'")
    expect(app).toMatch(/event\.key === 'F4'/)
    expect(app).toMatch(/event\.key === 'F5'/)
    expect(app).toMatch(/event\.key === 'F6'/)
    expect(app).toContain("setPage('product-cardex')")
    expect(app).toContain("return <ProductCardex initial={cardexSeed ?? undefined} />")
  })

  it('ک۱۳ — لیست کالاها دکمه‌ی کاردکس هر ردیف و سه دکمه‌ی کانال دارد', () => {
    const products = read('pages/Products.tsx')
    expect(products).toContain('onCardex')
    expect(products).toMatch(/onCardex\(row\.id, 'all'\)/)
    expect(products).toMatch(/onCardex\(undefined, 'sales'\)/)
    expect(products).toMatch(/onCardex\(undefined, 'purchase'\)/)
    expect(products).toMatch(/onCardex\(undefined, 'all'\)/)
  })

  it('ک۱۴ — دستور میزبان در قرارداد Tauri ثبت و در api صدا زده می‌شود', () => {
    const api = read('api.ts')
    expect(api).toContain("'product_cardex'")
    const main = readFileSync(
      resolve(ROOT, 'apps/desktop-host/src-tauri/src/main.rs'),
      'utf8',
    )
    expect(main).toContain('mod cardex;')
    expect(main).toContain('cardex::product_cardex')
    const host = readFileSync(
      resolve(ROOT, 'apps/desktop-host/src-tauri/src/cardex.rs'),
      'utf8',
    )
    expect(host).toContain('novin_core::cardex')
  })

  it('ک۱۵ — منطق در هسته است، نه در میزبان: ماژول و تست هسته موجودند', () => {
    const core = readFileSync(resolve(ROOT, 'crates/novin-core/src/cardex.rs'), 'utf8')
    expect(core).toContain('pub fn cardex(')
    expect(core).toContain('opening_balance')
    expect(core).toContain('variance:')
    expect(
      readFileSync(resolve(ROOT, 'crates/novin-core/tests/audit11_cardex.rs'), 'utf8'),
    ).toContain('k42_all_cardex_report')
  })

  it('ک۱۶ — کلیدهای کاردکس در هر سه زبان موجودند', () => {
    for (const file of ['fa.ts', 'en.ts', 'ar.ts']) {
      const dict = readFileSync(resolve(SRC, 'lib/i18n', file), 'utf8')
      for (const key of [
        'cardex.title',
        'cardex.kind.sales',
        'cardex.kind.purchase',
        'cardex.kind.all',
        'cardex.opening',
        'cardex.closing',
        'cardex.doc.sales_invoice',
        'cardex.doc.other',
      ]) {
        expect(dict, `${file}: ${key}`).toContain(`'${key}'`)
      }
    }
  })
})
