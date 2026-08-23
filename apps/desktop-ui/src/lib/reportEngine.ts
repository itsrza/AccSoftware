/**
 * موتور جمع‌بندی گزارش‌ساز.
 *
 * ## چرا این منطق جدا شده
 *
 * گزارش بدون جمع، گزارش نیست. اما جمع‌زدن ستون‌ها ظرافت دارد: ستون «مانده»
 * را نباید مثل ستون «مبلغ» جمع زد، و ستون «تعداد» با ستون «ریال» یکی نیست.
 * این منطق در یک ماژول تست‌پذیر جدا شده تا در چهار جای مختلف صفحه تکرار
 * نشود و نتیجه‌ی جمع کل با جمع گروه‌ها هرگز واگرا نشود.
 *
 * ## قاعده‌ی حسابداری
 *
 * جمع کل گزارش باید **دقیقاً** برابر مجموع جمع گروه‌ها باشد. اگر گروه‌بندی
 * عوض شود ولی جمع کل تغییر کند، یعنی ردیفی دوبار شمرده یا جا افتاده است.
 */

export type ReportValue = string | number | null | undefined
export type ReportRow = Record<string, ReportValue>

/** نحوه‌ی جمع‌بندی هر ستون. */
export type Aggregation = 'none' | 'sum' | 'count' | 'average' | 'min' | 'max'

export type ColumnKind = 'text' | 'money' | 'quantity' | 'date'

export type ReportColumn = {
  key: string
  label: string
  kind: ColumnKind
  /** جمع‌بندی پیش‌فرض این ستون. */
  aggregation: Aggregation
}

export const AGGREGATION_LABELS: Record<Aggregation, string> = {
  none: 'بدون جمع',
  sum: 'جمع',
  count: 'تعداد',
  average: 'میانگین',
  min: 'کمینه',
  max: 'بیشینه',
}

/** جمع‌بندی‌هایی که برای هر نوع ستون معنا دارند. */
export function allowedAggregations(kind: ColumnKind): Aggregation[] {
  switch (kind) {
    case 'money':
    case 'quantity':
      return ['none', 'sum', 'average', 'min', 'max', 'count']
    case 'date':
      return ['none', 'count', 'min', 'max']
    default:
      return ['none', 'count']
  }
}

const numeric = (value: ReportValue): number | null => {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null
  if (typeof value === 'string' && value.trim() !== '') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }
  return null
}

/**
 * محاسبه‌ی یک جمع‌بندی روی مجموعه‌ای از ردیف‌ها.
 *
 * ردیف‌هایی که مقدار عددی ندارند در `sum`/`average` نادیده گرفته می‌شوند —
 * نه اینکه صفر حساب شوند. صفر حساب کردنشان میانگین را اشتباه می‌کند.
 */
export function aggregate(
  rows: ReportRow[],
  key: string,
  aggregation: Aggregation,
): number | string | null {
  if (aggregation === 'none') return null
  if (aggregation === 'count') return rows.length

  const values = rows.map((row) => row[key])
  if (aggregation === 'min' || aggregation === 'max') {
    const numbers = values.map(numeric).filter((value): value is number => value !== null)
    if (numbers.length > 0) {
      return aggregation === 'min' ? Math.min(...numbers) : Math.max(...numbers)
    }
    // برای ستون تاریخ (رشته‌ی شمسی) مقایسه‌ی متنی درست کار می‌کند چون
    // قالب `YYYY/MM/DD` مرتب‌پذیر است.
    const texts = values
      .filter((value): value is string => typeof value === 'string' && value.trim() !== '')
      .sort()
    if (texts.length === 0) return null
    return aggregation === 'min' ? texts[0] : texts[texts.length - 1]
  }

  const numbers = values.map(numeric).filter((value): value is number => value !== null)
  if (numbers.length === 0) return null
  const total = numbers.reduce((sum, value) => sum + value, 0)
  if (aggregation === 'sum') return total
  return total / numbers.length
}

export type ReportGroup = {
  key: string
  rows: ReportRow[]
  /** جمع‌بندی هر ستون در این گروه. */
  totals: Record<string, number | string | null>
}

export type ReportResult = {
  groups: ReportGroup[]
  rowCount: number
  /** جمع‌بندی کل گزارش. */
  grandTotals: Record<string, number | string | null>
}

/**
 * فیلتر، مرتب‌سازی، گروه‌بندی و جمع‌بندی — همه در یک گذر.
 *
 * مرتب‌سازی عددی و متنی از هم تفکیک می‌شود: مرتب‌سازی متنی روی مبلغ باعث
 * می‌شود ۱۰۰ قبل از ۲۰ بیاید.
 */
export function buildReport(
  rows: ReportRow[],
  columns: ReportColumn[],
  options: {
    search?: string
    sortKey?: string
    sortDirection?: 'asc' | 'desc'
    groupKey?: string
    aggregations?: Record<string, Aggregation>
  },
): ReportResult {
  const search = (options.search ?? '').trim().toLowerCase()
  let filtered = rows
  if (search) {
    filtered = rows.filter((row) =>
      Object.values(row).some((value) => String(value ?? '').toLowerCase().includes(search)),
    )
  }

  if (options.sortKey) {
    const key = options.sortKey
    const factor = options.sortDirection === 'desc' ? -1 : 1
    const column = columns.find((item) => item.key === key)
    const isNumeric = column?.kind === 'money' || column?.kind === 'quantity'
    filtered = [...filtered].sort((first, second) => {
      const a = first[key]
      const b = second[key]
      if (isNumeric) {
        return ((numeric(a) ?? 0) - (numeric(b) ?? 0)) * factor
      }
      return String(a ?? '').localeCompare(String(b ?? ''), 'fa', { numeric: true }) * factor
    })
  }

  const aggregations = options.aggregations ?? {}
  const totalsOf = (subset: ReportRow[]) =>
    Object.fromEntries(
      columns.map((column) => [
        column.key,
        aggregate(subset, column.key, aggregations[column.key] ?? column.aggregation),
      ]),
    )

  let groups: ReportGroup[]
  if (options.groupKey) {
    const buckets = new Map<string, ReportRow[]>()
    for (const row of filtered) {
      const key = String(row[options.groupKey] ?? '—')
      const bucket = buckets.get(key)
      if (bucket) bucket.push(row)
      else buckets.set(key, [row])
    }
    groups = [...buckets.entries()]
      .sort((a, b) => a[0].localeCompare(b[0], 'fa', { numeric: true }))
      .map(([key, groupRows]) => ({ key, rows: groupRows, totals: totalsOf(groupRows) }))
  } else {
    groups = [{ key: '', rows: filtered, totals: totalsOf(filtered) }]
  }

  return {
    groups,
    rowCount: filtered.length,
    grandTotals: totalsOf(filtered),
  }
}

/**
 * بررسی سازگاری جمع گروه‌ها با جمع کل.
 *
 * اگر گروه‌بندی باعث شود ردیفی دوبار شمرده یا جا بیفتد، اینجا لو می‌رود.
 * فقط برای ستون‌های «جمع» و «تعداد» معنا دارد (میانگین جمع‌پذیر نیست).
 */
export function totalsAreConsistent(result: ReportResult, columns: ReportColumn[], aggregations: Record<string, Aggregation>): boolean {
  for (const column of columns) {
    const mode = aggregations[column.key] ?? column.aggregation
    if (mode !== 'sum' && mode !== 'count') continue
    const grand = result.grandTotals[column.key]
    if (typeof grand !== 'number') continue
    const summed = result.groups.reduce((total, group) => {
      const value = group.totals[column.key]
      return total + (typeof value === 'number' ? value : 0)
    }, 0)
    // مقایسه با رواداری بسیار کم برای خطای ممیز شناور در ستون‌های مقداری.
    if (Math.abs(summed - grand) > 1e-6) return false
  }
  return true
}
