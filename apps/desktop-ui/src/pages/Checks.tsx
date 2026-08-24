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
import { formatCount, formatRials as money, rialUnit } from '../lib/format'
import { checkStatusLabel, checkStatusTone, CHECK_STATUSES } from '../lib/checkStatus'
import { useI18n, type TranslationKey } from '../lib/i18n'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

type Kind = '' | 'received' | 'issued'

/** کلید ترجمه‌ی اثر خزانه‌ای هر گذار، تا کاربر بداند چه سندی صادر می‌شود. */
const EFFECT_NOTE: Record<CheckTransitionOption['treasury_effect'], TranslationKey> = {
  increase: 'checks.receiptVoucher',
  decrease: 'checks.paymentVoucher',
  none: 'checks.noFinancialEffect',
}

export function Checks() {
  const { t, locale } = useI18n()
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
          <div className="eyebrow">{t('checks.eyebrow')}</div>
          <h1>{t('checks.title')}</h1>
          <p>{t('checks.subtitle')}</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}

      {dash && (
        <div className="metric-strip">
          <div>
            <span>{t('checks.received')}</span>
            <b>{money(dash.total_received)} {rialUnit()}</b>
            <small>{t('invoices.itemsCount', {count: formatCount(dash.received_count)})}</small>
          </div>
          <div>
            <span>{t('checks.issued')}</span>
            <b>{money(dash.total_issued)} {rialUnit()}</b>
            <small>{t('invoices.itemsCount', {count: formatCount(dash.issued_count)})}</small>
          </div>
          <div>
            <span>{t('checks.dueThisWeek')}</span>
            <b className="amber">{t('invoices.itemsCount', {count: formatCount(dash.due_soon_count)})}</b>
            <small>{t('checks.needsFollowUp')}</small>
          </div>
          <div>
            <span>{t('checks.overdue')}</span>
            <b className="amber">{t('invoices.itemsCount', {count: formatCount(dash.overdue_count)})}</b>
            <small>{t('checks.unresolved')}</small>
          </div>
          <div>
            <span>{t('checks.bounced')}</span>
            <b className="red-text">{t('invoices.itemsCount', {count: formatCount(dash.bounced_count)})}</b>
            <small>{t('checks.bouncedNote')}</small>
          </div>
        </div>
      )}

      <div className="panel filter-panel">
        <div className="filter-grid">
          <label>
            <span>{t('checks.kind')}</span>
            <Select value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
              <option value="">{t('common.all')}</option>
              <option value="received">{t('checks.kindReceived')}</option>
              <option value="issued">{t('checks.kindIssued')}</option>
            </Select>
          </label>
          <label>
            <span>{t('common.status')}</span>
            <Select value={status} onChange={(e) => setStatus(e.target.value)}>
              <option value="">{t('invoices.allDocStatuses')}</option>
              {CHECK_STATUSES.map((value) => (
                <option key={value} value={value}>
                  {checkStatusLabel(value, locale)}
                </option>
              ))}
            </Select>
          </label>
          <label>
            <span>{t('checks.dueFrom')}</span>
            <input value={fromDue} onChange={(e) => setFromDue(e.target.value)} placeholder="1405/01/01" />
          </label>
          <label>
            <span>{t('checks.dueTo')}</span>
            <input value={toDue} onChange={(e) => setToDue(e.target.value)} placeholder="1405/12/29" />
          </label>
          <label className="grow">
            <span>{t('checks.searchHint')}</span>
            <input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t('common.searchShort')}
            />
          </label>
          <div className="filter-actions">
            <button className="ghost" onClick={resetFilters}>
              {t('common.clearFilters')}
            </button>
            <button className="primary" onClick={load} disabled={busy}>
              <Icon name="refresh" /> {t('common.refresh')}
            </button>
          </div>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>{t('checks.listTitle')}</h3>
            <p>{t('checks.listCount', {count: formatCount(sorted.length)})}</p>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('check_number')}>{t('checks.number')}</th>
                <th {...sortProps('check_type')}>{t('common.type')}</th>
                <th>{t('common.party')}</th>
                <th {...sortProps('amount')}>{t('checks.amountWithUnit', {unit: rialUnit()})}</th>
                <th {...sortProps('issue_date')}>{t('checks.issueDate')}</th>
                <th {...sortProps('due_date')}>{t('checks.dueDate')}</th>
                <th>{t('checks.bank')}</th>
                <th {...sortProps('status')}>{t('common.status')}</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((r) => (
                <tr key={r.id} className="clickable" onClick={() => openDetail(r)}>
                  <td className="code">{r.check_number}</td>
                  <td>{r.check_type === 'received' ? t('checks.kindReceived') : t('checks.kindIssued')}</td>
                  <td>{partyNames[r.party_id ?? ''] ?? '—'}</td>
                  <td className="num">{money(r.amount)}</td>
                  <td>{r.issue_date}</td>
                  <td>{r.due_date}</td>
                  <td>{r.bank_name || '—'}</td>
                  <td>
                    <span className={`status ${checkStatusTone(r.status)}`}>{checkStatusLabel(r.status, locale)}</span>
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={8} className="empty-row">
                    {t('checks.emptyRow')}
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
                <h2>{t('checks.detailTitle', {number: selected.check_number})}</h2>
                <p>
                  {selected.check_type === 'received'
                    ? t('checks.receivedCheque')
                    : t('checks.issuedCheque')}
                </p>
              </div>
              <button aria-label={t('common.close')} className="icon-btn" onClick={() => setSelected(null)}>
                <Icon name="close" />
              </button>
            </div>

            <dl className="detail-grid">
              <div>
                <dt>{t('common.amount')}</dt>
                <dd>{money(selected.amount)} {rialUnit()}</dd>
              </div>
              <div>
                <dt>{t('common.party')}</dt>
                <dd>{partyNames[selected.party_id ?? ''] ?? '—'}</dd>
              </div>
              <div>
                <dt>{t('checks.issueDate')}</dt>
                <dd>{selected.issue_date}</dd>
              </div>
              <div>
                <dt>{t('checks.dueDate')}</dt>
                <dd>{selected.due_date}</dd>
              </div>
              <div>
                <dt>{t('checks.bank')}</dt>
                <dd>{selected.bank_name || '—'}</dd>
              </div>
              <div>
                <dt>{t('checks.treasuryAccount')}</dt>
                <dd>{treasuryNames[selected.treasury_account_id ?? ''] ?? '—'}</dd>
              </div>
              <div>
                <dt>{t('checks.currentStatus')}</dt>
                <dd>
                  <span className={`status ${checkStatusTone(selected.status)}`}>
                    {checkStatusLabel(selected.status, locale)}
                  </span>
                </dd>
              </div>
            </dl>

            <h3 className="section-title">{t('checks.changeStatus')}</h3>
            {options.length === 0 ? (
              <p className="muted">{t('checks.finalStatus')}</p>
            ) : (
              <div className="transition-list">
                {options.map((option) => (
                  <button
                    key={option.status}
                    className="transition-btn"
                    disabled={busy}
                    onClick={() => applyTransition(option.status)}
                  >
                    {/* برچسب گذار از هسته می‌آید (فارسی)؛ برای زبان‌های دیگر
                      * همان وضعیت مقصد ترجمه می‌شود تا متن فارسی نماند. */}
                    <b>{locale === 'fa' ? option.label : checkStatusLabel(option.status, locale)}</b>
                    <small>{t(EFFECT_NOTE[option.treasury_effect])}</small>
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
