//! تبدیل تقویم میلادی ↔ شمسی (هجری خورشیدی).
//!
//! قاعده‌ی معماری: تاریخ‌ها **همیشه** به‌صورت میلادی ISO (`YYYY-MM-DD`) در پایگاه
//! داده ذخیره می‌شوند تا مرتب‌سازی، بازه‌گیری و مهاجرت داده استاندارد بماند؛
//! تبدیل به شمسی فقط در مرز نمایش انجام می‌شود.
//!
//! سال کبیسه مستقیماً از خود الگوریتم تبدیل استخراج می‌شود (نه از یک جدول
//! جداگانه) تا هیچ‌گاه دو منبع حقیقت متناقض نداشته باشیم.

use chrono::{Datelike, NaiveDate};

/// خطاهای تقویم.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CalendarError {
    #[error("CAL-001: تاریخ شمسی نامعتبر است")]
    InvalidJalali,
    #[error("CAL-002: تاریخ میلادی نامعتبر است")]
    InvalidGregorian,
    #[error("CAL-003: قالب تاریخ نامعتبر است")]
    InvalidFormat,
}

/// تاریخ شمسی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JalaliDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

const GREGORIAN_MONTH_OFFSETS: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

impl JalaliDate {
    /// ساخت تاریخ شمسی معتبر؛ در صورت نامعتبر بودن خطا برمی‌گرداند.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, CalendarError> {
        let candidate = JalaliDate { year, month, day };
        if !candidate.is_valid() {
            return Err(CalendarError::InvalidJalali);
        }
        Ok(candidate)
    }

    pub fn is_valid(&self) -> bool {
        if self.year < 1 || self.month == 0 || self.month > 12 || self.day == 0 {
            return false;
        }
        self.day <= days_in_jalali_month(self.year, self.month)
    }

    /// قالب استاندارد نمایش: `1404/05/30`
    pub fn format(&self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }

    /// خواندن از رشته‌ی `YYYY/MM/DD` یا `YYYY-MM-DD` با ارقام فارسی یا لاتین.
    pub fn parse(input: &str) -> Result<Self, CalendarError> {
        let normalized = crate::money::normalize_digits(input);
        let parts: Vec<&str> = normalized.split(['/', '-']).map(str::trim).collect();
        if parts.len() != 3 {
            return Err(CalendarError::InvalidFormat);
        }
        let year = parts[0]
            .parse::<i32>()
            .map_err(|_| CalendarError::InvalidFormat)?;
        let month = parts[1]
            .parse::<u32>()
            .map_err(|_| CalendarError::InvalidFormat)?;
        let day = parts[2]
            .parse::<u32>()
            .map_err(|_| CalendarError::InvalidFormat)?;
        JalaliDate::new(year, month, day)
    }

    /// تبدیل به میلادی (با اعتبارسنجی).
    pub fn to_gregorian(&self) -> Result<NaiveDate, CalendarError> {
        if !self.is_valid() {
            return Err(CalendarError::InvalidJalali);
        }
        to_gregorian_unchecked(self.year, self.month as i32, self.day as i32)
            .ok_or(CalendarError::InvalidGregorian)
    }
}

/// هسته‌ی تبدیل شمسی → میلادی بدون اعتبارسنجی روز/ماه.
fn to_gregorian_unchecked(jy: i32, jm: i32, jd: i32) -> Option<NaiveDate> {
    let (gy_base, jy0) = if jy >= 979 {
        (1600, jy - 979)
    } else {
        (621, jy)
    };
    let mut days = 365 * jy0
        + (jy0 / 33) * 8
        + (jy0 % 33 + 3) / 4
        + 78
        + jd
        + if jm < 7 {
            (jm - 1) * 31
        } else {
            (jm - 7) * 30 + 186
        };
    let mut gy = gy_base + 400 * (days / 146_097);
    days %= 146_097;
    if days > 36_524 {
        days -= 1;
        gy += 100 * (days / 36_524);
        days %= 36_524;
        if days >= 365 {
            days += 1;
        }
    }
    gy += 4 * (days / 1_461);
    days %= 1_461;
    if days > 365 {
        gy += (days - 1) / 365;
        days = (days - 1) % 365;
    }
    let mut gd = days + 1;
    let leap = is_gregorian_leap(gy);
    let month_lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut gm = 0usize;
    while gm < 12 && gd > month_lengths[gm] {
        gd -= month_lengths[gm];
        gm += 1;
    }
    if gm >= 12 {
        return None;
    }
    NaiveDate::from_ymd_opt(gy, gm as u32 + 1, gd as u32)
}

/// تبدیل تاریخ میلادی به شمسی.
pub fn from_gregorian(date: NaiveDate) -> JalaliDate {
    let gy = date.year();
    let gm = date.month() as i32;
    let gd = date.day() as i32;
    let (mut jy, gy0) = if gy >= 1600 {
        (979, gy - 1600)
    } else {
        (0, gy - 621)
    };
    let gy2 = if gm > 2 { gy0 + 1 } else { gy0 };
    let mut days = 365 * gy0 + (gy2 + 3) / 4 - (gy2 + 99) / 100 + (gy2 + 399) / 400 - 80
        + gd
        + GREGORIAN_MONTH_OFFSETS[(gm - 1) as usize];
    jy += 33 * (days / 12_053);
    days %= 12_053;
    jy += 4 * (days / 1_461);
    days %= 1_461;
    if days > 365 {
        jy += (days - 1) / 365;
        days = (days - 1) % 365;
    }
    let (month, day) = if days < 186 {
        (days / 31 + 1, days % 31 + 1)
    } else {
        ((days - 186) / 30 + 7, (days - 186) % 30 + 1)
    };
    JalaliDate {
        year: jy,
        month: month as u32,
        day: day as u32,
    }
}

/// آیا سال شمسی کبیسه است؟ (مستقیماً از طول سال در الگوریتم تبدیل)
pub fn is_jalali_leap(year: i32) -> bool {
    match (
        to_gregorian_unchecked(year, 1, 1),
        to_gregorian_unchecked(year + 1, 1, 1),
    ) {
        (Some(start), Some(next)) => (next - start).num_days() == 366,
        _ => false,
    }
}

/// تعداد روزهای ماه شمسی.
pub fn days_in_jalali_month(year: i32, month: u32) -> u32 {
    match month {
        1..=6 => 31,
        7..=11 => 30,
        12 => {
            if is_jalali_leap(year) {
                30
            } else {
                29
            }
        }
        _ => 0,
    }
}

fn is_gregorian_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// قالب کوتاه شمسی برای تاریخ میلادی — جایگزین منطق پراکنده‌ی قبلی در لایه‌ی IPC.
pub fn jalali_string(date: NaiveDate) -> String {
    from_gregorian(date).format()
}

/// تاریخ ISO میلادی (قالب ذخیره‌سازی استاندارد پایگاه داده).
pub fn iso_string(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
