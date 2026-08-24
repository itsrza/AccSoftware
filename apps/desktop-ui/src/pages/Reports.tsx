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
import { formatRials as money, percentSign, rialUnit } from '../lib/format'
import { useI18n, type TranslationKey } from '../lib/i18n'
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

const GROUPS: { labelKey: TranslationKey; items: [Kind, TranslationKey][] }[] = [
  {
    labelKey: 'reports.group.salesPurchase',
    items: [
      ['sales', 'reports.salesReport'],
      ['purchase', 'reports.purchaseReport'],
    ],
  },
  {
    labelKey: 'reports.group.inventory',
    items: [['inventory', 'reports.inventoryValue']],
  },
  {
    labelKey: 'reports.group.statutory',
    items: [
      ['journal', 'reports.journal'],
      ['ledger', 'reports.ledger'],
      ['trial', 'reports.trialBalance'],
    ],
  },
  {
    labelKey: 'reports.group.statements',
    items: [
      ['balance', 'reports.balanceSheet'],
      ['income', 'reports.incomeStatement'],
      ['profit', 'reports.grossProfitSummary'],
    ],
  },
  {
    labelKey: 'reports.group.parties',
    items: [
      ['receivable', 'reports.receivables'],
      ['payable', 'reports.payables'],
      ['agingReceivable', 'reports.receivableAging'],
      ['agingPayable', 'reports.payableAging'],
    ],
  },
  {
    labelKey: 'reports.group.treasury',
    items: [['cash', 'reports.liquidity']],
  },
]

const TITLE_KEY: Record<Kind, TranslationKey> = Object.fromEntries(
  GROUPS.flatMap((group) => group.items),
) as Record<Kind, TranslationKey>

/** گزارش‌هایی که بازه‌ی زمانی روی آن‌ها اثر دارد. بقیه «در لحظه»اند. */
const PERIODIC: Kind[] = ['sales', 'purchase', 'ledger', 'journal']
/** گزارش‌هایی که فقط «تا تاریخ» می‌گیرند. */
const AS_OF: Kind[] = ['balance', 'income', 'agingReceivable', 'agingPayable']

function Table({ headers, rows }: { headers: string[]; rows: (string | number)[][] }) {
  const { t } = useI18n()
  if (rows.length === 0) {
    return <EmptyState title={t('reports.empty')} hint={t('reports.emptyHint')} />
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

function Stat({ title, value, unit }: { title: string; value: number; unit?: string }) {
  const label = unit ?? rialUnit()
  return (
    <article
      data-card
      className="rounded-[var(--radius)] border border-border bg-card p-3.5 shadow-[var(--shadow-sm)]"
    >
      <p className="text-[11px] font-semibold text-muted">{title}</p>
      <p className="tnum mt-1.5 truncate text-[17px] font-extrabold text-text">
        {money(value)}
        <span className="ms-1 text-[10px] font-semibold text-faint">{label}</span>
      </p>
    </article>
  )
}

export function Reports() {
  const { t } = useI18n()
  /** برچسب وضعیت تسویه در سطرهای گزارش. */
  const paymentLabel = (value: string) =>
    value === 'paid'
      ? t('invoices.settled')
      : value === 'partial'
        ? t('invoices.partial')
        : t('invoices.unsettled')
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
    if (PERIODIC.includes(kind)) return t('reports.rangeNote', { from: range.from, to: range.to })
    if (AS_OF.includes(kind)) return t('reports.asOf', { date: range.to })
    return t('reports.liveBalance')
  }, [kind, range, t])

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('reports.eyebrow')}</div>
          <h1>{t('reports.title')}</h1>
          <p>{t('reports.subtitle')}</p>
        </div>
        <button className="ghost" onClick={() => void load()}>
          <RefreshCw className="size-4" aria-hidden /> {t('common.refresh')}
        </button>
      </div>

      <FilterBar
        range={range}
        onRange={setRange}
        fiscalRange={fiscalRange}
        filters={[
          {
            key: 'kind',
            label: t('reports.kind'),
            value: kind,
            width: 'xl:w-64',
            onChange: (value) => setKind(value as Kind),
            options: GROUPS.flatMap((group) =>
              group.items.map(([value, labelKey]) => ({
                value,
                label: `${t(group.labelKey)} — ${t(labelKey)}`,
              })),
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
          <CardHeader title={t(TITLE_KEY[kind])} subtitle={scope} />
        </div>

        {loading ? (
          <div className="px-4 pb-5 sm:px-5">
            <Skeleton className="h-64 w-full" />
          </div>
        ) : (
          <div className="px-2 pb-4 sm:px-3">
            {kind === 'sales' && (
              <Table
                headers={[
                  t('common.date'),
                  t('common.number'),
                  t('reports.customer'),
                  t('common.net'),
                  t('common.tax'),
                  t('common.amount'),
                  t('invoices.settlementShort'),
                ]}
                rows={sales.map((row) => [
                  row.date,
                  row.invoice_number,
                  row.contact_name || t('reports.noParty'),
                  money(row.subtotal - row.discount),
                  money(row.tax),
                  money(row.total),
                  paymentLabel(row.payment_status),
                ])}
              />
            )}
            {kind === 'purchase' && (
              <Table
                headers={[
                  t('common.date'),
                  t('common.number'),
                  t('reports.supplier'),
                  t('common.net'),
                  t('common.tax'),
                  t('common.amount'),
                  t('invoices.settlementShort'),
                ]}
                rows={purchase.map((row) => [
                  row.date,
                  row.invoice_number,
                  row.contact_name || t('reports.noParty'),
                  money(row.subtotal - row.discount),
                  money(row.tax),
                  money(row.total),
                  paymentLabel(row.payment_status),
                ])}
              />
            )}
            {kind === 'inventory' && (
              <Table
                headers={[
                  t('invoiceForm.product'),
                  t('common.warehouse'),
                  t('products.stock'),
                  t('reports.averageCost'),
                  t('reports.value'),
                ]}
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
                headers={[
                  t('common.code'),
                  t('reports.account'),
                  t('reports.debit'),
                  t('reports.credit'),
                  t('reports.balance'),
                ]}
                rows={ledger.map((row) => [
                  row.code,
                  row.name,
                  money(row.debit),
                  money(row.credit),
                  `${money(Math.abs(row.balance))} ${
                    row.balance >= 0 ? t('reports.debit') : t('reports.credit')
                  }`,
                ])}
              />
            )}
            {kind === 'trial' && trial && (
              <>
                <div className="mb-3 grid grid-cols-1 gap-3 px-2 sm:grid-cols-3">
                  <Stat title={t('reports.totalDebit')} value={trial.total_debit} />
                  <Stat title={t('reports.totalCredit')} value={trial.total_credit} />
                  <article
                    data-card
                    className="rounded-[var(--radius)] border border-border bg-card p-3.5"
                  >
                    <p className="text-[11px] font-semibold text-muted">{t('reports.balanceCheck')}</p>
                    <p
                      className={`mt-1.5 text-[17px] font-extrabold ${
                        trial.total_debit === trial.total_credit ? 'text-success' : 'text-danger'
                      }`}
                    >
                      {trial.total_debit === trial.total_credit
                        ? t('reports.balanced')
                        : t('reports.unbalanced')}
                    </p>
                  </article>
                </div>
                <Table
                  headers={[
                  t('common.code'),
                  t('reports.account'),
                  t('reports.debit'),
                  t('reports.credit'),
                  t('reports.balance'),
                ]}
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
                <Stat title={t('reports.salesRevenue')} value={profit.revenue} />
                <Stat title={t('reports.salesReturns')} value={profit.sales_returns} />
                <Stat title={t('reports.netRevenue')} value={profit.net_revenue} />
                <Stat title={t('reports.cogs')} value={profit.cogs} />
                <Stat title={t('reports.grossProfit')} value={profit.gross_profit} />
                <Stat
                  title={t('reports.margin')}
                  value={profit.gross_margin_percent}
                  unit={percentSign()}
                />
              </div>
            )}
            {kind === 'receivable' && (
              <Table
                headers={[
                  t('reports.customer'),
                  t('reports.invoiceCount'),
                  t('dashboard.chart.sales'),
                  t('invoices.settlementShort'),
                  t('reports.balance'),
                ]}
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
                headers={[
                  t('reports.supplier'),
                  t('reports.invoiceCount'),
                  t('dashboard.chart.purchases'),
                  t('invoices.settlementShort'),
                  t('reports.balance'),
                ]}
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
                  <Stat title={t('reports.totalLiquidity')} value={cash.total} />
                </div>
                <Table
                  headers={[t('reports.account'), t('common.type'), t('reports.balance')]}
                  rows={cash.accounts.map((row) => [
                    row.name,
                    row.account_type === 'bank'
                      ? t('reports.bank')
                      : row.account_type === 'cash'
                        ? t('reports.cashbox')
                        : t('reports.pettyCash'),
                    money(row.balance),
                  ])}
                />
              </>
            )}
            {kind === 'journal' && (
              <Table
                headers={[
                  t('common.date'),
                  t('common.number'),
                  t('common.description'),
                  t('reports.accountCode'),
                  t('reports.account'),
                  t('reports.debit'),
                  t('reports.credit'),
                ]}
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
                headers={[
                  t('common.code'),
                  t('reports.account'),
                  t('reports.nature'),
                  t('common.amount'),
                ]}
                rows={balance.lines.map((row) => [
                  row.code,
                  row.name,
                  row.nature === 'debit' ? t('reports.debit') : t('reports.credit'),
                  money(Math.abs(row.amount)),
                ])}
              />
            )}
            {kind === 'income' && income && (
              <Table
                headers={[
                  t('common.code'),
                  t('reports.account'),
                  t('reports.nature'),
                  t('common.amount'),
                ]}
                rows={income.lines.map((row) => [
                  row.code,
                  row.name,
                  row.nature === 'debit' ? t('reports.debit') : t('reports.credit'),
                  money(Math.abs(row.amount)),
                ])}
              />
            )}
            {(kind === 'agingReceivable' || kind === 'agingPayable') && (
              <Table
                headers={[
                  kind === 'agingReceivable' ? t('reports.customer') : t('reports.supplier'),
                  t('reports.bucket.current'),
                  t('reports.bucket.1_30'),
                  t('reports.bucket.31_60'),
                  t('reports.bucket.61_90'),
                  t('reports.bucket.over90'),
                  t('reports.total'),
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
