import type { ReactNode } from 'react'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Funnel,
  FunnelChart,
  LabelList,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  PolarAngleAxis,
  PolarGrid,
  PolarRadiusAxis,
  Radar,
  RadarChart,
  RadialBar,
  RadialBarChart,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  Treemap,
  XAxis,
  YAxis,
  ZAxis,
} from 'recharts'
import { formatNumber, formatRials as money } from '../lib/format'
import { useI18n } from '../lib/i18n'
import { Card, CardHeader } from './ui'

/**
 * کیت نمودارهای تحلیل بصری — ۱۹ نوع نمودار با تم سازمانی.
 *
 * ## چرا یک کیت مشترک
 *
 * همه‌ی نمودارهای برنامه باید یک زبان بصری داشته باشند: همان شش رنگ
 * سری (`--chart-1..6`)، همان راهنمای محور، همان حباب راهنما (tooltip) و
 * همان جهت‌گیری راست‌به‌چپ. اگر هر صفحه این‌ها را جداگانه بچیند، بعد از
 * اولین تغییر تم، نیمی از نمودارها واگرا می‌شوند.
 *
 * ## قرار داد داده
 *
 * هر کامپوننت فقط شکل را می‌شناسد؛ داده‌سازی و معنای حسابداری در صفحه‌ی
 * مصرف‌کننده (`VisualAnalytics`) انجام می‌شود تا نمودار هرگز «عدد ساختگی»
 * نداشته باشد.
 */

export const CHART_COLORS = [
  'var(--chart-1)',
  'var(--chart-4)',
  'var(--chart-5)',
  'var(--chart-2)',
  'var(--chart-6)',
  'var(--chart-3)',
] as const

const AXIS = { stroke: 'transparent', tickLine: false, axisLine: false } as const

/** قاب استاندارد هر نمودار: کارت + عنوان + زیرعنوان + ارتفاع یکسان. */
export function ChartFrame({
  title,
  subtitle,
  height = 260,
  children,
  span = 'col-span-12 md:col-span-6 xl:col-span-4',
}: {
  title: string
  subtitle?: string
  height?: number
  children: ReactNode
  span?: string
}) {
  return (
    <Card className={span}>
      <CardHeader title={title} subtitle={subtitle} />
      <div style={{ height }} className="w-full pt-1">
        {children}
      </div>
    </Card>
  )
}

function useTooltip() {
  const { dir } = useI18n()
  return {
    contentStyle: {
      background: 'var(--card)',
      border: '1px solid var(--border)',
      borderRadius: 14,
      boxShadow: 'var(--shadow-md)',
      fontSize: 12,
      fontFamily: 'inherit',
      direction: dir,
    } as const,
    labelStyle: { color: 'var(--text)', fontWeight: 700 } as const,
    itemStyle: { color: 'var(--muted)' } as const,
  }
}

const fmt = (v: unknown) => money(Number(v ?? 0))

/* ─────────────────────────── روند (خط/ناحیه) ─────────────────────────── */

export function LineTrend({ data, x, series }: { data: Record<string, unknown>[]; x: string; series: { key: string; name: string }[] }) {
  const { dir } = useI18n()
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <LineChart data={data} margin={{ top: 8, right: 10, left: 0, bottom: 0 }}>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={false} />
        <XAxis dataKey={x} {...AXIS} reversed={dir === 'rtl'} tick={{ fontSize: 11 }} />
        <YAxis {...AXIS} width={64} tick={{ fontSize: 10.5 }} tickFormatter={(v) => money(Number(v) / 1_000_000)} />
        <Tooltip {...tip} formatter={fmt} />
        <Legend wrapperStyle={{ fontSize: 11, paddingTop: 6 }} />
        {series.map((s, i) => (
          <Line key={s.key} type="monotone" dataKey={s.key} name={s.name} stroke={CHART_COLORS[i % 6]} strokeWidth={2.4} dot={false} activeDot={{ r: 4 }} />
        ))}
      </LineChart>
    </ResponsiveContainer>
  )
}

export function AreaTrend({ data, x, series }: { data: Record<string, unknown>[]; x: string; series: { key: string; name: string }[] }) {
  const { dir } = useI18n()
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <AreaChart data={data} margin={{ top: 8, right: 10, left: 0, bottom: 0 }}>
        <defs>
          {series.map((s, i) => (
            <linearGradient key={s.key} id={`va-area-${i}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={CHART_COLORS[i % 6]} stopOpacity={0.3} />
              <stop offset="100%" stopColor={CHART_COLORS[i % 6]} stopOpacity={0.02} />
            </linearGradient>
          ))}
        </defs>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={false} />
        <XAxis dataKey={x} {...AXIS} reversed={dir === 'rtl'} tick={{ fontSize: 11 }} />
        <YAxis {...AXIS} width={64} tick={{ fontSize: 10.5 }} tickFormatter={(v) => money(Number(v) / 1_000_000)} />
        <Tooltip {...tip} formatter={fmt} />
        <Legend wrapperStyle={{ fontSize: 11, paddingTop: 6 }} />
        {series.map((s, i) => (
          <Area key={s.key} type="monotone" dataKey={s.key} name={s.name} stroke={CHART_COLORS[i % 6]} strokeWidth={2.2} fill={`url(#va-area-${i})`} />
        ))}
      </AreaChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── ستون‌ها ─────────────────────────── */

type BarProps = {
  data: Record<string, unknown>[]
  x: string
  series: { key: string; name: string }[]
  horizontal?: boolean
  stacked?: boolean
  percent?: boolean
}

export function BarsChart({ data, x, series, horizontal, stacked, percent }: BarProps) {
  const { dir } = useI18n()
  const tip = useTooltip()
  const Category = horizontal ? YAxis : XAxis
  const Value = horizontal ? XAxis : YAxis
  return (
    <ResponsiveContainer>
      <BarChart data={data} layout={horizontal ? 'vertical' : 'horizontal'} margin={{ top: 8, right: 12, left: horizontal ? 12 : 0, bottom: 0 }}>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={Boolean(horizontal)} horizontal={!horizontal} />
        <Category dataKey={x} type={horizontal ? 'category' : undefined} {...AXIS} reversed={dir === 'rtl'} tick={{ fontSize: 10.5 }} width={horizontal ? 96 : undefined} />
        <Value {...AXIS} tick={{ fontSize: 10.5 }} width={horizontal ? undefined : 64} tickFormatter={(v) => (percent ? `${formatNumber(Number(v))}٪` : money(Number(v) / 1_000_000))} />
        <Tooltip {...tip} formatter={percent ? (v) => `${formatNumber(Number(v ?? 0))}٪` : fmt} />
        <Legend wrapperStyle={{ fontSize: 11, paddingTop: 6 }} />
        {series.map((s, i) => (
          <Bar
            key={s.key}
            dataKey={s.key}
            name={s.name}
            stackId={stacked ? 'va' : undefined}
            fill={CHART_COLORS[i % 6]}
            radius={horizontal ? [0, 6, 6, 0] : [6, 6, 0, 0]}
            maxBarSize={26}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  )
}

/** آبشار: ستون‌های شناور با پایه‌ی نامرئی (الگوی استاندارد waterfall). */
export function WaterfallChart({ data, x }: { data: { name: string; base: number; delta: number; tone?: 'up' | 'down' | 'total' }[]; x: string }) {
  const { dir } = useI18n()
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <BarChart data={data} margin={{ top: 8, right: 10, left: 0, bottom: 0 }}>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={false} />
        <XAxis dataKey={x} {...AXIS} reversed={dir === 'rtl'} tick={{ fontSize: 10.5 }} interval={0} />
        <YAxis {...AXIS} width={64} tick={{ fontSize: 10.5 }} tickFormatter={(v) => money(Number(v) / 1_000_000)} />
        <Tooltip
          {...tip}
          cursor={{ fill: 'var(--bg-soft)' }}
          formatter={(value, name) => (name === 'base' ? '' : money(Number(value ?? 0)))}
        />
        <Bar dataKey="base" stackId="wf" fill="transparent" name=" " />
        <Bar dataKey="delta" stackId="wf" radius={[6, 6, 0, 0]} maxBarSize={40} name="+/-">
          {data.map((row) => (
            <Cell
              key={row.name}
              fill={row.tone === 'down' ? 'var(--chart-3)' : row.tone === 'total' ? 'var(--chart-1)' : 'var(--chart-5)'}
            />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── دایره‌ای ─────────────────────────── */

export function CircleShare({ data, donut }: { data: { name: string; value: number }[]; donut?: boolean }) {
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <PieChart>
        <Pie data={data} dataKey="value" nameKey="name" innerRadius={donut ? '54%' : 0} outerRadius="78%" paddingAngle={2} stroke="var(--card)" strokeWidth={2}>
          {data.map((row, i) => (
            <Cell key={row.name} fill={CHART_COLORS[i % 6]} />
          ))}
        </Pie>
        <Tooltip {...tip} formatter={fmt} />
        <Legend wrapperStyle={{ fontSize: 10.5 }} />
      </PieChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── پراکندگی/حباب ─────────────────────────── */

export function ScatterPlot({ data, xName, yName, bubble }: { data: { x: number; y: number; z?: number; name: string }[]; xName: string; yName: string; bubble?: boolean }) {
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <ScatterChart margin={{ top: 10, right: 14, left: 0, bottom: 4 }}>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" />
        <XAxis dataKey="x" name={xName} {...AXIS} tick={{ fontSize: 10.5 }} tickFormatter={(v) => formatNumber(Number(v))} />
        <YAxis dataKey="y" name={yName} {...AXIS} width={64} tick={{ fontSize: 10.5 }} tickFormatter={(v) => money(Number(v) / 1_000_000)} />
        {bubble && <ZAxis dataKey="z" range={[60, 620]} />}
        <Tooltip
          {...tip}
          formatter={(value, name) => [name === yName ? money(Number(value ?? 0)) : formatNumber(Number(value ?? 0)), String(name)]}
          labelFormatter={() => ''}
        />
        <Scatter data={data} fill="var(--chart-4)" fillOpacity={bubble ? 0.55 : 0.8} stroke="var(--chart-1)" strokeOpacity={0.35} />
      </ScatterChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── گیج/قیف/رادار ─────────────────────────── */

export function GaugeChart({ value, label }: { value: number; label: string }) {
  const data = [{ name: label, value: Math.max(0, Math.min(100, value)), fill: 'var(--chart-4)' }]
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1">
      <div className="relative h-full w-full">
        <ResponsiveContainer>
          <RadialBarChart data={data} innerRadius="66%" outerRadius="100%" startAngle={210} endAngle={-30}>
            <PolarAngleAxis type="number" domain={[0, 100]} tick={false} />
            <RadialBar dataKey="value" cornerRadius={12} background={{ fill: 'var(--bg-soft)' }} />
          </RadialBarChart>
        </ResponsiveContainer>
        <div className="pointer-events-none absolute inset-0 grid place-items-center">
          <div className="text-center">
            <div className="tnum text-[26px] font-extrabold text-text">{formatNumber(Math.round(value))}٪</div>
            <div className="text-[10.5px] font-semibold text-faint">{label}</div>
          </div>
        </div>
      </div>
      <span className="sr-only">{`${label}: ${value}`}</span>
    </div>
  )
}

export function FunnelStages({ data }: { data: { name: string; value: number }[] }) {
  const tip = useTooltip()
  const fills = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-4)', 'var(--chart-5)', 'var(--chart-6)']
  return (
    <ResponsiveContainer>
      <FunnelChart>
        <Tooltip {...tip} formatter={(v) => formatNumber(Number(v ?? 0))} />
        <Funnel dataKey="value" nameKey="name" isAnimationActive>
          {data.map((row, i) => (
            <Cell key={row.name} fill={fills[i % fills.length]} />
          ))}
          <LabelList position="right" dataKey="name" fill="var(--muted)" fontSize={11} />
          <LabelList position="inside" dataKey="value" fill="var(--on-primary)" fontSize={11} formatter={(v: unknown) => formatNumber(Number(v ?? 0))} />
        </Funnel>
      </FunnelChart>
    </ResponsiveContainer>
  )
}

export function RadarProfile({ data, axes, series }: { data: Record<string, unknown>[]; axes: string; series: { key: string; name: string }[] }) {
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <RadarChart data={data} outerRadius="72%">
        <PolarGrid stroke="var(--grid)" />
        <PolarAngleAxis dataKey={axes} tick={{ fontSize: 10.5, fill: 'var(--muted)' }} />
        <PolarRadiusAxis {...AXIS} tick={{ fontSize: 9.5, fill: 'var(--faint)' }} tickFormatter={(v) => formatNumber(Number(v))} />
        <Tooltip {...tip} formatter={(v) => formatNumber(Number(v ?? 0))} />
        <Legend wrapperStyle={{ fontSize: 11, paddingTop: 6 }} />
        {series.map((s, i) => (
          <Radar key={s.key} dataKey={s.key} name={s.name} stroke={CHART_COLORS[i % 6]} fill={CHART_COLORS[i % 6]} fillOpacity={0.22} strokeWidth={2} />
        ))}
      </RadarChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── نقشه‌ی درختی ─────────────────────────── */

export function TreeMapShare({ data }: { data: { name: string; size: number }[] }) {
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <Treemap data={data} dataKey="size" nameKey="name" stroke="var(--card)" fill="var(--chart-2)">
        <Tooltip {...tip} formatter={(v) => money(Number(v ?? 0))} />
      </Treemap>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── هیستوگرام ─────────────────────────── */

export function HistogramChart({ data }: { data: { name: string; count: number }[] }) {
  const { dir } = useI18n()
  const tip = useTooltip()
  return (
    <ResponsiveContainer>
      <BarChart data={data} margin={{ top: 8, right: 10, left: 0, bottom: 0 }}>
        <CartesianGrid stroke="var(--grid)" strokeDasharray="3 6" vertical={false} />
        <XAxis dataKey="name" {...AXIS} reversed={dir === 'rtl'} tick={{ fontSize: 9.5 }} interval={0} angle={-18} textAnchor="end" height={44} />
        <YAxis {...AXIS} width={36} tick={{ fontSize: 10.5 }} tickFormatter={(v) => formatNumber(Number(v))} />
        <Tooltip {...tip} formatter={(v) => formatNumber(Number(v ?? 0))} />
        <Bar dataKey="count" fill="var(--chart-2)" radius={[5, 5, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}

/* ─────────────────────────── نقشه‌ی حرارتی (CSS) ─────────────────────────── */

/** شدت رنگ از روشن به پررنگِ طلایی — مستقل از recharts و سبک‌تر از آن. */
export function HeatGrid({ rows, cols, cell, rowKey }: { rows: string[]; cols: string[]; cell: (row: string, col: string) => number; rowKey?: string }) {
  const { t } = useI18n()
  const values = rows.flatMap((r) => cols.map((c) => cell(r, c)))
  const max = Math.max(1, ...values)
  const heat = (v: number) => {
    if (v <= 0) return { background: 'var(--bg-soft)' }
    const ratio = Math.sqrt(v / max)
    return {
      background: `color-mix(in srgb, var(--chart-4) ${Math.round(14 + ratio * 80)}%, var(--card))`,
    }
  }
  return (
    <div className="flex h-full flex-col gap-1.5 overflow-auto" dir="ltr">
      <div className="grid gap-1" style={{ gridTemplateColumns: `84px repeat(${cols.length}, minmax(34px, 1fr))` }}>
        <span />
        {cols.map((c) => (
          <span key={c} className="text-center text-[9.5px] font-bold text-faint">{c}</span>
        ))}
      </div>
      {rows.map((r) => (
        <div key={r} className="grid gap-1" style={{ gridTemplateColumns: `84px repeat(${cols.length}, minmax(34px, 1fr))` }}>
          <span className="truncate text-[10.5px] font-semibold text-muted" title={r}>{rowKey === 'month' ? r : r}</span>
          {cols.map((c) => (
            <span
              key={c}
              title={`${r} · ${c}: ${money(cell(r, c))}`}
              className="tnum grid h-7 place-items-center rounded-md text-[9px] font-bold text-text"
              style={heat(cell(r, c))}
            >
              {cell(r, c) > 0 ? formatNumber(Math.round(cell(r, c) / 1_000_000)) : ''}
            </span>
          ))}
        </div>
      ))}
      <span className="mt-auto text-[9.5px] text-faint">{t('visual.unitMillion')}</span>
    </div>
  )
}

/* ─────────────────────────── خط زمانی (CSS) ─────────────────────────── */

export function TimelineList({ items }: { items: { date: string; title: string; meta: string; tone: 'up' | 'down' | 'flat' }[] }) {
  const toneClass = { up: 'bg-[var(--chart-5)]', down: 'bg-[var(--chart-3)]', flat: 'bg-[var(--chart-2)]' } as const
  return (
    <ul className="h-full overflow-auto pe-1">
      {items.map((item, index) => (
        <li key={`${item.date}-${index}`} className="relative flex gap-3 pb-3.5">
          {index < items.length - 1 && <span className="absolute inset-y-0 start-[13px] w-px bg-border" aria-hidden />}
          <span className={`relative z-10 mt-1 size-[11px] shrink-0 rounded-full ring-4 ring-[var(--card)] ${toneClass[item.tone]}`} />
          <div className="min-w-0 flex-1">
            <p className="tnum text-[11px] font-extrabold text-text" dir="ltr">{item.date}</p>
            <p className="truncate text-[11.5px] font-semibold text-muted">{item.title}</p>
            <p className="truncate text-[10.5px] text-faint">{item.meta}</p>
          </div>
        </li>
      ))}
    </ul>
  )
}

/* ─────────────────────────── کارت KPI ─────────────────────────── */

export function KpiStat({
  label,
  value,
  hint,
  tone = 'brand',
  icon,
}: {
  label: string
  value: string
  hint?: string
  tone?: 'brand' | 'gold' | 'green' | 'red'
  icon?: ReactNode
}) {
  const toneBg = {
    brand: 'bg-[var(--primary-soft)] text-primary',
    gold: 'bg-[var(--accent-soft)] text-accent-strong',
    green: 'bg-[color-mix(in_srgb,var(--chart-5)_16%,var(--card))] text-[var(--chart-5)]',
    red: 'bg-[color-mix(in_srgb,var(--chart-3)_14%,var(--card))] text-[var(--chart-3)]',
  } as const
  return (
    <Card className="col-span-6 md:col-span-3">
      <div className="flex items-center gap-3">
        {icon && <span className={`grid size-10 shrink-0 place-items-center rounded-xl ${toneBg[tone]}`}>{icon}</span>}
        <div className="min-w-0">
          <p className="truncate text-[11px] font-semibold text-muted">{label}</p>
          <p className="tnum truncate text-[17px] font-extrabold text-text">{value}</p>
          {hint && <p className="truncate text-[10px] text-faint">{hint}</p>}
        </div>
      </div>
    </Card>
  )
}
