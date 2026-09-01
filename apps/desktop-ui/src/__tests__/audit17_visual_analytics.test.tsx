/**
 * @vitest-environment jsdom
 *
 * ممیزی دور ۱۷ — تحلیل بصری: پوشش ۱۹ نوع نمودار درخواستی روی داده‌ی واقعی.
 *
 * دو چیز سنجیده می‌شود:
 *  ۱. کیت نمودار همه‌ی انواع خواسته‌شده را دارد (قرارداد منبع).
 *  ۲. صفحه‌ی تحلیل بصری هر نوع را از منبع واقعی می‌سازد و هیچ نموداری
 *     عدد ثابت/ساختگی ندارد (نگهبان ضد-UI-ساختگی).
 */
import { describe, expect, it, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, waitFor, cleanup } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { VisualAnalytics } from '../pages/VisualAnalytics'
import { I18nProvider } from '../lib/i18n'
import {
  getDashboardKpis,
  getProfitLoss,
  getRecentInvoices,
  getReceivables,
  getSalesInvoices,
  getPurchaseInvoices,
  getTopProducts,
  getTreasurySummary,
} from '../api'

vi.mock('../api', () => ({
  getSalesInvoices: vi.fn(),
  getPurchaseInvoices: vi.fn(),
  getTopProducts: vi.fn(),
  getTreasurySummary: vi.fn(),
  getReceivables: vi.fn(),
  getRecentInvoices: vi.fn(),
  getDashboardKpis: vi.fn(),
  getProfitLoss: vi.fn(),
}))

const SRC = resolve(__dirname, '..')
const read = (path: string) => readFileSync(resolve(SRC, path), 'utf8')

const invoice = (overrides: Partial<Parameters<typeof getSalesInvoices>[0] extends never ? never : Record<string, unknown>> = {}) =>
  ({
    id: 'inv-1',
    number: 1,
    invoice_date: '1405/04/15',
    contact_id: 'c1',
    warehouse_id: 'w1',
    status: 'posted',
    payment_status: 'paid',
    subtotal: 100,
    discount: 0,
    tax: 9,
    total: 109,
    ...overrides,
  }) as never

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(getSalesInvoices).mockResolvedValue([
    invoice({ id: 'a', number: 1, payment_status: 'paid', total: 1_000_000 }),
    invoice({ id: 'b', number: 2, payment_status: 'partial', total: 2_000_000 }),
    invoice({ id: 'c', number: 3, payment_status: 'unpaid', total: 500_000 }),
  ] as never)
  vi.mocked(getPurchaseInvoices).mockResolvedValue([
    invoice({ id: 'p1', total: 800_000 }),
  ] as never)
  vi.mocked(getTopProducts).mockResolvedValue([
    { product_id: 'p1', name: 'کالای الف', quantity: 10, revenue: 5_000_000 },
    { product_id: 'p2', name: 'کالای ب', quantity: 4, revenue: 3_000_000 },
    { product_id: 'p3', name: 'کالای پ', quantity: 7, revenue: 1_200_000 },
    { product_id: 'p4', name: 'کالای ت', quantity: 2, revenue: 900_000 },
  ])
  vi.mocked(getTreasurySummary).mockResolvedValue([
    { id: 't1', name: 'صندوق مرکزی', account_type: 'cash', balance: 40_000_000, inflow: 0, outflow: 0, transaction_count: 0 },
    { id: 't2', name: 'بانک ملت', account_type: 'bank', balance: 60_000_000, inflow: 0, outflow: 0, transaction_count: 0 },
  ])
  vi.mocked(getReceivables).mockResolvedValue([
    { contact_id: 'c1', contact_name: 'مشتری نمونه', invoice_count: 2, invoiced: 3_000_000, settled: 1_000_000, remaining: 2_000_000 },
  ])
  vi.mocked(getRecentInvoices).mockResolvedValue([
    { id: 'r1', number: 3, invoice_date: '1405/04/20', contact_name: 'مشتری نمونه', total: 500_000, payment_status: 'unpaid', invoice_type: 'sales' },
  ])
  vi.mocked(getDashboardKpis).mockResolvedValue({
    sales: 10_000_000,
    purchases: 4_000_000,
    gross_profit: 3_500_000,
    receivables: 2_000_000,
    payables: 0,
    cash: 90_000_000,
    inventory_value: 0,
    low_stock_count: 0,
  })
  vi.mocked(getProfitLoss).mockResolvedValue({
    revenue: 10_000_000,
    sales_returns: 500_000,
    net_revenue: 9_500_000,
    cogs: 6_000_000,
    gross_profit: 3_500_000,
    gross_margin_percent: 37,
  })
})

afterEach(cleanup)

describe('م۱۷ · کیت نمودار — پوشش ۱۹ نوع درخواستی', () => {
  const kit = read('components/visualCharts.tsx')

  it('ن۱ — انواع دایره‌ای/دونات/ستونی/روند در کیت موجودند', () => {
    for (const name of ['CircleShare', 'BarsChart', 'LineTrend', 'AreaTrend', 'WaterfallChart']) {
      expect(kit, name).toContain(`function ${name}`)
    }
    // donut با innerRadius و pie بدون آن — هر دو از CircleShare
    expect(kit).toContain(`innerRadius={donut ? '54%' : 0}`)
  })

  it('ن۲ — پراکندگی/حباب/گیج/قیف/رادار/درختی موجودند', () => {
    for (const name of ['ScatterPlot', 'GaugeChart', 'FunnelStages', 'RadarProfile', 'TreeMapShare']) {
      expect(kit, name).toContain(`function ${name}`)
    }
    expect(kit).toContain('ZAxis')
    expect(kit).toContain('RadialBarChart')
  })

  it('ن۳ — هیستوگرام/حرارتی/خط‌زمانی/کارت KPI موجودند', () => {
    for (const name of ['HistogramChart', 'HeatGrid', 'TimelineList', 'KpiStat']) {
      expect(kit, name).toContain(`function ${name}`)
    }
  })

  it('ن۴ — کیت از تم سازمانی می‌گیرد نه رنگ سخت‌کدشده', () => {
    expect(kit).toContain('var(--chart-1)')
    expect(kit).toContain('var(--chart-4)')
    // رنگ hex سخت‌کدشده در کیت نباشد
    expect(kit).not.toMatch(/#[0-9a-fA-F]{6}/)
  })
})

describe('م۱۷ · صفحه‌ی تحلیل بصری — داده‌ی واقعی', () => {
  async function renderPage() {
    render(
      <I18nProvider initialLocale="fa">
        <VisualAnalytics />
      </I18nProvider>,
    )
    await waitFor(() => expect(screen.getByText('تحلیل بصری', { selector: 'h1'})).toBeTruthy())
    await waitFor(() =>
      expect(screen.queryByText('در حال بارگذاری…')).toBeNull(),
    )
  }

  it('ن۵ — همه‌ی منابع واقعی صدا زده می‌شوند', async () => {
    await renderPage()
    expect(getSalesInvoices).toHaveBeenCalled()
    expect(getPurchaseInvoices).toHaveBeenCalled()
    expect(getTopProducts).toHaveBeenCalled()
    expect(getTreasurySummary).toHaveBeenCalled()
    expect(getReceivables).toHaveBeenCalled()
    expect(getDashboardKpis).toHaveBeenCalled()
    expect(getProfitLoss).toHaveBeenCalled()
  })

  it('ن۶ — عنوان هر ۱۹+ نمودار رندر می‌شود', async () => {
    await renderPage()
    const titles = [
      'روند خطی', 'نمودار سطحی', 'دونات', 'دایره‌ای', 'گیج',
      'ستون گروهی', 'ستون افقی', 'ستون انباشته', 'انباشته‌ی ۱۰۰٪',
      'آبشار', 'هیستوگرام', 'پراکندگی', 'حباب', 'نقشه‌ی درختی',
      'قیف', 'رادار', 'نقشه‌ی حرارتی', 'چندخطی', 'خط زمانی',
    ]
    for (const title of titles) {
      expect(screen.getAllByText(title, { exact: false }).length, title).toBeGreaterThan(0)
    }
  })

  it('ن۷ — KPI کارت‌ها مقدار واقعی میزبان را نشان می‌دهند', async () => {
    await renderPage()
    expect(screen.getByText('فروش دوره')).toBeTruthy()
    expect(screen.getByText('۱۰٬۰۰۰٬۰۰۰')).toBeTruthy()
    expect(screen.getByText('سود ناخالص')).toBeTruthy()
  })

  it('ن۸ — هیچ داده‌ی ساختگی هاردکد در صفحه نیست', () => {
    const page = read('pages/VisualAnalytics.tsx')
    // مبالغ هاردکد به‌جز صفرهای نرمال‌ساز (۱_000_000 تقسیم) نباشد
    expect(page).not.toMatch(/total:\s*[0-9]{4,}/)
    expect(page).not.toMatch(/value:\s*[0-9]{5,}/)
  })
})

describe('م۱۷ · قرارداد مسیر و منو', () => {
  it('ن۹ — مسیر visual-analytics در مسیریاب و منو و عنوان صفحه هست', () => {
    const app = read('App.tsx')
    expect(app).toContain("'visual-analytics'")
    expect(app).toContain("case 'visual-analytics':")
    for (const file of ['fa.ts', 'en.ts', 'ar.ts']) {
      const dict = readFileSync(resolve(SRC, 'lib/i18n', file), 'utf8')
      expect(dict, file).toContain("'page.visual-analytics'")
      expect(dict, file).toContain("'visual.chart.donutTitle'")
    }
  })
})
