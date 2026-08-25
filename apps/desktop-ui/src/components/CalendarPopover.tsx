import {useEffect, useMemo, useRef, useState} from 'react'
import {CalendarDays} from 'lucide-react'
import {getCalendarOverview, type CalendarOccasion, type CalendarOverview} from '../api'
import {errorText} from '../lib/errors'
import {formatNumber, todayJalali} from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'
import {daysInJalaliMonth, jalaliToDate} from '../lib/dateRange'

/**
 * تقویم سه‌گانه — شمسی/میلادی/قمری با مناسبت‌ها.
 *
 * مرجع: نمایش تاریخ نرم‌افزارهای حسابداری ایران. روی تاریخِ نوار بالا کلیک
 * می‌شود و پنل باز می‌شود: امروز در هر سه تقویم، مناسبت‌های امروز، تقویم
 * ماه شمسی با نقطه روی روزهای مناسبت‌دار و فهرست مناسبت‌های ماه.
 *
 * ## چرا محاسبه از میزبان می‌آید
 * تبدیل قمری و جدول مناسبت‌ها در `novin_core` زندگی می‌کند و با تست‌های
 * لنگر قفل شده است؛ رابط کاربری فقط نمایش می‌دهد. نسخه‌ی پیش‌نمایش مرورگر
 * همان الگوریتم را در `lib/hijri` آینه کرده است.
 */
export function CalendarMenu() {
  const {t} = useI18n()
  const [open, setOpen] = useState(false)
  const [data, setData] = useState<CalendarOverview>()
  const [error, setError] = useState('')
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', close)
    return () => document.removeEventListener('mousedown', close)
  }, [open])

  useEffect(() => {
    if (!open || data || error) return
    getCalendarOverview().then(setData).catch((e) => setError(errorText(e)))
  }, [open, data, error])

  const weekdayKey = (index: number) => `weekday.${index}` as TranslationKey
  const weekdayShortKey = (index: number) => `weekdayShort.${index}` as TranslationKey
  const hijriMonthKey = (month: number) => `hijriMonth.${month}` as TranslationKey
  const jalaliMonthKey = (month: number) => `month.${month}` as TranslationKey
  const pad = (value: number) => String(value).padStart(2, '0')

  const grid = useMemo(() => {
    if (!data) return null
    const {jalali_year: year, jalali_month: month, jalali_day: today} = data.today
    const days = daysInJalaliMonth(year, month)
    const startWeekday = (jalaliToDate(year, month, 1).getDay() + 1) % 7
    const byDate = new Map<string, CalendarOccasion[]>()
    for (const occasion of data.occasions) {
      const list = byDate.get(occasion.jalali) ?? []
      list.push(occasion)
      byDate.set(occasion.jalali, list)
    }
    return {year, month, today, days, startWeekday, byDate}
  }, [data])

  const today = data?.today

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        onClick={() => setOpen((state) => !state)}
        aria-label={t('calendar.open')}
        aria-expanded={open}
        className="flex items-center gap-1.5 rounded-xl border border-border bg-card px-3 py-2 text-[11px] font-semibold text-muted transition-colors hover:text-text"
      >
        <CalendarDays className="size-3.5" aria-hidden />
        <span className="whitespace-nowrap">{todayJalali()}</span>
      </button>

      {open && (
        <div
          className="fade-up absolute end-0 top-full z-30 mt-2 w-[340px] rounded-2xl border border-border bg-card p-4 shadow-[var(--shadow-lg)]"
          role="dialog"
          aria-label={t('calendar.open')}
        >
          {error ? (
            <p className="error-box">{error}</p>
          ) : !today || !grid ? (
            <p className="empty-state">{t('common.loading')}</p>
          ) : (
            <>
              {/* ------------------------------------------------ امروز */}
              <p className="text-[11px] font-bold text-faint">{t('calendar.today')}</p>
              <p className="mt-1 text-[15px] font-extrabold text-text">
                {t(weekdayKey(today.weekday))} {formatNumber(today.jalali_day)}{' '}
                {t(jalaliMonthKey(today.jalali_month))} {formatNumber(today.jalali_year)}
              </p>
              <div className="mt-2 grid grid-cols-2 gap-1.5 text-[11px]">
                <span className="rounded-lg bg-bg-soft px-2 py-1.5 text-muted">
                  <b className="text-faint">{t('calendar.lunar')}: </b>
                  {formatNumber(today.hijri_day)} {t(hijriMonthKey(today.hijri_month))}{' '}
                  {formatNumber(today.hijri_year)}
                </span>
                <span className="rounded-lg bg-bg-soft px-2 py-1.5 text-muted" dir="ltr">
                  <b className="text-faint">{t('calendar.gregorian')}: </b>
                  {today.gregorian}
                </span>
              </div>

              {today.occasions.length > 0 ? (
                <ul className="mt-2 space-y-1">
                  {today.occasions.map((occasion) => (
                    <li
                      key={occasion.title}
                      className="flex items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-[11.5px] font-semibold"
                      style={{background: 'var(--accent-soft)'}}
                    >
                      <span>{occasion.title}</span>
                      {occasion.holiday && (
                        <span className="status neutral">{t('calendar.holiday')}</span>
                      )}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="mt-2 text-[11px] text-faint">{t('calendar.todayNone')}</p>
              )}

              {/* --------------------------------------- تقویم ماه شمسی */}
              <div className="mt-3 grid grid-cols-7 gap-1 text-center">
                {Array.from({length: 7}, (_, index) => (
                  <span key={index} className="text-[9.5px] font-bold text-faint">
                    {t(weekdayShortKey(index))}
                  </span>
                ))}
                {Array.from({length: grid.startWeekday}, (_, index) => (
                  <span key={`pad-${index}`} />
                ))}
                {Array.from({length: grid.days}, (_, index) => {
                  const day = index + 1
                  const key = `${grid.year}/${pad(grid.month)}/${pad(day)}`
                  const occasions = grid.byDate.get(key)
                  const isToday = day === grid.today
                  return (
                    <span
                      key={day}
                      className={`relative mx-auto grid size-7 place-items-center rounded-lg text-[11px] font-semibold ${
                        isToday ? 'bg-[var(--primary)] text-[var(--on-primary)]' : 'text-muted'
                      }`}
                      title={occasions?.map((occasion) => occasion.title).join(' — ')}
                    >
                      {formatNumber(day)}
                      {occasions && (
                        <span
                          className="absolute bottom-0.5 size-1 rounded-full bg-accent-strong"
                          aria-hidden
                        />
                      )}
                    </span>
                  )
                })}
              </div>

              {/* -------------------------------------- مناسبت‌های ماه */}
              <p className="mt-3 text-[11px] font-bold text-faint">{t('calendar.occasions')}</p>
              {data.occasions.length === 0 ? (
                <p className="mt-1 text-[11px] text-faint">{t('calendar.monthNone')}</p>
              ) : (
                <ul className="mt-1 max-h-40 space-y-1 overflow-y-auto">
                  {data.occasions.map((occasion, index) => (
                    <li
                      key={`${occasion.date}-${index}`}
                      className="flex items-center justify-between gap-2 border-b border-border pb-1 text-[11px] last:border-0"
                    >
                      <span className="tnum text-faint" dir="ltr">
                        {occasion.jalali}
                      </span>
                      <span className="min-w-0 flex-1 truncate text-end text-muted">
                        {occasion.title}
                      </span>
                      {occasion.holiday && (
                        <span className="status neutral">{t('calendar.holiday')}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}

              <p className="mt-2 text-[9.5px] leading-5 text-faint">{t('calendar.lunarNote')}</p>
            </>
          )}
        </div>
      )}
    </div>
  )
}
