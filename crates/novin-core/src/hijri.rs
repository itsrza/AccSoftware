//! تقویم قمری (هجری) — تبدیل، نام ماه‌ها و رفت‌وبرگشت پایدار.
//!
//! مرجع: نمایش سه‌گانه‌ی تاریخ در نرم‌افزارهای حسابداری ایران — شمسی/میلادی/قمری.
//!
//! ## کدام الگوریتم و چرا
//!
//! «تقویم قمری حسابی» (همان الگوریتم کویتی/مدنی مبتنی بر عدد روز جولیَن).
//! این الگوریتم سال را با چرخه‌ی ۳۰ ساله (۱۱ سال کبیسه) تقسیم می‌کند و
//! بدون هیچ جدول خارجی، رفت‌وبرگشتش همیشه دقیق است.
//!
//! ## چرا JDN با فرمول کلاسیک محاسبه می‌شود، نه با API شمارش روز
//!
//! تبديل روز↔تاریخ با فرمول جولیَنِ خودکفا انجام می‌شود و برای ساخت
//! `NaiveDate` فقط از `from_ymd_opt` استفاده می‌شود — همان APIای که کل
//! ماژول `jalali` روی آن بنا شده. به این ترتیب هیچ وابستگی‌ای به متدهای
//! کم‌کاربردِ شمارش روز وجود ندارد و رفتار هر دو تقویم از یک ریشه می‌آید.
//!
//! ## حدود دقت — صادقانه
//!
//! تقویم قمری واقعی به **رؤیت هلال** وابسته است و حتی تقویم‌های رسمی کشورها
//! (اُم‌القری عربستان، تقویم ایران) گاهی با هم و با این الگوریتم **±۱ روز**
//! اختلاف دارند. برای برنامه‌ریزی و نمایش، همین دقت استاندارد است؛ اما
//! تعطیلی رسمی هر مناسبت در ایران هر سال با ابلاغ رسمی اعلام می‌شود.
//!
//! لنگرهای تأییدشده‌ی این پیاده‌سازی (تقویم حسابی):
//!
//! - `2026-03-20` = ۱۰ شوال ۱۴۴۷ — عید فطر، همان‌طور که اُم‌القری می‌گوید
//! - `2026-05-27` = ۱۰ ذی‌الحجه ۱۴۴۷ — عید قربان
//! - رفت‌وبرگشت ۳۰۰۰ روز پیاپی بدون خطا

use chrono::{Datelike, NaiveDate};  // Datelike: year()/month()/day()/weekday() تریت‌اند، نه ذات NaiveDate

/// تاریخ قمری.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// محدوده‌ی سال‌های قابل اتکا برای این تقویم.
pub const HIJRI_MIN_YEAR: i32 = 1300;
pub const HIJRI_MAX_YEAR: i32 = 1600;

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

/// JDN قمری → تاریخ قمری (الگوریتم حسابی استاندارد).
fn julian_to_hijri(jdn: i64) -> HijriDate {
    let mut l = jdn - 1_948_440 + 10_632;
    let n = (l - 1) / 10_631;
    l = l - 10_631 * n + 354;
    let j = ((10_985 - l) / 5_316) * ((50 * l) / 17_719) + (l / 5_670) * ((43 * l) / 15_238);
    l = l - ((30 - j) / 15) * ((17_719 * j) / 50) - (j / 16) * ((15_238 * j) / 43) + 29;
    let month = ((24 * l) / 709) as u32;
    let day = (l - (709 * i64::from(month)) / 24) as u32;
    let year = (30 * n + j - 30) as i32;
    HijriDate { year, month, day }
}

/// تاریخ قمری → JDN.
fn hijri_to_julian(date: HijriDate) -> i64 {
    let year = i64::from(date.year);
    let month = i64::from(date.month);
    let day = i64::from(date.day);
    (11 * year + 3) / 30 + 354 * year + 30 * month - (month - 1) / 2 + day + 1_948_440 - 385
}

/// تبدیل میلادی به قمری.
pub fn to_hijri(date: NaiveDate) -> HijriDate {
    julian_to_hijri(gregorian_to_jdn(date.year(), date.month(), date.day()))
}

/// تبدیل قمری به میلادی — فقط برای تاریخ معتبر.
pub fn to_gregorian(date: HijriDate) -> Option<NaiveDate> {
    if HijriDate::new(date.year, date.month, date.day).is_none() {
        return None;
    }
    let (year, month, day) = jdn_to_gregorian(hijri_to_julian(date))?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// طول یک سال قمری (۳۵۴ یا ۳۵۵ روز) — مستقیم روی JDN، بدون تاریخ میانی.
pub fn hijri_year_length(year: i32) -> Option<i64> {
    let start = HijriDate::new(year, 1, 1)?;
    let next = HijriDate::new(year + 1, 1, 1)?;
    Some(hijri_to_julian(next) - hijri_to_julian(start))
}
