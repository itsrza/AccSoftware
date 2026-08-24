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
import { REPORT_PRINT_STYLE } from '../lib/printStyle'
import { formatNumber, formatRials as money, todayJalali } from '../lib/format'
import {storedLocale, translate, useI18n, type TranslationKey} from '../lib/i18n'
import {
  aggregate,
  allowedAggregations,
  buildReport,
  AGGREGATION_LABELS,
  Aggregation,
  ReportColumn,
  ReportRow,
} from '../lib/reportEngine'
import {Select} from '../components/Select'

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

const SOURCE_LABEL_KEYS: Record<Source, TranslationKey> = {
  sales: 'reports.salesReport',
  purchase: 'reports.purchaseReport',
  inventory: 'reports.inventoryValue',
  ledger: 'reports.ledger',
}

/**
 * تعریف ستون‌ها با **نوع** و **جمع‌بندی پیش‌فرض**.
 *
 * نوع ستون تعیین می‌کند چطور نمایش داده شود و چه جمع‌بندی‌هایی برایش معنا
 * دارد. مثلاً «مانده» را نباید مثل «مبلغ» جمع زد و «میانگین بها» جمع‌پذیر
 * نیست — میانگینِ میانگین‌ها عدد بی‌معنایی است.
 */
/** ستون‌های هر منبع؛ عنوان‌ها کلید ترجمه‌اند و در زمان نمایش ترجمه می‌شوند. */
type ColumnDef = Omit<ReportColumn, 'label'> & { labelKey: TranslationKey }

const COLUMNS: Record<Source, ColumnDef[]> = {
  sales: [
    { key: 'date', labelKey: 'common.date', kind: 'date', aggregation: 'none' },
    { key: 'invoice_number', labelKey: 'rb.col.invoiceNumber', kind: 'text', aggregation: 'count' },
    { key: 'contact_name', labelKey: 'reports.customer', kind: 'text', aggregation: 'none' },
    { key: 'subtotal', labelKey: 'rb.col.netAmount', kind: 'money', aggregation: 'sum' },
    { key: 'discount', labelKey: 'invoiceForm.discount', kind: 'money', aggregation: 'sum' },
    { key: 'tax', labelKey: 'common.tax', kind: 'money', aggregation: 'sum' },
    { key: 'total', labelKey: 'common.grandTotal', kind: 'money', aggregation: 'sum' },
    { key: 'payment_status', labelKey: 'rb.col.paymentStatus', kind: 'text', aggregation: 'none' },
  ],
  purchase: [
    { key: 'date', labelKey: 'common.date', kind: 'date', aggregation: 'none' },
    { key: 'invoice_number', labelKey: 'rb.col.invoiceNumber', kind: 'text', aggregation: 'count' },
    { key: 'contact_name', labelKey: 'reports.supplier', kind: 'text', aggregation: 'none' },
    { key: 'subtotal', labelKey: 'rb.col.netAmount', kind: 'money', aggregation: 'sum' },
    { key: 'discount', labelKey: 'invoiceForm.discount', kind: 'money', aggregation: 'sum' },
    { key: 'tax', labelKey: 'common.tax', kind: 'money', aggregation: 'sum' },
    { key: 'total', labelKey: 'common.grandTotal', kind: 'money', aggregation: 'sum' },
    { key: 'payment_status', labelKey: 'rb.col.paymentStatus', kind: 'text', aggregation: 'none' },
  ],
  inventory: [
    { key: 'product_name', labelKey: 'invoiceForm.product', kind: 'text', aggregation: 'count' },
    { key: 'warehouse_name', labelKey: 'common.warehouse', kind: 'text', aggregation: 'none' },
    { key: 'quantity', labelKey: 'products.stock', kind: 'quantity', aggregation: 'sum' },
    // میانگین بها جمع‌پذیر نیست؛ جمع آن عدد بی‌معنایی می‌سازد.
    { key: 'average_cost', labelKey: 'rb.col.averageCost', kind: 'money', aggregation: 'average' },
    { key: 'value', labelKey: 'rb.col.stockValue', kind: 'money', aggregation: 'sum' },
  ],
  ledger: [
    { key: 'code', labelKey: 'reports.accountCode', kind: 'text', aggregation: 'count' },
    { key: 'name', labelKey: 'rb.col.accountName', kind: 'text', aggregation: 'none' },
    { key: 'debit', labelKey: 'reports.debit', kind: 'money', aggregation: 'sum' },
    { key: 'credit', labelKey: 'reports.credit', kind: 'money', aggregation: 'sum' },
    { key: 'balance', labelKey: 'reports.balance', kind: 'money', aggregation: 'sum' },
  ],
}

/** ستون‌های ترجمه‌شده‌ی یک منبع. */
const columnsOf = (source: Source, translateKey: (key: TranslationKey) => string): ReportColumn[] =>
  COLUMNS[source].map((column) => ({ ...column, label: translateKey(column.labelKey) }))

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
  const { t } = useI18n()
  const [source, setSource] = useState<Source>('sales')
  const [rows, setRows] = useState<ReportRow[]>([])
  const [config, setConfig] = useState<Config>(defaultConfig('sales'))
  // نام پیش‌فرض به زبان فعال؛ کاربر می‌تواند عوضش کند و همان ذخیره می‌شود.
  const [name, setName] = useState(() => translate(storedLocale(), 'rb.customReport'))
  const [saved, setSaved] = useState<
    { id: string; name: string; source: string; config_json: string }[]
  >([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const available = columnsOf(source, t)

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
      setNotice(t('rb.saved'))
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
      setError(t('rb.invalidConfig'))
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
      setError(t('rb.printBlocked'))
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
        `<style>${REPORT_PRINT_STYLE}</style></head><body>` +
        `<h1>${escapeHtml(name)}</h1><p>${t(SOURCE_LABEL_KEYS[source])} — از ${config.from || t('rb.periodStart')} تا ${config.to || t('rb.today')} — ${report.rowCount} ردیف</p>` +
        `<table><thead><tr>${head}</tr></thead><tbody>${bodyRows}${grand}</tbody></table>` +
        `<script>window.onload=()=>window.print()</script></body></html>`,
    )
    printWindow.document.close()
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('nav.reports')}</div>
          <h1>{t('page.report-builder')}</h1>
          <p>
            {t('rb.subtitle')}
          </p>
        </div>
        <div className="filter-actions">
          <button className="ghost" onClick={() => loadSource(source, config.from, config.to)}>
            <Icon name="refresh" /> {t('common.refresh')}
          </button>
          <button className="primary" onClick={save}>
            {t('rb.saveReport')}
          </button>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="panel">
        <div className="filter-grid">
          <label>
            <span>{t('rb.reportName')}</span>
            <input value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            <span>{t('rb.dataSource')}</span>
            <Select value={source} onChange={(e) => changeSource(e.target.value as Source)}>
              {Object.entries(SOURCE_LABEL_KEYS).map(([key, labelKey]) => (
                <option key={key} value={key}>
                  {t(labelKey)}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('rb.fromDate')}</span>
            <input
              value={config.from}
              onChange={(e) => setConfig({ ...config, from: e.target.value })}
              placeholder="1405/01/01"
            />
          </label>
          <label>
            <span>{t('rb.toDate')}</span>
            <input
              value={config.to}
              onChange={(e) => setConfig({ ...config, to: e.target.value })}
              placeholder={todayJalali()}
            />
          </label>
          <label className="grow">
            <span>{t('rb.searchAllColumns')}</span>
            <input
              value={config.search}
              onChange={(e) => setConfig({ ...config, search: e.target.value })}
              placeholder={t('common.searchShort')}
            />
          </label>
          <label>
            <span>{t('rb.sortBy')}</span>
            <Select
              value={config.sortKey}
              onChange={(e) => setConfig({ ...config, sortKey: e.target.value })}
            >
              <option value="">{t('rb.noSort')}</option>
              {available.map((column) => (
                <option key={column.key} value={column.key}>
                  {column.label}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('rb.direction')}</span>
            <Select
              value={config.sortDirection}
              onChange={(e) =>
                setConfig({ ...config, sortDirection: e.target.value as 'asc' | 'desc' })
              }
            >
              <option value="asc">{t('rb.ascending')}</option>
              <option value="desc">{t('rb.descending')}</option>
            </Select>
          </label>
          <label>
            <span>{t('rb.grouping')}</span>
            <Select
              value={config.groupKey}
              onChange={(e) => setConfig({ ...config, groupKey: e.target.value })}
            >
              <option value="">{t('rb.noGrouping')}</option>
              {available
                .filter((column) => column.kind === 'text' || column.kind === 'date')
                .map((column) => (
                  <option key={column.key} value={column.key}>
                    {column.label}
                  </option>
                ))}
            </Select>
          </label>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>{t('rb.columnsAndAggregates')}</h3>
            <p>
              {t('rb.averageNote')}
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
                <Select
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
                </Select>
              </div>
            )
          })}
        </div>
        <div className="filter-actions">
          <button className="ghost" onClick={exportCsv}>
            <Icon name="download" /> {t('rb.exportCsv')}
          </button>
          <button className="ghost" onClick={exportExcel}>
            <Icon name="download" /> {t('rb.exportExcel')}
          </button>
          <button className="ghost" onClick={print}>
            <Icon name="file" /> {t('rb.printPdf')}
          </button>
        </div>
      </div>

      {saved.length > 0 && (
        <div className="panel">
          <div className="panel-head">
            <div>
              <h3>{t('rb.savedReports')}</h3>
              <p>{saved.length} گزارش</p>
            </div>
          </div>
          <div className="saved-list">
            {saved.map((item) => (
              <div className="saved-chip" key={item.id}>
                <button onClick={() => loadSaved(item)}>
                  <b>{item.name}</b>
                  <small>
                    {SOURCE_LABEL_KEYS[item.source as Source]
                      ? t(SOURCE_LABEL_KEYS[item.source as Source])
                      : item.source}
                  </small>
                </button>
                <button
                  className="icon-btn danger-icon"
                  aria-label={t('partyForm.remove')}
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
            <h3>{t('rb.preview')}</h3>
            <p>
              {report.rowCount} ردیف
              {config.groupKey ? ` در ${report.groups.length} گروه` : ''}
            </p>
          </div>
        </div>
        {loading ? (
          <div className="empty-state">{t('rb.preparing')}</div>
        ) : selected.length === 0 ? (
          <div className="empty-state">{t('rb.pickColumn')}</div>
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
                      {t('rb.noRows')}
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
                        {formatTotal(column, report.grandTotals[column.key]) || t('common.grandTotal')}
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
