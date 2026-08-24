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
import {formatNumber, formatRials, todayJalali, formatCount} from '../lib/format'
import {useI18n} from '../lib/i18n'
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
  const {t} = useI18n()
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
      setError(t('stock.errRequired'))
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
      setMessage(t('stock.frozen'))
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
      setError(t('stock.errNegative'))
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
        t('stock.confirmPost'),
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
          : t('stock.postedNoDiff'),
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
          <div className="eyebrow">{t('common.warehouse')}</div>
          <h1>{t('inv.tab.counts')}</h1>
          <p>
            {t('stock.flow')}
          </p>
        </div>
        <button className="btn btn-primary" onClick={() => setCreating(true)}>
          <Icon name="plus" size={15} /> {t('stock.newPeriod')}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="panel list-panel">
        <div className="toolbar">
          <strong>{t('stock.periods')}</strong>
          <span className="spacer" />
          <button className="icon-btn" aria-label={t('common.reload')} onClick={loadSessions} title={t('common.reload')}>
            <Icon name="refresh" />
          </button>
        </div>
        {sessions.length === 0 ? (
          <div className="empty-state">{t('stock.noPeriod')}</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>{t('partyForm.titlePrefix')}</th>
                  <th>{t('common.warehouse')}</th>
                  <th>{t('common.date')}</th>
                  <th>{t('inv.items')}</th>
                  <th>{t('stock.counted')}</th>
                  <th>{t('stock.withDifference')}</th>
                  <th>{t('common.status')}</th>
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
                        <span className="status done">{formatCount(0)}</span>
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
                      <button className="btn btn-sm">{t('stock.view')}</button>
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
              <span>{t('stock.counted')}</span>
              <b>
                {formatNumber(detail.counted_lines)} / {formatNumber(detail.total_lines)}
              </b>
              <small>{formatNumber(detail.uncounted_lines)} قلم باقی‌مانده</small>
            </div>
            <div>
              <span>{t('stock.surplus')}</span>
              <b className="green-text">{formatRials(detail.surplus_value)}</b>
              <small>{formatNumber(detail.surplus_lines)} قلم</small>
            </div>
            <div>
              <span>{t('stock.shortage')}</span>
              <b className="red-text">{formatRials(detail.shortage_value)}</b>
              <small>{formatNumber(detail.shortage_lines)} قلم</small>
            </div>
            <div>
              <span>{t('stock.netEffect')}</span>
              <b className={detail.net_value >= 0 ? 'green-text' : 'red-text'}>
                {formatRials(Math.abs(detail.net_value))}
              </b>
              <small>{detail.net_value >= 0 ? t('stock.increase') : t('stock.decrease')} ارزش انبار</small>
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
                {t('stock.postAndIssue')}
              </button>
            </div>

            {detail.blocking_reason && detail.status !== 'posted' && (
              <div className="error-box">{detail.blocking_reason}</div>
            )}

            <div className="table-wrap">
              <table className="large-table">
                <thead>
                  <tr>
                    <th>{t('common.code')}</th>
                    <th>{t('invoiceForm.product')}</th>
                    <th>{t('stock.systemQty')}</th>
                    <th>{t('stock.firstCount')}</th>
                    <th>{t('stock.recount')}</th>
                    <th>{t('inv.difference')}</th>
                    <th>{t('stock.diffValue')}</th>
                    <th>{t('stock.approve')}</th>
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
                            placeholder={line.needs_recount ? t('stock.required') : '—'}
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
                            <span className="amount-zero">{formatCount(0)}</span>
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
        <div className="modal-backdrop" role="presentation">
          <div className="modal">
            <h2>{t('stock.newPeriodTitle')}</h2>
            <p style={{fontSize: 12.5, color: 'var(--text-2)', marginTop: 0}}>
              {t('stock.freezeNote')}
            </p>
            <div className="form-row">
              <label>
                <span>{t('common.warehouse')}</span>
                <Select
                  value={form.warehouse_id}
                  onChange={(event) => setForm({...form, warehouse_id: event.target.value})}
                >
                  <option value="">{t('invoiceForm.selectPlaceholder')}</option>
                  {warehouses.map((warehouse) => (
                    <option key={warehouse.id} value={warehouse.id}>
                      {warehouse.name}
                    </option>
                  ))}
                </Select>
              </label>
              <label className="grow">
                <span>{t('stock.periodTitle')}</span>
                <input
                  value={form.title}
                  onChange={(event) => setForm({...form, title: event.target.value})}
                  placeholder={t('stock.titleSample')}
                />
              </label>
              <label>
                <span>{t('common.date')}</span>
                <input
                  value={form.count_date}
                  onChange={(event) => setForm({...form, count_date: event.target.value})}
                />
              </label>
            </div>
            <div className="form-actions">
              <button className="btn btn-primary" onClick={create} disabled={busy}>
                {busy ? t('stock.creating') : t('stock.createAndFreeze')}
              </button>
              <button className="btn" onClick={() => setCreating(false)}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
