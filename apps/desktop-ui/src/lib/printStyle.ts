/**
 * شیوه‌نامه‌ی پنجره‌ی چاپ.
 *
 * ## چرا تنها جای مجاز رنگ خام است
 *
 * پنجره‌ی چاپ سندی مستقل است و به شیوه‌نامه و توکن‌های برنامه دسترسی ندارد؛
 * `var(--accent)` آنجا هیچ مقداری ندارد. پس رنگ‌ها باید ثابت باشند.
 *
 * مقادیر عمداً از پالت برند گرفته شده‌اند تا خروجی چاپی با محیط برنامه
 * یکسان دیده شود. هیچ صفحه‌ی دیگری اجازه‌ی رنگ خام ندارد.
 */
export const BRAND_PRINT_COLORS = {
  navy: '#21254E',
  muted: '#62748E',
  border: '#d9dfe9',
  surface: '#F6F9FF',
  gold: '#faf6ef',
  white: '#fff',
} as const

/** شیوه‌نامه‌ی آماده برای تزریق در پنجره‌ی چاپ گزارش. */
export const REPORT_PRINT_STYLE = [
  `body{font-family:Tahoma,Arial;padding:28px;color:${BRAND_PRINT_COLORS.navy}}`,
  'h1{font-size:20px;margin:0 0 4px}',
  `p{color:${BRAND_PRINT_COLORS.muted};margin:0 0 18px;font-size:12px}`,
  'table{width:100%;border-collapse:collapse;font-size:12px}',
  `th,td{border:1px solid ${BRAND_PRINT_COLORS.border};padding:7px;text-align:right}`,
  `th{background:${BRAND_PRINT_COLORS.surface}}`,
  `tr.group td{background:${BRAND_PRINT_COLORS.surface};font-weight:bold}`,
  `tr.subtotal td{background:${BRAND_PRINT_COLORS.gold};font-weight:bold}`,
  `tr.grand td{background:${BRAND_PRINT_COLORS.navy};color:${BRAND_PRINT_COLORS.white};font-weight:bold}`,
].join('')
