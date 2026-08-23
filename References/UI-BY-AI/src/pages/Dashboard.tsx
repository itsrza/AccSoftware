import { CircleDollarSign, Landmark } from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import { FilterBar } from "../components/FilterBar";
import { KpiGrid } from "../components/KpiGrid";
import {
  CashFlow, ExpenseDonut, FinancialTrend, SalesTrend, TopProducts,
} from "../components/charts";
import { TopCustomers } from "../components/FinancePanels";
import { TransactionsTable } from "../components/TransactionsTable";
import { Badge, Card, CardHeader, Skeleton } from "../components/ui";
import type { Aging } from "../data/engine";
import { fmt, fmtCompact } from "../lib/format";

function AgingPanel({
  title, subtitle, aging, icon: Icon,
}: { title: string; subtitle: string; aging: Aging; icon: typeof Landmark }) {
  const { loading } = useApp();
  return (
    <Card className="h-full">
      <CardHeader
        title={title}
        subtitle={subtitle}
        action={
          loading ? undefined : (
            <Badge tone="neutral" dot={false} className="gap-1">
              <Icon className="size-3.5" aria-hidden />
              مانده باز: <span className="tnum font-extrabold text-text">{fmtCompact(aging.total)}</span> تومان
            </Badge>
          )
        }
      />
      {loading ? (
        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-16 w-full" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
          {aging.buckets.map((b) => (
            <div
              key={b.label}
              className={cn(
                "rounded-xl border p-3 transition-transform duration-200 hover:-translate-y-0.5",
                b.tone === "ok" && "border-[var(--success)]/25 bg-[var(--success-soft)]",
                b.tone === "warn" && "border-[var(--warning)]/25 bg-[var(--warning-soft)]",
                b.tone === "bad" && "border-[var(--danger)]/30 bg-[var(--danger-soft)]",
                b.tone === "done" && "border-border bg-card-soft",
              )}
            >
              <p className="text-[10px] font-bold text-muted">{b.label}</p>
              <p className="tnum mt-1.5 truncate text-sm font-extrabold text-text">
                {fmtCompact(b.amount)}
                <span className="ms-1 text-[9px] font-semibold text-faint">تومان</span>
              </p>
              <p className="tnum mt-0.5 text-[9.5px] text-faint">{fmt(b.count)} سند</p>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

export function Dashboard() {
  const { data } = useApp();
  return (
    <div className="mx-auto flex w-full max-w-[1720px] flex-col gap-4 p-3 sm:p-5 2xl:gap-5">
      <FilterBar />
      <KpiGrid />

      {/* hero: financial trend + cash flow */}
      <div className="fade-up grid grid-cols-12 gap-4 2xl:gap-5" style={{ animationDelay: "80ms" }}>
        <FinancialTrend className="col-span-12 xl:col-span-8" />
        <CashFlow className="col-span-12 md:col-span-6 xl:col-span-4" />
        <SalesTrend className="col-span-12 md:col-span-6 xl:col-span-6" />
        <ExpenseDonut className="col-span-12 md:col-span-6 xl:col-span-6" />
      </div>

      {/* financial status: receivables / payables */}
      <div className="fade-up grid grid-cols-12 gap-4 2xl:gap-5" style={{ animationDelay: "140ms" }}>
        <div className="col-span-12 lg:col-span-6">
          <AgingPanel
            title="مطالبات"
            subtitle="وضعیت مطالبات از مشتریان"
            aging={data.receivables}
            icon={CircleDollarSign}
          />
        </div>
        <div className="col-span-12 lg:col-span-6">
          <AgingPanel
            title="بدهی‌ها"
            subtitle="وضعیت بدهی به تامین‌کنندگان"
            aging={data.payables}
            icon={Landmark}
          />
        </div>
      </div>

      {/* products & customers */}
      <div className="fade-up grid grid-cols-12 gap-4 2xl:gap-5" style={{ animationDelay: "200ms" }}>
        <TopProducts className="col-span-12 xl:col-span-6" />
        <TopCustomers className="col-span-12 xl:col-span-6" />
      </div>

      <TransactionsTable />
    </div>
  );
}
