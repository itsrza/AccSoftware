import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip as ChartTooltip,
  XAxis,
  YAxis,
} from 'recharts'
import {
  AlertCircle,
  Boxes,
  Clock4,
  CreditCard,
  FileText,
  HandCoins,
  Package,
  TrendingUp,
  Wallet,
  type LucideIcon,
} from 'lucide-react'
import {
  getDashboardKpis,
  getLowStockReport,
  getRecentInvoices,
  getSalesTrend,
  getTopProducts,
  DashboardKpi,
  LowStock,
  RecentInvoice,
  SalesTrend,
  TopProduct,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money, formatNumber } from '../lib/format'
import { Badge, Card, CardHeader, EmptyState, ErrorState, Skeleton, TrendChip } from '../components/ui'

/**
 * داشبورد — بازطراحی‌شده بر اساس سیستم طراحی مرجع.
 *
 * ## قاعده‌ای که رعایت شد
 *
 * هیچ عدد یا نموداری ساختگی نیست. همه‌ی شاخص‌ها از فرمان‌های واقعی موتور
 * می‌آیند؛ اگر داده‌ای نباشد، «حالت خالی» نمایش داده می‌شود نه نمودار تزئینی.
 *
 * درصد تغییر هم فقط وقتی نشان داده می‌شود که **دوره‌ی قبلی داده داشته باشد**.
 * نمایش «۱۰۰٪+» برای دوره‌ای که قبلش صفر بوده، گمراه‌کننده است.
 */

type KpiDef = {
  key: string
  label: string
  icon: LucideIcon
  tone: string
  unit?: string
  /** برای مطالبات و بدهی‌ها، رشد خبر بدی است. */
  invert?: boolean
  value: (kpi: DashboardKpi) => number
}

const KPI_DEFS: KpiDef[] = [
  { key: 'sales', label: 'فروش دوره', icon: TrendingUp, tone: 'var(--chart-1)', unit: 'ریال', value: (k) => k.sales },
  { key: 'profit', label: 'سود ناخالص', icon: HandCoins, tone: 'var(--chart-4)', unit: 'ریال', value: (k) => k.gross_profit },
  { key: 'cash', label: 'موجودی نقد و بانک', icon: Wallet, tone: 'var(--chart-5)', unit: 'ریال', value: (k) => k.cash },
  { key: 'purchases', label: 'خرید دوره', icon: CreditCard, tone: 'var(--chart-6)', unit: 'ریال', value: (k) => k.purchases },
  { key: 'receivables', label: 'مطالبات', icon: Clock4, tone: 'var(--chart-2)', unit: 'ریال', invert: true, value: (k) => k.receivables },
  { key: 'payables', label: 'بدهی‌ها', icon: AlertCircle, tone: 'var(--chart-3)', unit: 'ریال', invert: true, value: (k) => k.payables },
  { key: 'inventory', label: 'ارزش موجودی', icon: Package, tone: 'var(--chart-2)', unit: 'ریال', value: (k) => k.inventory_value },
  { key: 'low', label: 'کالای کم‌موجود', icon: Boxes, tone: 'var(--chart-3)', unit: 'قلم', invert: true, value: (k) => k.low_stock_count },
]

function Sparkline({ data, tone }: { data: number[]; tone: string }) {
  const points = useMemo(() => data.map((value, index) => ({ index, value })), [data])
  const id = useMemo(() => `spark-${Math.random().toString(36).slice(2, 8)}`, [])
  if (points.length < 3) return null
  return (
    <div className="h-10 w-24 shrink-0" aria-hidden>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={points} margin={{ top: 2, bottom: 2, left: 0, right: 0 }}>
          <defs>
            <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={tone} stopOpacity={0.32} />
              <stop offset="100%" stopColor={tone} stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="value"
            stroke={tone}
            strokeWidth={1.8}
            fill={`url(#${id})`}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}

function KpiCard({
  def,
  kpi,
  spark,
  change,
  loading,
}: {
  def: KpiDef
  kpi?: DashboardKpi
  spark: number[]
  change: number | null
  loading: boolean
}) {
  const Icon = def.icon
  const value = kpi ? def.value(kpi) : 0
  return (
    <article
      data-card
      className="group relative overflow-hidden rounded-[var(--radius)] border border-border bg-card p-4 shadow-[var(--shadow-sm)] transition-all duration-300 hover:-translate-y-0.5 hover:shadow-[var(--shadow-md)]"
    >
      <div
        className="pointer-events-none absolute -end-8 -top-8 size-24 rounded-full opacity-[0.07] blur-2xl transition-opacity group-hover:opacity-[0.14]"
        style={{ background: def.tone }}
        aria-hidden
      />
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="flex items-center gap-2 text-[11.5px] font-semibold text-muted">
            <span
              className="grid size-7 shrink-0 place-items-center rounded-lg"
              style={{
                background: `color-mix(in srgb, ${def.tone} 13%, transparent)`,
                color: def.tone,
              }}
            >
              <Icon className="size-3.5" aria-hidden />
            </span>
            {def.label}
          </p>
          {loading ? (
            <Skeleton className="mt-2.5 h-6 w-32" />
          ) : (
            <p className="tnum mt-2 truncate text-[19px] font-extrabold tracking-tight text-text sm:text-xl">
              {def.unit === 'ریال' ? money(value) : formatNumber(value)}
              {def.unit && (
                <span className="ms-1.5 text-[10.5px] font-semibold text-faint">{def.unit}</span>
              )}
            </p>
          )}
        </div>
        {!loading && <Sparkline data={spark} tone={def.tone} />}
      </div>
      <div className="mt-2.5">
        {loading ? (
          <Skeleton className="h-4 w-24" />
        ) : (
          <TrendChip value={change} invert={def.invert} />
        )}
      </div>
    </article>
  )
}

const AXIS = { stroke: 'transparent', tickLine: false, axisLine: false } as const

function chartTooltipStyle() {
  return {
    contentStyle: {
      background: 'var(--card)',
      border: '1px solid var(--border)',
      borderRadius: 14,
      boxShadow: 'var(--shadow-md)',
      fontSize: 12,
      fontFamily: 'inherit',
      direction: 'rtl' as const,
    },
    labelStyle: { color: 'var(--muted)', fontSize: 11, marginBottom: 4 },
    itemStyle: { color: 'var(--text)' },
  }
}

export function Dashboard({ demo }: { demo: boolean }) {
  const [kpi, setKpi] = useState<DashboardKpi>()
  const [trend, setTrend] = useState<SalesTrend[]>([])
  const [products, setProducts] = useState<TopProduct[]>([])
  const [low, setLow] = useState<LowStock[]>([])
  const [recent, setRecent] = useState<RecentInvoice[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    try {
      const [kpis, salesTrend, top, lowStock, invoices] = await Promise.all([
        getDashboardKpis(),
        getSalesTrend(),
        getTopProducts(),
        getLowStockReport(),
        getRecentInvoices(),
      ])
      setKpi(kpis)
      setTrend(salesTrend)
      setProducts(top)
      setLow(lowStock)
      setRecent(invoices)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (demo) load()
    else setLoading(false)
  }, [demo, load])

  /**
   * درصد تغییر فروش نسبت به دوره‌ی قبل.
   *
   * اگر دوره‌ی قبل صفر بوده، درصد محاسبه نمی‌شود — «بی‌نهایت درصد رشد»
   * عددی است که فقط گمراه می‌کند.
   */
  const salesChange = useMemo(() => {
    if (trend.length < 2) return null
    const current = trend[trend.length - 1].sales
    const previous = trend[trend.length - 2].sales
    if (previous <= 0) return null
    return ((current - previous) / previous) * 100
  }, [trend])

  const purchaseChange = useMemo(() => {
    if (trend.length < 2) return null
    const current = trend[trend.length - 1].purchases
    const previous = trend[trend.length - 2].purchases
    if (previous <= 0) return null
    return ((current - previous) / previous) * 100
  }, [trend])

  const sparkFor = (key: string) => {
    if (key === 'sales' || key === 'profit') return trend.map((row) => row.sales)
    if (key === 'purchases') return trend.map((row) => row.purchases)
    if (key === 'inventory') return products.map((row) => row.revenue)
    return []
  }

  const changeFor = (key: string) => {
    if (key === 'sales' || key === 'profit') return salesChange
    if (key === 'purchases') return purchaseChange
    return null
  }

  const chartData = useMemo(
    () =>
      trend.map((row) => ({
        period: row.period,
        فروش: row.sales,
        خرید: row.purchases,
      })),
    [trend],
  )

  const productShare = useMemo(
    () =>
      products.slice(0, 6).map((product, index) => ({
        name: product.name,
        value: product.revenue,
        fill: `var(--chart-${(index % 6) + 1})`,
      })),
    [products],
  )

  if (!demo && !loading) {
    return (
      <section className="page">
        <div className="page-head">
          <div>
            <div className="eyebrow">نمای کلی</div>
            <h1>داشبورد</h1>
            <p>پس از ثبت اولین فاکتور و سند، شاخص‌های واقعی کسب‌وکار اینجا نمایش داده می‌شوند.</p>
          </div>
        </div>
        <EmptyState
          title="هنوز داده‌ای برای نمایش نیست"
          hint="از منوی فروش، اولین فاکتور را صادر کنید یا داده‌ی نمونه را فعال کنید."
        />
      </section>
    )
  }

  return (
    <section className="page space-y-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">نمای کلی</div>
          <h1>داشبورد</h1>
          <p>
            همه‌ی اعداد از دفاتر واقعی خوانده می‌شوند. درصد تغییر فقط وقتی نشان داده می‌شود که
            دوره‌ی قبل داده داشته باشد.
          </p>
        </div>
      </div>

      {error && (
        <ErrorState
          onRetry={() => {
            load()
          }}
        />
      )}

      <section
        aria-label="شاخص‌های کلیدی"
        className="fade-up grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4"
      >
        {KPI_DEFS.map((def) => (
          <KpiCard
            key={def.key}
            def={def}
            kpi={kpi}
            spark={sparkFor(def.key)}
            change={changeFor(def.key)}
            loading={loading}
          />
        ))}
      </section>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <Card className="xl:col-span-2">
          <CardHeader
            title="روند فروش و خرید"
            subtitle="مقایسه‌ی دوره‌به‌دوره بر اساس فاکتورهای ثبت‌شده"
          />
          {loading ? (
            <Skeleton className="h-72 w-full" />
          ) : chartData.length === 0 ? (
            <EmptyState title="فاکتوری در این دوره ثبت نشده است." hint="پس از صدور فاکتور، روند اینجا رسم می‌شود." />
          ) : (
            <div className="h-72 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={chartData} margin={{ top: 8, right: 8, left: 8, bottom: 0 }}>
                  <defs>
                    <linearGradient id="sales-fill" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="var(--chart-1)" stopOpacity={0.28} />
                      <stop offset="100%" stopColor="var(--chart-1)" stopOpacity={0.02} />
                    </linearGradient>
                    <linearGradient id="purchase-fill" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="var(--chart-4)" stopOpacity={0.24} />
                      <stop offset="100%" stopColor="var(--chart-4)" stopOpacity={0.02} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={false} />
                  <XAxis dataKey="period" {...AXIS} reversed />
                  <YAxis
                    {...AXIS}
                    width={70}
                    tickFormatter={(value: number) => money(value / 1_000_000)}
                  />
                  <ChartTooltip
                    {...chartTooltipStyle()}
                    formatter={(value) => `${money(Number(value ?? 0))} ریال`}
                  />
                  <Legend wrapperStyle={{ fontSize: 11, paddingTop: 8 }} />
                  <Area
                    type="monotone"
                    dataKey="فروش"
                    stroke="var(--chart-1)"
                    strokeWidth={2.2}
                    fill="url(#sales-fill)"
                  />
                  <Area
                    type="monotone"
                    dataKey="خرید"
                    stroke="var(--chart-4)"
                    strokeWidth={2.2}
                    fill="url(#purchase-fill)"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          )}
          <p className="mt-2 text-[10.5px] text-faint">محور عمودی بر حسب میلیون ریال است.</p>
        </Card>

        <Card>
          <CardHeader title="سهم کالاها از فروش" subtitle="شش کالای پرفروش دوره" />
          {loading ? (
            <Skeleton className="h-72 w-full" />
          ) : productShare.length === 0 ? (
            <EmptyState title="فروشی ثبت نشده است." hint="پس از صدور فاکتور، سهم کالاها محاسبه می‌شود." />
          ) : (
            <div className="h-72 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={productShare}
                    dataKey="value"
                    nameKey="name"
                    innerRadius="52%"
                    outerRadius="80%"
                    paddingAngle={2}
                    stroke="var(--card)"
                    strokeWidth={2}
                  >
                    {productShare.map((entry) => (
                      <Cell key={entry.name} fill={entry.fill} />
                    ))}
                  </Pie>
                  <ChartTooltip
                    {...chartTooltipStyle()}
                    formatter={(value) => `${money(Number(value ?? 0))} ریال`}
                  />
                  <Legend wrapperStyle={{ fontSize: 10.5 }} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          )}
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader
            title="کالاهای نزدیک به اتمام"
            subtitle="حد هشدار در مرکز تنظیمات قابل تغییر است"
            action={
              low.length > 0 ? <Badge tone="warning">{formatNumber(low.length)} قلم</Badge> : undefined
            }
          />
          {loading ? (
            <Skeleton className="h-56 w-full" />
          ) : low.length === 0 ? (
            <EmptyState title="موجودی همه‌ی کالاها بالای حد هشدار است." hint="نیازی به سفارش فوری نیست." />
          ) : (
            <div className="h-56 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart
                  data={low.slice(0, 8).map((row) => ({
                    name: row.name,
                    موجودی: row.quantity,
                    'حد سفارش': row.min_stock,
                  }))}
                  layout="vertical"
                  margin={{ top: 4, right: 12, left: 4, bottom: 0 }}
                >
                  <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" horizontal={false} />
                  <XAxis type="number" {...AXIS} />
                  <YAxis
                    type="category"
                    dataKey="name"
                    {...AXIS}
                    width={110}
                    tick={{ fontSize: 10.5 }}
                  />
                  <ChartTooltip {...chartTooltipStyle()} />
                  <Legend wrapperStyle={{ fontSize: 10.5 }} />
                  <Bar dataKey="موجودی" fill="var(--chart-3)" radius={[0, 6, 6, 0]} barSize={11} />
                  <Bar dataKey="حد سفارش" fill="var(--chart-2)" radius={[0, 6, 6, 0]} barSize={11} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}
        </Card>

        <Card pad={false}>
          <div className="p-4 sm:p-5">
            <CardHeader title="آخرین فاکتورها" subtitle="جدیدترین اسناد ثبت‌شده" />
          </div>
          {loading ? (
            <div className="px-4 pb-5 sm:px-5">
              <Skeleton className="h-56 w-full" />
            </div>
          ) : recent.length === 0 ? (
            <div className="px-4 pb-5 sm:px-5">
              <EmptyState title="فاکتوری ثبت نشده است." hint="از منوی فروش شروع کنید." />
            </div>
          ) : (
            <div className="max-h-56 overflow-y-auto px-2 pb-3">
              {recent.slice(0, 10).map((invoice) => (
                <div
                  key={invoice.id}
                  className="flex items-center justify-between gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-bg-soft"
                >
                  <div className="min-w-0">
                    <p className="truncate text-xs font-semibold text-text">
                      {invoice.contact_name ?? 'بدون طرف حساب'}
                    </p>
                    <p className="text-[10.5px] text-muted">
                      {invoice.invoice_type === 'purchase' ? 'خرید' : 'فروش'} · شماره{' '}
                      {invoice.number} · {invoice.invoice_date}
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    <span className="tnum text-xs font-bold text-text">{money(invoice.total)}</span>
                    <Badge tone={invoice.payment_status === 'paid' ? 'success' : 'warning'}>
                      {invoice.payment_status === 'paid'
                        ? 'تسویه'
                        : invoice.payment_status === 'partial'
                          ? 'بخشی'
                          : 'نسیه'}
                    </Badge>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      <Card>
        <CardHeader title="پرفروش‌ترین کالاها" subtitle="بر اساس مبلغ فروش دوره" />
        {loading ? (
          <Skeleton className="h-40 w-full" />
        ) : products.length === 0 ? (
          <EmptyState title="فروشی ثبت نشده است." hint="پس از صدور فاکتور، این فهرست پر می‌شود." />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-[11px] text-muted">
                  <th className="p-2 text-start font-semibold">کالا</th>
                  <th className="p-2 text-start font-semibold">مقدار فروش</th>
                  <th className="p-2 text-start font-semibold">مبلغ فروش (ریال)</th>
                  <th className="p-2 text-start font-semibold">سهم</th>
                </tr>
              </thead>
              <tbody>
                {products.slice(0, 8).map((product) => {
                  const total = products.reduce((sum, row) => sum + row.revenue, 0) || 1
                  const share = (product.revenue / total) * 100
                  return (
                    <tr key={product.product_id} className="border-t border-border">
                      <td className="p-2 font-semibold text-text">{product.name}</td>
                      <td className="tnum p-2 text-muted">{formatNumber(product.quantity)}</td>
                      <td className="tnum p-2 text-text">{money(product.revenue)}</td>
                      <td className="p-2">
                        <div className="flex items-center gap-2">
                          <div className="h-1.5 w-24 overflow-hidden rounded-full bg-bg-soft">
                            <div
                              className="h-full rounded-full bg-accent"
                              style={{ width: `${Math.min(100, share)}%` }}
                            />
                          </div>
                          <span className="tnum text-[10.5px] text-muted">
                            {formatNumber(share)}٪
                          </span>
                        </div>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </section>
  )
}
