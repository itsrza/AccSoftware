import {useEffect, useMemo, useState} from 'react'
import {PartyForm} from './PartyForm'
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
import {useSort} from '../lib/useSort'

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
  const [balanceFilter, setBalanceFilter] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [editorOpen, setEditorOpen] = useState(false)
  const [editingId, setEditingId] = useState<string | undefined>()

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
      const matchesBalance = !balanceFilter || row.balance_status === balanceFilter
      return matchesGroup && matchesSearch && matchesBalance
    })
  }, [data, search, groupFilter, balanceFilter])

  const {sorted, toggle, headerClass} = useSort(visible, 'display_name')

  const openEditor = (row: PartyRow) => {
    setEditingId(row.id)
    setEditorOpen(true)
  }

  const openNew = () => {
    setEditingId(undefined)
    setEditorOpen(true)
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
        <button className="primary" onClick={openNew}>
          <Icon name="plus" /> افزودن شخص
        </button>
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
          <select value={balanceFilter} onChange={(event) => setBalanceFilter(event.target.value)}>
            <option value="">همه‌ی وضعیت‌ها</option>
            <option value="debtor">فقط بدهکاران</option>
            <option value="creditor">فقط بستانکاران</option>
            <option value="settled">فقط بی‌حساب</option>
          </select>
          <span className="spacer" />
          <button aria-label="بروزرسانی" className="icon-btn" onClick={load} title="بارگذاری مجدد">
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
                  <th className={headerClass('display_name')} onClick={() => toggle('display_name')}>
                    نام
                  </th>
                  <th className={headerClass('party_type_label')} onClick={() => toggle('party_type_label')}>
                    نوع
                  </th>
                  <th className={headerClass('party_function_label')} onClick={() => toggle('party_function_label')}>
                    نقش
                  </th>
                  <th className={headerClass('group_title')} onClick={() => toggle('group_title')}>
                    گروه
                  </th>
                  <th>مسیر</th>
                  <th>بازاریاب</th>
                  <th className={headerClass('credit_limit')} onClick={() => toggle('credit_limit')}>
                    سقف اعتبار
                  </th>
                  <th className={headerClass('balance')} onClick={() => toggle('balance')}>
                    بدهکار / بستانکار
                  </th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {sorted.map((row) => (
                  <tr key={row.id}>
                    <td>{row.display_name}</td>
                    <td>{row.party_type_label}</td>
                    <td>{row.party_function_label}</td>
                    <td>{row.group_title}</td>
                    <td>{row.route_title ?? '—'}</td>
                    <td>{row.marketer_name ?? '—'}</td>
                    <td>{row.credit_limit > 0 ? formatRials(row.credit_limit) : '—'}</td>
                    <td>
                      {row.balance === 0 ? (
                        <span className="amount-zero">بی‌حساب</span>
                      ) : (
                        <span
                          className={row.balance > 0 ? 'amount-debit' : 'amount-credit'}
                          title={row.balance > 0 ? 'بدهکار' : 'بستانکار'}
                        >
                          {formatRials(Math.abs(row.balance))}
                          <small> {row.balance_indicator}</small>
                        </span>
                      )}
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

      {editorOpen && (
        <PartyForm
          partyId={editingId}
          onClose={() => setEditorOpen(false)}
          onSaved={async () => {
            setEditorOpen(false)
            await load()
          }}
        />
      )}
    </section>
  )
}
