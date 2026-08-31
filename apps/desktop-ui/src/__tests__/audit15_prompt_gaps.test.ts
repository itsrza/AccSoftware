/**
 * @vitest-environment node
 *
 * ممیزی دور ۱۵ — قرارداد منبع: اصلاحات پرامپت‌های مرجع روی میزبان.
 *
 * این‌ها دیوار پشتیبان رفتاری‌های امنیتی/حسابداری‌اند که در Rust تست
 * واحد مستقیم نداریم (میزبان در CI ویندوز کامپایل می‌شود)؛ قرارداد متن
 * سورس همان الگوی جاافتاده‌ی این مخزن است (audit8/audit10/…).
 */
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const ROOT = resolve(__dirname, '../../../..')
const read = (path: string) =>
  readFileSync(resolve(ROOT, path), 'utf8')

describe('م۱۵ · قرارداد اصلاحات پرامپت‌های مرجع', () => {
  it('ق۱ — conn() هر اتصال را با سه پراگمای یکپارچگی باز می‌کند', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    const start = main.indexOf('pub(crate) fn conn(')
    const body = main.slice(start, start + 1400)
    expect(body).toContain('foreign_keys')
    expect(body).toContain('journal_mode')
    expect(body).toContain('synchronous')
    expect(body).toContain('APP-002')
    expect(body).toContain('APP-003')
    expect(body).toContain('APP-004')
  })

  it('ق۲ — هیچ حساب hardcoded در میزبان نمانده', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).not.toMatch(/"acc-1[0-9]{3}"/)
    expect(main).not.toMatch(/"acc-[245][0-9]{3}"/)
    expect(main).toContain('get_account_mapping(')
  })

  it('ق۳ — پست فاکتور: تفکیک مالیات/تخفیف + رزرو + آزادسازی', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).toContain('invoice_posting_lines(')
    expect(main).toContain('reserved_quantity')
    expect(main).toContain("reference_type='invoice' AND reference_id=?1 AND status='reserved'")
    expect(main).toContain('tax_payable_default')
    expect(main).toContain('sales_discount_default')
  })

  it('ق۴ — چک برگشتیِ قبل از وصول سند پیگیری می‌سازد', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).toContain("check_bounce_pending")
    expect(main).toContain('check_bounce_tracking_default')
    expect(main).toContain('برگشت چک قبل از وصول')
  })

  it('ق۵ — SSRF: ریدایرکت خاموش + بررسی مجدد Allowlist + مقایسه کوچک‌حرفی', () => {
    const api = read('apps/desktop-host/src-tauri/src/api_profiles.rs')
    expect(api).toContain('Policy::none()')
    expect(api).toContain('host_allowed(')
    expect(api).toContain('to_lowercase()')
    expect(api).toContain('API-027')
    expect(api).toContain('is_redirection()')
  })

  it('ق۶ — current_user فقط کاربر فعال را برمی‌گرداند و نشست را می‌بندد', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    const start = main.indexOf('fn current_user(')
    const body = main.slice(start, start + 1200)
    expect(body).toContain('is_active=1')
    expect(body).toContain('QueryReturnedNoRows')
  })

  it('ق۷ — دستورهای نگاشت حساب ثبت و در api صدا زده می‌شوند', () => {
    const main = read('apps/desktop-host/src-tauri/src/main.rs')
    expect(main).toContain('get_account_mappings,')
    expect(main).toContain('set_account_mapping,')
    expect(main).toContain('accounting.settings.edit')
    const api = read('apps/desktop-ui/src/api.ts')
    expect(api).toContain("'get_account_mappings'")
    expect(api).toContain("'set_account_mapping'")
  })

  it('ق۸ — هسته: جدول نگاشت + تابع خالص خطوط سند', () => {
    const db = read('crates/novin-core/src/db/mod.rs')
    expect(db).toContain('CREATE TABLE IF NOT EXISTS account_mappings')
    expect(db).toContain('check_bounce_tracking_default')
    const inv = read('crates/novin-core/src/invoicing.rs')
    expect(inv).toContain('pub fn invoice_posting_lines(')
    // تست رفتاری هسته موجود است
    const tests = read('crates/novin-core/tests/audit15_prompt_gaps.rs')
    expect(tests).toContain('p03_sales_posting_splits_tax_and_discount')
    expect(tests).toContain('p07_reservation_released_on_post')
  })
})
