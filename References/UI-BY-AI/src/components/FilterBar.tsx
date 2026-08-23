import { useEffect, useState } from "react";
import { CalendarRange, RotateCcw, Check } from "lucide-react";
import { cn } from "../utils/cn";
import { useApp } from "../store/AppContext";
import { Popover, Select } from "./ui";
import { PRESETS, faDate, rangeLabel, type PresetId } from "../lib/format";
import {
  ACCOUNTS, BRANCHES, EXPENSE_CATEGORIES, PRODUCT_CATEGORIES, TX_TYPE_LABEL, USERS,
} from "../data/engine";

const toInput = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

export function FilterBar() {
  const { filters, setPreset, setCustomRange, patchFilters, resetFilters } = useApp();
  const [cFrom, setCFrom] = useState(toInput(filters.range.from));
  const [cTo, setCTo] = useState(toInput(filters.range.to));

  // keep custom inputs in sync when another preset is chosen elsewhere
  useEffect(() => {
    setCFrom(toInput(filters.range.from));
    setCTo(toInput(filters.range.to));
  }, [filters.range.from, filters.range.to]);

  const isDefault =
    filters.branchId === "all" &&
    filters.accountId === "all" &&
    filters.categoryId === "all" &&
    filters.txType === "all" &&
    filters.userId === "all" &&
    filters.range.preset === "thisMonth";

  const categories = [
    ...PRODUCT_CATEGORIES.map((c) => ({ value: c.id, label: `کالا: ${c.name}` })),
    ...EXPENSE_CATEGORIES.map((c) => ({ value: c.id, label: `هزینه: ${c.name}` })),
  ];

  return (
    <section
      aria-label="فیلترهای سراسری"
      className="fade-up rounded-[var(--radius)] border border-border bg-card p-3 shadow-[var(--shadow-sm)] sm:p-4"
    >
      {/* date presets */}
      <div className="flex items-center gap-2 overflow-x-auto pb-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        <span className="inline-flex shrink-0 items-center gap-1.5 text-[11px] font-bold text-muted">
          <CalendarRange className="size-4 text-accent" aria-hidden />
          بازه زمانی:
        </span>
        {PRESETS.filter((p) => p.id !== "custom").map((p) => (
          <button
            key={p.id}
            onClick={() => setPreset(p.id as PresetId)}
            aria-pressed={filters.range.preset === p.id}
            className={cn(
              "shrink-0 rounded-full border px-3 py-1.5 text-[11px] font-semibold transition-all",
              filters.range.preset === p.id
                ? "border-primary bg-primary text-[#f2f3fc] shadow-[var(--shadow-sm)] dark:border-accent dark:bg-accent dark:text-[#241c3d]"
                : "border-border bg-card text-muted hover:border-border-strong hover:text-text",
            )}
          >
            {p.label}
          </button>
        ))}

        <Popover
          label="بازه سفارشی"
          width="w-72"
          trigger={() => (
            <span
              className={cn(
                "inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1.5 text-[11px] font-semibold transition-all",
                filters.range.preset === "custom"
                  ? "border-primary bg-primary text-[#f2f3fc] dark:border-accent dark:bg-accent dark:text-[#241c3d]"
                  : "border-border bg-card text-muted hover:border-border-strong hover:text-text",
              )}
            >
              بازه سفارشی
            </span>
          )}
        >
          {(close) => (
            <div className="p-2" dir="rtl">
              <p className="pb-2 text-[11px] font-bold text-text">انتخاب بازه دلخواه</p>
              <div className="grid grid-cols-2 gap-2" dir="ltr">
                <label className="block">
                  <span className="mb-1 block text-right text-[10px] text-muted">از تاریخ</span>
                  <input
                    type="date"
                    value={cFrom}
                    max={cTo}
                    onChange={(e) => setCFrom(e.target.value)}
                    className="h-9 w-full rounded-lg border border-border bg-card px-2 text-[11px] text-text outline-none focus:border-accent"
                  />
                </label>
                <label className="block">
                  <span className="mb-1 block text-right text-[10px] text-muted">تا تاریخ</span>
                  <input
                    type="date"
                    value={cTo}
                    min={cFrom}
                    onChange={(e) => setCTo(e.target.value)}
                    className="h-9 w-full rounded-lg border border-border bg-card px-2 text-[11px] text-text outline-none focus:border-accent"
                  />
                </label>
              </div>
              <button
                onClick={() => {
                  const from = new Date(`${cFrom}T00:00:00`);
                  const to = new Date(`${cTo}T00:00:00`);
                  if (!isNaN(from.getTime()) && !isNaN(to.getTime())) {
                    setCustomRange(from, to);
                    close();
                  }
                }}
                className="mt-3 inline-flex w-full items-center justify-center gap-1.5 rounded-xl bg-primary py-2 text-xs font-bold text-[#f2f3fc] transition-transform hover:scale-[1.01] active:scale-95 dark:bg-accent dark:text-[#241c3d]"
              >
                <Check className="size-3.5" aria-hidden />
                اعمال بازه
              </button>
            </div>
          )}
        </Popover>
      </div>

      {/* dimension filters */}
      <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3 xl:flex xl:items-center">
        <Select
          label="شعبه"
          value={filters.branchId}
          onChange={(v) => patchFilters({ branchId: v })}
          options={[{ value: "all", label: "همه شعبه‌ها" }, ...BRANCHES.map((b) => ({ value: b.id, label: b.name }))]}
          className="xl:w-40"
        />
        <Select
          label="حساب"
          value={filters.accountId}
          onChange={(v) => patchFilters({ accountId: v })}
          options={[{ value: "all", label: "همه حساب‌ها" }, ...ACCOUNTS.map((a) => ({ value: a.id, label: a.name }))]}
          className="xl:w-44"
        />
        <Select
          label="دسته‌بندی"
          value={filters.categoryId}
          onChange={(v) => patchFilters({ categoryId: v })}
          options={[{ value: "all", label: "همه دسته‌ها" }, ...categories]}
          className="xl:w-44"
        />
        <Select
          label="نوع تراکنش"
          value={filters.txType}
          onChange={(v) => patchFilters({ txType: v })}
          options={[
            { value: "all", label: "همه تراکنش‌ها" },
            ...Object.entries(TX_TYPE_LABEL).map(([value, label]) => ({ value, label })),
          ]}
          className="xl:w-36"
        />
        <Select
          label="کاربر"
          value={filters.userId}
          onChange={(v) => patchFilters({ userId: v })}
          options={[{ value: "all", label: "همه کاربران" }, ...USERS.map((u) => ({ value: u.id, label: u.name }))]}
          className="xl:w-36"
        />

        <div className="col-span-2 flex items-center justify-between gap-2 sm:col-span-1 xl:ms-auto xl:w-auto">
          <p className="text-[10.5px] text-faint">
            {rangeLabel(filters.range)} · {faDate(filters.range.from)} تا {faDate(filters.range.to)}
          </p>
          <button
            onClick={resetFilters}
            disabled={isDefault}
            className={cn(
              "inline-flex h-9 shrink-0 items-center gap-1.5 rounded-xl border px-3 text-[11px] font-bold transition-all",
              isDefault
                ? "cursor-not-allowed border-border text-faint opacity-50"
                : "border-border text-muted hover:border-border-strong hover:text-text active:scale-95",
            )}
          >
            <RotateCcw className="size-3.5" aria-hidden />
            بازنشانی
          </button>
        </div>
      </div>
    </section>
  );
}
