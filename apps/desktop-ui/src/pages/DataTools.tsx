import {useMemo, useState} from 'react'
import {Icon} from '../components/Icon'
import {Select} from '../components/Select'
import {importData} from '../api'
import {errorText} from '../lib/errors'
import {formatRials as money, formatCount} from '../lib/format'

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

const SCHEMA: Record<Entity, {title: string; columns: Column[]; sample: Record<string, string>[]}> = {
  contacts: {
    title: 'اشخاص',
    columns: [
      {key: 'name', label: 'نام / عنوان', hint: 'نام شخص حقیقی یا عنوان شرکت', required: true},
      {key: 'kind', label: 'نوع', hint: 'person برای حقیقی، company برای حقوقی', required: true},
      {key: 'mobile', label: 'موبایل', hint: 'با صفر ابتدایی، بدون فاصله'},
      {key: 'is_customer', label: 'مشتری', hint: '1 یا 0'},
      {key: 'is_supplier', label: 'تأمین‌کننده', hint: '1 یا 0'},
    ],
    sample: [
      {name: 'شرکت آریا صنعت پارس', kind: 'company', mobile: '02188776655', is_customer: '1', is_supplier: '0'},
      {name: 'مهدی رضایی', kind: 'person', mobile: '09121234567', is_customer: '1', is_supplier: '0'},
      {name: 'بازرگانی نیک‌آور', kind: 'company', mobile: '02177001122', is_customer: '0', is_supplier: '1'},
    ],
  },
  products: {
    title: 'کالاها',
    columns: [
      {key: 'sku', label: 'کد کالا', hint: 'یکتا؛ تکراری رد می‌شود', required: true},
      {key: 'barcode', label: 'بارکد', hint: 'اختیاری'},
      {key: 'name', label: 'نام کالا', hint: 'نام نمایشی در فاکتور', required: true},
      {key: 'unit', label: 'واحد', hint: 'عدد، کیلوگرم، متر…', required: true},
      {key: 'sale_price', label: 'قیمت فروش', hint: 'به ریال، بدون جداکننده', money: true},
      {key: 'purchase_price', label: 'قیمت خرید', hint: 'به ریال، مبنای بهای تمام‌شده', money: true},
      {key: 'min_stock', label: 'حداقل موجودی', hint: 'مبنای هشدار «نزدیک به اتمام»'},
    ],
    sample: [
      {sku: 'P-1001', barcode: '6260100100015', name: 'روغن موتور ۴ لیتری', unit: 'عدد', sale_price: '4850000', purchase_price: '3900000', min_stock: '12'},
      {sku: 'P-1002', barcode: '6260100100022', name: 'فیلتر هوا پراید', unit: 'عدد', sale_price: '620000', purchase_price: '430000', min_stock: '40'},
      {sku: 'P-2010', barcode: '', name: 'سیم برق افشان ۱.۵', unit: 'متر', sale_price: '78000', purchase_price: '61000', min_stock: '500'},
    ],
  },
}

/** نمونه‌ی CSV با همان ستون‌هایی که وارد‌کننده انتظار دارد. */
function sampleCsv(entity: Entity): string {
  const {columns, sample} = SCHEMA[entity]
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
        {money(numeric)} ریال
      </small>
    </>
  )
}

export function DataTools() {
  const [entity, setEntity] = useState<Entity>('contacts')
  const [rows, setRows] = useState<Record<string, unknown>[]>([])
  const [fileName, setFileName] = useState('')
  const [busy, setBusy] = useState(false)
  const [msg, setMsg] = useState('')
  const [error, setError] = useState('')

  const schema = SCHEMA[entity]

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
        if (lines.length < 2) throw new Error('فایل باید حداقل یک ردیف داده داشته باشد.')
        const headers = lines[0].split(',').map((x) => x.trim().replace(/^"|"$/g, ''))
        const parsed = lines.slice(1).map((line) => {
          const values = line.split(',').map((x) => x.trim().replace(/^"|"$/g, ''))
          return Object.fromEntries(headers.map((h, i) => [h, values[i] ?? '']))
        })
        setRows(parsed)
        setMsg(`${formatCount(parsed.length)} ردیف برای بررسی آماده شد.`)
      } catch (e) {
        setRows([])
        setError(errorText(e))
      }
    }
    reader.readAsText(file, 'utf-8')
  }

  const downloadSample = () => {
    const blob = new Blob([sampleCsv(entity)], {type: 'text/csv;charset=utf-8'})
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
      setMsg(`${formatCount(rows.length)} ردیف با موفقیت وارد شد.`)
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
          <div className="eyebrow">ابزار داده</div>
          <h1>ورود و خروج اطلاعات</h1>
          <p>
            پیش از ثبت، داده‌ها را بررسی کنید. عملیات ورود در یک تراکنش انجام می‌شود؛ اگر یک ردیف
            خطا داشته باشد هیچ ردیفی ثبت نمی‌شود.
          </p>
        </div>
        <button className="ghost" onClick={downloadSample}>
          <Icon name="download" /> دریافت فایل نمونه
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}
      {msg && <div className="success-box">{msg}</div>}

      <div className="panel">
        <div className="filter-grid">
          <label>
            <span>نوع داده</span>
            <Select
              value={entity}
              aria-label="نوع داده"
              onChange={(e) => {
                setEntity(e.target.value as Entity)
                setRows([])
                setFileName('')
                setMsg('')
                setError('')
              }}
            >
              <option value="contacts">اشخاص</option>
              <option value="products">کالاها</option>
            </Select>
          </label>
          <label className="grow">
            <span>فایل CSV با کدگذاری UTF-8</span>
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
            <p>ترتیب ستون‌ها مهم نیست؛ نام ستون در سطر اول باید دقیقاً همین باشد.</p>
          </div>
        </div>
        <div className="table-wrap">
          <table className="large-table">
            <thead>
              <tr>
                <th>نام ستون در فایل</th>
                <th>معنی</th>
                <th>الزامی</th>
                <th>توضیح</th>
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
                      {column.required ? 'بله' : 'اختیاری'}
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
              <h3>نمونه‌ی نمایشی — پس از انتخاب فایل، چنین چیزی می‌بینید</h3>
              <p>این ردیف‌ها فقط برای نمایش ساختار هستند و هرگز ثبت نمی‌شوند.</p>
            </div>
            <span className="chip">نمونه</span>
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
                {formatCount(rows.length)} ردیف خوانده شد
                {rows.length > 50 ? ' — ۵۰ ردیف اول نمایش داده می‌شود.' : '.'}
              </p>
            </div>
          </div>
          {missing.length > 0 && (
            <div className="error-box">
              ستون‌های اجباری در فایل نیستند: {missing.join('، ')} — ورود اطلاعات مسدود است.
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
              انصراف
            </button>
            <button className="primary" disabled={busy || missing.length > 0} onClick={commit}>
              {busy ? 'در حال ورود…' : 'تأیید و ورود اطلاعات'}
            </button>
          </div>
        </div>
      )}
    </section>
  )
}
