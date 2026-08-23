import {useEffect, useState} from 'react'
import {
  approveAllVariances,
  createStocktake,
  getStocktake,
  getStocktakes,
  getWarehouses,
  postStocktake,
  setStocktakeCount,
  type StocktakeDetail,
  type StocktakeSessionRow,
  type Warehouse,
} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {formatNumber, formatRials, todayJalali} from '../lib/format'
import {Select} from '../components/Select'

/**
 * انبارگردانی — بازنویسی کامل بر پایه‌ی منطق حسابداری انبار.
 *
 * چرخه‌ی اجباری: ایجاد دوره (فریز موجودی) ← شمارش ← شمارش مجدد اقلام پرریسک
 * ← تأیید اختلاف ← ثبت نهایی و صدور سند تعدیل.
 *
 * تمام محاسبات (اختلاف، ارزش‌گذاری، شرایط ثبت) از موتور مالی می‌آید.
 */
export function Stocktaking() {
  const [sessions, setSessions] = useState<StocktakeSessionRow[]>([])
  const [warehouses, setWarehouses] = useState<Warehouse[]>([])
  const [detail, setDetail] = useState<StocktakeDetail | null>(null)
  const [error, setError] = useState('')
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [creating, setCreating] = useState(false)
  const [form, setForm] = useState({warehouse_id: '', title: '', count_date: todayJalali()})

  const loadSessions = async () => {
    setError('')
    try {
      const [list, warehouseList] = await Promise.all([getStocktakes(), getWarehouses()])
      setSessions(list)
      setWarehouses(warehouseList)
      if (warehouseList.length > 0 && !form.warehouse_id) {
        setForm((current) => ({...current, warehouse_id: warehouseList[0].id}))
      }
    } catch (e) {
      setError(errorText(e))
    }
  }
  useEffect(() => {
    loadSessions()
  }, [])

  const openSession = async (id: string) => {
    setError('')
    try {
      setDetail(await getStocktake(id))
    } catch (e) {
      setError(errorText(e))
    }
  }

  const refreshDetail = async () => {
    if (detail) await openSession(detail.id)
  }

  const create = async () => {
    if (!form.warehouse_id || !form.title.trim()) {
      setError('انبار و عنوان دوره الزامی است.')
      return
    }
    setBusy(true)
    setError('')
    try {
      const id = await createStocktake(form.warehouse_id, form.title, form.count_date)
      setCreating(false)
      setForm({...form, title: ''})
      await loadSessions()
      await openSession(id)
      setMessage('دوره ایجاد و موجودی سیستمی فریز شد. شمارش را آغاز کنید.')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const saveCount = async (lineId: string, raw: string, isRecount: boolean) => {
    const trimmed = raw.trim()
    const quantity = trimmed === '' ? null : Number(trimmed.replace(/[^\d.-]/g, ''))
    if (quantity !== null && (!Number.isFinite(quantity) || quantity < 0)) {
      setError('مقدار شمارش نمی‌تواند منفی باشد.')
      return
    }
    try {
      await setStocktakeCount(lineId, quantity, isRecount, null)
      await refreshDetail()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const approve = async (lineId: string, value: boolean) => {
    try {
      await setStocktakeCount(lineId, null, false, value)
      await refreshDetail()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const approveAll = async () => {
    if (!detail) return
    setBusy(true)
    try {
      const count = await approveAllVariances(detail.id)
      setMessage(`${count} قلم تأیید شد.`)
      await refreshDetail()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const post = async () => {
    if (!detail) return
    if (
      !confirm(
        'با ثبت نهایی، موجودی انبار اصلاح و سند تعدیل صادر می‌شود. این عملیات برگشت‌ناپذیر است. ادامه می‌دهید؟',
      )
    )
      return
    setBusy(true)
    setError('')
    try {
      const journalId = await postStocktake(detail.id)
      setMessage(
        journalId
          ? `انبارگردانی ثبت شد و سند تعدیل صادر گردید. شناسه سند: ${journalId}`
          : 'انبارگردانی ثبت شد. اختلافی وجود نداشت، پس سندی صادر نشد.',
      )
      await loadSessions()
      await refreshDetail()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">انبار</div>
          <h1>انبارگردانی</h1>
          <p>
            فریز موجودی ← شمارش ← شمارش مجدد ← تأیید اختلاف ← ثبت و صدور سند تعدیل
          </p>
        </div>
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          <Icon name="plus" size={15} /> دوره‌ی جدید
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="panel list-panel">
        <div className="toolbar">
          <strong>دوره‌های انبارگردانی</strong>
          <span className="spacer" />
          <button className="icon-btn" aria-label="بارگذاری مجدد" onClick={loadSessions} title="بارگذاری مجدد">
            <Icon name="refresh" />
          </button>
        </div>
        {sessions.length === 0 ? (
          <div className="empty-state">هنوز دوره‌ای ایجاد نشده است.</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>عنوان</th>
                  <th>انبار</th>
                  <th>تاریخ</th>
                  <th>اقلام</th>
                  <th>شمارش‌شده</th>
                  <th>دارای اختلاف</th>
                  <th>وضعیت</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {sessions.map((session) => (
                  <tr
                    key={session.id}
                    className="clickable"
                    onClick={() => openSession(session.id)}
                  >
                    <td>{session.title}</td>
                    <td>{session.warehouse_name}</td>
                    <td className="code">{session.count_date}</td>
                    <td>{formatNumber(session.total_lines)}</td>
                    <td>{formatNumber(session.counted_lines)}</td>
                    <td>
                      {session.variance_lines > 0 ? (
                        <span className="status danger">{formatNumber(session.variance_lines)}</span>
                      ) : (
                        <span className="status done">۰</span>
                      )}
                    </td>
                    <td>
                      <span
                        className={
                          session.status === 'posted'
                            ? 'status done'
                            : session.status === 'cancelled'
                              ? 'status danger'
                              : 'status pending'
                        }
                      >
                        {session.status_label}
                      </span>
                    </td>
                    <td>
                      <button className="btn btn-sm">مشاهده</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {detail && (
        <>
          <div className="metric-strip">
            <div>
              <span>شمارش‌شده</span>
              <b>
                {formatNumber(detail.counted_lines)} / {formatNumber(detail.total_lines)}
              </b>
              <small>{formatNumber(detail.uncounted_lines)} قلم باقی‌مانده</small>
            </div>
            <div>
              <span>اضافی</span>
              <b className="green-text">{formatRials(detail.surplus_value)}</b>
              <small>{formatNumber(detail.surplus_lines)} قلم</small>
            </div>
            <div>
              <span>کسری</span>
              <b className="red-text">{formatRials(detail.shortage_value)}</b>
              <small>{formatNumber(detail.shortage_lines)} قلم</small>
            </div>
            <div>
              <span>اثر خالص بر موجودی</span>
              <b className={detail.net_value >= 0 ? 'green-text' : 'red-text'}>
                {formatRials(Math.abs(detail.net_value))}
              </b>
              <small>{detail.net_value >= 0 ? 'افزایش' : 'کاهش'} ارزش انبار</small>
            </div>
          </div>

          <div className="panel">
            <div className="toolbar">
              <strong>
                {detail.title} — {detail.warehouse_name}
              </strong>
              <span className={`status ${detail.status === 'posted' ? 'done' : 'pending'}`}>
                {detail.status_label}
              </span>
              <span className="spacer" />
              {detail.unapproved_variances > 0 && (
                <button className="btn btn-sm" onClick={approveAll} disabled={busy}>
                  تأیید همه‌ی اختلاف‌ها ({formatNumber(detail.unapproved_variances)})
                </button>
              )}
              <button
                className="btn btn-primary btn-sm"
                onClick={post}
                disabled={busy || !detail.can_post || detail.status === 'posted'}
                title={detail.blocking_reason ?? ''}
              >
                ثبت نهایی و صدور سند تعدیل
              </button>
            </div>

            {detail.blocking_reason && detail.status !== 'posted' && (
              <div className="error-box">{detail.blocking_reason}</div>
            )}

            <div className="table-wrap">
              <table className="large-table">
                <thead>
                  <tr>
                    <th>کد</th>
                    <th>کالا</th>
                    <th>موجودی سیستم (فریزشده)</th>
                    <th>شمارش اول</th>
                    <th>شمارش مجدد</th>
                    <th>اختلاف</th>
                    <th>ارزش اختلاف</th>
                    <th>تأیید</th>
                  </tr>
                </thead>
                <tbody>
                  {detail.lines.map((line) => {
                    const locked = detail.status === 'posted' || detail.status === 'cancelled'
                    return (
                      <tr key={line.id} className={line.needs_recount ? 'needs-recount' : ''}>
                        <td className="code">{line.sku}</td>
                        <td>{line.product_name}</td>
                        <td className="code">{formatNumber(line.frozen_quantity)}</td>
                        <td>
                          <input
                            className="count-input"
                            defaultValue={line.counted_quantity ?? ''}
                            disabled={locked}
                            inputMode="decimal"
                            onBlur={(event) => {
                              const next = event.target.value.trim()
                              const previous =
                                line.counted_quantity === null ? '' : String(line.counted_quantity)
                              if (next !== previous) saveCount(line.id, next, false)
                            }}
                          />
                        </td>
                        <td>
                          <input
                            className="count-input"
                            defaultValue={line.recount_quantity ?? ''}
                            disabled={locked}
                            inputMode="decimal"
                            placeholder={line.needs_recount ? 'الزامی' : '—'}
                            onBlur={(event) => {
                              const next = event.target.value.trim()
                              const previous =
                                line.recount_quantity === null ? '' : String(line.recount_quantity)
                              if (next !== previous) saveCount(line.id, next, true)
                            }}
                          />
                        </td>
                        <td>
                          {line.variance === null ? (
                            <span className="amount-zero">—</span>
                          ) : line.variance === 0 ? (
                            <span className="amount-zero">۰</span>
                          ) : (
                            <span className={line.variance > 0 ? 'amount-credit' : 'amount-debit'}>
                              {line.variance > 0 ? '+' : '−'}
                              {formatNumber(Math.abs(line.variance))}
                            </span>
                          )}
                        </td>
                        <td>
                          {line.variance_value === 0 ? (
                            <span className="amount-zero">—</span>
                          ) : (
                            <span
                              className={
                                line.variance_value > 0 ? 'amount-credit' : 'amount-debit'
                              }
                            >
                              {formatRials(Math.abs(line.variance_value))}
                            </span>
                          )}
                        </td>
                        <td>
                          {line.variance !== null && line.variance !== 0 ? (
                            <input
                              type="checkbox"
                              checked={line.variance_approved}
                              disabled={locked}
                              onChange={(event) => approve(line.id, event.target.checked)}
                            />
                          ) : (
                            <span className="amount-zero">—</span>
                          )}
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
            <div className="side-summary">
              ردیف‌های نارنجی اختلافشان از {formatNumber(detail.recount_threshold_percent)}٪ بیشتر
              است و طبق کنترل داخلی باید دوباره شمرده شوند.
            </div>
          </div>
        </>
      )}

      {creating && (
        <div className="modal-backdrop" onClick={() => setCreating(false)}>
          <div className="modal" onClick={(event) => event.stopPropagation()}>
            <h2>دوره‌ی جدید انبارگردانی</h2>
            <p style={{fontSize: 12.5, color: 'var(--text-2)', marginTop: 0}}>
              با ایجاد دوره، موجودی سیستمی همه‌ی کالاهای این انبار در همین لحظه فریز می‌شود تا
              خرید و فروش حین شمارش، مبنای مقایسه را تغییر ندهد.
            </p>
            <div className="form-row">
              <label>
                <span>انبار</span>
                <Select
                  value={form.warehouse_id}
                  onChange={(event) => setForm({...form, warehouse_id: event.target.value})}
                >
                  <option value="">انتخاب کنید…</option>
                  {warehouses.map((warehouse) => (
                    <option key={warehouse.id} value={warehouse.id}>
                      {warehouse.name}
                    </option>
                  ))}
                </Select>
              </label>
              <label className="grow">
                <span>عنوان دوره</span>
                <input
                  value={form.title}
                  onChange={(event) => setForm({...form, title: event.target.value})}
                  placeholder="مثلاً انبارگردانی پایان سال ۱۴۰۵"
                />
              </label>
              <label>
                <span>تاریخ</span>
                <input
                  value={form.count_date}
                  onChange={(event) => setForm({...form, count_date: event.target.value})}
                />
              </label>
            </div>
            <div className="form-actions">
              <button className="btn btn-primary" onClick={create} disabled={busy}>
                {busy ? 'در حال ایجاد…' : 'ایجاد و فریز موجودی'}
              </button>
              <button className="btn" onClick={() => setCreating(false)}>
                انصراف
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
