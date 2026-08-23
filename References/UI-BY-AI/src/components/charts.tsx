import { useMemo, useState, type ReactNode } from "react";
import {
  Area, Bar, BarChart, CartesianGrid, Cell, ComposedChart, Line, Pie, PieChart,
  ResponsiveContainer, Tooltip, XAxis, YAxis,
} from "recharts";
import { ArrowDownLeft, ArrowUpRight, Scale } from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import { bucketize, type Bucket, type Granularity } from "../data/engine";
import { fmtCompact, fmtMoney } from "../lib/format";
import { Card, CardHeader, EmptyState, Segmented, Skeleton } from "./ui";

// ------------------------------------------------------------------ tooltip
function ChartTooltip({ active, payload, label }: any) {
  if (!active || !payload?.length) return null;
  return (
    <div className="min-w-44 rounded-xl border border-border bg-card px-3 py-2.5 shadow-[var(--shadow-lg)]" dir="rtl">
      <p className="mb-1.5 border-b border-border pb-1.5 text-[11px] font-bold text-text">{label}</p>
      <div className="space-y-1">
        {payload.map((p: any) => (
          <div key={p.dataKey} className="flex items-center justify-between gap-4 text-[11px]">
            <span className="flex items-center gap-1.5 text-muted">
              <span className="size-2 rounded-full" style={{ background: p.color ?? p.fill }} aria-hidden />
              {p.name}
            </span>
            <span className="tnum font-bold text-text">{fmtMoney(p.value ?? 0)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ legend
function LegendChips({
  items,
}: { items: { name: string; color: string; total: number }[] }) {
  return (
    <ul className="flex flex-wrap items-center gap-x-4 gap-y-1.5" aria-label="راهنمای نمودار">
      {items.map((i) => (
        <li key={i.name} className="flex items-center gap-1.5 text-[11px] text-muted">
          <span className="size-2.5 rounded-full" style={{ background: i.color }} aria-hidden />
          {i.name}
          <span className="tnum font-bold text-text">{fmtCompact(i.total)}</span>
        </li>
      ))}
    </ul>
  );
}

// ------------------------------------------------------------------ shell
function ChartShell({
  title, subtitle, action, children, height = 300, empty, className,
}: {
  title: string; subtitle?: string; action?: ReactNode; children: ReactNode;
  height?: number; empty?: boolean; className?: string;
}) {
  const { loading } = useApp();
  return (
    <Card className={cn("flex min-w-0 flex-col", className)}>
      <CardHeader title={title} subtitle={subtitle} action={action} />
      {loading ? (
        <div className="skeleton w-full" style={{ height }} aria-hidden />
      ) : empty ? (
        <EmptyState />
      ) : (
        <div style={{ height }} className="min-w-0" dir="ltr">
          {children}
        </div>
      )}
    </Card>
  );
}

const axisCommon = {
  tickLine: false,
  axisLine: false,
  tickMargin: 8,
} as const;

// ==================================================================
// روند مالی — hero
// ==================================================================
export function FinancialTrend({ className }: { className?: string }) {
  const { data, reducedMotion } = useApp();
  const t = data.trend;
  const totals = useMemo(
    () => t.reduce(
      (a, b) => ({
        sales: a.sales + b.sales,
        purchases: a.purchases + b.purchases,
        expenses: a.expenses + b.expenses,
        profit: a.profit + b.profit,
      }),
      { sales: 0, purchases: 0, expenses: 0, profit: 0 },
    ),
    [t],
  );

  return (
    <ChartShell
      className={className}
      title="روند مالی"
      subtitle="مقایسه فروش، خرید، هزینه و سود در بازه انتخابی"
      height={318}
      empty={t.length === 0}
      action={
        <LegendChips
          items={[
            { name: "فروش", color: "var(--chart-1)", total: totals.sales },
            { name: "خرید", color: "var(--chart-2)", total: totals.purchases },
            { name: "هزینه", color: "var(--chart-3)", total: totals.expenses },
            { name: "سود", color: "var(--chart-4)", total: totals.profit },
          ]}
        />
      }
    >
      <ResponsiveContainer width="100%" height="100%">
        <ComposedChart data={t} margin={{ top: 6, bottom: 0, left: 0, right: 0 }}>
          <defs>
            <linearGradient id="gSales" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--chart-1)" stopOpacity={0.26} />
              <stop offset="100%" stopColor="var(--chart-1)" stopOpacity={0.02} />
            </linearGradient>
            <linearGradient id="gPurch" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--chart-2)" stopOpacity={0.18} />
              <stop offset="100%" stopColor="var(--chart-2)" stopOpacity={0.02} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke="var(--grid)" vertical={false} strokeDasharray="4 6" />
          <XAxis dataKey="label" reversed {...axisCommon} minTickGap={28} />
          <YAxis
            orientation="right"
            width={64}
            {...axisCommon}
            tickFormatter={(v: number) => fmtCompact(v)}
          />
          <Tooltip content={<ChartTooltip />} />
          <Area type="monotone" dataKey="sales" name="فروش" stroke="var(--chart-1)" strokeWidth={2.2} fill="url(#gSales)" isAnimationActive={!reducedMotion} />
          <Area type="monotone" dataKey="purchases" name="خرید" stroke="var(--chart-2)" strokeWidth={1.8} fill="url(#gPurch)" isAnimationActive={!reducedMotion} />
          <Line type="monotone" dataKey="expenses" name="هزینه" stroke="var(--chart-3)" strokeWidth={1.8} dot={false} strokeDasharray="5 4" isAnimationActive={!reducedMotion} />
          <Line type="monotone" dataKey="profit" name="سود" stroke="var(--chart-4)" strokeWidth={2.6} dot={false} isAnimationActive={!reducedMotion} />
        </ComposedChart>
      </ResponsiveContainer>
    </ChartShell>
  );
}

// ==================================================================
// جریان نقدینگی
// ==================================================================
export function CashFlow({ className }: { className?: string }) {
  const { data, reducedMotion } = useApp();
  const t = data.trend;
  const sums = useMemo(
    () => t.reduce(
      (a, b) => ({ in: a.in + b.receipts, out: a.out + b.out, net: a.net + b.net }),
      { in: 0, out: 0, net: 0 },
    ),
    [t],
  );

  return (
    <ChartShell
      className={className}
      title="جریان نقدینگی"
      subtitle="دریافت، پرداخت و خالص جریان نقد"
      height={196}
      empty={t.length === 0}
      action={null}
    >
      <div className="flex h-full flex-col" dir="rtl">
        <div className="grid grid-cols-3 gap-2 px-1 pb-2">
          {[
            { label: "دریافت", value: sums.in, icon: ArrowDownLeft, tone: "var(--success)" },
            { label: "پرداخت", value: sums.out, icon: ArrowUpRight, tone: "var(--danger)" },
            { label: "خالص نقد", value: sums.net, icon: Scale, tone: "var(--accent)" },
          ].map((s) => (
            <div key={s.label} className="rounded-xl bg-card-soft px-2.5 py-2">
              <p className="flex items-center gap-1 text-[10px] font-semibold text-muted">
                <s.icon className="size-3" style={{ color: s.tone }} aria-hidden />
                {s.label}
              </p>
              <p className="tnum mt-1 truncate text-[13px] font-extrabold" style={{ color: s.tone }}>
                {fmtCompact(s.value)}
              </p>
            </div>
          ))}
        </div>
        <div className="min-h-0 flex-1" dir="ltr">
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart data={t} margin={{ top: 4, bottom: 0, left: 0, right: 0 }} barGap={2}>
              <CartesianGrid stroke="var(--grid)" vertical={false} strokeDasharray="4 6" />
              <XAxis dataKey="label" reversed {...axisCommon} minTickGap={40} />
              <YAxis orientation="right" width={56} {...axisCommon} tickFormatter={(v: number) => fmtCompact(v)} />
              <Tooltip content={<ChartTooltip />} />
              <Bar dataKey="receipts" name="دریافت" fill="var(--chart-5)" radius={[4, 4, 0, 0]} maxBarSize={14} isAnimationActive={!reducedMotion} />
              <Bar dataKey="out" name="پرداخت" fill="var(--chart-6)" radius={[4, 4, 0, 0]} maxBarSize={14} isAnimationActive={!reducedMotion} />
              <Line type="monotone" dataKey="net" name="خالص" stroke="var(--chart-4)" strokeWidth={2.4} dot={false} isAnimationActive={!reducedMotion} />
            </ComposedChart>
          </ResponsiveContainer>
        </div>
      </div>
    </ChartShell>
  );
}

// ==================================================================
// روند فروش
// ==================================================================
export function SalesTrend({ className }: { className?: string }) {
  const { data, filters, reducedMotion } = useApp();
  const [g, setG] = useState<Granularity | "auto">("auto");
  const gran: Granularity = g === "auto" ? data.granularity : g;

  const trend: Bucket[] = useMemo(() => {
    // reuse already dim-filtered tx set via trend rebuild for chosen granularity
    return bucketize(data.filtered, filters.range.from.getTime(), filters.range.to.getTime(), gran);
  }, [data.filtered, filters.range, gran]);

  const total = trend.reduce((a, b) => a + b.sales, 0);

  return (
    <ChartShell
      className={className}
      title="روند فروش"
      subtitle={`مجموع بازه: ${fmtMoney(total)}`}
      height={284}
      empty={trend.length === 0}
      action={
        <Segmented
          label="دانه‌بندی زمان"
          size="sm"
          value={g}
          onChange={setG}
          options={[
            { value: "auto", label: "خودکار" },
            { value: "day", label: "روزانه" },
            { value: "week", label: "هفتگی" },
            { value: "month", label: "ماهانه" },
          ]}
        />
      }
    >
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={trend} margin={{ top: 6, bottom: 0, left: 0, right: 0 }}>
          <defs>
            <linearGradient id="gBar" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--chart-4)" stopOpacity={1} />
              <stop offset="100%" stopColor="var(--chart-4)" stopOpacity={0.55} />
            </linearGradient>
          </defs>
          <CartesianGrid stroke="var(--grid)" vertical={false} strokeDasharray="4 6" />
          <XAxis dataKey="label" reversed {...axisCommon} minTickGap={26} />
          <YAxis orientation="right" width={64} {...axisCommon} tickFormatter={(v: number) => fmtCompact(v)} />
          <Tooltip content={<ChartTooltip />} />
          <Bar dataKey="sales" name="فروش" fill="url(#gBar)" radius={[6, 6, 0, 0]} maxBarSize={30} isAnimationActive={!reducedMotion} />
        </BarChart>
      </ResponsiveContainer>
    </ChartShell>
  );
}

// ==================================================================
// تحلیل هزینه‌ها — donut
// ==================================================================
const DONUT_COLORS = [
  "var(--chart-1)", "var(--chart-2)", "var(--chart-4)", "var(--chart-6)",
  "var(--chart-5)", "var(--chart-3)", "color-mix(in srgb, var(--chart-2) 45%, var(--chart-4))",
];

export function ExpenseDonut({ className }: { className?: string }) {
  const { data, reducedMotion, loading } = useApp();
  const items = data.expenses;
  const total = items.reduce((a, b) => a + b.value, 0);

  return (
    <Card className={cn("flex min-w-0 flex-col", className)}>
      <CardHeader title="تحلیل هزینه‌ها" subtitle={`مجموع: ${fmtMoney(total)}`} />
      {loading ? (
        <Skeleton className="h-[248px] w-full" />
      ) : items.length === 0 ? (
        <EmptyState title="هزینه‌ای در این بازه ثبت نشده است." />
      ) : (
        <div className="flex min-w-0 flex-col items-center gap-4 sm:flex-row">
          <div className="relative h-[210px] w-full min-w-0 shrink-0 sm:w-[48%]" dir="ltr">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Tooltip content={<ChartTooltip />} />
                <Pie
                  data={items}
                  dataKey="value"
                  nameKey="name"
                  innerRadius="62%"
                  outerRadius="92%"
                  paddingAngle={2.5}
                  cornerRadius={5}
                  strokeWidth={0}
                  isAnimationActive={!reducedMotion}
                >
                  {items.map((_, i) => (
                    <Cell key={i} fill={DONUT_COLORS[i % DONUT_COLORS.length]} />
                  ))}
                </Pie>
              </PieChart>
            </ResponsiveContainer>
            <div className="pointer-events-none absolute inset-0 grid place-items-center" dir="rtl">
              <div className="text-center">
                <p className="text-[10px] font-semibold text-muted">مجموع هزینه‌ها</p>
                <p className="tnum text-base font-extrabold text-text">{fmtCompact(total)}</p>
              </div>
            </div>
          </div>
          <ul className="w-full min-w-0 flex-1 space-y-1.5">
            {items.map((e, i) => (
              <li key={e.id} className="flex items-center gap-2 text-[11.5px]">
                <span className="size-2.5 shrink-0 rounded-full" style={{ background: DONUT_COLORS[i % DONUT_COLORS.length] }} aria-hidden />
                <span className="min-w-0 flex-1 truncate text-muted">{e.name}</span>
                <span className="tnum font-bold text-text">{fmtCompact(e.value)}</span>
                <span className="tnum w-12 shrink-0 text-left text-[10px] text-faint">
                  {new Intl.NumberFormat("fa-IR", { maximumFractionDigits: 1 }).format(
                    (e.value / total) * 100,
                  )}
                  ٪
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  );
}

// ==================================================================
// پرفروش‌ترین کالاها
// ==================================================================
export function TopProducts({ className }: { className?: string }) {
  const { data, reducedMotion, loading } = useApp();
  const items = data.topProducts;

  return (
    <Card className={cn("flex min-w-0 flex-col", className)}>
      <CardHeader
        title="پرفروش‌ترین کالاها"
        subtitle="بر اساس مبلغ فروش در بازه انتخابی"
      />
      {loading ? (
        <div className="space-y-2.5">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-8 w-full" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <EmptyState title="فروشی برای نمایش وجود ندارد." />
      ) : (
        <div className="h-[296px] min-w-0" dir="ltr">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={items} layout="vertical" margin={{ top: 0, bottom: 0, left: 4, right: 8 }}>
              <CartesianGrid stroke="var(--grid)" horizontal={false} strokeDasharray="4 6" />
              <XAxis type="number" reversed hide />
              <YAxis
                type="category"
                dataKey="name"
                orientation="right"
                width={132}
                {...axisCommon}
                tickFormatter={(v: string) => (v.length > 16 ? `${v.slice(0, 16)}…` : v)}
              />
              <Tooltip
                content={<ChartTooltip />}
                cursor={{ fill: "rgba(127,132,184,.07)" }}
              />
              <Bar
                dataKey="revenue"
                name="مبلغ فروش"
                fill="var(--chart-1)"
                radius={[4, 4, 4, 4]}
                barSize={15}
                isAnimationActive={!reducedMotion}
              />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </Card>
  );
}
