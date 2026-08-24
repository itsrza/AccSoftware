import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react'
import { setNumberLocale } from '../format'
import { fa, type Dictionary, type TranslationKey } from './fa'
import { en } from './en'
import { ar } from './ar'

/**
 * لایه‌ی چندزبانی برنامه.
 *
 * ## سه تصمیم طراحی
 *
 * ۱. **فارسی مرجع کلیدهاست.** نوع `TranslationKey` از شکل دیکشنری فارسی
 *    ساخته می‌شود، پس افزودن کلید تازه بدون معادل انگلیسی و عربی، خطای
 *    کامپایل می‌دهد — نه متن جامانده‌ای که کاربر ببیند.
 * ۲. **ارقام جزوی از زبان است.** فارسی «۱۲۳»، عربی «١٢٣» و انگلیسی «123».
 *    این کار در `lib/format.ts` انجام می‌شود تا صدها محل فراخوانی موجود
 *    دست‌نخورده بمانند.
 * ۳. **جهت صفحه از زبان مشتق می‌شود.** فارسی و عربی راست‌به‌چپ، انگلیسی
 *    چپ‌به‌راست؛ روی `<html>` می‌نشیند تا همه‌ی کلاس‌های منطقی تِیلویند
 *    (`ps-`, `pe-`, `start-`, `end-`) خودشان بچرخند.
 */

export type Locale = 'fa' | 'en' | 'ar'

export type LocaleInfo = {
  code: Locale
  /** نام زبان به خودِ آن زبان — قاعده‌ی جهانی انتخاب‌گر زبان. */
  nativeLabel: string
  dir: 'rtl' | 'ltr'
  /** برچسب کوتاه روی دکمه‌ی نوار بالا. */
  short: string
}

export const LOCALES: LocaleInfo[] = [
  { code: 'fa', nativeLabel: 'فارسی', dir: 'rtl', short: 'فا' },
  { code: 'en', nativeLabel: 'English', dir: 'ltr', short: 'EN' },
  { code: 'ar', nativeLabel: 'العربية', dir: 'rtl', short: 'ع' },
]

const DICTIONARIES: Record<Locale, Dictionary> = { fa, en, ar }

/** جهت نوشتار هر زبان. */
export function directionOf(locale: Locale): 'rtl' | 'ltr' {
  return LOCALES.find((item) => item.code === locale)?.dir ?? 'rtl'
}

const STORAGE_KEY = 'novin.locale'

/** خواندن زبان ذخیره‌شده؛ اگر چیزی نبود، فارسی. */
export function storedLocale(): Locale {
  try {
    const value = window.localStorage.getItem(STORAGE_KEY)
    if (value === 'fa' || value === 'en' || value === 'ar') return value
  } catch {
    /* در محیط بدون localStorage (تست‌ها) فارسی پیش‌فرض است */
  }
  return 'fa'
}

/** جایگزینی متغیرهای `{name}` در متن ترجمه. */
export function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    key in vars ? String(vars[key]) : match,
  )
}

/**
 * ترجمه‌ی بدون هوک — برای جاهایی که خارج از درخت React لازم است
 * (مثلاً ساخت عنوان پنجره‌ی چاپ).
 */
export function translate(
  locale: Locale,
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  const dictionary = DICTIONARIES[locale] ?? fa
  // اگر ترجمه‌ای خالی مانده باشد، فارسی نمایش داده می‌شود نه خودِ کلید:
  // متن فارسی برای کاربر ایرانی قابل فهم است، «dashboard.title» نیست.
  const template = dictionary[key] || fa[key] || key
  return interpolate(template, vars)
}

export type I18nValue = {
  locale: Locale
  dir: 'rtl' | 'ltr'
  t: (key: TranslationKey, vars?: Record<string, string | number>) => string
  setLocale: (locale: Locale) => void
}

const I18nContext = createContext<I18nValue | null>(null)

export function I18nProvider({
  children,
  initialLocale,
}: {
  children: ReactNode
  initialLocale?: Locale
}) {
  const [locale, setLocaleState] = useState<Locale>(() => initialLocale ?? storedLocale())

  // ارقام و جهت صفحه دو رویِ یک سکه‌اند: هر دو از زبان مشتق می‌شوند.
  useEffect(() => {
    setNumberLocale(locale)
    const root = document.documentElement
    root.lang = locale
    root.dir = directionOf(locale)
  }, [locale])

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next)
    setNumberLocale(next)
    try {
      window.localStorage.setItem(STORAGE_KEY, next)
    } catch {
      /* نبود localStorage نباید تغییر زبان را از کار بیندازد */
    }
  }, [])

  const value = useMemo<I18nValue>(
    () => ({
      locale,
      dir: directionOf(locale),
      t: (key, vars) => translate(locale, key, vars),
      setLocale,
    }),
    [locale, setLocale],
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

/**
 * دسترسی به زبان فعال.
 *
 * بیرون از `I18nProvider` هم کار می‌کند (فارسی) تا تست‌های واحدِ یک جزء
 * مجبور نباشند کل درخت را بپیچند.
 */
export function useI18n(): I18nValue {
  const context = useContext(I18nContext)
  if (context) return context
  return {
    locale: 'fa',
    dir: 'rtl',
    t: (key, vars) => translate('fa', key, vars),
    setLocale: () => undefined,
  }
}

export type { TranslationKey, Dictionary }
export { fa, en, ar }
