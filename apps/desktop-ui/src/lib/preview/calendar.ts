/**
 * داده‌ی نمونه‌ی تقویم سه‌گانه برای پیش‌نمایش مرورگر.
 *
 * جدول مناسبت‌ها آینه‌ی **یک‌به‌یک** همان جدول `novin_core::occasions` است؛
 * تست `audit12_calendar` برابری دو جدول را نگه می‌دارند تا واگرا نشوند.
 * محاسبه‌ی قمری از `lib/hijri` (آینه‌ی الگوریتم هسته) انجام می‌شود.
 */

import {toHijri} from '../hijri'
import {toJalali} from '../format'
import {daysInJalaliMonth, jalaliToDate} from '../dateRange'

export type PreviewOccasion = {
  calendar: 'jalali' | 'hijri'
  month: number
  day: number
  title: string
  holiday: boolean
}

/** مناسبت‌ها — همگام با crates/novin-core/src/occasions.rs */
export const PREVIEW_OCCASIONS: PreviewOccasion[] = [
  {calendar: 'jalali', month: 1, day: 1, title: 'نوروز', holiday: true},
  {calendar: 'jalali', month: 1, day: 2, title: 'عید نوروز', holiday: true},
  {calendar: 'jalali', month: 1, day: 3, title: 'عید نوروز', holiday: true},
  {calendar: 'jalali', month: 1, day: 4, title: 'عید نوروز', holiday: true},
  {calendar: 'jalali', month: 1, day: 12, title: 'روز جمهوری اسلامی', holiday: true},
  {calendar: 'jalali', month: 1, day: 13, title: 'سیزده‌بدر', holiday: true},
  {calendar: 'jalali', month: 3, day: 14, title: 'رحلت امام خمینی', holiday: true},
  {calendar: 'jalali', month: 3, day: 15, title: 'قیام ۱۵ خرداد', holiday: true},
  {calendar: 'jalali', month: 11, day: 22, title: 'پیروزی انقلاب اسلامی', holiday: true},
  {calendar: 'jalali', month: 12, day: 29, title: 'روز ملی شدن صنعت نفت', holiday: true},
  {calendar: 'hijri', month: 1, day: 9, title: 'تاسوعای حسینی', holiday: true},
  {calendar: 'hijri', month: 1, day: 10, title: 'عاشورای حسینی', holiday: true},
  {calendar: 'hijri', month: 2, day: 20, title: 'اربعین حسینی', holiday: true},
  {calendar: 'hijri', month: 2, day: 28, title: 'رحلت رسول اکرم (ص) و شهادت امام حسن مجتبی (ع)', holiday: true},
  {calendar: 'hijri', month: 2, day: 30, title: 'شهادت امام جعفر صادق (ع)', holiday: true},
  {calendar: 'hijri', month: 3, day: 8, title: 'شهادت امام حسن عسکری (ع)', holiday: false},
  {calendar: 'hijri', month: 3, day: 17, title: 'میلاد رسول اکرم (ص) و امام جعفر صادق (ع)', holiday: true},
  {calendar: 'hijri', month: 6, day: 3, title: 'شهادت حضرت فاطمه زهرا (س)', holiday: false},
  {calendar: 'hijri', month: 7, day: 13, title: 'ولادت امام علی (ع)', holiday: false},
  {calendar: 'hijri', month: 7, day: 27, title: 'مبعث رسول اکرم (ص)', holiday: true},
  {calendar: 'hijri', month: 8, day: 15, title: 'ولادت حضرت قائم (عج)', holiday: true},
  {calendar: 'hijri', month: 9, day: 21, title: 'شهادت امام علی (ع)', holiday: true},
  {calendar: 'hijri', month: 10, day: 1, title: 'عید سعید فطر', holiday: true},
  {calendar: 'hijri', month: 10, day: 2, title: 'تعطیل به مناسبت عید سعید فطر', holiday: true},
  {calendar: 'hijri', month: 12, day: 10, title: 'عید سعید قربان', holiday: true},
  {calendar: 'hijri', month: 12, day: 18, title: 'عید سعید غدیر خم', holiday: false},
]

const pad = (value: number) => String(value).padStart(2, '0')
const jalaliString = (date: Date) => {
  const j = toJalali(date)
  return `${j.year}/${pad(j.month)}/${pad(j.day)}`
}
const hijriString = (date: Date) => {
  const h = toHijri(date)
  return `${h.year}/${pad(h.month)}/${pad(h.day)}`
}

function occasionsOf(date: Date) {
  const j = toJalali(date)
  const h = toHijri(date)
  return PREVIEW_OCCASIONS.filter((occasion) =>
    occasion.calendar === 'jalali'
      ? occasion.month === j.month && occasion.day === j.day
      : occasion.month === h.month && occasion.day === h.day,
  ).map((occasion) => ({
    date: date.toISOString().slice(0, 10),
    jalali: jalaliString(date),
    hijri: hijriString(date),
    title: occasion.title,
    calendar: occasion.calendar,
    holiday: occasion.holiday,
  }))
}

/** پاسخ پیش‌نمایش `calendar_overview` — همان قرارداد میزبان. */
export function calendarOverviewResponse(args: Record<string, unknown>) {
  const now = new Date()
  const todayJ = toJalali(now)
  const todayH = toHijri(now)

  let from: Date
  let to: Date
  if (typeof args.fromJalali === 'string' && typeof args.toJalali === 'string') {
    const [fy, fm, fd] = args.fromJalali.split('/').map(Number)
    const [ty, tm, td] = args.toJalali.split('/').map(Number)
    from = jalaliToDate(fy, fm, fd)
    to = jalaliToDate(ty, tm, td)
  } else {
    from = jalaliToDate(todayJ.year, todayJ.month, 1)
    to = jalaliToDate(todayJ.year, todayJ.month, daysInJalaliMonth(todayJ.year, todayJ.month))
  }

  const occasions: ReturnType<typeof occasionsOf> = []
  // پیمایش روزشمار مستقل از منطقه‌ی زمانی — ظهر UTC مبنای امن است.
  const dayCount = Math.round((to.getTime() - from.getTime()) / 86_400_000)
  for (let index = 0; index <= dayCount; index += 1) {
    occasions.push(...occasionsOf(new Date(from.getTime() + index * 86_400_000)))
  }

  const iso = (date: Date) => date.toISOString().slice(0, 10)
  return {
    today: {
      iso: iso(now),
      jalali: jalaliString(now),
      jalali_year: todayJ.year,
      jalali_month: todayJ.month,
      jalali_day: todayJ.day,
      hijri: hijriString(now),
      hijri_year: todayH.year,
      hijri_month: todayH.month,
      hijri_day: todayH.day,
      hijri_month_name: '',
      gregorian: iso(now),
      weekday: (now.getDay() + 1) % 7,
      occasions: occasionsOf(now),
    },
    occasions,
  }
}
