import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  getDashboardKpis,
  getFiscalPeriodStatus,
  getLowStockReport,
  getPartyAging,
  getProfitLoss,
  getPurchaseReport,
  getRecentInvoices,
  getSalesReport,
  getTopProducts,
  DashboardKpi,
  LowStock,
  PartyAging,
  PartyBalance,
  ProfitLoss,
  PeriodStatus,
  RecentInvoice,
  SalesReportRow,
  TopProduct,
} from '../api'
import { errorText } from '../lib/errors'
import { previousRange, resolveRange, type JalaliRange } from '../lib/dateRange'

/**
 * لایه‌ی داده‌ی داشبورد.
 *
 * ## چرا از فایل صفحه جدا شد
 * صفحه‌ی داشبورد وظیفه‌اش «نمایش» است. جمع‌بندی دوره، مقایسه با دوره‌ی
 * قبل، گروه‌بندی روند و ساخت فهرست طرف حساب‌های برتر، منطق است نه نمایش.
 * جدا کردنشان هم فایل صفحه را کوچک نگه می‌دارد و هم این منطق را مستقیماً
 * قابل تست می‌کند.
 *
 * ## قاعده‌ی «دوره» در برابر «اکنون»
 * در حسابداری، اقلام **صورت سود و زیان** (فروش، خرید، مالیات) متعلق به یک
 * دوره‌اند و با تغییر بازه عوض می‌شوند. اقلام **ترازنامه** (موجودی نقد،
 * ارزش انبار، مطالبات، بدهی‌ها) مانده‌ی «در لحظه» هستند و بازه‌ی زمانی
 * روی آن‌ها معنا ندارد. این تفکیک در کارت‌های شاخص صریح نشان داده می‌شود
 * تا کاربر عدد را اشتباه تفسیر نکند.
 */

export type PaymentFilter = 'all' | 'paid' | 'partial' | 'unpaid'

export type PeriodTotals = {
  sales: number
  purchases: number
  vat: number
  invoiceCount: number
}

export type TrendPoint = { period: string; sales: number; purchases: number }

export type DashboardState = {
  loading: boolean
  error: string
  reload: () => void

  /** جمع‌های دوره‌ی انتخاب‌شده، پس از اعمال فیلترها. */
  period: PeriodTotals
  /** همان جمع‌ها برای دوره‌ی قبلی هم‌طول — مبنای درصد تغییر. */
  previous: PeriodTotals

  /** مانده‌های «در لحظه». */
  balances?: DashboardKpi
  profit?: ProfitLoss

  trend: TrendPoint[]
  receivableAging: PartyAging[]
  payableAging: PartyAging[]
  topCustomers: PartyBalance[]
  topSuppliers: PartyBalance[]
  topProducts: TopProduct[]
  lowStock: LowStock[]
  recent: RecentInvoice[]

  /** تعداد فاکتور فروش و خرید یافت‌شده پس از فیلتر — برای نوار فیلتر. */
  matchCount: number
}

const EMPTY_TOTALS: PeriodTotals = { sales: 0, purchases: 0, vat: 0, invoiceCount: 0 }

/** آیا سطر گزارش از فیلترهای انتخابی رد می‌شود؟ */
function passes(row: SalesReportRow, payment: PaymentFilter, search: string): boolean {
  if (payment !== 'all' && row.payment_status !== payment) return false
  if (search && !(row.contact_name ?? '').includes(search)) return false
  return true
}

function totalsOf(sales: SalesReportRow[], purchases: SalesReportRow[]): PeriodTotals {
  return {
    sales: sales.reduce((sum, row) => sum + row.total, 0),
    purchases: purchases.reduce((sum, row) => sum + row.total, 0),
    vat: sales.reduce((sum, row) => sum + row.tax, 0),
    invoiceCount: sales.length,
  }
}

/**
 * گروه‌بندی روند: بازه‌های کوتاه روزانه، بازه‌های بلند ماهانه.
 *
 * نمودار ۳۶۵ نقطه‌ای خوانا نیست و نمودار ۲ نقطه‌ای بی‌فایده. مرز ۴۵ روز
 * انتخاب شده تا «این ماه» روزانه و «امسال» ماهانه دیده شود.
 */
function buildTrend(
  sales: SalesReportRow[],
  purchases: SalesReportRow[],
  daily: boolean,
): TrendPoint[] {
  const key = (date: string) => (daily ? date : date.slice(0, 7))
  const map = new Map<string, TrendPoint>()
  const add = (rows: SalesReportRow[], field: 'sales' | 'purchases') => {
    for (const row of rows) {
      const bucket = key(row.date)
      const entry = map.get(bucket) ?? { period: bucket, sales: 0, purchases: 0 }
      entry[field] += row.total
      map.set(bucket, entry)
    }
  }
  add(sales, 'sales')
  add(purchases, 'purchases')
  return [...map.values()].sort((a, b) => a.period.localeCompare(b.period))
}

/** فهرست طرف حساب‌های برتر از روی سطرهای گزارش همان دوره. */
function topParties(rows: SalesReportRow[]): PartyBalance[] {
  const map = new Map<string, PartyBalance>()
  for (const row of rows) {
    const name = row.contact_name ?? 'بدون طرف حساب'
    const entry = map.get(name) ?? {
      contact_id: name,
      contact_name: name,
      invoice_count: 0,
      invoiced: 0,
      settled: 0,
      remaining: 0,
    }
    entry.invoice_count += 1
    entry.invoiced += row.total
    if (row.payment_status === 'paid') entry.settled += row.total
    entry.remaining = entry.invoiced - entry.settled
    map.set(name, entry)
  }
  return [...map.values()]
}

export function useDashboardData(
  range: JalaliRange,
  payment: PaymentFilter,
  search: string,
  enabled: boolean,
): DashboardState {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [salesRows, setSalesRows] = useState<SalesReportRow[]>([])
  const [purchaseRows, setPurchaseRows] = useState<SalesReportRow[]>([])
  const [prevSales, setPrevSales] = useState<SalesReportRow[]>([])
  const [prevPurchases, setPrevPurchases] = useState<SalesReportRow[]>([])
  const [balances, setBalances] = useState<DashboardKpi>()
  const [profit, setProfit] = useState<ProfitLoss>()
  const [receivableAging, setReceivableAging] = useState<PartyAging[]>([])
  const [payableAging, setPayableAging] = useState<PartyAging[]>([])
  const [topProducts, setTopProducts] = useState<TopProduct[]>([])
  const [lowStock, setLowStock] = useState<LowStock[]>([])
  const [recent, setRecent] = useState<RecentInvoice[]>([])

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    const back = previousRange(range)
    try {
      const [
        currentSales,
        currentPurchases,
        earlierSales,
        earlierPurchases,
        kpis,
        profitLoss,
        receivables,
        payables,
        products,
        low,
        invoices,
      ] = await Promise.all([
        getSalesReport(range.from, range.to),
        getPurchaseReport(range.from, range.to),
        getSalesReport(back.from, back.to),
        getPurchaseReport(back.from, back.to),
        getDashboardKpis(),
        getProfitLoss(),
        getPartyAging(true, range.to),
        getPartyAging(false, range.to),
        getTopProducts(),
        getLowStockReport(),
        getRecentInvoices(),
      ])
      setSalesRows(currentSales)
      setPurchaseRows(currentPurchases)
      setPrevSales(earlierSales)
      setPrevPurchases(earlierPurchases)
      setBalances(kpis)
      setProfit(profitLoss)
      setReceivableAging(receivables)
      setPayableAging(payables)
      setTopProducts(products)
      setLowStock(low)
      setRecent(invoices)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }, [range])

  useEffect(() => {
    if (enabled) load()
    else setLoading(false)
  }, [enabled, load])

  const filteredSales = useMemo(
    () => salesRows.filter((row) => passes(row, payment, search.trim())),
    [salesRows, payment, search],
  )
  const filteredPurchases = useMemo(
    () => purchaseRows.filter((row) => passes(row, payment, search.trim())),
    [purchaseRows, payment, search],
  )
  const filteredPrevSales = useMemo(
    () => prevSales.filter((row) => passes(row, payment, search.trim())),
    [prevSales, payment, search],
  )
  const filteredPrevPurchases = useMemo(
    () => prevPurchases.filter((row) => passes(row, payment, search.trim())),
    [prevPurchases, payment, search],
  )

  const period = useMemo(
    () => totalsOf(filteredSales, filteredPurchases),
    [filteredSales, filteredPurchases],
  )
  const previous = useMemo(
    () => totalsOf(filteredPrevSales, filteredPrevPurchases),
    [filteredPrevSales, filteredPrevPurchases],
  )

  const trend = useMemo(() => {
    const daily = range.from.slice(0, 7) === range.to.slice(0, 7)
    return buildTrend(filteredSales, filteredPurchases, daily)
  }, [filteredSales, filteredPurchases, range])

  const topCustomers = useMemo(() => topParties(filteredSales), [filteredSales])
  const topSuppliers = useMemo(() => topParties(filteredPurchases), [filteredPurchases])

  return {
    loading,
    error,
    reload: load,
    period,
    previous,
    balances,
    profit,
    trend,
    receivableAging,
    payableAging,
    topCustomers,
    topSuppliers,
    topProducts,
    lowStock,
    recent,
    matchCount: filteredSales.length + filteredPurchases.length,
  }
}

/**
 * بازه‌ی پیش‌فرض داشبورد: **سال مالی فعال**.
 *
 * چرا نه «این ماه»: حسابدار ایرانی واحد کارش سال مالی است؛ داشبوردی که
 * پیش‌فرضش ماه جاری باشد، در روزهای ابتدایی ماه تقریباً خالی است و در
 * شرکتی که سال مالی‌اش با سال تقویمی یکی نیست اصلاً بی‌ربط می‌شود. تا وقتی
 * بازه‌ی واقعی از پایگاه داده برسد، سال شمسی جاری فرض می‌شود.
 */
export const defaultRange = () => resolveRange('fiscalYear')

/** خواندن سال مالی فعال — یک بار، برای مقداردهی نوار فیلتر. */
export function useFiscalPeriod(enabled: boolean): PeriodStatus | undefined {
  const [period, setPeriod] = useState<PeriodStatus>()
  useEffect(() => {
    if (!enabled) return
    let alive = true
    getFiscalPeriodStatus()
      .then((value) => {
        if (alive) setPeriod(value)
      })
      .catch(() => {
        /* نبود سال مالی نباید داشبورد را از کار بیندازد */
      })
    return () => {
      alive = false
    }
  }, [enabled])
  return period
}

export const EMPTY_PERIOD = EMPTY_TOTALS

/** درصد تغییر؛ اگر دوره‌ی قبل صفر بوده، محاسبه نمی‌شود. */
export function pctChange(current: number, previous: number): number | null {
  if (previous <= 0) return null
  return ((current - previous) / previous) * 100
}
