import {useMemo, useState} from 'react'

export type SortDirection = 'asc' | 'desc'

/**
 * مرتب‌سازی سبک و سریع جدول‌ها در سمت کلاینت.
 *
 * چرا سمت کلاینت؟ صفحاتی که این قلاب را استفاده می‌کنند داده‌ی صفحه‌بندی‌شده
 * دریافت می‌کنند (حداکثر چند صد ردیف). مرتب‌سازی روی همان مجموعه با
 * `useMemo` انجام می‌شود، پس نه درخواست تازه‌ای به بک‌اند می‌رود و نه فشاری
 * به پایگاه داده وارد می‌شود.
 *
 * برای جدول‌های بزرگ‌تر از یک صفحه، مرتب‌سازی باید در SQL و با ایندکس انجام
 * شود؛ همان الگو در فاز گزارش‌ها پیاده می‌شود.
 */
export function useSort<T>(rows: T[], initialKey?: keyof T & string) {
  const [key, setKey] = useState<(keyof T & string) | undefined>(initialKey)
  const [direction, setDirection] = useState<SortDirection>('asc')

  const sorted = useMemo(() => {
    if (!key) return rows
    const factor = direction === 'asc' ? 1 : -1
    // کپی سطحی تا آرایه‌ی ورودی تغییر نکند
    return [...rows].sort((first, second) => {
      const a = first[key] as unknown
      const b = second[key] as unknown
      if (a == null && b == null) return 0
      if (a == null) return 1
      if (b == null) return -1
      if (typeof a === 'number' && typeof b === 'number') return (a - b) * factor
      if (typeof a === 'boolean' && typeof b === 'boolean') {
        return (Number(a) - Number(b)) * factor
      }
      return String(a).localeCompare(String(b), 'fa') * factor
    })
  }, [rows, key, direction])

  /** کلیک روی سربرگ: بار اول صعودی، بار دوم نزولی. */
  const toggle = (nextKey: keyof T & string) => {
    if (key === nextKey) {
      setDirection((current) => (current === 'asc' ? 'desc' : 'asc'))
    } else {
      setKey(nextKey)
      setDirection('asc')
    }
  }

  /** کلاس CSS سربرگ برای نمایش جهت مرتب‌سازی. */
  const headerClass = (columnKey: keyof T & string) =>
    `sortable${key === columnKey ? ` ${direction}` : ''}`

  /**
   * ویژگی‌های آماده برای `<th>`: کلاس جهت مرتب‌سازی، رویداد کلیک و
   * `aria-sort` برای دسترس‌پذیری. با این کار هیچ صفحه‌ای منطق مرتب‌سازی را
   * دوباره نمی‌نویسد.
   */
  const sortProps = (columnKey: keyof T & string) => ({
    className: headerClass(columnKey),
    onClick: () => toggle(columnKey),
    'aria-sort': (key === columnKey
      ? direction === 'asc'
        ? 'ascending'
        : 'descending'
      : 'none') as 'ascending' | 'descending' | 'none',
  })

  return {sorted, key, direction, toggle, headerClass, sortProps}
}
