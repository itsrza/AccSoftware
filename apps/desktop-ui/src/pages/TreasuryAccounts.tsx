import { useCallback, useEffect, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  deactivateTreasuryAccount,
  getAccounts,
  getNegativePolicies,
  getTreasuryAccountDetails,
  saveTreasuryAccount,
  PolicyInfo,
  TreasuryAccountInput,
  TreasuryAccountRow,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { useSort } from '../lib/useSort'
import {Select} from '../components/Select'

type Mode = 'bank' | 'cash'

/** صندوق و تنخواه هر دو «نقد» هستند؛ در یک صفحه دیده می‌شوند. */
const TYPES_OF: Record<Mode, TreasuryAccountRow['account_type'][]> = {
  bank: ['bank'],
  cash: ['cash', 'petty_cash'],
}

const emptyForm = (mode: Mode): TreasuryAccountInput => ({
  name: '',
  account_type: mode === 'bank' ? 'bank' : 'cash',
  has_pos_terminal: false,
  negative_policy: mode === 'bank' ? 'warn' : 'error',
  is_active: true,
})

/**
 * صفحه‌ی تعریف بانک‌ها و صندوق‌ها.
 *
 * یک منطق، دو نما: صندوق و بانک از نظر حسابداری هر دو «حساب خزانه»اند و
 * مانده‌شان در یک گزارش جمع می‌شود. تفاوت فقط در فیلدهای تکمیلی است.
 */
export function TreasuryAccounts({ mode }: { mode: Mode }) {
  const [rows, setRows] = useState<TreasuryAccountRow[]>([])
  const [policies, setPolicies] = useState<PolicyInfo[]>([])
  const [ledger, setLedger] = useState<{ id: string; code: string; name: string }[]>([])
  const [form, setForm] = useState<TreasuryAccountInput | null>(null)
  const [showInactive, setShowInactive] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const all = await getTreasuryAccountDetails(undefined, showInactive)
      setRows(all.filter((row) => TYPES_OF[mode].includes(row.account_type)))
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [mode, showInactive])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    ;(async () => {
      try {
        setPolicies(await getNegativePolicies())
      } catch (e) {
        setError(errorText(e))
      }
      try {
        const accounts = await getAccounts()
        // فقط حساب‌های سطح تفصیلی و معین می‌توانند به خزانه وصل شوند.
        setLedger(
          accounts
            .filter((a) => a.level === 'detail' || a.level === 'subsidiary')
            .map((a) => ({ id: a.id, code: a.code, name: a.name })),
        )
      } catch {
        /* اتصال حسابداری اختیاری است */
      }
    })()
  }, [])

  const save = async () => {
    if (!form) return
    setBusy(true)
    setNotice('')
    try {
      await saveTreasuryAccount(form)
      setNotice(form.id ? 'تغییرات ذخیره شد.' : 'حساب جدید ساخته شد.')
      setForm(null)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const deactivate = async (row: TreasuryAccountRow) => {
    setBusy(true)
    try {
      await deactivateTreasuryAccount(row.id)
      setNotice(`«${row.name}» غیرفعال شد.`)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const edit = (row: TreasuryAccountRow) =>
    setForm({
      id: row.id,
      name: row.name,
      account_type: row.account_type,
      account_number: row.account_number,
      iban: row.iban,
      card_number: row.card_number,
      branch_name: row.branch_name,
      branch_code: row.branch_code,
      holder_name: row.holder_name,
      has_pos_terminal: row.has_pos_terminal,
      negative_policy: row.negative_policy,
      linked_account_id: row.linked_account_id,
      is_active: row.is_active,
    })

  const { sorted, sortProps } = useSort(rows, 'name')
  const totalBalance = rows.reduce((sum, row) => sum + row.balance, 0)
  const isBank = form?.account_type === 'bank'

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">خزانه‌داری</div>
          <h1>{mode === 'bank' ? 'حساب‌های بانکی' : 'صندوق‌ها و تنخواه'}</h1>
          <p>
            {mode === 'bank'
              ? 'شبا و شماره کارت با الگوریتم رسمی بررسی می‌شوند؛ شماره‌ی اشتباه یعنی حواله‌ی گم‌شده.'
              : 'صندوق نقدی نمی‌تواند منفی شود — پولی که در صندوق نیست قابل پرداخت نیست.'}
          </p>
        </div>
        <button className="primary" onClick={() => setForm(emptyForm(mode))}>
          <Icon name="plus" /> {mode === 'bank' ? 'حساب بانکی جدید' : 'صندوق جدید'}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      <div className="metric-strip">
        <div>
          <span>تعداد</span>
          <b>{rows.length}</b>
          <small>{mode === 'bank' ? 'حساب بانکی' : 'صندوق و تنخواه'}</small>
        </div>
        <div>
          <span>مانده‌ی کل</span>
          <b className={totalBalance < 0 ? 'red-text' : ''}>{money(totalBalance)} ریال</b>
          <small>دریافت منهای پرداخت</small>
        </div>
        <div>
          <span>دارای کارتخوان</span>
          <b>{rows.filter((r) => r.has_pos_terminal).length}</b>
          <small>پایانه فروشگاهی</small>
        </div>
        <div>
          <span>بدون اتصال حسابداری</span>
          <b className={rows.some((r) => !r.linked_account_id) ? 'amber' : ''}>
            {rows.filter((r) => !r.linked_account_id).length}
          </b>
          <small>سند خودکار صادر نمی‌شود</small>
        </div>
      </div>

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>فهرست</h3>
            <p>{sorted.length} مورد</p>
          </div>
          <div className="filter-actions">
            <label className="inline-check">
              <input
                type="checkbox"
                checked={showInactive}
                onChange={(e) => setShowInactive(e.target.checked)}
              />
              <span>نمایش غیرفعال‌ها</span>
            </label>
            <button className="icon-btn" onClick={load} aria-label="بروزرسانی">
              <Icon name="refresh" />
            </button>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th {...sortProps('name')}>نام</th>
                <th {...sortProps('account_type')}>نوع</th>
                {mode === 'bank' && <th>شماره حساب</th>}
                {mode === 'bank' && <th>شبا</th>}
                {mode === 'bank' && <th>شعبه</th>}
                <th>حساب حسابداری</th>
                <th {...sortProps('balance')}>مانده (ریال)</th>
                <th>سیاست منفی</th>
                <th>عملیات</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((row) => (
                <tr key={row.id} className={row.is_active ? '' : 'row-muted'}>
                  <td>
                    {row.name}
                    {row.has_pos_terminal && <span className="chip">کارتخوان</span>}
                  </td>
                  <td>{row.account_type_label}</td>
                  {mode === 'bank' && <td className="code">{row.account_number ?? '—'}</td>}
                  {mode === 'bank' && <td className="code">{row.iban ?? '—'}</td>}
                  {mode === 'bank' && <td>{row.branch_name ?? '—'}</td>}
                  <td>{row.linked_account_name ?? <span className="amber">وصل نشده</span>}</td>
                  <td className={`num${row.balance < 0 ? ' red-text' : ''}`}>
                    {money(row.balance)}
                  </td>
                  <td>{row.negative_policy_label}</td>
                  <td>
                    <button className="table-action" onClick={() => edit(row)}>
                      ویرایش
                    </button>
                    {row.is_active && (
                      <button
                        className="table-action"
                        disabled={busy}
                        onClick={() => deactivate(row)}
                      >
                        غیرفعال
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {sorted.length === 0 && (
                <tr>
                  <td colSpan={mode === 'bank' ? 9 : 6} className="empty-row">
                    موردی ثبت نشده است.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {form && (
        <div className="modal-backdrop" role="presentation">
          <div className="modal form-modal">
            <div className="modal-head">
              <div>
                <h2>{form.id ? 'ویرایش حساب' : 'حساب جدید'}</h2>
                <p>فیلدهای ستاره‌دار الزامی‌اند.</p>
              </div>
              <button aria-label="بستن" className="icon-btn" onClick={() => setForm(null)}>
                <Icon name="close" />
              </button>
            </div>

            <div className="filter-grid">
              <label className="grow">
                <span>نام حساب *</span>
                <input
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                  placeholder={mode === 'bank' ? 'بانک ملت — جاری ۱۲۳۴' : 'صندوق مرکزی'}
                />
              </label>
              <label>
                <span>نوع *</span>
                <Select
                  value={form.account_type}
                  onChange={(e) =>
                    setForm({
                      ...form,
                      account_type: e.target.value as TreasuryAccountRow['account_type'],
                    })
                  }
                >
                  {mode === 'bank' ? (
                    <option value="bank">حساب بانکی</option>
                  ) : (
                    <>
                      <option value="cash">صندوق</option>
                      <option value="petty_cash">تنخواه</option>
                    </>
                  )}
                </Select>
              </label>
              {isBank && (
                <>
                  <label>
                    <span>شماره حساب</span>
                    <input
                      value={form.account_number ?? ''}
                      onChange={(e) => setForm({ ...form, account_number: e.target.value })}
                    />
                  </label>
                  <label className="grow">
                    <span>شماره شبا</span>
                    <input
                      value={form.iban ?? ''}
                      onChange={(e) => setForm({ ...form, iban: e.target.value })}
                      placeholder="IR280620000000001234567891"
                    />
                  </label>
                  <label>
                    <span>شماره کارت</span>
                    <input
                      value={form.card_number ?? ''}
                      onChange={(e) => setForm({ ...form, card_number: e.target.value })}
                      placeholder="6037991234567893"
                    />
                  </label>
                  <label>
                    <span>نام شعبه</span>
                    <input
                      value={form.branch_name ?? ''}
                      onChange={(e) => setForm({ ...form, branch_name: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>کد شعبه</span>
                    <input
                      value={form.branch_code ?? ''}
                      onChange={(e) => setForm({ ...form, branch_code: e.target.value })}
                    />
                  </label>
                  <label>
                    <span>نام صاحب حساب</span>
                    <input
                      value={form.holder_name ?? ''}
                      onChange={(e) => setForm({ ...form, holder_name: e.target.value })}
                    />
                  </label>
                </>
              )}
              <label className="grow">
                <span>حساب حسابداری متصل</span>
                <Select
                  value={form.linked_account_id ?? ''}
                  onChange={(e) => setForm({ ...form, linked_account_id: e.target.value })}
                >
                  <option value="">وصل نشده — سند خودکار صادر نمی‌شود</option>
                  {ledger.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.code} — {a.name}
                    </option>
                  ))}
                </Select>
              </label>
            </div>

            <h3 className="section-title">هشدار منفی شدن موجودی</h3>
            <div className="policy-list">
              {policies.map((policy) => (
                <label
                  key={policy.value}
                  className={`policy-card${form.negative_policy === policy.value ? ' selected' : ''}`}
                >
                  <input
                    type="radio"
                    name="negative-policy"
                    checked={form.negative_policy === policy.value}
                    onChange={() => setForm({ ...form, negative_policy: policy.value })}
                  />
                  <b>{policy.label}</b>
                  <small>{policy.explanation}</small>
                </label>
              ))}
            </div>

            <div className="form-row checkbox-row">
              <label className="inline-check">
                <input
                  type="checkbox"
                  checked={form.has_pos_terminal}
                  onChange={(e) => setForm({ ...form, has_pos_terminal: e.target.checked })}
                />
                <span>دارای پایانه فروشگاهی (کارتخوان)</span>
              </label>
              <label className="inline-check">
                <input
                  type="checkbox"
                  checked={form.is_active}
                  onChange={(e) => setForm({ ...form, is_active: e.target.checked })}
                />
                <span>فعال</span>
              </label>
            </div>

            <div className="modal-actions">
              <button className="primary" onClick={save} disabled={busy || !form.name.trim()}>
                ذخیره
              </button>
              <button className="ghost" onClick={() => setForm(null)}>
                انصراف
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
