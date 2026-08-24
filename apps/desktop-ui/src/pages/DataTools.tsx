import {useMemo, useState} from 'react'
import {Icon} from '../components/Icon'
import {Select} from '../components/Select'
import {importData} from '../api'
import {errorText} from '../lib/errors'
import {formatRials as money, formatCount, rialUnit} from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'

/**
 * ورود و خروج اطلاعات.
 *
 * ## چرا نمونه‌ی نمایشی دارد
 * کاربر پیش از انتخاب فایل نمی‌داند خروجی چه شکلی است و چه ستون‌هایی لازم
 * است. اینجا همیشه یک «نمونه» با داده‌ی واقعی‌نما نشان داده می‌شود تا
 * ساختار فایل قبل از هر تلاشی روشن باشد. نمونه با برچسب صریح از داده‌ی
 * واقعی جدا شده و هرگز ثبت نمی‌شود.
 */
type Entity = 'contacts' | 'products'

type Column = {key: string; label: string; hint: string; required?: boolean; money?: boolean}

type Schema = Record<Entity, {title: string; columns: Column[]; sample: Record<string, string>[]}>

/** ساختار فایل ورودی به زبان فعال — عنوان ستون‌ها همان چیزی است که کاربر می‌بیند. */
const schemaFor = (t: (key: TranslationKey) => string): Schema => ({
  contacts: {
    title: t('page.parties'),
    columns: [
      {key: 'name', label: t('dt.nameOrTitle'), hint: t('dt.nameHint'), required: true},
      {key: 'kind', label: t('common.type'), hint: t('dt.typeHint'), required: true},
      {key: 'mobile', label: t('partyForm.mobile'), hint: t('dt.mobileHint')},
      {key: 'is_customer', label: t('partyForm.customer'), hint: t('dt.oneOrZero')},
      {key: 'is_supplier', label: t('partyForm.supplier'), hint: t('dt.oneOrZero')},
    ],
    sample: [
      {name: t('print.sample.company'), kind: 'company', mobile: '02188776655', is_customer: '1', is_supplier: '0'},
      {name: t('dt.samplePerson'), kind: 'person', mobile: '09121234567', is_customer: '1', is_supplier: '0'},
      {name: t('dt.sampleCompany'), kind: 'company', mobile: '02177001122', is_customer: '0', is_supplier: '1'},
    ],
  },
  products: {
    title: t('page.products'),
    columns: [
      {key: 'sku', label: t('productForm.skuRequired'), hint: t('dt.skuHint'), required: true},
      {key: 'barcode', label: t('productForm.barcode'), hint: t('dt.optional')},
      {key: 'name', label: t('products.name'), hint: t('dt.displayNameHint'), required: true},
      {key: 'unit', label: t('common.unit'), hint: t('dt.unitHint'), required: true},
      {key: 'sale_price', label: t('dataPage.salePrice'), hint: t('dt.rialNoSeparator'), money: true},
      {key: 'purchase_price', label: t('dataPage.purchasePrice'), hint: t('dt.costBasis'), money: true},
      {key: 'min_stock', label: t('productForm.minStock'), hint: t('dt.lowStockBasis')},
    ],
    sample: [
      {sku: 'P-1001', barcode: '6260100100015', name: t('print.sample.item1'), unit: t('productForm.defaultUnit'), sale_price: '4850000', purchase_price: '3900000', min_stock: '12'},
      {sku: 'P-1002', barcode: '6260100100022', name: t('print.sample.item2'), unit: t('productForm.defaultUnit'), sale_price: '620000', purchase_price: '430000', min_stock: '40'},
      {sku: 'P-2010', barcode: '', name: t('print.sample.item3'), unit: t('print.unit.metre'), sale_price: '78000', purchase_price: '61000', min_stock: '500'},
    ],
  },
})

/** نمونه‌ی CSV با همان ستون‌هایی که وارد‌کننده انتظار دارد. */
function sampleCsv(entity: Entity, schema: Schema): string {
  const {columns, sample} = schema[entity]
  const head = columns.map((c) => c.key).join(',')
  const body = sample.map((row) => columns.map((c) => row[c.key] ?? '').join(','))
  return ['\ufeff' + head, ...body].join('\r\n')
}

/**
 * نمایش یک خانه‌ی جدول: مقدار خام فایل، و برای ستون‌های مبلغی یک خوانش
 * ریالی زیر آن. کاربر هم می‌بیند در فایل چه نوشته و هم می‌فهمد چقدر است.
 */
function cell(column: Column | undefined, raw: string | undefined) {
  const value = (raw ?? '').trim()
  if (!value) return '—'
  if (!column?.money) return value
  const numeric = Number(value.replace(/[,\s]/g, ''))
  if (!Number.isFinite(numeric)) return value
  return (
    <>
      <span dir="ltr">{value}</span>
      <small className="field-hint" style={{display: 'block'}}>
        {money(numeric)} {rialUnit()}
      </small>
    </>
  )
}

export function DataTools() {
  const { t } = useI18n()
  const [entity, setEntity] = useState<Entity>('contacts')
  const [rows, setRows] = useState<Record<string, unknown>[]>([])
  const [fileName, setFileName] = useState('')
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState('')
  const [error, setError] = useState('')

  const schema = schemaFor(t)[entity]

  /** ستون‌های اجباری که در فایل انتخاب‌شده پیدا نشدند. */
  const missing = useMemo(() => {
    if (!rows.length) return []
    const present = new Set(Object.keys(rows[0]))
    return schema.columns.filter((c) => c.required && !present.has(c.key)).map((c) => c.key)
  }, [rows, schema])

  const read = (file: File) => {
    setError('')
    setMsg('')
    setFileName(file.name)
    const reader = new FileReader()
    reader.onload = () => {
      try {
        const text = String(reader.result || '')
        const lines = text.replace(/^\uFEFF/, '').split(/\r?\n/).filter(Boolean)
        if (lines.length < 2) throw new Error(t('dt.errEmptyFile'))
        const headers = lines[0].split(',').map((x) => x.trim().replace(/^"|"$/g, ''))
        const parsed = lines.slice(1).map((line) => {
          const values = line.split(',').map((x) => x.trim().replace(/^"|"$/g, ''))
          return Object.fromEntries(headers.map((h, i) => [h, values[i] ?? '']))
        })
        setRows(parsed)
        setMsg(t('dt.rowsReady', {count: formatCount(parsed.length)}))
      } catch (e) {
        setRows([])
        setError(errorText(e))
      }
    }
    reader.readAsText(file, 'utf-8')
  }

  const downloadSample = () => {
    const blob = new Blob([sampleCsv(entity, schemaFor(t))], {type: 'text/csv;charset=utf-8'})
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = `نمونه-${schema.title}.csv`
    link.click()
    URL.revokeObjectURL(url)
  }

  const commit = async () => {
    if (!rows.length || missing.length) return
    setBusy(true)
    setError('')
    try {
      await importData(entity, rows)
      setMsg(t('dt.rowsImported', {count: formatCount(rows.length)}))
      setRows([])
      setFileName('')
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('dt.eyebrow')}</div>
          <h1>{t('page.data-tools')}</h1>
          <p>
            {t('dt.subtitle')}
          </p>
        </div>
        <button className="ghost" onClick={downloadSample}>
          <Icon name="download" /> {t('dt.downloadSample')}
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {msg && <div className="success-box">{msg}</div>}

      <div className="panel">
        <div className="filter-grid">
          <label>
            <span>{t('dt.dataKind')}</span>
            <Select
              value={entity}
              aria-label={t('dt.dataKind')}
              onChange={(e) => {
                setEntity(e.target.value as Entity)
                setRows([])
                setFileName('')
                setMsg('')
                setError('')
              }}
            >
              <option value="contacts">{t('page.parties')}</option>
              <option value="products">{t('page.products')}</option>
            </Select>
          </label>
          <label className="grow">
            <span>{t('dt.csvUtf8')}</span>
            <input
              type="file"
              accept=".csv,text/csv"
              onChange={(e) => e.target.files?.[0] && read(e.target.files[0])}
            />
          </label>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <div>
            <h3>ستون‌های مورد انتظار — {schema.title}</h3>
            <p>{t('dt.columnOrderNote')}</p>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th>{t('dt.columnName')}</th>
                <th>{t('dt.meaning')}</th>
                <th>{t('dt.required')}</th>
                <th>{t('transfer.note')}</th>
              </tr>
            </thead>
            <tbody>
              {schema.columns.map((column) => (
                <tr key={column.key}>
                  <td dir="ltr" style={{textAlign: 'right'}}>
                    <code>{column.key}</code>
                  </td>
                  <td>{column.label}</td>
                  <td>
                    <span className={column.required ? 'status danger' : 'status neutral'}>
                      {column.required ? t('dt.yes') : t('dt.optional')}
                    </span>
                  </td>
                  <td>{column.hint}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="panel">
          <div className="panel-head">
            <div>
              <h3>{t('dt.previewNote')}</h3>
              <p>{t('dt.previewOnly')}</p>
            </div>
            <span className="chip">{t('dt.sample')}</span>
          </div>
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>#</th>
                  {schema.columns.map((column) => (
                    <th key={column.key}>{column.label}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {schema.sample.map((row, index) => (
                  <tr key={index} className="row-muted">
                    <td>{formatCount(index + 1)}</td>
                    {schema.columns.map((column) => (
                      <td key={column.key}>{cell(column, row[column.key])}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="panel">
          <div className="panel-head">
            <div>
              <h3>پیش‌نمایش فایل {fileName}</h3>
              <p>
                {t('dt.rowsRead', {count: formatCount(rows.length)})}
                {rows.length > 50 ? t('dt.first50') : '.'}
              </p>
            </div>
          </div>
          {missing.length > 0 && (
            <div className="error-box">
              {t('dt.missingColumns', {list: missing.join(t('dt.listSeparator'))})}
            </div>
          )}
          <div className="table-wrap">
            <table className="large-table">
              <thead>
                <tr>
                  <th>#</th>
                  {Object.keys(rows[0]).map((key) => (
                    <th key={key}>{schema.columns.find((c) => c.key === key)?.label ?? key}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {rows.slice(0, 50).map((row, index) => (
                  <tr key={index}>
                    <td>{formatCount(index + 1)}</td>
                    {Object.keys(rows[0]).map((key) => (
                      <td key={key}>
                        {cell(schema.columns.find((c) => c.key === key), String(row[key] ?? ''))}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="modal-actions">
            <button
              className="secondary"
              onClick={() => {
                setRows([])
                setFileName('')
                setMsg('')
              }}
            >
              {t('common.cancel')}
            </button>
            <button className="primary" disabled={busy || missing.length > 0} onClick={commit}>
              {busy ? t('dt.importing') : t('dt.confirmImport')}
            </button>
          </div>
        </div>
      )}
    </section>
  )
}
