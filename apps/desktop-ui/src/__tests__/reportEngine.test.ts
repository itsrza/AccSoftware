/**
 * تست‌های موتور گزارش‌ساز.
 *
 * ## چرا این تست‌ها حیاتی‌اند
 *
 * گزارشی که جمعش با واقعیت نخواند، بدتر از نبود گزارش است — چون کاربر به آن
 * اعتماد می‌کند. مهم‌ترین ادعا این است که **جمع گروه‌ها همیشه با جمع کل
 * برابر است**؛ اگر نباشد، ردیفی دوبار شمرده یا جا افتاده.
 */
import { describe, expect, it } from 'vitest'
import {
  aggregate,
  allowedAggregations,
  buildReport,
  totalsAreConsistent,
  Aggregation,
  ReportColumn,
  ReportRow,
} from '../lib/reportEngine'

const columns: ReportColumn[] = [
  { key: 'date', label: 'تاریخ', kind: 'date', aggregation: 'none' },
  { key: 'customer', label: 'مشتری', kind: 'text', aggregation: 'count' },
  { key: 'amount', label: 'مبلغ', kind: 'money', aggregation: 'sum' },
  { key: 'quantity', label: 'تعداد', kind: 'quantity', aggregation: 'sum' },
]

const rows: ReportRow[] = [
  { date: '1405/01/10', customer: 'علی', amount: 1_000_000, quantity: 2 },
  { date: '1405/02/05', customer: 'زهرا', amount: 2_500_000, quantity: 5 },
  { date: '1405/02/20', customer: 'علی', amount: 750_000, quantity: 1 },
  { date: '1405/03/01', customer: 'رضا', amount: 3_250_000, quantity: 8 },
]

describe('موتور گزارش‌ساز', () => {
  it('جمع ستون مبلغ دقیقاً درست است', () => {
    expect(aggregate(rows, 'amount', 'sum')).toBe(7_500_000)
    expect(aggregate(rows, 'quantity', 'sum')).toBe(16)
  })

  it('تعداد ردیف مستقل از مقدار ستون است', () => {
    expect(aggregate(rows, 'customer', 'count')).toBe(4)
    // حتی ستونی که همه‌جا خالی است، تعداد ردیف را درست می‌دهد.
    expect(aggregate(rows, 'ghost', 'count')).toBe(4)
  })

  it('میانگین، ردیف بدون مقدار را صفر حساب نمی‌کند', () => {
    const withGaps: ReportRow[] = [
      { amount: 100 },
      { amount: null },
      { amount: 300 },
      { amount: undefined },
    ]
    // میانگین دو مقدار موجود = ۲۰۰، نه ۱۰۰ (که با صفر گرفتن خالی‌ها می‌شد)
    expect(aggregate(withGaps, 'amount', 'average')).toBe(200)
    expect(aggregate(withGaps, 'amount', 'sum')).toBe(400)
  })

  it('کمینه و بیشینه روی تاریخ شمسی درست کار می‌کند', () => {
    expect(aggregate(rows, 'date', 'min')).toBe('1405/01/10')
    expect(aggregate(rows, 'date', 'max')).toBe('1405/03/01')
  })

  it('جمع گروه‌ها همیشه با جمع کل برابر است', () => {
    const aggregations: Record<string, Aggregation> = { amount: 'sum', quantity: 'sum' }
    const grouped = buildReport(rows, columns, { groupKey: 'customer', aggregations })

    expect(grouped.groups).toHaveLength(3)
    expect(grouped.grandTotals.amount).toBe(7_500_000)

    const summed = grouped.groups.reduce(
      (total, group) => total + (group.totals.amount as number),
      0,
    )
    expect(summed).toBe(grouped.grandTotals.amount)
    expect(totalsAreConsistent(grouped, columns, aggregations)).toBe(true)
  })

  it('گروه‌بندی هیچ ردیفی را گم یا تکرار نمی‌کند', () => {
    const grouped = buildReport(rows, columns, { groupKey: 'customer' })
    const totalRows = grouped.groups.reduce((count, group) => count + group.rows.length, 0)
    expect(totalRows).toBe(rows.length)
  })

  it('فیلتر متنی روی همه‌ی ستون‌ها اثر می‌کند و جمع را هم اصلاح می‌کند', () => {
    const filtered = buildReport(rows, columns, { search: 'علی', aggregations: { amount: 'sum' } })
    expect(filtered.rowCount).toBe(2)
    expect(filtered.grandTotals.amount).toBe(1_750_000)
  })

  it('مرتب‌سازی عددی روی مبلغ، متنی انجام نمی‌شود', () => {
    const sorted = buildReport(rows, columns, { sortKey: 'amount', sortDirection: 'desc' })
    const amounts = sorted.groups[0].rows.map((row) => row.amount)
    expect(amounts).toEqual([3_250_000, 2_500_000, 1_000_000, 750_000])

    // اگر متنی مرتب می‌شد، «1000000» قبل از «750000» می‌آمد.
    const ascending = buildReport(rows, columns, { sortKey: 'amount', sortDirection: 'asc' })
    expect(ascending.groups[0].rows[0].amount).toBe(750_000)
  })

  it('جمع‌بندی‌های مجاز به نوع ستون وابسته است', () => {
    // ستون متنی جمع‌پذیر نیست — جمع نام مشتری‌ها بی‌معناست.
    expect(allowedAggregations('text')).not.toContain('sum')
    expect(allowedAggregations('text')).toContain('count')
    expect(allowedAggregations('money')).toContain('sum')
    expect(allowedAggregations('money')).toContain('average')
    // جمع تاریخ‌ها هم بی‌معناست.
    expect(allowedAggregations('date')).not.toContain('sum')
    expect(allowedAggregations('date')).toContain('max')
  })

  it('گزارش خالی، جمع خالی می‌دهد نه صفر گمراه‌کننده', () => {
    const empty = buildReport([], columns, { aggregations: { amount: 'sum' } })
    expect(empty.rowCount).toBe(0)
    // برای مجموعه‌ی خالی، جمع «مقدار ندارد» است نه صفر — صفر یعنی «جمع صفر شد».
    expect(empty.grandTotals.amount).toBeNull()
    // ولی تعداد ردیف صفر است، که واقعیت دارد.
    expect(empty.grandTotals.customer).toBe(0)
  })
})
