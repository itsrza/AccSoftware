/**
 * ارسال سند به چاپگر.
 *
 * ## چرا iframe و نه پنجره‌ی تازه
 * پنجره‌ی pop-up در WebView دسکتاپ ممکن است مسدود شود و کاربر چیزی نبیند.
 * یک iframe پنهان همیشه کار می‌کند، پنجره‌ی برنامه را ترک نمی‌کند و پس از
 * چاپ خودش را جمع می‌کند.
 *
 * ## چرا استایل داخل خود سند است
 * محتوای iframe سند مستقلی است و به CSS برنامه دسترسی ندارد. اگر استایل
 * بیرونی باشد، خروجی چاپ بدون قالب می‌شود — همان مشکل کلاسیک «چاپ خراب».
 */

import {
  parseDesign,
  renderDocument,
  type CompanyIdentity,
  type PrintDocument,
  type TemplateDesign,
  type TemplateKind,
} from './printTemplate'



/** چاپ یک سند HTML آماده. */
export function printHtml(html: string): Promise<void> {
  return new Promise((resolve) => {
    const frame = document.createElement('iframe')
    frame.setAttribute('aria-hidden', 'true')
    frame.style.position = 'fixed'
    frame.style.right = '-10000px'
    frame.style.bottom = '0'
    frame.style.width = '0'
    frame.style.height = '0'
    frame.style.border = '0'
    document.body.appendChild(frame)

    const cleanup = () => {
      window.setTimeout(() => {
        frame.remove()
        resolve()
      }, 400)
    }

    frame.onload = () => {
      try {
        const view = frame.contentWindow
        if (!view) {
          cleanup()
          return
        }
        view.focus()
        view.print()
      } catch {
        /* اگر چاپ ممکن نبود، صفحه نباید بشکند */
      }
      cleanup()
    }

    const doc = frame.contentDocument
    if (!doc) {
      cleanup()
      return
    }
    doc.open()
    doc.write(html)
    doc.close()
  })
}

/** چاپ یک سند بر اساس قالب ذخیره‌شده (یا قالب پیش‌فرض نوع سند). */
export function printWithTemplate(
  stored: string,
  kind: TemplateKind,
  fallback: TemplateDesign,
  company: CompanyIdentity,
  document_: PrintDocument,
  copies = 1,
): Promise<void> {
  const design = parseDesign(stored, kind) ?? fallback
  return printHtml(renderDocument(design, company, document_, copies))
}

/** خواندن هویت مجموعه از فهرست تنظیمات. */
export function companyFrom(
  settings: { key: string; value: string }[],
  fallbackName: string,
): CompanyIdentity {
  const value = (key: string, fallback = '') =>
    settings.find((item) => item.key === key)?.value ?? fallback
  const name = value('company.display_name').trim()
  const clean = (raw: string) => (raw.trim() === '—' ? '' : raw.trim())
  return {
    name: name || fallbackName,
    phone: clean(value('company.phone')),
    address: clean(value('company.address')),
    economicCode: clean(value('company.economic_code')),
    logo: value('company.logo'),
  }
}
