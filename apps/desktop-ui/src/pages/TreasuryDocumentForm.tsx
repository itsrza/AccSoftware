import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  createTreasuryDocument,
  getPaymentMethods,
  getParties,
  getTreasuryAccounts,
  getTreasuryDocuments,
  getTreasuryDocument,
  previewTreasuryDocument,
  PaymentMethodInfo,
  TreasuryAccount,
  TreasuryDocumentDetail,
  TreasuryDocumentLineInput,
  TreasuryDocumentPreview,
  TreasuryDocumentRow,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

type Kind = 'receipt' | 'payment'

type EditableLine = TreasuryDocumentLineInput & { key: number }

let nextKey = 1
const blankLine = (): EditableLine => ({ key: nextKey++, method: 'cash', amount: 0 })

/**
 * فرم سند دریافت و پرداخت چندروشی.
 *
 * قاعده‌ای که این صفحه رعایت می‌کند: **هیچ عددی در مرورگر محاسبه نمی‌شود.**
 * جمع‌ها و سند حسابداری از همان موتوری می‌آید که هنگام ثبت اجرا می‌شود، پس
 * پیش‌نمایش دقیقاً همان چیزی است که ثبت خواهد شد.
 */
export function TreasuryDocumentForm() {
  const [kind, setKind] = useState<Kind>('receipt')
  const [documentDate, setDocumentDate] = useState('')
  const [partyId, setPartyId] = useState('')
  const [description, setDescription] = useState('')
  const [lines, setLines] = useState<EditableLine[]>([blankLine()])
  const [methods, setMethods] = useState<PaymentMethodInfo[]>([])
  const [accounts, setAccounts] = useState<TreasuryAccount[]>([])
  const [parties, setParties] = useState<{ id: string; name: string }[]>([])
  const [preview, setPreview] = useState<TreasuryDocumentPreview>()
  const [documents, setDocuments] = useState<TreasuryDocumentRow[]>([])
  const [detail, setDetail] = useState<TreasuryDocumentDetail>()
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const methodOf = useCallback(
    (value: string) => methods.find((m) => m.value === value),
    [methods],
  )

  const loadDocuments = useCallback(async () => {
    try {
      setDocuments(await getTreasuryDocuments(kind))
    } catch (e) {
      setError(errorText(e))
    }
  }, [kind])

  useEffect(() => {
    ;(async () => {
      try {
        setMethods(await getPaymentMethods())
        setAccounts((await getTreasuryAccounts()).filter((a) => a.is_active))
        const list = await getParties()
        setParties(list.rows.map((p) => ({ id: p.id, name: p.display_name })))
      } catch (e) {
        setError(errorText(e))
      }
    })()
  }, [])

  useEffect(() => {
    loadDocuments()
  }, [loadDocuments])

  // پیش‌نمایش زنده: هر تغییر معتبری در سطرها، جمع‌ها و سند را از موتور می‌گیرد.
  useEffect(() => {
    const payload = lines
      .filter((line) => line.amount > 0)
      .map(({ key: _key, ...rest }) => rest)
    if (payload.length === 0) {
      setPreview(undefined)
      return
    }
    let cancelled = false
    const timer = setTimeout(async () => {
      try {
        const result = await previewTreasuryDocument(kind, payload)
        if (!cancelled) {
          setPreview(result)
          setError('')
        }
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
  }, [lines, kind])

  const updateLine = (key: number, patch: Partial<TreasuryDocumentLineInput>) =>
    setLines((current) => current.map((line) => (line.key === key ? { ...line, ...patch } : line)))

  const removeLine = (key: number) =>
    setLines((current) => (current.length === 1 ? current : current.filter((l) => l.key !== key)))

  const submit = async () => {
    setBusy(true)
    setNotice('')
    try {
      const payload = lines
        .filter((line) => line.amount > 0)
        .map(({ key: _key, ...rest }) => rest)
      const id = await createTreasuryDocument(
        kind,
        documentDate,
        partyId,
        description.trim() || undefined,
        payload,
      )
      setNotice(`سند با موفقیت ثبت شد (${id}).`)
      setLines([blankLine()])
      setDescription('')
      setPreview(undefined)
      await loadDocuments()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const canSubmit =
    !busy && documentDate.trim() !== '' && partyId !== '' && !!preview && preview.total > 0

  const { sorted, sortProps } = useSort(documents, 'number')

  const summaryRows = useMemo(() => {
    if (!preview) return []
    return [
      ['نقد', preview.cash],
      ['چک', preview.check],
      ['حواله بانکی', preview.bank_transfer],
      ['کارتخوان', preview.card_terminal],
      ['تخفیف نقدی', preview.discount],
      ['تهاتر', preview.offset],
    ].filter(([, value]) => (value as number) > 0) as [string, number][]
  }, [preview])

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">خزانه‌داری</div>
          <h1>سند دریافت و پرداخت</h1>
          <p>
            یک سند می‌تواند هم‌زمان نقد، چک، حواله، کارتخوان، تخفیف و تهاتر داشته باشد. چک به صندوق
            نمی‌رود؛ تا وصول نشود در «اسناد دریافتنی» می‌ماند.
          </p>
        </div>
        <div className="kind-switch">
          <button className={kind === 'receipt' ? 'active' : ''} onClick={() => setKind('receipt')}>
            سند دریافت
          </button>
          <button className={kind === 'payment' ? 'active' : ''} onClick={() => setKind('payment')}>
            سند پرداخت
          </button>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="panel">
        <div className="filter-grid">
          <label>
            <span>تاریخ سند</span>
            <input
              value={documentDate}
              onChange={(e) => setDocumentDate(e.target.value)}
              placeholder="1405/05/20"
            />
          </label>
          <label className="grow">
            <span>طرف حساب</span>
            <Select value={partyId} onChange={(e) => setPartyId(e.target.value)}>
              <option value="">انتخاب کنید…</option>
              {parties.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </label>
          <label className="grow">
            <span>شرح سند</span>
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="بابت تسویه فاکتور…"
            />
          </label>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>سطرهای تسویه</h3>
            <p>هر سطر یک روش تسویه است. فیلدهای اجباری هر روش را خود موتور تعیین می‌کند.</p>
          </div>
          <button className="ghost" onClick={() => setLines((c) => [...c, blankLine()])}>
            <Icon name="plus" /> افزودن سطر
          </button>
        </div>

        <div className="line-editor">
          {lines.map((line) => {
            const info = methodOf(line.method)
            return (
              <div className="line-row" key={line.key}>
                <label>
                  <span>روش</span>
                  <Select
                    value={line.method}
                    onChange={(e) => updateLine(line.key, { method: e.target.value })}
                  >
                    {methods.map((m) => (
                      <option key={m.value} value={m.value}>
                        {m.label}
                      </option>
                    ))}
                  </Select>
                </label>
                <label>
                  <span>مبلغ (ریال)</span>
                  <input
                    type="number"
                    min={0}
                    value={line.amount || ''}
                    onChange={(e) => updateLine(line.key, { amount: Number(e.target.value) || 0 })}
                  />
                </label>
                {info?.requires_treasury_account && (
                  <label>
                    <span>صندوق / بانک</span>
                    <Select
                      value={line.treasury_account_id ?? ''}
                      onChange={(e) => updateLine(line.key, { treasury_account_id: e.target.value })}
                    >
                      <option value="">انتخاب کنید…</option>
                      {accounts.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name}
                        </option>
                      ))}
                    </Select>
                  </label>
                )}
                {info?.requires_terminal && (
                  <label>
                    <span>شناسه پایانه</span>
                    <input
                      value={line.terminal_id ?? ''}
                      onChange={(e) => updateLine(line.key, { terminal_id: e.target.value })}
                    />
                  </label>
                )}
                {info?.requires_check_details && (
                  <>
                    <label>
                      <span>شماره چک</span>
                      <input
                        value={line.check_serial ?? ''}
                        onChange={(e) => updateLine(line.key, { check_serial: e.target.value })}
                      />
                    </label>
                    <label>
                      <span>سررسید چک</span>
                      <input
                        value={line.check_due_date ?? ''}
                        onChange={(e) => updateLine(line.key, { check_due_date: e.target.value })}
                        placeholder="1405/08/10"
                      />
                    </label>
                    <label>
                      <span>بانک</span>
                      <input
                        value={line.check_bank_name ?? ''}
                        onChange={(e) => updateLine(line.key, { check_bank_name: e.target.value })}
                      />
                    </label>
                    <label>
                      <span>شناسه صیادی</span>
                      <input
                        value={line.sayad_id ?? ''}
                        onChange={(e) => updateLine(line.key, { sayad_id: e.target.value })}
                      />
                    </label>
                  </>
                )}
                <label className="grow">
                  <span>توضیح سطر</span>
                  <input
                    value={line.description ?? ''}
                    onChange={(e) => updateLine(line.key, { description: e.target.value })}
                  />
                </label>
                <button aria-label="حذف سطر"
                  className="icon-btn danger-icon"
                  onClick={() => removeLine(line.key)}
                  disabled={lines.length === 1}
                 
                >
                  <Icon name="trash" />
                </button>
              </div>
            )
          })}
        </div>
      </div>

      {preview && (
        <div className="panel">
          <div className="panel-head">
            <div>
              <h3>پیش‌نمایش سند</h3>
              <p>این دقیقاً همان سندی است که ثبت خواهد شد.</p>
            </div>
          </div>
          <div className="preview-split">
            <div>
              <h4 className="section-title">تفکیک روش‌ها</h4>
              <table className="mini-table">
                <tbody>
                  {summaryRows.map(([label, value]) => (
                    <tr key={label}>
                      <td>{label}</td>
                      <td className="num">{money(value)}</td>
                    </tr>
                  ))}
                  <tr className="total-row">
                    <td>جمع سند</td>
                    <td className="num">{money(preview.total)}</td>
                  </tr>
                  <tr>
                    <td>جابه‌جایی واقعی خزانه</td>
                    <td className="num">{money(preview.treasury_movement)}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div>
              <h4 className="section-title">سند حسابداری</h4>
              <table className="mini-table">
                <thead>
                  <tr>
                    <th>حساب</th>
                    <th>بدهکار</th>
                    <th>بستانکار</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.journal_preview.map((line, index) => (
                    <tr key={`${line.account_id}-${index}`}>
                      <td>{line.account_name}</td>
                      <td className="num">{line.debit ? money(line.debit) : '—'}</td>
                      <td className="num">{line.credit ? money(line.credit) : '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
          <div className="modal-actions">
            <button className="primary" onClick={submit} disabled={!canSubmit}>
              ثبت نهایی سند
            </button>
          </div>
        </div>
      )}

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>اسناد ثبت‌شده</h3>
            <p>{sorted.length} سند — برای دیدن سطرها روی ردیف کلیک کنید.</p>
          </div>
          <button className="icon-btn" onClick={loadDocuments} aria-label="بروزرسانی">
            <Icon name="refresh" />
          </button>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('number')}>شماره</th>
                <th {...sortProps('document_date')}>تاریخ</th>
                <th {...sortProps('party_name')}>طرف حساب</th>
                <th>شرح</th>
                <th {...sortProps('total')}>مبلغ (ریال)</th>
                <th>تعداد سطر</th>
                <th {...sortProps('status')}>وضعیت</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((row) => (
                <tr
                  key={row.id}
                  className="clickable"
                  onClick={async () => {
                    try {
                      setDetail(await getTreasuryDocument(row.id))
                    } catch (e) {
                      setError(errorText(e))
                    }
                  }}
                >
                  <td className="code">{row.number}</td>
                  <td>{row.document_date}</td>
                  <td>{row.party_name ?? '—'}</td>
                  <td>{row.description ?? '—'}</td>
                  <td className="num">{money(row.total)}</td>
                  <td>{row.line_count}</td>
                  <td>
                    <span className={`status ${row.status === 'posted' ? 'done' : 'pending'}`}>
                      {row.status_label}
                    </span>
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={7} className="empty-row">
                    هنوز سندی از این نوع ثبت نشده است.
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
                <h2>
                  {detail.header.kind_label} شماره {detail.header.number}
                </h2>
                <p>
                  {detail.header.document_date} — {detail.header.party_name ?? 'بدون طرف حساب'}
                </p>
              </div>
              <button aria-label="بستن" className="icon-btn" onClick={() => setDetail(undefined)}>
                <Icon name="close" />
              </button>
            </div>
            <table className="mini-table">
              <thead>
                <tr>
                  <th>روش</th>
                  <th>مبلغ</th>
                  <th>صندوق / بانک</th>
                  <th>جزئیات</th>
                </tr>
              </thead>
              <tbody>
                {detail.lines.map((line) => (
                  <tr key={line.id}>
                    <td>{line.method_label}</td>
                    <td className="num">{money(line.amount)}</td>
                    <td>{line.treasury_account_name ?? '—'}</td>
                    <td>
                      {line.check_serial
                        ? `چک ${line.check_serial} — سررسید ${line.check_due_date ?? '—'}`
                        : line.terminal_id
                          ? `پایانه ${line.terminal_id}`
                          : (line.description ?? '—')}
                    </td>
                  </tr>
                ))}
                <tr className="total-row">
                  <td>جمع</td>
                  <td className="num">{money(detail.header.total)}</td>
                  <td colSpan={2} />
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      )}
    </section>
  )
}
