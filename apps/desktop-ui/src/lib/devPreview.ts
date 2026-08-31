/**
 * شبیه‌ساز پیش‌نمایش طراحی — **فقط حالت توسعه در مرورگر**.
 *
 * وقتی برنامه داخل Tauri اجرا می‌شود این فایل هیچ کاری نمی‌کند. تنها کاربرد آن،
 * بازبینی چیدمان و تجربه‌ی کاربری در مرورگر است، جایی که پل IPC وجود ندارد.
 *
 * ⚠️ این ماژول بخشی از محصول نیست: در بیلد تولیدی Tauri هرگز فعال نمی‌شود و
 * رابط کاربری هنگام فعال بودن آن یک بنر هشدار نمایش می‌دهد.
 *
 * داده‌ی این فایل **آینه‌ی داده‌ی نمونه‌ی واقعی** است که در
 * `crates/novin-core/src/db/demo.rs` تولید می‌شود: همان تعداد، همان الگوی
 * نام‌گذاری و همان روابط. هدف این است که چیدمان با حجم واقعی داده سنجیده شود،
 * نه با سه ردیف نمایشی.
 */

import { buildExtraResponses } from './preview/extras'
import { calendarOverviewResponse } from './preview/calendar'

export const isTauriRuntime = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const isDesignPreview = () => import.meta.env.DEV && !isTauriRuntime()

// ---------------------------------------------------------------------------
// تولید داده — قطعی و بدون تصادف، دقیقاً مانند seeder هسته
// ---------------------------------------------------------------------------

const PRODUCT_NAMES: [string, string, string, number][] = [
  ['مواد غذایی', 'برنج ایرانی درجه یک', 'کیلوگرم', 980000],
  ['مواد غذایی', 'روغن آفتابگردان', 'لیتر', 720000],
  ['مواد غذایی', 'چای سیاه ممتاز', 'بسته', 1450000],
  ['کالاهای متفرقه', 'کارتن بسته‌بندی', 'عدد', 85000],
  ['کالاهای متفرقه', 'نوار چسب پهن', 'عدد', 62000],
  ['آرایشی بهداشتی', 'شامپو ضدشوره', 'عدد', 540000],
  ['آرایشی بهداشتی', 'کرم مرطوب‌کننده', 'عدد', 890000],
  ['مد و پوشاک', 'پیراهن مردانه', 'عدد', 3400000],
  ['مد و پوشاک', 'شلوار جین', 'عدد', 4800000],
  ['مواد اولیه', 'ورق فلزی', 'کیلوگرم', 1250000],
  ['مواد اولیه', 'پارچه نخی', 'متر', 640000],
  ['کالاهای متفرقه', 'لامپ ال‌ای‌دی', 'عدد', 320000],
]

const FIRST_NAMES = ['محمد', 'علی', 'رضا', 'حسین', 'مهدی', 'فاطمه', 'زهرا', 'مریم', 'سارا', 'نگار']
const LAST_NAMES = ['محمدی', 'احمدی', 'رضایی', 'حسینی', 'کریمی', 'موسوی', 'جعفری', 'قاسمی', 'نوری', 'صادقی']
const COMPANY_NAMES = [
  'بازرگانی پارس',
  'شرکت آریا تجارت',
  'فروشگاه زنجیره‌ای مهر',
  'توزیع کالای البرز',
  'پخش سراسری ایرانیان',
  'صنایع غذایی سپید',
  'گروه تجاری کوروش',
  'بازرگانی نیک‌اندیش',
]
const CITIES = ['تهران', 'مشهد', 'اصفهان', 'شیراز', 'تبریز', 'کرج']

const warehouses = [
  {id: 'wh-main', name: 'انبار مرکزی', code: 'W01', is_active: true},
  {id: 'wh-branch', name: 'انبار شعبه', code: 'W02', is_active: true},
  {id: 'wh-mashhad', name: 'انبار مشهد', code: 'W03', is_active: true},
  {id: 'wh-tehran', name: 'انبار تهران', code: 'W04', is_active: true},
  {id: 'wh-scrap', name: 'انبار ضایعات', code: 'W05', is_active: true},
]

const products = Array.from({length: 60}, (_, index) => {
  const [group, base, unit, price] = PRODUCT_NAMES[index % PRODUCT_NAMES.length]
  const variant = Math.floor(index / PRODUCT_NAMES.length) + 1
  const salePrice = price + (index % 7) * 25000
  return {
    id: `demo-prod-${String(index).padStart(3, '0')}`,
    sku: String(10001 + index),
    barcode: `690${String(1000000 + index).padStart(10, '0')}`,
    name: variant > 1 ? `${base} مدل ${variant}` : base,
    unit,
    group,
    sale_price: salePrice,
    purchase_price: Math.round((salePrice * 72) / 100),
    min_stock: (index % 5) + 3,
    quantity: (index % 40) + 8,
  }
})

const contacts = Array.from({length: 50}, (_, index) => {
  const legal = index % 4 === 0
  const name = legal
    ? `${COMPANY_NAMES[index % COMPANY_NAMES.length]} ${Math.floor(index / 8) + 1}`
    : `${FIRST_NAMES[index % FIRST_NAMES.length]} ${LAST_NAMES[Math.floor(index / 3) % LAST_NAMES.length]}`
  const role = index % 7
  // مانده‌ی قطعی: دو‌سوم بدهکار، بخشی بستانکار، بخشی بی‌حساب
  const balance =
    index % 9 === 0 ? 0 : index % 3 === 1 ? -((index % 7) + 1) * 12_500_000 : ((index % 11) + 1) * 8_400_000
  return {
    id: `demo-contact-${String(index).padStart(3, '0')}`,
    name,
    kind: legal ? 'company' : 'person',
    mobile: `0912${String(1000000 + index * 137).slice(0, 7)}`,
    is_customer: role !== 5,
    is_supplier: role === 5 || role === 6,
    legal,
    credit_limit: ((index % 5) + 1) * 200_000_000,
    balance,
  }
})

const jalaliDate = (index: number) =>
  `1405/${String((index % 6) + 1).padStart(2, '0')}/${String((index % 28) + 1).padStart(2, '0')}`

const salesInvoices = Array.from({length: 55}, (_, index) => {
  const contact = contacts[index % contacts.length]
  const lineCount = (index % 3) + 1
  let subtotal = 0
  for (let line = 0; line < lineCount; line += 1) {
    const product = products[(index * 3 + line) % products.length]
    subtotal += Math.round(product.sale_price * (line + (index % 4) + 1))
  }
  const tax = Math.floor((subtotal * 9) / 100)
  return {
    id: `demo-sale-${String(index).padStart(3, '0')}`,
    number: 1000 + index,
    invoice_date: jalaliDate(index),
    contact_id: contact.id,
    contact_name: contact.name,
    warehouse_id: warehouses[index % 3].id,
    warehouse_name: warehouses[index % 3].name,
    status: 'posted',
    payment_status: index % 3 === 0 ? 'unpaid' : 'paid',
    subtotal,
    discount: 0,
    tax,
    total: subtotal + tax,
  }
})

const purchaseInvoices = Array.from({length: 25}, (_, index) => {
  const product = products[(index * 5) % products.length]
  const quantity = (index % 20) + 5
  const subtotal = Math.round(product.purchase_price * quantity)
  const tax = Math.floor((subtotal * 9) / 100)
  const contact = contacts[(index * 7 + 5) % contacts.length]
  return {
    id: `demo-purchase-${String(index).padStart(3, '0')}`,
    number: 500 + index,
    invoice_date: jalaliDate(index + 3),
    contact_id: contact.id,
    contact_name: contact.name,
    warehouse_id: warehouses[index % 2].id,
    warehouse_name: warehouses[index % 2].name,
    status: 'posted',
    payment_status: 'paid',
    subtotal,
    discount: 0,
    tax,
    total: subtotal + tax,
  }
})

const RECEIVED_CHECK_STATUSES = ['in_hand', 'deposited', 'collected', 'bounced', 'endorsed']
const ISSUED_CHECK_STATUSES = ['outstanding', 'paid', 'bounced']
const checks = Array.from({length: 20}, (_, index) => ({
  id: `demo-check-${String(index).padStart(3, '0')}`,
  check_number: String(700100 + index),
  check_type: index % 4 === 3 ? 'issued' : 'received',
  party_name: contacts[(index * 5) % contacts.length].name,
  amount: ((index % 10) + 1) * 8_500_000,
  issue_date: jalaliDate(index),
  due_date: jalaliDate(index + 4),
  status:
    index % 4 === 3
      ? ISSUED_CHECK_STATUSES[index % ISSUED_CHECK_STATUSES.length]
      : RECEIVED_CHECK_STATUSES[index % RECEIVED_CHECK_STATUSES.length],
  bank_name: `بانک ${CITIES[index % CITIES.length]}`,
}))

const treasuryAccounts = [
  {id: 'treasury-cash-1', name: 'صندوق مرکزی', account_type: 'cash', balance: 184_500_000, is_active: true},
  {id: 'treasury-cash-2', name: 'صندوق فروشگاه', account_type: 'cash', balance: 42_300_000, is_active: true},
  {id: 'treasury-bank-mellat', name: 'بانک ملت — جاری ۱۲۳۴', account_type: 'bank', balance: 1_284_000_000, is_active: true},
  {id: 'treasury-bank-saderat', name: 'بانک صادرات — جاری ۵۶۷۸', account_type: 'bank', balance: 620_500_000, is_active: true},
  {id: 'treasury-petty', name: 'تنخواه اداری', account_type: 'petty_cash', balance: 15_000_000, is_active: true},
]

/** اسناد دریافت و پرداخت نمونه، متصل به همان اشخاص و حساب‌های خزانه. */
const TREASURY_METHODS = ['cash', 'check', 'bank_transfer', 'card_terminal', 'discount', 'offset']
const METHOD_LABELS: Record<string, string> = {
  cash: 'نقد',
  check: 'چک',
  bank_transfer: 'حواله بانکی',
  card_terminal: 'کارتخوان',
  discount: 'تخفیف نقدی',
  offset: 'تهاتر',
}
const treasuryDocuments = Array.from({length: 24}, (_, index) => {
  const contact = contacts[(index * 7) % contacts.length]
  const kind = index % 3 === 2 ? 'payment' : 'receipt'
  return {
    id: `demo-tdoc-${String(index).padStart(3, '0')}`,
    kind,
    kind_label: kind === 'receipt' ? 'دریافت' : 'پرداخت',
    number: index + 1,
    document_date: jalaliDate(index + 2),
    party_id: contact.id,
    party_name: contact.name,
    description: kind === 'receipt' ? 'دریافت بابت فاکتور فروش' : 'پرداخت به تأمین‌کننده',
    total: ((index % 9) + 2) * 11_000_000,
    status: 'posted',
    status_label: 'ثبت‌شده',
    journal_id: `demo-jrn-tdoc-${index}`,
    line_count: (index % 3) + 1,
  }
})

function treasuryDocumentLines(header: (typeof treasuryDocuments)[number]) {
  const count = header.line_count
  const share = Math.floor(header.total / count)
  return Array.from({length: count}, (_, index) => {
    const method = TREASURY_METHODS[(header.number + index) % TREASURY_METHODS.length]
    const amount = index === count - 1 ? header.total - share * (count - 1) : share
    const account = treasuryAccounts[(header.number + index) % treasuryAccounts.length]
    const needsAccount = ['cash', 'bank_transfer', 'card_terminal'].includes(method)
    return {
      id: `${header.id}-l${index}`,
      method,
      method_label: METHOD_LABELS[method],
      amount,
      description: undefined,
      treasury_account_id: needsAccount ? account.id : undefined,
      treasury_account_name: needsAccount ? account.name : undefined,
      terminal_id: method === 'card_terminal' ? `POS-${1000 + header.number}` : undefined,
      check_serial: method === 'check' ? String(810000 + header.number) : undefined,
      check_due_date: method === 'check' ? jalaliDate(header.number + 40) : undefined,
      check_bank_name: method === 'check' ? 'بانک ملت' : undefined,
      sayad_id: undefined,
      check_id: undefined,
    }
  })
}

/** درخت کدینگ نمونه با ساختار واقعی والد-فرزند و مانده‌ی تجمعی. */
const RAW_ACCOUNTS: Array<[string, string, number, string | null, string, number, number]> = [
  ['1000', 'دارایی ها', 0, null, 'debit', 0, 0],
  ['1100', 'موجودی نقد و بانک', 1, '1000', 'debit', 0, 0],
  ['1101', 'صندوق مرکزی', 2, '1100', 'debit', 1_840_000_000, 1_655_500_000],
  ['1103', 'اسناد دریافتنی', 1, '1000', 'debit', 620_000_000, 310_000_000],
  ['1200', 'حساب های دریافتنی', 1, '1000', 'debit', 0, 0],
  ['1201', 'حساب مشتریان', 2, '1200', 'debit', 4_820_000_000, 3_910_000_000],
  ['1300', 'موجودی کالا', 1, '1000', 'debit', 2_140_000_000, 1_180_000_000],
  ['2000', 'بدهی ها', 0, null, 'credit', 0, 0],
  ['2100', 'حساب های پرداختنی', 1, '2000', 'credit', 0, 0],
  ['2101', 'تأمین کنندگان', 2, '2100', 'credit', 1_260_000_000, 2_310_000_000],
  ['2401', 'مالیات بر ارزش افزوده', 1, '2000', 'credit', 90_000_000, 434_000_000],
  ['4000', 'درآمد فروش', 0, null, 'credit', 0, 0],
  ['4100', 'فروش کالا', 1, '4000', 'credit', 0, 4_820_000_000],
  ['4400', 'تخفیفات نقدی اعطایی', 1, '4000', 'debit', 42_000_000, 0],
  ['5000', 'بهای تمام شده', 0, null, 'debit', 0, 0],
  ['5100', 'بهای تمام شده کالای فروش رفته', 1, '5000', 'debit', 3_180_000_000, 0],
]
const LEVEL_TITLES = ['گروه', 'کل', 'معین', 'تفصیلی']
const NATURE_LABELS: Record<string, string> = {debit: 'بدهکار', credit: 'بستانکار', mixed: 'دوطرفه'}
const demoAccounts = RAW_ACCOUNTS.map(([code, name, level, parent, nature, debit, credit]) => {
  const children = RAW_ACCOUNTS.filter((row) => row[3] === code)
  const collect = (root: string): Array<(typeof RAW_ACCOUNTS)[number]> => {
    const self = RAW_ACCOUNTS.filter((row) => row[0] === root)
    const kids = RAW_ACCOUNTS.filter((row) => row[3] === root).flatMap((row) => collect(row[0]))
    return [...self, ...kids]
  }
  const branch = collect(code)
  return {
    id: `acc-${code}`,
    code,
    name,
    level,
    level_title: LEVEL_TITLES[level],
    parent_id: parent ? `acc-${parent}` : undefined,
    nature,
    nature_label: NATURE_LABELS[nature],
    is_active: true,
    is_postable: children.length === 0,
    child_count: children.length,
    debit,
    credit,
    rollup_balance: branch.reduce((sum, row) => sum + row[5] - row[6], 0),
    requires_subsidiary: code === '1201' || code === '2101',
    subsidiary_group_id: code === '1201' || code === '2101' ? 'subgroup-persons' : undefined,
  }
})

/** برگشت‌های نمونه، متصل به فاکتورهای واقعی دمو. */
const demoReturns = Array.from({length: 8}, (_, index) => {
  const invoice = salesInvoices[index * 3]
  const total = ((index % 4) + 1) * 9_500_000
  const tax = Math.round((total * 9) / 100)
  const status = index % 4 === 0 ? 'draft' : index % 7 === 6 ? 'cancelled' : 'posted'
  return {
    id: `demo-return-${String(index).padStart(3, '0')}`,
    number: index + 1,
    return_date: jalaliDate(index + 6),
    original_invoice_id: invoice.id,
    original_invoice_number: invoice.number,
    contact_id: invoice.contact_id,
    contact_name: invoice.contact_name,
    warehouse_name: invoice.warehouse_name,
    status,
    status_label: status === 'draft' ? 'پیش‌نویس' : status === 'posted' ? 'ثبت‌شده' : 'باطل‌شده',
    total,
    tax,
    grand_total: total + tax,
    journal_id: status === 'posted' ? `demo-jrn-return-${index}` : undefined,
    line_count: 2,
  }
})

/** حواله‌های انتقال نمونه بین انبارهای واقعی دمو. */
const demoTransfers = Array.from({length: 10}, (_, index) => {
  const product = products[(index * 6) % products.length]
  const from = warehouses[index % warehouses.length]
  const to = warehouses[(index + 1) % warehouses.length]
  return {
    id: `demo-transfer-${String(index).padStart(3, '0')}`,
    product_id: product.id,
    from_warehouse_id: from.id,
    to_warehouse_id: to.id,
    quantity: (index % 6) + 2,
    unit_cost: product.purchase_price,
    status: index % 3 === 0 ? 'in_transit' : index % 7 === 6 ? 'cancelled' : 'received',
    note: index % 2 === 0 ? 'تأمین موجودی شعبه' : undefined,
  }
})

/** پیش‌فاکتور و سفارش خرید نمونه، با وضعیت‌های متنوع. */
const QUOTE_STATUSES = ['draft', 'sent', 'accepted', 'rejected', 'converted']
const QUOTE_STATUS_LABELS: Record<string, string> = {
  draft: 'پیش‌نویس',
  sent: 'ارسال‌شده',
  accepted: 'پذیرفته‌شده',
  rejected: 'ردشده',
  expired: 'منقضی',
  converted: 'تبدیل به فاکتور',
  cancelled: 'باطل‌شده',
}
const demoQuotes = Array.from({length: 18}, (_, index) => {
  const sales = index % 3 !== 2
  const contact = contacts[(index * 5) % contacts.length]
  const subtotal = ((index % 7) + 3) * 14_500_000
  const discount = index % 4 === 0 ? Math.round(subtotal * 0.04) : 0
  const tax = Math.round(((subtotal - discount) * 9) / 100)
  const status = QUOTE_STATUSES[index % QUOTE_STATUSES.length]
  return {
    id: `demo-quote-${String(index).padStart(3, '0')}`,
    kind: sales ? 'sales_quote' : 'purchase_order',
    kind_label: sales ? 'پیش‌فاکتور فروش' : 'سفارش خرید',
    number: Math.floor(index / 2) + 1,
    issue_date: jalaliDate(index + 1),
    valid_until: jalaliDate(index + 30),
    contact_id: contact.id,
    contact_name: contact.name,
    warehouse_name: warehouses[index % warehouses.length].name,
    description: sales ? 'پیشنهاد قیمت' : 'سفارش تأمین موجودی',
    subtotal,
    discount,
    tax,
    total: subtotal - discount + tax,
    status,
    status_label: QUOTE_STATUS_LABELS[status],
    converted_invoice_id: status === 'converted' ? `demo-sale-${index}` : undefined,
    line_count: 3,
    is_expired: index % 9 === 8,
  }
})

/** فرمول‌ها و رسیدهای تولید نمونه. */
const demoFormulas = Array.from({length: 5}, (_, index) => {
  const product = products[(index * 9 + 3) % products.length]
  const componentCount = (index % 2) + 2
  let estimated = 0
  for (let offset = 0; offset < componentCount; offset += 1) {
    const material = products[(index * 4 + offset) % products.length]
    estimated += material.purchase_price * (offset + 1) * 1.05
  }
  return {
    id: `demo-formula-${index}`,
    product_id: product.id,
    product_name: product.name,
    title: index % 2 === 0 ? 'فرمول استاندارد' : 'فرمول اقتصادی',
    output_quantity: 1,
    is_active: true,
    component_count: componentCount,
    estimated_unit_cost: Math.round(estimated),
    producible_now: (index + 1) * 7.5,
  }
})

function formulaComponents(header: (typeof demoFormulas)[number]) {
  const index = demoFormulas.indexOf(header)
  return Array.from({length: header.component_count}, (_, offset) => {
    const material = products[(index * 4 + offset) % products.length]
    const waste = offset === 0 ? 5 : 0
    const quantity = offset + 1
    return {
      id: `${header.id}-c${offset}`,
      product_id: material.id,
      product_name: material.name,
      unit: material.unit,
      quantity_per_unit: quantity,
      waste_percent: waste,
      unit_cost: material.purchase_price,
      effective_quantity: quantity * (1 + waste / 100),
      available_stock: (offset + 3) * 12,
    }
  })
}

const demoProductionOrders = Array.from({length: 6}, (_, index) => {
  const materials = ((index % 4) + 2) * 18_000_000
  const expenses = ((index % 3) + 1) * 4_500_000
  return {
    id: `demo-production-${index}`,
    number: index + 1,
    production_date: jalaliDate(index + 12),
    warehouse_name: warehouses[index % warehouses.length].name,
    materials_total: materials,
    expenses_total: expenses,
    total_cost: materials + expenses,
    status: 'posted',
    description: 'تولید دوره‌ای',
    journal_id: `demo-jrn-production-${index}`,
    input_count: (index % 2) + 2,
    output_count: 1,
  }
})

/** تنظیمات نمونه — بازتاب رجیستری واقعی میزبان. */
const demoSettings = [
  {key: 'inventory_valuation_method', group: 'inventory', group_label: 'انبار', label: 'روش ارزش‌گذاری موجودی', description: 'بهای تمام‌شده‌ی کالای خارج‌شده از انبار با کدام روش محاسبه شود.', effect: 'گزارش ارزش موجودی، بهای تمام‌شده‌ی فروش، سند تعدیل انبارگردانی', kind: 'choice', default_value: 'weighted_average', choices: [{value: 'weighted_average', label: 'میانگین موزون'}, {value: 'fifo', label: 'اولین صادره از اولین وارده (FIFO)'}, {value: 'lifo', label: 'اولین صادره از آخرین وارده (LIFO)'}], sensitive: true, value: 'weighted_average', is_customized: false},
  {key: 'inventory.low_stock_threshold', group: 'inventory', group_label: 'انبار', label: 'حد هشدار کمبود موجودی', description: 'اگر موجودی کالا از این عدد کمتر شود، در کارت «نزدیک به اتمام موجودی» دیده می‌شود.', effect: 'داشبورد انبار — کارت نزدیک به اتمام موجودی', kind: 'integer', default_value: '5', min: 0, max: 100000, sensitive: false, value: '5', is_customized: false},
  {key: 'inventory.recount_threshold_percent', group: 'inventory', group_label: 'انبار', label: 'درصد اختلاف الزام‌آور شمارش مجدد', description: 'اگر اختلاف شمارش از این درصد بیشتر باشد، شمارش دوم اجباری می‌شود.', effect: 'انبارگردانی — الزام شمارش مجدد', kind: 'integer', default_value: '5', min: 0, max: 100, sensitive: false, value: '5', is_customized: false},
  {key: 'inventory.allow_negative_stock', group: 'inventory', group_label: 'انبار', label: 'اجازه‌ی منفی شدن موجودی', description: 'اگر فعال باشد، فروش بیش از موجودی ممکن می‌شود.', effect: 'ثبت فاکتور فروش و رسید تولید', kind: 'boolean', default_value: 'false', sensitive: true, value: 'false', is_customized: false},
  {key: 'sales.default_vat_basis_points', group: 'sales', group_label: 'فروش و خرید', label: 'نرخ پیش‌فرض مالیات بر ارزش افزوده', description: 'بر حسب صدم‌درصد؛ ۹۰۰ یعنی ۹ درصد.', effect: 'فرم فاکتور، پیش‌فاکتور و سفارش خرید', kind: 'integer', default_value: '900', min: 0, max: 10000, sensitive: false, value: '900', is_customized: false},
  {key: 'quotes.default_validity_days', group: 'sales', group_label: 'فروش و خرید', label: 'اعتبار پیش‌فرض پیش‌فاکتور (روز)', description: 'تاریخ اعتبار پیش‌فاکتور خودکار پیشنهاد می‌شود.', effect: 'فرم پیش‌فاکتور — مقدار اولیه‌ی «اعتبار تا»', kind: 'integer', default_value: '30', min: 1, max: 365, sensitive: false, value: '30', is_customized: false},
  {key: 'treasury.default_negative_policy', group: 'treasury', group_label: 'خزانه و چک', label: 'سیاست پیش‌فرض منفی شدن موجودی', description: 'برای حساب خزانه‌ی تازه‌ساخته‌شده.', effect: 'فرم تعریف صندوق و بانک', kind: 'choice', default_value: 'warn', choices: [{value: 'error', label: 'خطا'}, {value: 'warn', label: 'هشدار'}, {value: 'ignore', label: 'بی‌تأثیر'}], sensitive: false, value: 'warn', is_customized: false},
  {key: 'checks.due_soon_days', group: 'treasury', group_label: 'خزانه و چک', label: 'بازه‌ی هشدار سررسید چک (روز)', description: 'چک‌هایی که تا این تعداد روز آینده سررسید می‌شوند هشدار می‌گیرند.', effect: 'داشبورد چک‌ها — شمارنده‌ی نزدیک سررسید', kind: 'integer', default_value: '7', min: 1, max: 180, sensitive: false, value: '7', is_customized: false},
  {key: 'parties.require_national_id', group: 'parties', group_label: 'اشخاص', label: 'الزام کد ملی / شناسه ملی', description: 'برای صدور صورتحساب رسمی لازم است.', effect: 'فرم ثبت شخص — اعتبارسنجی پیش از ذخیره', kind: 'boolean', default_value: 'false', sensitive: false, value: 'false', is_customized: false},
  {key: 'parties.enforce_credit_limit', group: 'parties', group_label: 'اشخاص', label: 'اعمال سقف اعتبار', description: 'فروش نسیه بیش از سقف اعتبار متوقف می‌شود.', effect: 'ثبت فاکتور فروش نسیه', kind: 'boolean', default_value: 'true', sensitive: false, value: 'true', is_customized: false},
  {key: 'production.default_cost_allocation', group: 'production', group_label: 'تولید', label: 'روش پیش‌فرض تخصیص بهای تمام‌شده', description: 'وقتی یک رسید تولید چند محصول دارد.', effect: 'فرم رسید تولید', kind: 'choice', default_value: 'by_quantity', choices: [{value: 'by_quantity', label: 'بر اساس مقدار'}, {value: 'by_market_value', label: 'بر اساس ارزش بازار'}], sensitive: false, value: 'by_quantity', is_customized: false},
  {key: 'accounting.require_description', group: 'accounting', group_label: 'حسابداری', label: 'الزام شرح در سطر سند', description: 'برای حسابرسی‌پذیری توصیه می‌شود.', effect: 'ثبت سند حسابداری', kind: 'boolean', default_value: 'false', sensitive: false, value: 'false', is_customized: false},
  {key: 'coding.level_widths', group: 'accounting', group_label: 'حسابداری', label: 'طرح کدینگ (عرض هر سطح)', description: 'تعداد رقم هر سطح، جدا شده با ویرگول.', effect: 'کدینگ حساب‌ها — پیشنهاد کد بعدی', kind: 'text', default_value: '1,2,2,2', sensitive: true, value: '1,2,2,2', is_customized: false},
  {key: 'appearance.language', group: 'appearance', group_label: 'ظاهر', label: 'زبان برنامه', description: 'زبان متن‌ها، جهت صفحه و شکل ارقام.', effect: 'کل رابط کاربری', kind: 'choice', default_value: 'fa', choices: [{value: 'fa', label: 'فارسی'}, {value: 'en', label: 'English'}, {value: 'ar', label: 'العربية'}], sensitive: false, value: 'fa', is_customized: false},
  {key: 'appearance.dark_mode', group: 'appearance', group_label: 'ظاهر', label: 'تم تاریک', description: 'حالت نمایش برنامه در شروع. پیش‌فرض تیره است.', effect: 'پوسته‌ی برنامه', kind: 'boolean', default_value: 'true', sensitive: false, value: 'true', is_customized: false},
  {key: 'appearance.sidebar_collapsed', group: 'appearance', group_label: 'ظاهر', label: 'منوی جمع‌شده در شروع', description: 'فضای بیشتری برای جدول‌ها می‌ماند.', effect: 'پوسته‌ی برنامه', kind: 'boolean', default_value: 'false', sensitive: false, value: 'false', is_customized: false},
  {key: 'appearance.rows_per_page', group: 'appearance', group_label: 'ظاهر', label: 'تعداد ردیف در هر صفحه', description: 'تعداد ردیف در جدول‌های بلند.', effect: 'جدول‌های فهرست', kind: 'integer', default_value: '50', min: 10, max: 500, sensitive: false, value: '50', is_customized: false},
  {key: 'company.display_name', group: 'company', group_label: 'هویت مجموعه', label: 'نام روی فاکتور و رسید', description: 'نامی که در سربرگ همه‌ی چاپ‌ها می‌آید.', effect: 'سربرگ فاکتور، رسید فروشگاهی، سند و برچسب', kind: 'text', default_value: 'شرکت نوین پرداز', sensitive: false, value: 'فروشگاه نمونه نوین پرداز', is_customized: true},
  {key: 'company.phone', group: 'company', group_label: 'هویت مجموعه', label: 'شماره تماس مجموعه', description: 'زیر نام مجموعه در سربرگ چاپ می‌آید.', effect: 'سربرگ فاکتور و رسید', kind: 'text', default_value: '021-00000000', sensitive: false, value: '021-88776655', is_customized: true},
  {key: 'company.address', group: 'company', group_label: 'هویت مجموعه', label: 'نشانی مجموعه', description: 'در پای فاکتور رسمی چاپ می‌شود.', effect: 'سربرگ فاکتور A4', kind: 'text', default_value: '—', sensitive: false, value: 'تهران، خیابان ولیعصر، پلاک ۱۲۰۰', is_customized: true},
  {key: 'company.economic_code', group: 'company', group_label: 'هویت مجموعه', label: 'کد اقتصادی', description: 'در صورتحساب رسمی الزامی است.', effect: 'سربرگ فاکتور رسمی', kind: 'text', default_value: '—', sensitive: false, value: '411111111111', is_customized: true},
  {key: 'company.logo', group: 'company', group_label: 'هویت مجموعه', label: 'لوگوی مجموعه', description: 'تصویر لوگو در سربرگ چاپ.', effect: 'سربرگ فاکتور، رسید و برچسب', kind: 'image', default_value: '', sensitive: false, value: '', is_customized: false},
  {key: 'user.avatar', group: 'company', group_label: 'هویت مجموعه', label: 'تصویر پروفایل کاربر', description: 'کنار نام کاربر در نوار بالا و منوی کناری دیده می‌شود.', effect: 'نوار بالا و پای منوی کناری', kind: 'image', default_value: '', sensitive: false, value: '', is_customized: false},
  {key: 'hardware.barcode_enabled', group: 'hardware', group_label: 'سخت‌افزار', label: 'بارکدخوان فعال باشد', description: 'اسکن بارکد کالا را خودکار به فاکتور اضافه می‌کند.', effect: 'فرم صدور فاکتور', kind: 'boolean', default_value: 'true', sensitive: false, value: 'true', is_customized: false},
  {key: 'hardware.barcode_min_length', group: 'hardware', group_label: 'سخت‌افزار', label: 'حداقل طول بارکد', description: 'کوتاه‌تر از این، تایپ دستی فرض می‌شود.', effect: 'تشخیص اسکن از تایپ', kind: 'integer', default_value: '6', min: 3, max: 40, sensitive: false, value: '6', is_customized: false},
  {key: 'hardware.barcode_max_gap_ms', group: 'hardware', group_label: 'سخت‌افزار', label: 'بیشترین فاصله‌ی دو کاراکتر (میلی‌ثانیه)', description: 'فاصله‌ی بیشتر یعنی انسان تایپ می‌کند.', effect: 'تشخیص اسکن از تایپ', kind: 'integer', default_value: '60', min: 15, max: 300, sensitive: false, value: '60', is_customized: false},
  {key: 'hardware.barcode_suffix', group: 'hardware', group_label: 'سخت‌افزار', label: 'کاراکتر پایان اسکن', description: 'اغلب دستگاه‌ها Enter می‌فرستند.', effect: 'تشخیص پایان اسکن', kind: 'choice', default_value: 'enter', choices: [{value: 'enter', label: 'Enter (پیش‌فرض اغلب دستگاه‌ها)'}, {value: 'tab', label: 'Tab'}, {value: 'none', label: 'بدون کاراکتر پایان — تشخیص با زمان'}], sensitive: false, value: 'enter', is_customized: false},
  {key: 'printing.receipt_paper', group: 'printing', group_label: 'چاپ', label: 'عرض کاغذ رسید فروشگاهی', description: 'پرینترهای حرارتی معمولاً ۸۰ یا ۵۸ میلی‌متری‌اند.', effect: 'اندازه‌ی صفحه در چاپ رسید', kind: 'choice', default_value: '80mm', choices: [{value: '80mm', label: '۸۰ میلی‌متر (رایج‌ترین)'}, {value: '58mm', label: '۵۸ میلی‌متر'}], sensitive: false, value: '80mm', is_customized: false},
  {key: 'printing.footer_note', group: 'printing', group_label: 'چاپ', label: 'پیام پایین رسید', description: 'جمله‌ی انتهای هر رسید فروشگاهی.', effect: 'پاورقی رسید', kind: 'text', default_value: 'از خرید شما سپاسگزاریم', sensitive: false, value: 'از خرید شما سپاسگزاریم', is_customized: false},
  {key: 'printing.copies', group: 'printing', group_label: 'چاپ', label: 'تعداد نسخه‌ی پیش‌فرض', description: 'چند نسخه از هر فاکتور یک‌جا چاپ شود.', effect: 'چاپ فاکتور و رسید', kind: 'integer', default_value: '1', min: 1, max: 5, sensitive: false, value: '1', is_customized: false},
]

const sum = (values: number[]) => values.reduce((total, value) => total + value, 0)

/** محاسبه‌ی تقریبی فاکتور فقط برای دیدن چیدمان — منبع حقیقت، موتور Rust است. */
function previewInvoiceStub(args: Record<string, unknown>) {
  const lines = (args.lines as Array<Record<string, number>>) ?? []
  const headerDiscount = Number(args.headerDiscount ?? 0)
  const freight = Number(args.freight ?? 0)
  const received = Number(args.received ?? 0)

  const gross = lines.map((line) => Math.round(line.quantity * line.unit_price))
  const lineDiscounts = lines.map((line, index) =>
    Math.round((gross[index] * (line.discount_bp ?? 0)) / 10000) + (line.discount_amount ?? 0),
  )
  const netSum = sum(gross.map((value, index) => value - lineDiscounts[index]))

  let subtotal = 0
  let discountTotal = 0
  let dutyTotal = 0
  let vatTotal = 0
  let netTotal = 0
  let costTotal = 0
  let commissionTotal = 0

  const rows = lines.map((line, index) => {
    const base = gross[index] - lineDiscounts[index]
    const headerShare = netSum > 0 ? Math.round((headerDiscount * base) / netSum) : 0
    const totalDiscount = lineDiscounts[index] + headerShare
    const net = gross[index] - totalDiscount
    const duty = Math.round((net * (line.duty_bp ?? 0)) / 10000)
    const vat = Math.round(((net + duty) * (line.vat_bp ?? 0)) / 10000)
    const commission = Math.round((net * (line.commission_bp ?? 0)) / 10000)
    const cost = Math.round((line.unit_cost ?? 0) * line.quantity)
    subtotal += gross[index]
    discountTotal += totalDiscount
    netTotal += net
    dutyTotal += duty
    vatTotal += vat
    costTotal += cost
    commissionTotal += commission
    return {
      gross: gross[index],
      tier_discount: 0,
      line_discount: lineDiscounts[index],
      header_discount_share: headerShare,
      coupon_share: 0,
      total_discount: totalDiscount,
      net,
      freight_share: 0,
      duty,
      vat,
      total: net + duty + vat,
      commission,
      cost,
      profit: net - cost - commission,
    }
  })

  const total = sum(rows.map((row) => row.total)) + freight
  const profit = netTotal - costTotal - commissionTotal
  return {
    lines: rows,
    subtotal,
    discount_total: discountTotal,
    net_total: netTotal,
    freight,
    duty_total: dutyTotal,
    vat_total: vatTotal,
    total,
    commission_total: commissionTotal,
    cost_total: costTotal,
    profit,
    profit_margin_bp: netTotal === 0 ? 0 : Math.round((profit * 10000) / netTotal),
    balance_before: 30774330,
    balance_after: 30774330 + total - received,
    invoice_remainder: total - received,
  }
}

const stocktakeLines = products.slice(0, 12).map((product, index) => {
  const counted = index % 5 === 4 ? null : product.quantity - (index % 3) + (index % 2)
  const variance = counted === null ? null : counted - product.quantity
  return {
    id: `stl-${index}`,
    product_id: product.id,
    product_name: product.name,
    sku: product.sku,
    frozen_quantity: product.quantity,
    counted_quantity: counted,
    recount_quantity: null,
    final_quantity: counted,
    variance,
    variance_value: variance === null ? 0 : Math.round(variance * product.purchase_price),
    variance_approved: index % 3 === 0,
    needs_recount: variance !== null && Math.abs(variance) > product.quantity * 0.05,
    unit_cost: product.purchase_price,
  }
})

const stocktakeSummary = () => {
  const counted = stocktakeLines.filter((line) => line.final_quantity !== null)
  const surplus = counted.filter((line) => line.variance_value > 0)
  const shortage = counted.filter((line) => line.variance_value < 0)
  const surplusValue = sum(surplus.map((line) => line.variance_value))
  const shortageValue = Math.abs(sum(shortage.map((line) => line.variance_value)))
  return {
    total_lines: stocktakeLines.length,
    counted_lines: counted.length,
    uncounted_lines: stocktakeLines.length - counted.length,
    surplus_lines: surplus.length,
    shortage_lines: shortage.length,
    unapproved_variances: counted.filter(
      (line) => line.variance !== 0 && line.variance !== null && !line.variance_approved,
    ).length,
    surplus_value: surplusValue,
    shortage_value: shortageValue,
    net_value: surplusValue - shortageValue,
  }
}

const priceLevels = (price: number) => [
  {level: 'retail', label: 'جزئی', price},
  {level: 'wholesale', label: 'کلی', price: Math.round(price * 0.94)},
  {level: 'partner', label: 'همکار', price: Math.round(price * 0.88)},
  {level: 'partner_tier2', label: 'همکار درجه ۲', price: null},
  {level: 'partner_tier3', label: 'همکار درجه ۳', price: null},
  {level: 'seasonal', label: 'فصلی', price: null},
  {level: 'exhibition', label: 'نمایشگاه', price: null},
]

const responses: Record<string, (args: Record<string, unknown>) => unknown> = {
  // --- پایه ---
  login: () => ({id: 'user-demo', username: 'admin', display_name: 'مدیر سیستم'}),
  current_user: () => ({id: 'user-demo', username: 'admin', display_name: 'مدیر سیستم'}),
  get_company: () => ({id: 'company-demo', name: 'نوین پرداز', national_id: '14000000000'}),
  get_demo_status: () => true,
  delete_demo_data: () => null,
  logout: () => null,

  // --- کالا و انبار ---
  list_products: () => products,
  list_warehouses: () => warehouses,
  list_stock_balances: () =>
    products.map((product, index) => ({
      product_id: product.id,
      warehouse_id: warehouses[index % 3].id,
      quantity: product.quantity,
      reserved_quantity: 0,
      available_quantity: product.quantity,
    })),
  list_inventory_advanced: () =>
    products.map((product, index) => ({
      product_id: product.id,
      product_name: product.name,
      warehouse_id: warehouses[index % 3].id,
      warehouse_name: warehouses[index % 3].name,
      quantity: product.quantity,
      unit_cost: product.purchase_price,
      total_value: product.quantity * product.purchase_price,
    })),
  get_inventory_valuation_method: () => 'fifo',
  list_inventory_lots: () => [],
  list_inventory_counts: () => [],
  list_product_groups: () => [
    {id: 'pgroup-food', code: '1', title: 'مواد غذایی', product_count: 15},
    {id: 'pgroup-misc', code: '2', title: 'کالاهای متفرقه', product_count: 15},
    {id: 'pgroup-cosmetic', code: '3', title: 'آرایشی بهداشتی', product_count: 10},
    {id: 'pgroup-fashion', code: '4', title: 'مد و پوشاک', product_count: 10},
    {id: 'pgroup-raw', code: '9', title: 'مواد اولیه', product_count: 10},
  ],
  list_product_prices: () =>
    products.map((product) => ({
      id: product.id,
      sku: product.sku,
      name: product.name,
      kind: 'simple',
      kind_label: 'کالای عمومی (ساده)',
      group_title: product.group,
      prices: priceLevels(product.sale_price),
    })),
  set_product_price: () => null,
  get_low_stock: () =>
    products
      .filter((product) => product.quantity <= product.min_stock + 2)
      .map((product) => ({
        product_id: product.id,
        product_name: product.name,
        sku: product.sku,
        quantity: product.quantity,
        reorder_point: product.min_stock,
      })),
  list_valuation_methods: () => [
    {
      method: 'fifo',
      label: 'اولین صادره از اولین وارده (FIFO)',
      is_active: true,
      explanation:
        'کالایی که زودتر خریده‌اید، زودتر هم فروخته می‌شود. پس بهای فروش از قدیمی‌ترین خرید برداشته می‌شود و آنچه در انبار می‌ماند با قیمت خریدهای جدیدتر ارزش‌گذاری می‌گردد. مناسب کالای تاریخ‌دار و بازاری که قیمت‌ها بالا می‌رود.',
    },
    {
      method: 'moving_average',
      label: 'میانگین متحرک',
      is_active: false,
      explanation:
        'با هر خرید جدید، میانگین قیمت کالا دوباره حساب می‌شود. همه‌ی فروش‌های بعدی با همان میانگین ثبت می‌شوند. سود و زیان یکنواخت‌تر می‌شود و نوسان قیمت خرید کمتر به چشم می‌آید.',
    },
    {
      method: 'weighted_average',
      label: 'میانگین موزون',
      is_active: false,
      explanation:
        'میانگین قیمت کل خریدهای یک دوره محاسبه و برای همه‌ی فروش‌های آن دوره استفاده می‌شود. ساده‌ترین روش برای گزارش‌گیری دوره‌ای است.',
    },
  ],

  // --- اشخاص ---
  list_contacts: () => contacts,
  list_parties: () => ({
    rows: contacts.map((contact) => ({
      id: contact.id,
      code: contact.id,
      display_name: contact.name,
      party_type: contact.legal ? 'private_legal' : 'natural',
      party_type_label: contact.legal ? 'حقوقی غیردولتی' : 'حقیقی',
      party_function: 'person',
      party_function_label: 'شخص',
      group_title: contact.is_supplier
        ? contact.is_customer
          ? 'مشتری و تأمین‌کننده'
          : 'بستانکاران تجاری'
        : 'بدهکاران تجاری',
      is_customer: contact.is_customer,
      is_supplier: contact.is_supplier,
      mobile: contact.mobile,
      route_title: 'مسیر مرکز',
      marketer_name: 'سعید بازاریان',
      credit_limit: contact.credit_limit,
      balance: contact.balance,
      balance_status:
        contact.balance > 0 ? 'debtor' : contact.balance < 0 ? 'creditor' : 'settled',
      balance_indicator: contact.balance > 0 ? 'بد' : contact.balance < 0 ? 'بس' : 'بی حساب',
    })),
    summary: {
      debtor_count: contacts.filter((c) => c.balance > 0).length,
      debtor_total: sum(contacts.filter((c) => c.balance > 0).map((c) => c.balance)),
      creditor_count: contacts.filter((c) => c.balance < 0).length,
      creditor_total: Math.abs(sum(contacts.filter((c) => c.balance < 0).map((c) => c.balance))),
      settled_count: contacts.filter((c) => c.balance === 0).length,
      total_count: contacts.length,
      net_total: sum(contacts.map((c) => c.balance)),
    },
  }),
  list_party_routes: () => [
    {id: 'route-center', code: 'R02', title: 'مسیر مرکز'},
    {id: 'route-north', code: 'R01', title: 'مسیر شمال شهر'},
  ],
  validate_party_identity: () => [],
  update_party_profile: () => null,

  // --- فاکتور ---
  list_sales_invoices: () => salesInvoices,
  list_purchase_invoices: () => purchaseInvoices,
  preview_invoice: previewInvoiceStub,
  create_sales_invoice: () => 'preview-invoice-001',
  post_sales_invoice: () => null,
  build_installment_plan: (args) => {
    const total = Number(args.total ?? 0) - Number(args.downPayment ?? 0)
    const count = Math.max(1, Number(args.count ?? 1))
    const base = Math.floor(total / count)
    const remainder = total - base * count
    return Array.from({length: count}, (_, index) => ({
      number: index + 1,
      due_date: '',
      due_date_jalali: jalaliDate(index + 5),
      amount: base + (index < remainder ? 1 : 0),
    }))
  },

  // --- حسابداری ---
  list_accounts: () => [
    {id: 'acc-1000', code: '1000', name: 'دارایی‌ها', level: 'group', nature: 'debit'},
    {id: 'acc-1101', code: '1101', name: 'صندوق', level: 'detail', nature: 'debit'},
    {id: 'acc-1201', code: '1201', name: 'حساب مشتریان', level: 'detail', nature: 'debit'},
    {id: 'acc-1300', code: '1300', name: 'موجودی کالا', level: 'general', nature: 'debit'},
    {id: 'acc-2101', code: '2101', name: 'تأمین‌کنندگان', level: 'detail', nature: 'credit'},
    {id: 'acc-4100', code: '4100', name: 'فروش کالا', level: 'general', nature: 'credit'},
  ],
  list_journals: () =>
    salesInvoices.slice(0, 30).map((invoice, index) => ({
      id: `demo-jrn-sale-${index}`,
      number: 2000 + index,
      entry_date: invoice.invoice_date,
      description: `فاکتور فروش شماره ${invoice.number}`,
      status: 'posted',
      total_debit: invoice.total,
      total_credit: invoice.total,
    })),
  list_postable_accounts: () => [
    {
      id: 'acc-1201',
      code: '1201',
      name: 'حساب مشتریان',
      nature: 'debit',
      requires_subsidiary: true,
      requires_cost_center: false,
      requires_project: false,
    },
    {
      id: 'acc-4100',
      code: '4100',
      name: 'فروش کالا',
      nature: 'credit',
      requires_subsidiary: false,
      requires_cost_center: false,
      requires_project: false,
    },
  ],
  list_cost_centers: () => [
    {id: 'cc-sales', code: '4001', title: 'واحد فروش'},
    {id: 'cc-admin', code: '4002', title: 'واحد اداری'},
  ],
  list_projects: () => [{id: 'project-demo', code: '5001', title: 'پروژه نمونه'}],
  list_subsidiary_groups: () => [
    {id: 'subgroup-persons', code: '10', title: 'اشخاص'},
    {id: 'subgroup-banks', code: '2030', title: 'بانک ها'},
  ],
  create_single_line_journal: () => 'journal-preview-1',

  // --- خزانه و چک ---
  // ---- فرم کامل شخص ----
  list_party_groups: () => [
    {id: 'pgroup-trade-debtor', code: 'G01', title: 'بدهکاران تجاری', parent_id: undefined, member_count: 18},
    {id: 'pgroup-trade-creditor', code: 'G02', title: 'بستانکاران تجاری', parent_id: undefined, member_count: 9},
    {id: 'pgroup-site', code: 'G03', title: 'سایت', parent_id: undefined, member_count: 6},
    {id: 'pgroup-colleagues', code: 'G04', title: 'همکاران', parent_id: undefined, member_count: 11},
    {id: 'pgroup-staff', code: 'G05', title: 'کارکنان', parent_id: undefined, member_count: 4},
    {id: 'pgroup-vip', code: 'G06', title: 'مشتریان ویژه', parent_id: 'pgroup-trade-debtor', member_count: 3},
  ],
  list_party_options: () => ({
    party_types: [
      {value: 'natural', label: 'حقیقی'},
      {value: 'private_legal', label: 'حقوقی غیردولتی'},
      {value: 'government_legal', label: 'حقوقی دولتی'},
      {value: 'civil_partnership', label: 'مشارکت مدنی'},
    ],
    party_functions: [
      {value: 'person', label: 'شخص'},
      {value: 'marketer', label: 'بازاریاب'},
      {value: 'supervisor', label: 'سوپروایزر'},
    ],
  }),
  get_party: (args: Record<string, unknown>) => {
    const index = contacts.findIndex((c) => c.id === args.id)
    const contact = contacts[index < 0 ? 0 : index]
    const legal = index % 4 === 0
    return {
      id: contact.id,
      code: String(1001 + (index < 0 ? 0 : index)),
      party_type: legal ? 'private_legal' : 'natural',
      party_type_label: legal ? 'حقوقی غیردولتی' : 'حقیقی',
      party_function: 'person',
      party_function_label: 'شخص',
      title_prefix: legal ? undefined : 'آقای',
      first_name: legal ? undefined : contact.name.split(' ')[0],
      last_name: legal ? undefined : contact.name.split(' ').slice(1).join(' '),
      company_name: legal ? contact.name : undefined,
      display_name: contact.name,
      national_id: legal ? '10293847568' : '0499370899',
      economic_code: undefined,
      group_id: 'pgroup-trade-debtor',
      route_id: 'route-center',
      marketer_id: undefined,
      opening_date: '1405/01/01',
      is_customer: true,
      is_supplier: false,
      is_active: true,
      mobile: '09121234567',
      email: 'info@example.com',
      website: undefined,
      province: 'تهران',
      city: 'تهران',
      address: 'خیابان نمونه، پلاک ۱۲',
      postal_code: undefined,
      job_title: 'بازرگان',
      introduction: 'معرفی همکار',
      credit_limit: 500_000_000,
      note: undefined,
      portal_username: undefined,
      has_portal_password: false,
      phones: [{id: 'p1', title: 'دفتر', number: '02122334455', is_primary: true}],
      bank_accounts: [
        {
          id: 'b1',
          bank_name: 'بانک ملت',
          branch_name: 'شعبه مرکزی',
          account_number: '1234567',
          iban: 'IR280620000000001234567891',
          card_number: '6037991234567893',
          holder_name: contact.name,
          is_default: true,
        },
      ],
      images: [],
      occasions: [{id: 'o1', title: 'تولد', jalali_month: 3, jalali_day: 14, remind_days_before: 3}],
    }
  },
  save_party: () => 'contact-preview',
  find_duplicate_party: () => null,
  deactivate_party: () => undefined,
  save_party_group: () => 'pgroup-preview',
  list_upcoming_occasions: () => [
    {contact_id: contacts[0].id, contact_name: contacts[0].name, title: 'تولد', jalali_month: 3, jalali_day: 14, remind_days_before: 3},
  ],
  // ---- کدینگ حساب‌ها ----
  get_coding_scheme: () => ({
    level_widths: [1, 2, 2, 2],
    level_titles: ['گروه', 'کل', 'معین', 'تفصیلی'],
    code_lengths: [1, 3, 5, 7],
    capacities: [9, 99, 99, 99],
  }),
  set_coding_scheme: () => ({
    level_widths: [1, 2, 2, 2],
    level_titles: ['گروه', 'کل', 'معین', 'تفصیلی'],
    code_lengths: [1, 3, 5, 7],
    capacities: [9, 99, 99, 99],
  }),
  list_account_tree: () => demoAccounts,
  suggest_account_code: () => '1400',
  save_account: () => 'acc-preview',
  deactivate_account: () => undefined,
  audit_coding_health: () => [
    {
      account_id: 'acc-1000',
      code: '1000',
      name: 'دارایی ها',
      severity: 'info',
      message:
        'طول این کد با طرح کدینگ فعلی نمی‌خواند؛ کدینگ مسطح است و کار می‌کند، ولی پیشنهاد کد خودکار برایش دقیق نیست.',
    },
  ],
  // ---- برگشت از فروش و خرید ----
  list_returnable_lines: (args: Record<string, unknown>) => {
    const invoice = salesInvoices.find((row) => row.id === args.invoiceId) ?? salesInvoices[0]
    const seed = invoice.number
    return products.slice(seed % 5, (seed % 5) + 3).map((product, index) => {
      const invoiced = ((seed + index) % 4) + 2
      const returned = index === 0 ? 1 : 0
      return {
        product_id: product.id,
        product_name: product.name,
        unit: product.unit,
        invoiced_quantity: invoiced,
        returned_quantity: returned,
        returnable_quantity: invoiced - returned,
        unit_price: product.sale_price,
      }
    })
  },
  list_returns: (args: Record<string, unknown>) =>
    demoReturns.filter((r) => !args.status || r.status === args.status),
  get_return: (args: Record<string, unknown>) => {
    const header = demoReturns.find((r) => r.id === args.id) ?? demoReturns[0]
    const lines = products.slice(0, 2).map((product, index) => ({
      id: `${header.id}-l${index}`,
      product_id: product.id,
      product_name: product.name,
      quantity: index + 1,
      unit_price: product.sale_price,
      line_total: (index + 1) * product.sale_price,
    }))
    return {header, lines}
  },
  post_sales_return_v2: () => undefined,
  post_purchase_return_v2: () => undefined,
  cancel_return: () => undefined,
  // ---- انتقال بین انبارها ----
  list_inventory_transfer_orders: () => demoTransfers,
  create_inventory_transfer_order: () => 'transfer-preview',
  receive_inventory_transfer: () => undefined,
  // ---- پیش‌فاکتور و سفارش خرید ----
  list_quotes: (args: Record<string, unknown>) =>
    demoQuotes.filter(
      (q) => q.kind === args.kind && (!args.status || q.status === args.status),
    ),
  get_quote: (args: Record<string, unknown>) => {
    const header = demoQuotes.find((q) => q.id === args.id) ?? demoQuotes[0]
    const lines = products.slice(header.number % 8, (header.number % 8) + 3).map((product, index) => {
      const quantity = index + 2
      const gross = quantity * product.sale_price
      const discount = index === 1 ? Math.round(gross * 0.05) : 0
      const tax = Math.round(((gross - discount) * 9) / 100)
      return {
        id: `${header.id}-l${index}`,
        product_id: product.id,
        product_name: product.name,
        unit: product.unit,
        quantity,
        unit_price: product.sale_price,
        discount,
        tax,
        line_total: gross - discount + tax,
        description: undefined,
      }
    })
    return {header, lines}
  },
  quote_transitions: (args: Record<string, unknown>) => {
    const row = demoQuotes.find((q) => q.id === args.id)
    const map: Record<string, Array<{status: string; label: string}>> = {
      draft: [
        {status: 'sent', label: 'ارسال‌شده'},
        {status: 'cancelled', label: 'باطل‌شده'},
      ],
      sent: [
        {status: 'accepted', label: 'پذیرفته‌شده'},
        {status: 'rejected', label: 'ردشده'},
        {status: 'expired', label: 'منقضی'},
        {status: 'cancelled', label: 'باطل‌شده'},
      ],
      accepted: [{status: 'cancelled', label: 'باطل‌شده'}],
      rejected: [{status: 'draft', label: 'پیش‌نویس'}],
      expired: [{status: 'draft', label: 'پیش‌نویس'}],
    }
    return map[row?.status ?? ''] ?? []
  },
  preview_quote: (args: Record<string, unknown>) => {
    const lines = (args.lines ?? []) as Array<Record<string, number>>
    const vat = Number(args.vatBasisPoints ?? 0)
    const subtotal = lines.reduce((total, line) => total + line.quantity * line.unit_price, 0)
    const discount = lines.reduce((total, line) => total + (line.discount ?? 0), 0)
    const net = subtotal - discount
    const tax = Math.floor((net * vat) / 10_000)
    return {subtotal, discount, net, tax, total: net + tax}
  },
  save_quote: () => 'quote-preview',
  set_quote_status: () => undefined,
  convert_quote: () => 'invoice-preview',
  // ---- تولید ----
  list_cost_allocations: () => [
    {value: 'by_quantity', label: 'بر اساس مقدار', explanation: 'کل بهای تمام‌شده به نسبت مقدار هر محصول تقسیم می‌شود. مناسب وقتی محصولات از یک جنس و هم‌ارزش‌اند. اگر ارزش محصولات خیلی متفاوت باشد، این روش بهای محصول ارزان را بالا می‌برد.'},
    {value: 'by_market_value', label: 'بر اساس ارزش بازار', explanation: 'کل بهای تمام‌شده به نسبت ارزش فروش هر محصول تقسیم می‌شود. مناسب وقتی یک محصول اصلی و یک محصول فرعی دارید. حاشیه‌ی سود هر دو محصول برابر درمی‌آید.'},
  ],
  list_production_expense_accounts: () => [
    {id: 'acc-5300', code: '5300', name: 'دستمزد مستقیم تولید'},
    {id: 'acc-5400', code: '5400', name: 'سربار تولید'},
    {id: 'acc-6300', code: '6300', name: 'کسری و ضایعات انبار'},
  ],
  list_production_formulas: () => demoFormulas,
  get_production_formula: (args: Record<string, unknown>) => {
    const header = demoFormulas.find((f) => f.id === args.id) ?? demoFormulas[0]
    return {header, components: formulaComponents(header)}
  },
  expand_production_formula: (args: Record<string, unknown>) => {
    const header = demoFormulas.find((f) => f.id === args.formulaId) ?? demoFormulas[0]
    const quantity = Number(args.outputQuantity ?? 1)
    return formulaComponents(header).map((component) => ({
      product_id: component.product_id,
      product_name: component.product_name,
      unit: component.unit,
      required_quantity: Number((component.effective_quantity * quantity).toFixed(4)),
      available_stock: component.available_stock,
      unit_cost: component.unit_cost,
    }))
  },
  save_production_formula: () => 'formula-preview',
  delete_production_formula: () => undefined,
  preview_production: (args: Record<string, unknown>) => {
    const input = (args.input ?? {}) as Record<string, unknown>
    const inputLines = (input.inputs ?? []) as Array<Record<string, number | string>>
    const outputLines = (input.outputs ?? []) as Array<Record<string, number | string>>
    const expenseLines = (input.expenses ?? []) as Array<Record<string, number | string>>
    const costOf = (id: string) => products.find((p) => p.id === id)?.purchase_price ?? 0
    const materials = inputLines.reduce(
      (total, line) => total + Number(line.quantity) * costOf(String(line.product_id)),
      0,
    )
    const expenseTotal = expenseLines.reduce((total, line) => total + Number(line.amount ?? 0), 0)
    const totalCost = Math.round(materials + expenseTotal)
    const weights = outputLines.map((line) =>
      input.cost_allocation === 'by_market_value'
        ? Number(line.quantity) * Number(line.market_unit_price ?? 0)
        : Number(line.quantity),
    )
    const weightSum = weights.reduce((a, b) => a + b, 0) || outputLines.length || 1
    let assigned = 0
    const outputs = outputLines.map((line, index) => {
      const share =
        index === outputLines.length - 1
          ? totalCost - assigned
          : Math.round((totalCost * (weights[index] || 1)) / weightSum)
      assigned += share
      const productId = String(line.product_id)
      return {
        product_id: productId,
        product_name: products.find((p) => p.id === productId)?.name ?? productId,
        quantity: Number(line.quantity),
        allocated_cost: share,
        unit_cost: Math.round(share / (Number(line.quantity) || 1)),
        previous_unit_cost: costOf(productId),
      }
    })
    return {
      materials_total: Math.round(materials),
      expenses_total: expenseTotal,
      total_cost: totalCost,
      outputs,
      warnings: [],
    }
  },
  post_production: () => 'production-preview',
  list_production_orders: () => demoProductionOrders,
  // ---- مرکز تنظیمات ----
  list_settings: () => demoSettings,
  set_setting: (args: Record<string, unknown>) => String(args.value ?? ''),
  reset_setting: (args: Record<string, unknown>) =>
    demoSettings.find((item) => item.key === args.key)?.default_value ?? '',
  list_treasury_accounts: () => treasuryAccounts,
  list_treasury_account_details: (args: Record<string, unknown>) =>
    treasuryAccounts
      .filter((a) => !args.accountType || a.account_type === args.accountType)
      .map((a, index) => {
        const inflow = ((index % 5) + 2) * 45_000_000
        const outflow = ((index % 4) + 1) * 30_000_000
        return {
          ...a,
          account_type_label:
            a.account_type === 'bank' ? 'حساب بانکی' : a.account_type === 'cash' ? 'صندوق' : 'تنخواه',
          account_number: a.account_type === 'bank' ? `${1234567 + index}` : undefined,
          iban: a.account_type === 'bank' ? `IR${28062 + index}0000000012345678${index}1` : undefined,
          card_number: a.account_type === 'bank' ? '6037991234567893' : undefined,
          branch_name: a.account_type === 'bank' ? 'شعبه مرکزی' : undefined,
          branch_code: a.account_type === 'bank' ? String(1200 + index) : undefined,
          holder_name: 'شرکت نوین پرداز',
          has_pos_terminal: a.account_type === 'bank',
          negative_policy: a.account_type === 'bank' ? 'warn' : 'error',
          negative_policy_label: a.account_type === 'bank' ? 'هشدار' : 'خطا',
          linked_account_id: 'acc-1101',
          linked_account_name: 'موجودی نقد و بانک',
          balance: inflow - outflow,
          inflow,
          outflow,
          transaction_count: (index + 1) * 7,
        }
      }),
  list_negative_policies: () => [
    {value: 'error', label: 'خطا', explanation: 'اگر برداشت باعث منفی شدن موجودی شود، عملیات انجام نمی‌شود. مناسب صندوق نقدی، چون پولی که در صندوق نیست قابل پرداخت نیست.'},
    {value: 'warn', label: 'هشدار', explanation: 'عملیات انجام می‌شود ولی هشدار داده می‌شود. مناسب حساب بانکی که ممکن است اضافه‌برداشت داشته باشد.'},
    {value: 'ignore', label: 'بی‌تأثیر', explanation: 'هیچ بررسی‌ای انجام نمی‌شود. فقط وقتی استفاده کنید که مانده‌ی این حساب را جای دیگری کنترل می‌کنید.'},
  ],
  save_treasury_account: () => 'treasury-preview',
  deactivate_treasury_account: () => undefined,
  list_treasury_transactions: () =>
    Array.from({length: 20}, (_, index) => ({
      id: `demo-tx-${index}`,
      transaction_type: index % 3 === 0 ? 'payment' : 'receipt',
      amount: ((index % 8) + 1) * 12_500_000,
      transaction_date: jalaliDate(index + 1),
      description: index % 3 === 0 ? 'پرداخت به تأمین‌کننده' : 'دریافت از مشتری',
    })),
  get_treasury_summary: () => ({
    cash_total: 241_800_000,
    bank_total: 1_904_500_000,
    receipts: 320_000_000,
    payments: 180_000_000,
  }),
  // ---- سند دریافت و پرداخت ----
  list_payment_methods: () => [
    {value: 'cash', label: 'نقد', requires_treasury_account: true, requires_terminal: false, requires_check_details: false, moves_treasury: true},
    {value: 'check', label: 'چک', requires_treasury_account: false, requires_terminal: false, requires_check_details: true, moves_treasury: false},
    {value: 'bank_transfer', label: 'حواله بانکی', requires_treasury_account: true, requires_terminal: false, requires_check_details: false, moves_treasury: true},
    {value: 'card_terminal', label: 'کارتخوان', requires_treasury_account: true, requires_terminal: true, requires_check_details: false, moves_treasury: true},
    {value: 'discount', label: 'تخفیف نقدی', requires_treasury_account: false, requires_terminal: false, requires_check_details: false, moves_treasury: false},
    {value: 'offset', label: 'تهاتر', requires_treasury_account: false, requires_terminal: false, requires_check_details: false, moves_treasury: false},
  ],
  // بازتاب ساده‌ی build_journal هسته، فقط برای دیدن چیدمان در مرورگر.
  preview_treasury_document: (args: Record<string, unknown>) => {
    const kind = args.kind as string
    const lines = (args.lines ?? []) as Array<Record<string, unknown>>
    const bucket = (name: string) =>
      lines.filter((l) => l.method === name).reduce((sum, l) => sum + Number(l.amount ?? 0), 0)
    const total = lines.reduce((sum, l) => sum + Number(l.amount ?? 0), 0)
    const movesTreasury = ['cash', 'bank_transfer', 'card_terminal']
    const accountName = (line: Record<string, unknown>) => {
      if (line.method === 'check') return kind === 'receipt' ? 'اسناد دریافتنی' : 'اسناد پرداختنی'
      if (line.method === 'discount') return 'تخفیفات نقدی اعطایی'
      if (line.method === 'offset') return kind === 'receipt' ? 'حساب مشتریان' : 'تأمین کنندگان'
      const account = treasuryAccounts.find((a) => a.id === line.treasury_account_id)
      return account?.name ?? 'صندوق'
    }
    const journal = lines.map((line) => ({
      account_id: String(line.treasury_account_id ?? line.method),
      account_name: accountName(line),
      debit: kind === 'receipt' ? Number(line.amount ?? 0) : 0,
      credit: kind === 'receipt' ? 0 : Number(line.amount ?? 0),
    }))
    journal.push({
      account_id: kind === 'receipt' ? 'acc-1201' : 'acc-2101',
      account_name: kind === 'receipt' ? 'حساب مشتریان' : 'تأمین کنندگان',
      debit: kind === 'receipt' ? 0 : total,
      credit: kind === 'receipt' ? total : 0,
    })
    return {
      cash: bucket('cash'),
      check: bucket('check'),
      bank_transfer: bucket('bank_transfer'),
      card_terminal: bucket('card_terminal'),
      discount: bucket('discount'),
      offset: bucket('offset'),
      total,
      treasury_movement: lines
        .filter((l) => movesTreasury.includes(String(l.method)))
        .reduce((sum, l) => sum + Number(l.amount ?? 0), 0),
      journal_preview: journal,
    }
  },
  list_treasury_documents: (args: Record<string, unknown>) =>
    treasuryDocuments.filter((d) => !args.kind || d.kind === args.kind),
  get_treasury_document: (args: Record<string, unknown>) => {
    const header = treasuryDocuments.find((d) => d.id === args.id) ?? treasuryDocuments[0]
    return {header, lines: treasuryDocumentLines(header)}
  },
  create_treasury_document: () => 'tdoc-preview',
  list_checks: () => checks,
  list_checks_filtered: () => checks,
  // بازتاب ماشین حالت هسته، فقط برای پیش‌نمایش مرورگر؛ منبع حقیقت Rust است.
  check_transition_options: (args: Record<string, unknown>) => {
    const row = checks.find((c) => c.id === args.checkId)
    if (!row) return []
    const map: Record<string, Array<{status: string; label: string; treasury_effect: string}>> = {
      in_hand: [
        {status: 'deposited', label: 'واگذار شده', treasury_effect: 'none'},
        {status: 'endorsed', label: 'خرج شده', treasury_effect: 'none'},
        {status: 'cashed', label: 'نقد شده', treasury_effect: 'increase'},
        {status: 'returned', label: 'عودت شده', treasury_effect: 'none'},
        {status: 'void', label: 'باطل شده', treasury_effect: 'none'},
      ],
      deposited: [
        {status: 'collected', label: 'وصول شده', treasury_effect: 'increase'},
        {status: 'bounced', label: 'برگشتی', treasury_effect: 'none'},
      ],
      endorsed: [{status: 'bounced', label: 'برگشتی', treasury_effect: 'none'}],
      collected: [{status: 'bounced', label: 'برگشتی', treasury_effect: 'decrease'}],
      cashed: [{status: 'bounced', label: 'برگشتی', treasury_effect: 'decrease'}],
      bounced:
        row.check_type === 'issued'
          ? [
              {status: 'outstanding', label: 'پرداختی در جریان', treasury_effect: 'none'},
              {status: 'paid', label: 'پرداخت شده', treasury_effect: 'decrease'},
              {status: 'returned', label: 'عودت شده', treasury_effect: 'none'},
            ]
          : [
              {status: 'in_hand', label: 'موجود', treasury_effect: 'none'},
              {status: 'deposited', label: 'واگذار شده', treasury_effect: 'none'},
              {status: 'returned', label: 'عودت شده', treasury_effect: 'none'},
            ],
      outstanding: [
        {status: 'paid', label: 'پرداخت شده', treasury_effect: 'decrease'},
        {status: 'bounced', label: 'برگشتی', treasury_effect: 'none'},
        {status: 'returned', label: 'عودت شده', treasury_effect: 'none'},
        {status: 'void', label: 'باطل شده', treasury_effect: 'none'},
      ],
    }
    return map[row.status] ?? []
  },
  get_check_dashboard: () => ({
    total_received: sum(checks.filter((c) => c.check_type === 'received').map((c) => c.amount)),
    received_count: checks.filter((c) => c.check_type === 'received').length,
    total_issued: sum(checks.filter((c) => c.check_type === 'issued').map((c) => c.amount)),
    issued_count: checks.filter((c) => c.check_type === 'issued').length,
    due_soon_count: 3,
    bounced_count: checks.filter((c) => c.status === 'bounced').length,
  }),
  update_check_status: () => null,

  // --- انبارگردانی ---
  list_stocktakes: () => [
    {
      id: 'st-1',
      title: 'انبارگردانی پایان سال ۱۴۰۵',
      warehouse_name: 'انبار مرکزی',
      count_date: '1405/05/29',
      status: 'counting',
      status_label: 'در حال شمارش',
      total_lines: stocktakeLines.length,
      counted_lines: stocktakeLines.filter((line) => line.final_quantity !== null).length,
      variance_lines: stocktakeLines.filter((line) => (line.variance ?? 0) !== 0).length,
    },
  ],
  get_stocktake: () => ({
    id: 'st-1',
    title: 'انبارگردانی پایان سال ۱۴۰۵',
    status: 'counting',
    status_label: 'در حال شمارش',
    warehouse_name: 'انبار مرکزی',
    count_date: '1405/05/29',
    recount_threshold_percent: 5,
    lines: stocktakeLines,
    ...stocktakeSummary(),
    can_post: false,
    blocking_reason: 'STK-003: شمارش همه‌ی اقلام کامل نشده است',
  }),
  create_stocktake: () => 'st-1',
  set_stocktake_count: () => null,
  approve_all_variances: () => 4,
  post_stocktake: () => 'journal-demo-1',
  preview_bulk_price_change: (args) => {
    const ids = (args.productIds as string[]) ?? []
    return ids.map((id) => {
      const product = products.find((item) => item.id === id)
      const oldPrice = product?.sale_price ?? 0
      const newPrice = Math.round(oldPrice * 1.1)
      return {
        product_id: id,
        product_name: product?.name ?? '',
        old_price: oldPrice,
        new_price: newPrice,
        difference: newPrice - oldPrice,
      }
    })
  },
  apply_bulk_price_change: (args) => ((args.productIds as string[]) ?? []).length,

  // --- داشبورد و گزارش ---
  get_dashboard_kpis: () => ({
    sales: sum(salesInvoices.map((invoice) => invoice.total)),
    purchases: sum(purchaseInvoices.map((invoice) => invoice.total)),
    gross_profit: 412_500_000,
    receivables: sum(contacts.filter((c) => c.balance > 0).map((c) => c.balance)),
    payables: Math.abs(sum(contacts.filter((c) => c.balance < 0).map((c) => c.balance))),
    cash: 241_800_000,
    inventory_value: sum(products.map((p) => p.quantity * p.purchase_price)),
    low_stock_count: products.filter((p) => p.quantity <= p.min_stock + 2).length,
  }),
  get_sales_trend: () =>
    Array.from({length: 6}, (_, index) => ({
      period: `1405/${String(index + 1).padStart(2, '0')}`,
      amount: sum(
        salesInvoices
          .filter((invoice) => invoice.invoice_date.startsWith(`1405/${String(index + 1).padStart(2, '0')}`))
          .map((invoice) => invoice.total),
      ),
    })),
  get_top_products: () =>
    products.slice(0, 8).map((product, index) => ({
      product_id: product.id,
      name: product.name,
      quantity: 40 - index * 3,
      amount: product.sale_price * (40 - index * 3),
    })),
  get_low_stock_report: () =>
    products
      .filter((product) => product.quantity <= product.min_stock + 2)
      .map((product) => ({
        product_id: product.id,
        name: product.name,
        quantity: product.quantity,
        min_stock: product.min_stock,
      })),
  get_recent_invoices: () =>
    [
      ...salesInvoices.slice(-6).map((invoice) => ({...invoice, invoice_type: 'sales'})),
      ...purchaseInvoices.slice(-4).map((invoice) => ({...invoice, invoice_type: 'purchase'})),
    ]
      .sort((a, b) => b.invoice_date.localeCompare(a.invoice_date))
      .map((invoice) => ({
        id: invoice.id,
        number: invoice.number,
        invoice_date: invoice.invoice_date,
        contact_name: invoice.contact_name,
        total: invoice.total,
        payment_status: invoice.payment_status,
        invoice_type: invoice.invoice_type,
      })),
  get_trial_balance: () => [
    {code: '1101', name: 'صندوق', debit: 241_800_000, credit: 0},
    {code: '1201', name: 'حساب مشتریان', debit: 1_482_000_000, credit: 0},
    {code: '1300', name: 'موجودی کالا', debit: 980_000_000, credit: 0},
    {code: '2101', name: 'تأمین‌کنندگان', debit: 0, credit: 420_000_000},
    {code: '4100', name: 'فروش کالا', debit: 0, credit: 2_283_800_000},
  ],
}

/* پاسخ‌های مشتق (گزارش‌ها، خزانه، ابزارها) از ماژول جدا می‌آیند تا این فایل
 * غول‌پیکر نشود و منطق «داده» از منطق «گزارش» جدا بماند. */
const allResponses: Record<string, (args: Record<string, unknown>) => unknown> = {
  ...responses,
  calendar_overview: calendarOverviewResponse,
  list_audit_logs: () => [
    {
      id: 'audit-1', user_id: 'user-demo', username: 'admin', action: 'create',
      entity_type: 'invoice', entity_id: 'demo-sale-1',
      before_json: null, after_json: '{"number":3,"total":26400000}',
      created_at: '2026-08-20 10:12:00',
    },
    {
      id: 'audit-2', user_id: 'user-demo', username: 'admin', action: 'update',
      entity_type: 'product', entity_id: 'prod-1',
      before_json: '{"sale_price":12000000}', after_json: '{"sale_price":12500000}',
      created_at: '2026-08-19 16:40:00',
    },
  ],
  list_backups: () => [
    {name: 'backup-2026-08-20.sqlite', size_bytes: 4_718_592},
    {name: 'backup-2026-08-13.sqlite', size_bytes: 4_194_304},
  ],
  backup_database: () => ({name: `backup-${new Date().toISOString().slice(0, 10)}.sqlite`, size_bytes: 5_242_880}),
  verify_backup_file: () => 'OK',
  restore_database: () => undefined,
  get_account_mappings: () => [
    {mapping_key: 'cash_default', account_id: 'acc-1101'},
    {mapping_key: 'ar_default', account_id: 'acc-1201'},
    {mapping_key: 'ap_default', account_id: 'acc-2101'},
    {mapping_key: 'sales_revenue_default', account_id: 'acc-4100'},
    {mapping_key: 'cogs_default', account_id: 'acc-5100'},
    {mapping_key: 'sales_return_default', account_id: 'acc-4200'},
    {mapping_key: 'purchase_return_default', account_id: 'acc-5200'},
    {mapping_key: 'tax_payable_default', account_id: 'acc-2401'},
    {mapping_key: 'tax_receivable_default', account_id: 'acc-2401'},
    {mapping_key: 'sales_discount_default', account_id: 'acc-4250'},
    {mapping_key: 'purchase_discount_default', account_id: 'acc-5250'},
    {mapping_key: 'check_bounce_tracking_default', account_id: 'acc-1260'},
  ],
  set_account_mapping: () => undefined,
  ...buildExtraResponses({
    products,
    contacts,
    salesInvoices,
    purchaseInvoices,
    accounts: demoAccounts,
    treasuryAccounts,
    warehouses,
    jalaliDate,
  }),
}

/** پاسخ شبیه‌سازی‌شده برای یک فرمان. */
export function designPreviewInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = allResponses[command]
  if (handler) {
    return new Promise((resolve) => setTimeout(() => resolve(handler(args ?? {}) as T), 60))
  }
  // فرمان شبیه‌سازی‌نشده نباید صفحه را با خطا بشکند؛ پاسخ خنثی برمی‌گردد و در
  // کنسول ثبت می‌شود تا در توسعه دیده شود.
  // eslint-disable-next-line no-console
  console.warn(`[پیش‌نمایش طراحی] فرمان «${command}» شبیه‌سازی نشده است.`)
  const neutral: unknown = command.startsWith('list_') || command.startsWith('get_')
    ? []
    : null
  return new Promise((resolve) => setTimeout(() => resolve(neutral as T), 30))
}
