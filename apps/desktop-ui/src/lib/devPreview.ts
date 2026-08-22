/**
 * شبیه‌ساز پیش‌نمایش طراحی — **فقط حالت توسعه در مرورگر**.
 *
 * وقتی برنامه داخل Tauri اجرا می‌شود این فایل هیچ کاری نمی‌کند. تنها کاربرد آن،
 * بازبینی چیدمان و تجربه‌ی کاربری در مرورگر است، جایی که پل IPC وجود ندارد.
 *
 * ⚠️ این ماژول بخشی از محصول نیست: در بیلد تولیدی Tauri هرگز فعال نمی‌شود و
 * رابط کاربری هنگام فعال بودن آن یک بنر هشدار نمایش می‌دهد.
 *
 * محاسبات مالی در اینجا **بازپیاده‌سازی نشده‌اند**؛ فقط چند مجموعه‌ی نمونه از
 * ساختار پاسخ برگردانده می‌شود تا چیدمان قابل قضاوت باشد.
 */

export const isTauriRuntime = () =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const isDesignPreview = () => import.meta.env.DEV && !isTauriRuntime()

const products = [
  {id: 'p1', sku: '1101', barcode: '8901001001001', name: 'iPhone SE 2022', unit: 'دستگاه', sale_price: 111111, purchase_price: 90000, min_stock: 2},
  {id: 'p2', sku: '1002', barcode: '8901001001002', name: 'بارکدخوان Pro', unit: 'دستگاه', sale_price: 8900000, purchase_price: 6500000, min_stock: 1},
  {id: 'p3', sku: '2003', barcode: '8901001002003', name: 'برنجی ایرانی عنبربو', unit: 'کیلوگرم', sale_price: 970000, purchase_price: 820000, min_stock: 10},
]

const contacts = [
  {id: 'c1', name: 'شرکت آریا تجارت', kind: 'company', mobile: '09120000001', is_customer: true, is_supplier: false},
  {id: 'c2', name: 'رضا زاهدی', kind: 'person', mobile: '09309767300', is_customer: true, is_supplier: false},
  {id: 'c3', name: 'تأمین‌کننده آریا', kind: 'company', mobile: '09120000004', is_customer: false, is_supplier: true},
]

const warehouses = [
  {id: 'w1', name: 'انبار مرکزی', code: 'W01', is_active: true},
  {id: 'w2', name: 'انبار شعبه', code: 'W02', is_active: true},
]

/** محاسبه‌ی تقریبی فقط برای دیدن چیدمان — منبع حقیقت، موتور Rust است. */
function previewInvoiceStub(args: Record<string, unknown>) {
  const lines = (args.lines as Array<Record<string, number>>) ?? []
  const headerDiscount = Number(args.headerDiscount ?? 0)
  const freight = Number(args.freight ?? 0)
  const received = Number(args.received ?? 0)

  const computed = lines.map((line) => {
    const gross = Math.round(line.quantity * line.unit_price)
    const lineDiscount = Math.round((gross * (line.discount_bp ?? 0)) / 10000) + (line.discount_amount ?? 0)
    return {gross, lineDiscount}
  })
  const netSum = computed.reduce((total, item) => total + item.gross - item.lineDiscount, 0)

  let subtotal = 0
  let discountTotal = 0
  let dutyTotal = 0
  let vatTotal = 0
  let netTotal = 0
  let costTotal = 0
  let commissionTotal = 0

  const rows = lines.map((line, index) => {
    const {gross, lineDiscount} = computed[index]
    const base = gross - lineDiscount
    const headerShare = netSum > 0 ? Math.round((headerDiscount * base) / netSum) : 0
    const totalDiscount = lineDiscount + headerShare
    const net = gross - totalDiscount
    const duty = Math.round((net * (line.duty_bp ?? 0)) / 10000)
    const vat = Math.round(((net + duty) * (line.vat_bp ?? 0)) / 10000)
    const commission = Math.round((net * (line.commission_bp ?? 0)) / 10000)
    const cost = Math.round((line.unit_cost ?? 0) * line.quantity)
    subtotal += gross
    discountTotal += totalDiscount
    netTotal += net
    dutyTotal += duty
    vatTotal += vat
    costTotal += cost
    commissionTotal += commission
    return {
      gross,
      tier_discount: 0,
      line_discount: lineDiscount,
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

  const total = rows.reduce((sum, row) => sum + row.total, 0) + freight
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

const responses: Record<string, (args: Record<string, unknown>) => unknown> = {
  list_products: () => products,
  list_contacts: () => contacts,
  list_warehouses: () => warehouses,
  preview_invoice: previewInvoiceStub,
  build_installment_plan: (args) => {
    const total = Number(args.total ?? 0) - Number(args.downPayment ?? 0)
    const count = Math.max(1, Number(args.count ?? 1))
    const base = Math.floor(total / count)
    const remainder = total - base * count
    return Array.from({length: count}, (_, index) => ({
      number: index + 1,
      due_date: '',
      due_date_jalali: `1404/${String(6 + index).padStart(2, '0')}/30`,
      amount: base + (index < remainder ? 1 : 0),
    }))
  },
  create_sales_invoice: () => 'preview-invoice-001',
  post_sales_invoice: () => null,
  list_parties: () => ({
    rows: contacts.map((contact, index) => ({
      id: contact.id,
      code: contact.id,
      display_name: contact.name,
      party_type: contact.kind === 'company' ? 'private_legal' : 'natural',
      party_type_label: contact.kind === 'company' ? 'حقوقی غیردولتی' : 'حقیقی',
      party_function: 'person',
      party_function_label: 'شخص',
      group_title: contact.is_customer ? 'بدهکاران تجاری' : 'بستانکاران تجاری',
      is_customer: contact.is_customer,
      is_supplier: contact.is_supplier,
      mobile: contact.mobile,
      route_title: 'مسیر مرکز',
      marketer_name: 'سعید بازاریان',
      credit_limit: 500000000,
      balance: [5749885636, -610541527, 0][index] ?? 0,
      balance_status: ['debtor', 'creditor', 'settled'][index] ?? 'settled',
      balance_indicator: ['بد', 'بس', 'بی حساب'][index] ?? 'بی حساب',
    })),
    summary: {
      debtor_count: 1,
      debtor_total: 5749885636,
      creditor_count: 1,
      creditor_total: 610541527,
      settled_count: 1,
      total_count: 3,
      net_total: 5139344109,
    },
  }),
  list_party_routes: () => [
    {id: 'route-center', code: 'R02', title: 'مسیر مرکز'},
    {id: 'route-north', code: 'R01', title: 'مسیر شمال شهر'},
  ],
  list_product_groups: () => [
    {id: 'g1', code: '1', title: 'مواد غذایی', product_count: 1},
    {id: 'g2', code: '2', title: 'کالاهای متفرقه', product_count: 2},
  ],
  list_product_prices: () =>
    products.map((product) => ({
      id: product.id,
      sku: product.sku,
      name: product.name,
      kind: 'simple',
      kind_label: 'کالای عمومی (ساده)',
      group_title: 'کالاهای متفرقه',
      prices: [
        {level: 'retail', label: 'جزئی', price: product.sale_price},
        {level: 'wholesale', label: 'کلی', price: Math.round(product.sale_price * 0.95)},
        {level: 'partner', label: 'همکار', price: Math.round(product.sale_price * 0.9)},
        {level: 'partner_tier2', label: 'همکار درجه ۲', price: null},
        {level: 'partner_tier3', label: 'همکار درجه ۳', price: null},
        {level: 'seasonal', label: 'فصلی', price: null},
        {level: 'exhibition', label: 'نمایشگاه', price: null},
      ],
    })),
  list_postable_accounts: () => [
    {id: 'acc-1201', code: '1201', name: 'حساب مشتریان', nature: 'debit', requires_subsidiary: true, requires_cost_center: false, requires_project: false},
    {id: 'acc-4100', code: '4100', name: 'فروش کالا', nature: 'credit', requires_subsidiary: false, requires_cost_center: false, requires_project: false},
  ],
  list_cost_centers: () => [{id: 'cc-sales', code: '4001', title: 'واحد فروش'}],
  list_stocktakes: () => [
    {
      id: 'st-1',
      title: 'انبارگردانی پایان سال ۱۴۰۵',
      warehouse_name: 'انبار مرکزی',
      count_date: '1405/05/29',
      status: 'counting',
      status_label: 'در حال شمارش',
      total_lines: 3,
      counted_lines: 2,
      variance_lines: 2,
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
    lines: [
      {id: 'l1', product_id: 'p1', product_name: 'iPhone SE 2022', sku: '1101', frozen_quantity: 7, counted_quantity: 5, recount_quantity: null, final_quantity: 5, variance: -2, variance_value: -180000, variance_approved: false, needs_recount: true, unit_cost: 90000},
      {id: 'l2', product_id: 'p2', product_name: 'بارکدخوان Pro', sku: '1002', frozen_quantity: 10, counted_quantity: 12, recount_quantity: null, final_quantity: 12, variance: 2, variance_value: 13000000, variance_approved: true, needs_recount: true, unit_cost: 6500000},
      {id: 'l3', product_id: 'p3', product_name: 'برنجی ایرانی عنبربو', sku: '2003', frozen_quantity: 40, counted_quantity: null, recount_quantity: null, final_quantity: null, variance: null, variance_value: 0, variance_approved: false, needs_recount: false, unit_cost: 820000},
    ],
    total_lines: 3,
    counted_lines: 2,
    uncounted_lines: 1,
    surplus_lines: 1,
    shortage_lines: 1,
    unapproved_variances: 1,
    surplus_value: 13000000,
    shortage_value: 180000,
    net_value: 12820000,
    can_post: false,
    blocking_reason: 'STK-003: شمارش همه‌ی اقلام کامل نشده است: 1 قلم باقی مانده',
  }),
  set_stocktake_count: () => null,
  approve_all_variances: () => 1,
  post_stocktake: () => 'journal-demo-1',
  get_low_stock: () => [
    {product_id: 'p1', product_name: 'iPhone SE 2022', sku: '1101', quantity: 2, reorder_point: 5},
    {product_id: 'p4', product_name: 'پیراهن مردانه', sku: '1', quantity: 0, reorder_point: 3},
  ],
  list_valuation_methods: () => [
    {method: 'fifo', label: 'اولین صادره از اولین وارده (FIFO)', is_active: true, explanation: 'کالایی که زودتر خریده‌اید، زودتر هم فروخته می‌شود. پس بهای فروش از قدیمی‌ترین خرید برداشته می‌شود و آنچه در انبار می‌ماند با قیمت خریدهای جدیدتر ارزش‌گذاری می‌گردد. مناسب کالای تاریخ‌دار و بازاری که قیمت‌ها بالا می‌رود.'},
    {method: 'moving_average', label: 'میانگین متحرک', is_active: false, explanation: 'با هر خرید جدید، میانگین قیمت کالا دوباره حساب می‌شود. همه‌ی فروش‌های بعدی با همان میانگین ثبت می‌شوند. سود و زیان یکنواخت‌تر می‌شود و نوسان قیمت خرید کمتر به چشم می‌آید.'},
    {method: 'weighted_average', label: 'میانگین موزون', is_active: false, explanation: 'میانگین قیمت کل خریدهای یک دوره محاسبه و برای همه‌ی فروش‌های آن دوره استفاده می‌شود. ساده‌ترین روش برای گزارش‌گیری دوره‌ای است.'},
  ],
  list_projects: () => [{id: 'project-demo', code: '5001', title: 'پروژه نمونه'}],
}

/** پاسخ شبیه‌سازی‌شده برای یک فرمان، یا خطا اگر تعریف نشده باشد. */
export function designPreviewInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = responses[command]
  if (!handler) {
    return Promise.reject(
      new Error(`PREVIEW-001: فرمان «${command}» در پیش‌نمایش طراحی شبیه‌سازی نشده است`),
    )
  }
  return new Promise((resolve) => setTimeout(() => resolve(handler(args ?? {}) as T), 80))
}
