import { useEffect, useMemo, useState } from 'react'
import { Download, Pencil, Plus, RefreshCw, Tags } from 'lucide-react'
import { getProductGroups, getProductsDetailed, type ProductGroupRow, type ProductListRow } from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money } from '../lib/format'
import { Badge, Card, CardHeader, EmptyState, ErrorState, Skeleton } from '../components/ui'
import { Select } from '../components/Select'
import { useSort } from '../lib/useSort'
import { ProductForm } from './ProductForm'

/**
 * لیست کالاها.
 *
 * مرجع: تصویر `8Xmc1p`. ستون‌ها دقیقاً همان‌هایی است که نرم‌افزار فعلی
 * دارد: کد، نام، موجودی، واحد، گروه، قیمت جزئی، قیمت همکار — به‌علاوه
 * نوع کالا و وضعیت مالیاتی که در پنل‌های کناری همان صفحه دیده می‌شوند.
 *
 * ## چرا جایگزین صفحه‌ی عمومی داده شد
 * پیش از این کالاها با همان جدول عمومی «اشخاص/کالا» نمایش داده می‌شدند و
 * فرم افزودنش هفت فیلد داشت. کالا در یک نرم‌افزار حسابداری، پرجزئیات‌ترین
 * موجودیت است: هفت سطح قیمت، چند واحدی، مالیات، تخفیف پلکانی و انواع
 * مختلف. بدون این‌ها فاکتور هم نمی‌تواند درست محاسبه شود.
 */

const LOW = 'کم‌موجود'

export function Products() {
  const [rows, setRows] = useState<ProductListRow[]>([])
  const [groups, setGroups] = useState<ProductGroupRow[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const [search, setSearch] = useState('')
  const [group, setGroup] = useState('all')
  const [kind, setKind] = useState('all')
  const [stockFilter, setStockFilter] = useState('all')
  const [editing, setEditing] = useState<{ id?: string } | null>(null)

  const load = () => {
    setLoading(true)
    setError('')
    Promise.all([getProductsDetailed(), getProductGroups().catch(() => [])])
      .then(([products, groupRows]) => {
        setRows(products)
        setGroups(groupRows)
      })
      .catch((e) => setError(errorText(e)))
      .finally(() => setLoading(false))
  }

  useEffect(load, [])

  const kinds = useMemo(() => {
    const map = new Map<string, string>()
    for (const row of rows) map.set(row.kind, row.kind_label)
    return [...map.entries()]
  }, [rows])

  const filtered = useMemo(() => {
    const needle = search.trim()
    return rows.filter((row) => {
      if (group !== 'all' && (row.group_title ?? '') !== group) return false
      if (kind !== 'all' && row.kind !== kind) return false
      if (stockFilter === 'low' && row.quantity > row.min_stock) return false
      if (stockFilter === 'out' && row.quantity > 0) return false
      if (!needle) return true
      return (
        row.sku.includes(needle) ||
        row.name.includes(needle) ||
        (row.barcode ?? '').includes(needle) ||
        (row.group_title ?? '').includes(needle)
      )
    })
  }, [rows, search, group, kind, stockFilter])

  const { sorted, sortProps } = useSort(filtered, 'sku')

  const totals = useMemo(
    () => ({
      count: filtered.length,
      stockValue: filtered.reduce((sum, row) => sum + row.quantity * row.purchase_price, 0),
      low: filtered.filter((row) => row.quantity <= row.min_stock).length,
      exempt: filtered.filter((row) => row.tax_exempt).length,
    }),
    [filtered],
  )

  const exportCsv = () => {
    const header = ['کد', 'نام', 'نوع', 'گروه', 'واحد', 'موجودی', 'جزئی', 'همکار', 'خرید']
    const lines = sorted.map((row) =>
      [
        row.sku,
        row.name,
        row.kind_label,
        row.group_title ?? '',
        row.unit,
        row.quantity,
        row.retail_price,
        row.partner_price,
        row.purchase_price,
      ]
        .map((cell) => `"${String(cell).replace(/"/g, '""')}"`)
        .join(','),
    )
    const blob = new Blob(['\ufeff' + [header.join(','), ...lines].join('\r\n')], {
      type: 'text/csv;charset=utf-8',
    })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = 'کالاها.csv'
    link.click()
    URL.revokeObjectURL(url)
  }

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">اطلاعات پایه</div>
          <h1>کالاها و خدمات</h1>
          <p>
            هر کالا هفت سطح قیمت، واحدهای فرعی، اطلاعات مالیاتی و تخفیف پلکانی دارد — همان چیزی که
            موتور فاکتور برای محاسبه‌ی درست لازم دارد.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="ghost" onClick={exportCsv} disabled={sorted.length === 0}>
            <Download className="size-3.5" aria-hidden /> خروجی CSV
          </button>
          <button aria-label="بارگذاری دوباره" className="icon-btn" onClick={load}>
            <RefreshCw className="size-4" aria-hidden />
          </button>
          <button className="primary" onClick={() => setEditing({})}>
            <Plus className="size-4" aria-hidden /> کالای جدید
          </button>
        </div>
      </div>

      {error && <ErrorState onRetry={load} />}

      <section className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {[
          { label: 'تعداد کالا', value: formatNumber(totals.count), unit: 'قلم' },
          { label: 'ارزش موجودی', value: money(totals.stockValue), unit: 'ریال' },
          { label: 'کم‌موجود', value: formatNumber(totals.low), unit: 'قلم', warn: true },
          { label: 'معاف از مالیات', value: formatNumber(totals.exempt), unit: 'قلم' },
        ].map((kpi) => (
          <article
            key={kpi.label}
            data-card
            className="rounded-[var(--radius)] border border-border bg-card p-3.5 shadow-[var(--shadow-sm)]"
          >
            <p className="text-[11px] font-semibold text-muted">{kpi.label}</p>
            {loading ? (
              <Skeleton className="mt-2 h-5 w-20" />
            ) : (
              <p
                className={`tnum mt-1.5 truncate text-[17px] font-extrabold ${
                  kpi.warn && totals.low > 0 ? 'text-warning' : 'text-text'
                }`}
              >
                {kpi.value}
                <span className="ms-1 text-[10px] font-semibold text-faint">{kpi.unit}</span>
              </p>
            )}
          </article>
        ))}
      </section>

      <Card>
        <div className="filter-grid">
          <label>
            <span>گروه کالا</span>
            <Select value={group} aria-label="گروه کالا" onChange={(e) => setGroup(e.target.value)}>
              <option value="all">همه‌ی گروه‌ها</option>
              {groups.map((item) => (
                <option key={item.id} value={item.title}>
                  {item.title}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>نوع کالا</span>
            <Select value={kind} aria-label="نوع کالا" onChange={(e) => setKind(e.target.value)}>
              <option value="all">همه‌ی انواع</option>
              {kinds.map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>وضعیت موجودی</span>
            <Select
              value={stockFilter}
              aria-label="وضعیت موجودی"
              onChange={(e) => setStockFilter(e.target.value)}
            >
              <option value="all">همه</option>
              <option value="low">کم‌موجود</option>
              <option value="out">ناموجود</option>
            </Select>
          </label>
          <label className="grow">
            <span>جستجو در کد، نام، بارکد یا گروه</span>
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="جستجو…" />
          </label>
        </div>
      </Card>

      <Card pad={false}>
        <div className="p-4 sm:p-5">
          <CardHeader
            title="فهرست کالاها"
            subtitle="برای ویرایش روی دکمه‌ی مداد هر ردیف بزنید"
            action={
              !loading ? (
                <Badge tone="neutral" dot={false}>
                  {formatNumber(sorted.length)} ردیف
                </Badge>
              ) : undefined
            }
          />
        </div>
        {loading ? (
          <div className="px-4 pb-5 sm:px-5">
            <Skeleton className="h-64 w-full" />
          </div>
        ) : sorted.length === 0 ? (
          <div className="px-4 pb-5 sm:px-5">
            <EmptyState title="کالایی با این فیلترها پیدا نشد." hint="فیلترها را بردارید یا کالای جدید بسازید." />
          </div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th {...sortProps('sku')}>کد</th>
                  <th {...sortProps('name')}>نام کالا</th>
                  <th {...sortProps('kind_label')}>نوع</th>
                  <th {...sortProps('group_title')}>گروه</th>
                  <th {...sortProps('quantity')}>موجودی</th>
                  <th {...sortProps('unit')}>واحد</th>
                  <th {...sortProps('retail_price')}>جزئی (ریال)</th>
                  <th {...sortProps('partner_price')}>همکار (ریال)</th>
                  <th>مالیات</th>
                  <th>عملیات</th>
                </tr>
              </thead>
              <tbody>
                {sorted.map((row) => {
                  const low = row.quantity <= row.min_stock
                  return (
                    <tr key={row.id}>
                      <td>
                        <b className="code">{row.sku}</b>
                      </td>
                      <td>{row.name}</td>
                      <td>
                        <span className="chip">{row.kind_label}</span>
                      </td>
                      <td>{row.group_title ?? '—'}</td>
                      <td className="num">
                        <span className={low ? 'status danger' : 'status done'}>
                          {formatNumber(row.quantity)}
                          {low ? ` — ${LOW}` : ''}
                        </span>
                      </td>
                      <td>{row.unit}</td>
                      <td className="num">{money(row.retail_price)}</td>
                      <td className="num">
                        {row.partner_price > 0 ? money(row.partner_price) : '—'}
                      </td>
                      <td>
                        {row.tax_exempt ? (
                          <span className="status neutral">معاف</span>
                        ) : (
                          <span className="status pending">
                            {formatNumber(row.vat_basis_points / 100)}٪
                          </span>
                        )}
                      </td>
                      <td>
                        <button
                          type="button"
                          className="table-action"
                          onClick={() => setEditing({ id: row.id })}
                        >
                          <Pencil className="size-3.5" aria-hidden /> ویرایش
                        </button>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      <p className="flex items-center gap-2 text-[11px] text-faint">
        <Tags className="size-3.5" aria-hidden />
        قیمت‌های همه‌ی کالاها را می‌توانید یک‌جا از صفحه‌ی «قیمت کالاها» ویرایش کنید.
      </p>

      {editing && (
        <ProductForm
          productId={editing.id}
          onClose={() => setEditing(null)}
          onSaved={() => {
            setEditing(null)
            load()
          }}
        />
      )}
    </section>
  )
}
