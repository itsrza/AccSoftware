import { useMemo, useState } from "react";
import {
  ChevronDown, PanelRightClose, PanelRightOpen, X, Building2, Check, CircleUserRound,
} from "lucide-react";
import { cn } from "../utils/cn";
import { BOTTOM_NAV, NAV_GROUPS, type NavItem } from "../data/navigation";
import { useApp } from "../store/AppContext";
import { BRANCHES } from "../data/engine";
import { Popover } from "./ui";

// ------------------------------------------------------------------ brand
function Brand({ collapsed }: { collapsed: boolean }) {
  return (
    <div className={cn("flex items-center gap-3 px-4 pt-5 pb-4", collapsed && "justify-center px-2")}>
      <div className="relative grid size-11 shrink-0 place-items-center rounded-2xl bg-gradient-to-br from-[#e7bd75] to-[#c8923c] shadow-[0_8px_20px_-6px_rgba(220,167,87,.55)]">
        <svg viewBox="0 0 24 24" className="size-6 text-[#21254E]" fill="currentColor" aria-hidden>
          <path d="M12 2 2.5 9.5 12 22l9.5-12.5L12 2Zm0 3.1 5.4 4.4L12 17.2 6.6 9.5 12 5.1Z" />
        </svg>
      </div>
      {!collapsed && (
        <div className="min-w-0 leading-tight">
          <p className="text-[15px] font-extrabold tracking-tight text-white">نوین پرداز</p>
          <p className="mt-0.5 text-[10px] font-medium text-[var(--sidebar-text)]">
            نرم‌افزار حسابداری یکپارچه
          </p>
        </div>
      )}
    </div>
  );
}

// ------------------------------------------------------ business context
function BranchChip({ collapsed }: { collapsed: boolean }) {
  const { filters, patchFilters } = useApp();
  const current = BRANCHES.find((b) => b.id === filters.branchId);

  return (
    <Popover
      label="انتخاب شعبه"
      width="w-56"
      block={!collapsed}
      trigger={() =>
        collapsed ? (
          <span className="mb-3 grid size-11 cursor-pointer place-items-center rounded-xl border border-[var(--sidebar-border)] bg-white/5 text-[var(--sidebar-text)] transition-colors hover:bg-white/10 hover:text-white">
            <Building2 className="size-5" aria-hidden />
          </span>
        ) : (
          <span className="mb-3 flex w-full cursor-pointer items-center gap-2.5 rounded-xl border border-[var(--sidebar-border)] bg-white/5 px-3 py-2.5 text-start transition-colors hover:bg-white/10">
            <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-[var(--accent-soft)] text-accent">
              <Building2 className="size-4" aria-hidden />
            </span>
            <span className="min-w-0 flex-1 leading-tight">
              <span className="block truncate text-xs font-bold text-white">شرکت نوین پرداز</span>
              <span className="block truncate text-[10px] text-[var(--sidebar-text)]">
                {current?.name ?? "همه شعبه‌ها"}
              </span>
            </span>
            <ChevronDown className="size-3.5 shrink-0 text-[var(--sidebar-text)]" aria-hidden />
          </span>
        )
      }
    >
      {(close) => (
        <div className="p-1">
          <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">انتخاب شعبه فعال</p>
          {[{ id: "all", name: "همه شعبه‌ها" }, ...BRANCHES].map((b) => (
            <button
              key={b.id}
              role="menuitem"
              onClick={() => {
                patchFilters({ branchId: b.id });
                close();
              }}
              className={cn(
                "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-xs transition-colors",
                filters.branchId === b.id
                  ? "bg-[var(--accent-soft)] font-bold text-accent-strong"
                  : "text-muted hover:bg-bg-soft hover:text-text",
              )}
            >
              {b.name}
              {filters.branchId === b.id && <Check className="size-3.5" aria-hidden />}
            </button>
          ))}
        </div>
      )}
    </Popover>
  );
}

// ------------------------------------------------------------------ items
function ItemContent({
  item, active, open, collapsed,
}: { item: NavItem; active: boolean; open?: boolean; collapsed: boolean }) {
  const Icon = item.icon;
  return (
    <>
      <span
        className={cn(
          "relative flex w-full items-center gap-3 rounded-xl py-2.5 text-[12.5px] font-medium transition-all duration-200",
          collapsed ? "justify-center px-0" : "px-3",
          active
            ? "bg-white/10 font-bold text-white shadow-[inset_0_1px_0_rgba(255,255,255,.08)]"
            : "text-[var(--sidebar-text)] hover:bg-white/5 hover:text-white",
        )}
      >
        {active && (
          <span className="absolute inset-y-2 start-0 w-[3px] rounded-full bg-accent" aria-hidden />
        )}
        <Icon
          className={cn("size-[18px] shrink-0 transition-colors", active && "text-accent")}
          aria-hidden
        />
        {!collapsed && <span className="min-w-0 flex-1 truncate text-start">{item.label}</span>}
        {!collapsed && item.children && (
          <ChevronDown
            className={cn("size-3.5 shrink-0 text-[var(--sidebar-text)] transition-transform duration-300", open && "rotate-180")}
            aria-hidden
          />
        )}
        {!collapsed && item.id === "checks" && (
          <span className="grid h-5 min-w-5 place-items-center rounded-full bg-accent px-1 text-[10px] font-bold text-[#21254E]">
            ۴
          </span>
        )}
      </span>
    </>
  );
}

function NavEntry({ item, collapsed }: { item: NavItem; collapsed: boolean }) {
  const { route, navigate } = useApp();
  const [manual, setManual] = useState<boolean | null>(null);
  const hasActiveChild = !!item.children?.some((c) => c.id === route);
  const active = route === item.id || hasActiveChild;
  const open = manual ?? hasActiveChild;

  // ---------- collapsed flyout for children
  if (collapsed && item.children) {
    return (
      <li className="group relative">
        <button
          onClick={() => navigate(item.children![0].id)}
          aria-label={item.label}
          aria-current={active ? "page" : undefined}
          className="w-full"
        >
          <ItemContent item={item} active={active} collapsed />
        </button>
        <div className="invisible absolute end-[calc(100%+10px)] top-0 z-50 w-44 rounded-xl border border-border bg-card p-1.5 opacity-0 shadow-[var(--shadow-lg)] transition-all duration-200 group-hover:visible group-hover:opacity-100">
          <p className="px-2 py-1 text-[10px] font-bold text-faint">{item.label}</p>
          {item.children.map((c) => (
            <button
              key={c.id}
              onClick={() => navigate(c.id)}
              className={cn(
                "block w-full rounded-lg px-2.5 py-2 text-start text-xs transition-colors",
                route === c.id ? "bg-[var(--accent-soft)] font-bold text-accent-strong" : "text-muted hover:bg-bg-soft hover:text-text",
              )}
            >
              {c.label}
            </button>
          ))}
        </div>
      </li>
    );
  }

  // ---------- leaf item
  if (!item.children) {
    return (
      <li className="group relative">
        <button
          onClick={() => navigate(item.id)}
          aria-label={collapsed ? item.label : undefined}
          aria-current={route === item.id ? "page" : undefined}
          className="w-full"
        >
          <ItemContent item={item} active={active} collapsed={collapsed} />
        </button>
        {collapsed && <Tooltip label={item.label} />}
      </li>
    );
  }

  // ---------- expandable group
  return (
    <li>
      <button
        onClick={() => setManual(!open)}
        aria-expanded={open}
        aria-label={item.label}
        className="w-full"
      >
        <ItemContent item={item} active={active} open={open} collapsed={collapsed} />
      </button>
      <div
        className={cn(
          "grid transition-[grid-template-rows] duration-300 ease-out",
          open ? "grid-rows-[1fr]" : "grid-rows-[0fr]",
        )}
      >
        <div className="overflow-hidden">
          <ul className="ms-5 mt-1 space-y-0.5 border-s border-white/10 ps-3 pb-1">
            {item.children.map((c) => (
              <li key={c.id}>
                <button
                  onClick={() => navigate(c.id)}
                  aria-current={route === c.id ? "page" : undefined}
                  className={cn(
                    "relative w-full rounded-lg px-3 py-2 text-start text-[11.5px] transition-colors",
                    route === c.id
                      ? "font-bold text-accent"
                      : "text-[var(--sidebar-text)] hover:text-white",
                  )}
                >
                  <span
                    className={cn(
                      "absolute -start-[13px] top-1/2 h-px w-2.5 bg-white/20",
                      route === c.id && "bg-accent",
                    )}
                    aria-hidden
                  />
                  {c.label}
                </button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </li>
  );
}

function Tooltip({ label }: { label: string }) {
  return (
    <span
      role="tooltip"
      className="pointer-events-none absolute end-[calc(100%+10px)] top-1/2 z-50 -translate-y-1/2 translate-x-1 rounded-lg border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold whitespace-nowrap text-text opacity-0 shadow-[var(--shadow-md)] transition-all duration-150 group-hover:translate-x-0 group-hover:opacity-100"
    >
      {label}
    </span>
  );
}

// ------------------------------------------------------------------ body
function SidebarBody({ collapsed }: { collapsed: boolean }) {
  const { toggleCollapsed, setMobileNav } = useApp();
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="relative">
        <Brand collapsed={collapsed} />
        <button
          onClick={toggleCollapsed}
          aria-label={collapsed ? "باز کردن منو" : "جمع کردن منو"}
          className="absolute top-6 -end-3 hidden size-7 place-items-center rounded-full border border-[var(--sidebar-border)] bg-[#262a58] text-[var(--sidebar-text)] shadow-md transition-colors hover:text-white lg:grid"
        >
          {collapsed ? (
            <PanelRightOpen className="size-3.5" aria-hidden />
          ) : (
            <PanelRightClose className="size-3.5" aria-hidden />
          )}
        </button>
        <button
          onClick={() => setMobileNav(false)}
          aria-label="بستن منو"
          className="absolute top-5 start-3 grid size-8 place-items-center rounded-lg text-[var(--sidebar-text)] transition-colors hover:bg-white/10 hover:text-white lg:hidden"
        >
          <X className="size-4" aria-hidden />
        </button>
      </div>

      <div className={cn(collapsed ? "flex justify-center" : "px-4")}>
        <BranchChip collapsed={collapsed} />
      </div>

      <nav aria-label="منوی اصلی" className="sidebar-scroll min-h-0 flex-1 overflow-y-auto px-3 pb-2">
        {NAV_GROUPS.map((g) => (
          <div key={g.id} className="mt-3 first:mt-0">
            {!collapsed && (
              <p className="px-3 pb-1.5 text-[10px] font-bold tracking-wide text-white/35">
                {g.label}
              </p>
            )}
            {collapsed && <div className="mx-2 my-3 h-px bg-white/8" aria-hidden />}
            <ul className="space-y-0.5">
              {g.items.map((item) => (
                <NavEntry key={item.id} item={item} collapsed={collapsed} />
              ))}
            </ul>
          </div>
        ))}
      </nav>

      {/* bottom */}
      <div className="border-t border-[var(--sidebar-border)] p-3">
        <ul className="space-y-0.5">
          {BOTTOM_NAV.map((item) => (
            <NavEntry key={item.id} item={item} collapsed={collapsed} />
          ))}
        </ul>

        {!collapsed ? (
          <div className="mt-3 flex items-center gap-2.5 rounded-xl border border-[var(--sidebar-border)] bg-white/5 px-3 py-2.5">
            <span className="relative grid size-9 shrink-0 place-items-center rounded-full bg-gradient-to-br from-[#565fa8] to-[#2e3270] text-xs font-extrabold text-white">
              ن‌پ
              <span className="pulse-dot absolute -bottom-0.5 -end-0.5 size-2.5 rounded-full border-2 border-[#1d2046] bg-success" aria-hidden />
            </span>
            <span className="min-w-0 flex-1 leading-tight">
              <span className="block truncate text-xs font-bold text-white">نگار پورحسینی</span>
              <span className="block text-[10px] text-[var(--sidebar-text)]">مدیر مالی · نسخه ۷٫۲</span>
            </span>
            <CircleUserRound className="size-4 shrink-0 text-[var(--sidebar-text)]" aria-hidden />
          </div>
        ) : (
          <div className="mt-3 flex justify-center">
            <span className="relative grid size-9 place-items-center rounded-full bg-gradient-to-br from-[#565fa8] to-[#2e3270] text-xs font-extrabold text-white">
              ن‌پ
              <span className="pulse-dot absolute -bottom-0.5 -end-0.5 size-2.5 rounded-full border-2 border-[#1d2046] bg-success" aria-hidden />
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ export
export function Sidebar() {
  const { collapsed, mobileNav, setMobileNav } = useApp();
  const width = useMemo(() => (collapsed ? "lg:w-[84px]" : "lg:w-[272px]"), [collapsed]);

  return (
    <>
      {/* desktop / tablet */}
      <aside
        className={cn(
          "fixed inset-y-0 start-0 z-40 hidden w-[84px] flex-col border-e border-[var(--sidebar-border)] bg-gradient-to-b from-[var(--sidebar-from)] to-[var(--sidebar-to)] transition-[width] duration-300 lg:flex",
          width,
        )}
      >
        <SidebarBody collapsed={collapsed} />
      </aside>

      {/* mobile drawer */}
      <div
        className={cn(
          "fixed inset-0 z-50 transition-opacity duration-300 lg:hidden",
          mobileNav ? "opacity-100" : "pointer-events-none opacity-0",
        )}
        aria-hidden={!mobileNav}
      >
        <div
          className="absolute inset-0 bg-[#12142e]/60 backdrop-blur-[2px]"
          onClick={() => setMobileNav(false)}
        />
        <aside
          className={cn(
            "absolute inset-y-0 start-0 flex w-[288px] max-w-[85vw] flex-col bg-gradient-to-b from-[var(--sidebar-from)] to-[var(--sidebar-to)] shadow-[var(--shadow-lg)] transition-transform duration-300",
            mobileNav ? "translate-x-0" : "translate-x-full",
          )}
        >
          <SidebarBody collapsed={false} />
        </aside>
      </div>
    </>
  );
}
