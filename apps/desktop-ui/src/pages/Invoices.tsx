import { useEffect, useMemo, useState } from 'react'
import { Download, Plus, RefreshCw } from 'lucide-react'
import {
  getParties,
  getPurchaseInvoices,
  getSalesInvoices,
  getWarehouses,
  InvoiceSummary,
} from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money } from '../lib/format'
import { Badge, Card, CardHeader, EmptyState, ErrorState, Skeleton } from '../components/ui'
import { FilterBar } from '../components/FilterBar'
import { useSort } from '../lib/useSort'
import { defaultRange, useFiscalRange } from './invoiceListData'
import { inRange, resolveRange, type JalaliRange } from '../lib/dateRange'

/**
 * فهرست فاکتورهای فروش و خرید.
 *
 * ## چرا بازنویسی شد
 * نسخه‌ی قبلی دو دکمه‌ی «فیلترها» و «خروجی» داشت که هر دو `disabled` بودند —
 * دقیقاً همان «دکمه‌ی بی‌عملکرد» که قاعده‌ی محصول ممنوعش کرده. همچنین نام
 * طرف حساب و انبار را نشان نمی‌داد، در حالی که در فهرست فاکتور نرم‌افزار
 * فعلی (تصویر `sFpxWK`) هر دو ستون وجود دارند و کاربر بدون آن‌ها نمی‌تواند
 * فاکتور را تشخیص بدهد.
 *
 * ## ساختار، منطبق با سیستم طراحی مرجع
 * نوار فیلتر سراسری ← نوار شاخص‌ها ← جدول قابل مرتب‌سازی. همان الگویی که
 * `ModulePage` مرجع برای همه‌ی ماژول‌ها دارد.
 */

const STATUS_LABEL: Record<string, string> = {
  posted: 'ثبت شده',
  draft: 'پیش‌نویس',
  cancelled: 'باطل شده',
  reversed: 'برگشت شده',
}

const PAYMENT_LABEL: Record<string, string> = {
  paid: 'تسویه کامل',
  partial: 'تسویه جزئی',
  unpaid: 'تسویه نشده',
}

type Row = InvoiceSummary & { contact_name: string; warehouse_name: string; net: number }

export function Invoices({ page, onNavigate }: { page: string; onNavigate?: (page: string) => void }) {
  const sale = page === 'sales'
  const title = sale ? 'فاکتورهای فروش' : 'فاکتورهای خرید'

  const [rows, setRows] = useState<InvoiceSummary[]>([])
  const [names, setNames] = useState<Record<string, string>>({})
  const [warehouses, setWarehouses] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const fiscalRange = useFiscalRange()
  const [range, setRange] = useState<JalaliRange>(() => defaultRange())
  const [status, setStatus] = useState('all')
  const [payment, setPayment] = useState('all')
  const [search, setSearch] = useState('')

  useEffect(() => {
    if (!fiscalRange) return
    setRange((current) =>
      current.preset === 'fiscalYear' ? { preset: 'fiscalYear', ...fiscalRange } : current,
    )
  }, [fiscalRange])

  const load = () => {
    setLoading(true)
    setError('')
    Promise.all([
      sale ? getSalesInvoices() : getPurchaseInvoices(),
      getParties().catch(() => ({ rows: [] as { id: string; display_name: string }[] })),
      getWarehouses().catch(() => []),
    ])
      .then(([invoices, parties, stores]) => {
        setRows(invoices)
        setNames(Object.fromEntries((parties.rows ?? []).map((row) => [row.id, row.display_name])))
        setWarehouses(Object.fromEntries((stores ?? []).map((row) => [row.id, row.name])))
      })
      .catch((e) => setError(errorText(e)))
      .finally(() => setLoading(false))
  }

  useEffect(load, [sale])

  const enriched: Row[] = useMemo(
    () =>
      rows.map((row) => ({
        ...row,
        contact_name: names[row.contact_id ?? ''] ?? 'بدون طرف حساب',
        warehouse_name: warehouses[row.warehouse_id ?? ''] ?? '—',
        net: row.subtotal - row.discount,
      })),
    [rows, names, warehouses],
  )

  const filtered = useMemo(() => {
    const needle = search.trim()
    return enriched.filter((row) => {
      if (!inRange(row.invoice_date, range.from, range.to)) return false
      if (status !== 'all' && row.status !== status) return false
      if (payment !== 'all' && row.payment_status !== payment) return false
      if (!needle) return true
      return (
        String(row.number).includes(needle) ||
        row.contact_name.includes(needle) ||
        row.warehouse_name.includes(needle)
      )
    })
  }, [enriched, range, status, payment, search])

  const { sorted, sortProps } = useSort(filtered, 'invoice_date')

  const totals = useMemo(
    () => ({
      count: filtered.length,
      net: filtered.reduce((sum, row) => sum + row.net, 0),
      tax: filtered.reduce((sum, row) => sum + row.tax, 0),
      total: filtered.reduce((sum, row) => sum + row.total, 0),
      unsettled: filtered
        .filter((row) => row.status === 'posted' && row.payment_status !== 'paid')
        .reduce((sum, row) => sum + row.total, 0),
    }),
    [filtered],
  )

  const isDefault =
    range.preset === 'fiscalYear' && status === 'all' && payment === 'all' && search.trim() === ''

  const reset = () => {
    setRange(resolveRange('fiscalYear', fiscalRange))
    setStatus('all')
    setPayment('all')
    setSearch('')
  }

  /** خروجی CSV واقعی از همان سطرهایی که روی صفحه دیده می‌شوند. */
  const exportCsv = () => {
    const header = [
      'شماره',
      'تاریخ',
      'طرف حساب',
      'انبار',
      'خالص',
      'مالیات',
      'جمع کل',
      'وضعیت',
      'تسویه',
    ]
    const lines = sorted.map((row) =>
      [
        row.number,
        row.invoice_date,
        row.contact_name,
        row.warehouse_name,
        row.net,
        row.tax,
        row.total,
        STATUS_LABEL[row.status] ?? row.status,
        PAYMENT_LABEL[row.payment_status] ?? row.payment_status,
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
    link.download = `${title}-${range.from.replace(/\//g, '')}-${range.to.replace(/\//g, '')}.csv`
    link.click()
    URL.revokeObjectURL(url)
  }

  const KPIS = [
    { label: 'تعداد فاکتور', value: formatNumber(totals.count), unit: 'فقره' },
    { label: 'مبلغ خالص', value: money(totals.net), unit: 'ریال' },
    { label: 'ارزش افزوده', value: money(totals.tax), unit: 'ریال' },
    { label: 'جمع کل', value: money(totals.total), unit: 'ریال' },
    { label: 'تسویه‌نشده', value: money(totals.unsettled), unit: 'ریال', warn: true },
  ]

  return (
    <section className="page flex flex-col gap-4" dir="rtl">
      <div className="page-head">
        <div>
          <div className="eyebrow">مدیریت و عملیات</div>
          <h1>{title}</h1>
          <p>ستون‌ها با کلیک روی سربرگ مرتب می‌شوند و خروجی از همین سطرهای فیلترشده ساخته می‌شود.</p>
        </div>
        <div className="flex items-center gap-2">
          <button className="ghost" onClick={exportCsv} disabled={sorted.length === 0}>
            <Download className="size-3.5" aria-hidden /> خروجی CSV
          </button>
          <button aria-label="بارگذاری دوباره" className="icon-btn" onClick={load}>
            <RefreshCw className="size-4" aria-hidden />
          </button>
          {sale && (
            <button className="primary" onClick={() => onNavigate?.('invoice-form')}>
              <Plus className="size-4" aria-hidden /> فاکتور جدید
            </button>
          )}
        </div>
      </div>

      <FilterBar
        range={range}
        onRange={setRange}
        fiscalRange={fiscalRange}
        filters={[
          {
            key: 'status',
            label: 'وضعیت سند',
            value: status,
            width: 'xl:w-40',
            onChange: setStatus,
            options: [
              { value: 'all', label: 'همه‌ی وضعیت‌ها' },
              { value: 'posted', label: 'ثبت شده' },
              { value: 'draft', label: 'پیش‌نویس' },
              { value: 'cancelled', label: 'باطل شده' },
              { value: 'reversed', label: 'برگشت شده' },
            ],
          },
          {
            key: 'payment',
            label: 'وضعیت تسویه',
            value: payment,
            width: 'xl:w-40',
            onChange: setPayment,
            options: [
              { value: 'all', label: 'همه‌ی تسویه‌ها' },
              { value: 'paid', label: 'تسویه کامل' },
              { value: 'partial', label: 'تسویه جزئی' },
              { value: 'unpaid', label: 'تسویه نشده' },
            ],
          },
        ]}
        search={search}
        onSearch={setSearch}
        searchPlaceholder="شماره، طرف حساب یا انبار…"
        onReset={reset}
        isDefault={isDefault}
        note={loading ? undefined : `${formatNumber(filtered.length)} از ${formatNumber(rows.length)}`}
      />

      {error && <ErrorState onRetry={load} />}

      <section className="grid grid-cols-2 gap-3 sm:grid-cols-3 xl:grid-cols-5">
        {KPIS.map((kpi) => (
          <article
            key={kpi.label}
            data-card
            className="rounded-[var(--radius)] border border-border bg-card p-3.5 shadow-[var(--shadow-sm)]"
          >
            <p className="text-[11px] font-semibold text-muted">{kpi.label}</p>
            {loading ? (
              <Skeleton className="mt-2 h-5 w-24" />
            ) : (
              <p
                className={`tnum mt-1.5 truncate text-[17px] font-extrabold tracking-tight ${
                  kpi.warn ? 'text-warning' : 'text-text'
                }`}
              >
                {kpi.value}
                <span className="ms-1 text-[10px] font-semibold text-faint">{kpi.unit}</span>
              </p>
            )}
          </article>
        ))}
      </section>

      <Card pad={false}>
        <div className="p-4 sm:p-5">
          <CardHeader
            title={title}
            subtitle="جمع‌های بالا فقط سطرهای فیلترشده را می‌شمارند"
            action={
              !loading && filtered.length > 0 ? (
                <Badge tone="neutral" dot={false}>
                  {formatNumber(filtered.length)} ردیف
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
            <EmptyState
              title="فاکتوری با این فیلترها پیدا نشد."
              hint="بازه یا وضعیت را تغییر دهید."
            />
          </div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th {...sortProps('number')}>شماره</th>
                  <th {...sortProps('invoice_date')}>تاریخ</th>
                  <th {...sortProps('contact_name')}>طرف حساب</th>
                  <th {...sortProps('warehouse_name')}>انبار</th>
                  <th {...sortProps('net')}>خالص (ریال)</th>
                  <th {...sortProps('tax')}>ارزش افزوده</th>
                  <th {...sortProps('total')}>جمع کل (ریال)</th>
                  <th {...sortProps('status')}>وضعیت</th>
                  <th {...sortProps('payment_status')}>تسویه</th>
                </tr>
              </thead>
              <tbody>
                {sorted.map((row) => (
                  <tr key={row.id}>
                    <td>
                      <b className="code">{formatNumber(row.number)}</b>
                    </td>
                    <td>{row.invoice_date}</td>
                    <td>{row.contact_name}</td>
                    <td>{row.warehouse_name}</td>
                    <td className="num">{money(row.net)}</td>
                    <td className="num">{money(row.tax)}</td>
                    <td className="num">
                      <b>{money(row.total)}</b>
                    </td>
                    <td>
                      <span
                        className={
                          row.status === 'posted'
                            ? 'status done'
                            : row.status === 'draft'
                              ? 'status pending'
                              : 'status danger'
                        }
                      >
                        {STATUS_LABEL[row.status] ?? row.status}
                      </span>
                    </td>
                    <td>
                      <span
                        className={
                          row.payment_status === 'paid'
                            ? 'status done'
                            : row.payment_status === 'partial'
                              ? 'status pending'
                              : 'status neutral'
                        }
                      >
                        {PAYMENT_LABEL[row.payment_status] ?? row.payment_status}
                      </span>
                    </td>
                  </tr>
                ))}
                <tr className="total-row">
                  <td colSpan={4}>جمع سطرهای نمایش‌داده‌شده</td>
                  <td className="num">{money(totals.net)}</td>
                  <td className="num">{money(totals.tax)}</td>
                  <td className="num">{money(totals.total)}</td>
                  <td colSpan={2} />
                </tr>
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </section>
  )
}
