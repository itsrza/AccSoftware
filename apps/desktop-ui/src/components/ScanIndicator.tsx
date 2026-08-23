import { Barcode, CheckCircle2, XCircle } from 'lucide-react'
import { cn } from '../lib/cn'

/**
 * نشانگر وضعیت بارکدخوان روی فرم فاکتور.
 *
 * چرا لازم است: وقتی اسکن با یک بوق کوتاه و بدون بازخورد بصری انجام شود،
 * کاربر نمی‌فهمد کالا اضافه شد یا بارکد ناشناخته بود. این نوار آخرین اسکن
 * را با نتیجه‌اش نشان می‌دهد و اگر بارکدخوان خاموش باشد هم صریح می‌گوید.
 */
export function ScanIndicator({
  enabled,
  last,
}: {
  enabled: boolean
  last: { code: string; name: string; ok: boolean } | null
}) {
  if (!enabled) {
    return (
      <div className="flex items-center gap-2 rounded-xl border border-dashed border-border-strong bg-card-soft px-3 py-2 text-[11.5px] text-muted">
        <Barcode className="size-4 text-faint" aria-hidden />
        بارکدخوان خاموش است — از مرکز تنظیمات ← سخت‌افزار روشنش کنید.
      </div>
    )
  }

  return (
    <div
      aria-live="polite"
      className={cn(
        'flex items-center gap-2 rounded-xl border px-3 py-2 text-[11.5px] transition-colors',
        last === null && 'border-border bg-card-soft text-muted',
        last?.ok && 'border-[var(--success)]/35 bg-[var(--success-soft)] text-success',
        last && !last.ok && 'border-[var(--danger)]/35 bg-[var(--danger-soft)] text-danger',
      )}
    >
      {last === null ? (
        <>
          <Barcode className="size-4 text-accent" aria-hidden />
          بارکدخوان آماده است — بارکد کالا را اسکن کنید تا خودکار به فاکتور اضافه شود.
        </>
      ) : last.ok ? (
        <>
          <CheckCircle2 className="size-4" aria-hidden />
          <span className="tnum" dir="ltr">
            {last.code}
          </span>
          <span className="font-bold">{last.name}</span>
          به فاکتور اضافه شد.
        </>
      ) : (
        <>
          <XCircle className="size-4" aria-hidden />
          بارکد{' '}
          <span className="tnum" dir="ltr">
            {last.code}
          </span>{' '}
          در فهرست کالاها پیدا نشد.
        </>
      )}
    </div>
  )
}
