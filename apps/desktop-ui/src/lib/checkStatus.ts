/**
 * واژه‌نامه‌ی وضعیت چک — دقیقاً مطابق ماشین حالت هسته‌ی مالی.
 *
 * قاعده: رابط کاربری هرگز نباید وضعیت انگلیسی به کاربر نشان دهد، و هرگز نباید
 * فهرست گذارهای مجاز را از خودش بسازد. برچسب‌ها اینجا هستند، ولی «چه گذاری
 * مجاز است» را همیشه backend می‌گوید (`getCheckTransitionOptions`).
 */
import { translate, type Locale, type TranslationKey } from './i18n'

export const CHECK_STATUSES = [
  'in_hand',
  'deposited',
  'collected',
  'cashed',
  'endorsed',
  'bounced',
  'returned',
  'void',
  'outstanding',
  'paid',
  'memo_in_hand',
  'memo_returned',
] as const

/** برچسب وضعیت به زبان فعال؛ وضعیت ناشناخته خودش برگردانده می‌شود. */
export function checkStatusLabel(status: string, locale: Locale = 'fa'): string {
  if (!(CHECK_STATUSES as readonly string[]).includes(status)) return status
  return translate(locale, `check.status.${status}` as TranslationKey)
}

/** نگاشت وضعیت→برچسب فارسی، برای جاهایی که فهرست کامل لازم است. */
export const CHECK_STATUS_LABELS: Record<string, string> = Object.fromEntries(
  CHECK_STATUSES.map((status) => [status, checkStatusLabel(status)]),
)

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
