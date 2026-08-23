import { useEffect, useMemo, useState } from 'react'
import { Printer, Save, Trash2, Upload } from 'lucide-react'
import {
  deletePrintTemplate,
  getPrintTemplates,
  getSettings,
  savePrintTemplate,
  setSetting,
  PrintTemplate,
} from '../api'
import { errorText } from '../lib/errors'
import { formatRials as money } from '../lib/format'
import { Badge, Card, CardHeader, EmptyState } from '../components/ui'
import { Select } from '../components/Select'
import { companyFrom, printHtml } from '../lib/printing'
import {
  COLUMN_LABEL,
  PAPER_LABEL,
  PAPER_WIDTH_MM,
  defaultDesign,
  parseDesign,
  renderBody,
  renderDocument,
  printStyles,
  type LineColumn,
  type PrintDocument,
  type TemplateDesign,
  type TemplateKind,
} from '../lib/printTemplate'

/**
 * طراح بصری قالب چاپ.
 *
 * ## چرا بازنویسی شد
 * نسخه‌ی قبلی یک textarea بود که از کاربر HTML می‌خواست. حسابدار HTML
 * نمی‌نویسد؛ نتیجه این بود که کسی قالب را عوض نمی‌کرد. حالا هر بخش یک
 * کلید روشن/خاموش دارد و پیش‌نمایش در **اندازه‌ی واقعی کاغذ** کنارش رسم
 * می‌شود.
 *
 * ## چرا پیش‌نمایش با همان موتور چاپ رسم می‌شود
 * اگر پیش‌نمایش با کد جداگانه ساخته شود، دیر یا زود با خروجی چاپگر فرق
 * می‌کند. اینجا دقیقاً همان `renderBody` و همان `printStyles` استفاده
 * می‌شود که چاپگر می‌گیرد — پس آنچه می‌بینید همان چیزی است که چاپ می‌شود.
 */

const KINDS: { value: TemplateKind; label: string }[] = [
  { value: 'invoice', label: 'فاکتور فروش' },
  { value: 'receipt', label: 'رسید فروشگاهی' },
  { value: 'journal', label: 'سند حسابداری' },
  { value: 'report', label: 'گزارش' },
  { value: 'label', label: 'برچسب کالا' },
]

/** نمونه‌ی داده برای پیش‌نمایش — تا کاربر قالب را با محتوای واقعی‌نما ببیند. */
const SAMPLE: PrintDocument = {
  number: '1042',
  date: '1405/05/30',
  partyName: 'شرکت آریا صنعت پارس',
  partyPhone: '021-88776655',
  lines: [
    { code: 'P-1001', name: 'روغن موتور ۴ لیتری', quantity: 3, unit: 'عدد', unit_price: 4_850_000, discount: 0, vat: 1_309_500, line_total: 15_859_500 },
    { code: 'P-1002', name: 'فیلتر هوا پراید', quantity: 10, unit: 'عدد', unit_price: 620_000, discount: 200_000, vat: 540_000, line_total: 6_540_000 },
    { code: 'P-2010', name: 'سیم برق افشان ۱.۵', quantity: 25, unit: 'متر', unit_price: 78_000, discount: 0, vat: 175_500, line_total: 2_125_500 },
  ],
  subtotal: 24_400_000,
  discount: 200_000,
  vat: 2_025_000,
  total: 24_525_000,
}

type Toggle = { key: keyof TemplateDesign; label: string; hint?: string }

const HEADER_TOGGLES: Toggle[] = [
  { key: 'showLogo', label: 'لوگوی مجموعه' },
  { key: 'showCompanyName', label: 'نام مجموعه' },
  { key: 'showPhone', label: 'شماره تماس' },
  { key: 'showAddress', label: 'نشانی' },
  { key: 'showEconomicCode', label: 'کد اقتصادی' },
]

const DOC_TOGGLES: Toggle[] = [
  { key: 'showDocumentNumber', label: 'شماره سند' },
  { key: 'showDate', label: 'تاریخ' },
  { key: 'showParty', label: 'نام طرف حساب' },
  { key: 'showPartyPhone', label: 'تلفن طرف حساب' },
]

const FOOTER_TOGGLES: Toggle[] = [
  { key: 'showSubtotal', label: 'جمع کل' },
  { key: 'showDiscount', label: 'تخفیف' },
  { key: 'showVat', label: 'ارزش افزوده' },
  { key: 'showTotal', label: 'مبلغ قابل پرداخت' },
  { key: 'showAmountInWords', label: 'مبلغ به حروف', hint: 'در فاکتور رسمی الزامی است' },
  { key: 'showBarcode', label: 'بارکد شماره سند' },
  { key: 'showSignature', label: 'محل مهر و امضا' },
  { key: 'zebra', label: 'سطرهای یک‌درمیان خاکستری' },
]

const ALL_COLUMNS = Object.keys(COLUMN_LABEL) as LineColumn[]

export function PrintTemplates() {
  const [items, setItems] = useState<PrintTemplate[]>([])
  const [settings, setSettings] = useState<{ key: string; value: string }[]>([])
  const [editingId, setEditingId] = useState<string>()
  const [name, setName] = useState('قالب جدید')
  const [kind, setKind] = useState<TemplateKind>('invoice')
  const [design, setDesign] = useState<TemplateDesign>(() => defaultDesign('invoice'))
  const [isDefault, setIsDefault] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const company = useMemo(() => companyFrom(settings, 'شرکت نوین پرداز'), [settings])

  const load = async () => {
    try {
      const [templates, config] = await Promise.all([
        getPrintTemplates(),
        getSettings().catch(() => []),
      ])
      setItems(templates)
      setSettings(config.map((item) => ({ key: item.key, value: item.value })))
      setError('')
    } catch (e) {
      setError(errorText(e))
    }
  }

  useEffect(() => {
    void load()
  }, [])

  const patch = (changes: Partial<TemplateDesign>) =>
    setDesign((current) => ({ ...current, ...changes }))

  const startNew = (nextKind: TemplateKind = 'invoice') => {
    setEditingId(undefined)
    setKind(nextKind)
    setName(`قالب ${KINDS.find((item) => item.value === nextKind)?.label ?? ''}`)
    setDesign(defaultDesign(nextKind))
    setIsDefault(false)
    setMessage('')
  }

  const edit = (template: PrintTemplate) => {
    const templateKind = (template.template_type as TemplateKind) ?? 'invoice'
    setEditingId(template.id)
    setName(template.name)
    setKind(templateKind)
    setDesign(parseDesign(template.content_html, templateKind) ?? defaultDesign(templateKind))
    setIsDefault(template.is_default)
    setMessage('')
  }

  const save = async () => {
    try {
      await savePrintTemplate(editingId, name, kind, JSON.stringify(design), isDefault)
      setMessage('قالب ذخیره شد.')
      await load()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const remove = async (id: string) => {
    if (!confirm('این قالب حذف شود؟')) return
    try {
      await deletePrintTemplate(id)
      if (editingId === id) startNew()
      await load()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const testPrint = () => printHtml(renderDocument(design, company, SAMPLE, 1))

  /** بارگذاری لوگو: تبدیل به Data URL و ذخیره در تنظیمات. */
  const uploadLogo = (file: File) => {
    if (file.size > 900_000) {
      setError('حجم لوگو باید کمتر از حدود ۹۰۰ کیلوبایت باشد.')
      return
    }
    const reader = new FileReader()
    reader.onload = async () => {
      try {
        const dataUrl = String(reader.result ?? '')
        await setSetting('company.logo', dataUrl)
        setMessage('لوگو ذخیره شد و در همه‌ی چاپ‌ها استفاده می‌شود.')
        await load()
      } catch (e) {
        setError(errorText(e))
      }
    }
    reader.readAsDataURL(file)
  }

  const previewHtml = useMemo(
    () => `<style>${printStyles(design)}</style>${renderBody(design, company, SAMPLE)}`,
    [design, company],
  )

  const toggleRow = (list: Toggle[]) => (
    <div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
      {list.map((item) => (
        <label
          key={String(item.key)}
          className="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-[12px] text-muted transition-colors hover:bg-bg-soft hover:text-text"
        >
          <input
            type="checkbox"
            checked={Boolean(design[item.key])}
            onChange={(event) => patch({ [item.key]: event.target.checked } as Partial<TemplateDesign>)}
          />
          <span className="flex-1">{item.label}</span>
          {item.hint && <span className="text-[10px] text-faint">{item.hint}</span>}
        </label>
      ))}
    </div>
  )

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">ابزار چاپ</div>
          <h1>طراح قالب چاپ</h1>
          <p>
            هر بخش را روشن یا خاموش کنید و نتیجه را در همان لحظه در اندازه‌ی واقعی کاغذ ببینید.
            پیش‌نمایش دقیقاً با همان موتوری رسم می‌شود که چاپگر می‌گیرد.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="ghost" onClick={testPrint}>
            <Printer className="size-4" aria-hidden /> چاپ آزمایشی
          </button>
          <button className="primary" onClick={save}>
            <Save className="size-4" aria-hidden /> ذخیره قالب
          </button>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="grid grid-cols-12 gap-4">
        {/* ---------------- تنظیمات قالب ---------------- */}
        <div className="col-span-12 flex flex-col gap-4 xl:col-span-7">
          <Card>
            <CardHeader title="مشخصات قالب" subtitle="نوع سند تعیین می‌کند این قالب کجا استفاده شود" />
            <div className="filter-grid">
              <label>
                <span>نام قالب</span>
                <input value={name} onChange={(event) => setName(event.target.value)} />
              </label>
              <label>
                <span>نوع سند</span>
                <Select
                  value={kind}
                  aria-label="نوع سند"
                  onChange={(event) => {
                    const next = event.target.value as TemplateKind
                    setKind(next)
                    setDesign(defaultDesign(next))
                  }}
                >
                  {KINDS.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>اندازه کاغذ</span>
                <Select
                  value={design.paper}
                  aria-label="اندازه کاغذ"
                  onChange={(event) => patch({ paper: event.target.value as TemplateDesign['paper'] })}
                >
                  {Object.entries(PAPER_LABEL).map(([value, label]) => (
                    <option key={value} value={value}>
                      {label}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>عنوان چاپ‌شده</span>
                <input value={design.title} onChange={(event) => patch({ title: event.target.value })} />
              </label>
              <label>
                <span>بزرگی قلم ({Math.round(design.fontScale * 100)}٪)</span>
                <input
                  type="range"
                  min={70}
                  max={140}
                  value={Math.round(design.fontScale * 100)}
                  onChange={(event) => patch({ fontScale: Number(event.target.value) / 100 })}
                />
              </label>
              <label className="inline-check">
                <input
                  type="checkbox"
                  checked={isDefault}
                  onChange={(event) => setIsDefault(event.target.checked)}
                />
                قالب پیش‌فرض این نوع سند
              </label>
            </div>
          </Card>

          <Card>
            <CardHeader
              title="سربرگ"
              subtitle="نام، تلفن و لوگوی مجموعه از مرکز تنظیمات خوانده می‌شوند"
              action={
                <label className="table-action cursor-pointer">
                  <Upload className="size-3.5" aria-hidden /> بارگذاری لوگو
                  <input
                    type="file"
                    accept="image/png,image/jpeg,image/svg+xml"
                    className="hidden"
                    onChange={(event) => event.target.files?.[0] && uploadLogo(event.target.files[0])}
                  />
                </label>
              }
            />
            {toggleRow(HEADER_TOGGLES)}
            <div className="mt-2 flex items-center gap-3 rounded-xl bg-bg-soft px-3 py-2 text-[11.5px] text-muted">
              <span>
                مجموعه: <b className="text-text">{company.name}</b>
              </span>
              {company.phone && <span>تلفن: {company.phone}</span>}
              <span className={company.logo ? 'text-success' : 'text-warning'}>
                {company.logo ? 'لوگو بارگذاری شده' : 'لوگو بارگذاری نشده'}
              </span>
            </div>
            {design.showLogo && company.logo && (
              <label className="mt-2 block">
                <span className="text-[11px] text-muted">
                  ارتفاع لوگو: {design.logoHeightMm} میلی‌متر
                </span>
                <input
                  type="range"
                  min={6}
                  max={30}
                  value={design.logoHeightMm}
                  onChange={(event) => patch({ logoHeightMm: Number(event.target.value) })}
                />
              </label>
            )}
          </Card>

          <Card>
            <CardHeader title="اطلاعات سند" subtitle="چه چیزی بالای جدول اقلام بیاید" />
            {toggleRow(DOC_TOGGLES)}
          </Card>

          <Card>
            <CardHeader
              title="ستون‌های جدول اقلام"
              subtitle="ترتیب انتخاب، همان ترتیب چاپ است"
              action={<Badge tone="neutral" dot={false}>{design.columns.length} ستون</Badge>}
            />
            <div className="flex flex-wrap gap-1.5">
              {ALL_COLUMNS.map((column) => {
                const active = design.columns.includes(column)
                return (
                  <button
                    key={column}
                    type="button"
                    onClick={() =>
                      patch({
                        columns: active
                          ? design.columns.filter((item) => item !== column)
                          : [...design.columns, column],
                      })
                    }
                    className={
                      active
                        ? 'rounded-full border border-primary bg-primary px-3 py-1.5 text-[11px] font-semibold text-[var(--on-primary)]'
                        : 'rounded-full border border-border bg-card px-3 py-1.5 text-[11px] font-semibold text-muted hover:border-border-strong hover:text-text'
                    }
                  >
                    {COLUMN_LABEL[column]}
                  </button>
                )
              })}
            </div>
            {design.columns.length === 0 && (
              <p className="mt-2 text-[11px] text-danger">حداقل یک ستون لازم است.</p>
            )}
          </Card>

          <Card>
            <CardHeader title="جمع‌ها و پاورقی" subtitle="بخش پایانی سند" />
            {toggleRow(FOOTER_TOGGLES)}
            <label className="mt-2 block">
              <span className="text-[11px] text-muted">پیام پاورقی</span>
              <input
                value={design.footerNote}
                onChange={(event) => patch({ footerNote: event.target.value })}
                placeholder="از خرید شما سپاسگزاریم"
              />
            </label>
          </Card>
        </div>

        {/* ---------------- پیش‌نمایش ---------------- */}
        <div className="col-span-12 xl:col-span-5">
          <div className="sticky top-4 flex flex-col gap-4">
            <Card>
              <CardHeader
                title="پیش‌نمایش زنده"
                subtitle={`${PAPER_LABEL[design.paper]} — عرض چاپ ${PAPER_WIDTH_MM[design.paper]} میلی‌متر`}
                action={
                  <Badge tone="neutral" dot={false}>
                    مبلغ نمونه: {money(SAMPLE.total)} ریال
                  </Badge>
                }
              />
              <div className="max-h-[60vh] overflow-auto rounded-xl bg-bg-soft p-4">
                <div
                  className="mx-auto bg-white shadow-[0_4px_18px_rgba(0,0,0,.18)]"
                  style={{ width: `${PAPER_WIDTH_MM[design.paper]}mm`, padding: '4mm' }}
                >
                  {/* پیش‌نمایش با همان HTML خروجی چاپ */}
                  <div dangerouslySetInnerHTML={{ __html: previewHtml }} />
                </div>
              </div>
            </Card>

            <Card>
              <CardHeader title="قالب‌های ذخیره‌شده" subtitle="برای ویرایش روی نام قالب بزنید" />
              {items.length === 0 ? (
                <EmptyState title="قالبی ثبت نشده است." hint="از بالا یک قالب بسازید و ذخیره کنید." />
              ) : (
                <ul className="flex flex-col gap-1.5">
                  {items.map((template) => (
                    <li
                      key={template.id}
                      className="flex items-center gap-2 rounded-xl border border-border px-3 py-2"
                    >
                      <button
                        type="button"
                        className="min-w-0 flex-1 text-start"
                        onClick={() => edit(template)}
                      >
                        <span className="block truncate text-xs font-bold text-text">
                          {template.name}
                        </span>
                        <span className="block text-[10.5px] text-muted">
                          {KINDS.find((item) => item.value === template.template_type)?.label ??
                            template.template_type}
                          {template.is_default ? ' — پیش‌فرض' : ''}
                        </span>
                      </button>
                      <button
                        type="button"
                        aria-label="حذف قالب"
                        className="icon-btn danger-icon"
                        onClick={() => remove(template.id)}
                      >
                        <Trash2 className="size-3.5" aria-hidden />
                      </button>
                    </li>
                  ))}
                </ul>
              )}
              <button type="button" className="ghost mt-3 w-full" onClick={() => startNew()}>
                قالب جدید
              </button>
            </Card>
          </div>
        </div>
      </div>
    </section>
  )
}
