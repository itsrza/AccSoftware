// تشخیصی دور ۱۱: فقط نصف اول — struct و new/format/month_name و نام ماه‌ها

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
