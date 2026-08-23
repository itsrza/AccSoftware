import {
  Clock4, AlertCircle, CalendarClock, CheckCircle2, Wallet, Landmark, CircleDollarSign,
} from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import type { Aging } from "../data/engine";
import { faDateShort, fmt, fmtCompact } from "../lib/format";
import { Badge, Card, CardHeader, Skeleton } from "./ui";

// ------------------------------------------------------------- aging card
const TONE_STYLE: Record<string, { dot: string; bar: string }> = {
  ok: { dot: "bg-success", bar: "bg-success" },
  warn: { dot: "bg-warning", bar: "bg-warning" },
  bad: { dot: "bg-danger", bar: "bg-danger" },
  done: { dot: "bg-[var(--faint)]", bar: "bg-[var(--border-strong)]" },
};

const TONE_ICON = {
  ok: CheckCircle2,
  warn: CalendarClock,
  bad: AlertCircle,
  done: Clock4,
} as const;

function AgingCard({
  title, subtitle, aging, icon: Icon, className,
}: {
  title: string; subtitle: string; aging: Aging; icon: typeof Wallet; className?: string;
}) {
  const { loading } = useApp();
  const max = Math.max(aging.total, 1);
  const allMax = Math.max(...aging.buckets.map((b) => b.amount), 1);

  return (
    <Card className={cn("flex min-w-0 flex-col", className)}>
      <CardHeader
        title={title}
        subtitle={subtitle}
        action={
          <span className="grid size-9 place-items-center rounded-xl bg-[var(--primary-soft)] text-primary">
            <Icon className="size-4.5" aria-hidden />
          </span>
        }
      />
      {loading ? (
        <div className="space-y-3">
          <Skeleton className="h-8 w-40" />
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-7 w-full" />
          ))}
        </div>
      ) : (
        <>
          <div className="mb-4 flex items-end justify-between">
            <div>
              <p className="text-[10.5px] font-semibold text-muted">مانده باز</p>
              <p className="tnum text-[22px] font-extrabold tracking-tight text-text">
                {fmtCompact(aging.total)}
                <span className="ms-1 text-[10px] font-semibold text-faint">تومان</span>
              </p>
            </div>
            {/* stacked share bar */}
            <div className="ms-3 flex h-2.5 w-32 overflow-hidden rounded-full bg-bg-soft sm:w-40" aria-hidden>
              {aging.buckets.filter((b) => b.tone !== "done").map((b) => (
                <span
                  key={b.label}
                  className={cn("h-full", TONE_STYLE[b.tone].bar)}
                  style={{ width: `${(b.amount / max) * 100}%` }}
                />
              ))}
            </div>
          </div>
          <ul className="space-y-2.5">
            {aging.buckets.map((b) => {
              const BIcon = TONE_ICON[b.tone];
              return (
                <li key={b.label} className="group">
                  <div className="flex items-center gap-2 text-[11.5px]">
                    <span className={cn("size-1.5 shrink-0 rounded-full", TONE_STYLE[b.tone].dot)} aria-hidden />
                    <span className="flex items-center gap-1.5 text-muted">
                      <BIcon className="size-3.5 text-faint" aria-hidden />
                      {b.label}
                      <span className="tnum text-[10px] text-faint">({fmt(b.count)} سند)</span>
                    </span>
                    <span className="tnum ms-auto font-bold text-text">{fmtCompact(b.amount)}</span>
                  </div>
                  <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-bg-soft" aria-hidden>
                    <div
                      className={cn("h-full rounded-full transition-all duration-500", TONE_STYLE[b.tone].bar)}
                      style={{ width: `${Math.max((b.amount / allMax) * 100, b.amount > 0 ? 4 : 0)}%` }}
                    />
                  </div>
                </li>
              );
            })}
          </ul>
        </>
      )}
    </Card>
  );
}

export function AgingCards() {
  const { data } = useApp();
  return (
    <>
      <AgingCard
        title="مطالبات"
        subtitle="وضعیت مطالبات از مشتریان"
        aging={data.receivables}
        icon={CircleDollarSign}
      />
      <AgingCard
        title="بدهی‌ها"
        subtitle="وضعیت بدهی به تامین‌کنندگان"
        aging={data.payables}
        icon={Landmark}
      />
    </>
  );
}

// ------------------------------------------------------------- top customers
export function TopCustomers({ className }: { className?: string }) {
  const { data, loading } = useApp();
  const items = data.topCustomers;
  const max = Math.max(...items.map((c) => c.amount), 1);

  return (
    <Card className={cn("flex min-w-0 flex-col", className)}>
      <CardHeader
        title="مشتریان برتر"
        subtitle="برترین طرف حساب‌ها بر اساس مبلغ خرید"
        action={<Badge tone="accent" dot={false}>۶ مشتری اول</Badge>}
      />
      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <p className="rounded-xl border border-dashed border-border-strong bg-card-soft py-8 text-center text-xs text-muted">
          اطلاعاتی برای نمایش وجود ندارد.
        </p>
      ) : (
        <ol className="space-y-1">
          {items.map((c, i) => (
            <li
              key={c.id}
              className="group flex items-center gap-3 rounded-xl px-2 py-2.5 transition-colors hover:bg-card-soft"
            >
              <span
                className={cn(
                  "grid size-7 shrink-0 place-items-center rounded-lg text-[11px] font-extrabold",
                  i === 0
                    ? "bg-[var(--accent-soft)] text-accent-strong"
                    : "bg-bg-soft text-muted group-hover:text-text",
                )}
              >
                {fmt(i + 1)}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-baseline justify-between gap-2">
                  <p className="truncate text-xs font-bold text-text">{c.name}</p>
                  <p className="tnum shrink-0 text-xs font-extrabold text-text">{fmtCompact(c.amount)}</p>
                </div>
                <div className="mt-1.5 flex items-center gap-2">
                  <div className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-bg-soft" aria-hidden>
                    <div
                      className={cn(
                        "h-full rounded-full transition-all duration-700",
                        i === 0 ? "bg-accent" : "bg-[var(--chart-2)] opacity-70",
                      )}
                      style={{ width: `${(c.amount / max) * 100}%` }}
                    />
                  </div>
                  <span className="tnum shrink-0 text-[9.5px] text-faint">
                    {fmt(c.count)} خرید · مانده {fmtCompact(Math.max(c.balance, 0))} · {faDateShort(c.last)}
                  </span>
                </div>
              </div>
            </li>
          ))}
        </ol>
      )}
    </Card>
  );
}
