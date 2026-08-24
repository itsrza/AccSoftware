import { useEffect, useRef, useState } from 'react'
import {
  BarChart3,
  Boxes,
  Building2,
  Check,
  ChevronDown,
  CircleUserRound,
  FileText,
  LayoutGrid,
  PanelRightClose,
  PanelRightOpen,
  Plug,
  Printer,
  Receipt,
  ScrollText,
  Settings,
  ShoppingCart,
  Upload,
  Users,
  Wallet,
  X,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '../lib/cn'
import { Avatar } from './Avatar'
import { formatCount } from '../lib/format'
import { useI18n } from '../lib/i18n'

/**
 * منوی کناری — منطبق با سیستم طراحی مرجع.
 *
 * ## سه ایرادی که این نسخه رفع می‌کند
 *
 * ۱. **تولتیپ در حالت جمع‌شده کار نمی‌کرد.** حالا هر آیتم بدون زیرمنو تولتیپ
 *    نام دارد و آیتم دارای زیرمنو، منوی شناور کامل باز می‌کند.
 * ۲. **اسکرول منو کار نمی‌کرد.** ناحیه‌ی پیمایش با `min-h-0` و
 *    `overflow-y-auto` جدا شده تا در ارتفاع کم واقعاً اسکرول شود.
 * ۳. **عرض منو با تغییر صفحه می‌پرید.** عرض ثابت است و فقط با دکمه‌ی
 *    جمع‌کردن تغییر می‌کند.
 *
 * طلایی فقط برای «تأکید» استفاده می‌شود: نوار آیتم فعال، آیکن فعال و
 * نشانگرها — نه پس‌زمینه‌ی بزرگ.
 */

export type NavChild = { label: string; page: string }
export type NavItem = {
  id: string
  label: string
  icon: LucideIcon
  page: string
  children?: NavChild[]
  /** نشان عددی کنار آیتم (مثلاً تعداد چک نزدیک سررسید). */
  badge?: number
}
export type NavGroup = { title: string; items: NavItem[] }

/** نگاشت نام آیکن قدیمی به آیکن lucide، تا تعریف منو دست‌نخورده بماند. */
export const ICONS: Record<string, LucideIcon> = {
  grid: LayoutGrid,
  receipt: Receipt,
  cart: ShoppingCart,
  package: Boxes,
  wallet: Wallet,
  check: ScrollText,
  file: FileText,
  bar: BarChart3,
  users: Users,
  settings: Settings,
  print: Printer,
  upload: Upload,
  plug: Plug,
}

function Tooltip({ label }: { label: string }) {
  return (
    <span
      role="tooltip"
      className="pointer-events-none absolute end-[calc(100%+10px)] top-1/2 z-50 -translate-y-1/2 translate-x-1 rounded-lg border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold whitespace-nowrap text-text opacity-0 shadow-[var(--shadow-md)] transition-all duration-150 group-hover:translate-x-0 group-hover:opacity-100"
    >
      {label}
    </span>
  )
}

function ItemContent({
  item,
  active,
  open,
  collapsed,
}: {
  item: NavItem
  active: boolean
  open?: boolean
  collapsed: boolean
}) {
  const Icon = item.icon
  return (
    <span
      className={cn(
        'relative flex w-full items-center gap-3 rounded-xl py-2.5 text-[12.5px] font-medium transition-all duration-200',
        collapsed ? 'justify-center px-0' : 'px-3',
        active
          ? 'bg-white/10 font-bold text-white shadow-[inset_0_1px_0_rgba(255,255,255,.08)]'
          : 'text-[var(--sidebar-text)] hover:bg-white/5 hover:text-white',
      )}
    >
      {active && (
        <span className="absolute inset-y-2 start-0 w-[3px] rounded-full bg-accent" aria-hidden />
      )}
      <Icon className={cn('size-[18px] shrink-0 transition-colors', active && 'text-accent')} aria-hidden />
      {!collapsed && <span className="min-w-0 flex-1 truncate text-start">{item.label}</span>}
      {!collapsed && item.children && (
        <ChevronDown
          className={cn(
            'size-3.5 shrink-0 text-[var(--sidebar-text)] transition-transform duration-300',
            open && 'rotate-180',
          )}
          aria-hidden
        />
      )}
      {!collapsed && item.badge !== undefined && item.badge > 0 && (
        <span className="grid h-5 min-w-5 place-items-center rounded-full bg-accent px-1 text-[10px] font-bold text-[#21254E]">
          {formatCount(item.badge)}
        </span>
      )}
    </span>
  )
}

function NavEntry({
  item,
  collapsed,
  page,
  navigate,
}: {
  item: NavItem
  collapsed: boolean
  page: string
  navigate: (page: string) => void
}) {
  const hasActiveChild = !!item.children?.some((child) => child.page === page)
  const active = page === item.page || hasActiveChild
  // منو خودکار باز نمی‌شود مگر شاخه‌ی فعال باشد — بازخورد «منوها خودکار بازند».
  const [manual, setManual] = useState<boolean | null>(null)
  const open = manual ?? hasActiveChild

  // ---- حالت جمع‌شده با زیرمنو: منوی شناور
  if (collapsed && item.children) {
    return (
      <li className="group relative">
        <button
          onClick={() => navigate(item.page)}
          aria-label={item.label}
          aria-current={active ? 'page' : undefined}
          className="w-full"
        >
          <ItemContent item={item} active={active} collapsed />
        </button>
        <div className="invisible absolute end-[calc(100%+10px)] top-0 z-50 w-52 rounded-xl border border-border bg-card p-1.5 opacity-0 shadow-[var(--shadow-lg)] transition-all duration-200 group-hover:visible group-hover:opacity-100">
          <p className="px-2 py-1 text-[10px] font-bold text-faint">{item.label}</p>
          {item.children.map((child) => (
            <button
              key={child.page}
              onClick={() => navigate(child.page)}
              className={cn(
                'block w-full rounded-lg px-2.5 py-2 text-start text-xs transition-colors',
                page === child.page
                  ? 'bg-[var(--accent-soft)] font-bold text-accent-strong'
                  : 'text-muted hover:bg-bg-soft hover:text-text',
              )}
            >
              {child.label}
            </button>
          ))}
        </div>
      </li>
    )
  }

  // ---- آیتم ساده
  if (!item.children) {
    return (
      <li className="group relative">
        <button
          onClick={() => navigate(item.page)}
          aria-label={collapsed ? item.label : undefined}
          aria-current={page === item.page ? 'page' : undefined}
          className="w-full"
        >
          <ItemContent item={item} active={active} collapsed={collapsed} />
        </button>
        {collapsed && <Tooltip label={item.label} />}
      </li>
    )
  }

  // ---- گروه بازشونده
  return (
    <li>
      <button
        onClick={() => setManual(!open)}
        aria-expanded={open}
        aria-label={item.label}
        className="w-full"
      >
        <ItemContent item={item} active={active} open={open} collapsed={collapsed} />
      </button>
      <div
        className={cn(
          'grid transition-[grid-template-rows] duration-300 ease-out',
          open ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]',
        )}
      >
        <div className="overflow-hidden">
          <ul className="ms-5 mt-1 space-y-0.5 border-s border-white/10 ps-3 pb-1">
            {item.children.map((child) => (
              <li key={child.page}>
                <button
                  onClick={() => navigate(child.page)}
                  aria-current={page === child.page ? 'page' : undefined}
                  className={cn(
                    'relative w-full rounded-lg px-3 py-2 text-start text-[11.5px] transition-colors',
                    page === child.page
                      ? 'font-bold text-accent'
                      : 'text-[var(--sidebar-text)] hover:text-white',
                  )}
                >
                  <span
                    className={cn(
                      'absolute -start-[13px] top-1/2 h-px w-2.5 bg-white/20',
                      page === child.page && 'bg-accent',
                    )}
                    aria-hidden
                  />
                  {child.label}
                </button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </li>
  )
}

function CompanyChip({
  collapsed,
  companyName,
  fiscalYear,
}: {
  collapsed: boolean
  companyName: string
  fiscalYear: string
}) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) setOpen(false)
    }
    const onKey = (event: KeyboardEvent) => event.key === 'Escape' && setOpen(false)
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [open])

  return (
    <div ref={ref} className={cn('relative mb-3', collapsed ? '' : 'w-full')}>
      <button
        type="button"
        aria-label={t('company.activeCompany')}
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className={cn(collapsed ? '' : 'block w-full text-start')}
      >
        {collapsed ? (
          <span className="grid size-11 cursor-pointer place-items-center rounded-xl border border-[var(--sidebar-border)] bg-white/5 text-[var(--sidebar-text)] transition-colors hover:bg-white/10 hover:text-white">
            <Building2 className="size-5" aria-hidden />
          </span>
        ) : (
          <span className="flex w-full cursor-pointer items-center gap-2.5 rounded-xl border border-[var(--sidebar-border)] bg-white/5 px-3 py-2.5 text-start transition-colors hover:bg-white/10">
            <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-[var(--accent-soft)] text-accent">
              <Building2 className="size-4" aria-hidden />
            </span>
            <span className="min-w-0 flex-1 leading-tight">
              <span className="block truncate text-xs font-bold text-white">{companyName}</span>
              <span className="block truncate text-[10px] text-[var(--sidebar-text)]">
                {t('company.fiscalYear', { year: fiscalYear })}
              </span>
            </span>
            <ChevronDown className="size-3.5 shrink-0 text-[var(--sidebar-text)]" aria-hidden />
          </span>
        )}
      </button>
      {open && (
        <div className="fade-up absolute top-[calc(100%+8px)] z-50 w-60 start-0 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">{t('company.activePeriod')}</p>
          <div className="flex items-center justify-between gap-2 rounded-lg bg-[var(--accent-soft)] px-2.5 py-2 text-xs font-bold text-accent-strong">
            <span className="min-w-0 truncate">{companyName}</span>
            <Check className="size-3.5 shrink-0" aria-hidden />
          </div>
          <p className="px-2.5 pt-2 text-[11px] leading-6 text-muted">
            {t('company.fiscalOpenHint', { year: fiscalYear })}
          </p>
        </div>
      )}
    </div>
  )
}

export function Sidebar({
  groups,
  bottom,
  page,
  navigate,
  collapsed,
  toggleCollapsed,
  mobileOpen,
  setMobileOpen,
  companyName,
  fiscalYear,
  userName,
  userRole,
  avatar,
  onOpenSettings,
}: {
  groups: NavGroup[]
  bottom: NavItem[]
  page: string
  navigate: (page: string) => void
  collapsed: boolean
  toggleCollapsed: () => void
  mobileOpen: boolean
  setMobileOpen: (value: boolean) => void
  companyName: string
  fiscalYear: string
  userName: string
  userRole: string
  avatar?: string
  onOpenSettings: () => void
}) {
  const { t, dir } = useI18n()
  const body = (isCollapsed: boolean) => (
    <div className="flex h-full min-h-0 flex-col">
      <div className="relative">
        <div
          className={cn(
            'brand-shine flex items-center gap-3 px-4 pt-5 pb-4',
            isCollapsed && 'justify-center px-2',
          )}
        >
          <div className="relative grid size-11 shrink-0 place-items-center rounded-2xl bg-gradient-to-br from-[#e7bd75] to-[#c8923c] shadow-[0_8px_20px_-6px_rgba(220,167,87,.55)]">
            <svg viewBox="0 0 24 24" className="size-6 text-[#21254E]" fill="currentColor" aria-hidden>
              <path d="M12 2 2.5 9.5 12 22l9.5-12.5L12 2Zm0 3.1 5.4 4.4L12 17.2 6.6 9.5 12 5.1Z" />
            </svg>
          </div>
          {!isCollapsed && (
            <div className="min-w-0 leading-tight">
              <p className="text-[15px] font-extrabold tracking-tight text-white">{t('app.name')}</p>
              <p className="mt-0.5 text-[10px] font-medium text-[var(--sidebar-text)]">
                {t('app.tagline')}
              </p>
            </div>
          )}
        </div>
        <button
          onClick={toggleCollapsed}
          aria-label={isCollapsed ? t('nav.openMenu') : t('nav.collapseMenu')}
          className="absolute top-6 -end-3 hidden size-7 place-items-center rounded-full border border-[var(--sidebar-border)] bg-[#262a58] text-[var(--sidebar-text)] shadow-md transition-colors hover:text-white lg:grid"
        >
          {isCollapsed ? (
            <PanelRightOpen className="size-3.5" aria-hidden />
          ) : (
            <PanelRightClose className="size-3.5" aria-hidden />
          )}
        </button>
        <button
          onClick={() => setMobileOpen(false)}
          aria-label={t('nav.closeMenu')}
          className="absolute top-5 start-3 grid size-8 place-items-center rounded-lg text-[var(--sidebar-text)] transition-colors hover:bg-white/10 hover:text-white lg:hidden"
        >
          <X className="size-4" aria-hidden />
        </button>
      </div>

      <div className={cn(isCollapsed ? 'flex justify-center' : 'px-4')}>
        <CompanyChip collapsed={isCollapsed} companyName={companyName} fiscalYear={fiscalYear} />
      </div>

      <nav aria-label={t('nav.mainMenu')} className="sidebar-scroll min-h-0 flex-1 overflow-y-auto px-3 pb-2">
        {groups.map((group) => (
          <div key={group.title} className="mt-3 first:mt-0">
            {!isCollapsed && (
              <p className="px-3 pb-1.5 text-[10px] font-bold tracking-wide text-white/35">
                {group.title}
              </p>
            )}
            {isCollapsed && <div className="mx-2 my-3 h-px bg-white/10" aria-hidden />}
            <ul className="space-y-0.5">
              {group.items.map((item) => (
                <NavEntry
                  key={item.id}
                  item={item}
                  collapsed={isCollapsed}
                  page={page}
                  navigate={navigate}
                />
              ))}
            </ul>
          </div>
        ))}
      </nav>

      <div className="border-t border-[var(--sidebar-border)] p-3">
        <ul className="space-y-0.5">
          {bottom.map((item) => (
            <NavEntry
              key={item.id}
              item={item}
              collapsed={isCollapsed}
              page={page}
              navigate={navigate}
            />
          ))}
        </ul>

        {!isCollapsed ? (
          <button
            onClick={onOpenSettings}
            aria-label={t('page.settings')}
            className="mt-3 flex w-full items-center gap-2.5 rounded-xl border border-[var(--sidebar-border)] bg-white/5 px-3 py-2.5 text-start transition-colors hover:bg-white/10"
          >
            <span className="relative shrink-0">
              <Avatar src={avatar} name={userName} size={36} />
              <span
                className="pulse-dot absolute -bottom-0.5 -end-0.5 size-2.5 rounded-full border-2 border-[#1d2046] bg-success"
                aria-hidden
              />
            </span>
            <span className="min-w-0 flex-1 leading-tight">
              <span className="block truncate text-xs font-bold text-white">{userName}</span>
              <span className="block text-[10px] text-[var(--sidebar-text)]">{userRole}</span>
            </span>
            <CircleUserRound className="size-4 shrink-0 text-[var(--sidebar-text)]" aria-hidden />
          </button>
        ) : (
          <button
            onClick={onOpenSettings}
            aria-label={t('page.settings')}
            className="mt-3 flex w-full justify-center"
          >
            <span className="relative">
              <Avatar src={avatar} name={userName} size={36} />
              <span
                className="pulse-dot absolute -bottom-0.5 -end-0.5 size-2.5 rounded-full border-2 border-[#1d2046] bg-success"
                aria-hidden
              />
            </span>
          </button>
        )}
      </div>
    </div>
  )

  return (
    <>
      <aside
        className={cn(
          'fixed inset-y-0 start-0 z-40 hidden flex-col border-e border-[var(--sidebar-border)] bg-gradient-to-b from-[var(--sidebar-from)] to-[var(--sidebar-to)] transition-[width] duration-300 lg:flex',
          collapsed ? 'w-[84px]' : 'w-[272px]',
        )}
      >
        {body(collapsed)}
      </aside>

      <div
        className={cn(
          'fixed inset-0 z-50 transition-opacity duration-300 lg:hidden',
          mobileOpen ? 'opacity-100' : 'pointer-events-none opacity-0',
        )}
        aria-hidden={!mobileOpen}
      >
        <div
          className="absolute inset-0 bg-[#12142e]/60 backdrop-blur-[2px]"
          onClick={() => setMobileOpen(false)}
        />
        <aside
          className={cn(
            'absolute inset-y-0 start-0 flex w-[288px] max-w-[85vw] flex-col bg-gradient-to-b from-[var(--sidebar-from)] to-[var(--sidebar-to)] shadow-[var(--shadow-lg)] transition-transform duration-300',
            mobileOpen ? 'translate-x-0' : dir === 'rtl' ? 'translate-x-full' : '-translate-x-full',
          )}
        >
          {body(false)}
        </aside>
      </div>
    </>
  )
}
