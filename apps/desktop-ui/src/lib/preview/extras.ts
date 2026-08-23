/**
 * پاسخ‌های پیش‌نمایش برای گزارش‌ها، خزانه و ابزارها.
 *
 * ## چرا فایل جدا
 * `devPreview.ts` داده‌ی پایه (کالا، شخص، فاکتور، چک…) را می‌سازد. این فایل
 * روی همان داده **مشتق** می‌سازد: گزارش فروش، دفتر کل، تراز، مانده‌ی
 * اشخاص، سنی‌سازی، کاردکس و… . جدا نگه داشتنش دو فایده دارد: هیچ‌کدام
 * غول‌پیکر نمی‌شوند، و منطق «مشتق» از منطق «داده» جدا می‌ماند.
 *
 * ## قاعده‌ی داده‌ی نمونه
 * هیچ عددی تصادفی نیست و هیچ عددی هم از هوا نمی‌آید: هر گزارش از همان
 * فاکتورها و اسنادی ساخته می‌شود که در بقیه‌ی صفحه‌ها دیده می‌شوند. اگر
 * کاربر جمع گزارش فروش را با فهرست فاکتورها مقایسه کند، باید بخواند.
 */

export type PreviewInvoice = {
  id: string
  number: number
  invoice_date: string
  contact_id: string
  contact_name: string
  warehouse_id: string
  warehouse_name: string
  status: string
  payment_status: string
  subtotal: number
  discount: number
  tax: number
  total: number
}

export type PreviewProduct = {
  id: string
  sku: string
  name: string
  unit: string
  group: string
  sale_price: number
  purchase_price: number
  min_stock: number
  quantity: number
}

export type PreviewContact = {
  id: string
  name: string
  kind: string
  is_customer: boolean
  is_supplier: boolean
  balance: number
}

export type PreviewAccount = {
  id: string
  code: string
  name: string
  /** شماره‌ی سطح در طرح کدینگ (۰ = گروه … ۳ = تفصیلی). */
  level: number
  is_postable: boolean
  nature: string
  debit: number
  credit: number
}

export type PreviewTreasuryAccount = {
  id: string
  name: string
  account_type: string
  balance: number
  is_active: boolean
}

export type PreviewWarehouse = { id: string; name: string; code: string }

export type PreviewDataset = {
  products: PreviewProduct[]
  contacts: PreviewContact[]
  salesInvoices: PreviewInvoice[]
  purchaseInvoices: PreviewInvoice[]
  accounts: PreviewAccount[]
  treasuryAccounts: PreviewTreasuryAccount[]
  warehouses: PreviewWarehouse[]
  jalaliDate: (index: number) => string
}

import { catalogResponses } from './catalog'

type Handler = (args: Record<string, unknown>) => unknown

const sum = (values: number[]) => values.reduce((total, value) => total + value, 0)

/** فیلتر بازه‌ی تاریخ شمسی — مقایسه‌ی متنی چون قالب `YYYY/MM/DD` است. */
function inRange(date: string, from?: unknown, to?: unknown): boolean {
  if (typeof from === 'string' && from && date < from) return false
  if (typeof to === 'string' && to && date > to) return false
  return true
}

// ---------------------------------------------------------------------------
// قالب‌های چاپ، گزارش‌های ذخیره‌شده، اتصالات و افزونه‌ها
// ---------------------------------------------------------------------------

const printTemplates = [
  {
    id: 'tpl-invoice-a4',
    name: 'فاکتور فروش A4',
    template_type: 'invoice',
    is_default: true,
    content_html: JSON.stringify({
      version: 1,
      paper: 'A4',
      showLogo: true,
      logoHeightMm: 16,
      showCompanyName: true,
      showPhone: true,
      showAddress: true,
      showEconomicCode: true,
      title: 'فاکتور فروش',
      showDocumentNumber: true,
      showDate: true,
      showParty: true,
      showPartyPhone: true,
      columns: ['row', 'code', 'name', 'quantity', 'unit', 'unit_price', 'discount', 'vat', 'line_total'],
      zebra: true,
      showSubtotal: true,
      showDiscount: true,
      showVat: true,
      showTotal: true,
      showAmountInWords: true,
      footerNote: '',
      showSignature: true,
      showBarcode: true,
      fontScale: 1,
    }),
  },
  {
    id: 'tpl-receipt-80',
    name: 'رسید حرارتی ۸۰ میلی‌متر',
    template_type: 'receipt',
    is_default: true,
    content_html: JSON.stringify({
      version: 1,
      paper: '80mm',
      showLogo: true,
      logoHeightMm: 12,
      showCompanyName: true,
      showPhone: true,
      showAddress: false,
      showEconomicCode: false,
      title: 'رسید فروش',
      showDocumentNumber: true,
      showDate: true,
      showParty: true,
      showPartyPhone: false,
      columns: ['name', 'quantity', 'unit_price', 'line_total'],
      zebra: false,
      showSubtotal: true,
      showDiscount: true,
      showVat: true,
      showTotal: true,
      showAmountInWords: false,
      footerNote: 'از خرید شما سپاسگزاریم',
      showSignature: false,
      showBarcode: true,
      fontScale: 0.95,
    }),
  },
  {
    id: 'tpl-journal',
    name: 'سند حسابداری رسمی',
    template_type: 'journal',
    is_default: true,
    content_html:
      '<section dir="rtl"><h2>سند شماره {{journal.number}}</h2><p>{{journal.date}}</p>' +
      '{{#lines}}<div>{{account.code}} {{account.name}} — بدهکار {{debit}} — بستانکار {{credit}}</div>{{/lines}}</section>',
  },
  {
    id: 'tpl-label',
    name: 'برچسب قفسه کالا',
    template_type: 'label',
    is_default: true,
    content_html:
      '<section dir="rtl" style="width:50mm"><b>{{product.name}}</b><div>{{product.sku}}</div>' +
      '<div>{{product.price}} ریال</div></section>',
  },
]

const customReports = [
  {
    id: 'rep-sales-by-customer',
    name: 'فروش به تفکیک مشتری',
    source: 'sales',
    config_json:
      '{"columns":["contact_name","total","tax"],"groupBy":"contact_name","sort":"total","direction":"desc","search":""}',
    created_at: '1405/03/12',
    updated_at: '1405/05/02',
  },
  {
    id: 'rep-purchase-monthly',
    name: 'خرید ماهانه',
    source: 'purchase',
    config_json: '{"columns":["date","invoice_number","total"],"groupBy":"date","sort":"date","direction":"asc","search":""}',
    created_at: '1405/04/01',
    updated_at: '1405/04/01',
  },
  {
    id: 'rep-inventory-value',
    name: 'ارزش موجودی انبارها',
    source: 'inventory',
    config_json:
      '{"columns":["product_name","warehouse_name","quantity","value"],"groupBy":"warehouse_name","sort":"value","direction":"desc","search":""}',
    created_at: '1405/02/20',
    updated_at: '1405/05/18',
  },
]

const apiProfiles = [
  {
    id: 'api-tax',
    name: 'سامانه مؤدیان — کارپوشه',
    base_url: 'https://tp.tax.gov.ir',
    auth_type: 'bearer' as const,
    auth_header: 'Authorization',
    timeout_ms: 15000,
    enabled: true,
    allowed_domains: 'tax.gov.ir',
  },
  {
    id: 'api-sms',
    name: 'پیامک یادآوری چک',
    base_url: 'https://api.sms-provider.ir',
    auth_type: 'api_key' as const,
    auth_header: 'X-API-KEY',
    timeout_ms: 8000,
    enabled: true,
    allowed_domains: 'sms-provider.ir',
  },
  {
    id: 'api-sayad',
    name: 'استعلام صیادی',
    base_url: 'https://sayad.cbi.ir',
    auth_type: 'basic' as const,
    auth_header: undefined,
    timeout_ms: 12000,
    enabled: false,
    allowed_domains: 'cbi.ir',
  },
]

const plugins = [
  {
    id: 'plugin-barcode',
    name: 'اسکنر بارکد USB',
    version: '1.2.0',
    description: 'خواندن بارکد از دستگاه‌های HID و درج خودکار در سطر فاکتور',
    enabled: true,
    permissions: ['device.hid', 'invoice.write'],
  },
  {
    id: 'plugin-backup-cloud',
    name: 'پشتیبان‌گیری ابری',
    version: '0.9.4',
    description: 'ارسال پشتیبان رمزنگاری‌شده به فضای ابری سازمان',
    enabled: false,
    permissions: ['fs.read', 'network.upload'],
  },
  {
    id: 'plugin-pos',
    name: 'درگاه کارتخوان',
    version: '2.0.1',
    description: 'اتصال مستقیم به پایانه فروشگاهی و ثبت خودکار سند دریافت',
    enabled: true,
    permissions: ['device.serial', 'treasury.write'],
  },
]

const backups = [
  { name: 'backup-1405-05-30.npdb', size: 4_812_544, created_at: '1405/05/30 23:10', verified: true },
  { name: 'backup-1405-05-23.npdb', size: 4_690_112, created_at: '1405/05/23 23:10', verified: true },
  { name: 'backup-1405-05-16.npdb', size: 4_512_000, created_at: '1405/05/16 23:10', verified: true },
]

const permissions = [
  { id: 'invoice.create', title: 'صدور فاکتور', group: 'فروش', granted: true },
  { id: 'invoice.delete', title: 'ابطال فاکتور', group: 'فروش', granted: false },
  { id: 'journal.post', title: 'ثبت قطعی سند', group: 'حسابداری', granted: true },
  { id: 'treasury.pay', title: 'ثبت پرداخت', group: 'خزانه', granted: true },
  { id: 'settings.sensitive', title: 'تغییر تنظیمات حساس', group: 'مدیریت', granted: false },
]

// ---------------------------------------------------------------------------
// سازنده‌ی پاسخ‌ها
// ---------------------------------------------------------------------------

export function buildExtraResponses(data: PreviewDataset): Record<string, Handler> {
  const {
    products,
    contacts,
    salesInvoices,
    purchaseInvoices,
    accounts,
    treasuryAccounts,
    warehouses,
    jalaliDate,
  } = data

  /** سطر گزارش فروش/خرید از روی سربرگ فاکتور. */
  const reportRow = (invoice: PreviewInvoice) => ({
    date: invoice.invoice_date,
    invoice_number: invoice.number,
    contact_name: invoice.contact_name,
    subtotal: invoice.subtotal,
    discount: invoice.discount,
    tax: invoice.tax,
    total: invoice.total,
    payment_status: invoice.payment_status,
  })

  /** مانده‌ی طرف حساب از روی فاکتورهای همان شخص. */
  const partyBalances = (invoices: PreviewInvoice[]) => {
    const map = new Map<string, { name: string; count: number; invoiced: number; settled: number }>()
    for (const invoice of invoices) {
      const entry = map.get(invoice.contact_id) ?? {
        name: invoice.contact_name,
        count: 0,
        invoiced: 0,
        settled: 0,
      }
      entry.count += 1
      entry.invoiced += invoice.total
      if (invoice.payment_status === 'paid') entry.settled += invoice.total
      map.set(invoice.contact_id, entry)
    }
    return [...map.entries()]
      .map(([contact_id, entry]) => ({
        contact_id,
        contact_name: entry.name,
        invoice_count: entry.count,
        invoiced: entry.invoiced,
        settled: entry.settled,
        remaining: entry.invoiced - entry.settled,
      }))
      .filter((row) => row.remaining > 0)
      .sort((a, b) => b.remaining - a.remaining)
  }

  const treasuryBalances = treasuryAccounts.map((account) => ({
    id: account.id,
    name: account.name,
    account_type: account.account_type,
    balance: account.balance,
    linked_account_id: undefined,
  }))

  /** گردش خزانه‌ی نمونه با مانده‌ی تجمعی درست. */
  const treasuryTransactions = Array.from({ length: 40 }, (_, index) => {
    const account = treasuryAccounts[index % treasuryAccounts.length]
    const receipt = index % 3 !== 2
    return {
      id: `demo-tx-${String(index).padStart(3, '0')}`,
      transaction_type: receipt ? 'receipt' : 'payment',
      amount: ((index % 12) + 1) * 6_500_000,
      transaction_date: jalaliDate(index),
      description: receipt ? 'دریافت از مشتری' : 'پرداخت به تأمین‌کننده',
      treasury_account_id: account.id,
      reference_type: receipt ? 'treasury_receipt' : 'treasury_payment',
      reference_id: `demo-doc-${String(index).padStart(3, '0')}`,
    }
  })

  const revenue = sum(salesInvoices.map((invoice) => invoice.subtotal))
  const cogs = Math.round(revenue * 0.72)
  const salesReturns = Math.round(revenue * 0.018)

  return {
    // ---------------------------------------------------------------- گزارش‌ها
    get_sales_report: (args) =>
      salesInvoices
        .filter((invoice) => inRange(invoice.invoice_date, args.fromDate, args.toDate))
        .map(reportRow),

    get_purchase_report: (args) =>
      purchaseInvoices
        .filter((invoice) => inRange(invoice.invoice_date, args.fromDate, args.toDate))
        .map(reportRow),

    get_inventory_valuation: () =>
      products.slice(0, 40).map((product, index) => {
        const warehouse = warehouses[index % warehouses.length]
        return {
          product_id: product.id,
          product_name: product.name,
          warehouse_id: warehouse.id,
          warehouse_name: warehouse.name,
          quantity: product.quantity,
          average_cost: product.purchase_price,
          value: product.quantity * product.purchase_price,
        }
      }),

    get_account_ledger_summary: () =>
      accounts
        .filter((account) => account.is_postable || account.debit > 0 || account.credit > 0)
        .map((account) => ({
          account_id: account.id,
          code: account.code,
          name: account.name,
          debit: account.debit,
          credit: account.credit,
          balance: account.debit - account.credit,
        })),

    get_account_ledger: (args) => {
      const accountId = String(args.accountId ?? '')
      let running = 0
      return Array.from({ length: 14 }, (_, index) => {
        const debit = index % 2 === 0 ? ((index % 5) + 1) * 9_400_000 : 0
        const credit = index % 2 === 0 ? 0 : ((index % 4) + 1) * 7_200_000
        running += debit - credit
        return {
          date: jalaliDate(index),
          journal_number: 2000 + index,
          journal_id: `demo-journal-${index}`,
          description: debit > 0 ? 'بدهکار شدن حساب بابت فاکتور' : 'بستانکار شدن حساب بابت دریافت',
          account_id: accountId,
          debit,
          credit,
          running_balance: running,
        }
      })
    },

    get_journal_book: (args) =>
      Array.from({ length: 30 }, (_, index) => {
        const account = accounts[index % accounts.length]
        const debit = index % 2 === 0 ? ((index % 6) + 1) * 5_800_000 : 0
        return {
          date: jalaliDate(index),
          number: 2000 + index,
          description: index % 2 === 0 ? 'ثبت فاکتور فروش' : 'ثبت دریافت از مشتری',
          account_code: account.code,
          account_name: account.name,
          debit,
          credit: debit === 0 ? ((index % 6) + 1) * 5_800_000 : 0,
        }
      }).filter((line) => inRange(line.date, args.fromDate, args.toDate)),

    get_profit_loss: () => ({
      revenue,
      sales_returns: salesReturns,
      net_revenue: revenue - salesReturns,
      cogs,
      gross_profit: revenue - salesReturns - cogs,
      gross_margin_percent:
        Math.round(((revenue - salesReturns - cogs) / (revenue - salesReturns)) * 1000) / 10,
    }),

    get_financial_statement: (args) => {
      const balanceSheet = args.statement !== 'income_statement'
      const lines = balanceSheet
        ? [
            { code: '1101', name: 'صندوق و تنخواه', amount: 241_800_000, nature: 'debit' },
            { code: '1103', name: 'بانک‌ها', amount: 1_904_500_000, nature: 'debit' },
            { code: '1301', name: 'حساب‌های دریافتنی', amount: sum(contacts.filter((c) => c.balance > 0).map((c) => c.balance)), nature: 'debit' },
            { code: '1105', name: 'موجودی کالا', amount: sum(products.map((p) => p.quantity * p.purchase_price)), nature: 'debit' },
            { code: '2101', name: 'حساب‌های پرداختنی', amount: Math.abs(sum(contacts.filter((c) => c.balance < 0).map((c) => c.balance))), nature: 'credit' },
            { code: '2400', name: 'مالیات پرداختنی', amount: sum(salesInvoices.map((i) => i.tax)), nature: 'credit' },
            { code: '3101', name: 'سرمایه', amount: 2_000_000_000, nature: 'credit' },
          ]
        : [
            { code: '4100', name: 'فروش کالا', amount: revenue, nature: 'credit' },
            { code: '4110', name: 'برگشت از فروش', amount: salesReturns, nature: 'debit' },
            { code: '6100', name: 'بهای تمام‌شده کالای فروش‌رفته', amount: cogs, nature: 'debit' },
            { code: '6200', name: 'هزینه‌های اداری', amount: 184_000_000, nature: 'debit' },
            { code: '6300', name: 'هزینه‌های فروش و بازاریابی', amount: 96_500_000, nature: 'debit' },
          ]
      const total = lines.reduce(
        (acc, line) => acc + (line.nature === 'debit' ? line.amount : -line.amount),
        0,
      )
      return {
        title: balanceSheet ? 'ترازنامه' : 'صورت سود و زیان',
        as_of: typeof args.asOf === 'string' && args.asOf ? args.asOf : '1405/05/30',
        lines,
        total: Math.abs(total),
      }
    },

    get_party_aging: (args) => {
      const sales = args.sales !== false
      const source = sales ? salesInvoices : purchaseInvoices
      return partyBalances(source)
        .slice(0, 20)
        .map((row, index) => {
          // توزیع قطعی مانده در سطل‌های سنی، بدون تصادف.
          const buckets = [
            [60, 25, 10, 5, 0],
            [30, 30, 20, 15, 5],
            [10, 20, 25, 25, 20],
          ][index % 3]
          const share = (percent: number) => Math.round((row.remaining * percent) / 100)
          const current = share(buckets[0])
          const d30 = share(buckets[1])
          const d60 = share(buckets[2])
          const d90 = share(buckets[3])
          return {
            contact_id: row.contact_id,
            contact_name: row.contact_name,
            current,
            days_1_30: d30,
            days_31_60: d60,
            days_61_90: d90,
            over_90: row.remaining - current - d30 - d60 - d90,
            total: row.remaining,
          }
        })
    },

    get_receivables: () => partyBalances(salesInvoices),
    get_payables: () => partyBalances(purchaseInvoices),

    get_stock_card: (args) => {
      const product = products.find((item) => item.id === args.productId) ?? products[0]
      let balance = 0
      return Array.from({ length: 16 }, (_, index) => {
        const inbound = index % 3 !== 2
        const quantity = (index % 7) + 2
        balance += inbound ? quantity : -Math.min(quantity, balance)
        return {
          date: jalaliDate(index),
          movement_type: inbound ? 'receipt' : 'issue',
          quantity,
          unit_cost: product.purchase_price,
          balance,
          reference_type: inbound ? 'purchase_invoice' : 'sales_invoice',
          note: inbound ? 'رسید خرید' : 'حواله فروش',
        }
      })
    },

    // ------------------------------------------------------------------ خزانه
    list_treasury_balances: () => treasuryBalances,
    get_cash_position: () => ({
      total: sum(treasuryAccounts.map((account) => account.balance)),
      accounts: treasuryBalances,
    }),
    list_treasury_transactions_filtered: (args) =>
      treasuryTransactions.filter(
        (tx) =>
          (!args.treasuryAccountId || tx.treasury_account_id === args.treasuryAccountId) &&
          inRange(tx.transaction_date, args.fromDate, args.toDate),
      ),
    get_treasury_statement: (args) => {
      let running = 0
      return treasuryTransactions
        .filter((tx) => tx.treasury_account_id === args.treasuryAccountId)
        .filter((tx) => inRange(tx.transaction_date, args.fromDate, args.toDate))
        .map((tx) => {
          running += tx.transaction_type === 'receipt' ? tx.amount : -tx.amount
          return { ...tx, running_balance: running }
        })
    },
    create_treasury_transaction: () => 'demo-tx-new',
    create_treasury_transfer: () => 'demo-transfer-new',
    create_treasury_account: () => 'treasury-new',
    update_treasury_account: () => null,

    // -------------------------------------------------------------- سال مالی
    get_fiscal_period_status: () => ({
      id: 'fy-demo',
      title: '۱۴۰۵',
      start_date: '1405/01/01',
      end_date: '1405/12/29',
      is_closed: false,
      draft_journals: 3,
      posted_journals: 128,
    }),
    close_fiscal_year: () => null,
    settle_invoice: () => 'demo-settlement-1',

    // ---------------------------------------------------- قالب چاپ و گزارش‌ساز
    list_print_templates: () => printTemplates,
    save_print_template: () => 'tpl-new',
    delete_print_template: () => null,
    list_custom_reports: () => customReports,
    save_custom_report: () => 'rep-new',
    delete_custom_report: () => null,

    // -------------------------------------------------- اتصالات، افزونه، پشتیبان
    list_api_profiles: () => apiProfiles,
    create_api_profile: () => 'api-new',
    set_api_profile_enabled: () => null,
    execute_api_request: () => ({
      status: 200,
      content_type: 'application/json',
      body: '{"ok":true,"message":"پاسخ نمونه — در پیش‌نمایش هیچ درخواست واقعی ارسال نمی‌شود"}',
    }),
    list_plugins: () => plugins,
    register_plugin: () => 'plugin-new',
    set_plugin_enabled: () => null,
    execute_plugin: () => 'اجرای نمونه با موفقیت انجام شد.',
    list_permissions: () => permissions,
    list_backups: () => backups,
    verify_backup_file: () => 'فایل پشتیبان سالم است و قابل بازیابی می‌باشد.',

    ...catalogResponses(data),

    // ------------------------------------------------------- عملیات نوشتنی
    create_contact: () => 'demo-contact-new',
    update_contact: () => null,
    delete_contact: () => null,
    create_product: () => 'demo-prod-new',
    update_product: () => null,
    delete_product: () => null,
    create_journal: () => 'demo-journal-new',
    create_check: () => 'demo-check-new',
    receive_stock: () => 'demo-movement-in',
    issue_stock: () => 'demo-movement-out',
    transfer_stock: () => 'demo-movement-transfer',
    adjust_stock: () => 'demo-movement-adjust',
    reserve_inventory: () => null,
    release_inventory: () => null,
    create_inventory_lot: () => 'demo-lot-new',
    create_inventory_count: () => 'demo-count-new',
    set_inventory_count_line: () => null,
    post_inventory_count: () => 'demo-journal-count',
    set_inventory_valuation_method: () => null,
    create_purchase_invoice: () => 'demo-purchase-new',
    post_purchase_invoice: () => 'demo-journal-purchase',
    create_sales_return: () => 'demo-return-new',
    post_sales_return: () => 'demo-journal-return',
    create_purchase_return: () => 'demo-preturn-new',
    post_purchase_return: () => 'demo-journal-preturn',
    import_data: (args) => {
      const rows = JSON.parse(String(args.rowsJson ?? '[]')) as unknown[]
      return `${rows.length} ردیف در حالت پیش‌نمایش بررسی شد (چیزی ذخیره نشد).`
    },
  }
}
