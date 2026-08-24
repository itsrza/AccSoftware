/**
 * اجزای پایه‌ی سیستم طراحی.
 *
 * برگرفته از سیستم طراحی مرجع (`References/UI-BY-AI`) و منطبق با توکن‌های
 * `design-system.css`. قاعده: هیچ صفحه‌ای رنگ یا شعاع خام ننویسد — همه‌چیز
 * از این اجزا و توکن‌ها می‌آید تا تم روشن و تیره همیشه هماهنگ بمانند.
 */
import {
  useEffect, useRef, useState, type ReactNode,
} from "react";
import {
  Inbox, AlertTriangle, RotateCcw, TrendingUp, TrendingDown, Minus,
} from "lucide-react";
import { cn } from "../lib/cn";
import { Select as DsSelect } from "./Select";
import { formatPercent as fmtPct } from "../lib/format";
import { useI18n } from "../lib/i18n";

// ------------------------------------------------------------- Card
export function Card({
  children, className, pad = true,
}: { children: ReactNode; className?: string; pad?: boolean }) {
  return (
    <section
      data-card
      className={cn(
        "rounded-[var(--radius)] border border-border bg-card shadow-[var(--shadow-sm)]",
        pad && "p-4 sm:p-5",
        className,
      )}
    >
      {children}
    </section>
  );
}

export function CardHeader({
  title, subtitle, action,
}: { title: string; subtitle?: string; action?: ReactNode }) {
  return (
    <header className="mb-4 flex flex-wrap items-start justify-between gap-3">
      <div className="min-w-0">
        <h2 className="flex items-center gap-2 text-[15px] font-bold text-text">
          <span className="inline-block h-4 w-1 rounded-full bg-accent" aria-hidden />
          {title}
        </h2>
        {subtitle && <p className="mt-1 text-xs text-muted">{subtitle}</p>}
      </div>
      {action}
    </header>
  );
}

// ------------------------------------------------------------- Badge
type Tone = "success" | "danger" | "warning" | "info" | "neutral" | "accent";
const TONES: Record<Tone, string> = {
  success: "bg-[var(--success-soft)] text-success",
  danger: "bg-[var(--danger-soft)] text-danger",
  warning: "bg-[var(--warning-soft)] text-warning",
  info: "bg-[var(--info-soft)] text-info",
  neutral: "bg-bg-soft text-muted",
  accent: "bg-[var(--accent-soft)] text-accent-strong",
};

export function Badge({
  children, tone = "neutral", className, dot = true,
}: { children: ReactNode; tone?: Tone; className?: string; dot?: boolean }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-semibold whitespace-nowrap",
        TONES[tone],
        className,
      )}
    >
      {dot && <span className="size-1.5 rounded-full bg-current" aria-hidden />}
      {children}
    </span>
  );
}

// ------------------------------------------------------------- Trend chip
export function TrendChip({
  value, invert = false, suffix,
}: { value: number | null; invert?: boolean; suffix?: string }) {
  const { t } = useI18n();
  const caption = suffix ?? t("ui.vsPrevious");
  if (value === null || !isFinite(value)) {
    return (
      <span className="inline-flex items-center gap-1 text-[11px] text-faint">
        <Minus className="size-3" /> {caption}: {t("ui.noData")}
      </span>
    );
  }
  const up = value >= 0;
  const good = invert ? !up : up;
  const Icon = up ? TrendingUp : TrendingDown;
  return (
    <span
      className={cn("inline-flex items-center gap-1.5 text-[11px] font-semibold")}
      title={caption}
    >
      <span
        className={cn(
          "inline-flex items-center gap-1 rounded-full px-2 py-0.5",
          good ? "bg-[var(--success-soft)] text-success" : "bg-[var(--danger-soft)] text-danger",
        )}
      >
        <Icon className="size-3" aria-hidden />
        <span className="tnum">{fmtPct(Math.abs(value)).replace("+", "")}</span>
      </span>
      <span className="text-faint font-normal">{suffix}</span>
    </span>
  );
}

// ------------------------------------------------------------- Popover
export function Popover({
  trigger, children, align = "start", width = "w-72", label, block = false,
}: {
  trigger: (open: boolean) => ReactNode;
  children: (close: () => void) => ReactNode;
  align?: "start" | "end";
  width?: string;
  label: string;
  block?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={ref} className={cn("relative", block ? "block w-full" : "inline-block")}>
      <button
        type="button"
        aria-label={label}
        aria-expanded={open}
        aria-haspopup="true"
        onClick={() => setOpen((o) => !o)}
        className={cn(block && "block w-full text-start")}
      >
        {trigger(open)}
      </button>
      {open && (
        <div
          role="menu"
          className={cn(
            "fade-up absolute top-[calc(100%+8px)] z-50 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]",
            width,
            align === "start" ? "start-0" : "end-0",
          )}
        >
          {children(() => setOpen(false))}
        </div>
      )}
    </div>
  );
}

// ------------------------------------------------------------- Select
/**
 * دراپ‌داون تک‌منبعِ حقیقت است و در `components/Select.tsx` زندگی می‌کند.
 * اینجا فقط بازصادر می‌شود تا صفحاتی که از `ui` می‌خوانند هم همان جزء را
 * بگیرند و دو ظاهر متفاوت در برنامه وجود نداشته باشد.
 */
export { Select } from "./Select";

/** نسخه‌ی «فهرست گزینه‌ها به‌صورت آرایه» — برای نوارهای فیلتر فشرده. */
export function OptionSelect({
  value, onChange, options, label, className,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
  label: string;
  className?: string;
}) {
  return (
    <label className={cn("block", className)}>
      <span className="sr-only">{label}</span>
      <DsSelect value={value} aria-label={label} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </DsSelect>
    </label>
  );
}

// ------------------------------------------------------------- Segmented control
export function Segmented<T extends string>({
  value, onChange, options, label, size = "md",
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string }[];
  label: string;
  size?: "sm" | "md";
}) {
  return (
    <div
      role="tablist"
      aria-label={label}
      className="inline-flex items-center gap-0.5 rounded-xl border border-border bg-bg-soft p-0.5"
    >
      {options.map((o) => (
        <button
          key={o.value}
          role="tab"
          aria-selected={value === o.value}
          onClick={() => onChange(o.value)}
          className={cn(
            "rounded-[10px] font-semibold transition-all",
            size === "sm" ? "px-2.5 py-1 text-[11px]" : "px-3 py-1.5 text-xs",
            value === o.value
              ? "bg-card text-text shadow-[var(--shadow-sm)]"
              : "text-muted hover:text-text",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

// ------------------------------------------------------------- States
export function EmptyState({
  title,
  hint,
  action,
}: { title?: string; hint?: string; action?: ReactNode }) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col items-center justify-center gap-2 rounded-2xl border border-dashed border-border-strong bg-card-soft px-6 py-10 text-center">
      <span className="grid size-12 place-items-center rounded-2xl bg-bg-soft text-faint">
        <Inbox className="size-6" aria-hidden />
      </span>
      <p className="text-sm font-semibold text-text">{title ?? t("ui.emptyTitle")}</p>
      <p className="text-xs text-muted">{hint ?? t("ui.emptyHint")}</p>
      {action}
    </div>
  );
}

export function ErrorState({ onRetry }: { onRetry?: () => void }) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-[var(--danger)]/40 bg-[var(--danger-soft)] px-6 py-10 text-center">
      <span className="grid size-12 place-items-center rounded-2xl bg-card text-danger">
        <AlertTriangle className="size-6" aria-hidden />
      </span>
      <p className="text-sm font-semibold text-text">{t("ui.errorTitle")}</p>
      <p className="text-xs text-muted">{t("ui.errorHint")}</p>
      {onRetry && (
        <button
          onClick={onRetry}
          className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-xs font-bold text-[#f4f5fd] transition-transform hover:scale-[1.03] active:scale-95 dark:bg-accent dark:text-[#1d1836]"
        >
          <RotateCcw className="size-3.5" aria-hidden />
          {t("common.retry")}
        </button>
      )}
    </div>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("skeleton", className)} aria-hidden />;
}
