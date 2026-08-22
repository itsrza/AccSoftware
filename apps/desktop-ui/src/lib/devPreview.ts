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
  list_inventory_transfer_orders: () => [],
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
  list_treasury_accounts: () => [
    {id: 'treasury-cash-1', name: 'صندوق مرکزی', account_type: 'cash', balance: 184_500_000},
    {id: 'treasury-cash-2', name: 'صندوق فروشگاه', account_type: 'cash', balance: 42_300_000},
    {id: 'treasury-bank-mellat', name: 'بانک ملت — جاری ۱۲۳۴', account_type: 'bank', balance: 1_284_000_000},
    {id: 'treasury-bank-saderat', name: 'بانک صادرات — جاری ۵۶۷۸', account_type: 'bank', balance: 620_500_000},
    {id: 'treasury-petty', name: 'تنخواه اداری', account_type: 'petty_cash', balance: 15_000_000},
  ],
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
    salesInvoices.slice(-8).map((invoice) => ({
      id: invoice.id,
      number: invoice.number,
      invoice_date: invoice.invoice_date,
      contact_name: invoice.contact_name,
      total: invoice.total,
      payment_status: invoice.payment_status,
    })),
  get_trial_balance: () => [
    {code: '1101', name: 'صندوق', debit: 241_800_000, credit: 0},
    {code: '1201', name: 'حساب مشتریان', debit: 1_482_000_000, credit: 0},
    {code: '1300', name: 'موجودی کالا', debit: 980_000_000, credit: 0},
    {code: '2101', name: 'تأمین‌کنندگان', debit: 0, credit: 420_000_000},
    {code: '4100', name: 'فروش کالا', debit: 0, credit: 2_283_800_000},
  ],
  list_custom_reports: () => [],
  list_print_templates: () => [],
  list_api_profiles: () => [],
  list_plugins: () => [],
  list_permissions: () => [],
  list_backups: () => [],
}

/** پاسخ شبیه‌سازی‌شده برای یک فرمان. */
export function designPreviewInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = responses[command]
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
