import {useEffect, useMemo, useState} from 'react'
import {
  getProductCardex,
  getProductsDetailed,
  getWarehouses,
  type CardexKindValue,
  type CardexReport,
  type ProductListRow,
  type Warehouse,
} from '../api'
import {errorText} from '../lib/errors'
import {formatNumber, formatRials as money, toJalali} from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'
import {parseJalali} from '../lib/dateRange'
import {Select} from '../components/Select'

/**
 * کاردکس کالا — گزارش حرکات کالا (F4 فروش / F5 خرید / F6 کلی).
 *
 * مرجع: لیست کالاهای نرم‌افزار فعلی (تصویر `8Xmc1p`).
 *
 * ## چرا ماندِ هر سطر از ابتدای تاریخِ کالا شروع می‌شود
 *
 * هسته افتتاحیه‌ی قبل از بازه را جدا حساب می‌کند و سربرگ نشان می‌دهد؛
 * ماندِ سطری همان افتتاحیه + جمع تجمعی است. اگر فقط از ابتدای بازه شروع
 * می‌شد، عوض‌کردن بازه، ماند را «از صفر» نشان می‌داد و کاربر گمان می‌کرد
 * انبار خالی شده است.
 *
 * ## چرا دکمه‌ی «نمایش» لازم است
 *
 * کاردکس روی همه‌ی حرکات کالا تا تاریخ «تا» پرس‌وجو می‌زند؛ اجرای خودکار
 * با هر کلید تاریخ یعنی بار اضافی روی پایگاه داده. کاربر صریح بخواهد.
 */

const KINDS: {value: CardexKindValue; labelKey: TranslationKey}[] = [
  {value: 'sales', labelKey: 'cardex.kind.sales'},
  {value: 'purchase', labelKey: 'cardex.kind.purchase'},
  {value: 'all', labelKey: 'cardex.kind.all'},
]

const DOC_LABELS: Record<string, TranslationKey> = {
  sales_invoice: 'cardex.doc.sales_invoice',
  purchase_invoice: 'cardex.doc.purchase_invoice',
  sales_return: 'cardex.doc.sales_return',
  purchase_return: 'cardex.doc.purchase_return',
  transfer: 'cardex.doc.transfer',
  inventory_count: 'cardex.doc.inventory_count',
  inventory_adjustment: 'cardex.doc.inventory_adjustment',
  opening: 'cardex.doc.opening',
}

const pad = (value: number) => String(value).padStart(2, '0')

function defaultRange(): {from: string; to: string} {
  const today = toJalali(new Date())
  return {
    from: `${today.year}/01/01`,
    to: `${today.year}/${pad(today.month)}/${pad(today.day)}`,
  }
}

export function ProductCardex({initial}: {initial?: {productId?: string; kind?: CardexKindValue}}) {
  const {t} = useI18n()
  const [products, setProducts] = useState<ProductListRow[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])
  const [productId, setProductId] = useState(initial?.productId ?? '')
  const [kind, setKind] = useState<CardexKindValue>(initial?.kind ?? 'all')
  const range = useMemo(defaultRange, [])
  const [from, setFrom] = useState(range.from)
  const [to, setTo] = useState(range.to)
  const [warehouseId, setWarehouseId] = useState('')
  const [report, setReport] = useState<CardexReport>()
  const [loading, setLoading] = useState(true)
  const [running, setRunning] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const [rows, whs] = await Promise.all([
          getProductsDetailed(),
          getWarehouses().catch(() => [] as Warehouse[]),
        ])
        if (cancelled) return
        setProducts(rows)
        setWarehouses(whs)
      } catch (e) {
        if (!cancelled) setError(errorText(e))
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const run = async () => {
    if (!productId) return
    if (!parseJalali(from) || !parseJalali(to)) {
      setError(t('cardex.badDate'))
      return
    }
    setRunning(true)
    setError('')
    try {
      setReport(
        await getProductCardex(productId, kind, from, to, warehouseId || undefined),
      )
    } catch (e) {
      setError(errorText(e))
      setReport(undefined)
    } finally {
      setRunning(false)
    }
  }

  // کالای آماده از صفحه‌ی کالاها → اولین نمایش خودکار
  const autoRan = useMemo(() => !initial?.productId, [initial?.productId])
  const [didAutoRun, setDidAutoRun] = useState(autoRan)
  useEffect(() => {
    if (didAutoRun || loading || !productId) return
    setDidAutoRun(true)
    run()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loading, didAutoRun, productId])

  const docLabel = (docKind: string) => DOC_LABELS[docKind] ?? 'cardex.doc.other'

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('cardex.title')}</div>
          <h1>{t('cardex.title')}</h1>
          <p>{t('cardex.subtitle')}</p>
        </div>
        <p className="hint">{t('cardex.hint')}</p>
      </div>

      {error && <div className="error-box">{error}</div>}

      <div className="panel list-panel">
        <div className="toolbar">
          <Select
            value={productId}
            aria-label={t('cardex.productRequired')}
            onChange={(event) => setProductId(event.target.value)}
          >
            <option value="">{t('cardex.noProduct')}</option>
            {products.map((row) => (
              <option key={row.id} value={row.id}>
                {row.sku} — {row.name}
              </option>
            ))}
          </Select>
          <Select
            value={warehouseId}
            aria-label={t('cardex.warehouse')}
            onChange={(event) => setWarehouseId(event.target.value)}
          >
            <option value="">{t('cardex.allWarehouses')}</option>
            {warehouses.map((wh) => (
              <option key={wh.id} value={wh.id}>
                {wh.name}
              </option>
            ))}
          </Select>
          <label className="inline-field">
            {t('cardex.from')}
            <input
              dir="ltr"
              value={from}
              onChange={(event) => setFrom(event.target.value)}
              placeholder="1404/01/01"
            />
          </label>
          <label className="inline-field">
            {t('cardex.to')}
            <input
              dir="ltr"
              value={to}
              onChange={(event) => setTo(event.target.value)}
              placeholder="1404/12/29"
            />
          </label>
          <button
            type="button"
            className="primary"
            disabled={running || loading || !productId}
            onClick={run}
          >
            {t('cardex.run')}
          </button>
        </div>

        <div className="tab-bar">
          {KINDS.map((item) => (
            <button
              key={item.value}
              type="button"
              className={kind === item.value ? 'active' : undefined}
              onClick={() => setKind(item.value)}
            >
              {t(item.labelKey)}
            </button>
          ))}
        </div>

        {loading ? (
          <div className="empty-state">{t('common.loading')}</div>
        ) : !report ? (
          <div className="empty-state">{t('cardex.noProduct')}</div>
        ) : report.entries.length === 0 ? (
          <div className="empty-state">{t('cardex.empty')}</div>
        ) : (
          <>
            <div className="stat-grid">
              <div className="stat-card">
                <span>{t('cardex.opening')}</span>
                <b>{formatNumber(report.opening_balance)}</b>
              </div>
              <div className="stat-card">
                <span>{t('cardex.totalIn')}</span>
                <b>{formatNumber(report.total_in)}</b>
              </div>
              <div className="stat-card">
                <span>{t('cardex.totalOut')}</span>
                <b>{formatNumber(report.total_out)}</b>
              </div>
              <div className="stat-card">
                <span>{t('cardex.closing')}</span>
                <b>{formatNumber(report.closing_balance)}</b>
              </div>
            </div>

            <div className="table-wrap">
              <table className="large-table">
                <thead>
                  <tr>
                    <th>{t('cardex.col.date')}</th>
                    <th>{t('cardex.col.doc')}</th>
                    <th>{t('cardex.col.warehouse')}</th>
                    <th className="num">{t('cardex.col.in')}</th>
                    <th className="num">{t('cardex.col.out')}</th>
                    <th className="num">{t('cardex.col.unitCost')}</th>
                    <th className="num">{t('cardex.col.value')}</th>
                    <th className="num">{t('cardex.col.balance')}</th>
                    <th>{t('cardex.col.note')}</th>
                  </tr>
                </thead>
                <tbody>
                  {report.entries.map((entry, index) => (
                    <tr key={`${entry.date_iso}-${index}`}>
                      <td dir="ltr">{entry.date_jalali}</td>
                      <td>
                        {t(docLabel(entry.doc_kind))}
                        {entry.doc_number !== null && <> №{formatNumber(entry.doc_number)}</>}
                      </td>
                      <td>{entry.warehouse_name}</td>
                      <td className="num">{entry.flow === 'in' ? formatNumber(entry.quantity) : '—'}</td>
                      <td className="num">{entry.flow === 'out' ? formatNumber(entry.quantity) : '—'}</td>
                      <td className="num">{entry.unit_cost ? money(entry.unit_cost) : '—'}</td>
                      <td className="num">{money(entry.value)}</td>
                      <td className="num">{formatNumber(entry.balance)}</td>
                      <td>{entry.note ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p className="hint">
              {report.product_name} — {t('common.unit')}: {report.product_unit}
            </p>
          </>
        )}
      </div>
    </section>
  )
}
