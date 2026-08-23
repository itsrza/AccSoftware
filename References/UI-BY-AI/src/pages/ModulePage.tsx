import { useMemo, useState } from "react";
import {
  ArrowLeft, Boxes, LayoutDashboard, Sparkles, TriangleAlert, Server, Database,
  RefreshCw, ShieldCheck, CloudUpload, MessageSquareText, CreditCard, Activity,
} from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import { NAV_GROUPS, BOTTOM_NAV, pageMeta } from "../data/navigation";
import { CHECK_STATUS_LABEL, type CheckStatus } from "../data/engine";
import { faDate, faDateMed, fmt } from "../lib/format";
import { Badge, Card, CardHeader } from "../components/ui";

// find icon for a route id
function routeIcon(id: string) {
  for (const g of NAV_GROUPS) for (const i of g.items) if (i.id === id) return i.icon;
  for (const b of BOTTOM_NAV) if (b.id === id) return b.icon;
  return LayoutDashboard;
}

const STATUS_TONE: Record<CheckStatus, "warning" | "success" | "danger"> = {
  inHand: "warning",
  cashed: "success",
  bounced: "danger",
};

function ChecksBoard() {
  const { db } = useApp();
  const sums = useMemo(() => {
    const inHand = db.checks.filter((c) => c.status === "inHand");
    return {
      inHand: inHand.reduce((a, c) => a + c.amount, 0),
      count: inHand.length,
    };
  }, [db]);

  return (
    <Card pad={false} className="overflow-hidden">
      <div className="p-4 sm:p-5 pb-0 sm:pb-0">
        <CardHeader
          title="چک‌های در گردش"
          subtitle={`${fmt(sums.count)} چک دریافتنی به مبلغ ${fmt(sums.inHand)} تومان`}
        />
      </div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[560px]">
          <thead>
            <tr className="border-b border-border bg-card-soft/60 text-right">
              {["طرف حساب", "شماره صیادی", "سررسید", "مبلغ (تومان)", "وضعیت"].map((h) => (
                <th key={h} scope="col" className="px-4 py-2.5 text-[11px] font-bold text-muted">{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {db.checks.map((c) => (
              <tr key={c.id} className="border-b border-border/60 transition-colors last:border-0 hover:bg-card-soft/80">
                <td className="px-4 py-3 text-[11.5px] font-semibold text-text">{c.partyName}</td>
                <td className="tnum px-4 py-3 text-[11.5px] text-muted">{c.sayyad}</td>
                <td className="tnum px-4 py-3 text-[11.5px] text-muted">{faDate(c.dueDate)}</td>
                <td className="tnum px-4 py-3 text-[12px] font-extrabold text-text">{fmt(c.amount)}</td>
                <td className="px-4 py-3">
                  <Badge tone={STATUS_TONE[c.status]}>{CHECK_STATUS_LABEL[c.status]}</Badge>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

function StockBoard() {
  const { db } = useApp();
  const low = db.products.filter((p) => p.stock < 20);
  return (
    <Card>
      <CardHeader
        title="هشدار موجودی"
        subtitle={`${fmt(low.length)} کالا با موجودی کمتر از ۲۰ عدد`}
        action={<Badge tone="warning" dot={false}>نیاز به شارژ</Badge>}
      />
      <ul className="grid gap-2 sm:grid-cols-2">
        {low.map((p) => (
          <li
            key={p.id}
            className="flex items-center gap-3 rounded-xl border border-border bg-card-soft px-3 py-2.5"
          >
            <span
              className={cn(
                "grid size-9 shrink-0 place-items-center rounded-lg",
                p.stock < 10 ? "bg-[var(--danger-soft)] text-danger" : "bg-[var(--warning-soft)] text-warning",
              )}
            >
              {p.stock < 10 ? <TriangleAlert className="size-4" aria-hidden /> : <Boxes className="size-4" aria-hidden />}
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-xs font-bold text-text">{p.name}</span>
              <span className="tnum block text-[10px] text-muted">
                موجودی {fmt(p.stock)} عدد · فروش کل {fmt(p.sold)} عدد
              </span>
            </span>
          </li>
        ))}
      </ul>
    </Card>
  );
}

function SystemBoard() {
  const { db } = useApp();
  const [smsState, setSmsState] = useState<"down" | "checking" | "up">("down");

  const retry = () => {
    if (smsState === "checking") return;
    setSmsState("checking");
    window.setTimeout(() => setSmsState("up"), 1200);
  };

  const services = [
    { icon: Database, name: "پایگاه داده", desc: `${fmt(db.txs.length)} رکورد تراکنش`, state: "up" as const },
    { icon: Activity, name: "موتور حسابداری", desc: `${fmt(db.checks.length)} چک فعال`, state: "up" as const },
    { icon: CloudUpload, name: "پشتیبان‌گیری ابری", desc: `آخرین نسخه: ${faDateMed(db.lastUpdate)}`, state: "up" as const },
    { icon: CreditCard, name: "سامانه مودیان", desc: "متصل · وضعیت عادی", state: "up" as const },
    {
      icon: MessageSquareText,
      name: "سرویس پیامک",
      desc: smsState === "up" ? "متصل شد · صف ارسال خالی" : smsState === "checking" ? "در حال بررسی اتصال…" : "پاسخ‌گو نیست",
      state: smsState,
    },
  ];

  return (
    <Card>
      <CardHeader
        title="سلامت سرویس‌ها"
        subtitle="پایش لحظه‌ای زیرساخت نرم‌افزار"
        action={<Badge tone="success">هسته سیستم پایدار است</Badge>}
      />
      <ul className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
        {services.map((s) => (
          <li
            key={s.name}
            className="flex items-center gap-3 rounded-xl border border-border bg-card-soft px-3.5 py-3"
          >
            <span
              className={cn(
                "grid size-10 shrink-0 place-items-center rounded-xl",
                s.state === "up" && "bg-[var(--success-soft)] text-success",
                s.state === "down" && "bg-[var(--danger-soft)] text-danger",
                s.state === "checking" && "bg-[var(--warning-soft)] text-warning",
              )}
            >
              <s.icon className={cn("size-5", s.state === "checking" && "animate-spin")} aria-hidden />
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-center gap-2 text-xs font-bold text-text">
                {s.name}
                <span
                  className={cn(
                    "size-1.5 rounded-full",
                    s.state === "up" && "pulse-dot bg-success",
                    s.state === "down" && "bg-danger",
                    s.state === "checking" && "bg-warning",
                  )}
                  aria-hidden
                />
              </span>
              <span className="mt-0.5 block truncate text-[10.5px] text-muted">{s.desc}</span>
            </span>
            {s.state !== "up" && (
              <button
                onClick={retry}
                disabled={s.state === "checking"}
                className="inline-flex shrink-0 items-center gap-1 rounded-lg bg-primary px-2.5 py-1.5 text-[10px] font-bold text-[#f2f3fc] transition-all hover:scale-105 active:scale-95 disabled:opacity-60 dark:bg-accent dark:text-[#241c3d]"
              >
                <RefreshCw className={cn("size-3", s.state === "checking" && "animate-spin")} aria-hidden />
                {s.state === "checking" ? "بررسی…" : "تلاش مجدد"}
              </button>
            )}
          </li>
        ))}
      </ul>
      <p className="mt-4 flex items-center gap-2 rounded-xl bg-bg-soft px-3.5 py-2.5 text-[10.5px] text-muted">
        <ShieldCheck className="size-4 shrink-0 text-success" aria-hidden />
        <Server className="size-4 shrink-0 text-faint" aria-hidden />
        نسخه نصب‌شده ۷٫۲٫۱ · به‌روزرسانی خودکار فعال است · پایگاه داده محلی رمزنگاری می‌شود.
      </p>
    </Card>
  );
}

export function ModulePage() {
  const { route, navigate } = useApp();
  const meta = pageMeta(route);
  const Icon = routeIcon(route);

  return (
    <div className="mx-auto flex w-full max-w-[1720px] flex-col gap-4 p-3 sm:p-5 2xl:gap-5">
      {/* module hero */}
      <Card className="relative overflow-hidden">
        <div
          className="pointer-events-none absolute -end-16 -top-24 size-72 rounded-full opacity-10 blur-3xl"
          style={{ background: "var(--accent)" }}
          aria-hidden
        />
        <div className="flex flex-wrap items-center gap-4">
          <span className="grid size-14 shrink-0 place-items-center rounded-2xl bg-primary text-accent shadow-[var(--shadow-md)] dark:bg-accent dark:text-[#241c3d]">
            <Icon className="size-7" aria-hidden />
          </span>
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-extrabold text-text sm:text-xl">{meta.title}</h2>
            <p className="mt-1 text-xs text-muted">
              {meta.desc || "زیرسیستم نرم‌افزار حسابداری نوین پرداز"} · گروه «{meta.groupLabel}»
            </p>
          </div>
          <button
            onClick={() => navigate("dashboard")}
            className="inline-flex items-center gap-2 rounded-xl border border-border bg-card px-4 py-2.5 text-xs font-bold text-muted transition-all hover:border-border-strong hover:text-text active:scale-95"
          >
            <LayoutDashboard className="size-4" aria-hidden />
            بازگشت به داشبورد
          </button>
        </div>
      </Card>

      {route === "checks" && <ChecksBoard />}
      {route === "inventory" && <StockBoard />}
      {route === "system" && <SystemBoard />}

      {/* workspace note */}
      <Card>
        <div className="flex flex-col items-start gap-3 sm:flex-row sm:items-center">
          <span className="grid size-11 shrink-0 place-items-center rounded-xl bg-[var(--accent-soft)] text-accent-strong">
            <Sparkles className="size-5" aria-hidden />
          </span>
          <div className="min-w-0 flex-1">
            <p className="text-sm font-bold text-text">میز کار «{meta.title}»</p>
            <p className="mt-1 text-xs leading-6 text-muted">
              این بخش به داده‌های زنده حسابداری متصل است. برای تحلیل لحظه‌ای شاخص‌ها، به داشبورد
              بازگردید یا از اقدام‌های سریع سربرگ استفاده کنید.
            </p>
          </div>
          <button
            onClick={() => navigate("dashboard")}
            className="inline-flex shrink-0 items-center gap-1.5 rounded-xl bg-primary px-4 py-2.5 text-xs font-bold text-[#f2f3fc] transition-transform hover:scale-[1.03] active:scale-95 dark:bg-accent dark:text-[#241c3d]"
          >
            مشاهده تحلیل مالی
            <ArrowLeft className="size-4" aria-hidden />
          </button>
        </div>
      </Card>

      {/* related shortcuts */}
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {[
          { id: "quick-invoice", label: "صدور فاکتور فروش" },
          { id: "treasury", label: "خزانه‌داری" },
          { id: "reports", label: "گزارشات" },
          { id: "settings", label: "تنظیمات برنامه" },
        ]
          .filter((l) => l.id !== route)
          .slice(0, 4)
          .map((l) => {
            const LIcon = routeIcon(l.id);
            const m = pageMeta(l.id);
            return (
              <button
                key={l.id}
                onClick={() => navigate(l.id)}
                className="group flex items-center gap-3 rounded-[var(--radius)] border border-border bg-card p-4 text-start shadow-[var(--shadow-sm)] transition-all hover:-translate-y-0.5 hover:shadow-[var(--shadow-md)]"
              >
                <span className="grid size-10 shrink-0 place-items-center rounded-xl bg-bg-soft text-muted transition-colors group-hover:bg-[var(--accent-soft)] group-hover:text-accent-strong">
                  <LIcon className="size-5" aria-hidden />
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-xs font-bold text-text">{l.label}</span>
                  <span className="block truncate text-[10px] text-muted">{m.desc}</span>
                </span>
                <ArrowLeft className="ms-auto size-4 shrink-0 text-faint transition-transform group-hover:-translate-x-1 group-hover:text-accent" aria-hidden />
              </button>
            );
          })}
      </div>
    </div>
  );
}
