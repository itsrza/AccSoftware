import {useEffect, useMemo, useRef, useState} from 'react'
import {Dashboard} from './pages/Dashboard'
import {Invoices} from './pages/Invoices'
import {DataPage} from './pages/DataPage'
import {Reports} from './pages/Reports'
import {ReportBuilder} from './pages/ReportBuilder'
import {Integrations} from './pages/Integrations'
import {Treasury} from './pages/Treasury'
import {TreasuryDocumentForm} from './pages/TreasuryDocumentForm'
import {TreasuryAccounts} from './pages/TreasuryAccounts'
import {ChartOfAccounts} from './pages/ChartOfAccounts'
import {Returns} from './pages/Returns'
import {InventoryTransfer} from './pages/InventoryTransfer'
import {Quotes} from './pages/Quotes'
import {Production} from './pages/Production'
import {Checks} from './pages/Checks'
import {Icon} from './components/Icon'
import {CommandPalette} from './components/CommandPalette'
import {SettingsCenter} from './components/SettingsCenter'
import {Operations} from './pages/Operations'
import {AdvancedInventory} from './pages/AdvancedInventory'
import {DataTools} from './pages/DataTools'
import {PrintTemplates} from './pages/PrintTemplates'
import {SingleLineJournal} from './pages/SingleLineJournal'
import {ProductPricing} from './pages/ProductPricing'
import {Parties} from './pages/Parties'
import {InvoiceForm} from './pages/InvoiceForm'
import {Stocktaking} from './pages/Stocktaking'
import {getDemoStatus, deleteDemo, login, getParties, getProducts, getCheckDashboard, getChecks} from './api'
import {Plus} from 'lucide-react'
import {cn} from './lib/cn'
import {Sidebar, ICONS, type NavGroup, type NavItem} from './components/Sidebar'
import {Topbar, type NotificationItem, type SearchHit} from './components/Topbar'
import {isDesignPreview} from './lib/devPreview'
import {errorText} from './lib/errors'
import './styles.css'
import './theme.css'

/** یک آیتم منو؛ `page` صفحه‌ای است که با کلیک روی خود آیتم باز می‌شود. */
type MenuItem = {
  id: string
  label: string
  icon: string
  /** صفحه‌ی پیش‌فرض هنگام کلیک روی عنوان (نه فلش) */
  page: string
  children?: {label: string; page: string}[]
}

/** گروه‌بندی منو مطابق ساختار منوی نرم‌افزار فعلی نوین پرداز. */
const MENU: {title: string; items: MenuItem[]}[] = [
  {
    title: 'اصلی',
    items: [{id: 'dashboard', label: 'داشبورد', icon: 'grid', page: 'dashboard'}],
  },
  {
    title: 'عملیات',
    items: [
      {
        id: 'sales',
        label: 'فروش',
        icon: 'receipt',
        page: 'invoice-form',
        children: [
          {label: 'صدور فاکتور فروش', page: 'invoice-form'},
          {label: 'فاکتورهای فروش', page: 'sales'},
          {label: 'برگشت از فروش', page: 'sales-return'},
          {label: 'پیش‌فاکتورها', page: 'proforma'},
        ],
      },
      {
        id: 'purchase',
        label: 'خرید',
        icon: 'cart',
        page: 'purchase',
        children: [
          {label: 'فاکتورهای خرید', page: 'purchase'},
          {label: 'برگشت از خرید', page: 'purchase-return'},
          {label: 'سفارش خرید', page: 'purchase-order'},
        ],
      },
      {
        id: 'inventory',
        label: 'انبار و کالا',
        icon: 'package',
        page: 'inventory',
        children: [
          {label: 'کالاها', page: 'products'},
          {label: 'قیمت کالاها', page: 'product-pricing'},
          {label: 'موجودی انبار', page: 'inventory'},
          {label: 'انتقال بین انبارها', page: 'inventory-transfer'},
          {label: 'انبارگردانی', page: 'inventory-count'},
          {label: 'تولید', page: 'production'},
        ],
      },
      {
        id: 'treasury',
        label: 'خزانه',
        icon: 'wallet',
        page: 'treasury-document',
        children: [
          {label: 'سند دریافت و پرداخت', page: 'treasury-document'},
          {label: 'گردش خزانه', page: 'treasury'},
          {label: 'بانک‌ها', page: 'banks'},
          {label: 'صندوق‌ها', page: 'cashboxes'},
        ],
      },
      {id: 'checks', label: 'چک‌ها', icon: 'check', page: 'checks'},
    ],
  },
  {
    title: 'حسابداری',
    items: [
      {
        id: 'accounting',
        label: 'اسناد حسابداری',
        icon: 'file',
        page: 'accounting',
        children: [
          {label: 'سند یک‌سطری', page: 'single-journal'},
          {label: 'اسناد حسابداری', page: 'accounting'},
          {label: 'کدینگ حساب‌ها', page: 'chart-of-accounts'},
        ],
      },
      {id: 'parties', label: 'اشخاص', icon: 'users', page: 'parties'},
    ],
  },
  {
    title: 'گزارش و ابزار',
    items: [
      {
        id: 'reports',
        label: 'گزارشات',
        icon: 'bar',
        page: 'reports',
        children: [
          {label: 'مرکز گزارشات', page: 'reports'},
          {label: 'گزارش‌ساز', page: 'report-builder'},
        ],
      },
      {id: 'data-tools', label: 'ورود و خروج اطلاعات', icon: 'file', page: 'data-tools'},
      {id: 'print-templates', label: 'قالب‌های چاپ', icon: 'file', page: 'print-templates'},
      {id: 'integrations', label: 'اتصالات و افزونه‌ها', icon: 'settings', page: 'integrations'},
    ],
  },
]

const PAGE_TITLES: Record<string, string> = {
  dashboard: 'داشبورد',
  'invoice-form': 'صدور فاکتور فروش',
  sales: 'فاکتورهای فروش',
  production: 'تولید',
  'sales-return': 'برگشت از فروش',
  proforma: 'پیش‌فاکتورها',
  purchase: 'فاکتورهای خرید',
  'purchase-return': 'برگشت از خرید',
  'purchase-order': 'سفارش خرید',
  products: 'کالاها',
  'product-pricing': 'قیمت کالاها',
  inventory: 'موجودی انبار',
  'inventory-transfer': 'انتقال بین انبارها',
  'inventory-count': 'انبارگردانی',
  treasury: 'خزانه',
  banks: 'بانک‌ها',
  cashboxes: 'صندوق‌ها',
  checks: 'چک‌ها',
  accounting: 'اسناد حسابداری',
  'single-journal': 'سند یک‌سطری',
  'chart-of-accounts': 'کدینگ حساب‌ها',
  parties: 'اشخاص',
  reports: 'مرکز گزارشات',
  'report-builder': 'گزارش‌ساز',
  'data-tools': 'ورود و خروج اطلاعات',
  'print-templates': 'قالب‌های چاپ',
  integrations: 'اتصالات و افزونه‌ها',
}

/** عملیات سریع دکمه‌ی شناور — در تنظیمات قابل تغییر خواهد بود. */
const QUICK_ACTIONS = [
  {page: 'invoice-form', label: 'ثبت فاکتور فروش', icon: 'receipt'},
  {page: 'purchase', label: 'ثبت فاکتور خرید', icon: 'cart'},
  {page: 'products', label: 'ثبت کالای جدید', icon: 'package'},
  {page: 'parties', label: 'ثبت شخص جدید', icon: 'users'},
  {page: 'single-journal', label: 'ثبت سند حسابداری', icon: 'file'},
  {page: 'treasury-document', label: 'ثبت سند دریافت', icon: 'wallet'},
]

/** بستن منوی بازشو با کلیک بیرون از آن. */
function useOutsideClose(onClose: () => void) {
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const handler = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose()
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [onClose])
  return ref
}

export default function App() {
  const [page, setPage] = useState('dashboard')
  // همه‌ی منوها هنگام باز شدن برنامه بسته‌اند.
  const [expanded, setExpanded] = useState<string[]>([])
  const [dark, setDark] = useState(false)
  const [collapsed, setCollapsed] = useState(false)
  const [mobileNav, setMobileNav] = useState(false)
  // داده‌ی سبک برای جستجوی سراسری و اعلان‌ها؛ یک بار خوانده می‌شود.
  const [directory, setDirectory] = useState<{
    parties: {id: string; name: string}[]
    products: {id: string; name: string}[]
    checks: {number: string; party: string; due: string}[]
    lowStock: number
    overdueChecks: number
    dueSoonChecks: number
    unpaidInvoices: number
  }>({parties: [], products: [], checks: [], lowStock: 0, overdueChecks: 0, dueSoonChecks: 0, unpaidInvoices: 0})
  const [settings, setSettings] = useState(false)
  const [palette, setPalette] = useState(false)
  const [openMenu, setOpenMenu] = useState<'' | 'bell' | 'profile' | 'company' | 'fab'>('')

  const DEMO_BUILD = import.meta.env.VITE_DEMO_MODE === 'true'
  const [demo, setDemo] = useState(false)
  const [demoBusy, setDemoBusy] = useState(false)
  const [booting, setBooting] = useState(true)
  const [bootError, setBootError] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    const boot = async () => {
      try {
        if (DEMO_BUILD && !isDesignPreview()) {
          await login('admin', 'demo')
          const status = await getDemoStatus()
          if (alive) setDemo(status)
        }
      } catch (e) {
        if (alive) setBootError(errorText(e))
      } finally {
        if (alive) setBooting(false)
      }
    }
    boot()
    return () => {
      alive = false
    }
  }, [DEMO_BUILD])

  // جستجوی سراسری و اعلان‌ها روی داده‌ی واقعی کار می‌کنند، نه فهرست ثابت.
  useEffect(() => {
    if (booting || bootError) return
    let alive = true
    ;(async () => {
      try {
        const [parties, products, checkDashboard, checks] = await Promise.all([
          getParties().catch(() => ({rows: [] as {id: string; display_name: string}[]})),
          getProducts().catch(() => [] as {id: string; name: string}[]),
          getCheckDashboard().catch(() => null),
          getChecks().catch(() => [] as {check_number: string; bank_name?: string; due_date: string}[]),
        ])
        if (!alive) return
        setDirectory({
          parties: (parties.rows ?? []).map((row) => ({id: row.id, name: row.display_name})),
          products: (products ?? []).map((row) => ({id: row.id, name: row.name})),
          checks: (checks ?? []).map((row) => ({
            number: row.check_number,
            party: row.bank_name ?? '',
            due: row.due_date,
          })),
          lowStock: 0,
          overdueChecks: checkDashboard?.overdue_count ?? 0,
          dueSoonChecks: checkDashboard?.due_soon_count ?? 0,
          unpaidInvoices: 0,
        })
      } catch {
        /* اعلان‌ها اختیاری‌اند؛ نبودشان نباید برنامه را از کار بیندازد */
      }
    })()
    return () => {
      alive = false
    }
  }, [booting, bootError])

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setPalette(true)
      }
      if (event.key === 'Escape') {
        setPalette(false)
        setSettings(false)
        setOpenMenu('')
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])

  const closeMenus = useMemo(() => () => setOpenMenu(''), [])
  const bellRef = useOutsideClose(closeMenus)
  const profileRef = useOutsideClose(closeMenus)
  const companyRef = useOutsideClose(closeMenus)
  const fabRef = useOutsideClose(closeMenus)

  const go = (target: string) => {
    setPage(target)
    setOpenMenu('')
  }
  const toggleExpand = (id: string) =>
    setExpanded((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    )

  if (booting) {
    return (
      <div className="boot-screen" dir="rtl">
        <div className="boot-card">
          <strong>حسابداری نوین پرداز</strong>
          <span>در حال آماده‌سازی…</span>
        </div>
      </div>
    )
  }
  if (bootError) {
    return (
      <div className="boot-screen" dir="rtl">
        <div className="boot-card error">
          <strong>اجرای برنامه انجام نشد</strong>
          <span>{bootError}</span>
        </div>
      </div>
    )
  }

  const renderPage = () => {
    switch (page) {
      case 'dashboard':
        return <Dashboard demo={demo} onSettings={() => setSettings(true)} onNavigate={setPage} />
      case 'invoice-form':
        return <InvoiceForm />
      case 'parties':
        return <Parties />
      case 'product-pricing':
        return <ProductPricing />
      case 'single-journal':
        return <SingleLineJournal />
      case 'products':
        return <DataPage kind="products" />
      case 'inventory':
        return <AdvancedInventory />
      case 'inventory-count':
        return <Stocktaking />
      case 'production':
        return <Production />
      case 'proforma':
        return <Quotes kind="sales_quote" />
      case 'purchase-order':
        return <Quotes kind="purchase_order" />
      case 'inventory-transfer':
        return <InventoryTransfer />
      case 'sales-return':
        return <Returns sale />
      case 'purchase-return':
        return <Returns sale={false} />
      case 'chart-of-accounts':
        return <ChartOfAccounts />
      case 'accounting':
        return <Operations mode="accounting" />
      case 'reports':
        return <Reports />
      case 'report-builder':
        return <ReportBuilder />
      case 'integrations':
        return <Integrations />
      case 'data-tools':
        return <DataTools />
      case 'print-templates':
        return <PrintTemplates />
      case 'treasury-document':
        return <TreasuryDocumentForm />
      case 'banks':
        return <TreasuryAccounts mode="bank" />
      case 'cashboxes':
        return <TreasuryAccounts mode="cash" />
      case 'treasury':
        return <Treasury />
      case 'checks':
        return <Checks />
      default:
        return <Invoices page={page} />
    }
  }

  /** جستجوی سراسری روی اشخاص، کالاها و شماره چک‌های واقعی. */
  const globalSearch = (query: string): SearchHit[] => {
    const hits: SearchHit[] = []
    for (const party of directory.parties) {
      if (hits.length >= 12) break
      if (party.name.includes(query)) hits.push({title: party.name, meta: 'شخص', page: 'parties'})
    }
    for (const product of directory.products) {
      if (hits.length >= 12) break
      if (product.name.includes(query)) hits.push({title: product.name, meta: 'کالا', page: 'products'})
    }
    for (const check of directory.checks) {
      if (hits.length >= 12) break
      if (check.number.includes(query))
        hits.push({title: `چک ${check.number}`, meta: `سررسید ${check.due}`, page: 'checks'})
    }
    return hits
  }

  const checkAlerts = directory.overdueChecks + directory.dueSoonChecks

  /** اعلان‌ها فقط از وضعیت واقعی داده ساخته می‌شوند. */
  const notifications: NotificationItem[] = []
  if (directory.overdueChecks > 0) {
    notifications.push({
      id: 'overdue-checks',
      title: `${directory.overdueChecks.toLocaleString('fa-IR')} چک سررسید گذشته`,
      meta: 'هنوز تعیین تکلیف نشده‌اند — وصول یا برگشت را ثبت کنید.',
      tone: 'danger',
      page: 'checks',
    })
  }
  if (directory.dueSoonChecks > 0) {
    notifications.push({
      id: 'due-soon-checks',
      title: `${directory.dueSoonChecks.toLocaleString('fa-IR')} چک نزدیک سررسید`,
      meta: 'بازه‌ی هشدار در مرکز تنظیمات قابل تغییر است.',
      tone: 'warning',
      page: 'checks',
    })
  }

  // ---- گروه‌بندی منو برای اجزای تازه ----
  const navGroups: NavGroup[] = MENU.filter((group) => group.title !== 'گزارش و ابزار').map(
    (group) => ({
      title: group.title,
      items: group.items.map((item) => ({
        id: item.id,
        label: item.label,
        icon: ICONS[item.icon] ?? ICONS.file,
        page: item.page,
        children: item.children,
        badge: item.id === 'checks' ? checkAlerts : undefined,
      })),
    }),
  )
  const bottomNav: NavItem[] = (MENU.find((group) => group.title === 'گزارش و ابزار')?.items ?? []).map(
    (item) => ({
      id: item.id,
      label: item.label,
      icon: ICONS[item.icon] ?? ICONS.file,
      page: item.page,
      children: item.children,
    }),
  )

  const breadcrumb =
    MENU.find((group) => group.items.some((item) => item.page === page || item.children?.some((child) => child.page === page)))
      ?.title ?? 'نوین پرداز'

  return (
    <div className={cn('min-h-screen', dark && 'dark')} dir="rtl">
      <Sidebar
        groups={navGroups}
        bottom={bottomNav}
        page={page}
        navigate={go}
        collapsed={collapsed}
        toggleCollapsed={() => setCollapsed((value) => !value)}
        mobileOpen={mobileNav}
        setMobileOpen={setMobileNav}
        companyName="شرکت نوین پرداز"
        fiscalYear="۱۴۰۵"
        userName="مدیر سیستم"
        userRole="دسترسی کامل"
      />

      <div
        className={cn(
          'min-h-screen transition-[padding] duration-300',
          collapsed ? 'lg:ps-[84px]' : 'lg:ps-[272px]',
        )}
      >
        <Topbar
          title={PAGE_TITLES[page] ?? 'نوین پرداز'}
          breadcrumb={breadcrumb}
          dark={dark}
          setDark={setDark}
          onOpenMobileNav={() => setMobileNav(true)}
          onOpenSettings={() => setSettings(true)}
          onLogout={() => setPalette(true)}
          search={globalSearch}
          navigate={go}
          notifications={notifications}
          quickActions={QUICK_ACTIONS.map((action) => ({label: action.label, page: action.page}))}
          userName="مدیر سیستم"
          userRole="دسترسی کامل"
        />

        <main className="px-4 pt-5 pb-24 sm:px-6">{renderPage()}</main>
      </div>

      {/* دکمه‌ی شناور ایجاد سریع — پایین سمت چپ، دور از منو */}
      <div className="fixed bottom-6 start-6 z-40" ref={fabRef}>
        {openMenu === 'fab' && (
          <div className="fade-up mb-3 w-56 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
            <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">افزودن سریع</p>
            {QUICK_ACTIONS.map((action) => (
              <button
                key={action.page}
                onClick={() => go(action.page)}
                className="block w-full rounded-lg px-2.5 py-2 text-start text-xs text-muted transition-colors hover:bg-bg-soft hover:text-text"
              >
                {action.label}
              </button>
            ))}
          </div>
        )}
        <button
          aria-label="افزودن سریع"
          onClick={() => setOpenMenu(openMenu === 'fab' ? '' : 'fab')}
          className={cn(
            'grid size-14 place-items-center rounded-2xl bg-gradient-to-br from-[#e7bd75] to-[#c8923c] text-[#21254E] shadow-[0_12px_28px_-8px_rgba(220,167,87,.6)] transition-transform hover:scale-105 active:scale-95',
            openMenu === 'fab' && 'rotate-45',
          )}
        >
          <Plus className="size-6" aria-hidden />
        </button>
        {DEMO_BUILD && demo && (
          <button
            className="mt-3 block w-full rounded-xl border border-[var(--danger)] bg-[var(--danger-soft)] px-3 py-2 text-[11px] font-semibold text-danger transition-colors hover:bg-card"
            disabled={demoBusy}
            onClick={async () => {
              if (!confirm('تمام داده‌های نمونه حذف می‌شوند. ادامه می‌دهید؟')) return
              setDemoBusy(true)
              try {
                await deleteDemo()
                setDemo(false)
              } finally {
                setDemoBusy(false)
              }
            }}
          >
            {demoBusy ? 'در حال حذف…' : 'حذف داده‌ی نمونه'}
          </button>
        )}
      </div>

      <CommandPalette open={palette} onClose={() => setPalette(false)} onSelect={go} />
      {settings && (
        <SettingsCenter onClose={() => setSettings(false)} dark={dark} setDark={setDark} />
      )}
    </div>
  )
}
