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
import {Products} from './pages/Products'
import {ProductCardex} from './pages/ProductCardex'
import {AuditLog} from './pages/AuditLog'
import {VisualAnalytics} from './pages/VisualAnalytics'
import {InvoiceForm} from './pages/InvoiceForm'
import {Stocktaking} from './pages/Stocktaking'
import {getDemoStatus, deleteDemo, login, logout, getParties, getProducts, getCheckDashboard, getChecks, getSettings} from './api'
import './security-hardening.css'
import {Plus} from 'lucide-react'
import {cn} from './lib/cn'
import {Sidebar, ICONS, type NavGroup, type NavItem} from './components/Sidebar'
import {Topbar, type NotificationItem, type SearchHit} from './components/Topbar'
import {isDesignPreview} from './lib/devPreview'
import {errorText} from './lib/errors'
import {useI18n, type TranslationKey} from './lib/i18n'
import {shortcutTarget, isTypingTarget} from './lib/shortcuts'
import {formatCount} from './lib/format'
// styles.css و theme.css از داخل design-system.css و در لایه‌ی `legacy`
// بارگذاری می‌شوند تا کلاس‌های تِیلویند بتوانند بر آن‌ها مقدم شوند.

/** یک آیتم منو؛ `page` صفحه‌ای است که با کلیک روی خود آیتم باز می‌شود. */
type MenuItem = {
  id: string
  /** کلید ترجمه‌ی عنوان — متن هرگز مستقیم در ساختار منو نوشته نمی‌شود. */
  labelKey: TranslationKey
  icon: string
  /** صفحه‌ی پیش‌فرض هنگام کلیک روی عنوان (نه فلش) */
  page: string
  /** زیرمنوها فقط شناسه‌ی صفحه‌اند؛ عنوانشان از `page.<id>` می‌آید. */
  children?: string[]
}

/** گروه‌بندی منو مطابق ساختار منوی نرم‌افزار فعلی نوین پرداز. */
const MENU: {id: string; titleKey: TranslationKey; items: MenuItem[]}[] = [
  {
    id: 'main',
    titleKey: 'nav.group.main',
    items: [{id: 'dashboard', labelKey: 'page.dashboard', icon: 'grid', page: 'dashboard'}],
  },
  {
    id: 'operations',
    titleKey: 'nav.group.operations',
    items: [
      {
        id: 'sales',
        labelKey: 'nav.sales',
        icon: 'receipt',
        page: 'invoice-form',
        children: ['invoice-form', 'sales', 'sales-return', 'proforma'],
      },
      {
        id: 'purchase',
        labelKey: 'nav.purchase',
        icon: 'cart',
        page: 'purchase',
        children: ['purchase', 'purchase-return', 'purchase-order'],
      },
      {
        id: 'inventory',
        labelKey: 'nav.inventory',
        icon: 'package',
        page: 'inventory',
        children: [
          'products',
          'product-pricing',
          'inventory',
          'inventory-transfer',
          'inventory-count',
          'production',
        ],
      },
      {
        id: 'treasury',
        labelKey: 'nav.treasury',
        icon: 'wallet',
        page: 'treasury-document',
        children: ['treasury-document', 'treasury', 'banks', 'cashboxes'],
      },
      {id: 'checks', labelKey: 'nav.checks', icon: 'check', page: 'checks'},
    ],
  },
  {
    id: 'accounting',
    titleKey: 'nav.group.accounting',
    items: [
      {
        id: 'accounting',
        labelKey: 'nav.journals',
        icon: 'file',
        page: 'accounting',
        children: ['single-journal', 'accounting', 'chart-of-accounts'],
      },
      {id: 'parties', labelKey: 'nav.parties', icon: 'users', page: 'parties'},
    ],
  },
  {
    id: 'tools',
    titleKey: 'nav.group.reports',
    items: [
      {
        id: 'reports',
        labelKey: 'nav.reports',
        icon: 'bar',
        page: 'reports',
        children: ['reports', 'report-builder', 'visual-analytics'],
      },
      {id: 'audit-log', labelKey: 'page.audit-log', icon: 'file', page: 'audit-log'},
      {id: 'data-tools', labelKey: 'page.data-tools', icon: 'upload', page: 'data-tools'},
      {id: 'settings', labelKey: 'page.settings', icon: 'settings', page: '__settings'},
    ],
  },
]

/* «قالب‌های چاپ» و «اتصالات و افزونه‌ها» عمداً در منوی کناری نیستند.
 *
 * هر دو ابزار «راه‌اندازی» هستند نه کار روزمره: یک‌بار تنظیم می‌شوند و
 * ماه‌ها دست نمی‌خورند. جای‌شان در منوی اصلی، ردیف‌های پرکاربرد را پایین
 * می‌راند. حالا از «مرکز تنظیمات ← ابزارهای پیشرفته» و از پالت فرمان
 * (Ctrl+K) در دسترس‌اند. */

/** عنوان صفحه از کلید `page.<id>` می‌آید؛ فهرست جداگانه‌ای لازم نیست. */
function pageTitleKey(page: string): TranslationKey {
  return (page === '__settings' ? 'page.settings' : `page.${page}`) as TranslationKey
}

/** عملیات سریع دکمه‌ی شناور. */
const QUICK_ACTIONS: {page: string; labelKey: TranslationKey; icon: string}[] = [
  {page: 'invoice-form', labelKey: 'fab.newSalesInvoice', icon: 'receipt'},
  {page: 'purchase', labelKey: 'fab.newPurchaseInvoice', icon: 'cart'},
  {page: 'products', labelKey: 'fab.newProduct', icon: 'package'},
  {page: 'parties', labelKey: 'fab.newParty', icon: 'users'},
  {page: 'single-journal', labelKey: 'fab.newJournal', icon: 'file'},
  {page: 'treasury-document', labelKey: 'fab.newReceipt', icon: 'wallet'},
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
  const {t, dir, locale, setLocale} = useI18n()
  const [page, setPage] = useState('dashboard')
  // کاردکس کالا (F4/F5/F6 مرجع) — با چه کالا و کانالی باز شود.
  const [cardexSeed, setCardexSeed] = useState<{
    productId?: string
    kind: 'sales' | 'purchase' | 'all'
  } | null>(null)
  // همه‌ی منوها هنگام باز شدن برنامه بسته‌اند.
  const [expanded, setExpanded] = useState<string[]>([])
  // تم پیش‌فرض تیره است — انتخاب کاربر در همین جلسه بر آن مقدم می‌شود.
  const [dark, setDark] = useState(true)
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
  // تصویر پروفایل از تنظیمات خوانده می‌شود؛ خالی یعنی نشان پیش‌فرض طلایی.
  const [avatar, setAvatar] = useState('')
  const [palette, setPalette] = useState(false)
  const [openMenu, setOpenMenu] = useState<'' | 'bell' | 'profile' | 'company' | 'fab'>('')

  // در پیش‌نمایش مرورگر هم داده‌ی نمونه باید دیده شود؛ وگرنه داشبورد و
  // همه‌ی فهرست‌ها «حالت خالی» نشان می‌دهند و چیزی برای بازبینی نمی‌ماند.
  const DEMO_BUILD = import.meta.env.VITE_DEMO_MODE === 'true' || isDesignPreview()
  const [demo, setDemo] = useState(false)
  const [demoBusy, setDemoBusy] = useState(false)
  const [booting, setBooting] = useState(true)
  const [bootError, setBootError] = useState<string | null>(null)
  // احراز هویت واقعی: در بیلد دمو ورود خودکار است، در بیلد تجاری دروازه‌ی ورود.
  const [authenticated, setAuthenticated] = useState(false)
  const [userName, setUserName] = useState('')
  const [authBusy, setAuthBusy] = useState(false)
  const [authError, setAuthError] = useState('')
  const loginUsernameRef = useRef<HTMLInputElement>(null)
  const loginPasswordRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    let alive = true
    const boot = async () => {
      try {
        if (isDesignPreview()) {
          if (alive) setDemo(true)
        } else if (DEMO_BUILD) {
          // حالت دمو: ورود خودکار کاربر نمایشی — فقط در بیلد صریح دمو.
          await login('admin', 'demo')
          const status = await getDemoStatus()
          if (alive) {
            setDemo(status)
            setAuthenticated(true)
          }
        }
      } catch {
        // خطای بوت هرگز جزئیات فنی به کاربر نشان نمی‌دهد.
        if (alive) setBootError('راه‌اندازی برنامه انجام نشد')
      } finally {
        if (alive) setBooting(false)
      }
    }
    boot()
    return () => {
      alive = false
    }
  }, [DEMO_BUILD])

  // تصویر پروفایل و تم اولیه از تنظیمات می‌آیند.
  useEffect(() => {
    if (booting || bootError) return
    let alive = true
    getSettings()
      .then((list) => {
        if (!alive) return
        setAvatar(list.find((item) => item.key === 'user.avatar')?.value ?? '')
        const stored = list.find((item) => item.key === 'appearance.dark_mode')
        if (stored?.is_customized) setDark(stored.value === 'true')
        // زبان ذخیره‌شده در پایگاه داده بر انتخاب موقتِ همین مرورگر مقدم است،
        // چون روی همه‌ی دستگاه‌های همان نصب اعمال می‌شود.
        const language = list.find((item) => item.key === 'appearance.language')
        if (language?.is_customized && language.value !== locale)
          setLocale(language.value as typeof locale)
      })
      .catch(() => {
        /* نبود تنظیمات نباید پوسته را از کار بیندازد */
      })
    return () => {
      alive = false
    }
    // `locale` عمداً در وابستگی‌ها نیست: این اثر فقط یک بار هنگام بالا آمدن
    // برنامه، زبانِ ذخیره‌شده را می‌خواند و نباید با تغییر دستی زبان دوباره
    // اجرا شود و انتخاب کاربر را برگرداند.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [booting, bootError])

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

  /* تم تیره روی ریشه‌ی سند اعمال می‌شود، نه روی یک div داخلی.
   *
   * چرا: `body{background:var(--bg)}` و `.dark body{...}` فقط وقتی کار
   * می‌کنند که کلاس `dark` بالادستِ `body` باشد. با گذاشتن کلاس روی یک
   * div داخلی، پس‌زمینه‌ی کل صفحه روشن می‌ماند و فقط کارت‌ها تیره می‌شوند —
   * همان چیزی که کاربر دید. `color-scheme` هم تنظیم می‌شود تا اسکرول‌بار،
   * نوار انتخاب متن و کنترل‌های بومی مرورگر با تم هماهنگ شوند. */
  useEffect(() => {
    const root = document.documentElement
    root.classList.toggle('dark', dark)
    root.style.colorScheme = dark ? 'dark' : 'light'
  }, [dark])

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
        return
      }
      // در صفحه‌ی کالاها، F4/F5/F6 همان کاردکس مرجع‌اند (تصویر 8Xmc1p)؛
      // در بقیه‌ی صفحه‌ها نگاشت سراسری (صندوق/بانک) برقرار می‌ماند.
      if (
        page === 'products' &&
        !isTypingTarget(event.target) &&
        (event.key === 'F4' || event.key === 'F5' || event.key === 'F6')
      ) {
        event.preventDefault()
        setSettings(false)
        setPalette(false)
        setOpenMenu('')
        setCardexSeed({
          productId: undefined,
          kind: event.key === 'F4' ? 'sales' : event.key === 'F5' ? 'purchase' : 'all',
        })
        setPage('product-cardex')
        return
      }
      // میانبرهای تک‌حرفی نوار کناری مرجع؛ حین تایپ در فرم غیرفعال‌اند.
      const target = shortcutTarget(event)
      if (target) {
        event.preventDefault()
        setSettings(false)
        setPalette(false)
        setOpenMenu('')
        setPage(target)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [page])

  const closeMenus = useMemo(() => () => setOpenMenu(''), [])
  const bellRef = useOutsideClose(closeMenus)
  const profileRef = useOutsideClose(closeMenus)
  const companyRef = useOutsideClose(closeMenus)
  const fabRef = useOutsideClose(closeMenus)

  const go = (target: string) => {
    // «مرکز تنظیمات» صفحه نیست؛ یک پوشش تمام‌صفحه است.
    if (target === '__settings') {
      setSettings(true)
      setOpenMenu('')
      return
    }
    setPage(target)
    setOpenMenu('')
  }

  /** باز کردن کاردکس از صفحه‌ی کالاها — با کالای از پیش انتخاب‌شده یا بدون آن. */
  const openCardex = (productId: string | undefined, kind: 'sales' | 'purchase' | 'all') => {
    setCardexSeed({productId, kind})
    setPage('product-cardex')
    setOpenMenu('')
  }
  const toggleExpand = (id: string) =>
    setExpanded((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    )

  const submitLogin = async (event: React.FormEvent) => {
    event.preventDefault()
    const username = String(loginUsernameRef.current?.value ?? '').trim()
    const password = String(loginPasswordRef.current?.value ?? '')
    if (!username || !password || authBusy) return
    setAuthBusy(true)
    setAuthError('')
    try {
      const user = await login(username.trim(),password)
      setUserName(user.display_name || user.username)
      setAuthenticated(true)
    } catch {
      // پیام عمومی — جزئیات باطن هرگز به کاربر نشان داده نمی‌شود.
      setAuthError('نام کاربری یا رمز عبور صحیح نیست')
    } finally {
      setAuthBusy(false)
    }
  }

  const doLogout = async () => {
    try {
      await logout()
    } catch {
      /* حتی اگر نشست منقضی شده باشد، خروج محلی انجام می‌شود */
    }
    setAuthenticated(false)
    setUserName('')
  }

  if (booting) {
    return (
      <div className="boot-screen" dir={dir}>
        <div className="boot-card">
          <strong>{t('app.name')}</strong>
          <span>{t('app.booting')}</span>
        </div>
      </div>
    )
  }
  if (bootError) {
    return (
      <div className="boot-screen" dir={dir}>
        <div className="boot-card error">
          <strong>{t('app.bootFailed')}</strong>
          <span>{bootError}</span>
        </div>
      </div>
    )
  }

  // دروازه‌ی احراز هویت: در بیلد تجاری، تا ورود موفق هیچ داده‌ای رندر نمی‌شود.
  if(!authenticated) {
    return (
      <div className="auth-screen" dir={dir}>
        <form className="auth-card" onSubmit={submitLogin}>
          <div className="auth-mark">NP</div>
          <div className="auth-title">{t('app.name')}</div>
          <div className="auth-subtitle">{t('app.loginSubtitle')}</div>
          {authError && <p className="error-box auth-error">{authError}</p>}
          <label>
            {t('app.username')}
            <input ref={loginUsernameRef} autoFocus autoComplete="username" />
          </label>
          <label>
            {t('app.password')}
            <input ref={loginPasswordRef} type="password" autoComplete="current-password" />
          </label>
          <button className="primary auth-submit" type="submit" disabled={authBusy}>
            {authBusy ? t('app.loggingIn') : t('app.loginAction')}
          </button>
        </form>
      </div>
    )
  }

  const renderPage = () => {
    switch (page) {
      case 'dashboard':
        return <Dashboard demo={demo} />
      case 'invoice-form':
        return <InvoiceForm />
      case 'parties':
        return <Parties />
      case 'product-pricing':
        return <ProductPricing />
      case 'single-journal':
        return <SingleLineJournal />
      case 'products':
        return <Products onCardex={openCardex} />
      case 'product-cardex':
        return <ProductCardex initial={cardexSeed ?? undefined} />
      case 'audit-log':
        return <AuditLog />
      case 'visual-analytics':
        return <VisualAnalytics />
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
        return <Invoices page={page} onNavigate={go} />
    }
  }

  /** جستجوی سراسری روی اشخاص، کالاها و شماره چک‌های واقعی. */
  const globalSearch = (query: string): SearchHit[] => {
    const hits: SearchHit[] = []
    for (const party of directory.parties) {
      if (hits.length >= 12) break
      if (party.name.includes(query))
        hits.push({title: party.name, meta: t('search.party'), page: 'parties'})
    }
    for (const product of directory.products) {
      if (hits.length >= 12) break
      if (product.name.includes(query))
        hits.push({title: product.name, meta: t('search.product'), page: 'products'})
    }
    for (const check of directory.checks) {
      if (hits.length >= 12) break
      if (check.number.includes(query))
        hits.push({
          title: t('search.check', {number: check.number}),
          meta: t('search.dueDate', {date: check.due}),
          page: 'checks',
        })
    }
    return hits
  }

  const checkAlerts = directory.overdueChecks + directory.dueSoonChecks

  /** اعلان‌ها فقط از وضعیت واقعی داده ساخته می‌شوند. */
  const notifications: NotificationItem[] = []
  if (directory.overdueChecks > 0) {
    notifications.push({
      id: 'overdue-checks',
      title: t('alert.overdueChecks', {count: formatCount(directory.overdueChecks)}),
      meta: t('alert.overdueChecksHint'),
      tone: 'danger',
      page: 'checks',
    })
  }
  if (directory.dueSoonChecks > 0) {
    notifications.push({
      id: 'due-soon-checks',
      title: t('alert.dueSoonChecks', {count: formatCount(directory.dueSoonChecks)}),
      meta: t('alert.dueSoonChecksHint'),
      tone: 'warning',
      page: 'checks',
    })
  }

  // ---- گروه‌بندی منو برای اجزای تازه ----
  const childItems = (pages: string[] | undefined) =>
    pages?.map((child) => ({label: t(pageTitleKey(child)), page: child}))
  const navGroups: NavGroup[] = MENU.filter((group) => group.id !== 'tools').map((group) => ({
    title: t(group.titleKey),
    items: group.items.map((item) => ({
      id: item.id,
      label: t(item.labelKey),
      icon: ICONS[item.icon] ?? ICONS.file,
      page: item.page,
      children: childItems(item.children),
      badge: item.id === 'checks' ? checkAlerts : undefined,
    })),
  }))
  const bottomNav: NavItem[] = (MENU.find((group) => group.id === 'tools')?.items ?? []).map(
    (item) => ({
      id: item.id,
      label: t(item.labelKey),
      icon: ICONS[item.icon] ?? ICONS.file,
      page: item.page,
      children: childItems(item.children),
    }),
  )

  const breadcrumbGroup = MENU.find((group) =>
    group.items.some((item) => item.page === page || item.children?.includes(page)),
  )
  const breadcrumb = breadcrumbGroup ? t(breadcrumbGroup.titleKey) : t('app.name')

  return (
    <div className="min-h-screen" dir={dir}>
      <Sidebar
        groups={navGroups}
        bottom={bottomNav}
        page={page}
        navigate={go}
        collapsed={collapsed}
        toggleCollapsed={() => setCollapsed((value) => !value)}
        mobileOpen={mobileNav}
        setMobileOpen={setMobileNav}
        companyName={t('app.company')}
        fiscalYear={formatCount(1405)}
        userName={t('app.systemAdmin')}
        userRole={t('app.fullAccess')}
        avatar={avatar}
        onOpenSettings={() => setSettings(true)}
      />

      <div
        className={cn(
          'min-h-screen transition-[padding] duration-300',
          collapsed ? 'lg:ps-[84px]' : 'lg:ps-[272px]',
        )}
      >
        <Topbar
          title={t(pageTitleKey(page))}
          breadcrumb={breadcrumb}
          dark={dark}
          setDark={setDark}
          onOpenMobileNav={() => setMobileNav(true)}
          onOpenSettings={() => setSettings(true)}
          onLogout={doLogout}
          search={globalSearch}
          navigate={go}
          notifications={notifications}
          userName={userName || t('app.systemAdmin')}
          userRole={t('app.fullAccess')}
          avatar={avatar}
        />

        <main className="px-4 pt-5 pb-24 sm:px-6">{renderPage()}</main>
      </div>

      {/* دکمه‌ی شناور ایجاد سریع — همیشه پایین سمت چپ صفحه.
        *
        * ## چرا این ساختار
        * نسخه‌ی قبلی، منوی بازشو و دکمه‌ی «حذف داده‌ی نمونه» را **هم‌سطح**
        * دکمه می‌گذاشت؛ چون عرض ظرف با عرض پهن‌ترین فرزندش تعیین می‌شود،
        * باز شدن منو ظرف را پهن می‌کرد و دکمه از جایش می‌پرید.
        *
        * حالا: منو `absolute` است (اصلاً در چیدمان اثر ندارد) و ستون
        * `items-end` است، پس هر فرزندی هر عرضی داشته باشد، لبه‌ی دکمه ثابت
        * می‌ماند. `end` در راست‌به‌چپ یعنی چپ، پس منوی کناری را نمی‌پوشاند. */}
      <div className="fixed bottom-6 end-6 z-40 flex flex-col items-end gap-3" ref={fabRef}>
        <div className="relative">
          {openMenu === 'fab' && (
            <div className="fade-up absolute bottom-[calc(100%+12px)] end-0 w-56 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
              <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">{t('fab.quickAdd')}</p>
              {QUICK_ACTIONS.map((action) => (
                <button
                  key={action.page}
                  onClick={() => go(action.page)}
                  className="block w-full rounded-lg px-2.5 py-2 text-start text-xs text-muted transition-colors hover:bg-bg-soft hover:text-text"
                >
                  {t(action.labelKey)}
                </button>
              ))}
            </div>
          )}
          <button
            aria-label={t('fab.quickAdd')}
            aria-expanded={openMenu === 'fab'}
            onClick={() => setOpenMenu(openMenu === 'fab' ? '' : 'fab')}
            className={cn(
              'fab-pulse grid size-14 place-items-center rounded-2xl bg-gradient-to-br from-[#e7bd75] to-[#c8923c] text-[#21254E] transition-transform hover:scale-105 active:scale-95',
              openMenu === 'fab' && 'rotate-45',
            )}
          >
            <Plus className="size-6" aria-hidden />
          </button>
        </div>
        {DEMO_BUILD && demo && (
          <button
            className="w-max whitespace-nowrap rounded-xl border border-[var(--danger)] bg-[var(--danger-soft)] px-3 py-2 text-[11px] font-semibold text-danger transition-colors hover:bg-card"
            disabled={demoBusy}
            onClick={async () => {
              if (!confirm(t('demo.deleteConfirm'))) return
              setDemoBusy(true)
              try {
                await deleteDemo()
                setDemo(false)
              } finally {
                setDemoBusy(false)
              }
            }}
          >
            {demoBusy ? t('demo.deleting') : t('demo.delete')}
          </button>
        )}
      </div>

      <CommandPalette open={palette} onClose={() => setPalette(false)} onSelect={go} />
      {settings && (
        <SettingsCenter
          onClose={() => setSettings(false)}
          dark={dark}
          setDark={setDark}
          navigate={(target) => {
            setSettings(false)
            go(target)
          }}
        />
      )}
    </div>
  )
}
