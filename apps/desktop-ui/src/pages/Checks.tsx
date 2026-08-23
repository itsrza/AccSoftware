import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  getCheckDashboard,
  getChecksFiltered,
  getCheckTransitionOptions,
  updateCheckStatus,
  getParties,
  getTreasuryAccounts,
  CheckDashboard,
  CheckSummary,
  CheckTransitionOption,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { checkStatusLabel, checkStatusTone, CHECK_STATUS_LABELS } from '../lib/checkStatus'
import { useSort } from '../lib/useSort'

type Kind = '' | 'received' | 'issued'

/** برچسب فارسی اثر خزانه‌ای یک گذار، تا کاربر بداند چه سندی صادر می‌شود. */
const EFFECT_NOTE: Record<CheckTransitionOption['treasury_effect'], string> = {
  increase: 'سند دریافت صادر می‌شود',
  decrease: 'سند پرداخت صادر می‌شود',
  none: 'بدون اثر مالی',
}

export function Checks() {
  const [dash, setDash] = useState<CheckDashboard>()
  const [rows, setRows] = useState<CheckSummary[]>([])
  const [kind, setKind] = useState<Kind>('')
  const [status, setStatus] = useState('')
  const [fromDue, setFromDue] = useState('')
  const [toDue, setToDue] = useState('')
  const [search, setSearch] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [selected, setSelected] = useState<CheckSummary | null>(null)
  const [options, setOptions] = useState<CheckTransitionOption[]>([])
  const [partyNames, setPartyNames] = useState<Record<string, string>>({})
  const [treasuryNames, setTreasuryNames] = useState<Record<string, string>>({})

  const load = useCallback(async () => {
    setBusy(true)
    try {
      setDash(await getCheckDashboard())
      setRows(await getChecksFiltered(kind || undefined, status || undefined, fromDue || undefined, toDue || undefined))
      setError('')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }, [kind, status, fromDue, toDue])

  useEffect(() => {
    load()
  }, [load])

  // نام شخص و حساب خزانه یک بار خوانده می‌شود؛ جدول نباید به ازای هر ردیف پرس‌وجو بزند.
  useEffect(() => {
    ;(async () => {
      try {
        const parties = await getParties()
        setPartyNames(Object.fromEntries(parties.rows.map((p) => [p.id, p.display_name])))
      } catch {
        /* نام شخص اختیاری است؛ نبودش نباید صفحه را از کار بیندازد */
      }
      try {
        const accounts = await getTreasuryAccounts()
        setTreasuryNames(Object.fromEntries(accounts.map((a) => [a.id, a.name])))
      } catch {
        /* همان‌طور */
      }
    })()
  }, [])

  const filtered = useMemo(() => {
    const needle = search.trim()
    if (!needle) return rows
    return rows.filter(
      (r) =>
        r.check_number.includes(needle) ||
        (r.bank_name ?? '').includes(needle) ||
        (partyNames[r.party_id ?? ''] ?? '').includes(needle),
    )
  }, [rows, search, partyNames])

  const { sorted, sortProps } = useSort(filtered, 'due_date')

  const openDetail = async (row: CheckSummary) => {
    setSelected(row)
    setOptions([])
    try {
      setOptions(await getCheckTransitionOptions(row.id))
    } catch (e) {
      setError(errorText(e))
    }
  }

  const applyTransition = async (target: string) => {
    if (!selected) return
    setBusy(true)
    try {
      await updateCheckStatus(selected.id, target)
      setSelected(null)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const resetFilters = () => {
    setKind('')
    setStatus('')
    setFromDue('')
    setToDue('')
    setSearch('')
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">خزانه‌داری</div>
          <h1>مدیریت چک‌ها</h1>
          <p>چرخه‌ی وضعیت چک و سند حسابداری آن به‌طور کامل توسط هسته‌ی مالی کنترل می‌شود.</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}

      {dash && (
        <div className="metric-strip">
          <div>
            <span>چک‌های دریافتی</span>
            <b>{money(dash.total_received)} ریال</b>
            <small>{dash.received_count} فقره</small>
          </div>
          <div>
            <span>چک‌های پرداختی</span>
            <b>{money(dash.total_issued)} ریال</b>
            <small>{dash.issued_count} فقره</small>
          </div>
          <div>
            <span>سررسید هفته‌ی جاری</span>
            <b className="amber">{dash.due_soon_count} فقره</b>
            <small>نیازمند پیگیری</small>
          </div>
          <div>
            <span>سررسید گذشته</span>
            <b className="amber">{dash.overdue_count} فقره</b>
            <small>هنوز تعیین تکلیف نشده</small>
          </div>
          <div>
            <span>برگشتی</span>
            <b className="red-text">{dash.bounced_count} فقره</b>
            <small>اثر معکوس ثبت شده</small>
          </div>
        </div>
      )}

      <div className="panel filter-panel">
        <div className="filter-grid">
          <label>
            <span>نوع چک</span>
            <select value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
              <option value="">همه</option>
              <option value="received">دریافتی</option>
              <option value="issued">پرداختی</option>
            </select>
          </label>
          <label>
            <span>وضعیت</span>
            <select value={status} onChange={(e) => setStatus(e.target.value)}>
              <option value="">همه‌ی وضعیت‌ها</option>
              {Object.entries(CHECK_STATUS_LABELS).map(([value, text]) => (
                <option key={value} value={value}>
                  {text}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span>سررسید از</span>
            <input value={fromDue} onChange={(e) => setFromDue(e.target.value)} placeholder="1405/01/01" />
          </label>
          <label>
            <span>سررسید تا</span>
            <input value={toDue} onChange={(e) => setToDue(e.target.value)} placeholder="1405/12/29" />
          </label>
          <label className="grow">
            <span>جستجو در شماره چک، بانک یا نام شخص</span>
            <input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="جستجو…" />
          </label>
          <div className="filter-actions">
            <button className="ghost" onClick={resetFilters}>
              پاک‌کردن فیلترها
            </button>
            <button className="primary" onClick={load} disabled={busy}>
              <Icon name="refresh" /> بروزرسانی
            </button>
          </div>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>فهرست چک‌ها</h3>
            <p>{sorted.length} فقره — برای دیدن جزئیات و تغییر وضعیت روی هر ردیف کلیک کنید.</p>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('check_number')}>شماره چک</th>
                <th {...sortProps('check_type')}>نوع</th>
                <th>طرف حساب</th>
                <th {...sortProps('amount')}>مبلغ (ریال)</th>
                <th {...sortProps('issue_date')}>تاریخ صدور</th>
                <th {...sortProps('due_date')}>سررسید</th>
                <th>بانک</th>
                <th {...sortProps('status')}>وضعیت</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((r) => (
                <tr key={r.id} className="clickable" onClick={() => openDetail(r)}>
                  <td className="code">{r.check_number}</td>
                  <td>{r.check_type === 'received' ? 'دریافتی' : 'پرداختی'}</td>
                  <td>{partyNames[r.party_id ?? ''] ?? '—'}</td>
                  <td className="num">{money(r.amount)}</td>
                  <td>{r.issue_date}</td>
                  <td>{r.due_date}</td>
                  <td>{r.bank_name || '—'}</td>
                  <td>
                    <span className={`status ${checkStatusTone(r.status)}`}>{checkStatusLabel(r.status)}</span>
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={8} className="empty-row">
                    چکی با این فیلترها یافت نشد.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {selected && (
        <div className="modal-backdrop" onClick={() => setSelected(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-head">
              <div>
                <h2>چک شماره {selected.check_number}</h2>
                <p>{selected.check_type === 'received' ? 'چک دریافتی' : 'چک پرداختی'}</p>
              </div>
              <button aria-label="بستن" className="icon-btn" onClick={() => setSelected(null)}>
                <Icon name="close" />
              </button>
            </div>

            <dl className="detail-grid">
              <div>
                <dt>مبلغ</dt>
                <dd>{money(selected.amount)} ریال</dd>
              </div>
              <div>
                <dt>طرف حساب</dt>
                <dd>{partyNames[selected.party_id ?? ''] ?? '—'}</dd>
              </div>
              <div>
                <dt>تاریخ صدور</dt>
                <dd>{selected.issue_date}</dd>
              </div>
              <div>
                <dt>سررسید</dt>
                <dd>{selected.due_date}</dd>
              </div>
              <div>
                <dt>بانک</dt>
                <dd>{selected.bank_name || '—'}</dd>
              </div>
              <div>
                <dt>حساب خزانه</dt>
                <dd>{treasuryNames[selected.treasury_account_id ?? ''] ?? '—'}</dd>
              </div>
              <div>
                <dt>وضعیت فعلی</dt>
                <dd>
                  <span className={`status ${checkStatusTone(selected.status)}`}>
                    {checkStatusLabel(selected.status)}
                  </span>
                </dd>
              </div>
            </dl>

            <h3 className="section-title">تغییر وضعیت</h3>
            {options.length === 0 ? (
              <p className="muted">این چک در وضعیت پایانی است و گذار دیگری ندارد.</p>
            ) : (
              <div className="transition-list">
                {options.map((option) => (
                  <button
                    key={option.status}
                    className="transition-btn"
                    disabled={busy}
                    onClick={() => applyTransition(option.status)}
                  >
                    <b>{option.label}</b>
                    <small>{EFFECT_NOTE[option.treasury_effect]}</small>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </section>
  )
}
