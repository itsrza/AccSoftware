import { useEffect, useMemo, useState } from 'react'
import { Download, Pencil, Plus, RefreshCw, ScrollText, Tags } from 'lucide-react'
import { getProductGroups, getProductsDetailed, type ProductGroupRow, type ProductListRow } from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money, percentSign, rialUnit } from '../lib/format'
import { useI18n } from '../lib/i18n'
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

export function Products({
  onCardex,
}: {
  /** باز کردن کاردکس — با کالای انتخاب‌شده یا بدون آن (F4/F5/F6 مرجع). */
  onCardex?: (productId: string | undefined, kind: 'sales' | 'purchase' | 'all') => void
}) {
  const { t } = useI18n()
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
    const header = [
      t('common.code'),
      t('common.name'),
      t('common.type'),
      t('common.group'),
      t('common.unit'),
      t('products.stock'),
      t('products.retail'),
      t('products.partner'),
      t('products.purchase'),
    ]
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
    link.download = t('products.exportFile')
    link.click()
    URL.revokeObjectURL(url)
  }

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('products.eyebrow')}</div>
          <h1>{t('products.title')}</h1>
          <p>{t('products.lead')}</p>
        </div>
        <div className="flex items-center gap-2">
          <button className="ghost" onClick={exportCsv} disabled={sorted.length === 0}>
            <Download className="size-3.5" aria-hidden /> {t('products.exportCsv')}
          </button>
          <button aria-label={t('common.reload')} className="icon-btn" onClick={load}>
            <RefreshCw className="size-4" aria-hidden />
          </button>
          <button className="primary" onClick={() => setEditing({})}>
            <Plus className="size-4" aria-hidden /> {t('products.newItem')}
          </button>
        </div>
      </div>

      {error && <ErrorState onRetry={load} />}

      <section className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {[
          { label: t('products.count'), value: formatNumber(totals.count), unit: t('products.pieces') },
          { label: t('products.stockValue'), value: money(totals.stockValue), unit: rialUnit() },
          {
            label: t('products.lowStock'),
            value: formatNumber(totals.low),
            unit: t('products.pieces'),
            warn: true,
          },
          {
            label: t('products.taxExemptCount'),
            value: formatNumber(totals.exempt),
            unit: t('products.pieces'),
          },
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
            <span>{t('products.group')}</span>
            <Select
              value={group}
              aria-label={t('products.group')}
              onChange={(e) => setGroup(e.target.value)}
            >
              <option value="all">{t('products.allGroups')}</option>
              {groups.map((item) => (
                <option key={item.id} value={item.title}>
                  {item.title}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('products.kind')}</span>
            <Select
              value={kind}
              aria-label={t('products.kind')}
              onChange={(e) => setKind(e.target.value)}
            >
              <option value="all">{t('products.allKinds')}</option>
              {kinds.map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('products.stockStatus')}</span>
            <Select
              value={stockFilter}
              aria-label={t('products.stockStatus')}
              onChange={(e) => setStockFilter(e.target.value)}
            >
              <option value="all">{t('common.all')}</option>
              <option value="low">{t('products.lowStock')}</option>
              <option value="out">{t('products.outOfStock')}</option>
            </Select>
          </label>
          <label className="grow">
            <span>{t('products.searchHint')}</span>
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t('common.searchShort')}
            />
          </label>
        </div>
      </Card>

      <Card pad={false}>
        <div className="p-4 sm:p-5">
          <CardHeader
            title={t('products.listTitle')}
            subtitle={t('products.listSubtitle')}
            action={
              <div className="flex items-center gap-2">
                {onCardex && (
                  <>
                    <button
                      type="button"
                      className="table-action"
                      onClick={() => onCardex(undefined, 'sales')}
                      title="F4"
                    >
                      <ScrollText className="size-3.5" aria-hidden /> {t('cardex.kind.sales')}
                    </button>
                    <button
                      type="button"
                      className="table-action"
                      onClick={() => onCardex(undefined, 'purchase')}
                      title="F5"
                    >
                      <ScrollText className="size-3.5" aria-hidden /> {t('cardex.kind.purchase')}
                    </button>
                    <button
                      type="button"
                      className="table-action"
                      onClick={() => onCardex(undefined, 'all')}
                      title="F6"
                    >
                      <ScrollText className="size-3.5" aria-hidden /> {t('cardex.kind.all')}
                    </button>
                  </>
                )}
                {!loading ? (
                  <Badge tone="neutral" dot={false}>
                    {t('common.rowsCount', { count: formatNumber(sorted.length) })}
                  </Badge>
                ) : undefined}
              </div>
            }
          />
        </div>
        {loading ? (
          <div className="px-4 pb-5 sm:px-5">
            <Skeleton className="h-64 w-full" />
          </div>
        ) : sorted.length === 0 ? (
          <div className="px-4 pb-5 sm:px-5">
            <EmptyState title={t('products.emptyTitle')} hint={t('products.emptyHint')} />
          </div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th {...sortProps('sku')}>{t('common.code')}</th>
                  <th {...sortProps('name')}>{t('products.name')}</th>
                  <th {...sortProps('kind_label')}>{t('common.type')}</th>
                  <th {...sortProps('group_title')}>{t('common.group')}</th>
                  <th {...sortProps('quantity')}>{t('products.stock')}</th>
                  <th {...sortProps('unit')}>{t('common.unit')}</th>
                  <th {...sortProps('retail_price')}>
                    {t('products.retailWithUnit', { unit: rialUnit() })}
                  </th>
                  <th {...sortProps('partner_price')}>
                    {t('products.partnerWithUnit', { unit: rialUnit() })}
                  </th>
                  <th>{t('common.tax')}</th>
                  <th>{t('common.actions')}</th>
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
                          {low ? ` — ${t('products.lowStock')}` : ''}
                        </span>
                      </td>
                      <td>{row.unit}</td>
                      <td className="num">{money(row.retail_price)}</td>
                      <td className="num">
                        {row.partner_price > 0 ? money(row.partner_price) : '—'}
                      </td>
                      <td>
                        {row.tax_exempt ? (
                          <span className="status neutral">{t('products.exempt')}</span>
                        ) : (
                          <span className="status pending">
                            {formatNumber(row.vat_basis_points / 100)}
                            {percentSign()}
                          </span>
                        )}
                      </td>
                      <td>
                        <div className="flex items-center gap-1.5">
                          {onCardex && (
                            <button
                              type="button"
                              className="table-action"
                              aria-label={t('cardex.title')}
                              title={t('cardex.title')}
                              onClick={() => onCardex(row.id, 'all')}
                            >
                              <ScrollText className="size-3.5" aria-hidden />
                            </button>
                          )}
                          <button
                            type="button"
                            className="table-action"
                            onClick={() => setEditing({ id: row.id })}
                          >
                            <Pencil className="size-3.5" aria-hidden /> {t('common.editAction')}
                          </button>
                        </div>
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
        {t('products.pricingHint')}
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
