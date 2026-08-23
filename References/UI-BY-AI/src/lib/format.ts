// ---------------------------------------------------------------------------
// Persian / Jalali formatting & date-range helpers
// ---------------------------------------------------------------------------

const faNum = new Intl.NumberFormat("fa-IR", { maximumFractionDigits: 0 });
const faNum1 = new Intl.NumberFormat("fa-IR", {
  maximumFractionDigits: 1,
  minimumFractionDigits: 0,
});
const faSigned = new Intl.NumberFormat("fa-IR", {
  maximumFractionDigits: 1,
  signDisplay: "always",
});

/** 2485000000 -> ۲٬۴۸۵٬۰۰۰٬۰۰۰ */
export const fmt = (n: number) => faNum.format(Math.round(n));

/** Compact: ۲٫۵ میلیارد / ۸۲۰ میلیون */
export function fmtCompact(n: number): string {
  const a = Math.abs(n);
  if (a >= 1e9) return `${faNum1.format(n / 1e9)} میلیارد`;
  if (a >= 1e6) return `${faNum1.format(n / 1e6)} میلیون`;
  if (a >= 1e3) return `${faNum1.format(n / 1e3)} هزار`;
  return faNum.format(Math.round(n));
}

/** +۱۲٫۴٪ / −۳٫۱٪ (visual minus handled by dir) */
export const fmtPct = (n: number) => `${faSigned.format(n)}٪`;

export const fmtMoney = (n: number) => `${fmt(n)} تومان`;

const dtf = new Intl.DateTimeFormat("fa-IR", {
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
});
const dtfShort = new Intl.DateTimeFormat("fa-IR", { day: "numeric", month: "short" });
const dtfMed = new Intl.DateTimeFormat("fa-IR", { day: "numeric", month: "long" });
const dtfMonth = new Intl.DateTimeFormat("fa-IR", { month: "long" });
const dtfFull = new Intl.DateTimeFormat("fa-IR", {
  weekday: "long",
  day: "numeric",
  month: "long",
  year: "numeric",
});

/** ۱۴۰۵/۰۵/۳۱ */
export const faDate = (d: Date | number) => dtf.format(d);
export const faDateShort = (d: Date | number) => dtfShort.format(d);
export const faDateMed = (d: Date | number) => dtfMed.format(d);
export const faMonth = (d: Date | number) => dtfMonth.format(d);
export const faDateFull = (d: Date | number) => dtfFull.format(d);

// Jalali calendar parts with latin digits (English locale + persian calendar)
const jalaliTF = new Intl.DateTimeFormat("en-u-ca-persian", {
  year: "numeric",
  month: "numeric",
  day: "numeric",
});

export function jParts(d: Date): { jy: number; jm: number; jd: number } {
  const parts = jalaliTF.formatToParts(d);
  const get = (t: string) => Number(parts.find((p) => p.type === t)?.value ?? 0);
  return { jy: get("year"), jm: get("month"), jd: get("day") };
}

export const DAY = 86_400_000;
export const startOfDay = (d: Date) => {
  const c = new Date(d);
  c.setHours(0, 0, 0, 0);
  return c;
};
export const endOfDay = (d: Date) => {
  const c = new Date(d);
  c.setHours(23, 59, 59, 999);
  return c;
};
export const addDays = (d: Date, n: number) => new Date(d.getTime() + n * DAY);

/** First day of the current Jalali month, n months back (0 = current). */
export function jalaliMonthStart(from: Date, monthsBack = 0): Date {
  let cursor = startOfDay(from);
  const { jd } = jParts(cursor);
  cursor = addDays(cursor, -(jd - 1)); // 1st of current jalali month
  for (let i = 0; i < monthsBack; i++) {
    cursor = addDays(cursor, -1); // last day of previous month
    cursor = addDays(cursor, -(jParts(cursor).jd - 1));
  }
  return cursor;
}

export type PresetId =
  | "today"
  | "yesterday"
  | "thisWeek"
  | "lastWeek"
  | "thisMonth"
  | "lastMonth"
  | "thisQuarter"
  | "thisYear"
  | "custom";

export const PRESETS: { id: PresetId; label: string }[] = [
  { id: "today", label: "امروز" },
  { id: "yesterday", label: "دیروز" },
  { id: "thisWeek", label: "این هفته" },
  { id: "lastWeek", label: "هفته گذشته" },
  { id: "thisMonth", label: "این ماه" },
  { id: "lastMonth", label: "ماه گذشته" },
  { id: "thisQuarter", label: "این فصل" },
  { id: "thisYear", label: "امسال" },
  { id: "custom", label: "بازه سفارشی" },
];

export interface DateRange {
  preset: PresetId;
  from: Date;
  to: Date;
}

const FA_DAYS = ["شنبه", "یکشنبه", "دوشنبه", "سه‌شنبه", "چهارشنبه", "پنجشنبه", "جمعه"];

export function faWeekday(d: Date) {
  return FA_DAYS[(d.getDay() + 1) % 7];
}

export function resolveRange(preset: PresetId, custom?: { from: Date; to: Date }, now = new Date()): DateRange {
  const today = startOfDay(now);
  const end = endOfDay(now);
  switch (preset) {
    case "today":
      return { preset, from: today, to: end };
    case "yesterday": {
      const y = addDays(today, -1);
      return { preset, from: y, to: endOfDay(y) };
    }
    case "thisWeek": {
      // Iranian week starts Saturday
      const offset = (now.getDay() + 1) % 7;
      return { preset, from: addDays(today, -offset), to: end };
    }
    case "lastWeek": {
      const offset = (now.getDay() + 1) % 7;
      const thisSat = addDays(today, -offset);
      return { preset, from: addDays(thisSat, -7), to: endOfDay(addDays(thisSat, -1)) };
    }
    case "thisMonth":
      return { preset, from: jalaliMonthStart(today), to: end };
    case "lastMonth": {
      const from = jalaliMonthStart(today, 1);
      const to = endOfDay(addDays(jalaliMonthStart(today), -1));
      return { preset, from, to };
    }
    case "thisQuarter": {
      const { jm } = jParts(today);
      const back = (jm - 1) % 3;
      return { preset, from: jalaliMonthStart(today, back), to: end };
    }
    case "thisYear": {
      const { jm } = jParts(today);
      return { preset, from: jalaliMonthStart(today, jm - 1), to: end };
    }
    case "custom": {
      if (custom) return { preset, from: startOfDay(custom.from), to: endOfDay(custom.to) };
      return { preset, from: jalaliMonthStart(today), to: end };
    }
  }
}

/** Shift a range back by its own length (for previous-period comparison). */
export function previousRange(range: DateRange): { from: Date; to: Date } {
  const len = startOfDay(range.to).getTime() - range.from.getTime();
  const days = Math.max(1, Math.round(len / DAY) + 1);
  const to = endOfDay(addDays(range.from, -1));
  const from = addDays(range.from, -days);
  return { from, to };
}

export function rangeLabel(range: DateRange): string {
  const preset = PRESETS.find((p) => p.id === range.preset);
  if (range.preset === "custom") return `${faDate(range.from)} تا ${faDate(range.to)}`;
  return preset?.label ?? "";
}

/** yyyy-mm-dd local key (timezone safe) */
export function dayKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export const parseFaNumber = (s: string) =>
  Number(s.replace(/[۰-۹]/g, (c) => String("۰۱۲۳۴۵۶۷۸۹".indexOf(c))).replace(/[٬,]/g, ""));
