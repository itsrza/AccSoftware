import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from './Icon'
import {
  closeFiscalYear,
  deleteDemo,
  getSettings,
  resetSetting,
  setSetting,
  SettingWithValue,
} from '../api'
import { errorText } from '../lib/errors'
import {Select} from './Select'

/**
 * مرکز تنظیمات.
 *
 * ## قاعده‌ی این صفحه: هیچ تنظیم تزئینی
 *
 * فهرست تنظیمات از backend می‌آید و **هر تنظیم می‌گوید دقیقاً کجا اثر
 * می‌گذارد**. تنظیمی که هیچ اثری ندارد بدتر از نبودنش است: کاربر عوضش
 * می‌کند، انتظار تغییر رفتار دارد و چیزی عوض نمی‌شود.
 *
 * اعتبارسنجی هم سمت backend انجام می‌شود — چون تنظیم خراب می‌تواند محاسبه‌ی
 * مالی را خراب کند (مثلاً نرخ مالیات منفی).
 */
export function SettingsCenter({
  onClose,
  dark,
  setDark,
  navigate,
}: {
  onClose: () => void
  dark: boolean
  setDark: (value: boolean) => void
  /** رفتن به صفحه‌ی ابزارهای راه‌اندازی که از منوی کناری برداشته شده‌اند. */
  navigate: (page: string) => void
}) {
  const [settings, setSettings] = useState<SettingWithValue[]>([])
  const [activeGroup, setActiveGroup] = useState('')
  const [search, setSearch] = useState('')
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busyKey, setBusyKey] = useState('')
  const [demoBusy, setDemoBusy] = useState(false)
  const [fiscalBusy, setFiscalBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const list = await getSettings()
      setSettings(list)
      setActiveGroup((current) => current || list[0]?.group || '')
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const groups = useMemo(() => {
    const map = new Map<string, { group: string; label: string; count: number; changed: number }>()
    for (const item of settings) {
      const entry = map.get(item.group) ?? {
        group: item.group,
        label: item.group_label,
        count: 0,
        changed: 0,
      }
      entry.count += 1
      if (item.is_customized) entry.changed += 1
      map.set(item.group, entry)
    }
    return [...map.values()]
  }, [settings])

  const visible = useMemo(() => {
    const needle = search.trim()
    if (needle) {
      return settings.filter(
        (item) =>
          item.label.includes(needle) ||
          item.description.includes(needle) ||
          item.effect.includes(needle),
      )
    }
    return settings.filter((item) => item.group === activeGroup)
  }, [settings, activeGroup, search])

  const apply = async (item: SettingWithValue, value: string) => {
    setBusyKey(item.key)
    setNotice('')
    try {
      const saved = await setSetting(item.key, value)
      setSettings((current) =>
        current.map((row) =>
          row.key === item.key
            ? { ...row, value: saved, is_customized: saved !== row.default_value }
            : row,
        ),
      )
      // تم باید فوراً اعمال شود تا کاربر نتیجه را ببیند.
      if (item.key === 'appearance.dark_mode') setDark(saved === 'true')
      setNotice(`«${item.label}» ذخیره شد.`)
      setError('')
    } catch (e) {
      setError(errorText(e))
      // مقدار نمایش‌داده‌شده باید با پایگاه داده بخواند، پس دوباره می‌خوانیم.
      await load()
    } finally {
      setBusyKey('')
    }
  }

  const reset = async (item: SettingWithValue) => {
    setBusyKey(item.key)
    try {
      const saved = await resetSetting(item.key)
      setSettings((current) =>
        current.map((row) =>
          row.key === item.key ? { ...row, value: saved, is_customized: false } : row,
        ),
      )
      if (item.key === 'appearance.dark_mode') setDark(saved === 'true')
      setNotice(`«${item.label}» به پیش‌فرض برگشت.`)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusyKey('')
    }
  }

  const removeDemo = async () => {
    if (!confirm('تمام داده‌های نمونه حذف می‌شوند. ادامه می‌دهید؟')) return
    setDemoBusy(true)
    try {
      await deleteDemo()
      setNotice('داده‌های نمونه حذف شد.')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setDemoBusy(false)
    }
  }

  const closeYear = async () => {
    if (!confirm('بستن سال مالی برگشت‌پذیر نیست. ادامه می‌دهید؟')) return
    setFiscalBusy(true)
    try {
      await closeFiscalYear()
      setNotice('سال مالی بسته شد.')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setFiscalBusy(false)
    }
  }

  const renderControl = (item: SettingWithValue) => {
    const disabled = busyKey === item.key
    switch (item.kind) {
      case 'boolean':
        return (
          <label className="switch">
            <input
              type="checkbox"
              disabled={disabled}
              checked={item.value === 'true'}
              onChange={(e) => apply(item, e.target.checked ? 'true' : 'false')}
            />
            <span>{item.value === 'true' ? 'فعال' : 'غیرفعال'}</span>
          </label>
        )
      case 'choice':
        return (
          <Select
            disabled={disabled}
            value={item.value}
            onChange={(e) => apply(item, e.target.value)}
          >
            {(item.choices ?? []).map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </Select>
        )
      case 'image':
        return (
          <div className="flex items-center gap-2">
            {item.value ? (
              <img
                src={item.value}
                alt=""
                className="h-10 w-16 rounded-lg border border-border bg-card object-contain"
              />
            ) : (
              <span className="grid h-10 w-16 place-items-center rounded-lg border border-dashed border-border-strong text-[10px] text-faint">
                بدون تصویر
              </span>
            )}
            <label className="table-action cursor-pointer">
              انتخاب تصویر
              <input
                type="file"
                accept="image/png,image/jpeg,image/svg+xml"
                className="hidden"
                disabled={disabled}
                onChange={(e) => {
                  const file = e.target.files?.[0]
                  if (!file) return
                  if (file.size > 900_000) {
                    setError('حجم تصویر باید کمتر از حدود ۹۰۰ کیلوبایت باشد.')
                    return
                  }
                  const reader = new FileReader()
                  reader.onload = () => apply(item, String(reader.result ?? ''))
                  reader.readAsDataURL(file)
                }}
              />
            </label>
            {item.value && (
              <button type="button" className="table-action" onClick={() => apply(item, '')}>
                حذف
              </button>
            )}
          </div>
        )
      case 'integer':
        return (
          <input
            type="number"
            disabled={disabled}
            min={item.min}
            max={item.max}
            defaultValue={item.value}
            onBlur={(e) => {
              if (e.target.value !== item.value) apply(item, e.target.value)
            }}
          />
        )
      default:
        return (
          <input
            disabled={disabled}
            defaultValue={item.value}
            onBlur={(e) => {
              if (e.target.value !== item.value) apply(item, e.target.value)
            }}
          />
        )
    }
  }

  const changedCount = settings.filter((item) => item.is_customized).length

  return (
    <div className="settings-overlay">
      <aside className="settings-nav">
        <div className="settings-brand">
          <button className="icon-btn" onClick={onClose} aria-label="بستن">
            <Icon name="close" />
          </button>
          <div>
            <b>مرکز تنظیمات</b>
            <span>
              {settings.length} تنظیم — {changedCount} مورد تغییر یافته
            </span>
          </div>
        </div>

        <div className="settings-search">
          <input
            placeholder="جستجو در تنظیمات…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        {groups.map((group) => (
          <button
            key={group.group}
            className={activeGroup === group.group && !search ? 'setting-nav active' : 'setting-nav'}
            onClick={() => {
              setSearch('')
              setActiveGroup(group.group)
            }}
          >
            <span>
              <b>{group.label}</b>
              <small>
                {group.count} تنظیم
                {group.changed > 0 ? ` — ${group.changed} تغییر یافته` : ''}
              </small>
            </span>
            <Icon name="chevron" size={14} />
          </button>
        ))}

        <button
          className={activeGroup === '__tools' && !search ? 'setting-nav active' : 'setting-nav'}
          onClick={() => {
            setSearch('')
            setActiveGroup('__tools')
          }}
        >
          <span>
            <b>ابزارهای پیشرفته</b>
            <small>قالب چاپ و اتصالات</small>
          </span>
          <Icon name="chevron" size={14} />
        </button>

        <button
          className={activeGroup === '__actions' && !search ? 'setting-nav active' : 'setting-nav'}
          onClick={() => {
            setSearch('')
            setActiveGroup('__actions')
          }}
        >
          <span>
            <b>عملیات مدیریتی</b>
            <small>داده‌ی نمونه و سال مالی</small>
          </span>
          <Icon name="chevron" size={14} />
        </button>
      </aside>

      <section className="settings-content">
        <header>
          <div>
            <div className="eyebrow">تنظیمات</div>
            <h1>
              {search
                ? `نتیجه‌ی جستجو (${visible.length})`
                : activeGroup === '__actions'
                  ? 'عملیات مدیریتی'
                  : activeGroup === '__tools'
                    ? 'ابزارهای پیشرفته'
                    : (groups.find((g) => g.group === activeGroup)?.label ?? '')}
            </h1>
            <p>هر تنظیم می‌گوید دقیقاً کجای برنامه اثر می‌گذارد.</p>
          </div>
          <div className="filter-actions">
            <button className="ghost" onClick={() => setDark(!dark)}>
              <Icon name={dark ? 'sun' : 'moon'} /> {dark ? 'تم روشن' : 'تم تاریک'}
            </button>
            <button className="icon-btn" onClick={onClose} aria-label="بستن">
              <Icon name="close" />
            </button>
          </div>
        </header>

        {error && <div className="error-box">{error}</div>}
        {notice && <div className="success-box">{notice}</div>}

        {activeGroup === '__tools' && !search ? (
          <div className="settings-stack">
            <div className="setting-row">
              <div className="setting-info">
                <b>قالب‌های چاپ</b>
                <span>
                  قالب فاکتور، رسید، سند و برچسب. یک‌بار تنظیم می‌شود و در همه‌ی چاپ‌های برنامه
                  استفاده می‌شود.
                </span>
                <small className="effect">اثر: خروجی چاپ فاکتور، رسید خزانه و سند حسابداری</small>
              </div>
              <button className="primary" onClick={() => navigate('print-templates')}>
                باز کردن
              </button>
            </div>
            <div className="setting-row">
              <div className="setting-info">
                <b>اتصالات و افزونه‌ها</b>
                <span>
                  تعریف اتصال‌های API و فعال/غیرفعال کردن افزونه‌ها با کنترل دسترسی دامنه.
                </span>
                <small className="effect">اثر: سرویس‌های بیرونی و Native Workerها</small>
              </div>
              <button className="primary" onClick={() => navigate('integrations')}>
                باز کردن
              </button>
            </div>
          </div>
        ) : activeGroup === '__actions' && !search ? (
          <div className="settings-stack">
            <div className="setting-row danger-card">
              <div className="setting-info">
                <b>حذف داده‌های نمونه</b>
                <span>
                  فقط رکوردهایی که با پیشوند نمونه ساخته شده‌اند حذف می‌شوند. داده‌ی واقعی شما
                  دست نمی‌خورد.
                </span>
                <small className="effect">اثر: پاک‌سازی کالاها، اشخاص، فاکتورها و اسناد نمونه</small>
              </div>
              <button className="danger" disabled={demoBusy} onClick={removeDemo}>
                {demoBusy ? 'در حال انجام…' : 'حذف داده‌های نمونه'}
              </button>
            </div>
            <div className="setting-row">
              <div className="setting-info">
                <b>بستن سال مالی</b>
                <span>
                  پس از بستن، هیچ سندی با تاریخ داخل این سال مالی ثبت نمی‌شود. این عمل
                  برگشت‌پذیر نیست.
                </span>
                <small className="effect">اثر: اعتبارسنجی تاریخ در همه‌ی فرم‌های مالی</small>
              </div>
              <button className="ghost" disabled={fiscalBusy} onClick={closeYear}>
                {fiscalBusy ? 'در حال بررسی…' : 'بستن سال مالی'}
              </button>
            </div>
          </div>
        ) : (
          <div className="settings-stack">
            {visible.map((item) => (
              <div className="setting-row" key={item.key}>
                <div className="setting-info">
                  <b>
                    {item.label}
                    {item.sensitive && <span className="chip">نیازمند مجوز مدیریتی</span>}
                    {item.is_customized && <span className="chip">تغییر یافته</span>}
                  </b>
                  <span>{item.description}</span>
                  <small className="effect">اثر: {item.effect}</small>
                </div>
                <div className="setting-control">
                  {renderControl(item)}
                  {item.is_customized && (
                    <button
                      className="table-action"
                      disabled={busyKey === item.key}
                      onClick={() => reset(item)}
                    >
                      بازگردانی
                    </button>
                  )}
                </div>
              </div>
            ))}
            {visible.length === 0 && (
              <div className="empty-state">تنظیمی با این جستجو یافت نشد.</div>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
