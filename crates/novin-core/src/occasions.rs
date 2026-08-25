//! مناسبت‌های شمسی و قمری ایران.
//!
//! مرجع: فهرست رسمی و رایج مناسبت‌ها/تعطیلات ایران — همان چیزی که نرم‌افزار
//! حسابداری در تقویم سه‌گانه‌اش نشان می‌دهد.
//!
//! ## دو تصمیم
//!
//! ۱. **مناسبت‌ها داده‌اند نه کد.** یک جدول ثابت از `(تقویم، ماه، روز)`؛
//! تابع `occasions_between` روزهای بازه را می‌پیماید و جدول را می‌پرسد.
//! پیمایش روز‌به‌روز شاید نابه‌یده به نظر برسد، اما حداکثر ۴۰۰ تکرار با یک
//! تبدیل ساده است و برخلاف روش‌های «هوشمند»، هرگز در سرِ ماه یا سال کبیسه
//! قمری خطا نمی‌کند.
//!
//! ۲. **پرچم «تعطیل» جدا از خود مناسبت است.** فهرست تعطیلات رسمی هر سال با
//! ابلاغ اعلام می‌شود؛ پرچم اینجا وضعیت متعارف هر سال است.

use crate::hijri::to_hijri;
use crate::jalali;
use chrono::NaiveDate;

/// یک مناسبت ثابت در جدول.
#[derive(Debug, Clone, Copy)]
pub struct Occasion {
    /// `jalali` یا `hijri`
    pub calendar: &'static str,
    pub month: u32,
    pub day: u32,
    pub title: &'static str,
    /// تعطیل رسمی (متعارف)
    pub holiday: bool,
}

/// جدول مناسبت‌ها — شمسی و قمری.
pub const OCCASIONS: &[Occasion] = &[
    // ------------------------------------------------------------- شمسی
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 1,
        title: "نوروز",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 2,
        title: "عید نوروز",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 3,
        title: "عید نوروز",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 4,
        title: "عید نوروز",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 12,
        title: "روز جمهوری اسلامی",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 1,
        day: 13,
        title: "سیزده‌بدر",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 3,
        day: 14,
        title: "رحلت امام خمینی",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 3,
        day: 15,
        title: "قیام ۱۵ خرداد",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 11,
        day: 22,
        title: "پیروزی انقلاب اسلامی",
        holiday: true,
    },
    Occasion {
        calendar: "jalali",
        month: 12,
        day: 29,
        title: "روز ملی شدن صنعت نفت",
        holiday: true,
    },
    // -------------------------------------------------------------- قمری
    Occasion {
        calendar: "hijri",
        month: 1,
        day: 9,
        title: "تاسوعای حسینی",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 1,
        day: 10,
        title: "عاشورای حسینی",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 2,
        day: 20,
        title: "اربعین حسینی",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 2,
        day: 28,
        title: "رحلت رسول اکرم (ص) و شهادت امام حسن مجتبی (ع)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 2,
        day: 30,
        title: "شهادت امام جعفر صادق (ع)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 3,
        day: 8,
        title: "شهادت امام حسن عسکری (ع)",
        holiday: false,
    },
    Occasion {
        calendar: "hijri",
        month: 3,
        day: 17,
        title: "میلاد رسول اکرم (ص) و امام جعفر صادق (ع)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 6,
        day: 3,
        title: "شهادت حضرت فاطمه زهرا (س)",
        holiday: false,
    },
    Occasion {
        calendar: "hijri",
        month: 7,
        day: 13,
        title: "ولادت امام علی (ع)",
        holiday: false,
    },
    Occasion {
        calendar: "hijri",
        month: 7,
        day: 27,
        title: "مبعث رسول اکرم (ص)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 8,
        day: 15,
        title: "ولادت حضرت قائم (عج)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 9,
        day: 21,
        title: "شهادت امام علی (ع)",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 10,
        day: 1,
        title: "عید سعید فطر",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 10,
        day: 2,
        title: "تعطیل به مناسبت عید سعید فطر",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 12,
        day: 10,
        title: "عید سعید قربان",
        holiday: true,
    },
    Occasion {
        calendar: "hijri",
        month: 12,
        day: 18,
        title: "عید سعید غدیر خم",
        holiday: false,
    },
];

/// خطای دامنه‌ی مناسبت‌ها.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OccasionError {
    #[error("بازه‌ی مناسبت‌ها نمی‌تواند بیش از ۴۰۰ روز باشد")]
    RangeTooLong,
    #[error("پایان بازه نمی‌تواند قبل از آغاز آن باشد")]
    InvalidRange,
}

/// یک روزِ مناسبت‌دار در بازه‌ی تقویمی.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccasionDay {
    /// تاریخ میلادی ISO
    pub date: NaiveDate,
    /// تاریخ شمسی نمایشی `1404/06/10`
    pub jalali: String,
    /// تاریخ قمری نمایشی `1448/03/10`
    pub hijri: String,
    pub title: &'static str,
    pub calendar: &'static str,
    pub holiday: bool,
}

/// مناسبت‌های یک روز مشخص.
pub fn occasions_on(date: NaiveDate) -> Vec<OccasionDay> {
    let jalali_date = jalali::from_gregorian(date);
    let hijri_date = to_hijri(date);
    let mut out = Vec::new();
    for occasion in OCCASIONS {
        let hits = match occasion.calendar {
            "jalali" => occasion.month == jalali_date.month && occasion.day == jalali_date.day,
            _ => occasion.month == hijri_date.month && occasion.day == hijri_date.day,
        };
        if hits {
            out.push(OccasionDay {
                date,
                jalali: jalali_date.format(),
                hijri: hijri_date.format(),
                title: occasion.title,
                calendar: occasion.calendar,
                holiday: occasion.holiday,
            });
        }
    }
    out
}

/// همه‌ی مناسبت‌های یک بازه به ترتیب تاریخ.
///
/// حداکثر ۴۰۰ روز — گِرد بیش از یک سال که تقویم روز‌شمار معنی ندارد.
pub fn occasions_between(
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<OccasionDay>, OccasionError> {
    if to < from {
        return Err(OccasionError::InvalidRange);
    }
    if (to - from).num_days() > 400 {
        return Err(OccasionError::RangeTooLong);
    }
    let mut days = Vec::new();
    let mut cursor = from;
    while cursor <= to {
        days.extend(occasions_on(cursor));
        // الگوی اثبات‌شده‌ی مخزن (jalali.rs/checks.rs) — بدون اتکا به AddAssign
        cursor = match cursor.checked_add_signed(chrono::Duration::days(1)) {
            Some(next) => next,
            None => break,
        };
    }
    Ok(days)
}

/// اطلاعات کامل یک روز برای نمایش سه‌گانه.
pub struct CalendarDay {
    pub date: NaiveDate,
    /// نام روز هفته: شنبه=۰ … جمعه=۶ (هفته‌ی ایرانی)
    pub weekday: u32,
    pub jalali: String,
    pub jalali_year: i32,
    pub jalali_month: u32,
    pub jalali_day: u32,
    pub hijri: String,
    pub hijri_year: i32,
    pub hijri_month: u32,
    pub hijri_day: u32,
    pub hijri_month_name: &'static str,
    pub gregorian: String,
    pub occasions: Vec<OccasionDay>,
}

/// نمای کامل یک روز تقویمی.
pub fn calendar_day(date: NaiveDate) -> CalendarDay {
    let jalali_date = jalali::from_gregorian(date);
    let hijri_date = to_hijri(date);
    CalendarDay {
        date,
        // هفته‌ی ایرانی: شنبه=۰ … جمعه=۶ — از مبدأ یک‌شنبه‌ی chrono جابه‌جا می‌شود.
        weekday: (date.weekday().num_days_from_sunday() + 1) % 7,
        jalali: jalali_date.format(),
        jalali_year: jalali_date.year,
        jalali_month: jalali_date.month,
        jalali_day: jalali_date.day,
        hijri: hijri_date.format(),
        hijri_year: hijri_date.year,
        hijri_month: hijri_date.month,
        hijri_day: hijri_date.day,
        hijri_month_name: hijri_date.month_name(),
        gregorian: jalali::iso_string(date),
        occasions: occasions_on(date),
    }
}
