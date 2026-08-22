/**
 * واژه‌نامه‌ی وضعیت چک — دقیقاً مطابق ماشین حالت هسته‌ی مالی.
 *
 * قاعده: رابط کاربری هرگز نباید وضعیت انگلیسی به کاربر نشان دهد، و هرگز نباید
 * فهرست گذارهای مجاز را از خودش بسازد. برچسب‌ها اینجا هستند، ولی «چه گذاری
 * مجاز است» را همیشه backend می‌گوید (`getCheckTransitionOptions`).
 */

export const CHECK_STATUS_LABELS: Record<string, string> = {
  in_hand: 'موجود',
  deposited: 'واگذار شده به بانک',
  collected: 'وصول شده',
  cashed: 'نقد شده',
  endorsed: 'خرج شده',
  bounced: 'برگشتی',
  returned: 'عودت شده',
  void: 'باطل شده',
  outstanding: 'پرداختی در جریان',
  paid: 'پرداخت شده',
  memo_in_hand: 'انتظامی موجود',
  memo_returned: 'انتظامی عودت شده',
}

/** برچسب فارسی وضعیت؛ اگر وضعیت ناشناخته بود خود مقدار برگردانده می‌شود. */
export function checkStatusLabel(status: string): string {
  return CHECK_STATUS_LABELS[status] ?? status
}

/** رنگ‌بندی معنایی: سبز پایان موفق، قرمز مشکل، خاکستری خنثی، کهربایی در جریان. */
export type CheckStatusTone = 'done' | 'danger' | 'pending' | 'neutral'

export function checkStatusTone(status: string): CheckStatusTone {
  switch (status) {
    case 'collected':
    case 'cashed':
    case 'paid':
      return 'done'
    case 'bounced':
    case 'returned':
      return 'danger'
    case 'void':
    case 'memo_in_hand':
    case 'memo_returned':
      return 'neutral'
    default:
      return 'pending'
  }
}

/** وضعیت‌هایی که مانده‌ی جاری شرکت را تشکیل می‌دهند (نه پایانی، نه انتظامی). */
export const OPEN_CHECK_STATUSES = ['in_hand', 'deposited', 'endorsed', 'outstanding']

export function isOpenCheck(status: string): boolean {
  return OPEN_CHECK_STATUSES.includes(status)
}
