import { UserRound } from 'lucide-react'
import { cn } from '../lib/cn'

/**
 * نشان کاربر.
 *
 * ## چرا جزء مشترک
 * تصویر پروفایل در سه جا دیده می‌شود: نوار بالا، پای منوی کناری و مرکز
 * تنظیمات. اگر هرکدام خودش را بسازد، دیر یا زود سه ظاهر متفاوت می‌شوند.
 *
 * ## چرا پیش‌فرض گرادیان طلایی است
 * تا وقتی کاربر تصویری انتخاب نکرده، نشان باید **عمدی** به نظر برسد نه
 * ناقص. گرادیان طلایی همان رنگ تأکید سازمانی است و آیکن کاربر داخلش
 * می‌گوید این جای تصویر شخص است.
 */
export function Avatar({
  src,
  name,
  size = 28,
  className,
}: {
  src?: string
  name: string
  /** قطر بر حسب پیکسل. */
  size?: number
  className?: string
}) {
  const style = { width: size, height: size }
  if (src) {
    return (
      <img
        src={src}
        alt={name}
        style={style}
        className={cn('shrink-0 rounded-full border border-border object-cover', className)}
      />
    )
  }
  return (
    <span
      style={style}
      aria-hidden
      className={cn(
        'grid shrink-0 place-items-center rounded-full bg-gradient-to-br from-[#e7bd75] to-[#c8923c] text-[#21254E]',
        className,
      )}
    >
      <UserRound style={{ width: size * 0.55, height: size * 0.55 }} aria-hidden />
    </span>
  )
}
