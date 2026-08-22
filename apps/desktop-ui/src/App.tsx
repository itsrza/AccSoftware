import {useEffect, useMemo, useRef, useState} from 'react'
import {Dashboard} from './pages/Dashboard'
import {Invoices} from './pages/Invoices'
import {DataPage} from './pages/DataPage'
import {Reports} from './pages/Reports'
import {ReportBuilder} from './pages/ReportBuilder'
import {Integrations} from './pages/Integrations'
import {Treasury} from './pages/Treasury'
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
import {UnderConstruction} from './components/UnderConstruction'
import {getDemoStatus, deleteDemo, login} from './api'
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
        ],
      },
      {
        id: 'treasury',
        label: 'خزانه',
        icon: 'wallet',
        page: 'treasury',
        children: [
          {label: 'دریافت و پرداخت', page: 'treasury'},
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
      case 'inventory-transfer':
        return (
          <UnderConstruction
            title="انتقال بین انبارها"
            description="فرم انتقال کالا بین انبارها با کنترل موجودی مبدأ، موجودی در راه و رسید انبار مقصد. موتور انتقال در هسته آماده است و فرم آن در فاز انبار ساخته می‌شود."
            reference="8Xmc1p"
            phase="فاز ۷ — انبار"
          />
        )
      case 'inventory-count':
        return (
          <UnderConstruction
            title="انبارگردانی"
            description="دوره‌ی انبارگردانی با فریز منطقی موجودی، شمارش، شمارش مجدد، ثبت و تأیید اختلاف و صدور سند تعدیل — دقیقاً بر اساس منطق حسابداری انبار. بازنویسی کامل این بخش در فاز انبار انجام می‌شود."
            reference="8Xmc1p"
            phase="فاز ۷ — انبار"
          />
        )
      case 'sales-return':
        return (
          <UnderConstruction
            title="برگشت از فروش"
            description="فرم برگشت از فروش با انتخاب فاکتور اصلی، کنترل مقدار برگشتی نسبت به فاکتور، اثر معکوس در انبار و صدور سند خودکار. موتور آن در هسته پیاده شده است."
            reference="FRPBDr"
            phase="فاز ۶"
          />
        )
      case 'proforma':
        return (
          <UnderConstruction
            title="پیش‌فاکتورها"
            description="صدور پیش‌فاکتور با اعتبار زمانی و تبدیل یک‌کلیکی به فاکتور فروش، بدون اثر انبار و مالی تا زمان تبدیل."
            reference="sFpxWK"
            phase="فاز ۶"
          />
        )
      case 'purchase-return':
        return (
          <UnderConstruction
            title="برگشت از خرید"
            description="فرم برگشت از خرید با کنترل مقدار نسبت به فاکتور خرید و اثر معکوس در بهای تمام‌شده."
            reference="PI5uot"
            phase="فاز ۶"
          />
        )
      case 'purchase-order':
        return (
          <UnderConstruction
            title="سفارش خرید"
            description="درخواست و سفارش خرید با پیگیری وضعیت و تبدیل به فاکتور خرید."
            reference="dgNqWj"
            phase="فاز ۶"
          />
        )
      case 'banks':
        return (
          <UnderConstruction
            title="حساب‌های بانکی"
            description="تعریف بانک با شماره شبا، شماره کارت، شعبه، پایانه فروشگاهی و سیاست هشدار منفی شدن موجودی. اعتبارسنجی شبا و کارت در هسته آماده و تست‌شده است."
            reference="p6hT01"
            phase="فاز ۶ — خزانه"
          />
        )
      case 'cashboxes':
        return (
          <UnderConstruction
            title="صندوق‌ها"
            description="تعریف صندوق با گروه تفصیلی، کد تفصیلی و سیاست هشدار منفی شدن موجودی (خطا / هشدار / بی‌تأثیر)."
            reference="WLumbs"
            phase="فاز ۶ — خزانه"
          />
        )
      case 'chart-of-accounts':
        return (
          <UnderConstruction
            title="کدینگ حساب‌ها"
            description="درخت کدینگ چندسطحی (گروه، کل، معین، تفصیلی) با شماره‌گذاری خودکار و الزامات تفصیلی شناور. موتور کدینگ در هسته پیاده و با ۱۰ تست پوشش داده شده است."
            reference="dgNqWj"
            phase="فاز ۶"
          />
        )
      case 'accounting':
      case 'chart-of-accounts':
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
      case 'treasury':
      case 'banks':
      case 'cashboxes':
        return <Treasury />
      case 'checks':
        return <Checks />
      default:
        return <Invoices page={page} />
    }
  }

  return (
    <div className={`app${dark ? ' dark' : ''}${collapsed ? ' sidebar-collapsed' : ''}`} dir="rtl">
      <aside className="sidebar">
        <div className="sidebar-head">
          <div className="sidebar-logo">
            <img
              src="https://novinacc.ir/wp-content/uploads/2023/07/cropped-%D9%84%D9%88%DA%AF%D9%88-300x83.png"
              alt="نوین پرداز"
              onError={(event) => {
                event.currentTarget.style.display = 'none'
                event.currentTarget.parentElement!.textContent = 'NP'
              }}
            />
          </div>
          <div className="sidebar-title">
            <strong>حسابداری نوین پرداز</strong>
            <span>NEXT GENERATION</span>
          </div>
        </div>

        <div className="dropdown-wrap" ref={companyRef}>
          <button
            className="company-btn"
            onClick={() => setOpenMenu(openMenu === 'company' ? '' : 'company')}
          >
            <div>
              <span>شرکت فعال</span>
              <b>نوین پرداز — شعبه مرکزی</b>
            </div>
            <Icon name="chevron" size={14} />
          </button>
          {openMenu === 'company' && (
            <div className="dropdown" style={{right: 10, left: 10, minWidth: 0}}>
              <div className="dropdown-title">انتخاب شرکت و سال مالی</div>
              <button className="dropdown-item">
                <Icon name="grid" size={15} />
                <div>
                  نوین پرداز — شعبه مرکزی
                  <small>سال مالی ۱۴۰۵ · فعال</small>
                </div>
              </button>
              <div className="dropdown-sep" />
              <button className="dropdown-item" onClick={() => setSettings(true)}>
                <Icon name="settings" size={15} />
                مدیریت شرکت‌ها و سال مالی
              </button>
            </div>
          )}
        </div>

        <nav className="sidebar-nav">
          {MENU.map((group) => (
            <div key={group.title}>
              <div className="nav-group-title">{group.title}</div>
              {group.items.map((item) => {
                const isActive =
                  page === item.page || item.children?.some((child) => child.page === page)
                return (
                  <div key={item.id}>
                    <div className="nav-row">
                      <button
                        className={`nav-item${isActive ? ' active' : ''}`}
                        data-tip={item.label}
                        onClick={() => go(item.page)}
                      >
                        <span className="nav-icon">
                          <Icon name={item.icon as never} size={19} />
                        </span>
                        <span className="nav-label">{item.label}</span>
                      </button>
                      {item.children && (
                        <button
                          className={`nav-expand${expanded.includes(item.id) ? ' open' : ''}`}
                          onClick={() => toggleExpand(item.id)}
                          title="باز کردن زیرمنو"
                        >
                          <Icon name="chevron" size={13} />
                        </button>
                      )}
                    </div>
                    {item.children && expanded.includes(item.id) && (
                      <div className="subnav">
                        {item.children.map((child) => (
                          <button
                            key={child.page}
                            className={page === child.page ? 'selected' : ''}
                            onClick={() => go(child.page)}
                          >
                            {child.label}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          ))}
        </nav>

        <div className="sidebar-foot">
          <div className="nav-row">
            <button className="nav-item" data-tip="تنظیمات برنامه" onClick={() => setSettings(true)}>
              <span className="nav-icon">
                <Icon name="settings" size={19} />
              </span>
              <span className="nav-label">تنظیمات برنامه</span>
            </button>
          </div>
        </div>

        <button
          className="sidebar-toggle"
          onClick={() => setCollapsed(!collapsed)}
          title={collapsed ? 'باز کردن منو' : 'جمع کردن منو'}
        >
          <Icon name="chevron" size={14} />
        </button>
      </aside>

      <main className="main">
        <header className="topbar">
          <div className="breadcrumbs">
            <span>نوین پرداز</span>
            <b>/</b>
            <strong>{PAGE_TITLES[page] ?? 'صفحه'}</strong>
          </div>

          <div className="top-actions">
            {isDesignPreview() && (
              <div className="preview-banner" title="داده‌ها شبیه‌سازی‌شده‌اند">
                ⚠ پیش‌نمایش طراحی
              </div>
            )}

            <button className="global-search" onClick={() => setPalette(true)}>
              <Icon name="search" size={16} />
              <span>جستجو یا اجرای دستور…</span>
              <kbd>Ctrl K</kbd>
            </button>

            <button className="icon-btn" onClick={() => setDark(!dark)} title="تغییر تم">
              <Icon name={dark ? 'sun' : 'moon'} size={17} />
            </button>

            <div className="dropdown-wrap" ref={bellRef}>
              <button
                className="icon-btn"
                onClick={() => setOpenMenu(openMenu === 'bell' ? '' : 'bell')}
                title="اعلان‌ها"
              >
                <Icon name="bell" size={17} />
                <i className="badge-dot" />
              </button>
              {openMenu === 'bell' && (
                <div className="dropdown">
                  <div className="dropdown-title">اعلان‌ها</div>
                  <button className="dropdown-item" onClick={() => go('checks')}>
                    <Icon name="check" size={15} />
                    <div>
                      چک نزدیک سررسید
                      <small>۱ فقره چک در ۷ روز آینده سررسید می‌شود</small>
                    </div>
                  </button>
                  <button className="dropdown-item" onClick={() => go('inventory')}>
                    <Icon name="package" size={15} />
                    <div>
                      کالای کم‌موجودی
                      <small>موجودی چند کالا زیر حد سفارش است</small>
                    </div>
                  </button>
                  <div className="dropdown-sep" />
                  <button className="dropdown-item" onClick={() => setSettings(true)}>
                    <Icon name="settings" size={15} />
                    تنظیمات اعلان‌ها
                  </button>
                </div>
              )}
            </div>

            <div className="dropdown-wrap" ref={profileRef}>
              <button
                className="profile-btn"
                onClick={() => setOpenMenu(openMenu === 'profile' ? '' : 'profile')}
              >
                <div className="avatar">م</div>
                <div>
                  <b>مدیر سیستم</b>
                  <span>Administrator</span>
                </div>
                <Icon name="chevron" size={13} />
              </button>
              {openMenu === 'profile' && (
                <div className="dropdown">
                  <div className="dropdown-title">حساب کاربری</div>
                  <button className="dropdown-item" onClick={() => setSettings(true)}>
                    <Icon name="users" size={15} />
                    <div>
                      مدیر سیستم
                      <small>نقش: Administrator — دسترسی کامل</small>
                    </div>
                  </button>
                  <div className="dropdown-sep" />
                  <button className="dropdown-item" onClick={() => setSettings(true)}>
                    <Icon name="settings" size={15} />
                    تنظیمات کاربر و تغییر رمز
                  </button>
                  <button className="dropdown-item" onClick={() => window.location.reload()}>
                    <Icon name="refresh" size={15} />
                    خروج از حساب
                  </button>
                </div>
              )}
            </div>
          </div>
        </header>

        <div className="workspace">{renderPage()}</div>
      </main>

      {/* --- دکمه‌های شناور پایین سمت چپ --- */}
      <div className="fab-stack" ref={fabRef}>
        {openMenu === 'fab' && (
          <div className="fab-menu">
            <div className="dropdown-title">عملیات سریع</div>
            {QUICK_ACTIONS.map((action) => (
              <button key={action.page} onClick={() => go(action.page)}>
                <span className="fab-icon">
                  <Icon name={action.icon as never} size={16} />
                </span>
                {action.label}
              </button>
            ))}
            <div className="dropdown-sep" />
            <button onClick={() => setSettings(true)}>
              <span className="fab-icon">
                <Icon name="settings" size={16} />
              </span>
              شخصی‌سازی این فهرست
            </button>
          </div>
        )}
        <button
          className={`fab${openMenu === 'fab' ? ' open' : ''}`}
          onClick={() => setOpenMenu(openMenu === 'fab' ? '' : 'fab')}
          title="عملیات سریع"
        >
          +
        </button>
        {DEMO_BUILD && demo && (
          <button
            className="demo-fab"
            disabled={demoBusy}
            onClick={async () => {
              if (!confirm('تمام محتوای نمونه حذف می‌شود. ادامه می‌دهید؟')) return
              setDemoBusy(true)
              try {
                await deleteDemo()
                setDemo(false)
                window.location.reload()
              } catch (e) {
                alert(errorText(e))
              } finally {
                setDemoBusy(false)
              }
            }}
          >
            {demoBusy ? 'در حال حذف…' : 'حذف محتوای دمو'}
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
