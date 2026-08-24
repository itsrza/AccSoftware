import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  cancelReturn,
  createPurchaseReturn,
  createSalesReturn,
  getPurchaseInvoices,
  getReturn,
  getReturnableLines,
  getReturns,
  getSalesInvoices,
  postPurchaseReturnV2,
  postSalesReturnV2,
  InvoiceSummary,
  ReturnDetail,
  ReturnRow,
  ReturnableLine,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import {useI18n} from '../lib/i18n'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

type Draft = Record<string, number>

/**
 * برگشت از فروش و برگشت از خرید.
 *
 * دو قاعده‌ای که این صفحه اجرا می‌کند:
 * ۱. مقدار برگشتی هرگز از باقیمانده‌ی فاکتور اصلی بیشتر نمی‌شود — کالایی که
 *    فروخته نشده، برگشت نمی‌خورد.
 * ۲. مالیات هم به نسبت برگشت داده می‌شود؛ وگرنه اظهارنامه‌ی ارزش افزوده
 *    اشتباه درمی‌آید.
 */
export function Returns({ sale }: { sale: boolean }) {
  const { t } = useI18n()
  const [invoices, setInvoices] = useState<InvoiceSummary[]>([])
  const [rows, setRows] = useState<ReturnRow[]>([])
  const [statusFilter, setStatusFilter] = useState('')
  const [selectedInvoice, setSelectedInvoice] = useState('')
  const [returnDate, setReturnDate] = useState('')
  const [candidates, setCandidates] = useState<ReturnableLine[]>([])
  const [draft, setDraft] = useState<Draft>({})
  const [detail, setDetail] = useState<ReturnDetail>()
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setRows(await getReturns(sale, statusFilter || undefined))
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [sale, statusFilter])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    ;(async () => {
      try {
        const list = sale ? await getSalesInvoices() : await getPurchaseInvoices()
        setInvoices(list.filter((i) => i.status === 'posted'))
      } catch (e) {
        setError(errorText(e))
      }
    })()
    // با تغییر نوع صفحه، پیش‌نویس قبلی باید پاک شود.
    setSelectedInvoice('')
    setCandidates([])
    setDraft({})
  }, [sale])

  useEffect(() => {
    if (!selectedInvoice) {
      setCandidates([])
      setDraft({})
      return
    }
    ;(async () => {
      try {
        const lines = await getReturnableLines(sale, selectedInvoice)
        setCandidates(lines)
        setDraft({})
        setError('')
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [selectedInvoice, sale])

  const setQuantity = (line: ReturnableLine, value: number) => {
    // سقف: باقیمانده‌ی قابل برگشت. بیشتر از این یعنی برگشت کالای نفروخته.
    const clamped = Math.max(0, Math.min(value, line.returnable_quantity))
    setDraft((current) => ({ ...current, [line.product_id]: clamped }))
  }

  const chosen = useMemo(
    () =>
      candidates
        .map((line) => ({ line, quantity: draft[line.product_id] ?? 0 }))
        .filter((item) => item.quantity > 0),
    [candidates, draft],
  )

  const draftTotal = chosen.reduce(
    (sum, item) => sum + Math.round(item.quantity * item.line.unit_price),
    0,
  )

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      const payload = chosen.map(
        (item) => [item.line.product_id, item.quantity, item.line.unit_price] as [string, number, number],
      )
      const id = sale
        ? await createSalesReturn(selectedInvoice, returnDate, payload)
        : await createPurchaseReturn(selectedInvoice, returnDate, payload)
      setNotice(`برگشت پیش‌نویس ساخته شد (${id}). برای اثر مالی، آن را ثبت قطعی کنید.`)
      setSelectedInvoice('')
      setDraft({})
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const finalize = async (row: ReturnRow) => {
    setBusy(true)
    try {
      if (sale) await postSalesReturnV2(row.id)
      else await postPurchaseReturnV2(row.id)
      setNotice(`برگشت شماره ${row.number} ثبت قطعی شد و سند حسابداری آن صادر شد.`)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const drop = async (row: ReturnRow) => {
    setBusy(true)
    try {
      await cancelReturn(sale, row.id)
      setNotice(`برگشت شماره ${row.number} باطل شد.`)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const { sorted, sortProps } = useSort(rows, 'number')
  const canSubmit = !busy && selectedInvoice !== '' && returnDate.trim() !== '' && chosen.length > 0

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{sale ? t('nav.sales') : t('nav.purchase')}</div>
          <h1>{sale ? t('returns.salesTitle') : t('returns.purchaseTitle')}</h1>
          <p>
            {sale
              ? t('returns.salesLead')
              : t('returns.purchaseLead')}
          </p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>{t('returns.new')}</h3>
            <p>{t('returns.pickInvoice')}</p>
          </div>
        </div>
        <div className="filter-grid">
          <label className="grow">
            <span>{t('returns.originalInvoiceRequired')}</span>
            <Select
              value={selectedInvoice}
              onChange={(e) => setSelectedInvoice(e.target.value)}
            >
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {invoices.map((invoice) => (
                <option key={invoice.id} value={invoice.id}>
                  شماره {invoice.number} — {invoice.invoice_date} — {money(invoice.total)} ریال
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('returns.dateRequired')}</span>
            <input
              value={returnDate}
              onChange={(e) => setReturnDate(e.target.value)}
              placeholder="1405/06/10"
            />
          </label>
        </div>

        {selectedInvoice && (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>{t('invoiceForm.product')}</th>
                  <th>{t('common.unit')}</th>
                  <th>{t('returns.invoiceQty')}</th>
                  <th>{t('returns.alreadyReturned')}</th>
                  <th>{t('returns.returnable')}</th>
                  <th>{t('returns.unitAmount')}</th>
                  <th>{t('returns.returnQty')}</th>
                  <th>{t('returns.lineTotal')}</th>
                </tr>
              </thead>
              <tbody>
                {candidates.map((line) => {
                  const quantity = draft[line.product_id] ?? 0
                  const exhausted = line.returnable_quantity <= 0
                  return (
                    <tr key={line.product_id} className={exhausted ? 'row-muted' : ''}>
                      <td>{line.product_name}</td>
                      <td>{line.unit}</td>
                      <td className="num">{line.invoiced_quantity}</td>
                      <td className="num">{line.returned_quantity || '—'}</td>
                      <td className="num">{line.returnable_quantity}</td>
                      <td className="num">{money(line.unit_price)}</td>
                      <td>
                        <input
                          className="qty-input"
                          type="number"
                          min={0}
                          max={line.returnable_quantity}
                          step="any"
                          disabled={exhausted}
                          value={quantity || ''}
                          onChange={(e) => setQuantity(line, Number(e.target.value) || 0)}
                        />
                      </td>
                      <td className="num">
                        {quantity > 0 ? money(Math.round(quantity * line.unit_price)) : '—'}
                      </td>
                    </tr>
                  )
                })}
                {candidates.length === 0 && (
                  <tr>
                    <td colSpan={8} className="empty-row">
                      {t('returns.nothingReturnable')}
                    </td>
                  </tr>
                )}
                {chosen.length > 0 && (
                  <tr className="total-row">
                    <td colSpan={7}>{t('returns.totalExclTax')}</td>
                    <td className="num">{money(draftTotal)}</td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        <div className="modal-actions">
          <button className="primary" onClick={submit} disabled={!canSubmit}>
            {t('returns.createDraft')}
          </button>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>{t('returns.list')}</h3>
            <p>{sorted.length} مورد</p>
          </div>
          <div className="filter-actions">
            <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">{t('invoices.allDocStatuses')}</option>
              <option value="draft">{t('quotes.status.draft')}</option>
              <option value="posted">{t('returns.status.posted')}</option>
              <option value="cancelled">{t('quotes.status.void')}</option>
            </Select>
            <button className="icon-btn" onClick={load} aria-label={t('common.refresh')}>
              <Icon name="refresh" />
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('number')}>{t('common.number')}</th>
                <th {...sortProps('return_date')}>{t('common.date')}</th>
                <th>{t('returns.originalInvoice')}</th>
                <th {...sortProps('contact_name')}>{t('common.party')}</th>
                <th>{t('common.warehouse')}</th>
                <th {...sortProps('total')}>{t('returns.netAmount')}</th>
                <th>{t('common.tax')}</th>
                <th {...sortProps('grand_total')}>{t('common.grandTotal')}</th>
                <th {...sortProps('status')}>{t('common.status')}</th>
                <th>{t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((row) => (
                <tr key={row.id}>
                  <td className="code">{row.number}</td>
                  <td>{row.return_date}</td>
                  <td>{row.original_invoice_number ?? '—'}</td>
                  <td>{row.contact_name ?? '—'}</td>
                  <td>{row.warehouse_name ?? '—'}</td>
                  <td className="num">{money(row.total)}</td>
                  <td className="num">{row.tax ? money(row.tax) : '—'}</td>
                  <td className="num">{money(row.grand_total)}</td>
                  <td>
                    <span
                      className={`status ${
                        row.status === 'posted'
                          ? 'done'
                          : row.status === 'cancelled'
                            ? 'neutral'
                            : 'pending'
                      }`}
                    >
                      {row.status_label}
                    </span>
                  </td>
                  <td>
                    <button
                      className="table-action"
                      onClick={async () => {
                        try {
                          setDetail(await getReturn(sale, row.id))
                        } catch (e) {
                          setError(errorText(e))
                        }
                      }}
                    >
                      {t('treasuryDoc.details')}
                    </button>
                    {row.status === 'draft' && (
                      <>
                        <button
                          className="table-action"
                          disabled={busy}
                          onClick={() => finalize(row)}
                        >
                          {t('returns.postFinal')}
                        </button>
                        <button className="table-action" disabled={busy} onClick={() => drop(row)}>
                          {t('returns.void')}
                        </button>
                      </>
                    )}
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={10} className="empty-row">
                    {t('returns.empty')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {detail && (
        <div className="modal-backdrop" onClick={() => setDetail(undefined)}>
          <div className="modal form-modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-head">
              <div>
                <h2>برگشت شماره {detail.header.number}</h2>
                <p>
                  {detail.header.return_date} — {detail.header.contact_name ?? t('invoices.noParty')}
                </p>
              </div>
              <button aria-label={t('common.close')} className="icon-btn" onClick={() => setDetail(undefined)}>
                <Icon name="close" />
              </button>
            </div>
            <table className="mini-table">
              <thead>
                <tr>
                  <th>{t('invoiceForm.product')}</th>
                  <th>{t('common.quantity')}</th>
                  <th>{t('returns.unitAmount')}</th>
                  <th>{t('common.total')}</th>
                </tr>
              </thead>
              <tbody>
                {detail.lines.map((line) => (
                  <tr key={line.id}>
                    <td>{line.product_name}</td>
                    <td className="num">{line.quantity}</td>
                    <td className="num">{money(line.unit_price)}</td>
                    <td className="num">{money(line.line_total)}</td>
                  </tr>
                ))}
                <tr>
                  <td colSpan={3}>{t('returns.reversedTax')}</td>
                  <td className="num">{money(detail.header.tax)}</td>
                </tr>
                <tr className="total-row">
                  <td colSpan={3}>{t('common.grandTotal')}</td>
                  <td className="num">{money(detail.header.grand_total)}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
    </section>
  )
}
