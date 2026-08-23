import { useEffect, useMemo, useState } from 'react'
import { RefreshCw } from 'lucide-react'
import {
  getAccountLedgerSummary,
  getCashPosition,
  getFinancialStatement,
  getInventoryValuation,
  getJournalBook,
  getPartyAging,
  getPayables,
  getProfitLoss,
  getPurchaseReport,
  getReceivables,
  getSalesReport,
  getTrialBalance,
  AccountLedgerSummary,
  CashPosition,
  FinancialStatement,
  InventoryValuation,
  JournalBookLine,
  PartyAging,
  PartyBalance,
  ProfitLoss,
  PurchaseReportRow,
  SalesReportRow,
  TrialBalance,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { Card, CardHeader, EmptyState, ErrorState, Skeleton } from '../components/ui'
import { FilterBar } from '../components/FilterBar'
import { defaultRange, useFiscalRange } from './invoiceListData'
import { resolveRange, type JalaliRange } from '../lib/dateRange'

/**
 * مرکز گزارشات.
 *
 * ## چرا انتخاب‌گر به‌جای زبانه
 * چهارده گزارش به‌صورت چهارده دکمه‌ی کنار هم، در هر نمایشگری دو خط می‌شد و
 * از کادر بیرون می‌زد. یک انتخاب‌گر گروه‌بندی‌شده هم جا می‌گیرد، هم گزارش‌ها
 * را دسته‌بندی می‌کند و هم روی نمایشگر کوچک نمی‌شکند.
 *
 * ## چرا نوار فیلتر مشترک
 * همان نوار داشبورد و فهرست فاکتور: بازه‌ی شمسی با پیش‌تنظیم و بازنشانی.
 * کاربر یک بار یاد می‌گیرد و همه‌جا همان است.
 */

type Kind =
  | 'sales'
  | 'purchase'
  | 'inventory'
  | 'ledger'
  | 'trial'
  | 'profit'
  | 'receivable'
  | 'payable'
  | 'cash'
  | 'journal'
  | 'balance'
  | 'income'
  | 'agingReceivable'
  | 'agingPayable'

const GROUPS: { label: string; items: [Kind, string][] }[] = [
  {
    label: 'فروش و خرید',
    items: [
      ['sales', 'گزارش فروش'],
      ['purchase', 'گزارش خرید'],
    ],
  },
  {
    label: 'انبار',
    items: [['inventory', 'ارزش موجودی انبار']],
  },
  {
    label: 'دفاتر قانونی',
    items: [
      ['journal', 'دفتر روزنامه'],
      ['ledger', 'گردش حساب‌ها'],
      ['trial', 'تراز آزمایشی'],
    ],
  },
  {
    label: 'صورت‌های مالی',
    items: [
      ['balance', 'ترازنامه'],
      ['income', 'صورت سود و زیان'],
      ['profit', 'خلاصه‌ی سود ناخالص'],
    ],
  },
  {
    label: 'طرف حساب‌ها',
    items: [
      ['receivable', 'مطالبات'],
      ['payable', 'بدهی‌ها'],
      ['agingReceivable', 'سنی‌سازی مطالبات'],
      ['agingPayable', 'سنی‌سازی بدهی‌ها'],
    ],
  },
  {
    label: 'خزانه',
    items: [['cash', 'وضعیت نقدینگی']],
  },
]

const TITLE: Record<Kind, string> = Object.fromEntries(
  GROUPS.flatMap((group) => group.items),
) as Record<Kind, string>

/** گزارش‌هایی که بازه‌ی زمانی روی آن‌ها اثر دارد. بقیه «در لحظه»اند. */
const PERIODIC: Kind[] = ['sales', 'purchase', 'ledger', 'journal']
/** گزارش‌هایی که فقط «تا تاریخ» می‌گیرند. */
const AS_OF: Kind[] = ['balance', 'income', 'agingReceivable', 'agingPayable']

const paymentLabel = (value: string) =>
  value === 'paid' ? 'تسویه شده' : value === 'partial' ? 'تسویه جزئی' : 'تسویه نشده'

function Table({ headers, rows }: { headers: string[]; rows: (string | number)[][] }) {
  if (rows.length === 0) {
    return <EmptyState title="داده‌ای برای این گزارش نیست." hint="بازه یا نوع گزارش را عوض کنید." />
  }
  return (
    <div className="table-wrap">
      <table className="large-table">
        <thead>
          <tr>
            {headers.map((header) => (
              <th key={header}>{header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            <tr key={index}>
              {row.map((cell, column) => (
                <td key={column}>{column === 0 ? <b>{cell}</b> : cell}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function Stat({ title, value, unit = 'ریال' }: { title: string; value: number; unit?: string }) {
  return (
    <article
      data-card
      className="rounded-[var(--radius)] border border-border bg-card p-3.5 shadow-[var(--shadow-sm)]"
    >
      <p className="text-[11px] font-semibold text-muted">{title}</p>
      <p className="tnum mt-1.5 truncate text-[17px] font-extrabold text-text">
        {money(value)}
        <span className="ms-1 text-[10px] font-semibold text-faint">{unit}</span>
      </p>
    </article>
  )
}

export function Reports() {
  const [kind, setKind] = useState<Kind>('sales')
  const fiscalRange = useFiscalRange()
  const [range, setRange] = useState<JalaliRange>(() => defaultRange())

  const [sales, setSales] = useState<SalesReportRow[]>([])
  const [purchase, setPurchase] = useState<PurchaseReportRow[]>([])
  const [inventory, setInventory] = useState<InventoryValuation[]>([])
  const [ledger, setLedger] = useState<AccountLedgerSummary[]>([])
  const [trial, setTrial] = useState<TrialBalance>()
  const [profit, setProfit] = useState<ProfitLoss>()
  const [receivable, setReceivable] = useState<PartyBalance[]>([])
  const [payable, setPayable] = useState<PartyBalance[]>([])
  const [cash, setCash] = useState<CashPosition>()
  const [journal, setJournal] = useState<JournalBookLine[]>([])
  const [balance, setBalance] = useState<FinancialStatement>()
  const [income, setIncome] = useState<FinancialStatement>()
  const [agingR, setAgingR] = useState<PartyAging[]>([])
  const [agingP, setAgingP] = useState<PartyAging[]>([])

  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!fiscalRange) return
    setRange((current) =>
      current.preset === 'fiscalYear' ? { preset: 'fiscalYear', ...fiscalRange } : current,
    )
  }, [fiscalRange])

  const load = async () => {
    setLoading(true)
    setError('')
    const from = range.from
    const to = range.to
    try {
      switch (kind) {
        case 'sales':
          setSales(await getSalesReport(from, to))
          break
        case 'purchase':
          setPurchase(await getPurchaseReport(from, to))
          break
        case 'inventory':
          setInventory(await getInventoryValuation())
          break
        case 'ledger':
          setLedger(await getAccountLedgerSummary(from, to))
          break
        case 'trial':
          setTrial(await getTrialBalance())
          break
        case 'profit':
          setProfit(await getProfitLoss())
          break
        case 'receivable':
          setReceivable(await getReceivables())
          break
        case 'payable':
          setPayable(await getPayables())
          break
        case 'cash':
          setCash(await getCashPosition())
          break
        case 'journal':
          setJournal(await getJournalBook(from, to))
          break
        case 'balance':
          setBalance(await getFinancialStatement('balance_sheet', to))
          break
        case 'income':
          setIncome(await getFinancialStatement('income_statement', to))
          break
        case 'agingReceivable':
          setAgingR(await getPartyAging(true, to))
          break
        case 'agingPayable':
          setAgingP(await getPartyAging(false, to))
          break
      }
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void load()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind, range.from, range.to])

  const scope = useMemo(() => {
    if (PERIODIC.includes(kind)) return `بازه: ${range.from} تا ${range.to}`
    if (AS_OF.includes(kind)) return `تا تاریخ ${range.to}`
    return 'مانده در لحظه — مستقل از بازه'
  }, [kind, range])

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">گزارش‌های واقعی</div>
          <h1>مرکز گزارشات</h1>
          <p>تمام ارقام از اسناد ثبت‌شده خوانده می‌شوند؛ هیچ عددی تخمینی نیست.</p>
        </div>
        <button className="ghost" onClick={() => void load()}>
          <RefreshCw className="size-4" aria-hidden /> بروزرسانی
        </button>
      </div>

      <FilterBar
        range={range}
        onRange={setRange}
        fiscalRange={fiscalRange}
        filters={[
          {
            key: 'kind',
            label: 'نوع گزارش',
            value: kind,
            width: 'xl:w-64',
            onChange: (value) => setKind(value as Kind),
            options: GROUPS.flatMap((group) =>
              group.items.map(([value, label]) => ({ value, label: `${group.label} — ${label}` })),
            ),
          },
        ]}
        onReset={() => {
          setKind('sales')
          setRange(resolveRange('fiscalYear', fiscalRange))
        }}
        isDefault={kind === 'sales' && range.preset === 'fiscalYear'}
        note={scope}
      />

      {error && <ErrorState onRetry={() => void load()} />}

      <Card pad={false}>
        <div className="p-4 sm:p-5">
          <CardHeader title={TITLE[kind]} subtitle={scope} />
        </div>

        {loading ? (
          <div className="px-4 pb-5 sm:px-5">
            <Skeleton className="h-64 w-full" />
          </div>
        ) : (
          <div className="px-2 pb-4 sm:px-3">
            {kind === 'sales' && (
              <Table
                headers={['تاریخ', 'شماره', 'مشتری', 'خالص', 'مالیات', 'مبلغ', 'تسویه']}
                rows={sales.map((row) => [
                  row.date,
                  row.invoice_number,
                  row.contact_name || 'بدون شخص',
                  money(row.subtotal - row.discount),
                  money(row.tax),
                  money(row.total),
                  paymentLabel(row.payment_status),
                ])}
              />
            )}
            {kind === 'purchase' && (
              <Table
                headers={['تاریخ', 'شماره', 'تأمین‌کننده', 'خالص', 'مالیات', 'مبلغ', 'تسویه']}
                rows={purchase.map((row) => [
                  row.date,
                  row.invoice_number,
                  row.contact_name || 'بدون شخص',
                  money(row.subtotal - row.discount),
                  money(row.tax),
                  money(row.total),
                  paymentLabel(row.payment_status),
                ])}
              />
            )}
            {kind === 'inventory' && (
              <Table
                headers={['کالا', 'انبار', 'موجودی', 'میانگین بها', 'ارزش']}
                rows={inventory.map((row) => [
                  row.product_name,
                  row.warehouse_name,
                  money(row.quantity),
                  money(row.average_cost),
                  money(row.value),
                ])}
              />
            )}
            {kind === 'ledger' && (
              <Table
                headers={['کد', 'حساب', 'بدهکار', 'بستانکار', 'مانده']}
                rows={ledger.map((row) => [
                  row.code,
                  row.name,
                  money(row.debit),
                  money(row.credit),
                  `${money(Math.abs(row.balance))} ${row.balance >= 0 ? 'بدهکار' : 'بستانکار'}`,
                ])}
              />
            )}
            {kind === 'trial' && trial && (
              <>
                <div className="mb-3 grid grid-cols-1 gap-3 px-2 sm:grid-cols-3">
                  <Stat title="جمع بدهکار" value={trial.total_debit} />
                  <Stat title="جمع بستانکار" value={trial.total_credit} />
                  <article
                    data-card
                    className="rounded-[var(--radius)] border border-border bg-card p-3.5"
                  >
                    <p className="text-[11px] font-semibold text-muted">کنترل توازن</p>
                    <p
                      className={`mt-1.5 text-[17px] font-extrabold ${
                        trial.total_debit === trial.total_credit ? 'text-success' : 'text-danger'
                      }`}
                    >
                      {trial.total_debit === trial.total_credit ? 'متوازن' : 'نامتوازن'}
                    </p>
                  </article>
                </div>
                <Table
                  headers={['کد', 'حساب', 'بدهکار', 'بستانکار', 'مانده']}
                  rows={trial.accounts.map((row) => [
                    row.code,
                    row.name,
                    money(row.debit),
                    money(row.credit),
                    money(Math.abs(row.balance)),
                  ])}
                />
              </>
            )}
            {kind === 'profit' && profit && (
              <div className="grid grid-cols-2 gap-3 px-2 sm:grid-cols-3 xl:grid-cols-6">
                <Stat title="درآمد فروش" value={profit.revenue} />
                <Stat title="برگشت از فروش" value={profit.sales_returns} />
                <Stat title="درآمد خالص" value={profit.net_revenue} />
                <Stat title="بهای تمام‌شده" value={profit.cogs} />
                <Stat title="سود ناخالص" value={profit.gross_profit} />
                <Stat title="حاشیه سود" value={profit.gross_margin_percent} unit="٪" />
              </div>
            )}
            {kind === 'receivable' && (
              <Table
                headers={['مشتری', 'تعداد فاکتور', 'فروش', 'تسویه', 'مانده']}
                rows={receivable.map((row) => [
                  row.contact_name,
                  row.invoice_count,
                  money(row.invoiced),
                  money(row.settled),
                  money(row.remaining),
                ])}
              />
            )}
            {kind === 'payable' && (
              <Table
                headers={['تأمین‌کننده', 'تعداد فاکتور', 'خرید', 'تسویه', 'مانده']}
                rows={payable.map((row) => [
                  row.contact_name,
                  row.invoice_count,
                  money(row.invoiced),
                  money(row.settled),
                  money(row.remaining),
                ])}
              />
            )}
            {kind === 'cash' && cash && (
              <>
                <div className="mb-3 px-2">
                  <Stat title="نقدینگی کل" value={cash.total} />
                </div>
                <Table
                  headers={['حساب', 'نوع', 'مانده']}
                  rows={cash.accounts.map((row) => [
                    row.name,
                    row.account_type === 'bank'
                      ? 'بانک'
                      : row.account_type === 'cash'
                        ? 'صندوق'
                        : 'تنخواه',
                    money(row.balance),
                  ])}
                />
              </>
            )}
            {kind === 'journal' && (
              <Table
                headers={['تاریخ', 'شماره', 'شرح', 'کد حساب', 'حساب', 'بدهکار', 'بستانکار']}
                rows={journal.map((row) => [
                  row.date,
                  row.number,
                  row.description,
                  row.account_code,
                  row.account_name,
                  money(row.debit),
                  money(row.credit),
                ])}
              />
            )}
            {kind === 'balance' && balance && (
              <Table
                headers={['کد', 'حساب', 'ماهیت', 'مبلغ']}
                rows={balance.lines.map((row) => [
                  row.code,
                  row.name,
                  row.nature === 'debit' ? 'بدهکار' : 'بستانکار',
                  money(Math.abs(row.amount)),
                ])}
              />
            )}
            {kind === 'income' && income && (
              <Table
                headers={['کد', 'حساب', 'ماهیت', 'مبلغ']}
                rows={income.lines.map((row) => [
                  row.code,
                  row.name,
                  row.nature === 'debit' ? 'بدهکار' : 'بستانکار',
                  money(Math.abs(row.amount)),
                ])}
              />
            )}
            {(kind === 'agingReceivable' || kind === 'agingPayable') && (
              <Table
                headers={[
                  kind === 'agingReceivable' ? 'مشتری' : 'تأمین‌کننده',
                  'سررسید نشده',
                  '۱ تا ۳۰',
                  '۳۱ تا ۶۰',
                  '۶۱ تا ۹۰',
                  'بیش از ۹۰',
                  'جمع',
                ]}
                rows={(kind === 'agingReceivable' ? agingR : agingP).map((row) => [
                  row.contact_name,
                  money(row.current),
                  money(row.days_1_30),
                  money(row.days_31_60),
                  money(row.days_61_90),
                  money(row.over_90),
                  money(row.total),
                ])}
              />
            )}
          </div>
        )}
      </Card>
    </section>
  )
}
