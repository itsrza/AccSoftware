import { useEffect, useMemo, useRef, useState } from 'react'
import {
  Bell,
  Check as CheckIcon,
  Languages,
  LogOut,
  Menu,
  Moon,
  Search,
  Settings,
  Sun,
  X,
} from 'lucide-react'
import { cn } from '../lib/cn'
import { Avatar } from './Avatar'
import { CalendarMenu } from './CalendarPopover'
import { formatCount } from '../lib/format'
import { LOCALES, useI18n } from '../lib/i18n'

/**
 * نوار بالای برنامه — منطبق با سیستم طراحی مرجع.
 *
 * ## ایرادهایی که این نسخه رفع می‌کند
 *
 * - **متن جستجو از باکس بیرون می‌زد** → فیلد با ارتفاع ثابت و `min-w-0`.
 * - **زنگ اعلان و پروفایل کار نمی‌کردند** → هر دو منوی واقعی دارند و
 *   اعلان‌ها از داده‌ی واقعی برنامه می‌آیند.
 * - **جستجو تزئینی بود** → نتیجه‌ها قابل کلیک‌اند و به صفحه‌ی مربوطه می‌برند.
 */

export type SearchHit = { title: string; meta: string; page: string }
export type NotificationItem = {
  id: string
  title: string
  meta: string
  tone: 'danger' | 'warning' | 'info'
  page?: string
}
export type QuickAction = { label: string; page: string }

function useDismiss(onDismiss: () => void) {
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onDismiss()
    }
    const onKey = (event: KeyboardEvent) => event.key === 'Escape' && onDismiss()
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey)
    }
  }, [onDismiss])
  return ref
}

function GlobalSearch({
  search,
  navigate,
}: {
  search: (query: string) => SearchHit[]
  navigate: (page: string) => void
}) {
  const { t } = useI18n()
  const [query, setQuery] = useState('')
  const [focused, setFocused] = useState(false)

  const hits = useMemo(() => {
    const trimmed = query.trim()
    if (trimmed.length < 2) return null
    return search(trimmed)
  }, [query, search])

  return (
    <div className="relative w-full min-w-0 max-w-md">
      <Search
        className="pointer-events-none absolute start-3 top-1/2 size-4 -translate-y-1/2 text-faint"
        aria-hidden
      />
      <input
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setTimeout(() => setFocused(false), 180)}
        onKeyDown={(event) => event.key === 'Escape' && setQuery('')}
        type="search"
        placeholder={t('topbar.searchPlaceholder')}
        aria-label={t('topbar.globalSearch')}
        className="h-10 w-full rounded-xl border border-border bg-bg-soft ps-9 pe-9 text-xs font-medium text-text placeholder:text-faint outline-none transition-all focus:border-accent focus:bg-card"
      />
      {query && (
        <button
          aria-label={t('topbar.clearSearch')}
          onClick={() => setQuery('')}
          className="absolute end-2.5 top-1/2 grid size-5 -translate-y-1/2 place-items-center rounded-full text-faint hover:bg-bg-soft hover:text-text"
        >
          <X className="size-3.5" aria-hidden />
        </button>
      )}

      {focused && hits && (
        <div className="fade-up absolute top-[calc(100%+8px)] z-50 w-full min-w-72 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          {hits.length === 0 ? (
            <p className="px-3 py-6 text-center text-xs text-muted">
              {t('topbar.noSearchResult', { query })}
            </p>
          ) : (
            <div className="max-h-80 overflow-y-auto">
              {hits.map((hit) => (
                <button
                  key={`${hit.page}-${hit.title}`}
                  onClick={() => {
                    setQuery('')
                    navigate(hit.page)
                  }}
                  className="flex w-full flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-start transition-colors hover:bg-bg-soft"
                >
                  <span className="text-xs font-semibold text-text">{hit.title}</span>
                  <span className="text-[11px] text-muted">{hit.meta}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function NotificationsMenu({
  items,
  navigate,
}: {
  items: NotificationItem[]
  navigate: (page: string) => void
}) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const ref = useDismiss(() => setOpen(false))
  const tones: Record<NotificationItem['tone'], string> = {
    danger: 'bg-[var(--danger-soft)] text-danger',
    warning: 'bg-[var(--warning-soft)] text-warning',
    info: 'bg-[var(--info-soft)] text-info',
  }

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((value) => !value)}
        aria-label={t('topbar.notifications')}
        aria-expanded={open}
        className="relative grid size-10 place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text"
      >
        <Bell className="size-4.5" aria-hidden />
        {items.length > 0 && (
          <span className="absolute -top-1 -end-1 grid h-4 min-w-4 place-items-center rounded-full bg-accent px-1 text-[10px] font-bold text-[#21254E]">
            {formatCount(items.length)}
          </span>
        )}
      </button>
      {open && (
        <div className="fade-up absolute top-[calc(100%+8px)] end-0 z-50 w-80 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">
            {t('topbar.notificationsNote')}
          </p>
          {items.length === 0 ? (
            <p className="px-3 py-6 text-center text-xs text-muted">
              {t('topbar.notificationsEmpty')}
            </p>
          ) : (
            <div className="max-h-80 overflow-y-auto">
              {items.map((item) => (
                <button
                  key={item.id}
                  onClick={() => {
                    setOpen(false)
                    if (item.page) navigate(item.page)
                  }}
                  className="flex w-full items-start gap-2.5 rounded-lg px-2.5 py-2.5 text-start transition-colors hover:bg-bg-soft"
                >
                  <span className={cn('mt-0.5 size-2 shrink-0 rounded-full', tones[item.tone])} />
                  <span className="min-w-0 flex-1">
                    <span className="block text-xs font-semibold text-text">{item.title}</span>
                    <span className="block text-[11px] leading-5 text-muted">{item.meta}</span>
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function ProfileMenu({
  userName,
  userRole,
  avatar,
  onSettings,
  onLogout,
}: {
  userName: string
  userRole: string
  avatar?: string
  onSettings: () => void
  onLogout: () => void
}) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const ref = useDismiss(() => setOpen(false))

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((value) => !value)}
        aria-label={t('topbar.account')}
        aria-expanded={open}
        className="flex items-center gap-2 rounded-xl border border-border bg-card px-2 py-1.5 transition-colors hover:border-border-strong"
      >
        <Avatar src={avatar} name={userName} size={28} />
        <span className="hidden min-w-0 leading-tight sm:block">
          <span className="block truncate text-[11px] font-bold text-text">{userName}</span>
          <span className="block text-[10px] text-faint">{userRole}</span>
        </span>
      </button>
      {open && (
        <div className="fade-up absolute top-[calc(100%+8px)] end-0 z-50 w-56 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          <div className="flex items-center gap-2.5 rounded-xl bg-bg-soft px-2.5 py-2.5">
            <Avatar src={avatar} name={userName} size={36} />
            <span className="min-w-0 leading-tight">
              <span className="block truncate text-xs font-bold text-text">{userName}</span>
              <span className="block text-[10px] text-muted">{userRole}</span>
            </span>
          </div>
          <button
            onClick={() => {
              setOpen(false)
              onSettings()
            }}
            className="mt-1 flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-xs text-muted transition-colors hover:bg-bg-soft hover:text-text"
          >
            <Settings className="size-3.5" aria-hidden /> {t('page.settings')}
          </button>
          <button
            onClick={() => {
              setOpen(false)
              onLogout()
            }}
            className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-xs text-danger transition-colors hover:bg-[var(--danger-soft)]"
          >
            <LogOut className="size-3.5" aria-hidden /> {t('topbar.logout')}
          </button>
        </div>
      )}
    </div>
  )
}

/**
 * انتخاب‌گر زبان.
 *
 * نام هر زبان به خودِ آن زبان نوشته شده است — کاربری که رابط را به زبانی
 * ناآشنا دیده، باید بتواند زبان خودش را پیدا کند.
 */
function LanguageMenu() {
  const { locale, setLocale, t } = useI18n()
  const [open, setOpen] = useState(false)
  const ref = useDismiss(() => setOpen(false))
  const active = LOCALES.find((item) => item.code === locale) ?? LOCALES[0]

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen((value) => !value)}
        aria-label={t('topbar.language')}
        aria-expanded={open}
        className="flex h-10 items-center gap-1.5 rounded-xl border border-border bg-card px-2.5 text-muted transition-colors hover:text-text"
      >
        <Languages className="size-4.5" aria-hidden />
        <span className="text-[11px] font-bold">{active.short}</span>
      </button>
      {open && (
        <div className="fade-up absolute top-[calc(100%+8px)] end-0 z-50 w-44 rounded-2xl border border-border bg-card p-2 shadow-[var(--shadow-lg)]">
          <p className="px-2.5 py-1.5 text-[10px] font-bold text-faint">{t('topbar.language')}</p>
          {LOCALES.map((item) => (
            <button
              key={item.code}
              onClick={() => {
                setLocale(item.code)
                setOpen(false)
              }}
              dir={item.dir}
              className={cn(
                'flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-xs transition-colors',
                item.code === locale
                  ? 'bg-[var(--accent-soft)] font-bold text-accent-strong'
                  : 'text-muted hover:bg-bg-soft hover:text-text',
              )}
            >
              <span>{item.nativeLabel}</span>
              {item.code === locale && <CheckIcon className="size-3.5 shrink-0" aria-hidden />}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

export function Topbar({
  title,
  breadcrumb,
  dark,
  setDark,
  onOpenMobileNav,
  onOpenSettings,
  onLogout,
  search,
  navigate,
  notifications,
  userName,
  userRole,
  avatar,
}: {
  title: string
  breadcrumb: string
  dark: boolean
  setDark: (value: boolean) => void
  onOpenMobileNav: () => void
  onOpenSettings: () => void
  onLogout: () => void
  search: (query: string) => SearchHit[]
  navigate: (page: string) => void
  notifications: NotificationItem[]
  userName: string
  userRole: string
  /** تصویر پروفایل کاربر؛ خالی یعنی نشان پیش‌فرض طلایی. */
  avatar?: string
}) {
  const { t } = useI18n()
  return (
    <header className="sticky top-0 z-30 border-b border-border bg-[color-mix(in_srgb,var(--bg)_88%,transparent)] backdrop-blur-md">
      <div className="flex items-center gap-3 px-4 py-3 sm:px-6">
        <button
          onClick={onOpenMobileNav}
          aria-label={t('nav.openMenu')}
          className="grid size-10 shrink-0 place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text lg:hidden"
        >
          <Menu className="size-4.5" aria-hidden />
        </button>

        <div className="hidden min-w-0 lg:block">
          <p className="text-[10px] font-semibold text-faint">{breadcrumb}</p>
          <h1 className="truncate text-[15px] font-extrabold text-text">{title}</h1>
        </div>

        {/* فضای انعطاف‌پذیر: خوشه‌ی چپ همیشه سر جایش می‌ماند و با تغییر
          * طول عنوان صفحه جابه‌جا نمی‌شود. */}
        <div className="min-w-0 flex-1" aria-hidden />

        <div className="flex shrink-0 items-center gap-2">
          <GlobalSearch search={search} navigate={navigate} />
          <CalendarMenu />
          <button
            onClick={() => setDark(!dark)}
            aria-label={dark ? t('topbar.lightTheme') : t('topbar.darkTheme')}
            className="grid size-10 place-items-center rounded-xl border border-border bg-card text-muted transition-colors hover:text-text"
          >
            {dark ? <Sun className="size-4.5" aria-hidden /> : <Moon className="size-4.5" aria-hidden />}
          </button>
          <LanguageMenu />
          <NotificationsMenu items={notifications} navigate={navigate} />
          <ProfileMenu
            userName={userName}
            userRole={userRole}
            avatar={avatar}
            onSettings={onOpenSettings}
            onLogout={onLogout}
          />
        </div>
      </div>
    </header>
  )
}
