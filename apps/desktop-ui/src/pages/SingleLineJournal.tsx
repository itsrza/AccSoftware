import {useEffect, useMemo, useState} from 'react'
import {
  createSingleLineJournal,
  getCostCenters,
  getPostableAccounts,
  getProjects,
  type DimensionOption,
  type PostableAccount,
} from '../api'
import {errorText} from '../lib/errors'
import {formatRials, parseAmount, rialUnit, todayJalali} from '../lib/format'
import {useI18n} from '../lib/i18n'
import {Select} from '../components/Select'

/** یک طرف سند: حساب + ابعاد مالی */
type Side = {
  accountId: string
  subsidiaryId: string
  costCenterId: string
  projectId: string
}

const emptySide: Side = {accountId: '', subsidiaryId: '', costCenterId: '', projectId: ''}

/**
 * صدور سند حسابداری یک‌سطری.
 *
 * بازطراحی کامل فرم `Rb2xiG` نرم‌افزار فعلی: یک مبلغ، یک شرح، یک طرف بدهکار و
 * یک طرف بستانکار، با جابه‌جایی سریع طرفین و اعتبارسنجی ابعاد مالی در بک‌اند.
 */
export function SingleLineJournal() {
  const {t} = useI18n()
  const [accounts, setAccounts] = useState<PostableAccount[]>([])
  const [costCenters, setCostCenters] = useState<DimensionOption[]>([])
  const [projects, setProjects] = useState<DimensionOption[]>([])
  const [date, setDate] = useState(todayJalali())
  const [description, setDescription] = useState('')
  const [amountText, setAmountText] = useState('')
  const [debit, setDebit] = useState<Side>(emptySide)
  const [credit, setCredit] = useState<Side>(emptySide)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [busy, setBusy] = useState(false)

  const load = async () => {
    setError('')
    try {
      const [a, c, p] = await Promise.all([getPostableAccounts(), getCostCenters(), getProjects()])
      setAccounts(a)
      setCostCenters(c)
      setProjects(p)
    } catch (e) {
      setError(errorText(e))
    }
  }
  useEffect(() => {
    load()
  }, [])

  const amount = useMemo(() => parseAmount(amountText) ?? 0, [amountText])
  const accountOf = (id: string) => accounts.find((account) => account.id === id)

  const swap = () => {
    setDebit(credit)
    setCredit(debit)
  }

  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setError('')
    setSuccess('')
    if (amount <= 0) {
      setError(t('journal.errAmount'))
      return
    }
    setBusy(true)
    try {
      const id = await createSingleLineJournal(date, description, amount, debit, credit)
      setSuccess(`سند با موفقیت ثبت شد. شناسه: ${id}`)
      setAmountText('')
      setDescription('')
      setDebit(emptySide)
      setCredit(emptySide)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const sideEditor = (title: string, side: Side, setSide: (value: Side) => void) => {
    const account = accountOf(side.accountId)
    return (
      <div className="panel side-panel">
        <h3>{title}</h3>
        <label>
          <span>{t('reports.account')}</span>
          <Select
            value={side.accountId}
            onChange={(event) => setSide({...side, accountId: event.target.value})}
            required
          >
            <option value="">{t('invoiceForm.selectPlaceholder')}</option>
            {accounts.map((item) => (
              <option key={item.id} value={item.id}>
                {item.code} — {item.name}
              </option>
            ))}
          </Select>
        </label>

        {account?.requires_subsidiary && (
          <label>
            <span>{t('journal.subsidiaryRequired')}</span>
            <input
              value={side.subsidiaryId}
              onChange={(event) => setSide({...side, subsidiaryId: event.target.value})}
              placeholder={t('journal.subsidiaryId')}
              required
            />
          </label>
        )}

        <label>
          <span>مرکز هزینه{account?.requires_cost_center ? t('journal.requiredSuffix') : ''}</span>
          <Select
            value={side.costCenterId}
            onChange={(event) => setSide({...side, costCenterId: event.target.value})}
            required={account?.requires_cost_center}
          >
            <option value="">{t('journal.noCostCenter')}</option>
            {costCenters.map((item) => (
              <option key={item.id} value={item.code}>
                {item.code} — {item.title}
              </option>
            ))}
          </Select>
        </label>

        <label>
          <span>پروژه{account?.requires_project ? t('journal.requiredSuffix') : ''}</span>
          <Select
            value={side.projectId}
            onChange={(event) => setSide({...side, projectId: event.target.value})}
            required={account?.requires_project}
          >
            <option value="">{t('journal.noProject')}</option>
            {projects.map((item) => (
              <option key={item.id} value={item.code}>
                {item.code} — {item.title}
              </option>
            ))}
          </Select>
        </label>

        <div className="side-summary">
          {account ? `ماهیت: ${account.nature === 'credit' ? t('reports.credit') : t('reports.debit')}` : t('journal.noAccountPicked')}
        </div>
      </div>
    )
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('nav.group.accounting')}</div>
          <h1>{t('journal.title')}</h1>
          <p>{t('journal.subtitle')}</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {success && <div className="success-box">{success}</div>}

      <form onSubmit={submit}>
        <div className="panel">
          <div className="form-row">
            <label>
              <span>{t('common.date')}</span>
              <input value={date} onChange={(event) => setDate(event.target.value)} required />
            </label>
            <label className="grow">
              <span>{t('common.description')}</span>
              <input
                value={description}
                onChange={(event) => setDescription(event.target.value)}
                placeholder={t('journal.description')}
                required
              />
            </label>
            <label>
              <span>{t('journal.amountWithUnit', {unit: rialUnit()})}</span>
              <input
                value={amountText}
                onChange={(event) => setAmountText(event.target.value)}
                inputMode="numeric"
                required
              />
            </label>
          </div>
          {amount > 0 && <div className="amount-preview">{formatRials(amount)} ریال</div>}
        </div>

        <div className="sides">
          {sideEditor(t('journal.debitSide'), debit, setDebit)}
          <button type="button" className="swap-btn" onClick={swap} title={t('journal.swapSides')}>
            &lt;&gt;
          </button>
          {sideEditor(t('journal.creditSide'), credit, setCredit)}
        </div>

        <div className="form-actions">
          <button type="submit" className="primary" disabled={busy}>
            {busy ? t('journal.posting') : t('journal.post')}
          </button>
          <button type="button" onClick={load} disabled={busy}>
            {t('common.reload')}
          </button>
        </div>
      </form>
    </section>
  )
}
