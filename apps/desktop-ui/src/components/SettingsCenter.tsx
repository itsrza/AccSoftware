import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from './Icon'
import {
  closeFiscalYear,
  deleteDemo,
  backupDatabase,
  getAccountMappings,
  getAccounts,
  listBackups,
  restoreDatabase,
  verifyBackupFile,
  getSettings,
  resetSetting,
  setAccountMapping,
  setSetting,
  SettingWithValue,
} from '../api'
import { errorText } from '../lib/errors'
import {Select} from './Select'
import { useI18n, type Locale } from '../lib/i18n'

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
  const { t, setLocale } = useI18n()
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
      // تم و زبان باید فوراً اعمال شوند تا کاربر نتیجه را ببیند؛ تنظیمی که
      // اثرش تا اجرای بعدی دیده نشود، برای کاربر «کار نکرد» معنی می‌دهد.
      if (item.key === 'appearance.dark_mode') setDark(saved === 'true')
      if (item.key === 'appearance.language') setLocale(saved as Locale)
      setNotice(t('settingsCenter.savedToast', { name: item.label }))
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
    if (!confirm(t('demo.deleteConfirm'))) return
    setDemoBusy(true)
    try {
      await deleteDemo()
      setNotice(t('settingsCenter.demoDeleted'))
    } catch (e) {
      setError(errorText(e))
    } finally {
      setDemoBusy(false)
    }
  }

  const closeYear = async () => {
    if (!confirm(t('settingsCenter.confirmClose'))) return
    setFiscalBusy(true)
    try {
      await closeFiscalYear()
      setNotice(t('settingsCenter.yearClosed'))
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
            <span>{item.value === 'true' ? t('settingsCenter.enabled') : t('settingsCenter.disabled')}</span>
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
                {t('settingsCenter.noImage')}
              </span>
            )}
            <label className="table-action cursor-pointer">
              {t('settingsCenter.pickImage')}
              <input
                type="file"
                accept="image/png,image/jpeg,image/svg+xml"
                className="hidden"
                disabled={disabled}
                onChange={(e) => {
                  const file = e.target.files?.[0]
                  if (!file) return
                  if (file.size > 900_000) {
                    setError(t('settingsCenter.imageTooBig'))
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
                {t('partyForm.remove')}
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
          <button className="icon-btn" onClick={onClose} aria-label={t('common.close')}>
            <Icon name="close" />
          </button>
          <div>
            <b>{t('page.settings')}</b>
            <span>
              {settings.length} تنظیم — {changedCount} مورد تغییر یافته
            </span>
          </div>
        </div>

        <div className="settings-search">
          <input
            placeholder={t('settings.searchPlaceholder')}
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
            <b>{t('settingsCenter.advancedTools')}</b>
            <small>{t('settingsCenter.printAndApi')}</small>
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
            <b>{t('settingsCenter.adminActions')}</b>
            <small>{t('settingsCenter.demoAndYear')}</small>
          </span>
          <Icon name="chevron" size={14} />
        </button>
      </aside>

      <section className="settings-content">
        <header>
          <div>
            <div className="eyebrow">{t('settingsCenter.settings')}</div>
            <h1>
              {search
                ? `نتیجه‌ی جستجو (${visible.length})`
                : activeGroup === '__actions'
                  ? t('settingsCenter.adminActions')
                  : activeGroup === '__tools'
                    ? t('settingsCenter.advancedTools')
                    : (groups.find((g) => g.group === activeGroup)?.label ?? '')}
            </h1>
            <p>{t('settings.subtitle')}</p>
          </div>
          <div className="filter-actions">
            <button className="ghost" onClick={() => setDark(!dark)}>
              <Icon name={dark ? 'sun' : 'moon'} /> {dark ? t('topbar.lightTheme') : t('topbar.darkTheme')}
            </button>
            <button className="icon-btn" onClick={onClose} aria-label={t('common.close')}>
              <Icon name="close" />
            </button>
          </div>
        </header>

        {error && <div className="error-box">{error}</div>}
        {notice && <div className="success-box">{notice}</div>}

        {/* نگاشت حساب‌های پیش‌فرض — هیچ سند خودکاری نباید حساب ثابت داشته باشد.
            * مقادیر از account_mappings میزبان می‌آید و بلافاصله ذخیره می‌شود. */}
        <AccountMappingsPanel onError={setError} />
        <BackupPanel onError={setError} />

        {activeGroup === '__tools' && !search ? (
          <div className="settings-stack">
            <div className="setting-row">
              <div className="setting-info">
                <b>{t('page.print-templates')}</b>
                <span>
                  {t('settingsCenter.printDesc')}
                </span>
                <small className="effect">{t('settingsCenter.printEffect')}</small>
              </div>
              <button className="primary" onClick={() => navigate('print-templates')}>
                {t('settingsCenter.open')}
              </button>
            </div>
            <div className="setting-row">
              <div className="setting-info">
                <b>{t('page.integrations')}</b>
                <span>
                  {t('settingsCenter.apiDesc')}
                </span>
                <small className="effect">{t('settingsCenter.apiEffect')}</small>
              </div>
              <button className="primary" onClick={() => navigate('integrations')}>
                {t('settingsCenter.open')}
              </button>
            </div>
          </div>
        ) : activeGroup === '__actions' && !search ? (
          <div className="settings-stack">
            <div className="setting-row danger-card">
              <div className="setting-info">
                <b>{t('settingsCenter.deleteDemo')}</b>
                <span>
                  {t('settingsCenter.demoDesc')}
                </span>
                <small className="effect">{t('settingsCenter.demoEffect')}</small>
              </div>
              <button className="danger" disabled={demoBusy} onClick={removeDemo}>
                {demoBusy ? t('settingsCenter.working') : t('settingsCenter.deleteDemo')}
              </button>
            </div>
            <div className="setting-row">
              <div className="setting-info">
                <b>{t('settingsCenter.closeYear')}</b>
                <span>
                  {t('settingsCenter.closeDesc')}
                </span>
                <small className="effect">{t('settingsCenter.closeEffect')}</small>
              </div>
              <button className="ghost" disabled={fiscalBusy} onClick={closeYear}>
                {fiscalBusy ? t('settingsCenter.checking') : t('settingsCenter.closeYear')}
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
                    {item.sensitive && <span className="chip">{t('settingsCenter.adminPermission')}</span>}
                    {item.is_customized && <span className="chip">{t('settingsCenter.changed')}</span>}
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
                      {t('settingsCenter.restore')}
                    </button>
                  )}
                </div>
              </div>
            ))}
            {visible.length === 0 && (
              <div className="empty-state">{t('settingsCenter.noMatch')}</div>
            )}
          </div>
        )}
      </section>
    </div>
  )
}


/** کلیدهای نگاشت — برچسب فارسی از i18n می‌آید (mapping.<key>). */
const MAPPING_KEYS = [
  'cash_default',
  'ar_default',
  'ap_default',
  'sales_revenue_default',
  'cogs_default',
  'sales_return_default',
  'purchase_return_default',
  'tax_payable_default',
  'tax_receivable_default',
  'sales_discount_default',
  'purchase_discount_default',
  'check_bounce_tracking_default',
] as const

function AccountMappingsPanel({ onError }: { onError: (message: string) => void }) {
  const { t } = useI18n()
  const [mappings, setMappings] = useState<{ mapping_key: string; account_id: string }[]>([])
  const [accounts, setAccounts] = useState<{ id: string; code: string; name: string }[]>([])
  const [busy, setBusy] = useState('')
  const [ok, setOk] = useState('')

  useEffect(() => {
    let alive = true
    Promise.all([getAccountMappings(), getAccounts()])
      .then(([map, acc]) => {
        if (!alive) return
        setMappings(map)
        setAccounts(acc)
      })
      .catch((e) => alive && onError(errorText(e)))
    return () => {
      alive = false
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const commit = async (key: string, accountId: string) => {
    setBusy(key)
    setOk('')
    try {
      await setAccountMapping(key, accountId)
      setMappings((current) =>
        current.map((row) => (row.mapping_key === key ? { ...row, account_id: accountId } : row)),
      )
      setOk(key)
    } catch (e) {
      onError(errorText(e))
    } finally {
      setBusy('')
    }
  }

  const label = (key: string) => t(`mapping.${key}` as never)

  return (
    <section className="panel list-panel" style={{ marginBottom: 18 }}>
      <div className="repeat-head">
        <h3 className="section-title">{t('settingsCenter.accountMappings')}</h3>
      </div>
      <p className="hint">{t('settingsCenter.accountMappingsHint')}</p>
      <div className="line-editor">
        {mappings.map((row) => {
          const missing = accounts.length > 0 && !accounts.some((a) => a.id === row.account_id)
          return (
            <div className="line-row" key={row.mapping_key}>
              <label className="grow">
                {label(row.mapping_key)}
                <Select
                  value={row.account_id}
                  aria-label={label(row.mapping_key)}
                  disabled={busy === row.mapping_key}
                  onChange={(event) => void commit(row.mapping_key, event.target.value)}
                >
                  <option value="">{t('settingsCenter.mappingUnset')}</option>
                  {accounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.code} — {account.name}
                    </option>
                  ))}
                </Select>
              </label>
              {ok === row.mapping_key && <span className="status passed">✓</span>}
              {missing && <span className="status neutral">{t('settingsCenter.mappingMissing')}</span>}
            </div>
          )
        })}
        {mappings.length === 0 && <p className="empty-state">{t('common.loading')}</p>}
      </div>
    </section>
  )
}


function BackupPanel({ onError }: { onError: (message: string) => void }) {
  const { t } = useI18n()
  const [items, setItems] = useState<{ name: string; size_bytes: number }[]>([])
  const [busy, setBusy] = useState('')
  const [confirming, setConfirming] = useState('')
  const [ok, setOk] = useState('')

  const load = () => {
    listBackups()
      .then(setItems)
      .catch(() => setItems([]))
  }
  useEffect(load, [])

  const create = async () => {
    setBusy('create')
    setOk('')
    try {
      await backupDatabase()
      setOk('create')
      load()
    } catch (e) {
      onError(errorText(e))
    } finally {
      setBusy('')
    }
  }

  const verify = async (name: string) => {
    setBusy(name)
    setOk('')
    try {
      await verifyBackupFile(name)
      setOk(name)
    } catch (e) {
      onError(errorText(e))
    } finally {
      setBusy('')
    }
  }

  const restore = async (name: string) => {
    if (confirming !== name) {
      setConfirming(name)
      return
    }
    setBusy(name)
    setConfirming('')
    try {
      await restoreDatabase(name)
      setOk(name)
    } catch (e) {
      onError(errorText(e))
    } finally {
      setBusy('')
    }
  }

  const mb = (bytes: number) => `${(bytes / 1_048_576).toFixed(1)} MB`

  return (
    <section className="panel list-panel" style={{ marginBottom: 18 }}>
      <div className="repeat-head">
        <h3 className="section-title">{t('settingsCenter.backup')}</h3>
        <button type="button" className="table-action" disabled={busy === 'create'} onClick={create}>
          {busy === 'create' ? t('common.loading') : t('settingsCenter.backupCreate')}
        </button>
      </div>
      <p className="hint">{t('settingsCenter.backupHint')}</p>
      <div className="line-editor">
        {items.map((item) => (
          <div className="line-row" key={item.name}>
            <span className="grow tnum" dir="ltr">{item.name}</span>
            <span className="tnum text-faint">{t('settingsCenter.backupSize')}: {mb(item.size_bytes)}</span>
            <button type="button" className="table-action" disabled={busy === item.name} onClick={() => void verify(item.name)}>
              {ok === item.name ? '✓' : t('settingsCenter.backupVerify')}
            </button>
            <button
              type="button"
              className={confirming === item.name ? 'table-action danger-icon' : 'table-action'}
              disabled={busy === item.name}
              onClick={() => void restore(item.name)}
            >
              {confirming === item.name ? t('settingsCenter.backupConfirm') : t('settingsCenter.backupRestore')}
            </button>
          </div>
        ))}
        {items.length === 0 && <p className="empty-state">{t('settingsCenter.backupEmpty')}</p>}
      </div>
    </section>
  )
}
