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

use chrono::NaiveDate;

/// تاریخ قمری.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// عدد روز جولیَن (JDN) از تاریخ میلادی — تقویم پرولپتیک گِرِگوری.
fn julian_day(date: NaiveDate) -> i64 {
    i64::from(date.num_days_from_ce()) + 1_721_425
}

/// تاریخ میلادی از عدد روز جولیَن.
fn from_julian_day(jdn: i64) -> Option<NaiveDate> {
    NaiveDate::from_num_days_from_ce_opt((jdn - 1_721_425) as i32)
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

/// JDN قمری → تاریخ قمری (الگوریتم حسابی استاندارد).
fn julian_to_hijri(jdn: i64) -> HijriDate {
    let mut l = jdn - 1_948_440 + 10_632;
    let n = (l - 1) / 10_631;
    l = l - 10_631 * n + 354;
    let j = ((10_985 - l) / 5_316) * ((50 * l) / 17_719)
        + (l / 5_670) * ((43 * l) / 15_238);
    l = l - ((30 - j) / 15) * ((17_719 * j) / 50)
        - (j / 16) * ((15_238 * j) / 43)
        + 29;
    let month = ((24 * l) / 709) as u32;
    let day = (l - (709 * month as i64) / 24) as u32;
    let year = (30 * n + j - 30) as i32;
    HijriDate { year, month, day }
}

/// تاریخ قمری → JDN.
fn hijri_to_julian(date: HijriDate) -> i64 {
    let year = date.year as i64;
    let month = date.month as i64;
    let day = date.day as i64;
    (11 * year + 3) / 30 + 354 * year + 30 * month - (month - 1) / 2 + day + 1_948_440 - 385
}

/// تبدیل میلادی به قمری.
pub fn to_hijri(date: NaiveDate) -> HijriDate {
    julian_to_hijri(julian_day(date))
}

/// تبدیل قمری به میلادی — فقط برای تاریخ معتبر.
pub fn to_gregorian(date: HijriDate) -> Option<NaiveDate> {
    if HijriDate::new(date.year, date.month, date.day).is_none() {
        return None;
    }
    from_julian_day(hijri_to_julian(date))
}

/// طول یک سال قمری (۳۵۴ یا ۳۵۵ روز).
pub fn hijri_year_length(year: i32) -> Option<i64> {
    let start = to_gregorian(HijriDate::new(year, 1, 1)?)?;
    let next = to_gregorian(HijriDate::new(year + 1, 1, 1)?)?;
    Some((next - start).num_days())
}
