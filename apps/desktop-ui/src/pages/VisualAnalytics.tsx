import { useEffect, useMemo, useState } from 'react'
import { Banknote, ShoppingCart, TrendingUp, Wallet } from 'lucide-react'
import {
  getDashboardKpis,
  getProfitLoss,
  getRecentInvoices,
  getReceivables,
  getSalesInvoices,
  getPurchaseInvoices,
  getTopProducts,
  getTreasurySummary,
  type InvoiceSummary,
  type RecentInvoice,
  type TopProduct,
  type TreasurySummary,
} from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money, rialUnit } from '../lib/format'
import { useI18n } from '../lib/i18n'
import { ErrorState, Skeleton } from '../components/ui'
import {
  AreaTrend,
  BarsChart,
  ChartFrame,
  CircleShare,
  FunnelStages,
  GaugeChart,
  HeatGrid,
  HistogramChart,
  KpiStat,
  LineTrend,
  RadarProfile,
  ScatterPlot,
  TimelineList,
  TreeMapShare,
  WaterfallChart,
} from '../components/visualCharts'

/**
 * تحلیل بصری — گالری نمودارهای حسابداری روی داده‌ی واقعی.
 *
 * مرجع: درخواست پوشش کامل انواع نمودار (Pie/Donut/Bar/…/Histogram).
 *
 * ## چرا این صفحه نمودار ساختگی ندارد
 *
 * هر نمودار از یک منبع واقعی می‌آید: فاکتورهای فروش/خرید (توزیع مبلغ،
 * وضعیت تسویه، روند ماهانه)، محصولات برتر (سهم و پراکندگی)، خزانه
 * (سهم حساب‌ها)، مطالبات (سن بدهی و نرخ وصول)، سود و زیان (آبشار) و
 * فاکتورهای اخیر (خط زمانی). اگر داده‌ای نبود، نمودار پیام خالی می‌دهد.
 */

type MonthBucket = {
  month: string
  sales: number
  purchases: number
  paid: number
  partial: number
  unpaid: number
  count: number
}

const monthLabel = (iso: string) => iso.slice(0, 7)

function bucketMonths(sales: InvoiceSummary[], purchases: InvoiceSummary[]): MonthBucket[] {
  const map = new Map<string, MonthBucket>()
  const touch = (key: string) =>
    map.get(key) ?? { month: key, sales: 0, purchases: 0, paid: 0, partial: 0, unpaid: 0, count: 0 }
  for (const invoice of sales) {
    if (invoice.status !== 'posted') continue
    const key = monthLabel(invoice.invoice_date)
    const row = { ...touch(key) }
    row.sales += invoice.total
    row.count += 1
    if (invoice.payment_status === 'paid') row.paid += invoice.total
    else if (invoice.payment_status === 'partial') row.partial += invoice.total
    else row.unpaid += invoice.total
    map.set(key, row)
  }
  for (const invoice of purchases) {
    if (invoice.status !== 'posted') continue
    const row = { ...touch(monthLabel(invoice.invoice_date)) }
    row.purchases += invoice.total
    map.set(row.month, row)
  }
  return [...map.values()].sort((a, b) => (a.month < b.month ? -1 : 1)).slice(-12)
}

export function VisualAnalytics() {
  const { t } = useI18n()
  const [sales, setSales] = useState<InvoiceSummary[]>([])
  const [purchases, setPurchases] = useState<InvoiceSummary[]>([])
  const [topProducts, setTopProducts] = useState<TopProduct[]>([])
  const [treasury, setTreasury] = useState<TreasurySummary[]>([])
  const [receivables, setReceivables] = useState<{ contact_name: string; invoiced: number; settled: number; remaining: number }[]>([])
  const [recent, setRecent] = useState<RecentInvoice[]>([])
  const [kpis, setKpis] = useState<{ sales: number; purchases: number; gross_profit: number; receivables: number; cash: number }>()
  const [profitLoss, setProfitLoss] = useState<{ revenue: number; sales_returns: number; net_revenue: number; cogs: number; gross_profit: number }>()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    let alive = true
    Promise.all([
      getSalesInvoices(),
      getPurchaseInvoices(),
      getTopProducts(),
      getTreasurySummary(),
      getReceivables(),
      getRecentInvoices(),
      getDashboardKpis(),
      getProfitLoss(),
    ])
      .then(([s, p, tp, tr, rc, ri, kp, pl]) => {
        if (!alive) return
        setSales(s)
        setPurchases(p)
        setTopProducts(tp)
        setTreasury(tr)
        setReceivables(rc)
        setRecent(ri)
        setKpis(kp)
        setProfitLoss(pl)
      })
      .catch((e) => alive && setError(errorText(e)))
      .finally(() => alive && setLoading(false))
    return () => {
      alive = false
    }
  }, [])

  const months = useMemo(() => bucketMonths(sales, purchases), [sales, purchases])

  /* ── KPI Cards ── */
  const kpiCards = useMemo(
    () => [
      { label: t('visual.kpi.sales'), value: kpis ? money(kpis.sales) : '—', icon: <TrendingUp className="size-4.5" aria-hidden />, tone: 'brand' as const },
      { label: t('visual.kpi.purchases'), value: kpis ? money(kpis.purchases) : '—', icon: <ShoppingCart className="size-4.5" aria-hidden />, tone: 'gold' as const },
      { label: t('visual.kpi.profit'), value: kpis ? money(kpis.gross_profit) : '—', icon: <Banknote className="size-4.5" aria-hidden />, tone: 'green' as const },
      { label: t('visual.kpi.receivables'), value: kpis ? money(kpis.receivables) : '—', icon: <Wallet className="size-4.5" aria-hidden />, tone: 'red' as const },
    ],
    [kpis, t],
  )

  /* ── Donut: وضعیت تسویه‌ی فروش ── */
  const settlement = useMemo(() => {
    const acc = { paid: 0, partial: 0, unpaid: 0 }
    for (const invoice of sales) {
      if (invoice.status !== 'posted') continue
      if (invoice.payment_status === 'paid') acc.paid += invoice.total
      else if (invoice.payment_status === 'partial') acc.partial += invoice.total
      else acc.unpaid += invoice.total
    }
    return [
      { name: t('visual.status.paid'), value: acc.paid },
      { name: t('visual.status.partial'), value: acc.partial },
      { name: t('visual.status.unpaid'), value: acc.unpaid },
    ].filter((row) => row.value > 0)
  }, [sales, t])

  /* ── Pie: سهم حساب‌های خزانه ── */
  const treasuryShare = useMemo(
    () =>
      treasury
        .filter((account) => account.balance > 0)
        .slice(0, 6)
        .map((account) => ({ name: account.name, value: Math.abs(account.balance) })),
    [treasury],
  )

  /* ── Bar: فروش در برابر خرید ماهانه ── */
  const monthlyCompare = useMemo(
    () => months.map((row) => ({ month: row.month, [t('visual.series.sales')]: row.sales, [t('visual.series.purchases')]: row.purchases })),
    [months, t],
  )

  /* ── Horizontal Bar: بدهکاران برتر ── */
  const topDebtors = useMemo(
    () =>
      [...receivables]
        .sort((a, b) => b.remaining - a.remaining)
        .slice(0, 7)
        .map((row) => ({ name: row.contact_name, [t('visual.series.remaining')]: row.remaining })),
    [receivables, t],
  )

  /* ── Stacked Bar: مانده مطالبات به تفکیک وضعیت تسویه در ماه‌ها ── */
  const stackedMonths = useMemo(
    () =>
      months.map((row) => ({
        month: row.month,
        [t('visual.status.paid')]: row.paid,
        [t('visual.status.partial')]: row.partial,
        [t('visual.status.unpaid')]: row.unpaid,
      })),
    [months, t],
  )

  /* ── Stacked 100%: سهم درصدی وضعیت تسویه در ماه ── */
  const percentMonths = useMemo(
    () =>
      months.map((row) => {
        const total = row.paid + row.partial + row.unpaid || 1
        return {
          month: row.month,
          [t('visual.status.paid')]: Math.round((row.paid / total) * 100),
          [t('visual.status.partial')]: Math.round((row.partial / total) * 100),
          [t('visual.status.unpaid')]: Math.round((row.unpaid / total) * 100),
        }
      }),
    [months, t],
  )

  /* ── Area: فروش تجمعی ── */
  const cumulative = useMemo(() => {
    let running = 0
    return months.map((row) => {
      running += row.sales
      return { month: row.month, [t('visual.series.cumulative')]: running }
    })
  }, [months, t])

  /* ── Multi-Line: فروش/خرید/خالص ── */
  const multiLine = useMemo(
    () =>
      months.map((row) => ({
        month: row.month,
        [t('visual.series.sales')]: row.sales,
        [t('visual.series.purchases')]: row.purchases,
        [t('visual.series.net')]: row.sales - row.purchases,
      })),
    [months, t],
  )

  /* ── Scatter & Bubble: محصولات (تعداد در برابر درآمد) ── */
  const productPoints = useMemo(
    () =>
      topProducts.slice(0, 14).map((product) => ({
        name: product.name,
        x: product.quantity,
        y: product.revenue,
        z: Math.max(1, product.revenue),
      })),
    [topProducts],
  )

  /* ── Gauge: نرخ وصول مطالبات ── */
  const collectionGauge = useMemo(() => {
    const invoiced = receivables.reduce((sum, row) => sum + row.invoiced, 0)
    const settled = receivables.reduce((sum, row) => sum + row.settled, 0)
    if (invoiced <= 0) return 0
    return Math.min(100, (settled / invoiced) * 100)
  }, [receivables])

  /* ── Funnel: چرخه‌ی عمر فاکتور فروش ── */
  const funnel = useMemo(() => {
    const posted = sales.filter((invoice) => invoice.status === 'posted')
    const stages = [
      { name: t('visual.funnel.issued'), value: sales.length },
      { name: t('visual.funnel.posted'), value: posted.length },
      { name: t('visual.funnel.settled'), value: posted.filter((invoice) => invoice.payment_status === 'paid').length },
    ]
    return stages.filter((stage) => stage.value > 0)
  }, [sales, t])

  /* ── Heatmap: شدت فروش ماه × وضعیت تسویه ── */
  const heatRows = useMemo(() => months.map((row) => row.month), [months])
  const heatCell = useMemo(() => {
    const lookup = new Map(months.map((row) => [row.month, row]))
    return (row: string, col: string) => {
      const bucket = lookup.get(row)
      if (!bucket) return 0
      if (col === 'paid') return bucket.paid
      if (col === 'partial') return bucket.partial
      return bucket.unpaid
    }
  }, [months])

  /* ── Treemap: سهم محصولات از درآمد ── */
  const productTree = useMemo(
    () => topProducts.slice(0, 12).map((product) => ({ name: product.name, size: Math.max(1, product.revenue) })),
    [topProducts],
  )

  /* ── Waterfall: از فروش تا سود ناخالص ── */
  const waterfall = useMemo(() => {
    if (!profitLoss) return []
    const pl = profitLoss
    return [
      { name: t('visual.waterfall.revenue'), base: 0, delta: pl.revenue, tone: 'up' as const },
      { name: t('visual.waterfall.returns'), base: pl.revenue - pl.sales_returns, delta: pl.sales_returns, tone: 'down' as const },
      { name: t('visual.waterfall.net'), base: 0, delta: pl.net_revenue, tone: 'total' as const },
      { name: t('visual.waterfall.cogs'), base: pl.net_revenue - pl.cogs, delta: pl.cogs, tone: 'down' as const },
      { name: t('visual.waterfall.gross'), base: 0, delta: pl.gross_profit, tone: 'total' as const },
    ]
  }, [profitLoss, t])

  /* ── Radar: پنج محصول برتر (درآمد نرمال‌شده × تعداد نرمال‌شده) ── */
  const radar = useMemo(() => {
    const top = topProducts.slice(0, 5)
    if (top.length < 3) return []
    const maxRevenue = Math.max(...top.map((p) => p.revenue), 1)
    const maxQuantity = Math.max(...top.map((p) => p.quantity), 1)
    return top.map((product) => ({
      product: product.name.length > 14 ? `${product.name.slice(0, 13)}…` : product.name,
      [t('visual.radar.revenue')]: Math.round((product.revenue / maxRevenue) * 100),
      [t('visual.radar.quantity')]: Math.round((product.quantity / maxQuantity) * 100),
    }))
  }, [topProducts, t])

  /* ── Timeline: فاکتورهای اخیر ── */
  const timeline = useMemo(
    () =>
      recent.slice(0, 8).map((invoice) => ({
        date: invoice.invoice_date,
        title: `${t('visual.timeline.invoice')} ${formatNumber(invoice.number)} — ${money(invoice.total)} ${rialUnit()}`,
        meta: invoice.contact_name ?? '—',
        tone: (invoice.invoice_type === 'sales' ? 'up' : 'down') as 'up' | 'down',
      })),
    [recent, t],
  )

  /* ── Histogram: توزیع مبلغ فاکتورهای فروش ── */
  const histogram = useMemo(() => {
    const posted = sales.filter((invoice) => invoice.status === 'posted')
    if (posted.length === 0) return []
    const totals = posted.map((invoice) => invoice.total).sort((a, b) => a - b)
    const min = totals[0]
    const max = totals[totals.length - 1]
    if (max <= min) return [{ name: money(min), count: totals.length }]
    const bins = 7
    const width = (max - min) / bins
    return Array.from({ length: bins }, (_, index) => {
      const from = min + index * width
      const to = from + width
      const count = totals.filter((value) => (index === bins - 1 ? value >= from && value <= to : value >= from && value < to)).length
      return { name: money(Math.round(from / 1_000_000)), count }
    })
  }, [sales])

  if (error) return <ErrorState onRetry={() => location.reload()} />

  const seriesSales = [{ key: t('visual.series.sales'), name: t('visual.series.sales') }]
  const seriesBoth = [
    { key: t('visual.series.sales'), name: t('visual.series.sales') },
    { key: t('visual.series.purchases'), name: t('visual.series.purchases') },
  ]
  const seriesStatus = [
    { key: t('visual.status.paid'), name: t('visual.status.paid') },
    { key: t('visual.status.partial'), name: t('visual.status.partial') },
    { key: t('visual.status.unpaid'), name: t('visual.status.unpaid') },
  ]

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('nav.group.reports')}</div>
          <h1>{t('page.visual-analytics')}</h1>
          <p>{t('visual.subtitle')}</p>
        </div>
      </div>

      {loading ? (
        <div className="grid grid-cols-12 gap-4">
          {Array.from({ length: 6 }, (_, index) => (
            <Skeleton key={index} className="col-span-12 md:col-span-6 xl:col-span-4 h-64" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-12 gap-4">
          {/* ───────── KPI Cards ───────── */}
          {kpiCards.map((card) => (
            <KpiStat key={card.label} label={card.label} value={card.value} tone={card.tone} icon={card.icon} />
          ))}

          {/* ───────── روند و ترکیب ───────── */}
          <ChartFrame title={t('visual.chart.lineTitle')} subtitle={t('visual.chart.lineSub')} span="col-span-12 md:col-span-6">
            {monthlyCompare.length > 0 ? (
              <LineTrend data={monthlyCompare} x="month" series={seriesBoth} />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.areaTitle')} subtitle={t('visual.chart.areaSub')} span="col-span-12 md:col-span-6">
            {cumulative.length > 0 ? (
              <AreaTrend data={cumulative} x="month" series={[{ key: t('visual.series.cumulative'), name: t('visual.series.cumulative') }]} />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.donutTitle')} subtitle={t('visual.chart.donutSub')}>
            {settlement.length > 0 ? <CircleShare data={settlement} donut /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.pieTitle')} subtitle={t('visual.chart.pieSub')}>
            {treasuryShare.length > 0 ? <CircleShare data={treasuryShare} /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.gaugeTitle')} subtitle={t('visual.chart.gaugeSub')}>
            <GaugeChart value={collectionGauge} label={t('visual.gauge.label')} />
          </ChartFrame>

          {/* ───────── ستون‌ها ───────── */}
          <ChartFrame title={t('visual.chart.barTitle')} subtitle={t('visual.chart.barSub')} span="col-span-12 md:col-span-6">
            {monthlyCompare.length > 0 ? (
              <BarsChart data={monthlyCompare} x="month" series={seriesBoth} />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.hbarTitle')} subtitle={t('visual.chart.hbarSub')}>
            {topDebtors.length > 0 ? (
              <BarsChart data={topDebtors} x="name" series={[{ key: t('visual.series.remaining'), name: t('visual.series.remaining') }]} horizontal />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.stackedTitle')} subtitle={t('visual.chart.stackedSub')}>
            {stackedMonths.length > 0 ? (
              <BarsChart data={stackedMonths} x="month" series={seriesStatus} stacked />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.stacked100Title')} subtitle={t('visual.chart.stacked100Sub')}>
            {percentMonths.length > 0 ? (
              <BarsChart data={percentMonths} x="month" series={seriesStatus} stacked percent />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.waterfallTitle')} subtitle={t('visual.chart.waterfallSub')} height={280}>
            {waterfall.length > 0 ? <WaterfallChart data={waterfall} x="name" /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.histogramTitle')} subtitle={t('visual.chart.histogramSub')}>
            {histogram.length > 0 ? <HistogramChart data={histogram} /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          {/* ───────── پراکندگی و سهم ───────── */}
          <ChartFrame title={t('visual.chart.scatterTitle')} subtitle={t('visual.chart.scatterSub')}>
            {productPoints.length > 1 ? (
              <ScatterPlot data={productPoints} xName={t('visual.axis.quantity')} yName={t('visual.axis.revenue')} />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.bubbleTitle')} subtitle={t('visual.chart.bubbleSub')}>
            {productPoints.length > 1 ? (
              <ScatterPlot data={productPoints} xName={t('visual.axis.quantity')} yName={t('visual.axis.revenue')} bubble />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.treemapTitle')} subtitle={t('visual.chart.treemapSub')} height={280}>
            {productTree.length > 0 ? <TreeMapShare data={productTree} /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.funnelTitle')} subtitle={t('visual.chart.funnelSub')}>
            {funnel.length > 1 ? <FunnelStages data={funnel} /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.radarTitle')} subtitle={t('visual.chart.radarSub')}>
            {radar.length > 2 ? (
              <RadarProfile
                data={radar}
                axes="product"
                series={[
                  { key: t('visual.radar.revenue'), name: t('visual.radar.revenue') },
                  { key: t('visual.radar.quantity'), name: t('visual.radar.quantity') },
                ]}
              />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          {/* ───────── حرارتی، چندخطی، خط زمانی ───────── */}
          <ChartFrame title={t('visual.chart.heatTitle')} subtitle={t('visual.chart.heatSub')} span="col-span-12 xl:col-span-6" height={300}>
            {heatRows.length > 0 ? (
              <HeatGrid rows={heatRows} cols={['paid', 'partial', 'unpaid']} cell={heatCell} rowKey="month" />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.multiTitle')} subtitle={t('visual.chart.multiSub')} span="col-span-12 xl:col-span-6">
            {multiLine.length > 0 ? (
              <LineTrend
                data={multiLine}
                x="month"
                series={[
                  ...seriesBoth,
                  { key: t('visual.series.net'), name: t('visual.series.net') },
                ]}
              />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>

          <ChartFrame title={t('visual.chart.timelineTitle')} subtitle={t('visual.chart.timelineSub')} span="col-span-12 xl:col-span-4" height={320}>
            {timeline.length > 0 ? <TimelineList items={timeline} /> : <p className="empty-state">{t('visual.empty')}</p>}
          </ChartFrame>

          {/* فروش سری ساده برای اسپارک پایانی */}
          <ChartFrame title={t('visual.chart.salesOnlyTitle')} subtitle={t('visual.chart.salesOnlySub')} span="col-span-12 md:col-span-6 xl:col-span-4">
            {monthlyCompare.length > 0 ? (
              <AreaTrend data={monthlyCompare} x="month" series={seriesSales} />
            ) : (
              <p className="empty-state">{t('visual.empty')}</p>
            )}
          </ChartFrame>
        </div>
      )}
    </section>
  )
}
