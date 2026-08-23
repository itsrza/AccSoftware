import { useMemo, useRef, useState } from "react";
import {
  Menu, Search, Bell, Sun, Moon, Plus, ChevronDown, CalendarDays, X,
  Zap, HandCoins, Rows3, ScrollText, CheckCheck, LogOut, Settings, CircleUserRound,
} from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import { pageMeta } from "../data/navigation";
import { Popover } from "./ui";
import { PRESETS, faDateFull, fmt, fmtCompact, type PresetId } from "../lib/format";
import { TX_TYPE_LABEL } from "../data/engine";

// ------------------------------------------------------------- global search
function GlobalSearch() {
  const { db, navigate } = useApp();
  const [q, setQ] = useState("");
  const [focus, setFocus] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const results = useMemo(() => {
    const query = q.trim();
    if (query.length < 2) return null;
    const customers = db.customers.filter((c) => c.name.includes(query)).slice(0, 4);
    const products = db.products.filter((p) => p.name.includes(query)).slice(0, 4);
    const txs = db.txs.filter((t) => t.doc.includes(query) || t.partyName.includes(query)).slice(0, 4);
    return { customers, products, txs, empty: !customers.length && !products.length && !txs.length };
  }, [q, db]);

  const go = (route: string) => {
    setQ("");
    setMobileOpen(false);
    navigate(route);
  };

  const field = (
    <div className="relative w-full">
      <Search className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-faint" aria-hidden />
      <input
        ref={inputRef}
        value={q}
        onChange={(e) => setQ(e.target.value)}
        onFocus={() => setFocus(true)}
        onBlur={() => setTimeout(() => setFocus(false), 180)}
        onKeyDown={(e) => e.key === "Escape" && (setQ(""), setMobileOpen(false))}
        type="search"
        placeholder="جستجو: مشتری، کالا، شماره سند…"
        aria-label="جستجوی سراسری"
        className="h-10 w-full rounded-xl border border-border bg-bg-soft ps-9 pe-9 text-xs font-medium text-text placeholder:text-faint outline-none transition-all focus:border-accent focus:bg-card"
      />
      {q && (
        <button
          aria-label="پاک کردن جستجو"
          onClick={() => setQ("")}
          className="absolute end-2.5 top-1/2 grid size-5 -translate-y-1/2 place-items-center rounded-full text-faint hover:bg-bg-soft hover:text-text"
        >
          <X className="size-3.5" aria-hidden />
        </button>
      )}

      {focus && results && (
        <div className="fade-up absolute top-[calc(100%+8px)] z-50 w-full min-w-72 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          {results.empty ? (
            <p className="px-3 py-6 text-center text-xs text-muted">
              نتیجه‌ای برای «{q}» یافت نشد.
            </p>
          ) : (
            <div className="max-h-80 overflow-y-auto">
              {results.txs.length > 0 && (
                <Group title="اسناد و تراکنش‌ها">
                  {results.txs.map((t) => (
                    <Row
                      key={t.id}
                      title={`${t.doc} — ${t.partyName}`}
                      meta={`${TX_TYPE_LABEL[t.type]} · ${fmtCompact(t.amount)} تومان`}
                      onClick={() => {
                        window.dispatchEvent(new CustomEvent("np-table-search", { detail: t.doc }));
                        go("dashboard");
                      }}
                    />
                  ))}
                </Group>
              )}
              {results.customers.length > 0 && (
                <Group title="اشخاص">
                  {results.customers.map((c) => (
                    <Row key={c.id} title={c.name} meta={c.city} onClick={() => go("persons")} />
                  ))}
                </Group>
              )}
              {results.products.length > 0 && (
                <Group title="کالاها">
                  {results.products.map((p) => (
                    <Row
                      key={p.id}
                      title={p.name}
                      meta={`موجودی: ${fmt(p.stock)}`}
                      onClick={() => go("inventory")}
                    />
                  ))}
                </Group>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );

  return (
    <>
      <div className="hidden w-full max-w-md flex-1 md:block">{field}</div>
      <button
        className="grid size-10 place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text md:hidden"
        aria-label="جستجو"
        onClick={() => setMobileOpen((o) => !o)}
      >
        <Search className="size-4.5" aria-hidden />
      </button>
      {mobileOpen && (
        <div className="fade-up absolute inset-x-3 top-[calc(100%+6px)] z-50 md:hidden">{field}</div>
      )}
    </>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="py-1">
      <p className="px-2.5 py-1 text-[10px] font-bold text-faint">{title}</p>
      {children}
    </div>
  );
}
function Row({ title, meta, onClick }: { title: string; meta: string; onClick: () => void }) {
  return (
    <button
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      className="flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-start transition-colors hover:bg-bg-soft"
    >
      <span className="min-w-0 truncate text-xs font-semibold text-text">{title}</span>
      <span className="shrink-0 text-[10px] text-faint">{meta}</span>
    </button>
  );
}

// ------------------------------------------------------------- notifications
function Notifications() {
  const { db, data, navigate } = useApp();
  const [read, setRead] = useState(false);

  const items = useMemo(() => {
    const now = Date.now();
    const soon = db.checks.filter(
      (c) => c.status === "inHand" && c.dueDate > now && c.dueDate < now + 8 * 86_400_000,
    );
    const notifs = [
      ...soon.slice(0, 3).map((c) => ({
        id: c.id,
        tone: "warning" as const,
        title: `چک ${c.partyName} سررسید نزدیک است`,
        meta: `${fmtCompact(c.amount)} تومان · صیادی ${c.sayyad}`,
        route: "checks",
      })),
      ...(data.receivables.buckets[2].count > 0
        ? [{
            id: "overdue",
            tone: "danger" as const,
            title: `${new Intl.NumberFormat("fa-IR").format(data.receivables.buckets[2].count)} سند مطالبات سررسید گذشته`,
            meta: `${fmtCompact(data.receivables.buckets[2].amount)} تومان نیاز به پیگیری`,
            route: "treasury",
          }]
        : []),
      ...db.products
        .filter((p) => p.stock < 12)
        .slice(0, 2)
        .map((p) => ({
          id: `low-${p.id}`,
          tone: "info" as const,
          title: `موجودی «${p.name}» رو به اتمام است`,
          meta: `تنها ${fmt(p.stock)} عدد باقی مانده`,
          route: "inventory",
        })),
    ];
    return notifs;
  }, [db, data]);

  const count = read ? 0 : items.length;

  return (
    <Popover
      label="اعلان‌ها"
      width="w-80"
      align="end"
      trigger={(open) => (
        <span
          className={cn(
            "relative grid size-10 cursor-pointer place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text",
            open && "border-accent text-text",
          )}
        >
          <Bell className="size-4.5" aria-hidden />
          {count > 0 && (
            <span className="absolute -top-1.5 -start-1.5 grid h-4.5 min-w-4.5 place-items-center rounded-full bg-danger px-1 text-[9px] font-bold text-white">
              {new Intl.NumberFormat("fa-IR").format(count)}
            </span>
          )}
        </span>
      )}
    >
      {(close) => (
        <div>
          <div className="flex items-center justify-between px-2 pb-1 pt-1">
            <p className="text-xs font-bold text-text">اعلان‌ها</p>
            <button
              onClick={() => setRead(true)}
              className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] font-semibold text-muted transition-colors hover:bg-bg-soft hover:text-text"
            >
              <CheckCheck className="size-3" aria-hidden />
              خواندم
            </button>
          </div>
          <div className="max-h-80 space-y-1 overflow-y-auto">
            {items.length === 0 && (
              <p className="px-3 py-6 text-center text-xs text-muted">اعلان جدیدی نیست.</p>
            )}
            {items.map((n) => (
              <button
                key={n.id}
                onClick={() => {
                  navigate(n.route);
                  close();
                }}
                className="flex w-full items-start gap-2.5 rounded-xl px-2.5 py-2.5 text-start transition-colors hover:bg-bg-soft"
              >
                <span
                  className={cn(
                    "mt-1.5 size-2 shrink-0 rounded-full",
                    n.tone === "warning" && "bg-warning",
                    n.tone === "danger" && "bg-danger",
                    n.tone === "info" && "bg-info",
                  )}
                  aria-hidden
                />
                <span className="min-w-0">
                  <span className={cn("block truncate text-xs text-text", !read && "font-bold")}>{n.title}</span>
                  <span className="block text-[10px] text-muted">{n.meta}</span>
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </Popover>
  );
}

// ------------------------------------------------------------- user menu
function UserMenu() {
  const { navigate, theme, toggleTheme, signOut } = useApp();
  return (
    <Popover
      label="منوی کاربر"
      align="end"
      width="w-64"
      trigger={() => (
        <span className="flex cursor-pointer items-center gap-2.5 rounded-xl border border-border bg-card py-1.5 ps-1.5 pe-2.5 transition-colors hover:border-border-strong">
          <span className="grid size-7 place-items-center rounded-lg bg-primary text-[11px] font-extrabold text-[#f2f3fc] dark:bg-accent dark:text-[#241c3d]">
            ن‌پ
          </span>
          <span className="hidden text-start leading-tight xl:block">
            <span className="block text-[11px] font-bold text-text">نگار پورحسینی</span>
            <span className="block text-[9.5px] text-faint">مدیر مالی</span>
          </span>
          <ChevronDown className="size-3.5 text-faint" aria-hidden />
        </span>
      )}
    >
      {(close) => (
        <div>
          <div className="mb-1 flex items-center gap-3 rounded-xl bg-bg-soft p-3">
            <span className="grid size-10 place-items-center rounded-xl bg-primary text-sm font-extrabold text-[#f2f3fc] dark:bg-accent dark:text-[#241c3d]">
              ن‌پ
            </span>
            <div className="leading-tight">
              <p className="text-xs font-bold text-text">نگار پورحسینی</p>
              <p className="text-[10px] text-muted">negar@novinpardaz.ir</p>
            </div>
          </div>
          {[
            { icon: CircleUserRound, label: "پروفایل و نقش‌ها", fn: () => navigate("persons-admin") },
            { icon: theme === "dark" ? Sun : Moon, label: theme === "dark" ? "تم روشن" : "تم تیره", fn: toggleTheme },
            { icon: Settings, label: "تنظیمات برنامه", fn: () => navigate("settings") },
          ].map((i) => (
            <button
              key={i.label}
              onClick={() => {
                i.fn();
                close();
              }}
              className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-medium text-muted transition-colors hover:bg-bg-soft hover:text-text"
            >
              <i.icon className="size-4" aria-hidden />
              {i.label}
            </button>
          ))}
          <div className="my-1 h-px bg-border" />
          <button
            onClick={() => {
              close();
              signOut();
            }}
            className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-xs font-semibold text-danger transition-colors hover:bg-[var(--danger-soft)]"
          >
            <LogOut className="size-4" aria-hidden />
            خروج از حساب
          </button>
        </div>
      )}
    </Popover>
  );
}

// ------------------------------------------------------------- header
export function Header() {
  const { setMobileNav, route, theme, toggleTheme, filters, setPreset, navigate } = useApp();
  const meta = pageMeta(route);

  return (
    <header className="sticky top-0 z-30 border-b border-border bg-[color-mix(in_srgb,var(--card)_82%,transparent)] backdrop-blur-xl">
      <div className="relative flex h-16 items-center gap-2.5 px-3 sm:px-5">
        <button
          className="grid size-10 place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text lg:hidden"
          aria-label="باز کردن منو"
          onClick={() => setMobileNav(true)}
        >
          <Menu className="size-5" aria-hidden />
        </button>

        {/* title + breadcrumb */}
        <div className="min-w-0 shrink-0">
          <h1 className="truncate text-sm font-extrabold text-text sm:text-[15px]">{meta.title}</h1>
          <nav aria-label="مسیر" className="hidden text-[10.5px] text-faint sm:block">
            نوین پرداز <span className="mx-1">/</span> {meta.groupLabel}
            {meta.groupLabel !== meta.title && (
              <>
                <span className="mx-1">/</span>
                <span className="text-muted">{meta.title}</span>
              </>
            )}
          </nav>
        </div>

        <GlobalSearch />

        <div className="ms-auto flex items-center gap-2">
          {/* compact date-range in header */}
          <Popover
            label="بازه زمانی"
            align="end"
            width="w-52"
            trigger={() => (
              <span className="hidden h-10 cursor-pointer items-center gap-2 rounded-xl border border-border bg-card px-3 text-xs font-semibold text-muted transition-colors hover:text-text sm:inline-flex">
                <CalendarDays className="size-4 text-accent" aria-hidden />
                {PRESETS.find((p) => p.id === filters.range.preset)?.label}
                <ChevronDown className="size-3.5 text-faint" aria-hidden />
              </span>
            )}
          >
            {(close) => (
              <div className="p-1">
                {PRESETS.filter((p) => p.id !== "custom").map((p) => (
                  <button
                    key={p.id}
                    onClick={() => {
                      setPreset(p.id as PresetId);
                      close();
                    }}
                    className={cn(
                      "block w-full rounded-lg px-3 py-2 text-start text-xs transition-colors",
                      filters.range.preset === p.id
                        ? "bg-[var(--accent-soft)] font-bold text-accent-strong"
                        : "text-muted hover:bg-bg-soft hover:text-text",
                    )}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            )}
          </Popover>

          {/* quick actions */}
          <Popover
            label="اقدام سریع"
            align="end"
            width="w-56"
            trigger={() => (
              <span className="inline-flex h-10 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3 text-xs font-bold text-[#f2f3fc] shadow-[var(--shadow-sm)] transition-transform hover:scale-[1.03] active:scale-95 dark:bg-accent dark:text-[#241c3d]">
                <Plus className="size-4" aria-hidden />
                <span className="hidden sm:inline">اقدام سریع</span>
              </span>
            )}
          >
            {(close) => (
              <div className="p-1">
                {[
                  { icon: Zap, label: "فاکتور فروش جدید", route: "quick-invoice" },
                  { icon: HandCoins, label: "دریافت وجه", route: "treasury" },
                  { icon: ScrollText, label: "ثبت چک", route: "checks" },
                  { icon: Rows3, label: "سند یکسطری", route: "simple-doc" },
                ].map((a) => (
                  <button
                    key={a.label}
                    onClick={() => {
                      navigate(a.route);
                      close();
                    }}
                    className="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2.5 text-xs font-semibold text-muted transition-colors hover:bg-[var(--accent-soft)] hover:text-accent-strong"
                  >
                    <a.icon className="size-4 text-accent" aria-hidden />
                    {a.label}
                  </button>
                ))}
              </div>
            )}
          </Popover>

          <Notifications />

          <button
            onClick={toggleTheme}
            aria-label={theme === "dark" ? "تغییر به تم روشن" : "تغییر به تم تیره"}
            className="grid size-10 place-items-center rounded-xl border border-border bg-card text-muted transition-all hover:rotate-12 hover:text-accent"
          >
            {theme === "dark" ? <Sun className="size-4.5" aria-hidden /> : <Moon className="size-4.5" aria-hidden />}
          </button>

          <UserMenu />
        </div>
      </div>
      <p className="border-t border-border px-5 py-1 text-[10px] text-faint lg:hidden">
        {faDateFull(new Date())}
      </p>
    </header>
  );
}
