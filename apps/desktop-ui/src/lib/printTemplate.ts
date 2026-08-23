/**
 * قالب چاپ — مدل ساختاریافته به‌جای HTML خام.
 *
 * ## چرا JSON و نه HTML
 * نسخه‌ی قبلی از کاربر می‌خواست HTML بنویسد. حسابدار HTML نمی‌نویسد. با یک
 * مدل ساختاریافته، طراح قالب می‌تواند **بصری** باشد (تیک زدن بخش‌ها و دیدن
 * نتیجه در همان لحظه) و خروجی همیشه HTML معتبر و چاپ‌پذیر است.
 *
 * قالب‌های قدیمیِ HTML خام هم پشتیبانی می‌شوند: اگر محتوای ذخیره‌شده با
 * `{` شروع نشود، همان HTML مستقیم استفاده می‌گردد.
 *
 * ## اندازه‌ی کاغذ
 * پرینترهای حرارتی رایج (اپسون TM-T20/T82، بیکسولون SRP-330/350،
 * رونگتا RP326، اسکار POS88C، میوا TP1000، ایکس‌پرینتر) روی ویندوز به‌صورت
 * چاپگر معمولی نصب می‌شوند. پس چاپ با `@page` و عرض دقیق کاغذ انجام
 * می‌شود و نیازی به ارسال دستور ESC/POS نیست.
 */

export type PaperSize = '80mm' | '58mm' | 'A4' | 'A5'

export type TemplateKind = 'invoice' | 'receipt' | 'journal' | 'report' | 'label'

/** ستون‌های قابل انتخاب برای جدول اقلام. */
export type LineColumn =
  | 'row'
  | 'code'
  | 'name'
  | 'quantity'
  | 'unit'
  | 'unit_price'
  | 'discount'
  | 'vat'
  | 'line_total'

export const COLUMN_LABEL: Record<LineColumn, string> = {
  row: 'ردیف',
  code: 'کد کالا',
  name: 'شرح کالا',
  quantity: 'مقدار',
  unit: 'واحد',
  unit_price: 'فی واحد',
  discount: 'تخفیف',
  vat: 'ارزش افزوده',
  line_total: 'مبلغ',
}

export type TemplateDesign = {
  version: 1
  paper: PaperSize
  /** سربرگ */
  showLogo: boolean
  logoHeightMm: number
  showCompanyName: boolean
  showPhone: boolean
  showAddress: boolean
  showEconomicCode: boolean
  title: string
  /** اطلاعات سند */
  showDocumentNumber: boolean
  showDate: boolean
  showParty: boolean
  showPartyPhone: boolean
  /** جدول اقلام */
  columns: LineColumn[]
  zebra: boolean
  /** جمع‌ها */
  showSubtotal: boolean
  showDiscount: boolean
  showVat: boolean
  showTotal: boolean
  showAmountInWords: boolean
  /** پاورقی */
  footerNote: string
  showSignature: boolean
  showBarcode: boolean
  fontScale: number
}

export const PAPER_LABEL: Record<PaperSize, string> = {
  '80mm': 'رول حرارتی ۸۰ میلی‌متر',
  '58mm': 'رول حرارتی ۵۸ میلی‌متر',
  A4: 'کاغذ A4',
  A5: 'کاغذ A5',
}

/** عرض قابل چاپ هر کاغذ بر حسب میلی‌متر. */
export const PAPER_WIDTH_MM: Record<PaperSize, number> = {
  '80mm': 72,
  '58mm': 48,
  A4: 186,
  A5: 128,
}

const RECEIPT_COLUMNS: LineColumn[] = ['name', 'quantity', 'unit_price', 'line_total']
const INVOICE_COLUMNS: LineColumn[] = [
  'row',
  'code',
  'name',
  'quantity',
  'unit',
  'unit_price',
  'discount',
  'vat',
  'line_total',
]

/** قالب پیش‌فرض برای هر نوع سند. */
export function defaultDesign(kind: TemplateKind): TemplateDesign {
  const receipt = kind === 'receipt'
  const label = kind === 'label'
  return {
    version: 1,
    paper: receipt ? '80mm' : label ? '58mm' : 'A4',
    showLogo: !label,
    logoHeightMm: receipt ? 12 : 16,
    showCompanyName: true,
    showPhone: true,
    showAddress: !receipt && !label,
    showEconomicCode: kind === 'invoice',
    title: receipt ? 'رسید فروش' : kind === 'journal' ? 'سند حسابداری' : 'فاکتور فروش',
    showDocumentNumber: !label,
    showDate: !label,
    showParty: !label,
    showPartyPhone: kind === 'invoice',
    columns: receipt ? RECEIPT_COLUMNS : label ? ['name', 'unit_price'] : INVOICE_COLUMNS,
    zebra: !receipt,
    showSubtotal: !label,
    showDiscount: !label,
    showVat: !label,
    showTotal: !label,
    showAmountInWords: kind === 'invoice',
    footerNote: receipt ? 'از خرید شما سپاسگزاریم' : '',
    showSignature: kind === 'invoice',
    showBarcode: !label,
    fontScale: receipt ? 0.95 : 1,
  }
}

/** خواندن قالب ذخیره‌شده؛ اگر JSON نبود، `null` تا HTML خام استفاده شود. */
export function parseDesign(stored: string, kind: TemplateKind): TemplateDesign | null {
  const trimmed = stored.trim()
  if (!trimmed.startsWith('{')) return null
  try {
    const parsed = JSON.parse(trimmed) as Partial<TemplateDesign>
    if (parsed.version !== 1) return null
    return { ...defaultDesign(kind), ...parsed }
  } catch {
    return null
  }
}

// ---------------------------------------------------------------------------
// داده‌ی چاپ
// ---------------------------------------------------------------------------

export type PrintLine = {
  code: string
  name: string
  quantity: number
  unit: string
  unit_price: number
  discount: number
  vat: number
  line_total: number
}

export type PrintDocument = {
  title?: string
  number: string
  date: string
  partyName: string
  partyPhone?: string
  lines: PrintLine[]
  subtotal: number
  discount: number
  vat: number
  total: number
}

export type CompanyIdentity = {
  name: string
  phone: string
  address: string
  economicCode: string
  logo: string
}

// ---------------------------------------------------------------------------
// تبدیل عدد به حروف فارسی
// ---------------------------------------------------------------------------

const ONES = ['', 'یک', 'دو', 'سه', 'چهار', 'پنج', 'شش', 'هفت', 'هشت', 'نه']
const TEENS = [
  'ده',
  'یازده',
  'دوازده',
  'سیزده',
  'چهارده',
  'پانزده',
  'شانزده',
  'هفده',
  'هجده',
  'نوزده',
]
const TENS = ['', '', 'بیست', 'سی', 'چهل', 'پنجاه', 'شصت', 'هفتاد', 'هشتاد', 'نود']
const HUNDREDS = [
  '',
  'یکصد',
  'دویست',
  'سیصد',
  'چهارصد',
  'پانصد',
  'ششصد',
  'هفتصد',
  'هشتصد',
  'نهصد',
]
const SCALES = ['', ' هزار', ' میلیون', ' میلیارد', ' هزار میلیارد']

function tripletToWords(value: number): string {
  const parts: string[] = []
  const hundreds = Math.floor(value / 100)
  const rest = value % 100
  if (hundreds > 0) parts.push(HUNDREDS[hundreds])
  if (rest >= 10 && rest <= 19) {
    parts.push(TEENS[rest - 10])
  } else {
    const tens = Math.floor(rest / 10)
    const ones = rest % 10
    if (tens > 0) parts.push(TENS[tens])
    if (ones > 0) parts.push(ONES[ones])
  }
  return parts.join(' و ')
}

/**
 * مبلغ به حروف — در فاکتور رسمی ایرانی الزامی است.
 *
 * خواندن مبلغ به حروف، جلوی جعل رقم را می‌گیرد؛ به همین دلیل در صورتحساب
 * رسمی درج می‌شود.
 */
export function amountInWords(rials: number): string {
  const value = Math.floor(Math.abs(rials))
  if (value === 0) return 'صفر ریال'
  const triplets: number[] = []
  let rest = value
  while (rest > 0) {
    triplets.push(rest % 1000)
    rest = Math.floor(rest / 1000)
  }
  const parts: string[] = []
  for (let index = triplets.length - 1; index >= 0; index -= 1) {
    if (triplets[index] === 0) continue
    parts.push(`${tripletToWords(triplets[index])}${SCALES[index] ?? ''}`)
  }
  const sign = rials < 0 ? 'منفی ' : ''
  return `${sign}${parts.join(' و ')} ریال`
}

// ---------------------------------------------------------------------------
// رندر
// ---------------------------------------------------------------------------

const faNumber = new Intl.NumberFormat('fa-IR', { maximumFractionDigits: 3 })
const money = (value: number) => faNumber.format(Math.round(value))

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function cellValue(column: LineColumn, line: PrintLine, index: number): string {
  switch (column) {
    case 'row':
      return faNumber.format(index + 1)
    case 'code':
      return escapeHtml(line.code)
    case 'name':
      return escapeHtml(line.name)
    case 'quantity':
      return faNumber.format(line.quantity)
    case 'unit':
      return escapeHtml(line.unit)
    case 'unit_price':
      return money(line.unit_price)
    case 'discount':
      return money(line.discount)
    case 'vat':
      return money(line.vat)
    case 'line_total':
      return money(line.line_total)
  }
}

const NUMERIC: LineColumn[] = ['quantity', 'unit_price', 'discount', 'vat', 'line_total', 'row']

/** استایل مشترک خروجی چاپ — درون خود سند تا در پنجره‌ی چاپ هم اعمال شود. */
export function printStyles(design: TemplateDesign): string {
  const width = PAPER_WIDTH_MM[design.paper]
  const roll = design.paper === '80mm' || design.paper === '58mm'
  const base = (roll ? 11 : 12) * design.fontScale
  return `
    @page { size: ${roll ? `${design.paper} auto` : design.paper}; margin: ${roll ? '3mm' : '10mm'}; }
    * { box-sizing: border-box; }
    body {
      margin: 0; direction: rtl;
      font-family: Vazirmatn, 'Segoe UI', Tahoma, sans-serif;
      font-size: ${base}px; color: #000; background: #fff;
      width: ${width}mm;
    }
    .np-head { text-align: center; border-bottom: 1px solid #000; padding-bottom: 4px; margin-bottom: 6px; }
    .np-head img { height: ${design.logoHeightMm}mm; object-fit: contain; display: block; margin: 0 auto 3px; }
    .np-name { font-size: ${base * 1.25}px; font-weight: 800; }
    .np-sub { font-size: ${base * 0.85}px; }
    .np-title { font-size: ${base * 1.1}px; font-weight: 700; margin: 5px 0; text-align: center; }
    .np-meta { display: flex; flex-wrap: wrap; gap: 2px 12px; font-size: ${base * 0.9}px; margin-bottom: 6px; }
    .np-meta span { white-space: nowrap; }
    table { width: 100%; border-collapse: collapse; font-size: ${base * 0.92}px; }
    th, td { padding: ${roll ? '2px 1px' : '4px 5px'}; border-bottom: 1px solid #999; text-align: right; }
    th { border-bottom: 1px solid #000; font-weight: 700; }
    td.num, th.num { text-align: left; font-variant-numeric: tabular-nums; }
    ${design.zebra ? 'tbody tr:nth-child(even){background:#f2f2f2;}' : ''}
    .np-totals { margin-top: 6px; font-size: ${base * 0.95}px; }
    .np-totals div { display: flex; justify-content: space-between; padding: 2px 0; }
    .np-totals .grand { border-top: 1px solid #000; margin-top: 3px; padding-top: 4px; font-weight: 800; font-size: ${base * 1.1}px; }
    .np-words { margin-top: 5px; font-size: ${base * 0.85}px; border: 1px solid #999; padding: 3px 5px; }
    .np-foot { margin-top: 8px; text-align: center; font-size: ${base * 0.85}px; }
    .np-sign { display: flex; justify-content: space-between; margin-top: 14px; font-size: ${base * 0.85}px; }
    .np-sign span { border-top: 1px dotted #000; padding-top: 3px; width: 45%; text-align: center; }
    .np-barcode { margin-top: 6px; text-align: center; font-family: 'Libre Barcode 39', monospace; letter-spacing: 2px; font-size: ${base * 0.9}px; }
  `
}

/** ساخت بدنه‌ی HTML سند از روی قالب و داده. */
export function renderBody(
  design: TemplateDesign,
  company: CompanyIdentity,
  document_: PrintDocument,
): string {
  const parts: string[] = []

  // --- سربرگ ---
  const head: string[] = []
  if (design.showLogo && company.logo) head.push(`<img src="${company.logo}" alt="" />`)
  if (design.showCompanyName) head.push(`<div class="np-name">${escapeHtml(company.name)}</div>`)
  const sub: string[] = []
  if (design.showPhone && company.phone) sub.push(`تلفن: ${escapeHtml(company.phone)}`)
  if (design.showAddress && company.address) sub.push(escapeHtml(company.address))
  if (design.showEconomicCode && company.economicCode)
    sub.push(`کد اقتصادی: ${escapeHtml(company.economicCode)}`)
  if (sub.length) head.push(`<div class="np-sub">${sub.join(' — ')}</div>`)
  if (head.length) parts.push(`<div class="np-head">${head.join('')}</div>`)

  parts.push(`<div class="np-title">${escapeHtml(document_.title ?? design.title)}</div>`)

  // --- اطلاعات سند ---
  const meta: string[] = []
  if (design.showDocumentNumber) meta.push(`<span>شماره: ${escapeHtml(document_.number)}</span>`)
  if (design.showDate) meta.push(`<span>تاریخ: ${escapeHtml(document_.date)}</span>`)
  if (design.showParty) meta.push(`<span>طرف حساب: ${escapeHtml(document_.partyName)}</span>`)
  if (design.showPartyPhone && document_.partyPhone)
    meta.push(`<span>تلفن: ${escapeHtml(document_.partyPhone)}</span>`)
  if (meta.length) parts.push(`<div class="np-meta">${meta.join('')}</div>`)

  // --- جدول اقلام ---
  if (design.columns.length > 0) {
    const header = design.columns
      .map(
        (column) =>
          `<th class="${NUMERIC.includes(column) ? 'num' : ''}">${COLUMN_LABEL[column]}</th>`,
      )
      .join('')
    const body = document_.lines
      .map(
        (line, index) =>
          `<tr>${design.columns
            .map(
              (column) =>
                `<td class="${NUMERIC.includes(column) ? 'num' : ''}">${cellValue(column, line, index)}</td>`,
            )
            .join('')}</tr>`,
      )
      .join('')
    parts.push(`<table><thead><tr>${header}</tr></thead><tbody>${body}</tbody></table>`)
  }

  // --- جمع‌ها ---
  const totals: string[] = []
  if (design.showSubtotal)
    totals.push(`<div><span>جمع کل</span><span>${money(document_.subtotal)}</span></div>`)
  if (design.showDiscount && document_.discount > 0)
    totals.push(`<div><span>تخفیف</span><span>${money(document_.discount)}</span></div>`)
  if (design.showVat)
    totals.push(`<div><span>ارزش افزوده</span><span>${money(document_.vat)}</span></div>`)
  if (design.showTotal)
    totals.push(
      `<div class="grand"><span>مبلغ قابل پرداخت</span><span>${money(document_.total)} ریال</span></div>`,
    )
  if (totals.length) parts.push(`<div class="np-totals">${totals.join('')}</div>`)

  if (design.showAmountInWords)
    parts.push(`<div class="np-words">به حروف: ${amountInWords(document_.total)}</div>`)

  if (design.showBarcode)
    parts.push(`<div class="np-barcode">*${escapeHtml(document_.number)}*</div>`)

  if (design.showSignature)
    parts.push('<div class="np-sign"><span>مهر و امضای فروشنده</span><span>امضای خریدار</span></div>')

  if (design.footerNote) parts.push(`<div class="np-foot">${escapeHtml(design.footerNote)}</div>`)

  return parts.join('\n')
}

/** سند HTML کامل و آماده‌ی چاپ. */
export function renderDocument(
  design: TemplateDesign,
  company: CompanyIdentity,
  document_: PrintDocument,
  copies = 1,
): string {
  const single = renderBody(design, company, document_)
  const pages = Array.from({ length: Math.max(1, copies) }, (_, index) =>
    index === 0 ? single : `<div style="page-break-before:always">${single}</div>`,
  ).join('\n')
  return `<!doctype html><html lang="fa" dir="rtl"><head><meta charset="utf-8" />
<title>${escapeHtml(document_.title ?? design.title)}</title>
<style>${printStyles(design)}</style></head><body>${pages}</body></html>`
}
