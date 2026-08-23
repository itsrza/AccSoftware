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
      setNotice(sales ? 'پیش‌فاکتور ثبت شد.' : 'سفارش خرید ثبت شد.')
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
          <div className="eyebrow">{sales ? 'فروش' : 'خرید'}</div>
          <h1>{sales ? 'پیش‌فاکتورها' : 'سفارش‌های خرید'}</h1>
          <p>
            {sales ? 'پیش‌فاکتور' : 'سفارش خرید'} تعهد است نه رویداد مالی: سند حسابداری نمی‌سازد و
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
          <Icon name="plus" /> {sales ? 'پیش‌فاکتور جدید' : 'سفارش جدید'}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="metric-strip">
        <div>
          <span>کل</span>
          <b>{rows.length}</b>
          <small>{sales ? 'پیش‌فاکتور' : 'سفارش'}</small>
        </div>
        <div>
          <span>پذیرفته‌شده</span>
          <b className="green-text">{accepted.length}</b>
          <small>آماده‌ی تبدیل به فاکتور</small>
        </div>
        <div>
          <span>ارزش پذیرفته‌شده‌ها</span>
          <b>{money(accepted.reduce((sum, r) => sum + r.total, 0))} ریال</b>
          <small>تعهد، نه درآمد</small>
        </div>
        <div>
          <span>منقضی</span>
          <b className={expiring.length > 0 ? 'red-text' : ''}>{expiring.length}</b>
          <small>تاریخ اعتبار گذشته</small>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>فهرست</h3>
            <p>{sorted.length} مورد — برای جزئیات و تغییر وضعیت روی ردیف کلیک کنید.</p>
          </div>
          <div className="filter-actions">
            <Select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
              <option value="">همه‌ی وضعیت‌ها</option>
              <option value="draft">پیش‌نویس</option>
              <option value="sent">ارسال‌شده</option>
              <option value="accepted">پذیرفته‌شده</option>
              <option value="rejected">ردشده</option>
              <option value="converted">تبدیل‌شده</option>
              <option value="cancelled">باطل‌شده</option>
            </Select>
            <button className="icon-btn" onClick={load} aria-label="بروزرسانی">
              <Icon name="refresh" />
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('number')}>شماره</th>
                <th {...sortProps('issue_date')}>تاریخ صدور</th>
                <th {...sortProps('valid_until')}>اعتبار تا</th>
                <th {...sortProps('contact_name')}>طرف حساب</th>
                <th>اقلام</th>
                <th {...sortProps('subtotal')}>مبلغ</th>
                <th>تخفیف</th>
                <th>مالیات</th>
                <th {...sortProps('total')}>جمع کل</th>
                <th {...sortProps('status')}>وضعیت</th>
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
                      {row.is_expired && row.status !== 'converted' ? 'منقضی' : row.status_label}
                    </span>
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={10} className="empty-row">
                    موردی ثبت نشده است.
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
                <h2>{sales ? 'پیش‌فاکتور جدید' : 'سفارش خرید جدید'}</h2>
                <p>مالیات روی مبلغ پس از تخفیف محاسبه می‌شود.</p>
              </div>
              <button aria-label="بستن" className="icon-btn" onClick={() => setFormOpen(false)}>
                <Icon name="close" />
              </button>
            </div>

            <div className="tab-body">
              <div className="filter-grid">
                <label>
                  <span>تاریخ صدور *</span>
                  <input
                    value={issueDate}
                    onChange={(e) => setIssueDate(e.target.value)}
                    placeholder="1405/06/01"
                  />
                </label>
                <label>
                  <span>اعتبار تا</span>
                  <input
                    value={validUntil}
                    onChange={(e) => setValidUntil(e.target.value)}
                    placeholder="1405/07/01"
                  />
                </label>
                <label className="grow">
                  <span>{sales ? 'مشتری' : 'تأمین‌کننده'} *</span>
                  <Select value={contactId} onChange={(e) => setContactId(e.target.value)}>
                    <option value="">انتخاب کنید…</option>
                    {parties.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </Select>
                </label>
                <label>
                  <span>انبار</span>
                  <Select value={warehouseId} onChange={(e) => setWarehouseId(e.target.value)}>
                    <option value="">تعیین نشده</option>
                    {warehouses.map((w) => (
                      <option key={w.id} value={w.id}>
                        {w.name}
                      </option>
                    ))}
                  </Select>
                </label>
                <label>
                  <span>نرخ مالیات</span>
                  <Select value={vat} onChange={(e) => setVat(Number(e.target.value))}>
                    <option value={0}>معاف</option>
                    <option value={900}>۹٪</option>
                    <option value={1000}>۱۰٪</option>
                  </Select>
                </label>
                <label className="grow">
                  <span>توضیح</span>
                  <input value={description} onChange={(e) => setDescription(e.target.value)} />
                </label>
              </div>

              <div className="repeat-head">
                <h4 className="section-title">اقلام</h4>
                <button className="ghost" onClick={() => setLines((c) => [...c, blankLine()])}>
                  <Icon name="plus" /> افزودن قلم
                </button>
              </div>
              {lines.map((line) => (
                <div className="line-row" key={line.key}>
                  <label className="grow">
                    <span>کالا</span>
                    <Select
                      value={line.product_id}
                      onChange={(e) => setLine(line.key, { product_id: e.target.value })}
                    >
                      <option value="">انتخاب کنید…</option>
                      {products.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </Select>
                  </label>
                  <label>
                    <span>مقدار</span>
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
                    <span>قیمت واحد</span>
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
                    <span>تخفیف سطر</span>
                    <input
                      type="number"
                      min={0}
                      value={line.discount || ''}
                      onChange={(e) => setLine(line.key, { discount: Number(e.target.value) || 0 })}
                    />
                  </label>
                  <button aria-label="حذف قلم"
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
                    جمع: <b>{money(preview.subtotal)}</b>
                  </span>
                  <span>
                    تخفیف: <b>{money(preview.discount)}</b>
                  </span>
                  <span>
                    خالص: <b>{money(preview.net)}</b>
                  </span>
                  <span>
                    مالیات: <b>{money(preview.tax)}</b>
                  </span>
                  <span>
                    قابل پرداخت: <b>{money(preview.total)} ریال</b>
                  </span>
                </div>
              )}
            </div>

            <div className="modal-actions">
              <button className="primary" onClick={submit} disabled={!canSubmit}>
                ثبت
              </button>
              <button className="ghost" onClick={() => setFormOpen(false)}>
                انصراف
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
                  {detail.header.issue_date} — {detail.header.contact_name ?? 'بدون طرف حساب'}
                  {detail.header.is_expired && ' — تاریخ اعتبار گذشته است'}
                </p>
              </div>
              <button aria-label="بستن" className="icon-btn" onClick={() => setDetail(undefined)}>
                <Icon name="close" />
              </button>
            </div>

            <table className="mini-table">
              <thead>
                <tr>
                  <th>کالا</th>
                  <th>مقدار</th>
                  <th>قیمت واحد</th>
                  <th>تخفیف</th>
                  <th>مالیات</th>
                  <th>جمع</th>
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
                  <td colSpan={5}>جمع کل</td>
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
                <h3 className="section-title">تغییر وضعیت</h3>
                {transitions.length === 0 ? (
                  <p className="muted">این سند در وضعیت پایانی است.</p>
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
                    <h3 className="section-title">تبدیل به فاکتور</h3>
                    <div className="filter-grid">
                      <label>
                        <span>تاریخ فاکتور</span>
                        <input
                          value={convertDate}
                          onChange={(e) => setConvertDate(e.target.value)}
                        />
                      </label>
                      <p className="hint">
                        فاکتور به‌صورت <b>پیش‌نویس</b> ساخته می‌شود، نه ثبت‌شده — چون ممکن است
                        بین پیشنهاد و فروش، قیمت یا موجودی تغییر کرده باشد. اثر مالی فقط پس از
                        ثبت قطعی فاکتور ایجاد می‌شود.
                      </p>
                    </div>
                    <div className="modal-actions">
                      <button
                        className="primary"
                        onClick={doConvert}
                        disabled={busy || !convertDate.trim()}
                      >
                        ساخت فاکتور پیش‌نویس
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
