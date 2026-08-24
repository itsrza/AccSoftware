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
import { formatCount, formatRials as money, rialUnit } from '../lib/format'
import {useI18n, type TranslationKey} from '../lib/i18n'
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

const KINDS: { value: TemplateKind; labelKey: TranslationKey }[] = [
  { value: 'invoice', labelKey: 'print.kind.invoice' },
  { value: 'receipt', labelKey: 'print.kind.receipt' },
  { value: 'journal', labelKey: 'print.kind.voucher' },
  { value: 'report', labelKey: 'print.kind.report' },
  { value: 'label', labelKey: 'print.kind.label' },
]

/** نمونه‌ی داده برای پیش‌نمایش — تا کاربر قالب را با محتوای واقعی‌نما ببیند. */
const sampleDocument = (t: (key: TranslationKey) => string): PrintDocument => ({
  number: '1042',
  date: '1405/05/30',
  partyName: t('print.sample.company'),
  partyPhone: '021-88776655',
  lines: [
    { code: 'P-1001', name: t('print.sample.item1'), quantity: 3, unit: t('productForm.defaultUnit'), unit_price: 4_850_000, discount: 0, vat: 1_309_500, line_total: 15_859_500 },
    { code: 'P-1002', name: t('print.sample.item2'), quantity: 10, unit: t('productForm.defaultUnit'), unit_price: 620_000, discount: 200_000, vat: 540_000, line_total: 6_540_000 },
    { code: 'P-2010', name: t('print.sample.item3'), quantity: 25, unit: t('print.unit.metre'), unit_price: 78_000, discount: 0, vat: 175_500, line_total: 2_125_500 },
  ],
  subtotal: 24_400_000,
  discount: 200_000,
  vat: 2_025_000,
  total: 24_525_000,
})

type Toggle = { key: keyof TemplateDesign; labelKey: TranslationKey; hintKey?: TranslationKey }

const HEADER_TOGGLES: Toggle[] = [
  { key: 'showLogo', labelKey: 'print.block.logo' },
  { key: 'showCompanyName', labelKey: 'print.block.companyName' },
  { key: 'showPhone', labelKey: 'print.block.phone' },
  { key: 'showAddress', labelKey: 'print.block.address' },
  { key: 'showEconomicCode', labelKey: 'print.block.economicCode' },
]

const DOC_TOGGLES: Toggle[] = [
  { key: 'showDocumentNumber', labelKey: 'print.block.documentNumber' },
  { key: 'showDate', labelKey: 'common.date' },
  { key: 'showParty', labelKey: 'print.block.partyName' },
  { key: 'showPartyPhone', labelKey: 'print.block.partyPhone' },
]

const FOOTER_TOGGLES: Toggle[] = [
  { key: 'showSubtotal', labelKey: 'common.grandTotal' },
  { key: 'showDiscount', labelKey: 'invoiceForm.discount' },
  { key: 'showVat', labelKey: 'common.vat' },
  { key: 'showTotal', labelKey: 'print.block.payable' },
  { key: 'showAmountInWords', labelKey: 'print.block.amountInWords', hintKey: 'print.block.requiredOnOfficial' },
  { key: 'showBarcode', labelKey: 'print.block.barcode' },
  { key: 'showSignature', labelKey: 'print.block.signature' },
  { key: 'zebra', labelKey: 'print.block.zebraRows' },
]

const ALL_COLUMNS = Object.keys(COLUMN_LABEL) as LineColumn[]

export function PrintTemplates() {
  const { t } = useI18n()
  const [items, setItems] = useState<PrintTemplate[]>([])
  const [settings, setSettings] = useState<{ key: string; value: string }[]>([])
  const [editingId, setEditingId] = useState<string>()
  const [name, setName] = useState(t('print.newTemplate'))
  const [kind, setKind] = useState<TemplateKind>('invoice')
  const [design, setDesign] = useState<TemplateDesign>(() => defaultDesign('invoice'))
  const [isDefault, setIsDefault] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const company = useMemo(() => companyFrom(settings, t('app.company')), [settings])

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
    const kindKey = KINDS.find((item) => item.value === nextKind)?.labelKey
    setName(t('print.templateOf', { kind: kindKey ? t(kindKey) : '' }))
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
      setMessage(t('print.saved'))
      await load()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const remove = async (id: string) => {
    if (!confirm(t('print.confirmDelete'))) return
    try {
      await deletePrintTemplate(id)
      if (editingId === id) startNew()
      await load()
    } catch (e) {
      setError(errorText(e))
    }
  }

  const testPrint = () => printHtml(renderDocument(design, company, sampleDocument(t), 1))

  /** بارگذاری لوگو: تبدیل به Data URL و ذخیره در تنظیمات. */
  const uploadLogo = (file: File) => {
    if (file.size > 900_000) {
      setError(t('print.logoTooBig'))
      return
    }
    const reader = new FileReader()
    reader.onload = async () => {
      try {
        const dataUrl = String(reader.result ?? '')
        await setSetting('company.logo', dataUrl)
        setMessage(t('print.logoSaved'))
        await load()
      } catch (e) {
        setError(errorText(e))
      }
    }
    reader.readAsDataURL(file)
  }

  const previewHtml = useMemo(
    () => `<style>${printStyles(design)}</style>${renderBody(design, company, sampleDocument(t))}`,
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
          <span className="flex-1">{t(item.labelKey)}</span>
          {item.hintKey && <span className="text-[10px] text-faint">{t(item.hintKey)}</span>}
        </label>
      ))}
    </div>
  )

  return (
    <section className="page flex flex-col gap-4">
      <div className="page-head">
        <div>
          <div className="eyebrow">{t('print.eyebrow')}</div>
          <h1>{t('print.title')}</h1>
          <p>
            {t('print.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="ghost" onClick={testPrint}>
            <Printer className="size-4" aria-hidden /> {t('print.testPrint')}
          </button>
          <button className="primary" onClick={save}>
            <Save className="size-4" aria-hidden /> {t('print.saveTemplate')}
          </button>
        </div>
      </div>

      {error && <div className="error-box">{error}</div>}
      {message && <div className="success-box">{message}</div>}

      <div className="grid grid-cols-12 gap-4">
        {/* ---------------- تنظیمات قالب ---------------- */}
        <div className="col-span-12 flex flex-col gap-4 xl:col-span-7">
          <Card>
            <CardHeader title={t('print.templateInfo')} subtitle={t('print.kindHint')} />
            <div className="filter-grid">
              <label>
                <span>{t('print.templateName')}</span>
                <input value={name} onChange={(event) => setName(event.target.value)} />
              </label>
              <label>
                <span>{t('print.documentKind')}</span>
                <Select
                  value={kind}
                  aria-label={t('print.documentKind')}
                  onChange={(event) => {
                    const next = event.target.value as TemplateKind
                    setKind(next)
                    setDesign(defaultDesign(next))
                  }}
                >
                  {KINDS.map((item) => (
                    <option key={item.value} value={item.value}>
                      {t(item.labelKey)}
                    </option>
                  ))}
                </Select>
              </label>
              <label>
                <span>{t('print.paperSize')}</span>
                <Select
                  value={design.paper}
                  aria-label={t('print.paperSize')}
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
                <span>{t('print.printedTitle')}</span>
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
                {t('print.defaultForKind')}
              </label>
            </div>
          </Card>

          <Card>
            <CardHeader
              title={t('print.header')}
              subtitle={t('print.headerHint')}
              action={
                <label className="table-action cursor-pointer">
                  <Upload className="size-3.5" aria-hidden /> {t('print.uploadLogo')}
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
                {t('print.companyLabel')} <b className="text-text">{company.name}</b>
              </span>
              {company.phone && <span>تلفن: {company.phone}</span>}
              <span className={company.logo ? 'text-success' : 'text-warning'}>
                {company.logo ? t('print.logoLoaded') : t('print.logoMissing')}
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
            <CardHeader title={t('print.documentInfo')} subtitle={t('print.documentInfoHint')} />
            {toggleRow(DOC_TOGGLES)}
          </Card>

          <Card>
            <CardHeader
              title={t('print.lineColumns')}
              subtitle={t('print.orderHint')}
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
              <p className="mt-2 text-[11px] text-danger">{t('print.needColumn')}</p>
            )}
          </Card>

          <Card>
            <CardHeader title={t('print.totalsFooter')} subtitle={t('print.footerSection')} />
            {toggleRow(FOOTER_TOGGLES)}
            <label className="mt-2 block">
              <span className="text-[11px] text-muted">{t('print.footerMessage')}</span>
              <input
                value={design.footerNote}
                onChange={(event) => patch({ footerNote: event.target.value })}
                placeholder={t('print.thanks')}
              />
            </label>
          </Card>
        </div>

        {/* ---------------- پیش‌نمایش ---------------- */}
        <div className="col-span-12 xl:col-span-5">
          <div className="sticky top-4 flex flex-col gap-4">
            <Card>
              <CardHeader
                title={t('print.livePreview')}
                subtitle={t('print.paperNote', {
                  paper: PAPER_LABEL[design.paper],
                  mm: formatCount(PAPER_WIDTH_MM[design.paper]),
                })}
                action={
                  <Badge tone="neutral" dot={false}>
                    {t('print.sampleAmount', {
                      amount: money(sampleDocument(t).total),
                      unit: rialUnit(),
                    })}
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
              <CardHeader title={t('print.savedTemplates')} subtitle={t('print.clickToEdit')} />
              {items.length === 0 ? (
                <EmptyState title={t('print.empty')} hint={t('print.emptyHint')} />
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
                          {(() => {
                            const key = KINDS.find(
                              (item) => item.value === template.template_type,
                            )?.labelKey
                            return key ? t(key) : template.template_type
                          })()}
                          {template.is_default ? t('print.defaultSuffix') : ''}
                        </span>
                      </button>
                      <button
                        type="button"
                        aria-label={t('print.deleteTemplate')}
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
                {t('print.newTemplate')}
              </button>
            </Card>
          </div>
        </div>
      </div>
    </section>
  )
}
