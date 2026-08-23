import {useCallback, useEffect, useMemo, useRef, useState} from 'react'
import {
  buildInstallmentPlan,
  createSalesInvoice,
  getContacts,
  getProducts,
  getWarehouses,
  postSalesInvoice,
  previewInvoice,
  type Contact,
  type InstallmentRow,
  type InvoicePreview,
  type Product,
  type Warehouse,
} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {formatNumber, formatRials, parseAmount, todayJalali} from '../lib/format'

/** یک سطر فاکتور در حال ویرایش */
type Line = {
  key: string
  product_id: string
  quantity: number
  unit_price: number
  discount_amount: number
  discount_bp: number
  vat_bp: number
  duty_bp: number
  commission_bp: number
  unit_cost: number
  serials: string[]
  serial_tracked: boolean
}

const emptyLine = (): Line => ({
  key: Math.random().toString(36).slice(2),
  product_id: '',
  quantity: 1,
  unit_price: 0,
  discount_amount: 0,
  discount_bp: 0,
  vat_bp: 900,
  duty_bp: 0,
  commission_bp: 0,
  unit_cost: 0,
  serials: [],
  serial_tracked: false,
})

/**
 * فرم صدور فاکتور فروش.
 *
 * بازطراحی کامل فرم `sFpxWK` نرم‌افزار فعلی. تمام جمع‌ها، سود فاکتور و مانده‌ی
 * طرف حساب از فرمان `preview_invoice` می‌آید — یعنی همان موتوری که هنگام ثبت
 * نهایی اجرا می‌شود. هیچ محاسبه‌ی مالی در لایه‌ی React انجام نمی‌شود.
 */
export function InvoiceForm() {
  const [products, setProducts] = useState<Product[]>([])
  const [contacts, setContacts] = useState<Contact[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])

  const [date, setDate] = useState(todayJalali())
  const [contactId, setContactId] = useState('')
  const [warehouseId, setWarehouseId] = useState('')
  const [lines, setLines] = useState<Line[]>([])
  const [headerDiscount, setHeaderDiscount] = useState('0')
  const [freight, setFreight] = useState('0')
  const [freightAllocated, setFreightAllocated] = useState(false)

  const [settlement, setSettlement] = useState({cash: '0', check: '0', transfer: '0', card: '0'})
  const [preview, setPreview] = useState<InvoicePreview | null>(null)
  const [installments, setInstallments] = useState<InstallmentRow[]>([])
  const [installmentCount, setInstallmentCount] = useState('3')
  const [showProfit, setShowProfit] = useState(false)

  const [editing, setEditing] = useState<Line | null>(null)
  const [selectedKey, setSelectedKey] = useState('')
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')
  const [saving, setSaving] = useState(false)

  const previewToken = useRef(0)

  useEffect(() => {
    const load = async () => {
      try {
        const [p, c, w] = await Promise.all([getProducts(), getContacts(), getWarehouses()])
        setProducts(p)
        setContacts(c)
        setWarehouses(w)
        if (w.length > 0) setWarehouseId(w[0].id)
      } catch (e) {
        setError(errorText(e))
      }
    }
    load()
  }, [])

  const received = useMemo(
    () =>
      (parseAmount(settlement.cash) ?? 0) +
      (parseAmount(settlement.check) ?? 0) +
      (parseAmount(settlement.transfer) ?? 0) +
      (parseAmount(settlement.card) ?? 0),
    [settlement],
  )

  /** محاسبه‌ی زنده از بک‌اند */
  const refreshPreview = useCallback(async () => {
    if (lines.length === 0 || lines.some((line) => !line.product_id)) {
      setPreview(null)
      return
    }
    const token = ++previewToken.current
    try {
      const result = await previewInvoice({
        lines: lines.map((line) => ({
          product_id: line.product_id,
          quantity: line.quantity,
          unit_price: line.unit_price,
          discount_amount: line.discount_amount,
          discount_bp: line.discount_bp,
          vat_bp: line.vat_bp,
          duty_bp: line.duty_bp,
          commission_bp: line.commission_bp,
          unit_cost: line.unit_cost,
          serials: line.serials,
          serial_tracked: line.serial_tracked,
        })),
        headerDiscount: parseAmount(headerDiscount) ?? 0,
        freight: parseAmount(freight) ?? 0,
        freightAllocated,
        contactId: contactId || null,
        received,
      })
      if (token === previewToken.current) {
        setPreview(result)
        setError('')
      }
    } catch (e) {
      if (token === previewToken.current) {
        setPreview(null)
        setError(errorText(e))
      }
    }
  }, [lines, headerDiscount, freight, freightAllocated, contactId, received])

  useEffect(() => {
    const timer = setTimeout(refreshPreview, 180)
    return () => clearTimeout(timer)
  }, [refreshPreview])

  const productOf = (id: string) => products.find((product) => product.id === id)

  const openNewLine = () => {
    const line = emptyLine()
    setEditing(line)
  }

  const commitLine = (line: Line) => {
    setLines((current) => {
      const exists = current.some((item) => item.key === line.key)
      return exists ? current.map((item) => (item.key === line.key ? line : item)) : [...current, line]
    })
    setEditing(null)
  }

  const removeLine = (key: string) => setLines((current) => current.filter((item) => item.key !== key))

  // میانبرهای صفحه‌کلید مطابق نرم‌افزار فعلی
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (editing) return
      if (event.key === 'Insert') {
        event.preventDefault()
        openNewLine()
      } else if (event.key === 'F7' && selectedKey) {
        event.preventDefault()
        const line = lines.find((item) => item.key === selectedKey)
        if (line) setEditing({...line})
      } else if (event.key === 'Delete' && selectedKey) {
        event.preventDefault()
        removeLine(selectedKey)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [editing, selectedKey, lines])

  const save = async () => {
    if (!preview || lines.length === 0) {
      setError('فاکتور بدون سطر قابل ثبت نیست.')
      return
    }
    setSaving(true)
    setError('')
    setMessage('')
    try {
      // مبالغ تخفیف و مالیات هر سطر از همان محاسبه‌ی نمایش‌داده‌شده می‌آید،
      // تا آنچه ثبت می‌شود دقیقاً همان چیزی باشد که کاربر دیده است.
      const payload = lines.map((line, index) => {
        const computed = preview.lines[index]
        return [
          line.product_id,
          line.quantity,
          line.unit_price,
          computed.total_discount,
          computed.vat + computed.duty,
        ] as [string, number, number, number, number]
      })
      const id = await createSalesInvoice(date, contactId || undefined, warehouseId || undefined, payload)
      await postSalesInvoice(id)
      setMessage(`فاکتور با موفقیت ثبت و سند آن صادر شد. شناسه: ${id}`)
      setLines([])
      setSettlement({cash: '0', check: '0', transfer: '0', card: '0'})
      setInstallments([])
    } catch (e) {
      setError(errorText(e))
    } finally {
      setSaving(false)
    }
  }

  const generateInstallments = async () => {
    if (!preview) return
    try {
      const plan = await buildInstallmentPlan(
        preview.total,
        received,
        Number(installmentCount) || 1,
        date,
      )
      setInstallments(plan)
    } catch (e) {
      setError(errorText(e))
    }
  }

  return (
    <section className="page invoice-page">
      <div className="page-head">
        <div>
          <div className="eyebrow">فروش</div>
          <h1>صدور فاکتور فروش</h1>
          <p>Insert افزودن سطر · F7 ویرایش · Delete حذف — همه‌ی محاسبات از موتور مالی می‌آید.</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="panel">
        <div className="form-row">
          <label>
            <span>تاریخ</span>
            <input value={date} onChange={(event) => setDate(event.target.value)} />
          </label>
          <label className="grow">
            <span>طرف حساب</span>
            <select value={contactId} onChange={(event) => setContactId(event.target.value)}>
              <option value="">انتخاب کنید…</option>
              {contacts.map((contact) => (
                <option key={contact.id} value={contact.id}>
                  {contact.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>انبار</span>
            <select value={warehouseId} onChange={(event) => setWarehouseId(event.target.value)}>
              <option value="">انتخاب کنید…</option>
              {warehouses.map((warehouse) => (
                <option key={warehouse.id} value={warehouse.id}>
                  {warehouse.name}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="toolbar">
          <strong>اقلام فاکتور</strong>
          <button className="table-action" onClick={openNewLine}>
            افزودن کالا (Ins)
          </button>
          <button
            className="table-action"
            disabled={!selectedKey}
            onClick={() => {
              const line = lines.find((item) => item.key === selectedKey)
              if (line) setEditing({...line})
            }}
          >
            ویرایش (F7)
          </button>
          <button
            className="table-action"
            disabled={!selectedKey}
            onClick={() => removeLine(selectedKey)}
          >
            حذف (Delete)
          </button>
          <button className="icon-btn" aria-label="محاسبه مجدد" onClick={refreshPreview} title="محاسبه مجدد">
            <Icon name="refresh" />
          </button>
        </div>

        {lines.length === 0 ? (
          <div className="empty-state">سطری اضافه نشده است. کلید Insert را بزنید.</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>ردیف</th>
                  <th>کالا</th>
                  <th>مقدار</th>
                  <th>فی واحد</th>
                  <th>تخفیف</th>
                  <th>عوارض</th>
                  <th>ارزش افزوده</th>
                  <th>جمع</th>
                </tr>
              </thead>
              <tbody>
                {lines.map((line, index) => {
                  const computed = preview?.lines[index]
                  const product = productOf(line.product_id)
                  return (
                    <tr
                      key={line.key}
                      className={selectedKey === line.key ? 'selected-row' : ''}
                      onClick={() => setSelectedKey(line.key)}
                      onDoubleClick={() => setEditing({...line})}
                    >
                      <td>{index + 1}</td>
                      <td>{product ? `${product.sku} — ${product.name}` : '—'}</td>
                      <td>{formatNumber(line.quantity)}</td>
                      <td>{formatRials(line.unit_price)}</td>
                      <td>{computed ? formatRials(computed.total_discount) : '…'}</td>
                      <td>{computed ? formatRials(computed.duty) : '…'}</td>
                      <td>{computed ? formatRials(computed.vat) : '…'}</td>
                      <td>
                        <b>{computed ? formatRials(computed.total) : '…'}</b>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="invoice-bottom">
        <div className="panel">
          <h3>تخفیف و هزینه</h3>
          <div className="form-row">
            <label>
              <span>تخفیف سرجمع</span>
              <input
                value={headerDiscount}
                onChange={(event) => setHeaderDiscount(event.target.value)}
                inputMode="numeric"
              />
            </label>
            <label>
              <span>کرایه حمل</span>
              <input
                value={freight}
                onChange={(event) => setFreight(event.target.value)}
                inputMode="numeric"
              />
            </label>
          </div>
          <label className="checkbox-row">
            <input
              type="checkbox"
              checked={freightAllocated}
              onChange={(event) => setFreightAllocated(event.target.checked)}
            />
            <span>کرایه روی سطرها سرشکن شود (ورود به بهای تمام‌شده)</span>
          </label>

          <h3>تسویه</h3>
          <div className="form-row">
            {(
              [
                ['cash', 'نقد'],
                ['check', 'چک'],
                ['transfer', 'حواله'],
                ['card', 'کارتخوان'],
              ] as const
            ).map(([field, label]) => (
              <label key={field}>
                <span>{label}</span>
                <input
                  value={settlement[field]}
                  onChange={(event) => setSettlement({...settlement, [field]: event.target.value})}
                  inputMode="numeric"
                />
              </label>
            ))}
          </div>
          <div className="side-summary">جمع دریافتی: {formatRials(received)} ریال</div>
        </div>

        <div className="panel totals-panel">
          <h3>جمع فاکتور</h3>
          {preview ? (
            <table className="totals-table">
              <tbody>
                <tr>
                  <td>جمع</td>
                  <td>{formatRials(preview.subtotal)}</td>
                </tr>
                <tr>
                  <td>تخفیف</td>
                  <td className="red-text">−{formatRials(preview.discount_total)}</td>
                </tr>
                <tr>
                  <td>خالص</td>
                  <td>{formatRials(preview.net_total)}</td>
                </tr>
                <tr>
                  <td>عوارض</td>
                  <td>{formatRials(preview.duty_total)}</td>
                </tr>
                <tr>
                  <td>ارزش افزوده</td>
                  <td>{formatRials(preview.vat_total)}</td>
                </tr>
                <tr>
                  <td>کرایه حمل</td>
                  <td>{formatRials(preview.freight)}</td>
                </tr>
                <tr className="grand">
                  <td>جمع کل فاکتور</td>
                  <td>{formatRials(preview.total)}</td>
                </tr>
                <tr>
                  <td>مانده فاکتور</td>
                  <td>{formatRials(preview.invoice_remainder)}</td>
                </tr>
              </tbody>
            </table>
          ) : (
            <div className="empty-state">در انتظار سطر معتبر…</div>
          )}

          {preview && (
            <div className="balance-bar">
              <span>مانده قبل: {formatRials(preview.balance_before)}</span>
              <span>مانده پس از فاکتور: {formatRials(preview.balance_after)}</span>
            </div>
          )}

          <div className="form-actions">
            <button className="primary" onClick={save} disabled={saving || !preview}>
              {saving ? 'در حال ثبت…' : 'ذخیره و صدور سند'}
            </button>
            <button onClick={() => setShowProfit((value) => !value)} disabled={!preview}>
              محاسبه سود فاکتور
            </button>
          </div>

          {showProfit && preview && (
            <div className="profit-box">
              <div>
                بهای تمام‌شده: <b>{formatRials(preview.cost_total)}</b>
              </div>
              <div>
                پورسانت: <b>{formatRials(preview.commission_total)}</b>
              </div>
              <div>
                سود ناخالص:{' '}
                <b className={preview.profit >= 0 ? 'green-text' : 'red-text'}>
                  {formatRials(preview.profit)}
                </b>
              </div>
              <div>
                حاشیه سود: <b>{(preview.profit_margin_bp / 100).toFixed(2)}٪</b>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="panel">
        <div className="toolbar">
          <strong>اقساط</strong>
          <label className="inline-label">
            <span>تعداد</span>
            <input
              value={installmentCount}
              onChange={(event) => setInstallmentCount(event.target.value)}
              inputMode="numeric"
              style={{width: 70}}
            />
          </label>
          <button className="table-action" onClick={generateInstallments} disabled={!preview}>
            تولید جدول اقساط
          </button>
        </div>
        {installments.length > 0 && (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>قسط</th>
                  <th>سررسید</th>
                  <th>مبلغ</th>
                </tr>
              </thead>
              <tbody>
                {installments.map((item) => (
                  <tr key={item.number}>
                    <td>{item.number}</td>
                    <td className="code">{item.due_date_jalali}</td>
                    <td>{formatRials(item.amount)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {editing && (
        <LineEditor
          line={editing}
          products={products}
          onCancel={() => setEditing(null)}
          onSave={commitLine}
        />
      )}
    </section>
  )
}

/** دیالوگ افزودن/ویرایش سطر — معادل پنجره‌ی «افزودن کالا» نرم‌افزار فعلی. */
function LineEditor({
  line,
  products,
  onCancel,
  onSave,
}: {
  line: Line
  products: Product[]
  onCancel: () => void
  onSave: (line: Line) => void
}) {
  const [draft, setDraft] = useState<Line>(line)
  const product = products.find((item) => item.id === draft.product_id)

  const pick = (productId: string) => {
    const selected = products.find((item) => item.id === productId)
    setDraft({
      ...draft,
      product_id: productId,
      unit_price: selected?.sale_price ?? 0,
      unit_cost: selected?.purchase_price ?? 0,
    })
  }

  const numeric = (field: keyof Line) => (event: React.ChangeEvent<HTMLInputElement>) =>
    setDraft({...draft, [field]: parseAmount(event.target.value) ?? 0})

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal" onClick={(event) => event.stopPropagation()}>
        <h2>افزودن کالا به فاکتور</h2>

        <div className="form-row">
          <label className="grow">
            <span>کالا</span>
            <select value={draft.product_id} onChange={(event) => pick(event.target.value)}>
              <option value="">انتخاب کنید…</option>
              {products.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.sku} — {item.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>مقدار</span>
            <input
              value={String(draft.quantity)}
              onChange={(event) =>
                setDraft({...draft, quantity: Number(parseAmount(event.target.value) ?? 0)})
              }
              inputMode="decimal"
            />
          </label>
          <label>
            <span>فی واحد {product ? `(${product.unit})` : ''}</span>
            <input value={String(draft.unit_price)} onChange={numeric('unit_price')} inputMode="numeric" />
          </label>
        </div>

        <div className="form-row">
          <label>
            <span>تخفیف (مبلغ)</span>
            <input
              value={String(draft.discount_amount)}
              onChange={numeric('discount_amount')}
              inputMode="numeric"
            />
          </label>
          <label>
            <span>تخفیف (درصد×۱۰۰)</span>
            <input value={String(draft.discount_bp)} onChange={numeric('discount_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>ارزش افزوده (درصد×۱۰۰)</span>
            <input value={String(draft.vat_bp)} onChange={numeric('vat_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>عوارض (درصد×۱۰۰)</span>
            <input value={String(draft.duty_bp)} onChange={numeric('duty_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>پورسانت (درصد×۱۰۰)</span>
            <input
              value={String(draft.commission_bp)}
              onChange={numeric('commission_bp')}
              inputMode="numeric"
            />
          </label>
        </div>

        <label className="checkbox-row">
          <input
            type="checkbox"
            checked={draft.serial_tracked}
            onChange={(event) => setDraft({...draft, serial_tracked: event.target.checked})}
          />
          <span>کالای سریال‌دار</span>
        </label>
        {draft.serial_tracked && (
          <label>
            <span>سریال‌ها (با کاما جدا شوند — باید {draft.quantity} مورد باشد)</span>
            <input
              value={draft.serials.join(',')}
              onChange={(event) =>
                setDraft({
                  ...draft,
                  serials: event.target.value
                    .split(',')
                    .map((value) => value.trim())
                    .filter(Boolean),
                })
              }
            />
          </label>
        )}

        <div className="form-actions">
          <button className="primary" onClick={() => onSave(draft)} disabled={!draft.product_id}>
            تأیید
          </button>
          <button onClick={onCancel}>انصراف</button>
        </div>
      </div>
    </div>
  )
}
