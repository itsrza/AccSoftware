import { useEffect, useMemo, useState } from "react";
import {
  ArrowDown, ArrowUp, ArrowUpDown, ChevronLeft, ChevronRight, Search, X,
  ShoppingCart, ShoppingBag, Wallet, CreditCard, Banknote, ArrowLeftRight,
  type LucideIcon,
} from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import {
  TX_STATUS_LABEL, TX_TYPE_LABEL, type Tx, type TxStatus, type TxType,
} from "../data/engine";
import { faDate, fmt } from "../lib/format";
import { Badge, Card, CardHeader, EmptyState, Select, Skeleton } from "./ui";

const TYPE_META: Record<TxType, { icon: LucideIcon; tone: "info" | "warning" | "success" | "danger" | "accent" | "neutral" }> = {
  sale: { icon: ShoppingCart, tone: "info" },
  purchase: { icon: ShoppingBag, tone: "neutral" },
  receipt: { icon: Wallet, tone: "success" },
  payment: { icon: CreditCard, tone: "danger" },
  expense: { icon: Banknote, tone: "warning" },
  transfer: { icon: ArrowLeftRight, tone: "accent" },
};

const STATUS_TONE: Record<TxStatus, "success" | "warning" | "danger" | "neutral"> = {
  settled: "success",
  pending: "warning",
  due: "danger",
  cancelled: "neutral",
};

const PAGE_SIZE = 8;
type SortKey = "date" | "amount" | "doc";

export function TransactionsTable() {
  const { data, loading } = useApp();
  const [q, setQ] = useState("");
  const [type, setType] = useState("all");
  const [status, setStatus] = useState("all");
  const [sortKey, setSortKey] = useState<SortKey>("date");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [page, setPage] = useState(0);

  // header global search can push a query here
  useEffect(() => {
    const handler = (e: Event) => {
      setQ(String((e as CustomEvent).detail ?? ""));
      setPage(0);
    };
    window.addEventListener("np-table-search", handler);
    return () => window.removeEventListener("np-table-search", handler);
  }, []);

  const rows = useMemo(() => {
    let list = data.filtered;
    const query = q.trim();
    if (query) {
      list = list.filter((t) => t.partyName.includes(query) || t.doc.includes(query) || TX_TYPE_LABEL[t.type].includes(query));
    }
    if (type !== "all") list = list.filter((t) => t.type === type);
    if (status !== "all") list = list.filter((t) => t.status === status);

    const sorted = [...list].sort((a, b) => {
      const k = sortKey === "doc" ? "doc" : sortKey;
      const va = k === "doc" ? a.doc : (a[k] as number);
      const vb = k === "doc" ? b.doc : (b[k] as number);
      if (va < vb) return sortDir === "asc" ? -1 : 1;
      if (va > vb) return sortDir === "asc" ? 1 : -1;
      return 0;
    });
    return sorted;
  }, [data.filtered, q, type, status, sortKey, sortDir]);

  const pageCount = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const pageRows = rows.slice(safePage * PAGE_SIZE, safePage * PAGE_SIZE + PAGE_SIZE);

  const toggleSort = (k: SortKey) => {
    if (sortKey === k) setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    else {
      setSortKey(k);
      setSortDir("desc");
    }
  };

  const Th = ({ id, label, align = "start" }: { id: SortKey; label: string; align?: "start" | "end" }) => (
    <th
      scope="col"
      aria-sort={sortKey === id ? (sortDir === "asc" ? "ascending" : "descending") : undefined}
      className={cn("px-3 py-2.5 font-bold", align === "end" && "text-left")}
    >
      <button
        onClick={() => toggleSort(id)}
        className={cn(
          "inline-flex items-center gap-1 text-[11px] text-muted transition-colors hover:text-text",
          sortKey === id && "text-accent-strong dark:text-accent",
        )}
      >
        {label}
        {sortKey === id ? (
          sortDir === "asc" ? <ArrowUp className="size-3" aria-hidden /> : <ArrowDown className="size-3" aria-hidden />
        ) : (
          <ArrowUpDown className="size-3 opacity-50" aria-hidden />
        )}
      </button>
    </th>
  );

  const hasFilter = q || type !== "all" || status !== "all";

  return (
    <Card pad={false} className="overflow-hidden">
      <div className="p-4 pb-0 sm:p-5 sm:pb-0">
        <CardHeader
          title="تراکنش‌های اخیر"
          subtitle={`${fmt(rows.length)} تراکنش در بازه و فیلترهای انتخابی`}
          action={
            <div className="flex flex-wrap items-center gap-2">
              <label className="relative">
                <span className="sr-only">جستجوی تراکنش</span>
                <Search className="pointer-events-none absolute start-3 top-1/2 size-3.5 -translate-y-1/2 text-faint" aria-hidden />
                <input
                  value={q}
                  onChange={(e) => {
                    setQ(e.target.value);
                    setPage(0);
                  }}
                  placeholder="طرف حساب یا شماره سند…"
                  className="h-9 w-40 rounded-xl border border-border bg-card ps-8 pe-7 text-[11px] text-text placeholder:text-faint outline-none transition-colors focus:border-accent sm:w-48"
                />
                {q && (
                  <button
                    aria-label="پاک کردن"
                    onClick={() => setQ("")}
                    className="absolute end-2 top-1/2 -translate-y-1/2 text-faint hover:text-text"
                  >
                    <X className="size-3.5" aria-hidden />
                  </button>
                )}
              </label>
              <Select
                label="نوع تراکنش"
                value={type}
                onChange={(v) => {
                  setType(v);
                  setPage(0);
                }}
                options={[
                  { value: "all", label: "همه انواع" },
                  ...Object.entries(TX_TYPE_LABEL).map(([value, label]) => ({ value, label })),
                ]}
                className="w-32"
              />
              <Select
                label="وضعیت"
                value={status}
                onChange={(v) => {
                  setStatus(v);
                  setPage(0);
                }}
                options={[
                  { value: "all", label: "همه وضعیت‌ها" },
                  ...Object.entries(TX_STATUS_LABEL).map(([value, label]) => ({ value, label })),
                ]}
                className="w-32"
              />
            </div>
          }
        />
      </div>

      {loading ? (
        <div className="space-y-2 p-5 pt-2">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-11 w-full" />
          ))}
        </div>
      ) : pageRows.length === 0 ? (
        <div className="p-5 pt-2">
          <EmptyState
            title={hasFilter ? "تراکنشی با این مشخصات یافت نشد." : "تراکنشی در این بازه ثبت نشده است."}
            action={
              hasFilter ? (
                <button
                  onClick={() => {
                    setQ("");
                    setType("all");
                    setStatus("all");
                  }}
                  className="mt-2 rounded-xl bg-primary px-4 py-2 text-xs font-bold text-[#f2f3fc] transition-transform hover:scale-[1.03] active:scale-95 dark:bg-accent dark:text-[#241c3d]"
                >
                  حذف فیلترها
                </button>
              ) : undefined
            }
          />
        </div>
      ) : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full min-w-[640px] border-collapse text-start">
              <thead>
                <tr className="border-b border-border bg-card-soft/60 text-start">
                  <Th id="date" label="تاریخ" />
                  <Th id="doc" label="شماره سند" />
                  <th scope="col" className="px-3 py-2.5 text-[11px] font-bold text-muted">نوع تراکنش</th>
                  <th scope="col" className="px-3 py-2.5 text-[11px] font-bold text-muted">طرف حساب</th>
                  <Th id="amount" label="مبلغ (تومان)" align="end" />
                  <th scope="col" className="px-3 py-2.5 text-[11px] font-bold text-muted">وضعیت</th>
                </tr>
              </thead>
              <tbody>
                {pageRows.map((t: Tx) => {
                  const meta = TYPE_META[t.type];
                  const Icon = meta.icon;
                  const moneyIn = t.type === "receipt" || t.type === "sale";
                  return (
                    <tr
                      key={t.id}
                      className="border-b border-border/60 transition-colors last:border-0 hover:bg-card-soft/80"
                    >
                      <td className="tnum whitespace-nowrap px-3 py-3 text-[11.5px] text-muted">
                        {faDate(t.date)}
                      </td>
                      <td className="px-3 py-3 text-[11.5px] font-bold text-text">{t.doc}</td>
                      <td className="px-3 py-3">
                        <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-text">
                          <span
                            className={cn(
                              "grid size-6 place-items-center rounded-lg",
                              meta.tone === "info" && "bg-[var(--info-soft)] text-info",
                              meta.tone === "success" && "bg-[var(--success-soft)] text-success",
                              meta.tone === "danger" && "bg-[var(--danger-soft)] text-danger",
                              meta.tone === "warning" && "bg-[var(--warning-soft)] text-warning",
                              meta.tone === "accent" && "bg-[var(--accent-soft)] text-accent-strong",
                              meta.tone === "neutral" && "bg-bg-soft text-muted",
                            )}
                          >
                            <Icon className="size-3" aria-hidden />
                          </span>
                          {TX_TYPE_LABEL[t.type]}
                        </span>
                      </td>
                      <td className="max-w-44 truncate px-3 py-3 text-[11.5px] text-muted">{t.partyName}</td>
                      <td
                        className={cn(
                          "tnum whitespace-nowrap px-3 py-3 text-left text-[12px] font-extrabold",
                          t.type === "receipt" && "text-success",
                          (t.type === "payment" || t.type === "expense") && "text-danger",
                          !moneyIn && t.type !== "payment" && t.type !== "expense" && "text-text",
                          t.type === "sale" && "text-text",
                        )}
                      >
                        {t.type === "receipt" ? "+" : t.type === "payment" || t.type === "expense" ? "−" : ""}
                        {fmt(t.amount)}
                      </td>
                      <td className="px-3 py-3">
                        <Badge tone={STATUS_TONE[t.status]}>{TX_STATUS_LABEL[t.status]}</Badge>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* pagination */}
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border px-4 py-3 sm:px-5">
            <p className="tnum text-[10.5px] text-faint">
              نمایش {fmt(safePage * PAGE_SIZE + 1)} تا {fmt(Math.min(rows.length, safePage * PAGE_SIZE + PAGE_SIZE))} از {fmt(rows.length)} تراکنش
            </p>
            <div className="flex items-center gap-1.5">
              <button
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={safePage === 0}
                aria-label="صفحه قبل"
                className="grid size-8 place-items-center rounded-lg border border-border text-muted transition-colors hover:bg-card-soft hover:text-text disabled:opacity-40"
              >
                <ChevronRight className="size-4" aria-hidden />
              </button>
              {Array.from({ length: pageCount }).map((_, i) => {
                if (pageCount > 6 && Math.abs(i - safePage) > 1 && i !== 0 && i !== pageCount - 1) {
                  if (Math.abs(i - safePage) === 2)
                    return <span key={i} className="px-0.5 text-[10px] text-faint">…</span>;
                  return null;
                }
                return (
                  <button
                    key={i}
                    onClick={() => setPage(i)}
                    aria-label={`صفحه ${fmt(i + 1)}`}
                    aria-current={i === safePage ? "page" : undefined}
                    className={cn(
                      "tnum grid size-8 place-items-center rounded-lg border text-[11px] font-bold transition-colors",
                      i === safePage
                        ? "border-primary bg-primary text-[#f2f3fc] dark:border-accent dark:bg-accent dark:text-[#241c3d]"
                        : "border-border text-muted hover:bg-card-soft hover:text-text",
                    )}
                  >
                    {fmt(i + 1)}
                  </button>
                );
              })}
              <button
                onClick={() => setPage((p) => Math.min(pageCount - 1, p + 1))}
                disabled={safePage >= pageCount - 1}
                aria-label="صفحه بعد"
                className="grid size-8 place-items-center rounded-lg border border-border text-muted transition-colors hover:bg-card-soft hover:text-text disabled:opacity-40"
              >
                <ChevronLeft className="size-4" aria-hidden />
              </button>
            </div>
          </div>
        </>
      )}
    </Card>
  );
}
