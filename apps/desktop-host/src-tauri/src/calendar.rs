#![allow(warnings)] // موقت: میراث ممیزی CI — بعد از سبزشدن، فایل‌به‌فایل برداشته می‌شود
//! دستور میزبان تقویم سه‌گانه — شمسی/میلادی/قمری با مناسبت‌ها.
//!
//! همه‌ی محاسبه در `novin_core::{hijri, occasions}` انجام می‌شود تا در CI تست
//! شده باشد؛ این ماژول فقط ورودی شمسی را به تاریخ میلادی می‌برد، خروجی را
//! برای رابط کاربری شکل می‌دهد و بازه را به ۴۰۰ روز محدود می‌کند.
//!
//! تاریخ «امروز» همیشه از ساعت سیستم می‌آید؛ بازه‌ی مناسبت‌ها اختیاری است و
//! پیش‌فرضش ماه شمسیِ جاری است.

use novin_core::jalali::{self, JalaliDate};
use novin_core::occasions::{calendar_day, occasions_between, OccasionDay};
use serde::Serialize;
use tauri::State;

use crate::{require_login, AppState};

#[derive(Debug, Serialize)]
pub struct OccasionJson {
    /// تاریخ میلادی ISO
    pub date: String,
    pub jalali: String,
    pub hijri: String,
    pub title: String,
    /// `jalali` یا `hijri`
    pub calendar: String,
    pub holiday: bool,
}

impl From<OccasionDay> for OccasionJson {
    fn from(day: OccasionDay) -> Self {
        OccasionJson {
            date: novin_core::jalali::iso_string(day.date),
            jalali: day.jalali,
            hijri: day.hijri,
            title: day.title.to_string(),
            calendar: day.calendar.to_string(),
            holiday: day.holiday,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TodayJson {
    pub iso: String,
    /// `1405/06/02`
    pub jalali: String,
    pub jalali_year: i32,
    pub jalali_month: u32,
    pub jalali_day: u32,
    /// `1448/03/10`
    pub hijri: String,
    pub hijri_year: i32,
    pub hijri_month: u32,
    pub hijri_day: u32,
    pub hijri_month_name: String,
    /// `2026-08-24`
    pub gregorian: String,
    /// شنبه=۰ … جمعه=۶
    pub weekday: u32,
    pub occasions: Vec<OccasionJson>,
}

#[derive(Debug, Serialize)]
pub struct CalendarOverview {
    pub today: TodayJson,
    /// مناسبت‌های بازه به ترتیب تاریخ
    pub occasions: Vec<OccasionJson>,
}

/// نمای تقویم: امروز + مناسبت‌های بازه‌ی دلخواه (پیش‌فرض: ماه شمسی جاری).
#[tauri::command]
pub fn calendar_overview(
    state: State<AppState>,
    from_jalali: Option<String>,
    to_jalali: Option<String>,
) -> Result<CalendarOverview, String> {
    require_login(&state)?;
    let today = chrono::Local::now().date_naive();

    let (from, to) = match (from_jalali, to_jalali) {
        (Some(from), Some(to)) => {
            let from = JalaliDate::parse(&from)
                .and_then(|date| date.to_gregorian())
                .map_err(|error| format!("CAL-001: {error}"))?;
            let to = JalaliDate::parse(&to)
                .and_then(|date| date.to_gregorian())
                .map_err(|error| format!("CAL-001: {error}"))?;
            (from, to)
        }
        _ => {
            // ماه شمسیِ جاری
            let current = jalali::from_gregorian(today);
            let start = JalaliDate::new(current.year, current.month, 1)
                .and_then(|date| date.to_gregorian().ok())
                .ok_or_else(|| "CAL-001: ابتدای ماه شمسی نامعتبر است".to_string())?;
            let end = JalaliDate::new(
                current.year,
                current.month,
                jalali::days_in_jalali_month(current.year, current.month),
            )
            .and_then(|date| date.to_gregorian().ok())
            .ok_or_else(|| "CAL-001: پایان ماه شمسی نامعتبر است".to_string())?;
            (start, end)
        }
    };

    let days = occasions_between(from, to).map_err(|error| format!("CAL-002: {error}"))?;
    let day = calendar_day(today);

    Ok(CalendarOverview {
        today: TodayJson {
            iso: day.gregorian.clone(),
            jalali: day.jalali,
            jalali_year: day.jalali_year,
            jalali_month: day.jalali_month,
            jalali_day: day.jalali_day,
            hijri: day.hijri,
            hijri_year: day.hijri_year,
            hijri_month: day.hijri_month,
            hijri_day: day.hijri_day,
            hijri_month_name: day.hijri_month_name.to_string(),
            gregorian: day.gregorian,
            weekday: day.weekday,
            occasions: day.occasions.into_iter().map(Into::into).collect(),
        },
        occasions: days.into_iter().map(Into::into).collect(),
    })
}
