import {
  AlertCircle,
  CalendarClock,
  CheckCircle2,
  CircleDollarSign,
  Clock4,
  Landmark,
} from 'lucide-react'
import { cn } from '../lib/cn'
import { Badge, Card, CardHeader, Skeleton } from './ui'
import { formatNumber, formatRials as money } from '../lib/format'
import type { PartyAging, PartyBalance } from '../api'

/**
 * پنل‌های وضعیت مالی داشبورد — منطبق با سیستم طراحی مرجع.
 *
 * ## چرا «سنی‌سازی» و نه فقط یک عدد
 * «مطالبات: ۱٫۲ میلیارد» چیزی به مدیر نمی‌گوید. آنچه تصمیم می‌سازد این است
 * که چه بخشی از آن سررسید نشده و چه بخشی بیش از ۹۰ روز معوق است. ساختار
 * سطل‌های سنی همان چیزی است که در گزارش «سنی‌سازی» نرم‌افزار فعلی وجود
 * دارد و مرجع طراحی هم آن را به‌صورت نوار سهم نشان می‌دهد.
 *
 * تمام اعداد از فرمان واقعی `get_party_aging` می‌آیند؛ هیچ عدد ساختگی نیست.
 */

type Tone = 'ok' | 'warn' | 'bad' | 'done'

const TONE_STYLE: Record<Tone, { dot: string; bar: string; card: string }> = {
  ok: {
    dot: 'bg-success',
    bar: 'bg-success',
    card: 'border-[var(--success)]/25 bg-[var(--success-soft)]',
  },
  warn: {
    dot: 'bg-warning',
    bar: 'bg-warning',
    card: 'border-[var(--warning)]/25 bg-[var(--warning-soft)]',
  },
  bad: {
    dot: 'bg-danger',
    bar: 'bg-danger',
    card: 'border-[var(--danger)]/30 bg-[var(--danger-soft)]',
  },
  done: {
    dot: 'bg-[var(--faint)]',
    bar: 'bg-[var(--border-strong)]',
    card: 'border-border bg-card-soft',
  },
}

const TONE_ICON: Record<Tone, typeof Clock4> = {
  ok: CheckCircle2,
  warn: CalendarClock,
  bad: AlertCircle,
  done: Clock4,
}

export type AgingBucket = { label: string; amount: number; count: number; tone: Tone }

/**
 * تبدیل خروجی `get_party_aging` به سطل‌های نمایشی.
 *
 * «تعداد» یعنی تعداد **طرف حساب‌هایی** که در آن سطل مبلغ باز دارند — نه
 * تعداد فاکتور. برچسب هم همین را می‌گوید تا هیچ ابهامی نماند.
 */
export function toBuckets(rows: PartyAging[]): { buckets: AgingBucket[]; total: number } {
  const definitions: { label: string; tone: Tone; pick: (row: PartyAging) => number }[] = [
    { label: 'سررسید نشده', tone: 'ok', pick: (row) => row.current },
    { label: '۱ تا ۳۰ روز', tone: 'warn', pick: (row) => row.days_1_30 },
    { label: '۳۱ تا ۶۰ روز', tone: 'warn', pick: (row) => row.days_31_60 },
    { label: '۶۱ تا ۹۰ روز', tone: 'bad', pick: (row) => row.days_61_90 },
    { label: 'بیش از ۹۰ روز', tone: 'bad', pick: (row) => row.over_90 },
  ]
  const buckets = definitions.map((definition) => ({
    label: definition.label,
    tone: definition.tone,
    amount: rows.reduce((sum, row) => sum + definition.pick(row), 0),
    count: rows.filter((row) => definition.pick(row) > 0).length,
  }))
  return { buckets, total: rows.reduce((sum, row) => sum + row.total, 0) }
}

export function AgingPanel({
  title,
  subtitle,
  rows,
  loading,
  kind,
}: {
  title: string
  subtitle: string
  rows: PartyAging[]
  loading: boolean
  kind: 'receivable' | 'payable'
}) {
  const { buckets, total } = toBuckets(rows)
  const largest = Math.max(...buckets.map((bucket) => bucket.amount), 1)
  const share = Math.max(total, 1)
  const Icon = kind === 'receivable' ? CircleDollarSign : Landmark

  return (
    <Card className="flex h-full min-w-0 flex-col">
      <CardHeader
        title={title}
        subtitle={subtitle}
        action={
          <span className="grid size-9 place-items-center rounded-xl bg-[var(--primary-soft)] text-primary">
            <Icon className="size-4" aria-hidden />
          </span>
        }
      />

      {loading ? (
        <div className="space-y-3">
          <Skeleton className="h-8 w-40" />
          {Array.from({ length: 5 }).map((_, index) => (
            <Skeleton key={index} className="h-7 w-full" />
          ))}
        </div>
      ) : total === 0 ? (
        <p className="rounded-xl border border-dashed border-border-strong bg-card-soft py-8 text-center text-xs text-muted">
          {kind === 'receivable'
            ? 'همه‌ی فاکتورهای فروش تسویه شده‌اند.'
            : 'بدهی تسویه‌نشده‌ای به تأمین‌کنندگان وجود ندارد.'}
        </p>
      ) : (
        <>
          <div className="mb-4 flex items-end justify-between">
            <div>
              <p className="text-[10.5px] font-semibold text-muted">مانده باز</p>
              <p className="tnum text-[22px] font-extrabold tracking-tight text-text">
                {money(total)}
                <span className="ms-1 text-[10px] font-semibold text-faint">ریال</span>
              </p>
            </div>
            <div
              className="ms-3 flex h-2.5 w-32 overflow-hidden rounded-full bg-bg-soft sm:w-40"
              aria-hidden
            >
              {buckets.map((bucket) => (
                <span
                  key={bucket.label}
                  className={cn('h-full', TONE_STYLE[bucket.tone].bar)}
                  style={{ width: `${(bucket.amount / share) * 100}%` }}
                />
              ))}
            </div>
          </div>

          <ul className="space-y-2.5">
            {buckets.map((bucket) => {
              const BucketIcon = TONE_ICON[bucket.tone]
              return (
                <li key={bucket.label}>
                  <div className="flex items-center gap-2 text-[11.5px]">
                    <span
                      className={cn('size-1.5 shrink-0 rounded-full', TONE_STYLE[bucket.tone].dot)}
                      aria-hidden
                    />
                    <span className="flex items-center gap-1.5 text-muted">
                      <BucketIcon className="size-3.5 text-faint" aria-hidden />
                      {bucket.label}
                      <span className="tnum text-[10px] text-faint">
                        ({formatNumber(bucket.count)} طرف حساب)
                      </span>
                    </span>
                    <span className="tnum ms-auto font-bold text-text">{money(bucket.amount)}</span>
                  </div>
                  <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-bg-soft" aria-hidden>
                    <div
                      className={cn(
                        'h-full rounded-full transition-all duration-500',
                        TONE_STYLE[bucket.tone].bar,
                      )}
                      style={{
                        width: `${Math.max((bucket.amount / largest) * 100, bucket.amount > 0 ? 4 : 0)}%`,
                      }}
                    />
                  </div>
                </li>
              )
            })}
          </ul>
        </>
      )}
    </Card>
  )
}

/**
 * برترین طرف حساب‌ها بر اساس مبلغ فاکتورشده در دوره.
 *
 * مانده‌ی باز هم کنارش می‌آید: مشتری پرخرید که پول نمی‌دهد، همان‌قدر مهم
 * است که مشتری پرخریدِ خوش‌حساب.
 */
export function TopParties({
  title,
  subtitle,
  rows,
  loading,
  className,
}: {
  title: string
  subtitle: string
  rows: PartyBalance[]
  loading: boolean
  className?: string
}) {
  const items = [...rows].sort((a, b) => b.invoiced - a.invoiced).slice(0, 6)
  const largest = Math.max(...items.map((item) => item.invoiced), 1)

  return (
    <Card className={cn('flex min-w-0 flex-col', className)}>
      <CardHeader
        title={title}
        subtitle={subtitle}
        action={
          items.length > 0 ? (
            <Badge tone="accent" dot={false}>
              {formatNumber(items.length)} مورد اول
            </Badge>
          ) : undefined
        }
      />
      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, index) => (
            <Skeleton key={index} className="h-12 w-full" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <p className="rounded-xl border border-dashed border-border-strong bg-card-soft py-8 text-center text-xs text-muted">
          در این بازه فاکتوری ثبت نشده است.
        </p>
      ) : (
        <ol className="space-y-1">
          {items.map((item, index) => (
            <li
              key={item.contact_id}
              className="group flex items-center gap-3 rounded-xl px-2 py-2.5 transition-colors hover:bg-card-soft"
            >
              <span
                className={cn(
                  'grid size-7 shrink-0 place-items-center rounded-lg text-[11px] font-extrabold',
                  index === 0
                    ? 'bg-[var(--accent-soft)] text-accent-strong'
                    : 'bg-bg-soft text-muted group-hover:text-text',
                )}
              >
                {formatNumber(index + 1)}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline justify-between gap-2">
                  <p className="truncate text-xs font-bold text-text">{item.contact_name}</p>
                  <p className="tnum shrink-0 text-xs font-extrabold text-text">
                    {money(item.invoiced)}
                  </p>
                </div>
                <div className="mt-1.5 flex items-center gap-2">
                  <div
                    className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-bg-soft"
                    aria-hidden
                  >
                    <div
                      className={cn(
                        'h-full rounded-full transition-all duration-700',
                        index === 0 ? 'bg-accent' : 'bg-[var(--chart-2)] opacity-70',
                      )}
                      style={{ width: `${(item.invoiced / largest) * 100}%` }}
                    />
                  </div>
                  <span className="tnum shrink-0 text-[9.5px] text-faint">
                    {formatNumber(item.invoice_count)} فاکتور · مانده {money(item.remaining)}
                  </span>
                </div>
              </div>
            </li>
          ))}
        </ol>
      )}
    </Card>
  )
}
