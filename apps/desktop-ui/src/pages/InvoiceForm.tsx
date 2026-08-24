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
import {getSettings, getPrintTemplates, type PrintTemplate, type SettingWithValue} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {
  formatCount,
  formatNumber,
  formatRials,
  parseAmount,
  percentText,
  rialUnit,
  todayJalali,
} from '../lib/format'
import {useI18n} from '../lib/i18n'
import {Select} from '../components/Select'
import {DEFAULT_SCANNER, scannerOptionsFrom, useBarcodeScanner} from '../lib/barcode'
import {companyFrom, printWithTemplate} from '../lib/printing'
import {defaultDesign, type PrintDocument, type TemplateKind} from '../lib/printTemplate'
import {ScanIndicator} from '../components/ScanIndicator'

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
  const {t} = useI18n()
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

  const [settings, setSettings] = useState<SettingWithValue[]>([])
  const [templates, setTemplates] = useState<PrintTemplate[]>([])
  const [lastScan, setLastScan] = useState<{code: string; name: string; ok: boolean} | null>(null)

  const previewToken = useRef(0)

  useEffect(() => {
    const load = async () => {
      try {
        const [p, c, w] = await Promise.all([getProducts(), getContacts(), getWarehouses()])
        setProducts(p)
        setContacts(c)
        setWarehouses(w)
        if (w.length > 0) setWarehouseId(w[0].id)
        // تنظیمات بارکدخوان و هویت مجموعه برای اسکن و چاپ.
        setSettings(await getSettings().catch(() => []))
        setTemplates(await getPrintTemplates().catch(() => []))
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

  // ---------------------------------------------------------------- بارکدخوان
  const scanner = useMemo(
    () => (settings.length ? scannerOptionsFrom(settings) : {...DEFAULT_SCANNER, enabled: false}),
    [settings],
  )

  /**
   * افزودن کالا با اسکن بارکد.
   *
   * اگر کالا از قبل در فاکتور باشد، به‌جای سطر تکراری، مقدارش یکی زیاد
   * می‌شود — رفتاری که صندوق‌دار انتظار دارد وقتی دو عدد از یک کالا را
   * پشت سر هم اسکن می‌کند.
   */
  const addByBarcode = useCallback(
    (code: string) => {
      const normalized = code.trim()
      const product = products.find(
        (item) => item.barcode === normalized || item.sku === normalized,
      )
      if (!product) {
        setLastScan({code: normalized, name: '', ok: false})
        return
      }
      setLines((current) => {
        const existing = current.find((line) => line.product_id === product.id)
        if (existing) {
          return current.map((line) =>
            line.key === existing.key ? {...line, quantity: line.quantity + 1} : line,
          )
        }
        return [
          ...current,
          {
            ...emptyLine(),
            product_id: product.id,
            unit_price: product.sale_price,
            unit_cost: product.purchase_price,
          },
        ]
      })
      setLastScan({code: normalized, name: product.name, ok: true})
    },
    [products],
  )

  useBarcodeScanner(scanner, addByBarcode)

  // ------------------------------------------------------------------- چاپ
  const printInvoice = async (kind: TemplateKind) => {
    if (!preview) return
    const company = companyFrom(settings, t('app.company'))
    const template =
      templates.find((item) => item.template_type === kind && item.is_default) ??
      templates.find((item) => item.template_type === kind)
    const copies = Number(settings.find((item) => item.key === 'printing.copies')?.value ?? '1') || 1

    const document_: PrintDocument = {
      title: kind === 'receipt' ? t('invoiceForm.salesReceipt') : t('invoiceForm.salesInvoice'),
      number: String(preview.lines.length ? Date.now() % 100000 : 0),
      date,
      partyName: contacts.find((item) => item.id === contactId)?.name ?? t('invoiceForm.walkInCustomer'),
      partyPhone: contacts.find((item) => item.id === contactId)?.mobile,
      lines: lines.map((line, index) => {
        const computed = preview.lines[index]
        const product = productOf(line.product_id)
        return {
          code: product?.sku ?? '',
          name: product?.name ?? '',
          quantity: line.quantity,
          unit: product?.unit ?? '',
          unit_price: line.unit_price,
          discount: computed?.total_discount ?? 0,
          vat: computed?.vat ?? 0,
          line_total: computed?.total ?? 0,
        }
      }),
      subtotal: preview.subtotal,
      discount: preview.discount_total,
      vat: preview.vat_total + preview.duty_total,
      total: preview.total,
    }

    await printWithTemplate(
      template?.content_html ?? '',
      kind,
      defaultDesign(kind),
      company,
      document_,
      copies,
    )
  }

  const save = async () => {
    if (!preview || lines.length === 0) {
      setError(t('invoiceForm.noLineError'))
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
      setMessage(t('invoiceForm.savedMessage', {id}))
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
          <div className="eyebrow">{t('invoiceForm.eyebrow')}</div>
          <h1>{t('page.invoice-form')}</h1>
          <p>{t('invoiceForm.hint')}</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="panel">
        <div className="form-row">
          <label>
            <span>{t('common.date')}</span>
            <input value={date} onChange={(event) => setDate(event.target.value)} />
          </label>
          <label className="grow">
            <span>{t('common.party')}</span>
            <Select value={contactId} onChange={(event) => setContactId(event.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {contacts.map((contact) => (
                <option key={contact.id} value={contact.id}>
                  {contact.name}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('common.warehouse')}</span>
            <Select value={warehouseId} onChange={(event) => setWarehouseId(event.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {warehouses.map((warehouse) => (
                <option key={warehouse.id} value={warehouse.id}>
                  {warehouse.name}
                </option>
              ))}
            </Select>
          </label>
        </div>
      </div>

      <div className="mb-3">
        <ScanIndicator enabled={scanner.enabled} last={lastScan} />
      </div>

      <div className="panel list-panel">
        <div className="toolbar">
          <strong>{t('invoiceForm.lines')}</strong>
          <button className="table-action" onClick={openNewLine}>
            {t('invoiceForm.addLine')}
          </button>
          <button
            className="table-action"
            disabled={!selectedKey}
            onClick={() => {
              const line = lines.find((item) => item.key === selectedKey)
              if (line) setEditing({...line})
            }}
          >
            {t('invoiceForm.editLine')}
          </button>
          <button
            className="table-action"
            disabled={!selectedKey}
            onClick={() => removeLine(selectedKey)}
          >
            {t('invoiceForm.deleteLine')}
          </button>
          <button
            className="icon-btn"
            aria-label={t('invoiceForm.recalculate')}
            onClick={refreshPreview}
            title={t('invoiceForm.recalculate')}
          >
            <Icon name="refresh" />
          </button>
        </div>

        {lines.length === 0 ? (
          <div className="empty-state">{t('invoiceForm.noLines')}</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>{t('invoiceForm.rowNo')}</th>
                  <th>{t('invoiceForm.product')}</th>
                  <th>{t('common.quantity')}</th>
                  <th>{t('invoiceForm.unitPrice')}</th>
                  <th>{t('invoiceForm.discount')}</th>
                  <th>{t('invoiceForm.duty')}</th>
                  <th>{t('common.vat')}</th>
                  <th>{t('invoiceForm.lineTotal')}</th>
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
                      <td>{formatCount(index + 1)}</td>
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
          <h3>{t('invoiceForm.discountAndCost')}</h3>
          <div className="form-row">
            <label>
              <span>{t('invoiceForm.headerDiscount')}</span>
              <input
                value={headerDiscount}
                onChange={(event) => setHeaderDiscount(event.target.value)}
                inputMode="numeric"
              />
            </label>
            <label>
              <span>{t('invoiceForm.freight')}</span>
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
            <span>{t('invoiceForm.allocateFreight')}</span>
          </label>

          <h3>{t('invoiceForm.settlement')}</h3>
          <div className="form-row">
            {(
              [
                ['cash', 'invoiceForm.cash'],
                ['check', 'invoiceForm.cheque'],
                ['transfer', 'invoiceForm.transfer'],
                ['card', 'invoiceForm.card'],
              ] as const
            ).map(([field, labelKey]) => (
              <label key={field}>
                <span>{t(labelKey)}</span>
                <input
                  value={settlement[field]}
                  onChange={(event) => setSettlement({...settlement, [field]: event.target.value})}
                  inputMode="numeric"
                />
              </label>
            ))}
          </div>
          <div className="side-summary">
            {t('invoiceForm.receivedTotal', {amount: formatRials(received), unit: rialUnit()})}
          </div>
        </div>

        <div className="panel totals-panel">
          <h3>{t('invoiceForm.totals')}</h3>
          {preview ? (
            <table className="totals-table">
              <tbody>
                <tr>
                  <td>{t('invoiceForm.subtotal')}</td>
                  <td>{formatRials(preview.subtotal)}</td>
                </tr>
                <tr>
                  <td>{t('invoiceForm.discount')}</td>
                  <td className="red-text">−{formatRials(preview.discount_total)}</td>
                </tr>
                <tr>
                  <td>{t('invoiceForm.netTotal')}</td>
                  <td>{formatRials(preview.net_total)}</td>
                </tr>
                <tr>
                  <td>{t('invoiceForm.duty')}</td>
                  <td>{formatRials(preview.duty_total)}</td>
                </tr>
                <tr>
                  <td>{t('common.vat')}</td>
                  <td>{formatRials(preview.vat_total)}</td>
                </tr>
                <tr>
                  <td>{t('invoiceForm.freight')}</td>
                  <td>{formatRials(preview.freight)}</td>
                </tr>
                <tr className="grand">
                  <td>{t('invoiceForm.grandTotal')}</td>
                  <td>{formatRials(preview.total)}</td>
                </tr>
                <tr>
                  <td>{t('invoiceForm.remainder')}</td>
                  <td>{formatRials(preview.invoice_remainder)}</td>
                </tr>
              </tbody>
            </table>
          ) : (
            <div className="empty-state">{t('invoiceForm.waiting')}</div>
          )}

          {preview && (
            <div className="balance-bar">
              <span>{t('invoiceForm.balanceBefore', {amount: formatRials(preview.balance_before)})}</span>
              <span>{t('invoiceForm.balanceAfter', {amount: formatRials(preview.balance_after)})}</span>
            </div>
          )}

          {/* دکمه‌های پانل جمع فاکتور در ستونی ۳۸۰ پیکسلی می‌نشینند؛ با
            * چیدمان عمومی `form-actions` (که برای فرم تمام‌عرض ساخته شده)
            * هر دکمه یک سطر می‌گرفت و بیش از اندازه بزرگ دیده می‌شد. اینجا
            * شبکه‌ی دوستونیِ فشرده است: دو چاپ کنار هم، ثبت تمام‌عرض. */}
          <div className="invoice-actions">
            <button
              className="ghost"
              onClick={() => void printInvoice('receipt')}
              disabled={!preview || lines.length === 0}
            >
              <Icon name="print" size={14} /> {t('invoiceForm.printReceipt')}
            </button>
            <button
              className="ghost"
              onClick={() => void printInvoice('invoice')}
              disabled={!preview || lines.length === 0}
            >
              <Icon name="print" size={14} /> {t('invoiceForm.printInvoice')}
            </button>
            <button className="primary wide" onClick={save} disabled={saving || !preview}>
              {saving ? t('invoiceForm.saving') : t('invoiceForm.save')}
            </button>
            <button
              className="wide subtle"
              onClick={() => setShowProfit((value) => !value)}
              disabled={!preview}
            >
              {t('invoiceForm.profitToggle')}
            </button>
          </div>

          {showProfit && preview && (
            <div className="profit-box">
              <div>
                {t('invoiceForm.cost')}: <b>{formatRials(preview.cost_total)}</b>
              </div>
              <div>
                {t('invoiceForm.commission')}: <b>{formatRials(preview.commission_total)}</b>
              </div>
              <div>
                {t('invoiceForm.grossProfit')}:{' '}
                <b className={preview.profit >= 0 ? 'green-text' : 'red-text'}>
                  {formatRials(preview.profit)}
                </b>
              </div>
              <div>
                {t('invoiceForm.margin')}: <b>{percentText(preview.profit_margin_bp / 100, 2)}</b>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="panel">
        <div className="toolbar">
          <strong>{t('invoiceForm.installments')}</strong>
          <label className="inline-label">
            <span>{t('invoiceForm.installmentCount')}</span>
            <input
              value={installmentCount}
              onChange={(event) => setInstallmentCount(event.target.value)}
              inputMode="numeric"
              style={{width: 70}}
            />
          </label>
          <button className="table-action" onClick={generateInstallments} disabled={!preview}>
            {t('invoiceForm.generateInstallments')}
          </button>
        </div>
        {installments.length > 0 && (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>{t('invoiceForm.installmentNo')}</th>
                  <th>{t('checks.dueDate')}</th>
                  <th>{t('common.amount')}</th>
                </tr>
              </thead>
              <tbody>
                {installments.map((item) => (
                  <tr key={item.number}>
                    <td>{formatCount(item.number)}</td>
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
  const {t} = useI18n()
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
    <div className="modal-backdrop" role="presentation">
      <div className="modal form-modal">
        <div className="modal-head">
          <div>
            <div className="eyebrow">{t('invoiceForm.lineEditorEyebrow')}</div>
            <h2>{t('invoiceForm.lineEditorTitle')}</h2>
          </div>
          <button aria-label={t('common.close')} type="button" className="icon-btn" onClick={onCancel}>
            <Icon name="close" />
          </button>
        </div>

        <div className="form-row">
          <label className="grow">
            <span>{t('invoiceForm.product')}</span>
            <Select value={draft.product_id} onChange={(event) => pick(event.target.value)}>
              <option value="">{t('invoiceForm.selectPlaceholder')}</option>
              {products.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.sku} — {item.name}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('common.quantity')}</span>
            <input
              value={String(draft.quantity)}
              onChange={(event) =>
                setDraft({...draft, quantity: Number(parseAmount(event.target.value) ?? 0)})
              }
              inputMode="decimal"
            />
          </label>
          <label>
            <span>{t('invoiceForm.unitPriceWith', {unit: product ? `(${product.unit})` : ''})}</span>
            <input value={String(draft.unit_price)} onChange={numeric('unit_price')} inputMode="numeric" />
          </label>
        </div>

        <div className="form-row">
          <label>
            <span>{t('invoiceForm.discountAmount')}</span>
            <input
              value={String(draft.discount_amount)}
              onChange={numeric('discount_amount')}
              inputMode="numeric"
            />
          </label>
          <label>
            <span>{t('invoiceForm.discountBp')}</span>
            <input value={String(draft.discount_bp)} onChange={numeric('discount_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>{t('invoiceForm.vatBp')}</span>
            <input value={String(draft.vat_bp)} onChange={numeric('vat_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>{t('invoiceForm.dutyBp')}</span>
            <input value={String(draft.duty_bp)} onChange={numeric('duty_bp')} inputMode="numeric" />
          </label>
          <label>
            <span>{t('invoiceForm.commissionBp')}</span>
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
          <span>{t('invoiceForm.serialTracked')}</span>
        </label>
        {draft.serial_tracked && (
          <label>
            <span>{t('invoiceForm.serials', {count: formatCount(draft.quantity)})}</span>
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

        <div className="modal-actions">
          <button type="button" className="secondary" onClick={onCancel}>
            {t('common.cancel')}
          </button>
          <button className="primary" onClick={() => onSave(draft)} disabled={!draft.product_id}>
            {t('invoiceForm.confirmAdd')}
          </button>
        </div>
      </div>
    </div>
  )
}
