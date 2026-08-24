/**
 * ممیزی دور ۹ — سه ایراد گزارش‌شده‌ی کاربر در چیدمان.
 *
 * ۱. دکمه‌ی شناور با هر کلیک از چپ به راست می‌پرید.
 * ۲. دکمه‌ی «حذف داده‌ی نمونه» به آن چسبیده بود و با هم جابه‌جا می‌شدند.
 * ۳. دکمه‌های پانل «جمع فاکتور» بیش از اندازه درشت بودند.
 *
 * هر سه ایراد یک ریشه‌ی مشترک داشتند: عرض ظرف با عرض پهن‌ترین فرزندش تعیین
 * می‌شود. این تست‌ها همان ریشه را قفل می‌کنند، نه فقط ظاهر را.
 */
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const SRC = resolve(__dirname, '..')
const read = (relative: string) => readFileSync(resolve(SRC, relative), 'utf8')

const app = read('App.tsx')
const invoice = read('pages/InvoiceForm.tsx')
const theme = read('theme.css')

/** بدنه‌ی یک قاعده‌ی CSS به نام سلکتور. */
function rule(css: string, selector: string): string {
  const index = css.indexOf(`${selector} {`)
  expect(index, `قاعده‌ی «${selector}» پیدا نشد`).toBeGreaterThan(-1)
  return css.slice(index, css.indexOf('}', index))
}

/** بخش JSX دکمه‌ی شناور. */
const fabBlock = app.slice(app.indexOf('fixed bottom-6 end-6'), app.indexOf('<CommandPalette'))

describe('م۹ · دکمه‌ی شناور ایجاد سریع', () => {
  it('ک۱ — ظرف دکمه ستونی با تراز انتهایی است، پس عرض فرزندان جایش را عوض نمی‌کند', () => {
    expect(fabBlock).toContain('flex flex-col items-end')
  })

  it('ک۲ — منوی بازشو absolute است و اصلاً در چیدمان اثر ندارد', () => {
    const menu = fabBlock.slice(fabBlock.indexOf("openMenu === 'fab' &&"))
    expect(menu).toContain('absolute bottom-[calc(100%+12px)] end-0')
    // نسخه‌ی معیوب، منو را هم‌سطح دکمه و با `mb-3` می‌گذاشت.
    expect(menu).not.toContain('fade-up mb-3 w-56')
  })

  it('ک۳ — خود دکمه اندازه‌ی ثابت دارد و داخل ظرف نسبی نشسته است', () => {
    expect(fabBlock).toContain('<div className="relative">')
    expect(fabBlock).toContain('grid size-14 place-items-center')
  })

  it('ک۴ — دکمه‌ی حذف داده‌ی نمونه تمام‌عرض نیست و دکمه‌ی شناور را نمی‌کشد', () => {
    const demo = fabBlock.slice(fabBlock.indexOf('DEMO_BUILD && demo'))
    expect(demo).toContain('w-max whitespace-nowrap')
    expect(demo).not.toContain('mt-3 block w-full')
  })

  it('ک۵ — دکمه در راست‌به‌چپ سمت چپ می‌ماند (کلاس منطقی `end`)', () => {
    expect(fabBlock).toContain('fixed bottom-6 end-6')
    expect(fabBlock).not.toContain('fixed bottom-6 right-6')
  })

  it('ک۶ — پالس پس‌زمینه‌ی دکمه سر جایش است', () => {
    expect(fabBlock).toContain('fab-pulse')
    expect(theme).toContain('.fab-pulse')
  })
})

describe('م۹ · دکمه‌های پانل جمع فاکتور', () => {
  it('ک۷ — پانل جمع، چیدمان اختصاصی خودش را دارد نه `form-actions` تمام‌عرض', () => {
    expect(invoice).toContain('className="invoice-actions"')
    expect(invoice).not.toContain('className="form-actions"')
  })

  it('ک۸ — چیدمان دوستونی است تا دو دکمه‌ی چاپ کنار هم بنشینند', () => {
    const body = rule(theme, '.invoice-actions')
    expect(body).toContain('display: grid')
    expect(body).toContain('grid-template-columns: 1fr 1fr')
  })

  it('ک۹ — ارتفاع دکمه‌ها کنترل‌شده است (۳۶ پیکسل، ثبت ۴۰ پیکسل)', () => {
    expect(rule(theme, '.invoice-actions button')).toContain('height: 36px')
    expect(rule(theme, '.invoice-actions button.primary')).toContain('height: 40px')
    // دکمه‌ی «محاسبه سود» فرعی است و باید از ثبت کوچک‌تر دیده شود.
    expect(rule(theme, '.invoice-actions button.subtle')).toContain('height: 32px')
  })

  it('ک۱۰ — دکمه‌ی ثبت تمام‌عرض است و آیکن چاپ کوچک شده', () => {
    expect(invoice).toContain('className="primary wide"')
    expect(rule(theme, '.invoice-actions button.wide')).toContain('grid-column: 1 / -1')
    expect(invoice).toContain('<Icon name="print" size={14} />')
  })

  it('ک۱۱ — دکمه‌ی غیرفعال ظاهر غیرفعال دارد ولی حذف نمی‌شود', () => {
    // قاعده‌ی محصول: هیچ دکمه‌ای بی‌دلیل ناپدید نمی‌شود؛ فقط غیرفعال می‌شود.
    expect(rule(theme, '.invoice-actions button:disabled')).toContain('cursor: not-allowed')
    expect(invoice).toContain('disabled={saving || !preview}')
  })
})
