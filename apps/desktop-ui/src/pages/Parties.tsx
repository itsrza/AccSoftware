import {useEffect, useMemo, useState} from 'react'
import {
  getParties,
  getPartyRoutes,
  updatePartyProfile,
  validatePartyIdentity,
  type PartyListResult,
  type PartyRow,
  type RouteRow,
} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {formatRials} from '../lib/format'

const PARTY_TYPES = [
  {value: 'natural', label: 'حقیقی'},
  {value: 'private_legal', label: 'حقوقی غیردولتی'},
  {value: 'government_legal', label: 'حقوقی دولتی'},
  {value: 'civil_partnership', label: 'مشارکت مدنی'},
]

const PARTY_FUNCTIONS = [
  {value: 'person', label: 'شخص'},
  {value: 'marketer', label: 'بازاریاب'},
  {value: 'supervisor', label: 'سوپروایزر'},
]

/**
 * مدیریت اشخاص — بازطراحی صفحه‌ی «لیست اشخاص».
 *
 * پنل خلاصه‌ی حساب (بدهکاران / بستانکاران / بی‌حساب) از بک‌اند می‌آید و روی
 * مانده‌ی واقعی اسناد ثبت‌شده محاسبه می‌شود، نه داده‌ی نمایشی.
 */
export function Parties() {
  const [data, setData] = useState<PartyListResult | null>(null)
  const [routes, setRoutes] = useState<RouteRow[]>([])
  const [search, setSearch] = useState('')
  const [groupFilter, setGroupFilter] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [editing, setEditing] = useState<PartyRow | null>(null)
  const [form, setForm] = useState({
    party_type: 'natural',
    party_function: 'person',
    national_id: '',
    economic_code: '',
    postal_code: '',
    credit_limit: '0',
    route_id: '',
    marketer_id: '',
  })
  const [problems, setProblems] = useState<string[]>([])
  const [saving, setSaving] = useState(false)

  const load = async () => {
    setLoading(true)
    setError('')
    try {
      const [list, routeRows] = await Promise.all([getParties(), getPartyRoutes()])
      setData(list)
      setRoutes(routeRows)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }
  useEffect(() => {
    load()
  }, [])

  const groups = useMemo(
    () => Array.from(new Set((data?.rows ?? []).map((row) => row.group_title))),
    [data],
  )

  const visible = useMemo(() => {
    const term = search.trim()
    return (data?.rows ?? []).filter((row) => {
      const matchesGroup = !groupFilter || row.group_title === groupFilter
      const matchesSearch =
        !term || row.display_name.includes(term) || (row.mobile ?? '').includes(term)
      return matchesGroup && matchesSearch
    })
  }, [data, search, groupFilter])

  const openEditor = (row: PartyRow) => {
    setEditing(row)
    setProblems([])
    setForm({
      party_type: row.party_type,
      party_function: row.party_function,
      national_id: '',
      economic_code: '',
      postal_code: '',
      credit_limit: String(row.credit_limit),
      route_id: '',
      marketer_id: '',
    })
  }

  const checkIdentity = async () => {
    try {
      const found = await validatePartyIdentity({
        partyType: form.party_type,
        nationalId: form.national_id || null,
        economicCode: form.economic_code || null,
        postalCode: form.postal_code || null,
        mobile: editing?.mobile ?? null,
        iban: null,
        cardNumber: null,
      })
      setProblems(found)
      return found.length === 0
    } catch (e) {
      setError(errorText(e))
      return false
    }
  }

  const save = async () => {
    if (!editing) return
    setSaving(true)
    setError('')
    try {
      if (!(await checkIdentity())) return
      await updatePartyProfile({
        contactId: editing.id,
        partyType: form.party_type,
        partyFunction: form.party_function,
        nationalId: form.national_id || null,
        economicCode: form.economic_code || null,
        postalCode: form.postal_code || null,
        creditLimit: Number(form.credit_limit) || 0,
        routeId: form.route_id || null,
        marketerId: form.marketer_id || null,
      })
      setEditing(null)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setSaving(false)
    }
  }

  const summary = data?.summary

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">PARTIES</div>
          <h1>مدیریت اشخاص</h1>
          <p>مشتریان، تأمین‌کنندگان، بازاریاب‌ها و سوپروایزرها با مانده‌ی واقعی حساب.</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}

      {summary && (
        <div className="metric-strip">
          <div>
            <span>بدهکاران</span>
            <b className="red-text">{formatRials(summary.debtor_total)}</b>
            <small>{summary.debtor_count} نفر</small>
          </div>
          <div>
            <span>بستانکاران</span>
            <b className="green-text">{formatRials(summary.creditor_total)}</b>
            <small>{summary.creditor_count} نفر</small>
          </div>
          <div>
            <span>بی‌حساب</span>
            <b>{summary.settled_count}</b>
            <small>نفر</small>
          </div>
          <div>
            <span>خالص مانده</span>
            <b className={summary.net_total >= 0 ? 'red-text' : 'green-text'}>
              {formatRials(Math.abs(summary.net_total))}
            </b>
            <small>{summary.total_count} شخص</small>
          </div>
        </div>
      )}

      <div className="panel list-panel">
        <div className="toolbar">
          <input
            className="search-input"
            placeholder="جستجوی نام یا موبایل…"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <select value={groupFilter} onChange={(event) => setGroupFilter(event.target.value)}>
            <option value="">همه‌ی گروه‌ها</option>
            {groups.map((group) => (
              <option key={group} value={group}>
                {group}
              </option>
            ))}
          </select>
          <button className="icon-btn" onClick={load} title="بارگذاری مجدد">
            <Icon name="refresh" />
          </button>
        </div>

        {loading ? (
          <div className="empty-state">در حال بارگذاری…</div>
        ) : visible.length === 0 ? (
          <div className="empty-state">شخصی یافت نشد.</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>نام</th>
                  <th>نوع</th>
                  <th>نقش</th>
                  <th>گروه</th>
                  <th>مسیر</th>
                  <th>بازاریاب</th>
                  <th>سقف اعتبار</th>
                  <th>حساب فعلی</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {visible.map((row) => (
                  <tr key={row.id}>
                    <td>{row.display_name}</td>
                    <td>{row.party_type_label}</td>
                    <td>{row.party_function_label}</td>
                    <td>{row.group_title}</td>
                    <td>{row.route_title ?? '—'}</td>
                    <td>{row.marketer_name ?? '—'}</td>
                    <td>{row.credit_limit > 0 ? formatRials(row.credit_limit) : '—'}</td>
                    <td>
                      <span
                        className={
                          row.balance_status === 'debtor'
                            ? 'status danger'
                            : row.balance_status === 'creditor'
                              ? 'status done'
                              : 'status pending'
                        }
                      >
                        {row.balance_indicator}
                      </span>{' '}
                      {row.balance !== 0 && formatRials(Math.abs(row.balance))}
                    </td>
                    <td>
                      <button className="table-action" onClick={() => openEditor(row)}>
                        مشخصات
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {editing && (
        <div className="modal-backdrop" onClick={() => setEditing(null)}>
          <div className="modal" onClick={(event) => event.stopPropagation()}>
            <h2>مشخصات تکمیلی: {editing.display_name}</h2>

            {problems.length > 0 && (
              <div className="error-box">
                {problems.map((problem) => (
                  <div key={problem}>{problem}</div>
                ))}
              </div>
            )}

            <div className="form-row">
              <label>
                <span>نوع شخصیت</span>
                <select
                  value={form.party_type}
                  onChange={(event) => setForm({...form, party_type: event.target.value})}
                >
                  {PARTY_TYPES.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>نقش</span>
                <select
                  value={form.party_function}
                  onChange={(event) => setForm({...form, party_function: event.target.value})}
                >
                  {PARTY_FUNCTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <div className="form-row">
              <label>
                <span>{form.party_type === 'natural' ? 'کد ملی' : 'شناسه ملی'}</span>
                <input
                  value={form.national_id}
                  onChange={(event) => setForm({...form, national_id: event.target.value})}
                  onBlur={checkIdentity}
                  inputMode="numeric"
                />
              </label>
              <label>
                <span>کد اقتصادی</span>
                <input
                  value={form.economic_code}
                  onChange={(event) => setForm({...form, economic_code: event.target.value})}
                  onBlur={checkIdentity}
                  inputMode="numeric"
                />
              </label>
              <label>
                <span>کد پستی</span>
                <input
                  value={form.postal_code}
                  onChange={(event) => setForm({...form, postal_code: event.target.value})}
                  onBlur={checkIdentity}
                  inputMode="numeric"
                />
              </label>
            </div>

            <div className="form-row">
              <label>
                <span>سقف اعتبار (ریال)</span>
                <input
                  value={form.credit_limit}
                  onChange={(event) => setForm({...form, credit_limit: event.target.value})}
                  inputMode="numeric"
                />
              </label>
              <label>
                <span>مسیر پخش</span>
                <select
                  value={form.route_id}
                  onChange={(event) => setForm({...form, route_id: event.target.value})}
                >
                  <option value="">—</option>
                  {routes.map((route) => (
                    <option key={route.id} value={route.id}>
                      {route.code} — {route.title}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>بازاریاب</span>
                <select
                  value={form.marketer_id}
                  onChange={(event) => setForm({...form, marketer_id: event.target.value})}
                >
                  <option value="">—</option>
                  {(data?.rows ?? [])
                    .filter((row) => row.party_function !== 'person')
                    .map((row) => (
                      <option key={row.id} value={row.id}>
                        {row.display_name}
                      </option>
                    ))}
                </select>
              </label>
            </div>

            <div className="form-actions">
              <button className="primary" onClick={save} disabled={saving}>
                {saving ? 'در حال ذخیره…' : 'ذخیره'}
              </button>
              <button onClick={() => setEditing(null)}>انصراف</button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
