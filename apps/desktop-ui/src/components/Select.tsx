import {
  Children,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { createPortal } from 'react-dom'
import { ChevronDown, Check } from 'lucide-react'
import { cn } from '../lib/cn'

/**
 * دراپ‌داون سیستم طراحی — جایگزین `<select>` بومی.
 *
 * ## چرا نوشته شد
 * `<select>` بومی فهرست بازشده‌اش را سیستم‌عامل رسم می‌کند. نتیجه در ویندوز
 * یک کادر خاکستری «ویندوز فرمی» است که نه فونت وزیرمتن دارد، نه شعاع گوشه‌ی
 * قالب، نه تم تیره. این جزء همان رفتار را با DOM خودمان می‌سازد تا ظاهر در
 * تمام برنامه یکدست بماند.
 *
 * ## چرا API آن دقیقاً مثل `<select>` است
 * ده‌ها فرم در برنامه از `<select value onChange><option/></select>` استفاده
 * می‌کنند و بعضی فرم‌ها مقدار را با `FormData` می‌خوانند. برای اینکه مهاجرت
 * بدون بازنویسی منطق فرم‌ها انجام شود:
 *   • گزینه‌ها از فرزندان `<option>`/`<optgroup>` خوانده می‌شوند؛
 *   • `onChange` یک شیء با شکل `{target:{value,name}}` می‌دهد؛
 *   • یک `<input type="hidden">` هم‌نام رندر می‌شود تا `FormData` کار کند؛
 *   • `required` با اعتبارسنجی خودِ فرم سازگار است.
 *
 * ## دسترس‌پذیری
 * الگوی `combobox` + `listbox`: پیمایش با جهت‌نماها، Home/End، تایپ برای
 * جستجو، Enter/Space برای انتخاب و Escape برای بستن.
 */

export type SelectOption = { value: string; label: string; disabled?: boolean; group?: string }

type ChangeLike = { target: { value: string; name?: string } }

/** استخراج گزینه‌ها از فرزندان JSX — پشتیبانی از `option` و `optgroup`. */
function collectOptions(children: ReactNode, group?: string): SelectOption[] {
  const out: SelectOption[] = []
  Children.forEach(children, (child) => {
    if (!isValidElement(child)) return
    const props = child.props as {
      value?: string | number
      label?: string
      disabled?: boolean
      children?: ReactNode
    }
    if (child.type === 'optgroup') {
      out.push(...collectOptions(props.children, props.label ?? ''))
      return
    }
    if (child.type === 'option') {
      const label = textOf(props.children) || String(props.value ?? '')
      out.push({
        value: props.value === undefined ? label : String(props.value),
        label,
        disabled: props.disabled,
        group,
      })
      return
    }
    // آرایه یا Fragment
    out.push(...collectOptions(props.children, group))
  })
  return out
}

function textOf(node: ReactNode): string {
  if (node === null || node === undefined || typeof node === 'boolean') return ''
  if (typeof node === 'string' || typeof node === 'number') return String(node)
  if (Array.isArray(node)) return node.map(textOf).join('')
  if (isValidElement(node)) return textOf((node.props as { children?: ReactNode }).children)
  return ''
}

export function Select({
  value,
  defaultValue,
  onChange,
  name,
  required,
  disabled,
  children,
  className,
  placeholder = 'انتخاب کنید…',
  'aria-label': ariaLabel,
  id,
}: {
  /** عدد هم پذیرفته می‌شود چون بعضی فرم‌ها مقدار عددی نگه می‌دارند. */
  value?: string | number
  defaultValue?: string | number
  onChange?: (event: ChangeLike) => void
  name?: string
  required?: boolean
  disabled?: boolean
  children: ReactNode
  className?: string
  placeholder?: string
  'aria-label'?: string
  id?: string
}) {
  const options = useMemo(() => collectOptions(children), [children])
  const controlled = value !== undefined
  const [inner, setInner] = useState(defaultValue === undefined ? '' : String(defaultValue))
  const current = controlled ? String(value) : inner

  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const [rect, setRect] = useState<{ top: number; left: number; width: number; drop: 'down' | 'up' }>()
  const buttonRef = useRef<HTMLButtonElement>(null)
  const listRef = useRef<HTMLUListElement>(null)
  const typed = useRef({ text: '', at: 0 })
  const listId = useId()

  const selected = options.find((option) => option.value === current)
  const enabled = options.filter((option) => !option.disabled)

  const commit = useCallback(
    (next: string) => {
      if (!controlled) setInner(next)
      onChange?.({ target: { value: next, name } })
      setOpen(false)
      buttonRef.current?.focus()
    },
    [controlled, name, onChange],
  )

  /** موقعیت فهرست نسبت به دکمه — با پورتال تا هیچ `overflow:hidden` آن را نبُرد. */
  const place = useCallback(() => {
    const node = buttonRef.current
    if (!node) return
    const box = node.getBoundingClientRect()
    const below = window.innerHeight - box.bottom
    const drop: 'down' | 'up' = below < 240 && box.top > below ? 'up' : 'down'
    setRect({
      top: drop === 'down' ? box.bottom + 6 : box.top - 6,
      left: box.left,
      width: box.width,
      drop,
    })
  }, [])

  useLayoutEffect(() => {
    if (!open) return
    place()
    setActive(Math.max(0, options.findIndex((option) => option.value === current)))
  }, [open, place, options, current])

  useEffect(() => {
    if (!open) return
    const close = (event: MouseEvent) => {
      if (buttonRef.current?.contains(event.target as Node)) return
      if (listRef.current?.contains(event.target as Node)) return
      setOpen(false)
    }
    document.addEventListener('mousedown', close)
    window.addEventListener('resize', place)
    window.addEventListener('scroll', place, true)
    return () => {
      document.removeEventListener('mousedown', close)
      window.removeEventListener('resize', place)
      window.removeEventListener('scroll', place, true)
    }
  }, [open, place])

  useEffect(() => {
    if (!open || !listRef.current) return
    const node = listRef.current.querySelector<HTMLElement>(`[data-index="${active}"]`)
    node?.scrollIntoView?.({ block: 'nearest' })
  }, [open, active])

  const step = (delta: number) => {
    if (!options.length) return
    let next = active
    for (let i = 0; i < options.length; i += 1) {
      next = (next + delta + options.length) % options.length
      if (!options[next].disabled) break
    }
    setActive(next)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (disabled) return
    if (!open && (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault()
      setOpen(true)
      return
    }
    if (!open) return
    switch (event.key) {
      case 'Escape':
        event.preventDefault()
        setOpen(false)
        break
      case 'ArrowDown':
        event.preventDefault()
        step(1)
        break
      case 'ArrowUp':
        event.preventDefault()
        step(-1)
        break
      case 'Home':
        event.preventDefault()
        setActive(options.findIndex((option) => !option.disabled))
        break
      case 'End':
        event.preventDefault()
        setActive(options.length - 1)
        break
      case 'Enter':
      case ' ': {
        event.preventDefault()
        const option = options[active]
        if (option && !option.disabled) commit(option.value)
        break
      }
      case 'Tab':
        setOpen(false)
        break
      default:
        if (event.key.length === 1) {
          const now = Date.now()
          typed.current = {
            text: now - typed.current.at > 700 ? event.key : typed.current.text + event.key,
            at: now,
          }
          const hit = options.findIndex(
            (option) => !option.disabled && option.label.startsWith(typed.current.text),
          )
          if (hit >= 0) setActive(hit)
        }
    }
  }

  return (
    <span className={cn('np-select relative block w-full', className)}>
      <button
        ref={buttonRef}
        id={id}
        type="button"
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-label={ariaLabel}
        aria-required={required || undefined}
        disabled={disabled}
        onClick={() => !disabled && setOpen((state) => !state)}
        onKeyDown={onKeyDown}
        className={cn(
          'flex w-full items-center gap-2 rounded-xl border bg-card px-3 py-[9px] text-start text-[12.5px] font-medium transition-colors',
          disabled
            ? 'cursor-not-allowed border-border text-faint opacity-60'
            : 'cursor-pointer border-border text-text hover:border-border-strong',
          open && 'border-accent shadow-[0_0_0_3px_var(--accent-soft)]',
          !selected && 'text-faint',
        )}
      >
        <span className="min-w-0 flex-1 truncate">{selected ? selected.label : placeholder}</span>
        <ChevronDown
          className={cn('size-3.5 shrink-0 text-faint transition-transform duration-200', open && 'rotate-180')}
          aria-hidden
        />
      </button>

      {/* مقدار برای `FormData` — فرم‌های موجود با همین نام مقدار را می‌خوانند. */}
      {name && <input type="hidden" name={name} value={current} />}
      {/* اعتبارسنجی بومی «الزامی» بدون نمایش کنترل سیستم‌عامل. */}
      {required && (
        <input
          tabIndex={-1}
          aria-hidden
          required
          value={current}
          onChange={() => undefined}
          className="pointer-events-none absolute bottom-1 start-3 h-0 w-0 opacity-0"
        />
      )}

      {open &&
        rect &&
        createPortal(
          <ul
            ref={listRef}
            id={listId}
            role="listbox"
            aria-label={ariaLabel}
            style={{
              position: 'fixed',
              top: rect.drop === 'down' ? rect.top : undefined,
              bottom: rect.drop === 'up' ? window.innerHeight - rect.top : undefined,
              left: rect.left,
              width: rect.width,
            }}
            className="np-select-list fade-up z-[200] max-h-[280px] overflow-y-auto rounded-xl border border-border bg-card p-1.5 shadow-[var(--shadow-lg)]"
            dir="rtl"
          >
            {options.length === 0 && (
              <li className="px-3 py-3 text-center text-[11.5px] text-faint">گزینه‌ای موجود نیست</li>
            )}
            {options.map((option, index) => {
              const isSelected = option.value === current
              const showGroup = option.group && option.group !== options[index - 1]?.group
              return (
                <li key={`${option.group ?? ''}-${option.value}-${index}`}>
                  {showGroup && (
                    <p className="px-2.5 pt-2 pb-1 text-[10px] font-bold text-faint">{option.group}</p>
                  )}
                  <button
                    type="button"
                    data-index={index}
                    role="option"
                    aria-selected={isSelected}
                    disabled={option.disabled}
                    onMouseEnter={() => setActive(index)}
                    onClick={() => !option.disabled && commit(option.value)}
                    className={cn(
                      'flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-start text-[12.5px] transition-colors',
                      option.disabled
                        ? 'cursor-not-allowed text-faint'
                        : isSelected
                          ? 'bg-[var(--accent-soft)] font-bold text-accent-strong'
                          : index === active
                            ? 'bg-bg-soft text-text'
                            : 'text-muted',
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate">{option.label}</span>
                    {isSelected && <Check className="size-3.5 shrink-0" aria-hidden />}
                  </button>
                </li>
              )
            })}
          </ul>,
          document.body,
        )}
    </span>
  )
}

export { collectOptions as __collectOptions }
