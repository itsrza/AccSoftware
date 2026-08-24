import { useEffect, useState } from 'react'
import { CalendarRange, Check, RotateCcw, Search } from 'lucide-react'
import { cn } from '../lib/cn'
import { Popover, Select } from './ui'
import {
  PRESETS,
  parseJalali,
  resolveRange,
  type JalaliRange,
  type PresetId,
} from '../lib/dateRange'
import { useI18n, type TranslationKey } from '../lib/i18n'

/**
 * نوار فیلتر سراسری — منطبق با سیستم طراحی مرجع.
 *
 * ## چرا یک جزء مشترک
 * در مرجع، هم داشبورد و هم صفحه‌های ماژول از یک نوار فیلتر استفاده می‌کنند.
 * یکسان بودن آن یعنی کاربر یک بار یاد می‌گیرد و همه‌جا همان رفتار را
 * می‌بیند: پیش‌تنظیم‌های تاریخ به‌صورت قرص، بازه‌ی سفارشی در یک پاپ‌اور، و
 * دکمه‌ی بازنشانی که وقتی چیزی تغییر نکرده غیرفعال است.
 *
 * ## قاعده‌ی «هیچ فیلتر تزئینی»
 * فیلترهای بُعدی (`filters`) را صفحه‌ی میزبان تعیین می‌کند و فقط وقتی
 * فرستاده می‌شوند که واقعاً روی داده اثر بگذارند. نوار فیلتری که یک
 * دراپ‌داون بی‌اثر داشته باشد، بدتر از نداشتن آن است.
 */

export type BarFilter = {
  key: string
  label: string
  value: string
  options: { value: string; label: string }[]
  onChange: (value: string) => void
  /** عرض دلخواه در نمایشگر بزرگ. */
  width?: string
}

export function FilterBar({
  range,
  onRange,
  filters = [],
  search,
  onSearch,
  searchPlaceholder,
  onReset,
  isDefault,
  note,
  fiscalRange,
}: {
  range: JalaliRange
  onRange: (range: JalaliRange) => void
  /** بازه‌ی سال مالی فعال — از پایگاه داده می‌آید، نه از تقویم. */
  fiscalRange?: { from: string; to: string }
  filters?: BarFilter[]
  search?: string
  onSearch?: (value: string) => void
  searchPlaceholder?: string
  onReset: () => void
  /** آیا همه‌چیز روی حالت پیش‌فرض است؟ دکمه‌ی بازنشانی با آن غیرفعال می‌شود. */
  isDefault: boolean
  /** توضیح کوتاه سمت چپ، مثلاً تعداد رکورد یافت‌شده. */
  note?: string
}) {
  const { t } = useI18n()
  /** برچسب هر پیش‌تنظیم از همان شناسه‌اش مشتق می‌شود، پس افزودن زبان تازه
   * فهرست پیش‌تنظیم‌ها را دست‌نخورده می‌گذارد. */
  const presetLabel = (id: string) => t(`filter.preset.${id}` as TranslationKey)
  const placeholder = searchPlaceholder ?? t('filter.searchDefault')
  const [customFrom, setCustomFrom] = useState(range.from)
  const [customTo, setCustomTo] = useState(range.to)
  const [customError, setCustomError] = useState('')

  // وقتی پیش‌تنظیم از جای دیگری عوض شود، ورودی‌های سفارشی هم هم‌گام می‌مانند.
  useEffect(() => {
    setCustomFrom(range.from)
    setCustomTo(range.to)
  }, [range.from, range.to])

  const applyCustom = (close: () => void) => {
    const from = parseJalali(customFrom)
    const to = parseJalali(customTo)
    if (!from || !to) {
      setCustomError(t('filter.errFormat'))
      return
    }
    if (customFrom > customTo) {
      setCustomError(t('filter.errOrder'))
      return
    }
    setCustomError('')
    onRange(resolveRange('custom', { from: customFrom, to: customTo }))
    close()
  }

  return (
    <section
      aria-label={t('filter.globalFilters')}
      className="fade-up rounded-[var(--radius)] border border-border bg-card p-3 shadow-[var(--shadow-sm)] sm:p-4"
    >
      {/* --- پیش‌تنظیم‌های تاریخ --- */}
      <div className="flex items-center gap-2 overflow-x-auto pb-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        <span className="inline-flex shrink-0 items-center gap-1.5 text-[11px] font-bold text-muted">
          <CalendarRange className="size-4 text-accent" aria-hidden />
          {t('filter.label')}
        </span>

        {PRESETS.map((preset) => (
          <button
            key={preset.id}
            onClick={() =>
              onRange(
                resolveRange(
                  preset.id as PresetId,
                  preset.id === 'fiscalYear' ? fiscalRange : undefined,
                ),
              )
            }
            aria-pressed={range.preset === preset.id}
            className={cn(
              'shrink-0 rounded-full border px-3 py-1.5 text-[11px] font-semibold transition-all',
              range.preset === preset.id
                ? 'border-primary bg-primary text-[var(--on-primary)] shadow-[var(--shadow-sm)]'
                : 'border-border bg-card text-muted hover:border-border-strong hover:text-text',
            )}
          >
            {presetLabel(preset.id)}
          </button>
        ))}

        <Popover
          label={t('filter.custom')}
          width="w-72"
          align="end"
          trigger={() => (
            <span
              className={cn(
                'inline-flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full border px-3 py-1.5 text-[11px] font-semibold transition-all',
                range.preset === 'custom'
                  ? 'border-primary bg-primary text-[var(--on-primary)]'
                  : 'border-border bg-card text-muted hover:border-border-strong hover:text-text',
              )}
            >
              {t('filter.custom')}
            </span>
          )}
        >
          {(close) => (
            <div className="p-2">
              <p className="pb-2 text-[11px] font-bold text-text">{t('filter.pickCustom')}</p>
              <div className="grid grid-cols-2 gap-2">
                <label className="block">
                  <span className="mb-1 block text-[10px] text-muted">{t('filter.from')}</span>
                  <input
                    value={customFrom}
                    onChange={(event) => setCustomFrom(event.target.value)}
                    placeholder="1405/01/01"
                    inputMode="numeric"
                    className="h-9 w-full rounded-lg border border-border bg-card px-2 text-[11px] text-text outline-none focus:border-accent"
                  />
                </label>
                <label className="block">
                  <span className="mb-1 block text-[10px] text-muted">{t('filter.to')}</span>
                  <input
                    value={customTo}
                    onChange={(event) => setCustomTo(event.target.value)}
                    placeholder="1405/12/29"
                    inputMode="numeric"
                    className="h-9 w-full rounded-lg border border-border bg-card px-2 text-[11px] text-text outline-none focus:border-accent"
                  />
                </label>
              </div>
              {customError && (
                <p className="mt-2 text-[10.5px] font-semibold text-danger">{customError}</p>
              )}
              <button
                onClick={() => applyCustom(close)}
                className="mt-3 inline-flex w-full items-center justify-center gap-1.5 rounded-xl bg-primary py-2 text-xs font-bold text-[var(--on-primary)] transition-transform hover:scale-[1.01] active:scale-95"
              >
                <Check className="size-3.5" aria-hidden />
                {t('filter.applyRange')}
              </button>
            </div>
          )}
        </Popover>
      </div>

      {/* --- فیلترهای بُعدی --- */}
      <div className="mt-3 grid grid-cols-1 gap-2 sm:grid-cols-2 xl:flex xl:items-center">
        {filters.map((filter) => (
          <label key={filter.key} className={cn('block min-w-0', filter.width)}>
            <span className="sr-only">{filter.label}</span>
            <Select
              value={filter.value}
              aria-label={filter.label}
              onChange={(event) => filter.onChange(event.target.value)}
            >
              {filter.options.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
          </label>
        ))}

        {onSearch && (
          <label className="relative block min-w-0 xl:w-56">
            <span className="sr-only">{placeholder}</span>
            <Search
              className="pointer-events-none absolute top-1/2 start-3 size-3.5 -translate-y-1/2 text-faint"
              aria-hidden
            />
            <input
              value={search ?? ''}
              onChange={(event) => onSearch(event.target.value)}
              placeholder={placeholder}
              className="h-[38px] w-full rounded-xl border border-border bg-card ps-9 pe-3 text-[12.5px] text-text outline-none transition-colors hover:border-border-strong focus:border-accent"
            />
          </label>
        )}

        <div className="flex items-center justify-between gap-2 xl:ms-auto xl:w-auto">
          <p className="text-[10.5px] text-faint">
            {presetLabel(range.preset)} · {range.from} {t('filter.rangeJoin')} {range.to}
            {note ? ` · ${note}` : ''}
          </p>
          <button
            onClick={onReset}
            disabled={isDefault}
            className={cn(
              'inline-flex h-9 shrink-0 items-center gap-1.5 rounded-xl border px-3 text-[11px] font-bold transition-all',
              isDefault
                ? 'cursor-not-allowed border-border text-faint opacity-50'
                : 'border-border text-muted hover:border-border-strong hover:text-text active:scale-95',
            )}
          >
            <RotateCcw className="size-3.5" aria-hidden />
            {t('common.reset')}
          </button>
        </div>
      </div>
    </section>
  )
}
