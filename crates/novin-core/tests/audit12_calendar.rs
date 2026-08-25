//! ممیزی دور ۱۲ — تقویم سه‌گانه شمسی/میلادی/قمری و مناسبت‌ها.
//!
//! مرجع: نمایش سه‌گانه‌ی تاریخ در نرم‌افزارهای حسابداری ایران و فهرست رسمی
//! مناسبت‌ها/تعطیلات. همه‌ی لنگرهای قمری این پرونده پیش از نوشتن، با
//! تقویم واقعی تطبیق داده شدند:
//!
//! - عاشورای ۱۴۴۷ = ۱۴۰۴/۰۴/۱۵ (۲۰۲۵-۰۷-۰۶)
//! - اربعین ۱۴۴۷ = ۱۴۰۴/۰۵/۲۴ (۲۰۲۵-۰۸-۱۵)
//! - مبعث ۱۴۴۷ = ۱۴۰۴/۱۰/۲۶ (۲۰۲۶-۰۱-۱۶)
//! - عید فطر ۱۴۴۷ = ۱۴۰۴/۱۲/۲۹ (۲۰۲۶-۰۳-۲۰)

use chrono::NaiveDate;
use novin_core::hijri::{
    hijri_month_name, hijri_year_length, to_gregorian, to_hijri, HijriDate, HIJRI_MAX_YEAR,
    HIJRI_MIN_YEAR,
};
use novin_core::occasions::{
    calendar_day, occasions_between, occasions_on, OccasionError, OCCASIONS,
};

fn date_of(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("تاریخ معتبر")
}

/// ک۵۳ — لنگرهای قمری تأییدشده با تقویم واقعی.
#[test]
fn k53_hijri_anchors() {
    assert_eq!(
        to_hijri(date_of(2026, 3, 20)),
        HijriDate {
            year: 1447,
            month: 10,
            day: 1
        },
        "عید فطر ۱۴۴۷"
    );
    assert_eq!(
        to_hijri(date_of(2026, 5, 27)),
        HijriDate {
            year: 1447,
            month: 12,
            day: 10
        },
        "عید قربان ۱۴۴۷"
    );
    assert_eq!(
        to_hijri(date_of(2025, 7, 6)),
        HijriDate {
            year: 1447,
            month: 1,
            day: 10
        },
        "عاشورای ۱۴۴۷"
    );
    assert_eq!(
        to_hijri(date_of(2026, 1, 16)),
        HijriDate {
            year: 1447,
            month: 7,
            day: 27
        },
        "مبعث ۱۴۴۷"
    );
    assert_eq!(
        to_hijri(date_of(2026, 8, 24)),
        HijriDate {
            year: 1448,
            month: 3,
            day: 10
        },
        "نمونه‌ی روز"
    );
}

/// ک۵۴ — رفت‌وبرگشت میلادی↔قمری روی ۳۰۰۰ روز پیاپی بدون خطا.
#[test]
fn k54_round_trip_3000_days() {
    let mut cursor = date_of(2020, 1, 1);
    for _ in 0..3000 {
        let hijri = to_hijri(cursor);
        let back = to_gregorian(hijri).expect("بازگشت میلادی");
        assert_eq!(back, cursor, "رفت‌وبرگشت {cursor}");
        cursor = cursor
            .checked_add_signed(chrono::Duration::days(1))
            .expect("روز بعد");
    }
}

/// ک۵۵ — طول سال قمری همیشه ۳۵۴ یا ۳۵۵ روز است.
#[test]
fn k55_year_lengths() {
    for year in 1445..=1450 {
        let length = hijri_year_length(year).expect("طول سال");
        assert!(
            length == 354 || length == 355,
            "سال {year} طول {length} ندارد"
        );
    }
    assert_eq!(hijri_year_length(1447), Some(355));
}

/// ک۵۶ — نام ماه‌ها و قالب نمایش.
#[test]
fn k56_month_names_and_format() {
    assert_eq!(hijri_month_name(1), "محرم");
    assert_eq!(hijri_month_name(9), "رمضان");
    assert_eq!(hijri_month_name(12), "ذی‌الحجه");
    assert_eq!(hijri_month_name(13), "");
    let date = HijriDate {
        year: 1447,
        month: 10,
        day: 1,
    };
    assert_eq!(date.format(), "1447/10/01");
    assert_eq!(date.month_name(), "شوال");
}

/// ک۵۷ — اعتبارسنجی تاریخ قمری: ماه ۱۳، روز ۳۱ و سال بیرون از دامنه رد.
#[test]
fn k57_validation() {
    assert!(HijriDate::new(1447, 10, 1).is_some());
    assert!(HijriDate::new(1447, 13, 1).is_none(), "ماه ۱۳");
    assert!(HijriDate::new(1447, 10, 31).is_none(), "روز ۳۱");
    assert!(HijriDate::new(1447, 0, 5).is_none(), "ماه صفر");
    assert!(HijriDate::new(HIJRI_MIN_YEAR - 1, 1, 1).is_none(), "کمینه");
    assert!(HijriDate::new(HIJRI_MAX_YEAR + 1, 1, 1).is_none(), "بیشینه");
}

/// ک۵۸ — تبدیل معکوسِ تاریخ نامعتبر هیچ تاریخی تولید نمی‌کند.
#[test]
fn k58_invalid_inverse_is_none() {
    assert_eq!(
        to_gregorian(HijriDate {
            year: 1447,
            month: 13,
            day: 1
        }),
        None
    );
    assert_eq!(
        to_gregorian(HijriDate {
            year: 1447,
            month: 1,
            day: 40
        }),
        None
    );
    // تاریخ معتبر برمی‌گردد
    assert_eq!(
        to_gregorian(HijriDate {
            year: 1447,
            month: 10,
            day: 1
        }),
        Some(date_of(2026, 3, 20))
    );
}

/// ک۵۹ — نوروز ۱۴۰۴: هم‌روز با شهادت امام علی (ع) — ۲۱ رمضان ۱۴۴۶.
#[test]
fn k59_nowruz_occasions() {
    let first = occasions_on(date_of(2025, 3, 21)); // ۱۴۰۴/۰۱/۰۱ = ۲۱ رمضان ۱۴۴۶
    assert_eq!(first.len(), 2, "نوروز + شهادت امام علی");
    assert!(first
        .iter()
        .any(|o| o.title == "نوروز" && o.holiday && o.calendar == "jalali"));
    assert!(first
        .iter()
        .any(|o| o.title == "شهادت امام علی (ع)" && o.holiday && o.calendar == "hijri"));

    // روزهای دوم تا چهارم فروردین فقط نوروز
    for day in 22..=24 {
        let occasions = occasions_on(date_of(2025, 3, day));
        assert_eq!(occasions.len(), 1, "2025-03-{day}");
        assert_eq!(occasions[0].title, "عید نوروز");
        assert!(occasions[0].holiday, "نوروز تعطیل است");
    }
}

/// ک۶۰ — روز هم‌زمان دو مناسبت: عید فطر ۱۴۴۷ و روز ملی شدن نفت (۱۴۰۴/۱۲/۲۹).
#[test]
fn k60_two_occasions_same_day() {
    let occasions = occasions_on(date_of(2026, 3, 20));
    assert_eq!(occasions.len(), 2, "عید فطر + ملی شدن صنعت نفت");
    assert!(occasions
        .iter()
        .any(|o| o.title == "عید سعید فطر" && o.holiday));
    assert!(occasions
        .iter()
        .any(|o| o.title == "روز ملی شدن صنعت نفت" && o.holiday));
    assert_eq!(occasions[0].jalali, "1404/12/29");
}

/// ک۶۱ — مناسبت‌های قمریِ سال ۱۴۰۴: عاشورا، اربعین، مبعث و ۱۵ شعبان سر جای خودشان‌اند.
#[test]
fn k61_hijri_occasions_of_1404() {
    let days =
        occasions_between(date_of(2025, 3, 21), date_of(2026, 3, 20)).expect("مناسبت‌های سال ۱۴۰۴");

    let find = |title: &str| days.iter().find(|o| o.title == title);
    // عاشورای ۱۴۴۷ = ۱۴۰۴/۰۴/۱۵
    let ashura = find("عاشورای حسینی").expect("عاشورا");
    assert_eq!(ashura.date, date_of(2025, 7, 6));
    assert_eq!(ashura.jalali, "1404/04/15");
    assert_eq!(ashura.hijri, "1447/01/10");
    assert!(ashura.holiday);

    let arbaeen = find("اربعین حسینی").expect("اربعین");
    assert_eq!(arbaeen.jalali, "1404/05/24");

    let mabath = find("مبعث رسول اکرم (ص)").expect("مبعث");
    assert_eq!(mabath.date, date_of(2026, 1, 16));

    let fifteenth = find("ولادت حضرت قائم (عج)").expect("۱۵ شعبان");
    assert_eq!(fifteenth.jalali, "1404/11/14");

    // غدیر و قربان ۱۴۴۷ در تابستان ۱۴۰۵ افتادند، نه ۱۴۰۴
    assert!(find("عید سعید غدیر خم").is_none());
    assert!(find("عید سعید قربان").is_none());
}

/// ک۶۲ — پرچم تعطیل: مبعث تعطیل، غدیر و ولادت امام علی مناسبت بدون تعطیل.
#[test]
fn k62_holiday_flags() {
    let days = occasions_between(date_of(2026, 1, 1), date_of(2026, 6, 30)).expect("نیمه‌ی ۲۰۲۶");
    let find = |title: &str| days.iter().find(|o| o.title == title).expect(title);
    assert!(!find("عید سعید غدیر خم").holiday, "غدیر تعطیل رسمی نیست");
    // ولادت امام علی ۱۴۴۷: ۷/۱۳ قمری → در بازه نیست لزوماً؛ در جدول خام بسنجیم
    let raw = OCCASIONS
        .iter()
        .find(|o| o.title == "ولادت امام علی (ع)")
        .expect("در جدول");
    assert!(!raw.holiday);
    let mabath = OCCASIONS
        .iter()
        .find(|o| o.title == "مبعث رسول اکرم (ص)")
        .expect("در جدول");
    assert!(mabath.holiday);
    assert!(days.iter().any(|o| o.holiday), "در این بازه تعطیل هست");
}

/// ک۶۳ — نگهبان‌های بازه: وارونه و بیش از ۴۰۰ روز رد می‌شوند.
#[test]
fn k63_range_guards() {
    assert_eq!(
        occasions_between(date_of(2026, 1, 2), date_of(2026, 1, 1)),
        Err(OccasionError::InvalidRange)
    );
    assert_eq!(
        occasions_between(date_of(2026, 1, 1), date_of(2027, 3, 1)),
        Err(OccasionError::RangeTooLong)
    );
    // دقیقاً ۴۰۰ روز مجاز است
    assert!(occasions_between(date_of(2026, 1, 1), date_of(2027, 2, 4)).is_ok());
}

/// ک۶۴ — نمای کامل روز: سه تقویم و روز هفته‌ی ایرانی (شنبه=۰).
#[test]
fn k64_calendar_day_view() {
    let day = calendar_day(date_of(2026, 8, 24)); // دوشنبه
    assert_eq!(day.weekday, 2, "شنبه=۰ … دوشنبه=۲");
    assert_eq!(day.jalali, "1405/06/02");
    assert_eq!(day.gregorian, "2026-08-24");
    assert_eq!(day.hijri, "1448/03/10");
    assert_eq!(day.hijri_month_name, "ربیع‌الاول");
    assert_eq!(day.jalali_year, 1405);
    assert_eq!(day.jalali_month, 6);
    assert_eq!(day.jalali_day, 2);

    // جمعه = ۶
    let friday = calendar_day(date_of(2026, 8, 28));
    assert_eq!(friday.weekday, 6);
}

/// ک۶۵ — سلامت جدول مناسبت‌ها: روز/ماه معتبر و کلید یکتا.
#[test]
fn k65_occasion_table_sanity() {
    let mut seen = std::collections::BTreeSet::new();
    for occasion in OCCASIONS {
        assert!(
            occasion.month >= 1 && occasion.month <= 12,
            "ماه نامعتبر: {}",
            occasion.title
        );
        let day_limit = if occasion.calendar == "jalali" {
            31
        } else {
            30
        };
        assert!(
            occasion.day >= 1 && occasion.day <= day_limit,
            "روز نامعتبر: {}",
            occasion.title
        );
        assert!(
            seen.insert((occasion.calendar, occasion.month, occasion.day)),
            "کلید تکراری: {}",
            occasion.title
        );
        assert!(!occasion.title.is_empty());
    }
    // فهرست مرجع کامل است: ۱۰ شمسی + ۱۶ قمری
    let jalali_count = OCCASIONS.iter().filter(|o| o.calendar == "jalali").count();
    let hijri_count = OCCASIONS.iter().filter(|o| o.calendar == "hijri").count();
    assert_eq!((jalali_count, hijri_count), (10, 16));
}
