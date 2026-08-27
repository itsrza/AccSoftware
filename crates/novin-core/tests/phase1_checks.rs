#![allow(warnings)]
// موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! # تست‌های سخت‌گیرانه‌ی فاز ۱ — چرخه‌ی چک و راس‌گیری
//!
//! این مجموعه بر پایه‌ی **داده‌ی واقعی** صفحه‌ی «دفتر اسناد دریافتنی/پرداختنی»
//! نرم‌افزار فعلی نوین پرداز (تصویر `1hNwr0`) نوشته شده است: هشت فقره چک با
//! مبالغ و تاریخ‌های واقعی که جمع کل آن‌ها در نرم‌افزار `1,327,076,888` ریال است.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | سازگاری با نرم‌افزار فعلی | جمع و روزشمار دقیقاً با اعداد نمایش‌داده‌شده یکی است |
//! | ۲ | راس‌گیری | فرمول وزنی و تاریخ راس درست و پایدارند |
//! | ۳ | راس‌گیری | حالت‌های مرزی (تک‌چک، سبد خالی، مبلغ صفر) کنترل می‌شوند |
//! | ۴ | چرخه‌ی چک دریافتی | مسیر موجود → واگذار → وصول درست است |
//! | ۵ | چرخه‌ی چک دریافتی | گذارهای غیرمجاز قاطعانه رد می‌شوند |
//! | ۶ | چرخه‌ی چک پرداختی | مسیر پرداختی → پرداخت‌شده و برگشت درست است |
//! | ۷ | چک انتظامی | هرگز اثر مالی ندارد |
//! | ۸ | اثر خزانه | فقط گذارهای درست، مانده را جابه‌جا می‌کنند |
//! | ۹ | اعتبارسنجی ثبت چک | مبلغ و ترتیب تاریخ‌ها کنترل می‌شود |
//! | ۱۰ | یادآوری سررسید | فقط چک‌های باز و در بازه انتخاب می‌شوند |

use chrono::NaiveDate;
use novin_core::checks::{
    allowed_transitions, due_within, maturity_date, status_belongs_to_kind, transition,
    treasury_effect, validate_check, weighted_maturity, CheckError, CheckItem, CheckKind,
    CheckStatus, TreasuryEffect,
};
use novin_core::jalali::JalaliDate;
use novin_core::money::Money;

/// تاریخ سیستم در تصویر مرجع: ۱۴۰۵/۰۵/۲۹
fn reference_today() -> NaiveDate {
    JalaliDate::new(1405, 5, 29)
        .unwrap()
        .to_gregorian()
        .unwrap()
}

/// هشت فقره چک واقعی از تصویر `1hNwr0` (مبلغ ریالی، سررسید شمسی).
fn reference_portfolio() -> Vec<CheckItem> {
    [
        (7_000_000, (1401, 7, 21)),
        (10_000_000, (1401, 8, 23)),
        (3_820_000, (1401, 8, 25)),
        (5_000_000, (1401, 9, 20)),
        (5_570_000, (1402, 3, 1)),
        (686_888, (1402, 3, 9)),
        (45_000_000, (1402, 3, 20)),
        (1_250_000_000, (1404, 5, 31)),
    ]
    .into_iter()
    .map(|(amount, (year, month, day))| {
        CheckItem::new(
            Money::from_rials(amount),
            JalaliDate::new(year, month, day)
                .unwrap()
                .to_gregorian()
                .unwrap(),
        )
    })
    .collect()
}

// ---------------------------------------------------------------------------
// تست ۱ — انطباق عدد به عدد با نرم‌افزار فعلی
// ---------------------------------------------------------------------------
#[test]
fn t01_matches_legacy_software_numbers_exactly() {
    let today = reference_today();
    let portfolio = reference_portfolio();

    // «جمع کل چکها: 1,327,076,888 ریال» در تصویر
    let total: Money = portfolio.iter().map(|item| item.amount).sum();
    assert_eq!(total, Money::from_rials(1_327_076_888));
    assert_eq!(total.format_grouped(), "1,327,076,888");
    assert_eq!(portfolio.len(), 8, "«تعداد کل چکها: 8 فقره»");

    // «راس چکهای انتخاب شده: -1407 روز» = فاصله‌ی سررسید ردیف اول تا تاریخ سیستم
    assert_eq!(
        portfolio[0].days_to_due(today),
        -1407,
        "روزشمار تقویم باید با نرم‌افزار فعلی مو‌به‌مو یکی باشد"
    );

    // چند نقطه‌ی دیگر برای اطمینان از نبود خطای انباشته
    assert_eq!(portfolio[1].days_to_due(today), -1375);
    assert_eq!(portfolio[7].days_to_due(today), -363);
}

// ---------------------------------------------------------------------------
// تست ۲ — راس‌گیری وزنی
// ---------------------------------------------------------------------------
#[test]
fn t02_weighted_maturity_is_correct() {
    let today = reference_today();
    let portfolio = reference_portfolio();
    let average = weighted_maturity(today, &portfolio).unwrap();

    assert_eq!(average.count, 8);
    assert_eq!(average.total_amount, Money::from_rials(1_327_076_888));
    // Σ(مبلغ×روز)/Σ(مبلغ) ≈ -413.89 → گرد به -414
    assert_eq!(average.days, -414);

    // تاریخ راس باید دقیقاً همان تعداد روز از مبنا فاصله داشته باشد
    let maturity = maturity_date(today, &portfolio).unwrap();
    assert_eq!((maturity - today).num_days(), -414);

    // راس سبدی با دو چک هم‌مبلغ باید وسط دو سررسید بیفتد
    let symmetric = vec![
        CheckItem::new(
            Money::from_rials(1_000_000),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        ),
        CheckItem::new(
            Money::from_rials(1_000_000),
            NaiveDate::from_ymd_opt(2026, 1, 11).unwrap(),
        ),
    ];
    let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    assert_eq!(weighted_maturity(base, &symmetric).unwrap().days, 5);

    // چک سنگین‌تر باید راس را به سمت خود بکشد: (۹×۰ + ۱×۱۰۰)/۱۰ = ۱۰ روز
    let skewed = vec![
        CheckItem::new(Money::from_rials(9_000_000), base),
        CheckItem::new(
            Money::from_rials(1_000_000),
            base + chrono::Duration::days(100),
        ),
    ];
    assert_eq!(weighted_maturity(base, &skewed).unwrap().days, 10);
}

// ---------------------------------------------------------------------------
// تست ۳ — حالت‌های مرزی راس‌گیری
// ---------------------------------------------------------------------------
#[test]
fn t03_maturity_edge_cases_are_guarded() {
    let base = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();

    assert_eq!(
        weighted_maturity(base, &[]),
        Err(CheckError::EmptyPortfolio)
    );

    // سبدی با جمع صفر (مثلاً چک ابطال‌شده با مبلغ صفر) نباید تقسیم بر صفر بدهد
    let zero = vec![CheckItem::new(Money::ZERO, base)];
    assert_eq!(
        weighted_maturity(base, &zero),
        Err(CheckError::InvalidAmount)
    );

    // تک‌چک: راس همان فاصله‌ی خودش است
    let single = vec![CheckItem::new(
        Money::from_rials(5_000_000),
        NaiveDate::from_ymd_opt(2026, 6, 19).unwrap(),
    )];
    assert_eq!(weighted_maturity(base, &single).unwrap().days, 30);
    assert_eq!(
        maturity_date(base, &single).unwrap(),
        NaiveDate::from_ymd_opt(2026, 6, 19).unwrap()
    );

    // سبد کاملاً گذشته باید راس منفی بدهد
    let past = vec![CheckItem::new(
        Money::from_rials(1_000),
        NaiveDate::from_ymd_opt(2025, 5, 20).unwrap(),
    )];
    assert!(weighted_maturity(base, &past).unwrap().days < 0);
}

// ---------------------------------------------------------------------------
// تست ۴ — چرخه‌ی موفق چک دریافتی
// ---------------------------------------------------------------------------
#[test]
fn t04_received_check_happy_path() {
    let kind = CheckKind::Received;
    let mut status = CheckStatus::initial(kind, false);
    assert_eq!(status, CheckStatus::InHand);
    assert_eq!(status.label(), "موجود");
    assert!(status.is_open());

    status = transition(kind, status, CheckStatus::Deposited).unwrap();
    assert_eq!(status.label(), "واگذار شده");

    status = transition(kind, status, CheckStatus::Collected).unwrap();
    assert_eq!(status.label(), "وصول شده");
    assert!(status.is_terminal());
    assert!(!status.is_open(), "چک وصول‌شده دیگر در جریان نیست");

    // مسیرهای جایگزین معتبر
    assert!(transition(kind, CheckStatus::InHand, CheckStatus::Endorsed).is_ok());
    assert!(transition(kind, CheckStatus::InHand, CheckStatus::Cashed).is_ok());
    assert!(transition(kind, CheckStatus::InHand, CheckStatus::Returned).is_ok());
    assert!(transition(kind, CheckStatus::InHand, CheckStatus::Void).is_ok());
    // چک برگشتی قابل واگذاری مجدد است
    assert!(transition(kind, CheckStatus::Bounced, CheckStatus::Deposited).is_ok());

    // هر وضعیت باید نام یکتا و قابل بازخوانی داشته باشد
    for candidate in allowed_transitions(kind, CheckStatus::InHand) {
        assert_eq!(CheckStatus::parse(candidate.as_str()), Some(*candidate));
    }
}

// ---------------------------------------------------------------------------
// تست ۵ — گذارهای غیرمجاز رد می‌شوند
// ---------------------------------------------------------------------------
#[test]
fn t05_invalid_transitions_are_rejected() {
    let kind = CheckKind::Received;

    // چک وصول‌شده نمی‌تواند دوباره واگذار شود
    assert_eq!(
        transition(kind, CheckStatus::Collected, CheckStatus::Deposited),
        Err(CheckError::InvalidTransition {
            from: "collected",
            to: "deposited"
        })
    );
    // پرش از روی واگذاری مستقیم به وصول مجاز نیست
    assert!(transition(kind, CheckStatus::InHand, CheckStatus::Collected).is_err());
    // چک باطل‌شده هیچ گذاری ندارد
    assert!(allowed_transitions(kind, CheckStatus::Void).is_empty());
    assert!(transition(kind, CheckStatus::Void, CheckStatus::InHand).is_err());
    // وضعیت مخصوص چک پرداختی روی چک دریافتی معنا ندارد
    assert_eq!(
        transition(kind, CheckStatus::InHand, CheckStatus::Paid),
        Err(CheckError::StatusNotAllowedForKind { kind: "received" })
    );
    assert!(!status_belongs_to_kind(
        CheckKind::Received,
        CheckStatus::Outstanding
    ));
    assert!(!status_belongs_to_kind(
        CheckKind::Issued,
        CheckStatus::Deposited
    ));
}

// ---------------------------------------------------------------------------
// تست ۶ — چرخه‌ی چک پرداختی
// ---------------------------------------------------------------------------
#[test]
fn t06_issued_check_lifecycle() {
    let kind = CheckKind::Issued;
    let mut status = CheckStatus::initial(kind, false);
    assert_eq!(status, CheckStatus::Outstanding);
    assert_eq!(status.label(), "پرداختی");

    status = transition(kind, status, CheckStatus::Paid).unwrap();
    assert_eq!(status.label(), "پرداخت شده");
    assert!(status.is_terminal());

    // چک برگشتی می‌تواند دوباره در جریان قرار گیرد یا پرداخت شود
    assert!(transition(kind, CheckStatus::Outstanding, CheckStatus::Bounced).is_ok());
    assert!(transition(kind, CheckStatus::Bounced, CheckStatus::Outstanding).is_ok());
    assert!(transition(kind, CheckStatus::Bounced, CheckStatus::Paid).is_ok());
    // ابطال فقط پیش از پرداخت ممکن است
    assert!(transition(kind, CheckStatus::Outstanding, CheckStatus::Void).is_ok());
    assert!(transition(kind, CheckStatus::Paid, CheckStatus::Void).is_err());
}

// ---------------------------------------------------------------------------
// تست ۷ — چک انتظامی هرگز اثر مالی ندارد
// ---------------------------------------------------------------------------
#[test]
fn t07_memo_checks_never_touch_the_books() {
    for kind in [CheckKind::Received, CheckKind::Issued] {
        let status = CheckStatus::initial(kind, true);
        assert_eq!(status, CheckStatus::MemoInHand);
        assert!(status.is_memo());
        assert!(!status.is_open(), "چک انتظامی جزو دارایی جاری نیست");

        let returned = transition(kind, status, CheckStatus::MemoReturned).unwrap();
        assert_eq!(returned.label(), "انتظامی عودت شده");
        // «خروج از عودت چک انتظامی» در نرم‌افزار فعلی
        assert!(transition(kind, returned, CheckStatus::MemoInHand).is_ok());

        // هیچ گذار انتظامی نباید اثر خزانه‌ای داشته باشد
        assert_eq!(
            treasury_effect(kind, status, returned),
            TreasuryEffect::None
        );
        assert_eq!(
            treasury_effect(kind, returned, status),
            TreasuryEffect::None
        );
        assert_eq!(
            treasury_effect(kind, status, CheckStatus::Void),
            TreasuryEffect::None
        );
    }
}

// ---------------------------------------------------------------------------
// تست ۸ — اثر خزانه‌ای گذارها
// ---------------------------------------------------------------------------
#[test]
fn t08_treasury_effects_are_exact() {
    use CheckStatus::*;

    // وصول و نقد کردن چک دریافتی → افزایش موجودی
    assert_eq!(
        treasury_effect(CheckKind::Received, Deposited, Collected),
        TreasuryEffect::Increase
    );
    assert_eq!(
        treasury_effect(CheckKind::Received, InHand, Cashed),
        TreasuryEffect::Increase
    );
    // پرداخت چک شخصی → کاهش موجودی
    assert_eq!(
        treasury_effect(CheckKind::Issued, Outstanding, Paid),
        TreasuryEffect::Decrease
    );
    // برگشت چکی که قبلاً وصول شده بود → اثر معکوس
    assert_eq!(
        treasury_effect(CheckKind::Received, Collected, Bounced),
        TreasuryEffect::Decrease
    );
    // صرف واگذاری به بانک هنوز پولی وارد نکرده است
    assert_eq!(
        treasury_effect(CheckKind::Received, InHand, Deposited),
        TreasuryEffect::None
    );
    // خرج کردن چک، وجه نقد ایجاد نمی‌کند
    assert_eq!(
        treasury_effect(CheckKind::Received, InHand, Endorsed),
        TreasuryEffect::None
    );
    // ابطال و عودت اثر خزانه‌ای ندارند
    assert_eq!(
        treasury_effect(CheckKind::Received, InHand, Void),
        TreasuryEffect::None
    );
    assert_eq!(
        treasury_effect(CheckKind::Issued, Outstanding, Returned),
        TreasuryEffect::None
    );
}

// ---------------------------------------------------------------------------
// تست ۹ — اعتبارسنجی ثبت چک
// ---------------------------------------------------------------------------
#[test]
fn t09_check_registration_is_validated() {
    let issue = JalaliDate::new(1404, 5, 6).unwrap().to_gregorian().unwrap();
    let due = JalaliDate::new(1404, 5, 31)
        .unwrap()
        .to_gregorian()
        .unwrap();

    // چک واقعی ردیف آخر تصویر مرجع
    assert!(validate_check(Money::from_rials(1_250_000_000), issue, due).is_ok());
    // سررسید هم‌روز صدور مجاز است (چک روز)
    assert!(validate_check(Money::from_rials(1_000), issue, issue).is_ok());

    assert_eq!(
        validate_check(Money::ZERO, issue, due),
        Err(CheckError::InvalidAmount)
    );
    assert_eq!(
        validate_check(Money::from_rials(-1), issue, due),
        Err(CheckError::InvalidAmount)
    );
    assert_eq!(
        validate_check(Money::from_rials(1_000), due, issue),
        Err(CheckError::DueBeforeIssue)
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — یادآوری چک‌های نزدیک سررسید
// ---------------------------------------------------------------------------
#[test]
fn t10_due_soon_filter_only_returns_open_checks() {
    let base = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
    let at = |offset: i64| base + chrono::Duration::days(offset);

    let items = vec![
        // در بازه و باز
        (
            CheckItem::new(Money::from_rials(100), at(0)),
            CheckStatus::InHand,
        ),
        (
            CheckItem::new(Money::from_rials(200), at(7)),
            CheckStatus::Deposited,
        ),
        // در بازه ولی بسته یا انتظامی → نباید بیاید
        (
            CheckItem::new(Money::from_rials(300), at(3)),
            CheckStatus::Collected,
        ),
        (
            CheckItem::new(Money::from_rials(400), at(4)),
            CheckStatus::Void,
        ),
        (
            CheckItem::new(Money::from_rials(500), at(5)),
            CheckStatus::MemoInHand,
        ),
        // خارج از بازه
        (
            CheckItem::new(Money::from_rials(600), at(30)),
            CheckStatus::InHand,
        ),
        (
            CheckItem::new(Money::from_rials(700), at(-1)),
            CheckStatus::InHand,
        ),
    ];

    let due_soon = due_within(base, 7, &items);
    assert_eq!(due_soon.len(), 2, "فقط چک‌های باز و داخل بازه");
    let amounts: Vec<i64> = due_soon.iter().map(|item| item.amount.rials()).collect();
    assert_eq!(amounts, vec![100, 200]);

    // مرز بازه شامل است، یک روز بعدش نه
    assert_eq!(due_within(base, 6, &items).len(), 1);
    assert_eq!(due_within(base, 0, &items).len(), 1);
    // چک سررسیدگذشته در یادآوری نمی‌آید (آن برای گزارش معوق است)
    assert!(due_within(base, 7, &items)
        .iter()
        .all(|item| item.days_to_due(base) >= 0));
}
