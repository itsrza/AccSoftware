/**
 * پاسخ‌های پیش‌نمایش برای کالا و خدمات.
 *
 * چرا فایل جدا: فرم تعریف کالا پرجزئیات‌ترین فرم برنامه است (هفت سطح قیمت،
 * چند واحدی، مالیات، تخفیف پلکانی، طلا). داده‌ی نمونه‌اش به‌تنهایی به اندازه‌ی
 * یک ماژول است و ماندنش در `extras.ts` آن فایل را غول‌پیکر می‌کرد.
 */

import { toJalali } from '../format'
import { jalaliToDate, shiftJalali } from '../dateRange'
import type { PreviewDataset } from './extras'

type Handler = (args: Record<string, unknown>) => unknown

/** هفت سطح قیمت نمونه از روی قیمت پایه. */
function priceLevels(base: number) {
  const factors: [string, string, number | null][] = [
    ['retail', 'جزئی', 1],
    ['wholesale', 'کلی', 0.94],
    ['partner', 'همکار', 0.88],
    ['partner_tier2', 'همکار درجه ۲', 0.85],
    ['partner_tier3', 'همکار درجه ۳', null],
    ['seasonal', 'فصلی', 0.9],
    ['exhibition', 'نمایشگاه', null],
  ]
  return factors.map(([level, label, factor]) => ({
    level,
    label,
    price: factor === null ? null : Math.round(base * factor),
  }))
}

export function catalogResponses(data: PreviewDataset): Record<string, Handler> {
  const { products, warehouses } = data
  return {
  // ----------------------------------------------------------- کالا
  list_product_kinds: () => ({
    kinds: [
      {value: 'simple', label: 'کالای عمومی', tracks_inventory: true},
      {value: 'composite', label: 'کالای مرکب', tracks_inventory: true},
      {value: 'variant', label: 'کالای تنوع‌دار', tracks_inventory: true},
      {value: 'gold_jewelry', label: 'طلا و جواهر', tracks_inventory: true},
      {value: 'service', label: 'خدمت', tracks_inventory: false},
    ],
    levels: [
      {value: 'retail', label: 'جزئی'},
      {value: 'wholesale', label: 'کلی'},
      {value: 'partner', label: 'همکار'},
      {value: 'partner_tier2', label: 'همکار درجه ۲'},
      {value: 'partner_tier3', label: 'همکار درجه ۳'},
      {value: 'seasonal', label: 'فصلی'},
      {value: 'exhibition', label: 'نمایشگاه'},
    ],
  }),

  list_products_detailed: () =>
    products.map((product, index) => ({
      id: product.id,
      kind: index % 17 === 0 ? 'service' : index % 23 === 0 ? 'gold_jewelry' : 'simple',
      kind_label:
        index % 17 === 0 ? 'خدمت' : index % 23 === 0 ? 'طلا و جواهر' : 'کالای عمومی',
      sku: product.sku,
      barcode: `690${String(1000000 + index).padStart(10, '0')}`,
      name: product.name,
      unit: product.unit,
      group_title: product.group,
      quantity: product.quantity,
      retail_price: product.sale_price,
      partner_price: Math.round(product.sale_price * 0.88),
      purchase_price: product.purchase_price,
      min_stock: product.min_stock,
      vat_basis_points: index % 11 === 0 ? 0 : 900,
      tax_exempt: index % 11 === 0,
    })),

  get_product_profile: (args) => {
    const product = products.find((item) => item.id === args.id) ?? products[0]
    const index = products.indexOf(product)
    return {
      id: product.id,
      kind: 'simple',
      kind_label: 'کالای عمومی',
      sku: product.sku,
      barcode: `690${String(1000000 + index).padStart(10, '0')}`,
      name: product.name,
      display_name: undefined,
      brand: undefined,
      group_id: undefined,
      group_title: product.group,
      unit: product.unit,
      sale_price: product.sale_price,
      purchase_price: product.purchase_price,
      min_stock: product.min_stock,
      max_stock: product.min_stock * 10,
      reorder_point: product.min_stock * 2,
      vat_basis_points: 900,
      duty_basis_points: 0,
      tax_code: undefined,
      tax_exempt: false,
      prices: priceLevels(product.sale_price),
      units: [{unit_name: 'کارتن', factor: 12, is_default_sale: false}],
      tiers: [{min_quantity: 50, discount_bp: 500}],
      gold: undefined,
      stock: warehouses.slice(0, 3).map((warehouse, position) => ({
        warehouse_id: warehouse.id,
        warehouse_name: warehouse.name,
        quantity: Math.max(0, product.quantity - position * 3),
      })),
      total_stock: product.quantity,
    }
  },
  save_product_profile: (args) => {
    const input = args.input as {id?: string} | undefined
    return input?.id ?? 'demo-prod-new'
  },
  preview_gold_price: (args) => {
    const rate = Number(args.ratePerGram ?? 0)
    const metal = Math.round(rate * 4.2)
    const making = Math.round(metal * 0.12)
    const profit = Math.round((metal + making) * 0.07)
    const vat = Math.round((metal + making + profit) * 0.09)
    return {metal_value: metal, making_charge: making, profit, vat, total: metal + making + profit + vat}
  },

  // ------------------------------------------------------- کاردکس کالا
  // ساخت حرکت‌های قطعی از خود کالای درخواستی — نه اعداد تصادفی.
  // ماند هر سطر واقعاً تجمعی حساب می‌شود تا نمودار پیش‌نمایش درست باشد.
  product_cardex: (args) => {
    const product = products.find((item) => item.id === args.productId) ?? products[0]
    const kind = String(args.kind ?? 'all')
    const warehouse = warehouses[0]
    const today = toJalali(new Date())
    const pad = (value: number) => String(value).padStart(2, '0')
    const todayStr = `${today.year}/${pad(today.month)}/${pad(today.day)}`
    const daysAgo = (offset: number) => shiftJalali(todayStr, -offset) ?? todayStr
    const iso = (jalali: string) => {
      const [year, month, day] = jalali.split('/').map(Number)
      return jalaliToDate(year, month, day).toISOString().slice(0, 10)
    }

    const cost = product.purchase_price
    const drafts = [
      // (کانال، جهت، مقدار، بهای واحد، نوع سند، شماره، روز‌های پیش، یادداشت)
      {channel: 'internal', flow: 'in', quantity: product.quantity + 6, unitCost: cost, doc: 'opening', number: null as number | null, offset: 120, note: 'موجودی اول دوره'},
      {channel: 'purchase', flow: 'in', quantity: 4, unitCost: cost, doc: 'purchase_invoice', number: 12, offset: 21, note: 'خرید'},
      {channel: 'sales', flow: 'in', quantity: 1, unitCost: 0, doc: 'sales_return', number: 2, offset: 14, note: 'برگشت از فروش'},
      {channel: 'sales', flow: 'out', quantity: 3, unitCost: 0, doc: 'sales_invoice', number: 7, offset: 7, note: 'فروش'},
      {channel: 'purchase', flow: 'out', quantity: 2, unitCost: cost, doc: 'purchase_return', number: 3, offset: 3, note: 'برگشت از خرید'},
    ]
      .filter((row) => kind === 'all' || row.channel === kind)
      .map((row) => ({...row, jalali: daysAgo(row.offset)}))
      .sort((a, b) => (a.jalali < b.jalali ? -1 : a.jalali > b.jalali ? 1 : 0))

    let balance = 0
    let totalIn = 0
    let totalOut = 0
    const entries = drafts.map((row) => {
      if (row.flow === 'in') {
        balance += row.quantity
        totalIn += row.quantity
      } else {
        balance -= row.quantity
        totalOut += row.quantity
      }
      return {
        date_iso: iso(row.jalali),
        date_jalali: row.jalali,
        warehouse_name: warehouse?.name ?? 'انبار مرکزی',
        flow: row.flow,
        doc_kind: row.doc,
        doc_number: row.number,
        quantity: row.quantity,
        unit_cost: row.unitCost,
        value: Math.round(row.quantity * row.unitCost),
        balance,
        note: row.note,
      }
    })

    return {
      product_id: product.id,
      product_name: product.name,
      product_unit: product.unit,
      kind,
      opening_balance: 0,
      total_in: totalIn,
      total_out: totalOut,
      closing_balance: balance,
      entries,
    }
  }
  }
}
