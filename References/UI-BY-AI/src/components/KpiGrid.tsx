import { useMemo } from "react";
import { Area, AreaChart, ResponsiveContainer } from "recharts";
import {
  TrendingUp, HandCoins, Wallet, CreditCard, Clock4, AlertCircle, Package, FileText,
  type LucideIcon,
} from "lucide-react";
import { useApp } from "../store/AppContext";
import { pctChange, type Bucket } from "../data/engine";
import { fmt } from "../lib/format";
import { Skeleton, TrendChip } from "./ui";

interface KpiDef {
  key: string;
  label: string;
  icon: LucideIcon;
  tone: string; // css color
  invert?: boolean;
  value: (k: ReturnType<typeof useApp>["data"]["kpis"]) => number;
  spark: (b: Bucket) => number;
  unit?: string;
}

const KPI_DEFS: KpiDef[] = [
  { key: "sales", label: "فروش کل", icon: TrendingUp, tone: "var(--chart-1)", value: (k) => k.sales, spark: (b) => b.sales, unit: "تومان" },
  { key: "profit", label: "سود خالص", icon: HandCoins, tone: "var(--chart-4)", value: (k) => k.profit, spark: (b) => b.profit, unit: "تومان" },
  { key: "receipts", label: "دریافت‌ها", icon: Wallet, tone: "var(--chart-5)", value: (k) => k.receipts, spark: (b) => b.receipts, unit: "تومان" },
  { key: "payments", label: "پرداخت‌ها", icon: CreditCard, tone: "var(--chart-6)", value: (k) => k.payments, spark: (b) => b.out, unit: "تومان" },
  { key: "receivables", label: "مطالبات", icon: Clock4, tone: "var(--chart-2)", value: (k) => k.receivables, spark: () => 0, invert: true, unit: "تومان" },
  { key: "payables", label: "بدهی‌ها", icon: AlertCircle, tone: "var(--chart-3)", value: (k) => k.payables, spark: () => 0, invert: true, unit: "تومان" },
  { key: "inventory", label: "موجودی کالا", icon: Package, tone: "var(--chart-2)", value: (k) => k.inventory, spark: (b) => b.purchases, unit: "تومان" },
  { key: "invoices", label: "فاکتورهای فروش", icon: FileText, tone: "var(--chart-5)", value: (k) => k.invoices, spark: () => 1, unit: "فاکتور" },
];

function Sparkline({ data, tone }: { data: number[]; tone: string }) {
  const points = useMemo(() => data.map((v, i) => ({ i, v })), [data]);
  const id = useMemo(() => `sp-${Math.random().toString(36).slice(2, 8)}`, []);
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
            dataKey="v"
            stroke={tone}
            strokeWidth={1.8}
            fill={`url(#${id})`}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

function KpiCard({ def }: { def: KpiDef }) {
  const { data, loading } = useApp();
  const value = def.value(data.kpis);
  const prev = def.value(data.prev);
  const change = pctChange(value, prev);

  const sparkData = useMemo(() => {
    if (def.key === "receivables" || def.key === "payables") {
      // sparse visual: steady baseline from receivable aging buckets
      return data.receivables && def.key === "receivables"
        ? data.receivables.buckets.map((b) => b.amount)
        : data.payables.buckets.map((b) => b.amount).reverse();
    }
    if (def.key === "invoices") return data.trend.filter((b) => b.sales > 0).map((b) => b.sales).slice(-14);
    return data.trend.map(def.spark).slice(-14);
  }, [data, def]);

  const Icon = def.icon;

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
              style={{ background: `color-mix(in srgb, ${def.tone} 13%, transparent)`, color: def.tone }}
            >
              <Icon className="size-3.5" aria-hidden />
            </span>
            {def.label}
          </p>
          {loading ? (
            <Skeleton className="mt-2.5 h-6 w-32" />
          ) : (
            <p className="tnum mt-2 truncate text-[19px] font-extrabold tracking-tight text-text sm:text-xl">
              {fmt(value)}
              {def.unit && <span className="ms-1.5 text-[10.5px] font-semibold text-faint">{def.unit}</span>}
            </p>
          )}
        </div>
        {!loading && sparkData.length > 2 && <Sparkline data={sparkData} tone={def.tone} />}
      </div>
      <div className="mt-2.5">
        {loading ? <Skeleton className="h-4 w-24" /> : <TrendChip value={change} invert={def.invert} />}
      </div>
    </article>
  );
}

export function KpiGrid() {
  return (
    <section
      aria-label="شاخص‌های کلیدی"
      className="fade-up grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4 2xl:gap-4"
    >
      {KPI_DEFS.map((def) => (
        <KpiCard key={def.key} def={def} />
      ))}
    </section>
  );
}
