/**
 * @vitest-environment jsdom
 *
 * ممیزی دور ۱۰ — فرم «افزودن کالا» به‌صورت رفتاری.
 *
 * مرجع: تصاویر `6FM9Ow` (انتخاب نوع کالا)، `NztJl5` (فرم تعریف کالا) و
 * `8Xmc1p` (لیست کالاها) از نرم‌افزار فعلی — `docs/FEATURE_BASELINE.md` بخش ۳.
 *
 * ## چه چیزی اینجا سنجیده می‌شود که جای دیگر سنجیده نمی‌شود
 *
 * تست‌های Rust درستیِ «قواعد» را می‌سنجند؛ این پرونده درستیِ «فرم» را:
 * چه چیزی کاربر می‌بیند، چه چیزی با هر حرکت به میزبان فرستاده می‌شود و
 * وقتی میزبان خطا می‌دهد فرم چه واکنشی نشان می‌دهد. فرم با ماک کامل API
 * رندر می‌شود و همه‌ی ادعاها روی DOM و payload واقعی‌اند — نه روی متن فایل.
 */
import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor, within, cleanup } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { ProductForm } from '../pages/ProductForm'
import { I18nProvider } from '../lib/i18n'
import { parseAmount } from '../lib/format'
import {
  getProductGroups,
  getProductKinds,
  getProductProfile,
  previewGoldPrice,
  saveProductProfile,
  type PriceLevelOption,
  type ProductDetail,
  type ProductKindOption,
} from '../api'

vi.mock('../api', () => ({
  getProductKinds: vi.fn(),
  getProductGroups: vi.fn(),
  getProductProfile: vi.fn(),
  saveProductProfile: vi.fn(),
  previewGoldPrice: vi.fn(),
}))

const SRC = resolve(__dirname, '..')
const ROOT = resolve(SRC, '../../..')
const read = (path: string) => readFileSync(resolve(SRC, path), 'utf8')

// ---------------------------------------------------------------------------
// داده‌ی ماک — همان چیزی که list_product_kinds میزبان برمی‌گرداند
// ---------------------------------------------------------------------------

const KINDS: ProductKindOption[] = [
  { value: 'simple', label: 'کالای عمومی (ساده)', tracks_inventory: true },
  { value: 'composite', label: 'کالای مرکب', tracks_inventory: true },
  { value: 'variant', label: 'کالای تنوع‌دار', tracks_inventory: true },
  { value: 'gold_jewelry', label: 'طلا و جواهر', tracks_inventory: true },
  { value: 'service', label: 'خدمت', tracks_inventory: false },
]

const LEVELS: PriceLevelOption[] = [
  { value: 'retail', label: 'جزئی' },
  { value: 'wholesale', label: 'کلی' },
  { value: 'partner', label: 'همکار' },
  { value: 'partner_tier2', label: 'همکار درجه ۲' },
  { value: 'partner_tier3', label: 'همکار درجه ۳' },
  { value: 'seasonal', label: 'فصلی' },
  { value: 'exhibition', label: 'نمایشگاه' },
]

function detailFixture(overrides: Partial<ProductDetail> = {}): ProductDetail {
  return {
    id: 'gold-1',
    kind: 'gold_jewelry',
    kind_label: 'طلا و جواهر',
    sku: 'GOLD-01',
    barcode: '6901234567890',
    name: 'گردنبند طلا',
    display_name: 'گردنبند هدیه',
    brand: 'طلای نوین',
    group_id: 'g1',
    group_title: 'طلا',
    unit: 'عدد',
    sale_price: 0,
    purchase_price: 400_000_000,
    min_stock: 1,
    max_stock: 10,
    reorder_point: 2,
    vat_basis_points: 900,
    duty_basis_points: 0,
    tax_code: 'URN123',
    tax_exempt: false,
    prices: LEVELS.map((level) => ({ level: level.value, label: level.label, price: null })),
    units: [],
    tiers: [],
    gold: { weight_grams: 10, carat: 18, making_charge_bp: 700, profit_bp: 500 },
    stock: [],
    total_stock: 0,
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// ابزارک‌های مشترک
// ---------------------------------------------------------------------------

async function renderForm(props: { productId?: string } = {}) {
  const onClose = vi.fn()
  const onSaved = vi.fn()
  render(
    <I18nProvider initialLocale="fa">
      <ProductForm productId={props.productId} onClose={onClose} onSaved={onSaved} />
    </I18nProvider>,
  )
  await screen.findByText(props.productId ? 'ویرایش کالا' : 'تعریف کالای جدید')
  return { onClose, onSaved }
}

const tabButtons = () =>
  Array.from(document.querySelectorAll('.tab-bar button')).map((button) => button.textContent)

const activeTab = () => document.querySelector('.tab-bar button.active')?.textContent ?? ''

const openTab = (label: string) =>
  fireEvent.click(
    within(document.querySelector('.tab-bar') as HTMLElement).getByRole('button', { name: label }),
  )

async function selectKind(label: string) {
  fireEvent.click(screen.getByRole('combobox', { name: 'نوع کالا' }))
  fireEvent.click(await screen.findByRole('option', { name: label }))
}

/** ورودی مبلغ فرم نمایش جداکننده دارد و با blur خام commit می‌شود. */
function fillMoney(label: string, raw: string) {
  const input = screen.getByLabelText(label) as HTMLInputElement
  fireEvent.focus(input)
  fireEvent.change(input, { target: { value: raw } })
  fireEvent.blur(input)
}

function setInput(label: string | RegExp, value: string) {
  fireEvent.change(screen.getByLabelText(label), { target: { value } })
}

const clickSave = () => fireEvent.click(screen.getByRole('button', { name: 'ذخیره کالا' }))

const lastPayload = () => vi.mocked(saveProductProfile).mock.calls.at(-1)![0]

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(getProductKinds).mockResolvedValue({ kinds: KINDS, levels: LEVELS })
  vi.mocked(getProductGroups).mockResolvedValue([])
  vi.mocked(saveProductProfile).mockResolvedValue('product-new-1')
})

afterEach(cleanup)

// ---------------------------------------------------------------------------
// ساختار فرم — تصویر NztJl5
// ---------------------------------------------------------------------------

describe('م۱۰ · چیدمان فرم افزودن کالا (NztJl5)', () => {
  it('ف۱ — زبانه‌های کالای ساده به ترتیب مرجع‌اند و زبانه‌ی طلا پنهان است', async () => {
    await renderForm()
    expect(tabButtons()).toEqual([
      'مشخصات عمومی',
      'سطوح قیمت',
      'چند واحدی',
      'اطلاعات مالیاتی',
      'موجودی و سفارش',
      'تخفیف پلکانی',
    ])
    expect(screen.queryByRole('button', { name: 'طلا و جواهر' })).toBeNull()
  })

  it('ف۲ — دیالوگ نوع کالا هر پنج نوع مرجع را ارائه می‌دهد (6FM9Ow)', async () => {
    await renderForm()
    fireEvent.click(screen.getByRole('combobox', { name: 'نوع کالا' }))
    const labels = (await screen.findAllByRole('option')).map((option) => option.textContent)
    expect(labels).toEqual([
      'کالای عمومی (ساده)',
      'کالای مرکب',
      'کالای تنوع‌دار',
      'طلا و جواهر',
      'خدمت',
    ])
  })

  it('ف۳ — نوع طلا زبانه‌ی طلا را می‌آورد و نوع خدمت زبانه‌های انباری را می‌برد', async () => {
    await renderForm()
    await selectKind('طلا و جواهر')
    expect(tabButtons()).toContain('طلا و جواهر')

    await selectKind('خدمت')
    const tabs = tabButtons()
    expect(tabs).not.toContain('موجودی و سفارش')
    expect(tabs).not.toContain('چند واحدی')
    expect(tabs).toContain('سطوح قیمت')
    // یادداشت خدمت باید در زبانه‌ی عمومی دیده شود
    expect(document.body.textContent).toContain('خدمت موجودی انبار ندارد')
  })

  it('ف۴ — کالای تازه با پیش‌فرض‌های درست ذخیره می‌شود (مالیات ۹٪، هفت سطح خالی)', async () => {
    await renderForm()
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalledTimes(1))
    const payload = lastPayload()
    expect(payload.kind).toBe('simple')
    expect(payload.unit).toBe('عدد')
    expect(payload.vat_basis_points).toBe(900)
    expect(payload.duty_basis_points).toBe(0)
    expect(payload.tax_exempt).toBe(false)
    expect(payload.prices).toEqual(
      LEVELS.map((level) => ({ level: level.value, price: null })),
    )
    expect(payload.units).toEqual([])
    expect(payload.tiers).toEqual([])
    expect(payload.purchase_price).toBe(0)
  })

  it('ف۵ — شناسه‌های کالا (کد، بارکد، نام، برند، نام نمایشی) همه به میزبان می‌روند', async () => {
    await renderForm()
    setInput('کد کالا ★', 'SHIRT-01')
    fireEvent.change(screen.getByPlaceholderText('با بارکدخوان هم می‌توانید پر کنید'), {
      target: { value: '6901234567890' },
    })
    setInput('نام کالا ★', 'پیراهن مردانه')
    setInput('نام نمایشی روی فاکتور', 'پیراهن کلاسیک')
    setInput('برند', 'نوین‌پوش')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    const payload = lastPayload()
    expect(payload.sku).toBe('SHIRT-01')
    expect(payload.barcode).toBe('6901234567890')
    expect(payload.name).toBe('پیراهن مردانه')
    expect(payload.display_name).toBe('پیراهن کلاسیک')
    expect(payload.brand).toBe('نوین‌پوش')
  })

  it('ف۶ — ورودی مبلغ، ارقام فارسی و جداکننده‌ی هزارگان هر دو را می‌فهمد', async () => {
    await renderForm()
    fillMoney('قیمت خرید (ریال)', '1,500,000')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().purchase_price).toBe(1_500_000)

    fillMoney('قیمت خرید (ریال)', '۱٬۵۰۰٬۰۰۰')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalledTimes(2))
    expect(lastPayload().purchase_price).toBe(1_500_000)

    // موتور ورودی مبلغ خودش هم در برابر ورودی‌های ساختگی مقاوم است
    expect(parseAmount('۲٬۰۰۰')).toBe(2000)
    expect(parseAmount('')).toBeNull()
    expect(parseAmount('abc')).toBeNull()
  })
})

// ---------------------------------------------------------------------------
// سطوح قیمت — تصویر NztJl5 زبانه‌ی «سطوح قیمت‌ها»
// ---------------------------------------------------------------------------

describe('م۱۰ · سطوح قیمت', () => {
  it('ف۷ — هر هفت سطح مرجع با برچسب فارسی خودشان فیلد دارند', async () => {
    await renderForm()
    openTab('سطوح قیمت')
    for (const label of LEVELS.map((level) => level.label)) {
      expect(screen.getByLabelText(label), label).toBeTruthy()
    }
  })

  it('ف۸ — وارد کردن قیمت فقط همان سطح را پر می‌کند؛ بقیه خالی می‌مانند', async () => {
    await renderForm()
    openTab('سطوح قیمت')
    fillMoney('جزئی', '2500000')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    const prices = Object.fromEntries(
      lastPayload().prices.map((row) => [row.level, row.price]),
    )
    expect(prices).toEqual({
      retail: 2_500_000,
      wholesale: null,
      partner: null,
      partner_tier2: null,
      partner_tier3: null,
      seasonal: null,
      exhibition: null,
    })
  })

  it('ف۹ — پاک کردن قیمت، سطح را «تعریف‌نشده» می‌کند تا زنجیره‌ی جایگزینی کار کند', async () => {
    await renderForm()
    openTab('سطوح قیمت')
    fillMoney('کلی', '950000')
    fillMoney('کلی', '')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    const wholesale = lastPayload().prices.find((row) => row.level === 'wholesale')
    expect(wholesale?.price).toBeNull()
    expect(
      screen.getByText('سطح خالی یعنی «تعریف نشده»', { exact: false }),
    ).toBeTruthy()
  })
})

// ---------------------------------------------------------------------------
// چند واحدی و تخفیف پلکانی
// ---------------------------------------------------------------------------

describe('م۱۰ · واحدهای فرعی و تخفیف پلکانی', () => {
  it('ف۱۰ — واحد فرعی با نام و ضریب ذخیره می‌شود و پیش‌فرض فروش یگانه است', async () => {
    await renderForm()
    openTab('چند واحدی')
    fireEvent.click(screen.getByRole('button', { name: 'افزودن واحد' }))
    setInput('نام واحد', 'کارتن')
    setInput('ضریب تبدیل', '12')
    fireEvent.click(screen.getByRole('button', { name: 'افزودن واحد' }))
    const names = screen.getAllByLabelText('نام واحد')
    const factors = screen.getAllByLabelText('ضریب تبدیل')
    fireEvent.change(names[1], { target: { value: 'دسته' } })
    fireEvent.change(factors[1], { target: { value: '0.5' } })
    // پیش‌فرض فروش دومی → اولی باید خودش خاموش شود
    const defaults = screen.getAllByLabelText('واحد پیش‌فرض فروش')
    fireEvent.click(defaults[1])
    expect((defaults[0] as HTMLInputElement).checked).toBe(false)
    expect((defaults[1] as HTMLInputElement).checked).toBe(true)

    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().units).toEqual([
      { unit_name: 'کارتن', factor: 12, is_default_sale: false },
      { unit_name: 'دسته', factor: 0.5, is_default_sale: true },
    ])
  })

  it('ف۱۱ — پله‌ی تخفیف درصد را به پایه‌نقطه تبدیل می‌کند (۱۵٪ → ۱۵۰۰bp)', async () => {
    await renderForm()
    openTab('تخفیف پلکانی')
    fireEvent.click(screen.getByRole('button', { name: 'افزودن پله' }))
    setInput('از مقدار', '10')
    setInput('درصد تخفیف', '15')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().tiers).toEqual([{ min_quantity: 10, discount_bp: 1500 }])
  })
})

// ---------------------------------------------------------------------------
// مالیات — زبانه‌ی «اطلاعات مالیاتی» و سامانه مؤدیان
// ---------------------------------------------------------------------------

describe('م۱۰ · اطلاعات مالیاتی', () => {
  it('ف۱۲ — معافیت، انتخاب نرخ و نرخ عوارض را قفل می‌کند و در payload می‌نشیند', async () => {
    await renderForm()
    openTab('اطلاعات مالیاتی')
    fireEvent.click(screen.getByLabelText('کالای معاف از مالیات'))
    const vat = screen.getByRole('combobox', { name: 'نرخ ارزش افزوده' }) as HTMLButtonElement
    expect(vat.disabled).toBe(true)
    expect((screen.getByLabelText('نرخ عوارض (درصد×۱۰۰)') as HTMLInputElement).disabled).toBe(
      true,
    )
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().tax_exempt).toBe(true)
  })

  it('ف۱۳ — شناسه‌ی سامانه مؤدیان با جهت LTR ذخیره می‌شود', async () => {
    await renderForm()
    openTab('اطلاعات مالیاتی')
    const field = screen.getByLabelText('شناسه کالا در سامانه مؤدیان') as HTMLInputElement
    expect(field.dir).toBe('ltr')
    fireEvent.change(field, { target: { value: 'URN123456' } })
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().tax_code).toBe('URN123456')
  })
})

// ---------------------------------------------------------------------------
// طلا و جواهر — تصویر 6FM9Ow نوع F4
// ---------------------------------------------------------------------------

describe('م۱۰ · طلا و جواهر', () => {
  it('ف۱۴ — وزن، عیار، اجرت و سود — همه به پایه‌نقطه در payload', async () => {
    await renderForm()
    await selectKind('طلا و جواهر')
    openTab('طلا و جواهر')
    setInput('وزن (گرم) ★', '10')
    fireEvent.click(screen.getByRole('combobox', { name: 'عیار' }))
    fireEvent.click(await screen.findByRole('option', { name: '۲۱ عیار' }))
    setInput('اجرت ساخت (درصد)', '7')
    setInput('سود فروشنده (درصد)', '5')
    clickSave()
    await waitFor(() => expect(saveProductProfile).toHaveBeenCalled())
    expect(lastPayload().gold).toEqual({
      weight_grams: 10,
      carat: 21,
      making_charge_bp: 700,
      profit_bp: 500,
    })
  })

  it('ف۱۵ — پیش‌نمایش قیمت طلا فقط برای کالای ذخیره‌شده هست و نرخ را می‌گیرد', async () => {
    // کالای تازه: کالای طلا هنوز ذخیره نشده، پیش‌نمایش نباید باشد
    await renderForm()
    await selectKind('طلا و جواهر')
    openTab('طلا و جواهر')
    expect(screen.queryByRole('button', { name: 'محاسبه' })).toBeNull()

    cleanup()
    vi.mocked(getProductProfile).mockResolvedValue(detailFixture())
    vi.mocked(previewGoldPrice).mockResolvedValue({
      metal_value: 500_000_000,
      making_charge: 35_000_000,
      profit: 26_750_000,
      vat: 5_557_500,
      total: 567_307_500,
    })
    await renderForm({ productId: 'gold-1' })
    openTab('طلا و جواهر')
    setInput('نرخ هر گرم (ریال)', '50000000')
    fireEvent.click(screen.getByRole('button', { name: 'محاسبه' }))
    await waitFor(() => expect(previewGoldPrice).toHaveBeenCalledWith('gold-1', 50_000_000))
    // تفکیک کامل باید نمایش داده شود — با ارقام فارسی
    await waitFor(() =>
      expect(document.body.textContent).toContain('۵۰۰٬۰۰۰٬۰۰۰'),
    )
    expect(document.body.textContent).toContain('قابل پرداخت')
  })
})

// ---------------------------------------------------------------------------
// ویرایش و بارگذاری مجدد
// ---------------------------------------------------------------------------

describe('م۱۰ · ویرایش کالا', () => {
  it('ف۱۶ — پروفایل موجود در همه‌ی فیلدها پیش‌فرض می‌شود', async () => {
    vi.mocked(getProductProfile).mockResolvedValue(
      detailFixture({
        kind: 'simple',
        kind_label: 'کالای عمومی (ساده)',
        sku: 'SHIRT-99',
        name: 'پیراهن ویرایشی',
        purchase_price: 800_000,
      }),
    )
    await renderForm({ productId: 'gold-1' })
    expect((screen.getByLabelText('کد کالا ★') as HTMLInputElement).value).toBe('SHIRT-99')
    expect((screen.getByLabelText('نام کالا ★') as HTMLInputElement).value).toBe('پیراهن ویرایشی')
    // نشان نوع کالا پایین فرم (combobox هم همان متن را دارد، پس چندتایی است)
    expect(screen.getAllByText('کالای عمومی (ساده)').length).toBeGreaterThanOrEqual(2)
    expect(getProductProfile).toHaveBeenCalledWith('gold-1')
  })
})

// ---------------------------------------------------------------------------
// خطاها — قرارداد کد ITM بین فرم و میزبان
// ---------------------------------------------------------------------------

describe('م۱۰ · رفتار فرم با خطاهای میزبان', () => {
  it('ف۱۷ — خطای اعتبارسنجی با کد پیگیری نمایش داده می‌شود و دکمه آزاد می‌ماند', async () => {
    vi.mocked(saveProductProfile).mockRejectedValue('ITM-003: کد کالا نمی‌تواند خالی باشد')
    await renderForm()
    clickSave()
    const box = await screen.findByText('کد کالا نمی‌تواند خالی باشد', { exact: false })
    expect(box.textContent).toContain('ITM-003')
    expect((screen.getByRole('button', { name: 'ذخیره کالا' }) as HTMLButtonElement).disabled).toBe(
      false,
    )
    // خطای زبانه‌ی عمومی نباید کاربر را به زبانه‌ی دیگر ببرد
    expect(activeTab()).toBe('مشخصات عمومی')
  })

  it('ف۱۸ — خطای قیمت (ITM-009) فرم را به زبانه‌ی سطوح قیمت می‌برد', async () => {
    vi.mocked(saveProductProfile).mockRejectedValue('ITM-009: قیمت سطح نمی‌تواند منفی باشد')
    await renderForm()
    clickSave()
    await waitFor(() => expect(activeTab()).toBe('سطوح قیمت'))
  })

  it('ف۱۹ — خطای واحد فرعی (ITM-012) فرم را به زبانه‌ی چند واحدی می‌برد', async () => {
    vi.mocked(saveProductProfile).mockRejectedValue(
      'ITM-012: CAT-004: ضریب تبدیل واحد باید بزرگ‌تر از صفر باشد',
    )
    await renderForm()
    clickSave()
    await waitFor(() => expect(activeTab()).toBe('چند واحدی'))
  })

  it('ف۲۰ — خطای طلا (ITM-017) فرم را به زبانه‌ی طلا می‌برد', async () => {
    vi.mocked(saveProductProfile).mockRejectedValue(
      'ITM-017: وزن کالای طلا باید بیشتر از صفر باشد',
    )
    await renderForm()
    await selectKind('طلا و جواهر')
    clickSave()
    await waitFor(() => expect(activeTab()).toBe('طلا و جواهر'))
  })

  it('ف۲۱ — کد تکراری (ITM-020) کاربر را در زبانه‌ی عمومی نگه می‌دارد', async () => {
    vi.mocked(saveProductProfile).mockRejectedValue(
      'ITM-020: کد کالا تکراری است یا ثبت انجام نشد',
    )
    await renderForm()
    openTab('سطوح قیمت')
    clickSave()
    await waitFor(() => expect(screen.getByText(/ITM-020/)).toBeTruthy())
    await waitFor(() => expect(activeTab()).toBe('مشخصات عمومی'))
  })

  it('ف۲۲ — ذخیره‌ی موفق onSaved را با شناسه صدا می‌زند و فرم را نمی‌بندد', async () => {
    const { onClose, onSaved } = await renderForm()
    setInput('کد کالا ★', 'OK-1')
    clickSave()
    await waitFor(() => expect(onSaved).toHaveBeenCalledWith('product-new-1'))
    expect(onClose).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// قرارداد سورس — دیوار پشتیبان تست‌های رفتاری
// ---------------------------------------------------------------------------

describe('م۱۰ · قرارداد فرم و میزبان (نگهبان سورس)', () => {
  const form = read('pages/ProductForm.tsx')

  it('ف۲۳ — نگاشت خطا روی کدهای واقعی ITM است، نه کدهای مرده‌ی PRD', () => {
    expect(form).not.toMatch(/PRD-\d/)
    // شش شاخه‌ی نگاشت: مالیات، قیمت، واحد، پله، طلا، عمومی
    expect(form).toContain('ITM-007')
    expect(form).toMatch(/ITM-00\[89\]\|ITM-021/)
    expect(form).toMatch(/ITM-01\[0123\]/)
    expect(form).toMatch(/ITM-01\[45\]/)
    expect(form).toMatch(/ITM-01\[678\]/)
    expect(form).toMatch(/ITM-0\(0\[23456\]\|19\|20\)/)
  })

  it('ف۲۴ — همه‌ی کدهای ITM که فرم نگاشت می‌کند در میزبان واقعاً تعریف شده‌اند', () => {
    const host = readFileSync(
      resolve(ROOT, 'apps/desktop-host/src-tauri/src/products_form.rs'),
      'utf8',
    )
    // بازه‌ی کامل کدهایی که شش شاخه‌ی نگاشت فرم می‌پوشانند
    const mapped = [
      'ITM-002',
      'ITM-003',
      'ITM-004',
      'ITM-005',
      'ITM-006',
      'ITM-007',
      'ITM-008',
      'ITM-009',
      'ITM-010',
      'ITM-011',
      'ITM-012',
      'ITM-013',
      'ITM-014',
      'ITM-015',
      'ITM-016',
      'ITM-017',
      'ITM-018',
      'ITM-019',
      'ITM-020',
      'ITM-021',
    ]
    for (const code of mapped) {
      expect(host, `${code} باید در میزبان باشد`).toContain(code)
    }
  })

  it('ف۲۵ — فرم کالا هیچ واحد پولی سخت‌کدشده‌ای ندارد و از rialUnit می‌گیرد', () => {
    expect(form).not.toMatch(/\}\s*ریال/)
    expect(form).toContain('rialUnit()')
  })

  it('ف۲۶ — دستورهای Tauri فرم کالا همان‌هایی هستند که میزبان ثبت کرده', () => {
    const api = read('api.ts')
    const host = readFileSync(
      resolve(ROOT, 'apps/desktop-host/src-tauri/src/main.rs'),
      'utf8',
    )
    for (const command of [
      'list_product_kinds',
      'list_product_groups',
      'get_product_profile',
      'save_product_profile',
      'preview_gold_price',
    ]) {
      expect(api, command).toContain(`'${command}'`)
      expect(host, command).toContain(command)
    }
  })
})
