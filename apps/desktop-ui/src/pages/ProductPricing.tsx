import {useEffect, useMemo, useState} from 'react'
import {
  getProductGroups,
  getProductPrices,
  setProductPrice,
  type ProductGroupRow,
  type ProductPriceRow,
} from '../api'
import {Icon} from '../components/Icon'
import {errorText} from '../lib/errors'
import {formatRials, parseAmount} from '../lib/format'

/**
 * مرکز قیمت‌گذاری کالاها — معادل «قیمت کالاها» در نوار ابزار لیست کالاها.
 *
 * هر هفت سطح قیمت (جزئی، کلی، همکار، همکار درجه ۲ و ۳، فصلی، نمایشگاه) به‌صورت
 * جدولی و قابل ویرایش درجا نمایش داده می‌شود. سطح خالی یعنی «تعریف‌نشده» و در
 * فروش، زنجیره‌ی جایگزینی هسته تصمیم می‌گیرد کدام قیمت اعمال شود.
 */
export function ProductPricing() {
  const [rows, setRows] = useState<ProductPriceRow[]>([])
  const [groups, setGroups] = useState<ProductGroupRow[]>([])
  const [groupFilter, setGroupFilter] = useState('')
  const [search, setSearch] = useState('')
  const [error, setError] = useState('')
  const [savingKey, setSavingKey] = useState('')
  const [loading, setLoading] = useState(true)

  const load = async () => {
    setLoading(true)
    setError('')
    try {
      const [priceRows, groupRows] = await Promise.all([getProductPrices(), getProductGroups()])
      setRows(priceRows)
      setGroups(groupRows)
    } catch (e) {
      setError(errorText(e))
    } finally {
      setLoading(false)
    }
  }
  useEffect(() => {
    load()
  }, [])

  const levels = rows[0]?.prices ?? []

  const visible = useMemo(
    () =>
      rows.filter((row) => {
        const matchesGroup = !groupFilter || row.group_title === groupFilter
        const term = search.trim()
        const matchesSearch = !term || row.name.includes(term) || row.sku.includes(term)
        return matchesGroup && matchesSearch
      }),
    [rows, groupFilter, search],
  )

  const commit = async (productId: string, level: string, raw: string) => {
    const key = `${productId}:${level}`
    setSavingKey(key)
    setError('')
    try {
      const trimmed = raw.trim()
      const value = trimmed === '' ? null : parseAmount(trimmed)
      if (trimmed !== '' && value === null) {
        setError('مبلغ واردشده معتبر نیست.')
        return
      }
      await setProductPrice(productId, level, value)
      setRows((current) =>
        current.map((row) =>
          row.id === productId
            ? {
                ...row,
                prices: row.prices.map((price) =>
                  price.level === level ? {...price, price: value} : price,
                ),
              }
            : row,
        ),
      )
    } catch (e) {
      setError(errorText(e))
      await load()
    } finally {
      setSavingKey('')
    }
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">کالا و خدمات</div>
          <h1>قیمت کالاها</h1>
          <p>هفت سطح قیمت برای هر کالا. سطح خالی یعنی از سطح بالاتر استفاده می‌شود.</p>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}

      <div className="panel list-panel">
        <div className="toolbar">
          <input
            className="search-input"
            placeholder="جستجوی نام یا کد کالا…"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <select value={groupFilter} onChange={(event) => setGroupFilter(event.target.value)}>
            <option value="">همه‌ی گروه‌ها</option>
            {groups.map((group) => (
              <option key={group.id} value={group.title}>
                {group.code} — {group.title} ({group.product_count})
              </option>
            ))}
          </select>
          <button aria-label="بروزرسانی" className="icon-btn" onClick={load} title="بارگذاری مجدد">
            <Icon name="refresh" />
          </button>
        </div>

        {loading ? (
          <div className="empty-state">در حال بارگذاری…</div>
        ) : visible.length === 0 ? (
          <div className="empty-state">کالایی یافت نشد.</div>
        ) : (
          <div className="table-wrap">
            <table className="large-table price-table">
              <thead>
                <tr>
                  <th>کد</th>
                  <th>نام کالا</th>
                  <th>نوع</th>
                  <th>گروه</th>
                  {levels.map((level) => (
                    <th key={level.level}>{level.label}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {visible.map((row) => (
                  <tr key={row.id}>
                    <td className="code">{row.sku}</td>
                    <td>{row.name}</td>
                    <td>
                      <span className="status pending">{row.kind_label}</span>
                    </td>
                    <td>{row.group_title ?? '—'}</td>
                    {row.prices.map((price) => {
                      const key = `${row.id}:${price.level}`
                      return (
                        <td key={price.level} className="price-cell">
                          <input
                            defaultValue={price.price === null ? '' : String(price.price)}
                            disabled={savingKey === key}
                            placeholder="—"
                            inputMode="numeric"
                            onBlur={(event) => {
                              const next = event.target.value.trim()
                              const previous = price.price === null ? '' : String(price.price)
                              if (next !== previous) commit(row.id, price.level, next)
                            }}
                            onKeyDown={(event) => {
                              if (event.key === 'Enter') event.currentTarget.blur()
                            }}
                          />
                          {price.price !== null && (
                            <small>{formatRials(price.price)}</small>
                          )}
                        </td>
                      )
                    })}
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
