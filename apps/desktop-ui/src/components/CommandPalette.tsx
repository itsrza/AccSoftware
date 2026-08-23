import {useEffect, useMemo, useRef, useState} from 'react'
import {Icon} from './Icon'

/**
 * پالت فرمان (Ctrl+K).
 *
 * شناسه‌ی هر مورد دقیقاً همان `page` مسیریابی `App.tsx` است؛ نسخه‌ی قبلی
 * شناسه‌های ناموجود («contacts») داشت و کاربر را به صفحه‌ی پیش‌فرض می‌برد.
 * ابزارهایی که از منوی کناری برداشته شده‌اند (قالب چاپ، اتصالات) اینجا
 * قابل دسترس‌اند.
 */
type Item = {id: string; label: string; group: string; icon: string; keywords?: string}

const ITEMS: Item[] = [
  {id: 'dashboard', label: 'داشبورد', group: 'ناوبری', icon: 'grid'},
  {id: 'invoice-form', label: 'صدور فاکتور فروش', group: 'فروش', icon: 'receipt', keywords: 'فاکتور جدید'},
  {id: 'sales', label: 'فاکتورهای فروش', group: 'فروش', icon: 'receipt'},
  {id: 'sales-return', label: 'برگشت از فروش', group: 'فروش', icon: 'receipt'},
  {id: 'proforma', label: 'پیش‌فاکتورها', group: 'فروش', icon: 'file'},
  {id: 'purchase', label: 'فاکتورهای خرید', group: 'خرید', icon: 'cart'},
  {id: 'purchase-return', label: 'برگشت از خرید', group: 'خرید', icon: 'cart'},
  {id: 'purchase-order', label: 'سفارش خرید', group: 'خرید', icon: 'cart'},
  {id: 'products', label: 'کالاها', group: 'انبار', icon: 'package'},
  {id: 'product-pricing', label: 'قیمت کالاها', group: 'انبار', icon: 'package'},
  {id: 'inventory', label: 'موجودی انبار', group: 'انبار', icon: 'package'},
  {id: 'inventory-transfer', label: 'انتقال بین انبارها', group: 'انبار', icon: 'package'},
  {id: 'inventory-count', label: 'انبارگردانی', group: 'انبار', icon: 'package'},
  {id: 'production', label: 'تولید', group: 'انبار', icon: 'package'},
  {id: 'treasury-document', label: 'سند دریافت و پرداخت', group: 'خزانه', icon: 'wallet'},
  {id: 'treasury', label: 'گردش خزانه', group: 'خزانه', icon: 'wallet'},
  {id: 'banks', label: 'بانک‌ها', group: 'خزانه', icon: 'wallet'},
  {id: 'cashboxes', label: 'صندوق‌ها', group: 'خزانه', icon: 'wallet'},
  {id: 'checks', label: 'چک‌ها', group: 'خزانه', icon: 'check'},
  {id: 'single-journal', label: 'سند یک‌سطری', group: 'حسابداری', icon: 'file'},
  {id: 'accounting', label: 'اسناد حسابداری', group: 'حسابداری', icon: 'file'},
  {id: 'chart-of-accounts', label: 'کدینگ حساب‌ها', group: 'حسابداری', icon: 'file'},
  {id: 'parties', label: 'اشخاص', group: 'حسابداری', icon: 'users', keywords: 'مشتری تأمین‌کننده'},
  {id: 'reports', label: 'مرکز گزارشات', group: 'گزارش', icon: 'bar'},
  {id: 'report-builder', label: 'گزارش‌ساز', group: 'گزارش', icon: 'bar'},
  {id: 'data-tools', label: 'ورود و خروج اطلاعات', group: 'ابزار', icon: 'upload', keywords: 'csv اکسل'},
  {id: 'print-templates', label: 'قالب‌های چاپ', group: 'ابزار', icon: 'print'},
  {id: 'integrations', label: 'اتصالات و افزونه‌ها', group: 'ابزار', icon: 'plug', keywords: 'api افزونه'},
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
  const [q, setQ] = useState('')
  const [active, setActive] = useState(0)
  const listRef = useRef<HTMLDivElement>(null)

  const filtered = useMemo(() => {
    const needle = q.trim()
    if (!needle) return ITEMS
    return ITEMS.filter(
      (item) =>
        item.label.includes(needle) ||
        item.group.includes(needle) ||
        (item.keywords ?? '').includes(needle),
    )
  }, [q])

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
            placeholder="دستور یا صفحه را جستجو کنید…"
            aria-label="جستجوی فرمان"
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
              <small>{item.group}</small>
              <Icon name="chevron" size={14} />
            </button>
          ))}
          {!filtered.length && (
            <div className="empty-state">
              <p>نتیجه‌ای پیدا نشد.</p>
            </div>
          )}
        </div>
        <div className="command-foot">
          <span>Enter انتخاب</span>
          <span>↑ ↓ حرکت</span>
          <span>Esc بستن</span>
        </div>
      </div>
    </div>
  )
}
