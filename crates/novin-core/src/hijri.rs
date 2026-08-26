#![allow(warnings)]
// تشخیصی دور ۱۴: نصف اول + فقط لایه‌ی JDN بدون توابع تبدیل قمری

use chrono::NaiveDate;

/// محدوده‌ی سال‌های قابل اتکا برای این تقویم.
pub const HIJRI_MIN_YEAR: i32 = 1300;
pub const HIJRI_MAX_YEAR: i32 = 1600;

/// تاریخ قمری.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl HijriDate {
    pub fn new(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(HIJRI_MIN_YEAR..=HIJRI_MAX_YEAR).contains(&year) {
            return None;
        }
        if month == 0 || month > 12 || day == 0 || day > 30 {
            return None;
        }
        Some(HijriDate { year, month, day })
    }

    /// قالب استاندارد نمایش: `1447/10/01`
    pub fn format(&self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }

    /// نام ماه قمری.
    pub fn month_name(&self) -> &'static str {
        hijri_month_name(self.month)
    }
}

/// نام ماه‌های قمری به فارسی.
pub fn hijri_month_name(month: u32) -> &'static str {
    match month {
        1 => "محرم",
        2 => "صفر",
        3 => "ربیع‌الاول",
        4 => "ربیع‌الثانی",
        5 => "جمادی‌الاول",
        6 => "جمادی‌الثانی",
        7 => "رجب",
        8 => "شعبان",
        9 => "رمضان",
        10 => "شوال",
        11 => "ذی‌القعده",
        12 => "ذی‌الحجه",
        _ => "",
    }
}

/// عدد روز جولیَن از تاریخ میلادی — تقویم پرولپتیک گِرِگوری (فرمول کلاسیک).
fn gregorian_to_jdn(year: i32, month: u32, day: u32) -> i64 {
    let a = (14 - i64::from(month)) / 12;
    let y = i64::from(year) + 4800 - a;
    let m = i64::from(month) + 12 * a - 3;
    i64::from(day) + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32_045
}

/// تاریخ میلادی از عدد روز جولیَن — فرمول معکوس کلاسیک.
fn jdn_to_gregorian(jdn: i64) -> Option<(i32, u32, u32)> {
    let a = jdn + 32_044;
    let b = (4 * a + 3) / 146_097;
    let c = a - (146_097 * b) / 4;
    let d = (4 * c + 3) / 1_461;
    let e = c - (1_461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    if !(1..=12).contains(&month) || day < 1 || day > 31 || !(1..=9999).contains(&year) {
        return None;
    }
    Some((year as i32, month as u32, day as u32))
}

/// نگه‌دارنده‌ی تشخیصی — تابع تست JDN.
pub fn debug_jdn(date: NaiveDate) -> i64 {
    gregorian_to_jdn(date.year(), date.month(), date.day())
}
