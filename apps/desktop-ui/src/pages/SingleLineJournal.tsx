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
import {formatRials, parseAmount, todayJalali} from '../lib/format'

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
      setError('مبلغ سند باید بزرگ‌تر از صفر باشد.')
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
          <span>حساب</span>
          <select
            value={side.accountId}
            onChange={(event) => setSide({...side, accountId: event.target.value})}
            required
          >
            <option value="">انتخاب کنید…</option>
            {accounts.map((item) => (
              <option key={item.id} value={item.id}>
                {item.code} — {item.name}
              </option>
            ))}
          </select>
        </label>

        {account?.requires_subsidiary && (
          <label>
            <span>تفصیلی (الزامی)</span>
            <input
              value={side.subsidiaryId}
              onChange={(event) => setSide({...side, subsidiaryId: event.target.value})}
              placeholder="شناسه تفصیلی"
              required
            />
          </label>
        )}

        <label>
          <span>مرکز هزینه{account?.requires_cost_center ? ' (الزامی)' : ''}</span>
          <select
            value={side.costCenterId}
            onChange={(event) => setSide({...side, costCenterId: event.target.value})}
            required={account?.requires_cost_center}
          >
            <option value="">بدون مرکز هزینه</option>
            {costCenters.map((item) => (
              <option key={item.id} value={item.code}>
                {item.code} — {item.title}
              </option>
            ))}
          </select>
        </label>

        <label>
          <span>پروژه{account?.requires_project ? ' (الزامی)' : ''}</span>
          <select
            value={side.projectId}
            onChange={(event) => setSide({...side, projectId: event.target.value})}
            required={account?.requires_project}
          >
            <option value="">بدون پروژه</option>
            {projects.map((item) => (
              <option key={item.id} value={item.code}>
                {item.code} — {item.title}
              </option>
            ))}
          </select>
        </label>

        <div className="side-summary">
          {account ? `ماهیت: ${account.nature === 'credit' ? 'بستانکار' : 'بدهکار'}` : 'حسابی انتخاب نشده'}
        </div>
      </div>
    )
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">حسابداری</div>
          <h1>صدور سند یک‌سطری</h1>
          <p>سریع‌ترین راه ثبت سند: یک مبلغ، یک طرف بدهکار، یک طرف بستانکار.</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {success && <div className="success-box">{success}</div>}

      <form onSubmit={submit}>
        <div className="panel">
          <div className="form-row">
            <label>
              <span>تاریخ</span>
              <input value={date} onChange={(event) => setDate(event.target.value)} required />
            </label>
            <label className="grow">
              <span>شرح</span>
              <input
                value={description}
                onChange={(event) => setDescription(event.target.value)}
                placeholder="شرح سند"
                required
              />
            </label>
            <label>
              <span>مبلغ (ریال)</span>
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
          {sideEditor('طرف حساب بدهکار', debit, setDebit)}
          <button type="button" className="swap-btn" onClick={swap} title="جابه‌جایی طرفین">
            &lt;&gt;
          </button>
          {sideEditor('طرف حساب بستانکار', credit, setCredit)}
        </div>

        <div className="form-actions">
          <button type="submit" className="primary" disabled={busy}>
            {busy ? 'در حال ثبت…' : 'ثبت سند'}
          </button>
          <button type="button" onClick={load} disabled={busy}>
            بارگذاری مجدد
          </button>
        </div>
      </form>
    </section>
  )
}
