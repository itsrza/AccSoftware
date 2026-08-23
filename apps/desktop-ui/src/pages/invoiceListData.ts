import { useEffect, useState } from 'react'
import { getFiscalPeriodStatus } from '../api'
import { resolveRange, type JalaliRange } from '../lib/dateRange'

/**
 * کمکی مشترک صفحه‌های فهرست.
 *
 * سال مالی فعال از پایگاه داده خوانده می‌شود، نه از تقویم سیستم. دلیلش
 * ساده است: سال مالی شرکت لزوماً با سال شمسی یکی نیست، و فهرست فاکتوری که
 * پیش‌فرضش خارج از سال مالی باشد همیشه خالی به نظر می‌رسد.
 */
export function useFiscalRange(): { from: string; to: string } | undefined {
  const [range, setRange] = useState<{ from: string; to: string }>()
  useEffect(() => {
    let alive = true
    getFiscalPeriodStatus()
      .then((period) => {
        if (alive) setRange({ from: period.start_date, to: period.end_date })
      })
      .catch(() => {
        /* نبود سال مالی نباید فهرست را از کار بیندازد */
      })
    return () => {
      alive = false
    }
  }, [])
  return range
}

/** بازه‌ی پیش‌فرض فهرست‌ها: سال مالی فعال. */
export const defaultRange = (): JalaliRange => resolveRange('fiscalYear')
