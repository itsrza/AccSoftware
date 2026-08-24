import { useCallback, useEffect, useMemo, useState } from 'react'
import { Icon } from '../components/Icon'
import {
  auditCodingHealth,
  deactivateAccount,
  getAccountTree,
  getCodingScheme,
  getSubsidiaryGroups,
  saveAccount,
  suggestAccountCode,
  AccountNodeRow,
  CodingIssue,
  CodingSchemeInfo,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money, formatCount} from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'
import {Select} from '../components/Select'

type Draft = {
  id?: string
  code: string
  name: string
  nature: string
  parent_id?: string
  requires_subsidiary: boolean
  subsidiary_group_id?: string
  is_active: boolean
}

const NATURES: { value: string; labelKey: TranslationKey }[] = [
  { value: 'debit', labelKey: 'reports.debit' },
  { value: 'credit', labelKey: 'reports.credit' },
  { value: 'mixed', labelKey: 'coa.both' },
]

/**
 * درخت کدینگ حساب‌ها.
 *
 * سه قاعده‌ی حسابداری که این صفحه نشان می‌دهد و اجرا می‌کند:
 * ۱. فقط برگ‌های درخت قابل ثبت سند مستقیم‌اند.
 * ۲. مانده‌ی هر شاخه جمع فرزندانش است.
 * ۳. ماهیت فرزند باید با والد بخواند.
 */
export function ChartOfAccounts() {
  const {t} = useI18n()
  const [rows, setRows] = useState<AccountNodeRow[]>([])
  const [scheme, setScheme] = useState<CodingSchemeInfo>()
  const [issues, setIssues] = useState<CodingIssue[]>([])
  const [groups, setGroups] = useState<{ id: string; title: string }[]>([])
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [search, setSearch] = useState('')
  const [showInactive, setShowInactive] = useState(false)
  const [draft, setDraft] = useState<Draft | null>(null)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      const tree = await getAccountTree(showInactive)
      setRows(tree)
      // شاخه‌های سطح اول به‌طور پیش‌فرض باز باشند تا صفحه خالی به نظر نرسد.
      setExpanded((current) =>
        current.size > 0
          ? current
          : new Set(tree.filter((r) => !r.parent_id).map((r) => r.id)),
      )
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }, [showInactive])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    ;(async () => {
      try {
        setScheme(await getCodingScheme())
      } catch (e) {
        setError(errorText(e))
      }
      try {
        setIssues(await auditCodingHealth())
      } catch {
        /* گزارش سلامت اختیاری است */
      }
      try {
        const list = await getSubsidiaryGroups()
        setGroups(list.map((g) => ({ id: g.id, title: g.title })))
      } catch {
        /* گروه تفصیلی اختیاری است */
      }
    })()
  }, [])

  const byParent = useMemo(() => {
    const map = new Map<string, AccountNodeRow[]>()
    for (const row of rows) {
      const key = row.parent_id ?? '__root__'
      map.set(key, [...(map.get(key) ?? []), row])
    }
    return map
  }, [rows])

  // هنگام جستجو، درخت تخت می‌شود تا نتیجه پیدا شود؛ بدون آن باید همه‌ی
  // شاخه‌ها را دستی باز کرد.
  const searching = search.trim().length > 0
  const matches = useMemo(() => {
    const term = search.trim()
    if (!term) return rows
    return rows.filter((row) => row.code.includes(term) || row.name.includes(term))
  }, [rows, search])

  const toggle = (id: string) =>
    setExpanded((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })

  const openNew = async (parent?: AccountNodeRow) => {
    setNotice('')
    let code = ''
    try {
      code = await suggestAccountCode(parent?.id)
    } catch (e) {
      setError(errorText(e))
    }
    setDraft({
      code,
      name: '',
      nature: parent?.nature ?? 'debit',
      parent_id: parent?.id,
      requires_subsidiary: false,
      is_active: true,
    })
  }

  const openEdit = (row: AccountNodeRow) =>
    setDraft({
      id: row.id,
      code: row.code,
      name: row.name,
      nature: row.nature,
      parent_id: row.parent_id,
      requires_subsidiary: row.requires_subsidiary,
      subsidiary_group_id: row.subsidiary_group_id,
      is_active: row.is_active,
    })

  const save = async () => {
    if (!draft) return
    setBusy(true)
    try {
      await saveAccount(draft)
      setNotice(draft.id ? t('coa.updated') : t('coa.created'))
      setDraft(null)
      await load()
      setIssues(await auditCodingHealth())
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const remove = async (row: AccountNodeRow) => {
    setBusy(true)
    try {
      await deactivateAccount(row.id)
      setNotice(`«${row.name}» غیرفعال شد.`)
      await load()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  const renderRow = (row: AccountNodeRow, depth: number): React.ReactNode[] => {
    const children = byParent.get(row.id) ?? []
    const isOpen = expanded.has(row.id)
    const output: React.ReactNode[] = [
      <tr key={row.id} className={row.is_active ? '' : 'row-muted'}>
        <td>
          <div className="tree-cell" style={{ paddingInlineStart: `${depth * 22}px` }}>
            {children.length > 0 ? (
              <button
                className={`tree-toggle${isOpen ? ' open' : ''}`}
                onClick={() => toggle(row.id)}
                aria-label={isOpen ? t('coa.collapse') : t('coa.expand')}
              >
                <Icon name="chevron" size={14} />
              </button>
            ) : (
              <span className="tree-leaf" />
            )}
            <span className="code">{row.code}</span>
          </div>
        </td>
        <td>{row.name}</td>
        <td>{row.level_title}</td>
        <td>{row.nature_label}</td>
        <td>
          {row.is_postable ? (
            <span className="status done">{t('coa.postable')}</span>
          ) : (
            <span className="status neutral">تجمیعی ({row.child_count} زیرحساب)</span>
          )}
        </td>
        <td className="num">{row.debit ? money(row.debit) : '—'}</td>
        <td className="num">{row.credit ? money(row.credit) : '—'}</td>
        <td className={`num${row.rollup_balance < 0 ? ' red-text' : ''}`}>
          {money(Math.abs(row.rollup_balance))}
        </td>
        <td>
          <button className="table-action" onClick={() => openNew(row)}>
            {t('coa.child')}
          </button>
          <button className="table-action" onClick={() => openEdit(row)}>
            {t('common.editAction')}
          </button>
          {row.is_active && (
            <button className="table-action" disabled={busy} onClick={() => remove(row)}>
              {t('coa.inactive')}
            </button>
          )}
        </td>
      </tr>,
    ]
    if (isOpen) {
      for (const child of children) output.push(...renderRow(child, depth + 1))
    }
    return output
  }

  const roots = byParent.get('__root__') ?? []
  const errorIssues = issues.filter((i) => i.severity === 'error')
  const infoIssues = issues.filter((i) => i.severity === 'info')

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('nav.group.accounting')}</div>
          <h1>{t('coa.title')}</h1>
          <p>
            {t('coa.subtitle')}
          </p>
        </div>
        <button className="primary" onClick={() => openNew()}>
          <Icon name="plus" /> {t('coa.groupLevel')}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {notice && <div className="success-box">{notice}</div>}

      {scheme && (
        <div className="metric-strip">
          {scheme.level_titles.map((title, index) => (
            <div key={title}>
              <span>{title}</span>
              <b>{scheme.code_lengths[index]} رقم</b>
              <small>تا {formatCount(scheme.capacities[index])} حساب</small>
            </div>
          ))}
          <div>
            <span>{t('coa.totalAccounts')}</span>
            <b>{rows.length}</b>
            <small>{rows.filter((r) => r.is_postable).length} قابل ثبت سند</small>
          </div>
        </div>
      )}

      {errorIssues.length > 0 && (
        <div className="error-box">
          <b>ایراد کدینگ ({errorIssues.length} مورد):</b>
          {errorIssues.map((issue) => (
            <div key={issue.account_id + issue.message}>
              {issue.code} — {issue.name}: {issue.message}
            </div>
          ))}
        </div>
      )}
      {infoIssues.length > 0 && (
        <div className="warn-box">
          {infoIssues.length} حساب با طرح کدینگ فعلی نمی‌خوانند. کدینگ مسطح است و کاملاً کار
          می‌کند؛ فقط پیشنهاد کد خودکار برای آن‌ها تقریبی است.
        </div>
      )}

      <div className="panel list-panel">
        <div className="panel-head">
          <div>
            <h3>{t('coa.tree')}</h3>
            <p>{searching ? `${matches.length} نتیجه` : `${roots.length} شاخه‌ی اصلی`}</p>
          </div>
          <div className="filter-actions">
            <input
              className="search-input"
              placeholder={t('coa.search')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <label className="inline-check">
              <input
                type="checkbox"
                checked={showInactive}
                onChange={(e) => setShowInactive(e.target.checked)}
              />
              <span>{t('coa.inactiveOnes')}</span>
            </label>
            <button className="icon-btn" onClick={load} aria-label={t('common.refresh')}>
              <Icon name="refresh" />
            </button>
          </div>
        </div>

        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th>{t('common.code')}</th>
                <th>{t('coa.accountName')}</th>
                <th>{t('coa.level')}</th>
                <th>{t('coa.nature')}</th>
                <th>{t('common.status')}</th>
                <th>{t('reports.debit')}</th>
                <th>{t('reports.credit')}</th>
                <th>{t('coa.branchBalance')}</th>
                <th>{t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {searching
                ? matches.map((row) => renderRow(row, 0)).flat()
                : roots.map((row) => renderRow(row, 0)).flat()}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={9} className="empty-row">
                    {t('coa.empty')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {draft && (
        <div className="modal-backdrop" role="presentation">
          <div className="modal">
            <div className="modal-head">
              <div>
                <h2>{draft.id ? t('coa.editAccount') : t('coa.newAccount')}</h2>
                <p>
                  {draft.parent_id
                    ? `زیرمجموعه‌ی ${rows.find((r) => r.id === draft.parent_id)?.name ?? ''}`
                    : t('coa.groupLevelNoParent')}
                </p>
              </div>
              <button aria-label={t('common.close')} className="icon-btn" onClick={() => setDraft(null)}>
                <Icon name="close" />
              </button>
            </div>

            <div className="filter-grid">
              <label>
                <span>{t('coa.codeRequired')}</span>
                <input
                  value={draft.code}
                  onChange={(e) => setDraft({ ...draft, code: e.target.value })}
                  inputMode="numeric"
                />
              </label>
              <label className="grow">
                <span>{t('coa.nameRequired')}</span>
                <input
                  value={draft.name}
                  onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                />
              </label>
              <label>
                <span>{t('coa.natureRequired')}</span>
                <Select
                  value={draft.nature}
                  onChange={(e) => setDraft({ ...draft, nature: e.target.value })}
                >
                  {NATURES.map((n) => (
                    <option key={n.value} value={n.value}>
                      {t(n.labelKey)}
                    </option>
                  ))}
                </Select>
              </label>
              <label className="grow">
                <span>{t('coa.subsidiaryRequired')}</span>
                <Select
                  value={draft.subsidiary_group_id ?? ''}
                  onChange={(e) =>
                    setDraft({
                      ...draft,
                      subsidiary_group_id: e.target.value || undefined,
                      requires_subsidiary: e.target.value !== '',
                    })
                  }
                >
                  <option value="">{t('coa.noSubsidiary')}</option>
                  {groups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.title}
                    </option>
                  ))}
                </Select>
              </label>
              <div className="checkbox-row">
                <label className="inline-check">
                  <input
                    type="checkbox"
                    checked={draft.is_active}
                    onChange={(e) => setDraft({ ...draft, is_active: e.target.checked })}
                  />
                  <span>{t('partyForm.active')}</span>
                </label>
              </div>
              <p className="hint">
                {t('coa.natureHint')}
              </p>
            </div>

            <div className="modal-actions">
              <button
                className="primary"
                onClick={save}
                disabled={busy || !draft.code.trim() || !draft.name.trim()}
              >
                {t('common.save')}
              </button>
              <button className="ghost" onClick={() => setDraft(null)}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  )
}
