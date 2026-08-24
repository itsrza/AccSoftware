import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  convertQuote,
  getParties,
  getProducts,
  getQuote,
  getQuoteTransitions,
  getQuotes,
  getWarehouses,
  previewQuote,
  saveQuote,
  setQuoteStatus,
  Product,
  QuoteDetail,
  QuoteLineInput,
  QuotePreview,
  QuoteRow,
  QuoteTransition,
  Warehouse,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import {useI18n} from '../lib/i18n'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

type Kind = 'sales_quote' | 'purchase_order'
type EditableLine = QuoteLineInput & { key: number }

let nextKey = 1
const blankLine = (): EditableLine => ({ key: nextKey++, product_id: '', quantity: 1, unit_price: 0 })

/**
 * پیش‌فاکتور فروش و سفارش خرید.
 *
 * هر دو **تعهد** هستند، نه رویداد مالی: سند حسابداری نمی‌سازند و موجودی را
 * تغییر نمی‌دهند. اثر مالی فقط در لحظه‌ی تبدیل به فاکتور متولد می‌شود.
 */
export function Quotes({ kind }: { kind: Kind }) {
  const { t } = useI18n()
  const sales = kind === 'sales_quote'
  const [rows, setRows] = useState<QuoteRow[]>([])
  const [products, setProducts] = useState<Product[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])
  const [parties, setParties] = useState<{ id: string; name: string }[]>([])
  const [statusFilter, setStatusFilter] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [issueDate, setIssueDate] = useState('')
  const [validUntil, setValidUntil] = useState('')
  const [contactId, setContactId] = useState('')
  const [warehouseId, setWarehouseId] = useState('')
  const [description, setDescription] = useState('')
  const [vat, setVat] = useState(900)
  const [lines, setLines] = useState<EditableLine[]>([blankLine()])
  const [preview, setPreview] = useState<QuotePreview>()
  const [detail, setDetail] = useState<QuoteDetail>()
  const [transitions, setTransitions] = useState<QuoteTransition[]>([])
  const [convertDate, setConvertDate] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setRows(await getQuotes(kind, statusFilter || undefined))
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [kind, statusFilter])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    ;(async () => {
      try {
        setProducts(await getProducts())
        setWarehouses(await getWarehouses())
        const list = await getParties()
        setParties(
          list.rows
            .filter((p) => (sales ? p.is_customer : p.is_supplier))
            .map((p) => ({ id: p.id, name: p.display_name })),
        )
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [sales])

  // پیش‌نمایش جمع‌ها از موتور می‌آید، نه از محاسبه‌ی مرورگر.
  useEffect(() => {
    const payload = lines
      .filter((line) => line.product_id && line.quantity > 0)
      .map(({ key: _key, ...rest }) => rest)
    if (payload.length === 0) {
      setPreview(undefined)
      return
    }
    let cancelled = false
    const timer = setTimeout(async () => {
      try {
        const result = await previewQuote(payload, vat)
        if (!cancelled) setPreview(result)
      } catch (e) {
        if (!cancelled) {
          setPreview(undefined)
          setError(errorText(e))
        }
      }
    }, 250)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [lines, vat])

  const setLine = (key: number, patch: Partial<QuoteLineInput>) =>
    setLines((current) =>
      current.map((line) => {
        if (line.key !== key) return line
        const next = { ...line, ...patch }
        // با انتخاب کالا، قیمت پیشنهادی خودکار پر می‌شود.
        if (patch.product_id) {
          const product = products.find((p) => p.id === patch.product_id)
          if (product) next.unit_price = sales ? product.sale_price : product.purchase_price
        }
        return next
      }),
    )

  const resetForm = () => {
    setIssueDate('')
    setValidUntil('')
    setContactId('')
    setWarehouseId('')
    setDescription('')
    setLines([blankLine()])
    setPreview(undefined)
  }

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      const payload = lines
        .filter((line) => line.product_id && line.quantity > 0)
        .map(({ key: _key, ...rest }) => rest)
      await saveQuote({
        kind,
        issue_date: issueDate,
        valid_until: validUntil || undefined,
        contact_id: contactId,
        warehouse_id: warehouseId || undefined,
        description: description || undefined,
        vat_basis_points: vat,
        lines: payload,
      })
      setNotice(sales ? t('quotes.savedQuote') : t('quotes.savedOrder'))
      setFormOpen(false)
      resetForm()
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const openDetail = async (row: QuoteRow) => {
    try {
      setDetail(await getQuote(row.id))
      setTransitions(await getQuoteTransitions(row.id))
      setConvertDate(row.issue_date)
    } catch (e) {
      setError(errorText(e))
    }
  }

  const applyStatus = async (status: string) => {
    if (!detail) return
    setBusy(true)
    try {
      await setQuoteStatus(detail.header.id, status)
      setDetail(undefined)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const doConvert = async () => {
    if (!detail) return
    setBusy(true)
    try {
      const invoiceId = await convertQuote(detail.header.id, convertDate)
      setNotice(
        `فاکتور پیش‌نویس ساخته شد (${invoiceId}). قیمت‌ها و موجودی را بررسی و سپس ثبت قطعی کنید.`,
      )
      setDetail(undefined)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const { sorted, sortProps } = useSort(rows, 'number')
  const canSubmit =
    !busy &&
    issueDate.trim() !== '' &&
    contactId !== '' &&
    !!preview &&
    preview.total >= 0 &&
    lines.some((line) => line.product_id && line.quantity > 0)

  const accepted = rows.filter((r) => r.status === 'accepted')
  const expiring = rows.filter((r) => r.is_expired && r.status !== 'converted')

  const statusTone = (row: QuoteRow) => {
    if (row.status === 'converted') return 'done'
    if (row.status === 'rejected' || row.status === 'cancelled') return 'neutral'
    if (row.is_expired) return 'danger'
    return 'pending'
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{sales ? t('nav.sales') : t('nav.purchase')}</div>
          <h1>{sales ? t('page.proforma') : t('quotes.orders')}</h1>
          <p>
            {sales ? t('quotes.quote') : t('quotes.order')} تعهد است نه رویداد مالی: سند حسابداری نمی‌سازد و
            موجودی را تغییر نمی‌دهد. اثر مالی فقط با تبدیل به فاکتور ایجاد می‌شود.
          </p>
        </div>
        <button
          className="primary"
          onClick={() => {
            resetForm()
            setFormOpen(true)
          }}
        >
          <Icon name="plus" /> {sales ? t('quotes.newQuote') : t('quotes.newOrder')}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="metric-strip">
        <div>
          <span>{t('quotes.all')}</span>
          <b>{rows.length}</b>
          <small>{sales ? t('quotes.quote') : t('quotes.orderShort')}</small>
        </div>
        <div>
          <span>{t('quotes.accepted')}</span>
          <b className="green-text">{accepted.length}</b>
          <small>{t('quotes.readyToConvert')}</small>
        </div>
        <div>
          <span>{t('quotes.acceptedValue')}</span>
          <b>{money(accepted.reduce((sum, r) => sum + r.total, 0))} ریال</b>
          <small>{t('quotes.commitmentNote')}</small>
        </div>
        <div>
          <span>{t('quotes.expired')}</span>
          <b className={expiring.length > 0 ? 'red-text' : ''}>{expiring.length}</b>
          <small>{t('quotes.expiredValidity')}</small>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>{t('quotes.list')}</h3>
            <p>{sorted.length} مورد — برای جزئیات و تغییر وضعیت روی ردیف کلیک کنید.</p>
          </div>
          <div className="filter-actions">
            <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">{t('invoices.allDocStatuses')}</option>
              <option value="draft">{t('quotes.status.draft')}</option>
              <option value="sent">{t('quotes.status.sent')}</option>
              <option value="accepted">{t('quotes.accepted')}</option>
              <option value="rejected">{t('quotes.status.rejected')}</option>
              <option value="converted">{t('quotes.status.converted')}</option>
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
                <th {...sortProps('issue_date')}>{t('quotes.issueDate')}</th>
                <th {...sortProps('valid_until')}>{t('quotes.validUntil')}</th>
                <th {...sortProps('contact_name')}>{t('common.party')}</th>
                <th>{t('inv.items')}</th>
                <th {...sortProps('subtotal')}>{t('common.amount')}</th>
                <th>{t('invoiceForm.discount')}</th>
                <th>{t('common.tax')}</th>
                <th {...sortProps('total')}>{t('common.grandTotal')}</th>
                <th {...sortProps('status')}>{t('common.status')}</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((row) => (
                <tr key={row.id} className="clickable" onClick={() => openDetail(row)}>
                  <td className="code">{row.number}</td>
                  <td>{row.issue_date}</td>
                  <td>{row.valid_until ?? '—'}</td>
                  <td>{row.contact_name ?? '—'}</td>
                  <td className="num">{row.line_count}</td>
                  <td className="num">{money(row.subtotal)}</td>
                  <td className="num">{row.discount ? money(row.discount) : '—'}</td>
                  <td className="num">{row.tax ? money(row.tax) : '—'}</td>
                  <td className="num">{money(row.total)}</td>
                  <td>
                    <span className={`status ${statusTone(row)}`}>
                      {row.is_expired && row.status !== 'converted' ? t('quotes.expired') : row.status_label}
                    </span>
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={10} className="empty-row">
                    {t('quotes.emptyRow')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {formOpen && (
        <div className="modal-backdrop" role="presentation">
          <div className="modal party-modal">
            <div className="modal-head">
              <div>
                <h2>{sales ? t('quotes.newQuote') : t('quotes.newOrderTitle')}</h2>
                <p>{t('quotes.taxNote')}</p>
              </div>
              <button aria-label={t('common.close')} className="icon-btn" onClick={() => setFormOpen(false)}>
                <Icon name="close" />
              </button>
            </div>

            <div className="tab-body">
              <div className="filter-grid">
                <label>
                  <span>{t('quotes.issueDateRequired')}</span>
                  <input
                    value={issueDate}
                    onChange={(e) => setIssueDate(e.target.value)}
                    placeholder="1405/06/01"
                  />
                </label>
                <label>
                  <span>{t('quotes.validUntil')}</span>
                  <input
                    value={validUntil}
                    onChange={(e) => setValidUntil(e.target.value)}
                    placeholder="1405/07/01"
                  />
                </label>
                <label className="grow">
                  <span>{sales ? t('partyForm.customer') : t('partyForm.supplier')} *</span>
                  <Select value={contactId} onChange={(e) => setContactId(e.target.value)}>
                    <option value="">{t('invoiceForm.selectPlaceholder')}</option>
                    {parties.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </Select>
                </label>
                <label>
                  <span>{t('common.warehouse')}</span>
                  <Select value={warehouseId} onChange={(e) => setWarehouseId(e.target.value)}>
                    <option value="">{t('quotes.notSet')}</option>
                    {warehouses.map((w) => (
                      <option key={w.id} value={w.id}>
                        {w.name}
                      </option>
                    ))}
                  </Select>
                </label>
                <label>
                  <span>{t('quotes.taxRate')}</span>
                  <Select value={vat} onChange={(e) => setVat(Number(e.target.value))}>
                    <option value={0}>{t('quotes.exempt')}</option>
                    <option value={900}>{t('quotes.vat9')}</option>
                    <option value={1000}>{t('quotes.vat10')}</option>
                  </Select>
                </label>
                <label className="grow">
                  <span>{t('transfer.note')}</span>
                  <input value={description} onChange={(e) => setDescription(e.target.value)} />
                </label>
              </div>

              <div className="repeat-head">
                <h4 className="section-title">{t('inv.items')}</h4>
                <button className="ghost" onClick={() => setLines((c) => [...c, blankLine()])}>
                  <Icon name="plus" /> {t('quotes.addLine')}
                </button>
              </div>
              {lines.map((line) => (
                <div className="line-row" key={line.key}>
                  <label className="grow">
                    <span>{t('invoiceForm.product')}</span>
                    <Select
                      value={line.product_id}
                      onChange={(e) => setLine(line.key, { product_id: e.target.value })}
                    >
                      <option value="">{t('invoiceForm.selectPlaceholder')}</option>
                      {products.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </Select>
                  </label>
                  <label>
                    <span>{t('common.quantity')}</span>
                    <input
                      type="number"
                      min={0}
                      step="any"
                      value={line.quantity || ''}
                      onChange={(e) =>
                        setLine(line.key, { quantity: Number(e.target.value) || 0 })
                      }
                    />
                  </label>
                  <label>
                    <span>{t('quotes.unitPrice')}</span>
                    <input
                      type="number"
                      min={0}
                      value={line.unit_price || ''}
                      onChange={(e) =>
                        setLine(line.key, { unit_price: Number(e.target.value) || 0 })
                      }
                    />
                  </label>
                  <label>
                    <span>{t('quotes.lineDiscount')}</span>
                    <input
                      type="number"
                      min={0}
                      value={line.discount || ''}
                      onChange={(e) => setLine(line.key, { discount: Number(e.target.value) || 0 })}
                    />
                  </label>
                  <button aria-label={t('quotes.removeLine')}
                    className="icon-btn danger-icon"
                    disabled={lines.length === 1}
                    onClick={() =>
                      setLines((current) => current.filter((item) => item.key !== line.key))
                    }
                   
                  >
                    <Icon name="trash" />
                  </button>
                </div>
              ))}

              {preview && (
                <div className="inline-summary">
                  <span>
                    {t('quotes.sum')} <b>{money(preview.subtotal)}</b>
                  </span>
                  <span>
                    {t('quotes.discount')} <b>{money(preview.discount)}</b>
                  </span>
                  <span>
                    {t('quotes.net')} <b>{money(preview.net)}</b>
                  </span>
                  <span>
                    {t('quotes.tax')} <b>{money(preview.tax)}</b>
                  </span>
                  <span>
                    {t('quotes.payable')} <b>{money(preview.total)} ریال</b>
                  </span>
                </div>
              )}
            </div>

            <div className="modal-actions">
              <button className="primary" onClick={submit} disabled={!canSubmit}>
                {t('quotes.save')}
              </button>
              <button className="ghost" onClick={() => setFormOpen(false)}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}

      {detail && (
        <div className="modal-backdrop" onClick={() => setDetail(undefined)}>
          <div className="modal form-modal">
            <div className="modal-head">
              <div>
                <h2>
                  {detail.header.kind_label} شماره {detail.header.number}
                </h2>
                <p>
                  {detail.header.issue_date} — {detail.header.contact_name ?? t('invoices.noParty')}
                  {detail.header.is_expired && t('quotes.expiredSuffix')}
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
                  <th>{t('quotes.unitPrice')}</th>
                  <th>{t('invoiceForm.discount')}</th>
                  <th>{t('common.tax')}</th>
                  <th>{t('common.total')}</th>
                </tr>
              </thead>
              <tbody>
                {detail.lines.map((line) => (
                  <tr key={line.id}>
                    <td>{line.product_name}</td>
                    <td className="num">
                      {line.quantity} {line.unit}
                    </td>
                    <td className="num">{money(line.unit_price)}</td>
                    <td className="num">{line.discount ? money(line.discount) : '—'}</td>
                    <td className="num">{line.tax ? money(line.tax) : '—'}</td>
                    <td className="num">{money(line.line_total)}</td>
                  </tr>
                ))}
                <tr className="total-row">
                  <td colSpan={5}>{t('common.grandTotal')}</td>
                  <td className="num">{money(detail.header.total)}</td>
                </tr>
              </tbody>
            </table>

            {detail.header.converted_invoice_id ? (
              <p className="muted">
                این سند به فاکتور تبدیل شده است ({detail.header.converted_invoice_id}) و دیگر
                قابل تغییر نیست.
              </p>
            ) : (
              <>
                <h3 className="section-title">{t('quotes.changeStatus')}</h3>
                {transitions.length === 0 ? (
                  <p className="muted">{t('quotes.finalStatus')}</p>
                ) : (
                  <div className="transition-list">
                    {transitions.map((option) => (
                      <button
                        key={option.status}
                        className="transition-btn"
                        disabled={busy}
                        onClick={() => applyStatus(option.status)}
                      >
                        <b>{option.label}</b>
                      </button>
                    ))}
                  </div>
                )}

                {detail.header.status === 'accepted' && (
                  <>
                    <h3 className="section-title">{t('quotes.convert')}</h3>
                    <div className="filter-grid">
                      <label>
                        <span>{t('quotes.invoiceDate')}</span>
                        <input
                          value={convertDate}
                          onChange={(e) => setConvertDate(e.target.value)}
                        />
                      </label>
                      <p className="hint">
                        {t('quotes.convertNotePrefix')} <b>{t('quotes.status.draft')}</b> {t('quotes.convertNote')}
                      </p>
                    </div>
                    <div className="modal-actions">
                      <button
                        className="primary"
                        onClick={doConvert}
                        disabled={busy || !convertDate.trim()}
                      >
                        {t('quotes.createDraftInvoice')}
                      </button>
                    </div>
                  </>
                )}
              </>
            )}
          </div>
        </div>
      )}
    </section>
  )
}
