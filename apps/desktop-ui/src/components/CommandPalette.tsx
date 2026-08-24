import {useEffect, useMemo, useRef, useState} from 'react'
import {Icon} from './Icon'
import {useI18n, type TranslationKey} from '../lib/i18n'

/**
 * پالت فرمان (Ctrl+K).
 *
 * شناسه‌ی هر مورد دقیقاً همان `page` مسیریابی `App.tsx` است؛ نسخه‌ی قبلی
 * شناسه‌های ناموجود («contacts») داشت و کاربر را به صفحه‌ی پیش‌فرض می‌برد.
 * ابزارهایی که از منوی کناری برداشته شده‌اند (قالب چاپ، اتصالات) اینجا
 * قابل دسترس‌اند.
 *
 * عنوان هر مورد از کلید `page.<id>` می‌آید، پس با تغییر زبان برنامه،
 * پالت هم ترجمه می‌شود و جستجو روی متن همان زبان کار می‌کند.
 */
type Item = {id: string; group: TranslationKey; icon: string; keywords?: string}

const ITEMS: Item[] = [
  {id: 'dashboard', group: 'palette.group.navigation', icon: 'grid'},
  {id: 'invoice-form', group: 'palette.group.sales', icon: 'receipt', keywords: 'فاکتور جدید new invoice فاتورة'},
  {id: 'sales', group: 'palette.group.sales', icon: 'receipt'},
  {id: 'sales-return', group: 'palette.group.sales', icon: 'receipt'},
  {id: 'proforma', group: 'palette.group.sales', icon: 'file'},
  {id: 'purchase', group: 'palette.group.purchase', icon: 'cart'},
  {id: 'purchase-return', group: 'palette.group.purchase', icon: 'cart'},
  {id: 'purchase-order', group: 'palette.group.purchase', icon: 'cart'},
  {id: 'products', group: 'palette.group.inventory', icon: 'package'},
  {id: 'product-pricing', group: 'palette.group.inventory', icon: 'package'},
  {id: 'inventory', group: 'palette.group.inventory', icon: 'package'},
  {id: 'inventory-transfer', group: 'palette.group.inventory', icon: 'package'},
  {id: 'inventory-count', group: 'palette.group.inventory', icon: 'package'},
  {id: 'production', group: 'palette.group.inventory', icon: 'package'},
  {id: 'treasury-document', group: 'palette.group.treasury', icon: 'wallet'},
  {id: 'treasury', group: 'palette.group.treasury', icon: 'wallet'},
  {id: 'banks', group: 'palette.group.treasury', icon: 'wallet'},
  {id: 'cashboxes', group: 'palette.group.treasury', icon: 'wallet'},
  {id: 'checks', group: 'palette.group.treasury', icon: 'check'},
  {id: 'single-journal', group: 'palette.group.accounting', icon: 'file'},
  {id: 'accounting', group: 'palette.group.accounting', icon: 'file'},
  {id: 'chart-of-accounts', group: 'palette.group.accounting', icon: 'file'},
  {id: 'parties', group: 'palette.group.accounting', icon: 'users', keywords: 'مشتری تأمین‌کننده customer supplier عميل مورد'},
  {id: 'reports', group: 'palette.group.reports', icon: 'bar'},
  {id: 'report-builder', group: 'palette.group.reports', icon: 'bar'},
  {id: 'data-tools', group: 'palette.group.tools', icon: 'upload', keywords: 'csv اکسل excel'},
  {id: 'print-templates', group: 'palette.group.tools', icon: 'print'},
  {id: 'integrations', group: 'palette.group.tools', icon: 'plug', keywords: 'api افزونه add-on إضافة'},
]

export function CommandPalette({
  open,
  onClose,
  onSelect,
}: {
  open: boolean
  onClose: () => void
  onSelect: (id: string) => void
}) {
  const {t} = useI18n()
  const [q, setQ] = useState('')
  const [active, setActive] = useState(0)
  const listRef = useRef<HTMLDivElement>(null)

  /** عنوان و گروه به زبان فعال، تا جستجو روی همان متنی کار کند که کاربر می‌بیند. */
  const rows = useMemo(
    () =>
      ITEMS.map((item) => ({
        ...item,
        label: t(`page.${item.id}` as TranslationKey),
        groupLabel: t(item.group),
      })),
    [t],
  )

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase()
    if (!needle) return rows
    return rows.filter(
      (item) =>
        item.label.toLowerCase().includes(needle) ||
        item.groupLabel.toLowerCase().includes(needle) ||
        (item.keywords ?? '').toLowerCase().includes(needle),
    )
  }, [q, rows])

  useEffect(() => {
    if (!open) return
    setQ('')
    setActive(0)
  }, [open])

  useEffect(() => setActive(0), [q])

  useEffect(() => {
    if (!open) return
    const handler = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        setActive((i) => (filtered.length ? (i + 1) % filtered.length : 0))
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        setActive((i) => (filtered.length ? (i - 1 + filtered.length) % filtered.length : 0))
      }
      if (event.key === 'Enter' && filtered[active]) {
        event.preventDefault()
        onSelect(filtered[active].id)
        onClose()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [open, onClose, onSelect, filtered, active])

  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView?.({block: 'nearest'})
  }, [active])

  if (!open) return null

  return (
    <div className="command-backdrop" onClick={onClose}>
      <div className="command-palette" onClick={(e) => e.stopPropagation()}>
        <div className="command-input">
          <Icon name="search" />
          <input
            autoFocus
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder={t('palette.placeholder')}
            aria-label={t('palette.aria')}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="command-list" ref={listRef} role="listbox">
          {filtered.map((item, i) => (
            <button
              key={item.id}
              data-i={i}
              role="option"
              aria-selected={i === active}
              className={i === active ? 'selected' : undefined}
              onMouseEnter={() => setActive(i)}
              onClick={() => {
                onSelect(item.id)
                onClose()
              }}
            >
              <Icon name={item.icon as never} />
              <span>{item.label}</span>
              <small>{item.groupLabel}</small>
              <Icon name="chevron" size={14} />
            </button>
          ))}
          {!filtered.length && (
            <div className="empty-state">
              <p>{t('common.noResult')}</p>
            </div>
          )}
        </div>
        <div className="command-foot">
          <span>{t('palette.enter')}</span>
          <span>{t('palette.move')}</span>
          <span>{t('palette.escape')}</span>
        </div>
      </div>
    </div>
  )
}
