import { useEffect, useMemo, useRef, useState } from 'react'
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
  HandCoins,
  Package,
  Receipt,
  TrendingUp,
  Wallet,
  type LucideIcon,
} from 'lucide-react'
import { errorText } from '../lib/errors'
import { formatRials as money, formatNumber } from '../lib/format'
import { Badge, Card, CardHeader, EmptyState, ErrorState, Skeleton, TrendChip } from '../components/ui'
import { FilterBar } from '../components/FilterBar'
import { AgingPanel, TopParties } from '../components/DashboardPanels'
import {
  defaultRange,
  pctChange,
  useDashboardData,
  useFiscalPeriod,
  type PaymentFilter,
} from './dashboardData'
import { resolveRange, type JalaliRange } from '../lib/dateRange'

/**
 * داشبورد — چیدمان منطبق با سیستم طراحی مرجع.
 *
 * ترتیب مرجع: نوار فیلتر سراسری ← شبکه‌ی شاخص‌ها ← نمودارها ← وضعیت مالی
 * (مطالبات و بدهی‌ها) ← کالا و مشتری ← آخرین تراکنش‌ها.
 *
 * ## دو قاعده‌ای که رعایت شده
 * ۱. **هیچ عدد یا نمودار ساختگی نیست.** همه از فرمان‌های واقعی موتور می‌آید
 *    و نبودن داده به «حالت خالی» می‌رسد، نه به نمودار تزئینی.
 * ۲. **دوره در برابر اکنون.** اقلام سود و زیان با بازه عوض می‌شوند و
 *    درصد تغییر دارند؛ مانده‌های ترازنامه‌ای «در لحظه»اند و روی کارتشان
 *    صریح نوشته شده تا با عدد دوره اشتباه گرفته نشوند.
 */

type KpiDef = {
  key: string
  label: string
  icon: LucideIcon
  tone: string
  unit: string
  /** آیا این شاخص به بازه‌ی انتخابی وابسته است؟ */
  periodic: boolean
  /** برای مطالبات و بدهی‌ها، رشد خبر بدی است. */
  invert?: boolean
}

const KPI_DEFS: KpiDef[] = [
  { key: 'sales', label: 'فروش دوره', icon: TrendingUp, tone: 'var(--chart-1)', unit: 'ریال', periodic: true },
  { key: 'purchases', label: 'خرید دوره', icon: CreditCard, tone: 'var(--chart-6)', unit: 'ریال', periodic: true },
  { key: 'vat', label: 'ارزش افزوده دوره', icon: Receipt, tone: 'var(--chart-4)', unit: 'ریال', periodic: true },
  { key: 'invoices', label: 'فاکتور فروش', icon: HandCoins, tone: 'var(--chart-5)', unit: 'فقره', periodic: true },
  { key: 'receivables', label: 'مطالبات', icon: Clock4, tone: 'var(--chart-2)', unit: 'ریال', periodic: false, invert: true },
  { key: 'payables', label: 'بدهی‌ها', icon: AlertCircle, tone: 'var(--chart-3)', unit: 'ریال', periodic: false, invert: true },
  { key: 'cash', label: 'موجودی نقد و بانک', icon: Wallet, tone: 'var(--chart-5)', unit: 'ریال', periodic: false },
  { key: 'inventory', label: 'ارزش موجودی کالا', icon: Package, tone: 'var(--chart-2)', unit: 'ریال', periodic: false },
]

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
  value,
  change,
  spark,
  loading,
}: {
  def: KpiDef
  value: number
  change: number | null
  spark: number[]
  loading: boolean
}) {
  const Icon = def.icon
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
              {def.unit === 'فقره' ? formatNumber(value) : money(value)}
              <span className="ms-1.5 text-[10.5px] font-semibold text-faint">{def.unit}</span>
            </p>
          )}
        </div>
        {!loading && def.periodic && <Sparkline data={spark} tone={def.tone} />}
      </div>
      <div className="mt-2.5">
        {loading ? (
          <Skeleton className="h-4 w-24" />
        ) : def.periodic ? (
          <TrendChip value={change} invert={def.invert} />
        ) : (
          <span className="inline-flex items-center gap-1 text-[11px] text-faint">
            <Clock4 className="size-3" aria-hidden />
            مانده در لحظه — مستقل از بازه
          </span>
        )}
      </div>
    </article>
  )
}

export function Dashboard({ demo }: { demo: boolean }) {
  const [range, setRange] = useState<JalaliRange>(() => defaultRange())
  const [payment, setPayment] = useState<PaymentFilter>('all')
  const [search, setSearch] = useState('')

  // بازه‌ی سال مالی از پایگاه داده می‌آید؛ یک بار روی نوار فیلتر می‌نشیند.
  const fiscal = useFiscalPeriod(demo)
  const fiscalRange = useMemo(
    () => (fiscal ? { from: fiscal.start_date, to: fiscal.end_date } : undefined),
    [fiscal],
  )
  const fiscalApplied = useRef(false)
  useEffect(() => {
    if (!fiscalRange || fiscalApplied.current) return
    fiscalApplied.current = true
    setRange((current) =>
      current.preset === 'fiscalYear' ? { preset: 'fiscalYear', ...fiscalRange } : current,
    )
  }, [fiscalRange])

  const data = useDashboardData(range, payment, search, demo)
  const { loading } = data

  const isDefault = range.preset === 'fiscalYear' && payment === 'all' && search.trim() === ''
  const reset = () => {
    setRange(resolveRange('fiscalYear', fiscalRange))
    setPayment('all')
    setSearch('')
  }

  const valueOf = (key: string): number => {
    switch (key) {
      case 'sales':
        return data.period.sales
      case 'purchases':
        return data.period.purchases
      case 'vat':
        return data.period.vat
      case 'invoices':
        return data.period.invoiceCount
      case 'receivables':
        return data.balances?.receivables ?? 0
      case 'payables':
        return data.balances?.payables ?? 0
      case 'cash':
        return data.balances?.cash ?? 0
      case 'inventory':
        return data.balances?.inventory_value ?? 0
      default:
        return 0
    }
  }

  const changeOf = (key: string): number | null => {
    switch (key) {
      case 'sales':
        return pctChange(data.period.sales, data.previous.sales)
      case 'purchases':
        return pctChange(data.period.purchases, data.previous.purchases)
      case 'vat':
        return pctChange(data.period.vat, data.previous.vat)
      case 'invoices':
        return pctChange(data.period.invoiceCount, data.previous.invoiceCount)
      default:
        return null
    }
  }

  const sparkOf = (key: string): number[] => {
    if (key === 'purchases') return data.trend.map((point) => point.purchases)
    if (key === 'invoices') return data.trend.map((point) => (point.sales > 0 ? 1 : 0))
    return data.trend.map((point) => point.sales)
  }

  const chartData = useMemo(
    () => data.trend.map((point) => ({ period: point.period, فروش: point.sales, خرید: point.purchases })),
    [data.trend],
  )

  const productShare = useMemo(
    () =>
      data.topProducts.slice(0, 6).map((product, index) => ({
        name: product.name,
        value: product.revenue,
        fill: `var(--chart-${(index % 6) + 1})`,
      })),
    [data.topProducts],
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
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">نمای کلی</div>
          <h1>داشبورد</h1>
          <p>
            اقلام سود و زیان با بازه‌ی انتخابی عوض می‌شوند؛ مانده‌های ترازنامه‌ای همیشه «در لحظه»
            نمایش داده می‌شوند.
          </p>
        </div>
      </div>

      <FilterBar
        range={range}
        onRange={setRange}
        filters={[
          {
            key: 'payment',
            label: 'وضعیت تسویه',
            value: payment,
            width: 'xl:w-44',
            onChange: (value) => setPayment(value as PaymentFilter),
            options: [
              { value: 'all', label: 'همه‌ی وضعیت‌ها' },
              { value: 'paid', label: 'تسویه‌شده' },
              { value: 'partial', label: 'تسویه‌ی جزئی' },
              { value: 'unpaid', label: 'تسویه‌نشده' },
            ],
          },
        ]}
        search={search}
        onSearch={setSearch}
        searchPlaceholder="جستجوی طرف حساب…"
        onReset={reset}
        isDefault={isDefault}
        fiscalRange={fiscalRange}
        note={loading ? undefined : `${formatNumber(data.matchCount)} فاکتور`}
      />

      {data.error && <ErrorState onRetry={data.reload} />}

      {/* --- شبکه‌ی شاخص‌ها --- */}
      <section
        aria-label="شاخص‌های کلیدی"
        className="fade-up grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4"
      >
        {KPI_DEFS.map((def) => (
          <KpiCard
            key={def.key}
            def={def}
            value={valueOf(def.key)}
            change={changeOf(def.key)}
            spark={sparkOf(def.key)}
            loading={loading}
          />
        ))}
      </section>

      {/* --- نمودارها --- */}
      <div className="fade-up grid grid-cols-12 gap-4" style={{ animationDelay: '80ms' }}>
        <Card className="col-span-12 xl:col-span-8">
          <CardHeader
            title="روند فروش و خرید"
            subtitle="بر اساس فاکتورهای همان بازه — بازه‌ی یک‌ماهه روزانه، بازه‌ی بلندتر ماهانه"
            action={
              data.profit ? (
                <Badge tone="accent" dot={false}>
                  حاشیه سود ناخالص سال مالی: {formatNumber(data.profit.gross_margin_percent)}٪
                </Badge>
              ) : undefined
            }
          />
          {loading ? (
            <Skeleton className="h-72 w-full" />
          ) : chartData.length === 0 ? (
            <EmptyState
              title="در این بازه فاکتوری ثبت نشده است."
              hint="بازه را تغییر دهید یا فیلترها را بردارید."
            />
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
                  <YAxis {...AXIS} width={70} tickFormatter={(value: number) => money(value / 1_000_000)} />
                  <ChartTooltip
                    {...chartTooltipStyle()}
                    formatter={(value) => `${money(Number(value ?? 0))} ریال`}
                  />
                  <Legend wrapperStyle={{ fontSize: 11, paddingTop: 8 }} />
                  <Area type="monotone" dataKey="فروش" stroke="var(--chart-1)" strokeWidth={2.2} fill="url(#sales-fill)" />
                  <Area type="monotone" dataKey="خرید" stroke="var(--chart-4)" strokeWidth={2.2} fill="url(#purchase-fill)" />
                </AreaChart>
              </ResponsiveContainer>
            </div>
          )}
          <p className="mt-2 text-[10.5px] text-faint">محور عمودی بر حسب میلیون ریال است.</p>
        </Card>

        <Card className="col-span-12 md:col-span-6 xl:col-span-4">
          <CardHeader title="سهم کالاها از فروش" subtitle="شش کالای پرفروش سال مالی" />
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

      {/* --- وضعیت مالی: مطالبات و بدهی‌ها --- */}
      <div className="fade-up grid grid-cols-12 gap-4" style={{ animationDelay: '140ms' }}>
        <div className="col-span-12 lg:col-span-6">
          <AgingPanel
            title="مطالبات"
            subtitle="سنی‌سازی طلب از مشتریان تا پایان بازه"
            rows={data.receivableAging}
            loading={loading}
            kind="receivable"
          />
        </div>
        <div className="col-span-12 lg:col-span-6">
          <AgingPanel
            title="بدهی‌ها"
            subtitle="سنی‌سازی بدهی به تأمین‌کنندگان تا پایان بازه"
            rows={data.payableAging}
            loading={loading}
            kind="payable"
          />
        </div>
      </div>

      {/* --- مشتریان و تأمین‌کنندگان برتر --- */}
      <div className="fade-up grid grid-cols-12 gap-4" style={{ animationDelay: '200ms' }}>
        <TopParties
          className="col-span-12 xl:col-span-6"
          title="مشتریان برتر"
          subtitle="بیشترین مبلغ فاکتور فروش در بازه‌ی انتخابی"
          rows={data.topCustomers}
          loading={loading}
        />
        <TopParties
          className="col-span-12 xl:col-span-6"
          title="تأمین‌کنندگان برتر"
          subtitle="بیشترین مبلغ فاکتور خرید در بازه‌ی انتخابی"
          rows={data.topSuppliers}
          loading={loading}
        />
      </div>

      {/* --- انبار و آخرین تراکنش‌ها --- */}
      <div className="fade-up grid grid-cols-12 gap-4" style={{ animationDelay: '260ms' }}>
        <Card className="col-span-12 xl:col-span-6">
          <CardHeader
            title="کالاهای نزدیک به اتمام"
            subtitle="حد هشدار در مرکز تنظیمات قابل تغییر است"
            action={
              data.lowStock.length > 0 ? (
                <Badge tone="warning">{formatNumber(data.lowStock.length)} قلم</Badge>
              ) : undefined
            }
          />
          {loading ? (
            <Skeleton className="h-56 w-full" />
          ) : data.lowStock.length === 0 ? (
            <EmptyState title="موجودی همه‌ی کالاها بالای حد هشدار است." hint="نیازی به سفارش فوری نیست." />
          ) : (
            <div className="h-56 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart
                  data={data.lowStock.slice(0, 8).map((row) => ({
                    name: row.name,
                    موجودی: row.quantity,
                    'حد سفارش': row.min_stock,
                  }))}
                  layout="vertical"
                  margin={{ top: 4, right: 12, left: 4, bottom: 0 }}
                >
                  <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" horizontal={false} />
                  <XAxis type="number" {...AXIS} />
                  <YAxis type="category" dataKey="name" {...AXIS} width={110} tick={{ fontSize: 10.5 }} />
                  <ChartTooltip {...chartTooltipStyle()} />
                  <Legend wrapperStyle={{ fontSize: 10.5 }} />
                  <Bar dataKey="موجودی" fill="var(--chart-3)" radius={[0, 6, 6, 0]} barSize={11} />
                  <Bar dataKey="حد سفارش" fill="var(--chart-2)" radius={[0, 6, 6, 0]} barSize={11} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}
        </Card>

        <Card className="col-span-12 xl:col-span-6" pad={false}>
          <div className="p-4 sm:p-5">
            <CardHeader title="آخرین فاکتورها" subtitle="جدیدترین اسناد ثبت‌شده" />
          </div>
          {loading ? (
            <div className="px-4 pb-5 sm:px-5">
              <Skeleton className="h-56 w-full" />
            </div>
          ) : data.recent.length === 0 ? (
            <div className="px-4 pb-5 sm:px-5">
              <EmptyState title="فاکتوری ثبت نشده است." hint="از منوی فروش شروع کنید." />
            </div>
          ) : (
            <div className="max-h-56 overflow-y-auto px-2 pb-3">
              {data.recent.slice(0, 10).map((invoice) => (
                <div
                  key={invoice.id}
                  className="flex items-center justify-between gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-bg-soft"
                >
                  <div className="min-w-0">
                    <p className="truncate text-xs font-semibold text-text">
                      {invoice.contact_name ?? 'بدون طرف حساب'}
                    </p>
                    <p className="text-[10.5px] text-muted">
                      {invoice.invoice_type === 'purchase' ? 'خرید' : 'فروش'} · شماره {invoice.number} ·{' '}
                      {invoice.invoice_date}
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

      {/* --- پرفروش‌ترین کالاها --- */}
      <Card className="fade-up" >
        <CardHeader title="پرفروش‌ترین کالاها" subtitle="بر اساس مبلغ فروش سال مالی" />
        {loading ? (
          <Skeleton className="h-40 w-full" />
        ) : data.topProducts.length === 0 ? (
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
                {data.topProducts.slice(0, 8).map((product) => {
                  const total = data.topProducts.reduce((sum, row) => sum + row.revenue, 0) || 1
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
                          <span className="tnum text-[10.5px] text-muted">{formatNumber(share)}٪</span>
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

