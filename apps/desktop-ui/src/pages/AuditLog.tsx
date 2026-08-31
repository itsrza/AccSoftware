import {useCallback, useEffect, useState} from 'react'
import {RefreshCw} from 'lucide-react'
import {getAuditLogs, type AuditLogRow} from '../api'
import {errorText} from '../lib/errors'
import {useI18n} from '../lib/i18n'
import {Select} from '../components/Select'

/**
 * لاگ عملکرد کاربران — نمایشگر audit_logs.
 *
 * داده از قبل در هر عملیات حساس ثبت می‌شود (audit میزبان)؛ این صفحه آن را
 * با فیلتر (بازه، کاربر، نوع موجودیت) و سقف تعداد ردیف در دسترس مدیر
 * می‌گذارد. دستور با مجوز system.audit.view محافظت می‌شود.
 *
 * تاریخ‌ها ISO میلادی ذخیره می‌شوند (created_at)؛ فیلتر همان قالب را
 * می‌گیرد و نمایش، شمسی است.
 */
const ENTITY_TYPES = [
  'contact',
  'product',
  'invoice',
  'check',
  'treasury_account',
  'journal',
  'account_mapping',
  'api_profile',
  'return',
  'plugin',
] as const

export function AuditLog() {
  const {t} = useI18n()
  const [rows, setRows] = useState<AuditLogRow[]>([])
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [entity, setEntity] = useState('')
  const [limit, setLimit] = useState(200)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const load = useCallback(() => {
    setLoading(true)
    setError('')
    getAuditLogs(
      from || undefined,
      to || undefined,
      undefined,
      entity || undefined,
      limit,
    )
      .then(setRows)
      .catch((e) => setError(errorText(e)))
      .finally(() => setLoading(false))
  }, [from, to, entity, limit])

  useEffect(() => {
    load()
  }, [load])

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('nav.group.reports')}</div>
          <h1>{t('page.audit-log')}</h1>
          <p>{t('auditLog.subtitle')}</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}

      <div className="panel list-panel">
        <div className="toolbar">
          <label className="inline-field">
            {t('auditLog.from')}
            <input dir="ltr" placeholder="2026-01-01" value={from} onChange={(e) => setFrom(e.target.value)} />
          </label>
          <label className="inline-field">
            {t('auditLog.to')}
            <input dir="ltr" placeholder="2026-12-31" value={to} onChange={(e) => setTo(e.target.value)} />
          </label>
          <Select
            value={entity}
            aria-label={t('auditLog.entity')}
            onChange={(e) => setEntity(e.target.value)}
          >
            <option value="">{t('auditLog.allEntities')}</option>
            {ENTITY_TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`auditLog.entity.${type}`)}
              </option>
            ))}
          </Select>
          <Select
            value={String(limit)}
            aria-label={t('auditLog.limit')}
            onChange={(e) => setLimit(Number(e.target.value))}
          >
            {[100, 200, 500, 1000].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </Select>
          <button type="button" className="icon-btn" onClick={load} aria-label={t('common.refresh')}>
            <RefreshCw className="size-3.5" aria-hidden />
          </button>
        </div>

        {loading ? (
          <div className="empty-state">{t('common.loading')}</div>
        ) : rows.length === 0 ? (
          <div className="empty-state">{t('auditLog.empty')}</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>{t('auditLog.col.time')}</th>
                  <th>{t('auditLog.col.user')}</th>
                  <th>{t('auditLog.col.action')}</th>
                  <th>{t('auditLog.col.entity')}</th>
                  <th>{t('auditLog.col.ref')}</th>
                  <th>{t('auditLog.col.change')}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={row.id}>
                    <td dir="ltr" className="tnum">{row.created_at}</td>
                    <td>{row.username ?? row.user_id ?? '—'}</td>
                    <td>
                      <span className={`status ${row.action === 'delete' ? 'rejected' : 'pending'}`}>
                        {row.action}
                      </span>
                    </td>
                    <td>
                      {row.entity_type
                        ? t(`auditLog.entity.${row.entity_type}` as never) === `auditLog.entity.${row.entity_type}`
                          ? row.entity_type
                          : t(`auditLog.entity.${row.entity_type}` as never)
                        : '—'}
                    </td>
                    <td dir="ltr" className="tnum">{row.entity_id ?? '—'}</td>
                    <td className="tnum" title={row.after_json ?? row.before_json ?? ''}>
                      {(row.after_json ?? row.before_json ?? '—').slice(0, 60)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  )
}
