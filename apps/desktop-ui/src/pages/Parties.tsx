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
import {formatCount, formatRials} from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'
import {useSort} from '../lib/useSort'
import {Select} from '../components/Select'

/* برچسب نوع و نقش شخص از هسته می‌آید (فارسی). برای انگلیسی و عربی همان
 * شناسه‌ی ساختاری ترجمه می‌شود تا در فهرست، متن فارسی جا نماند. */
const PARTY_TYPE_KEY: Record<string, TranslationKey> = {
  natural: 'parties.kind.natural',
  private_legal: 'parties.kind.private',
  government_legal: 'parties.kind.government',
  civil_partnership: 'parties.kind.partnership',
}

const PARTY_FUNCTION_KEY: Record<string, TranslationKey> = {
  person: 'parties.role.person',
  marketer: 'parties.role.agent',
  supervisor: 'parties.role.supervisor',
}

/**
 * مدیریت اشخاص — بازطراحی صفحه‌ی «لیست اشخاص».
 *
 * پنل خلاصه‌ی حساب (بدهکاران / بستانکاران / بی‌حساب) از بک‌اند می‌آید و روی
 * مانده‌ی واقعی اسناد ثبت‌شده محاسبه می‌شود، نه داده‌ی نمایشی.
 */
export function Parties() {
  const {t, locale} = useI18n()
  /** فارسی از هسته می‌آید؛ زبان‌های دیگر از دیکشنری. */
  const typeLabel = (row: PartyRow) =>
    locale === 'fa' || !PARTY_TYPE_KEY[row.party_type]
      ? row.party_type_label
      : t(PARTY_TYPE_KEY[row.party_type])
  const functionLabel = (row: PartyRow) =>
    locale === 'fa' || !PARTY_FUNCTION_KEY[row.party_function]
      ? row.party_function_label
      : t(PARTY_FUNCTION_KEY[row.party_function])
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
          <div className="eyebrow">{t('parties.eyebrow')}</div>
          <h1>{t('parties.title')}</h1>
          <p>{t('parties.subtitle')}</p>
        </div>
        <button className="primary" onClick={openNew}>
          <Icon name="plus" /> {t('parties.add')}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}

      {summary && (
        <div className="metric-strip">
          <div>
            <span>{t('parties.debtors')}</span>
            <b className="red-text">{formatRials(summary.debtor_total)}</b>
            <small>{t('parties.peopleCount', {count: formatCount(summary.debtor_count)})}</small>
          </div>
          <div>
            <span>{t('parties.creditors')}</span>
            <b className="green-text">{formatRials(summary.creditor_total)}</b>
            <small>{t('parties.peopleCount', {count: formatCount(summary.creditor_count)})}</small>
          </div>
          <div>
            <span>{t('parties.balanced')}</span>
            <b>{formatCount(summary.settled_count)}</b>
            <small>{t('common.person')}</small>
          </div>
          <div>
            <span>{t('parties.netBalance')}</span>
            <b className={summary.net_total >= 0 ? 'red-text' : 'green-text'}>
              {formatRials(Math.abs(summary.net_total))}
            </b>
            <small>{t('parties.personsCount', {count: formatCount(summary.total_count)})}</small>
          </div>
        </div>
      )}

      <div className="panel list-panel">
        <div className="toolbar">
          <input
            className="search-input"
            placeholder={t('parties.searchHint')}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <Select value={groupFilter} onChange={(event) => setGroupFilter(event.target.value)}>
            <option value="">{t('parties.allGroups')}</option>
            {groups.map((group) => (
              <option key={group} value={group}>
                {group}
              </option>
            ))}
          </Select>
          <Select value={balanceFilter} onChange={(event) => setBalanceFilter(event.target.value)}>
            <option value="">{t('parties.allStatuses')}</option>
            <option value="debtor">{t('parties.onlyDebtors')}</option>
            <option value="creditor">{t('parties.onlyCreditors')}</option>
            <option value="settled">{t('parties.onlyBalanced')}</option>
          </Select>
          <span className="spacer" />
          <button
            aria-label={t('common.refresh')}
            className="icon-btn"
            onClick={load}
            title={t('common.reload')}
          >
            <Icon name="refresh" />
          </button>
        </div>

        {loading ? (
          <div className="empty-state">{t('common.loading')}</div>
        ) : visible.length === 0 ? (
          <div className="empty-state">{t('parties.empty')}</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th className={headerClass('display_name')} onClick={() => toggle('display_name')}>
                    {t('common.name')}
                  </th>
                  <th className={headerClass('party_type_label')} onClick={() => toggle('party_type_label')}>
                    {t('common.type')}
                  </th>
                  <th className={headerClass('party_function_label')} onClick={() => toggle('party_function_label')}>
                    {t('parties.role')}
                  </th>
                  <th className={headerClass('group_title')} onClick={() => toggle('group_title')}>
                    {t('common.group')}
                  </th>
                  <th>{t('parties.route')}</th>
                  <th>{t('parties.marketer')}</th>
                  <th className={headerClass('credit_limit')} onClick={() => toggle('credit_limit')}>
                    {t('parties.creditLimit')}
                  </th>
                  <th className={headerClass('balance')} onClick={() => toggle('balance')}>
                    {t('parties.balanceColumn')}
                  </th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {sorted.map((row) => (
                  <tr key={row.id}>
                    <td>{row.display_name}</td>
                    <td>{typeLabel(row)}</td>
                    <td>{functionLabel(row)}</td>
                    <td>{row.group_title}</td>
                    <td>{row.route_title ?? '—'}</td>
                    <td>{row.marketer_name ?? '—'}</td>
                    <td>{row.credit_limit > 0 ? formatRials(row.credit_limit) : '—'}</td>
                    <td>
                      {row.balance === 0 ? (
                        <span className="amount-zero">{t('parties.balanced')}</span>
                      ) : (
                        <span
                          className={row.balance > 0 ? 'amount-debit' : 'amount-credit'}
                          title={row.balance > 0 ? t('parties.debtor') : t('parties.creditor')}
                        >
                          {formatRials(Math.abs(row.balance))}
                          <small> {row.balance_indicator}</small>
                        </span>
                      )}
                    </td>
                    <td>
                      <button className="table-action" onClick={() => openEditor(row)}>
                        {t('parties.profile')}
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
