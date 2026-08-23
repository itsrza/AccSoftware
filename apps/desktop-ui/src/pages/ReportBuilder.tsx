import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  deleteCustomReport,
  getAccountLedgerSummary,
  getInventoryValuation,
  getPurchaseReport,
  getSalesReport,
  listCustomReports,
  saveCustomReport,
} from '../api'
import { errorText } from '../lib/errors'
import { formatNumber, formatRials as money, todayJalali } from '../lib/format'
import {
  aggregate,
  allowedAggregations,
  buildReport,
  AGGREGATION_LABELS,
  Aggregation,
  ReportColumn,
  ReportRow,
} from '../lib/reportEngine'

type Source = 'sales' | 'purchase' | 'inventory' | 'ledger'

type Config = {
  columns: string[]
  search: string
  sortKey: string
  sortDirection: 'asc' | 'desc'
  groupKey: string
  aggregations: Record<string, Aggregation>
  from: string
  to: string
}

const SOURCE_LABELS: Record<Source, string> = {
  sales: 'گزارش فروش',
  purchase: 'گزارش خرید',
  inventory: 'ارزش موجودی انبار',
  ledger: 'گردش حساب‌ها',
}

/**
 * تعریف ستون‌ها با **نوع** و **جمع‌بندی پیش‌فرض**.
 *
 * نوع ستون تعیین می‌کند چطور نمایش داده شود و چه جمع‌بندی‌هایی برایش معنا
 * دارد. مثلاً «مانده» را نباید مثل «مبلغ» جمع زد و «میانگین بها» جمع‌پذیر
 * نیست — میانگینِ میانگین‌ها عدد بی‌معنایی است.
 */
const COLUMNS: Record<Source, ReportColumn[]> = {
  sales: [
    { key: 'date', label: 'تاریخ', kind: 'date', aggregation: 'none' },
    { key: 'invoice_number', label: 'شماره فاکتور', kind: 'text', aggregation: 'count' },
    { key: 'contact_name', label: 'مشتری', kind: 'text', aggregation: 'none' },
    { key: 'subtotal', label: 'مبلغ خالص', kind: 'money', aggregation: 'sum' },
    { key: 'discount', label: 'تخفیف', kind: 'money', aggregation: 'sum' },
    { key: 'tax', label: 'مالیات', kind: 'money', aggregation: 'sum' },
    { key: 'total', label: 'جمع کل', kind: 'money', aggregation: 'sum' },
    { key: 'payment_status', label: 'وضعیت پرداخت', kind: 'text', aggregation: 'none' },
  ],
  purchase: [
    { key: 'date', label: 'تاریخ', kind: 'date', aggregation: 'none' },
    { key: 'invoice_number', label: 'شماره فاکتور', kind: 'text', aggregation: 'count' },
    { key: 'contact_name', label: 'تأمین‌کننده', kind: 'text', aggregation: 'none' },
    { key: 'subtotal', label: 'مبلغ خالص', kind: 'money', aggregation: 'sum' },
    { key: 'discount', label: 'تخفیف', kind: 'money', aggregation: 'sum' },
    { key: 'tax', label: 'مالیات', kind: 'money', aggregation: 'sum' },
    { key: 'total', label: 'جمع کل', kind: 'money', aggregation: 'sum' },
    { key: 'payment_status', label: 'وضعیت پرداخت', kind: 'text', aggregation: 'none' },
  ],
  inventory: [
    { key: 'product_name', label: 'کالا', kind: 'text', aggregation: 'count' },
    { key: 'warehouse_name', label: 'انبار', kind: 'text', aggregation: 'none' },
    { key: 'quantity', label: 'موجودی', kind: 'quantity', aggregation: 'sum' },
    // میانگین بها جمع‌پذیر نیست؛ جمع آن عدد بی‌معنایی می‌سازد.
    { key: 'average_cost', label: 'میانگین بها', kind: 'money', aggregation: 'average' },
    { key: 'value', label: 'ارزش موجودی', kind: 'money', aggregation: 'sum' },
  ],
  ledger: [
    { key: 'code', label: 'کد حساب', kind: 'text', aggregation: 'count' },
    { key: 'name', label: 'نام حساب', kind: 'text', aggregation: 'none' },
    { key: 'debit', label: 'بدهکار', kind: 'money', aggregation: 'sum' },
    { key: 'credit', label: 'بستانکار', kind: 'money', aggregation: 'sum' },
    { key: 'balance', label: 'مانده', kind: 'money', aggregation: 'sum' },
  ],
}

const defaultConfig = (source: Source): Config => ({
  columns: COLUMNS[source].map((column) => column.key),
  search: '',
  sortKey: '',
  sortDirection: 'asc',
  groupKey: '',
  aggregations: Object.fromEntries(
    COLUMNS[source].map((column) => [column.key, column.aggregation]),
  ),
  from: '',
  to: '',
})

const escapeHtml = (value: string) =>
  value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')

function download(name: string, content: string, type: string) {
  const blob = new Blob([content], { type })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = name
  link.click()
  setTimeout(() => URL.revokeObjectURL(link.href), 500)
}

/**
 * گزارش‌ساز پویا.
 *
 * ## دو باگ واقعی که این بازنویسی بست
 *
 * ۱. **فیلتر تاریخ میلادی بود.** ورودی‌ها `type="date"` بودند و مقدار
 *    `2025-08-21` می‌فرستادند، در حالی که همه‌ی تاریخ‌های سیستم شمسی‌اند
 *    (`1405/05/30`). نتیجه: فیلتر تاریخ هیچ‌وقت درست کار نمی‌کرد.
 *
 * ۲. **گزارش جمع نداشت.** گزارشی که جمع ستون‌ها را نشان ندهد، در حسابداری
 *    قابل استفاده نیست. حالا هر ستون جمع‌بندی قابل انتخاب دارد، هر گروه
 *    جمع خودش را نشان می‌دهد و جمع کل هم پایین جدول می‌آید.
 */
export function ReportBuilder() {
  const [source, setSource] = useState<Source>('sales')
  const [rows, setRows] = useState<ReportRow[]>([])
  const [config, setConfig] = useState<Config>(defaultConfig('sales'))
  const [name, setName] = useState('گزارش سفارشی')
  const [saved, setSaved] = useState<
    { id: string; name: string; source: string; config_json: string }[]
  >([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const available = COLUMNS[source]

  const loadSource = useCallback(
    async (target: Source, from: string, to: string) => {
      setLoading(true)
      setError('')
      try {
        const data =
          target === 'sales'
            ? await getSalesReport(from || undefined, to || undefined)
            : target === 'purchase'
              ? await getPurchaseReport(from || undefined, to || undefined)
              : target === 'inventory'
                ? await getInventoryValuation()
                : await getAccountLedgerSummary(from || undefined, to || undefined)
        setRows(data as unknown as ReportRow[])
      } catch (e) {
        setError(errorText(e))
        setRows([])
      } finally {
        setLoading(false)
      }
    },
    [],
  )

  useEffect(() => {
    loadSource(source, config.from, config.to)
    // فقط با تغییر منبع یا بازه، داده دوباره خوانده می‌شود — نه با هر تغییر ستون.
  }, [source, config.from, config.to, loadSource])

  const refreshSaved = useCallback(async () => {
    try {
      setSaved(await listCustomReports())
    } catch (e) {
      setError(errorText(e))
    }
  }, [])

  useEffect(() => {
    refreshSaved()
  }, [refreshSaved])

  const selected = useMemo(
    () => available.filter((column) => config.columns.includes(column.key)),
    [available, config.columns],
  )

  const report = useMemo(
    () =>
      buildReport(rows, selected, {
        search: config.search,
        sortKey: config.sortKey,
        sortDirection: config.sortDirection,
        groupKey: config.groupKey,
        aggregations: config.aggregations,
      }),
    [rows, selected, config],
  )

  const changeSource = (next: Source) => {
    setSource(next)
    setConfig((current) => ({ ...defaultConfig(next), from: current.from, to: current.to }))
  }

  const formatCell = (column: ReportColumn, value: unknown) => {
    if (value === null || value === undefined || value === '') return '—'
    if (column.kind === 'money') return money(Number(value))
    if (column.kind === 'quantity') return formatNumber(Number(value))
    return String(value)
  }

  const formatTotal = (column: ReportColumn, value: number | string | null) => {
    if (value === null) return ''
    const mode = config.aggregations[column.key] ?? column.aggregation
    if (mode === 'count') return `${formatNumber(Number(value))} ردیف`
    if (typeof value !== 'number') return String(value)
    if (column.kind === 'money') return money(value)
    return formatNumber(value)
  }

  const save = async () => {
    setNotice('')
    try {
      await saveCustomReport(undefined, name, source, JSON.stringify(config))
      setNotice('گزارش ذخیره شد.')
      await refreshSaved()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const loadSaved = (item: { name: string; source: string; config_json: string }) => {
    try {
      const parsed = JSON.parse(item.config_json) as Partial<Config>
      const target = item.source as Source
      const base = defaultConfig(target)
      setSource(target)
      setName(item.name)
      setConfig({ ...base, ...parsed, aggregations: { ...base.aggregations, ...parsed.aggregations } })
    } catch {
      setError('تنظیمات گزارش ذخیره‌شده نامعتبر است.')
    }
  }

  const exportRows = report.groups.flatMap((group) => group.rows)

  const exportCsv = () => {
    const quote = (value: unknown) => `"${String(value ?? '').replaceAll('"', '""')}"`
    const lines = [
      selected.map((column) => quote(column.label)).join(','),
      ...exportRows.map((row) => selected.map((column) => quote(row[column.key])).join(',')),
      // ردیف جمع هم در خروجی می‌آید؛ گزارشی که در فایل جمع نداشته باشد ناقص است.
      selected
        .map((column) => quote(formatTotal(column, report.grandTotals[column.key])))
        .join(','),
    ]
    download(`${name}.csv`, `\ufeff${lines.join('\r\n')}`, 'text/csv;charset=utf-8')
  }

  const exportExcel = () => {
    const head = selected.map((column) => `<th>${escapeHtml(column.label)}</th>`).join('')
    const body = exportRows
      .map(
        (row) =>
          `<tr>${selected
            .map((column) => `<td>${escapeHtml(String(row[column.key] ?? ''))}</td>`)
            .join('')}</tr>`,
      )
      .join('')
    const footer = `<tr>${selected
      .map(
        (column) =>
          `<th>${escapeHtml(formatTotal(column, report.grandTotals[column.key]))}</th>`,
      )
      .join('')}</tr>`
    const html = `<html><head><meta charset="utf-8"></head><body dir="rtl"><table border="1"><thead><tr>${head}</tr></thead><tbody>${body}</tbody><tfoot>${footer}</tfoot></table></body></html>`
    download(`${name}.xls`, html, 'application/vnd.ms-excel;charset=utf-8')
  }

  const print = () => {
    const printWindow = window.open('', '_blank', 'width=1100,height=800')
    if (!printWindow) {
      setError('پنجره چاپ توسط مرورگر مسدود شد.')
      return
    }
    const head = selected.map((column) => `<th>${escapeHtml(column.label)}</th>`).join('')
    const bodyRows = report.groups
      .map((group) => {
        const groupHeader = group.key
          ? `<tr class="group"><td colspan="${selected.length}"><b>${escapeHtml(group.key)}</b> — ${group.rows.length} ردیف</td></tr>`
          : ''
        const groupRows = group.rows
          .map(
            (row) =>
              `<tr>${selected
                .map((column) => `<td>${escapeHtml(formatCell(column, row[column.key]))}</td>`)
                .join('')}</tr>`,
          )
          .join('')
        const groupTotals = group.key
          ? `<tr class="subtotal">${selected
              .map(
                (column) =>
                  `<td>${escapeHtml(formatTotal(column, group.totals[column.key]))}</td>`,
              )
              .join('')}</tr>`
          : ''
        return groupHeader + groupRows + groupTotals
      })
      .join('')
    const grand = `<tr class="grand">${selected
      .map((column) => `<td>${escapeHtml(formatTotal(column, report.grandTotals[column.key]))}</td>`)
      .join('')}</tr>`
    printWindow.document.write(
      `<html dir="rtl"><head><meta charset="utf-8"><title>${escapeHtml(name)}</title>` +
        `<style>body{font-family:Tahoma,Arial;padding:28px;color:#21254E}h1{font-size:20px;margin:0 0 4px}` +
        `p{color:#62748E;margin:0 0 18px;font-size:12px}table{width:100%;border-collapse:collapse;font-size:12px}` +
        `th,td{border:1px solid #d9dfe9;padding:7px;text-align:right}th{background:#F6F9FF}` +
        `tr.group td{background:#F6F9FF;font-weight:bold}tr.subtotal td{background:#faf6ef;font-weight:bold}` +
        `tr.grand td{background:#21254E;color:#fff;font-weight:bold}</style></head><body>` +
        `<h1>${escapeHtml(name)}</h1><p>${SOURCE_LABELS[source]} — از ${config.from || 'ابتدای دوره'} تا ${config.to || 'امروز'} — ${report.rowCount} ردیف</p>` +
        `<table><thead><tr>${head}</tr></thead><tbody>${bodyRows}${grand}</tbody></table>` +
        `<script>window.onload=()=>window.print()</script></body></html>`,
    )
    printWindow.document.close()
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">گزارشات</div>
          <h1>گزارش‌ساز</h1>
          <p>
            ستون‌ها، فیلتر، گروه‌بندی و نوع جمع هر ستون را خودتان تعیین کنید. تاریخ‌ها شمسی‌اند و
            جمع گروه‌ها همیشه با جمع کل می‌خواند.
          </p>
        </div>
        <div className="filter-actions">
          <button className="ghost" onClick={() => loadSource(source, config.from, config.to)}>
            <Icon name="refresh" /> بروزرسانی
          </button>
          <button className="primary" onClick={save}>
            ذخیره گزارش
          </button>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="panel">
        <div className="filter-grid">
          <label>
            <span>نام گزارش</span>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            <span>منبع داده</span>
            <select value={source} onChange={(e) => changeSource(e.target.value as Source)}>
              {Object.entries(SOURCE_LABELS).map(([key, label]) => (
                <option key={key} value={key}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>از تاریخ (شمسی)</span>
            <input
              value={config.from}
              onChange={(e) => setConfig({ ...config, from: e.target.value })}
              placeholder="1405/01/01"
            />
          </label>
          <label>
            <span>تا تاریخ (شمسی)</span>
            <input
              value={config.to}
              onChange={(e) => setConfig({ ...config, to: e.target.value })}
              placeholder={todayJalali()}
            />
          </label>
          <label className="grow">
            <span>جستجو در همه‌ی ستون‌ها</span>
            <input
              value={config.search}
              onChange={(e) => setConfig({ ...config, search: e.target.value })}
              placeholder="جستجو…"
            />
          </label>
          <label>
            <span>مرتب‌سازی بر اساس</span>
            <select
              value={config.sortKey}
              onChange={(e) => setConfig({ ...config, sortKey: e.target.value })}
            >
              <option value="">بدون مرتب‌سازی</option>
              {available.map((column) => (
                <option key={column.key} value={column.key}>
                  {column.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>جهت</span>
            <select
              value={config.sortDirection}
              onChange={(e) =>
                setConfig({ ...config, sortDirection: e.target.value as 'asc' | 'desc' })
              }
            >
              <option value="asc">صعودی</option>
              <option value="desc">نزولی</option>
            </select>
          </label>
          <label>
            <span>گروه‌بندی</span>
            <select
              value={config.groupKey}
              onChange={(e) => setConfig({ ...config, groupKey: e.target.value })}
            >
              <option value="">بدون گروه‌بندی</option>
              {available
                .filter((column) => column.kind === 'text' || column.kind === 'date')
                .map((column) => (
                  <option key={column.key} value={column.key}>
                    {column.label}
                  </option>
                ))}
            </select>
          </label>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>ستون‌ها و نوع جمع</h3>
            <p>
              «میانگین بها» جمع‌پذیر نیست — جمع میانگین‌ها عدد بی‌معنایی می‌سازد، پس پیش‌فرضش
              میانگین است.
            </p>
          </div>
        </div>
        <div className="column-config">
          {available.map((column) => {
            const active = config.columns.includes(column.key)
            return (
              <div key={column.key} className={`column-card${active ? ' active' : ''}`}>
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={active}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        columns: e.target.checked
                          ? [...config.columns, column.key]
                          : config.columns.filter((key) => key !== column.key),
                      })
                    }
                  />
                  <span>{column.label}</span>
                </label>
                <select
                  disabled={!active}
                  value={config.aggregations[column.key] ?? column.aggregation}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      aggregations: {
                        ...config.aggregations,
                        [column.key]: e.target.value as Aggregation,
                      },
                    })
                  }
                >
                  {allowedAggregations(column.kind).map((mode) => (
                    <option key={mode} value={mode}>
                      {AGGREGATION_LABELS[mode]}
                    </option>
                  ))}
                </select>
              </div>
            )
          })}
        </div>
        <div className="filter-actions">
          <button className="ghost" onClick={exportCsv}>
            <Icon name="download" /> خروجی CSV
          </button>
          <button className="ghost" onClick={exportExcel}>
            <Icon name="download" /> خروجی اکسل
          </button>
          <button className="ghost" onClick={print}>
            <Icon name="file" /> چاپ / PDF
          </button>
        </div>
      </div>

      {saved.length > 0 && (
        <div className="panel">
          <div className="panel-head">
            <div>
              <h3>گزارش‌های ذخیره‌شده</h3>
              <p>{saved.length} گزارش</p>
            </div>
          </div>
          <div className="saved-list">
            {saved.map((item) => (
              <div className="saved-chip" key={item.id}>
                <button onClick={() => loadSaved(item)}>
                  <b>{item.name}</b>
                  <small>{SOURCE_LABELS[item.source as Source] ?? item.source}</small>
                </button>
                <button
                  className="icon-btn danger-icon"
                  aria-label="حذف"
                  onClick={async () => {
                    try {
                      await deleteCustomReport(item.id)
                      await refreshSaved()
                    } catch (e) {
                      setError(errorText(e))
                    }
                  }}
                >
                  <Icon name="trash" size={15} />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>پیش‌نمایش</h3>
            <p>
              {report.rowCount} ردیف
              {config.groupKey ? ` در ${report.groups.length} گروه` : ''}
            </p>
          </div>
        </div>
        {loading ? (
          <div className="empty-state">در حال آماده‌سازی…</div>
        ) : selected.length === 0 ? (
          <div className="empty-state">حداقل یک ستون انتخاب کنید.</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  {selected.map((column) => (
                    <th key={column.key}>{column.label}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {report.groups.map((group) => (
                  <>
                    {group.key && (
                      <tr className="group-row" key={`h-${group.key}`}>
                        <td colSpan={selected.length}>
                          <b>{group.key}</b> — {group.rows.length} ردیف
                        </td>
                      </tr>
                    )}
                    {group.rows.map((row, index) => (
                      <tr key={`${group.key}-${index}`}>
                        {selected.map((column) => (
                          <td
                            key={column.key}
                            className={
                              column.kind === 'money' || column.kind === 'quantity' ? 'num' : ''
                            }
                          >
                            {formatCell(column, row[column.key])}
                          </td>
                        ))}
                      </tr>
                    ))}
                    {group.key && (
                      <tr className="subtotal-row" key={`t-${group.key}`}>
                        {selected.map((column) => (
                          <td
                            key={column.key}
                            className={
                              column.kind === 'money' || column.kind === 'quantity' ? 'num' : ''
                            }
                          >
                            {formatTotal(column, group.totals[column.key])}
                          </td>
                        ))}
                      </tr>
                    )}
                  </>
                ))}
                {report.rowCount === 0 && (
                  <tr>
                    <td colSpan={selected.length} className="empty-row">
                      داده‌ای با این فیلترها یافت نشد.
                    </td>
                  </tr>
                )}
                {report.rowCount > 0 && (
                  <tr className="total-row">
                    {selected.map((column) => (
                      <td
                        key={column.key}
                        className={
                          column.kind === 'money' || column.kind === 'quantity' ? 'num' : ''
                        }
                      >
                        {formatTotal(column, report.grandTotals[column.key]) || 'جمع کل'}
                      </td>
                    ))}
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  )
}

/** برای استفاده‌ی تست‌ها — جمع‌بندی مستقیم بدون رندر. */
export const __testables = { aggregate, buildReport, COLUMNS }
