#![allow(warnings)]
// موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! # تست‌های سخت‌گیرانه‌ی فاز ۶ — انبارگردانی و عملیات جمعی
//!
//! مرجع: منوی «عملیات انبار» و لیست کالاهای نرم‌افزار فعلی.
//! بازخورد کارفرما: «انبارگردانی خیلی ساده و بدون استفاده پیاده شده؛ باید بر
//! اساس منطق حسابداری جلو برویم.»
//!
//! | # | موضوع | ادعای غیرقابل مذاکره |
//! |---|-------|------|
//! | ۱ | چرخه‌ی دوره | مسیر اجباری پیش‌نویس ← شمارش ← بازبینی ← ثبت |
//! | ۲ | فریز موجودی | مبنای مقایسه پس از فریز تغییر نمی‌کند |
//! | ۳ | شمارش مجدد | شمارش دوم بر اول ارجحیت دارد |
//! | ۴ | کنترل داخلی | اختلاف بزرگ باید دوباره شمرده شود |
//! | ۵ | تأیید اختلاف | ثبت بدون تأیید اختلاف ممنوع |
//! | ۶ | کامل بودن شمارش | ثبت با قلم شمارش‌نشده ممنوع |
//! | ۷ | سند تعدیل | کسری هزینه، اضافی درآمد، سند متعادل |
//! | ۸ | خلاصه | ارزش‌گذاری اختلاف با بهای واقعی |
//! | ۹ | عملیات جمعی | تغییر قیمت گروهی هرگز قیمت منفی نمی‌سازد |
//! | ۱۰ | کم‌موجودی + پایگاه داده | آستانه، مرتب‌سازی و اسکیما |

use novin_core::accounting::validate_journal;
use novin_core::inventory::ValuationMethod;
use novin_core::money::Money;
use novin_core::stocktaking::{
    build_adjustment_journal, ensure_postable, lines_needing_recount, low_stock_items,
    preview_bulk_price, summarize, transition, BulkError, BulkPriceChange, CountLine,
    StocktakeError, StocktakeStatus, VarianceAccounts,
};

fn accounts() -> VarianceAccounts {
    VarianceAccounts {
        inventory: "acc-1300".into(),
        shortage_expense: "acc-6300".into(),
        surplus_income: "acc-4300".into(),
    }
}

/// یک قلم شمارش‌شده و تأییدشده.
fn counted(product: &str, frozen: f64, counted: f64, unit_cost: i64) -> CountLine {
    let mut line = CountLine::new(product, frozen, Money::from_rials(unit_cost));
    line.counted_quantity = Some(counted);
    line.variance_approved = true;
    line
}

// ---------------------------------------------------------------------------
// تست ۱ — چرخه‌ی وضعیت دوره
// ---------------------------------------------------------------------------
#[test]
fn t01_stocktake_lifecycle_is_enforced() {
    let mut status = StocktakeStatus::Draft;
    assert_eq!(status.label(), "پیش‌نویس");
    assert!(!status.is_frozen(), "پیش‌نویس هنوز فریز نشده");

    status = transition(status, StocktakeStatus::Counting).unwrap();
    assert!(status.is_frozen(), "با شروع شمارش موجودی فریز می‌شود");

    status = transition(status, StocktakeStatus::Review).unwrap();
    assert_eq!(status.label(), "بازبینی اختلاف");

    // بازگشت به شمارش برای شمارش مجدد مجاز است
    assert!(transition(status, StocktakeStatus::Counting).is_ok());

    status = transition(status, StocktakeStatus::Posted).unwrap();
    assert!(status.is_locked());

    // پرش از بازبینی مستقیم به ثبت ممنوع است
    assert_eq!(
        transition(StocktakeStatus::Counting, StocktakeStatus::Posted),
        Err(StocktakeError::InvalidTransition {
            from: "counting",
            to: "posted"
        })
    );
    // پرش از پیش‌نویس مستقیم به بازبینی ممنوع
    assert!(transition(StocktakeStatus::Draft, StocktakeStatus::Review).is_err());
    // دوره‌ی ثبت‌شده هیچ گذاری ندارد
    assert!(transition(StocktakeStatus::Posted, StocktakeStatus::Counting).is_err());
    assert!(transition(StocktakeStatus::Cancelled, StocktakeStatus::Draft).is_err());

    for status in [
        StocktakeStatus::Draft,
        StocktakeStatus::Counting,
        StocktakeStatus::Review,
        StocktakeStatus::Posted,
        StocktakeStatus::Cancelled,
    ] {
        assert_eq!(StocktakeStatus::parse(status.as_str()), Some(status));
    }
}

// ---------------------------------------------------------------------------
// تست ۲ — فریز موجودی
// ---------------------------------------------------------------------------
#[test]
fn t02_frozen_quantity_is_the_only_baseline() {
    // موجودی سیستمی هنگام فریز ۱۰۰ بوده است
    let mut line = CountLine::new("prod-1", 100.0, Money::from_rials(50_000));
    assert_eq!(line.variance(), None, "پیش از شمارش اختلافی نیست");
    assert!(!line.is_counted());

    line.counted_quantity = Some(97.0);
    assert_eq!(line.variance(), Some(-3.0), "کسری سه عدد");
    assert!(line.has_variance());
    assert_eq!(line.variance_value().unwrap(), Money::from_rials(-150_000));

    // حتی اگر بعداً موجودی سیستمی عوض شود، مبنا همان عدد فریزشده است.
    // (شبیه‌سازی: ساخت قلم جدید با موجودی متفاوت، مقدار شمارش یکسان)
    let mut later = CountLine::new("prod-1", 90.0, Money::from_rials(50_000));
    later.counted_quantity = Some(97.0);
    assert_eq!(later.variance(), Some(7.0));
    assert_ne!(
        line.variance(),
        later.variance(),
        "تغییر مبنا نتیجه را عوض می‌کند؛ برای همین فریز لازم است"
    );

    // شمارش برابر موجودی یعنی بدون اختلاف
    let mut exact = CountLine::new("prod-2", 40.0, Money::from_rials(1_000));
    exact.counted_quantity = Some(40.0);
    assert!(!exact.has_variance());
    assert_eq!(exact.variance_value().unwrap(), Money::ZERO);
}

// ---------------------------------------------------------------------------
// تست ۳ — شمارش مجدد
// ---------------------------------------------------------------------------
#[test]
fn t03_recount_overrides_first_count() {
    let mut line = CountLine::new("prod-1", 100.0, Money::from_rials(10_000));
    line.counted_quantity = Some(80.0);
    assert_eq!(line.final_quantity(), Some(80.0));
    assert_eq!(line.variance(), Some(-20.0));

    // شمارش مجدد نتیجه را اصلاح می‌کند
    line.recount_quantity = Some(99.0);
    assert_eq!(line.final_quantity(), Some(99.0), "شمارش دوم ملاک است");
    assert_eq!(line.variance(), Some(-1.0));
    assert_eq!(line.variance_value().unwrap(), Money::from_rials(-10_000));

    // شمارش مجدد صفر هم معتبر است (کالا اصلاً موجود نیست)
    line.recount_quantity = Some(0.0);
    assert_eq!(line.final_quantity(), Some(0.0));
    assert_eq!(line.variance(), Some(-100.0));
}

// ---------------------------------------------------------------------------
// تست ۴ — الزام شمارش مجدد برای اختلاف بزرگ
// ---------------------------------------------------------------------------
#[test]
fn t04_large_variance_requires_recount() {
    let lines = vec![
        // اختلاف ۲٪ — زیر آستانه
        counted("p1", 100.0, 98.0, 1_000),
        // اختلاف ۲۰٪ — بالای آستانه
        counted("p2", 100.0, 80.0, 1_000),
        // اختلاف ۱۰٪ دقیقاً روی آستانه‌ی ۱۰
        counted("p3", 100.0, 110.0, 1_000),
        // بدون اختلاف
        counted("p4", 50.0, 50.0, 1_000),
    ];

    let needing = lines_needing_recount(&lines, 10.0);
    let ids: Vec<&str> = needing
        .iter()
        .map(|line| line.product_id.as_str())
        .collect();
    assert_eq!(ids, vec!["p2", "p3"], "مرز آستانه شامل است");

    // با آستانه‌ی ۱٪ تقریباً همه باید دوباره شمرده شوند
    assert_eq!(lines_needing_recount(&lines, 1.0).len(), 3);
    // با آستانه‌ی ۵۰٪ هیچ‌کدام
    assert_eq!(lines_needing_recount(&lines, 50.0).len(), 0);

    // قلمی که قبلاً دوباره شمرده شده، دیگر در فهرست نیست
    let mut recounted = lines.clone();
    recounted[1].recount_quantity = Some(100.0);
    assert_eq!(lines_needing_recount(&recounted, 10.0).len(), 1);

    // کالایی که موجودی سیستمی‌اش صفر بوده ولی شمارش عددی دارد
    let mut from_zero = CountLine::new("p5", 0.0, Money::from_rials(1_000));
    from_zero.counted_quantity = Some(3.0);
    assert_eq!(lines_needing_recount(&[from_zero], 10.0).len(), 1);
}

// ---------------------------------------------------------------------------
// تست ۵ — تأیید اختلاف اجباری است
// ---------------------------------------------------------------------------
#[test]
fn t05_unapproved_variance_blocks_posting() {
    let mut line = CountLine::new("p1", 100.0, Money::from_rials(10_000));
    line.counted_quantity = Some(95.0);
    line.variance_approved = false;

    assert_eq!(
        ensure_postable(&[line.clone()]),
        Err(StocktakeError::UnapprovedVariance { count: 1 })
    );
    assert_eq!(
        build_adjustment_journal(&[line.clone()], &accounts()),
        Err(StocktakeError::UnapprovedVariance { count: 1 })
    );

    // پس از تأیید، ثبت ممکن می‌شود
    line.variance_approved = true;
    assert!(ensure_postable(&[line]).is_ok());

    // قلم بدون اختلاف نیازی به تأیید ندارد
    let mut no_variance = CountLine::new("p2", 10.0, Money::from_rials(1_000));
    no_variance.counted_quantity = Some(10.0);
    no_variance.variance_approved = false;
    assert!(ensure_postable(&[no_variance]).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۶ — شمارش ناقص
// ---------------------------------------------------------------------------
#[test]
fn t06_incomplete_count_blocks_posting() {
    let complete = counted("p1", 10.0, 10.0, 1_000);
    let uncounted = CountLine::new("p2", 20.0, Money::from_rials(1_000));
    let another_uncounted = CountLine::new("p3", 30.0, Money::from_rials(1_000));

    assert_eq!(
        ensure_postable(&[complete.clone(), uncounted.clone(), another_uncounted]),
        Err(StocktakeError::IncompleteCount { remaining: 2 })
    );

    // دوره‌ی خالی
    assert_eq!(ensure_postable(&[]), Err(StocktakeError::EmptySession));

    // مقدار منفی مردود است
    let mut negative = complete.clone();
    negative.counted_quantity = Some(-1.0);
    assert_eq!(
        ensure_postable(&[negative]),
        Err(StocktakeError::NegativeCount)
    );

    // شمارش صفر معتبر است (کالا تمام شده)
    let zero = counted("p4", 5.0, 0.0, 1_000);
    assert!(ensure_postable(&[zero]).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۷ — سند تعدیل انبارگردانی
// ---------------------------------------------------------------------------
#[test]
fn t07_adjustment_journal_follows_accounting_rules() {
    // اضافی خالص: ۵ عدد × ۱۰۰٬۰۰۰ = ۵۰۰٬۰۰۰
    let surplus = vec![counted("p1", 100.0, 105.0, 100_000)];
    let journal = build_adjustment_journal(&surplus, &accounts()).unwrap();
    let totals = validate_journal(&journal).unwrap();
    assert_eq!(totals.total_debit, Money::from_rials(500_000));
    assert_eq!(journal[0].account_id, "acc-1300", "موجودی کالا بدهکار می‌شود");
    assert_eq!(journal[0].debit, Money::from_rials(500_000));
    assert_eq!(journal[1].account_id, "acc-4300", "اضافات انبار بستانکار");

    // کسری خالص: ۳ عدد × ۲۰۰٬۰۰۰ = ۶۰۰٬۰۰۰
    let shortage = vec![counted("p1", 50.0, 47.0, 200_000)];
    let journal = build_adjustment_journal(&shortage, &accounts()).unwrap();
    validate_journal(&journal).unwrap();
    let inventory_line = journal.iter().find(|l| l.account_id == "acc-1300").unwrap();
    let expense_line = journal.iter().find(|l| l.account_id == "acc-6300").unwrap();
    assert_eq!(
        inventory_line.credit,
        Money::from_rials(600_000),
        "موجودی بستانکار"
    );
    assert_eq!(
        expense_line.debit,
        Money::from_rials(600_000),
        "کسری هزینه است"
    );

    // ترکیب کسری و اضافی در یک دوره
    let mixed = vec![
        counted("p1", 100.0, 110.0, 50_000), // اضافی ۵۰۰٬۰۰۰
        counted("p2", 80.0, 74.0, 100_000),  // کسری ۶۰۰٬۰۰۰
    ];
    let journal = build_adjustment_journal(&mixed, &accounts()).unwrap();
    let totals = validate_journal(&journal).unwrap();
    assert_eq!(
        totals.total_debit, totals.total_credit,
        "سند باید متعادل باشد"
    );
    // اثر خالص روی موجودی: ۱۰۰٬۰۰۰ بستانکار
    let inventory_line = journal.iter().find(|l| l.account_id == "acc-1300").unwrap();
    assert_eq!(inventory_line.credit, Money::from_rials(100_000));
    // ولی درآمد و هزینه باید ناخالص ثبت شوند، نه خالص‌شده
    assert_eq!(
        journal
            .iter()
            .find(|l| l.account_id == "acc-4300")
            .unwrap()
            .credit,
        Money::from_rials(500_000)
    );
    assert_eq!(
        journal
            .iter()
            .find(|l| l.account_id == "acc-6300")
            .unwrap()
            .debit,
        Money::from_rials(600_000)
    );

    // انبارگردانی بدون اختلاف سند نمی‌خواهد
    let clean = vec![counted("p1", 10.0, 10.0, 1_000)];
    assert!(build_adjustment_journal(&clean, &accounts())
        .unwrap()
        .is_empty());

    // حساب تعریف‌نشده
    let missing = VarianceAccounts {
        inventory: String::new(),
        shortage_expense: "acc-6300".into(),
        surplus_income: "acc-4300".into(),
    };
    assert_eq!(
        build_adjustment_journal(&surplus, &missing),
        Err(StocktakeError::MissingVarianceAccounts)
    );
}

// ---------------------------------------------------------------------------
// تست ۸ — خلاصه‌ی دوره
// ---------------------------------------------------------------------------
#[test]
fn t08_summary_values_variance_at_real_cost() {
    let mut pending = CountLine::new("p3", 10.0, Money::from_rials(5_000));
    pending.counted_quantity = Some(8.0); // اختلاف تأییدنشده
    let lines = vec![
        counted("p1", 100.0, 110.0, 50_000), // اضافی ۵۰۰٬۰۰۰
        counted("p2", 80.0, 74.0, 100_000),  // کسری ۶۰۰٬۰۰۰
        pending,                             // کسری ۱۰٬۰۰۰ تأییدنشده
        CountLine::new("p4", 5.0, Money::from_rials(1_000)), // شمارش‌نشده
        counted("p5", 20.0, 20.0, 9_000),    // بدون اختلاف
    ];

    let summary = summarize(&lines).unwrap();
    assert_eq!(summary.total_lines, 5);
    assert_eq!(summary.counted_lines, 4);
    assert_eq!(summary.uncounted_lines, 1);
    assert_eq!(summary.surplus_lines, 1);
    assert_eq!(summary.shortage_lines, 2);
    assert_eq!(summary.unapproved_variances, 1);
    assert_eq!(summary.surplus_value, Money::from_rials(500_000));
    assert_eq!(summary.shortage_value, Money::from_rials(610_000));
    assert_eq!(summary.net_value, Money::from_rials(-110_000));

    // اختلاف با بهای واقعی ارزش‌گذاری می‌شود، نه با یک عدد ثابت
    let cheap = summarize(&[counted("x", 10.0, 5.0, 1_000)]).unwrap();
    let expensive = summarize(&[counted("x", 10.0, 5.0, 900_000)]).unwrap();
    assert_eq!(cheap.shortage_value, Money::from_rials(5_000));
    assert_eq!(expensive.shortage_value, Money::from_rials(4_500_000));

    // توضیح ساده‌ی روش‌های ارزش‌گذاری برای کاربر غیرحسابدار
    for method in [
        ValuationMethod::Fifo,
        ValuationMethod::MovingAverage,
        ValuationMethod::WeightedAverage,
    ] {
        assert!(method.plain_explanation().chars().count() > 80);
        assert!(!method.label().is_empty());
    }
}

// ---------------------------------------------------------------------------
// تست ۹ — عملیات جمعی روی قیمت
// ---------------------------------------------------------------------------
#[test]
fn t09_bulk_price_change_never_produces_negative() {
    let products = vec![
        ("p1".to_string(), Money::from_rials(100_000)),
        ("p2".to_string(), Money::from_rials(333_333)),
    ];

    // افزایش ۱۰٪
    let result = preview_bulk_price(&products, BulkPriceChange::Percent(1_000), 0).unwrap();
    assert_eq!(result[0].new_price, Money::from_rials(110_000));
    assert_eq!(result[0].difference, Money::from_rials(10_000));
    assert_eq!(result[1].new_price, Money::from_rials(366_666));

    // کاهش ۱۰٪
    let result = preview_bulk_price(&products, BulkPriceChange::Percent(-1_000), 0).unwrap();
    assert_eq!(result[0].new_price, Money::from_rials(90_000));

    // گرد کردن به ۱۰٬۰۰۰ ریال
    let result = preview_bulk_price(&products, BulkPriceChange::Percent(1_000), 10_000).unwrap();
    assert_eq!(result[1].new_price, Money::from_rials(370_000));

    // مبلغ ثابت و جایگزینی
    let result = preview_bulk_price(
        &products,
        BulkPriceChange::Amount(Money::from_rials(5_000)),
        0,
    )
    .unwrap();
    assert_eq!(result[0].new_price, Money::from_rials(105_000));
    let result =
        preview_bulk_price(&products, BulkPriceChange::Set(Money::from_rials(1_000)), 0).unwrap();
    assert_eq!(result[0].new_price, Money::from_rials(1_000));
    assert_eq!(result[1].new_price, Money::from_rials(1_000));

    // کاهش بیش از قیمت → کل عملیات رد می‌شود، نه اینکه بی‌صدا صفر شود
    assert_eq!(
        preview_bulk_price(
            &products,
            BulkPriceChange::Amount(Money::from_rials(-200_000)),
            0
        ),
        Err(BulkError::NegativeResult {
            product_id: "p1".into()
        })
    );
    // درصد نامعتبر و انتخاب خالی
    assert_eq!(
        preview_bulk_price(&products, BulkPriceChange::Percent(-20_000), 0),
        Err(BulkError::InvalidPercent)
    );
    assert_eq!(
        preview_bulk_price(&[], BulkPriceChange::Percent(100), 0),
        Err(BulkError::EmptySelection)
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — کم‌موجودی و اسکیمای پایگاه داده
// ---------------------------------------------------------------------------
#[test]
fn t10_low_stock_and_schema() {
    // (شناسه، نام، موجودی، حد سفارش)
    let products = vec![
        ("p1".to_string(), "پرینتر".to_string(), 2.0, 0.0),
        ("p2".to_string(), "بارکدخوان".to_string(), 7.0, 10.0),
        ("p3".to_string(), "برنج".to_string(), 40.0, 5.0),
        ("p4".to_string(), "پیراهن".to_string(), 0.0, 0.0),
    ];

    // آستانه‌ی عمومی ۵: p1 و p4 (زیر ۵) و p2 (زیر حد سفارش خودش ۱۰)
    let items = low_stock_items(&products, 5.0);
    let ids: Vec<&str> = items.iter().map(|item| item.product_id.as_str()).collect();
    assert_eq!(ids, vec!["p4", "p1", "p2"], "کم‌موجودترین‌ها اول");
    assert_eq!(items[0].quantity, 0.0);

    // حد سفارش خود کالا بر آستانه‌ی عمومی ارجح است وقتی بزرگ‌تر باشد
    assert!(items.iter().any(|item| item.product_id == "p2"));
    // آستانه‌ی صفر: فقط کالاهایی که حد سفارش دارند یا موجودی صفر است
    let strict = low_stock_items(&products, 0.0);
    assert_eq!(strict.len(), 2, "p4 با موجودی صفر و p2 زیر حد سفارش");

    // --- پایگاه داده ---
    let conn = novin_core::db::open_in_memory().unwrap();
    for table in ["stocktake_sessions", "stocktake_lines", "bulk_operations"] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "جدول {table} ساخته نشده است");
    }

    // آستانه‌ی کم‌موجودی باید در تنظیمات قابل تغییر باشد
    let threshold: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='inventory.low_stock_threshold'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(threshold, "5");

    // حساب‌های کسری و اضافی انبار باید در کدینگ باشند
    let variance_accounts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE code IN ('6300','4300')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(variance_accounts, 2, "حساب‌های تعدیل انبار تعریف نشده‌اند");

    // وضعیت نامعتبر دوره باید توسط خود پایگاه داده رد شود
    let invalid = conn.execute(
        "INSERT INTO stocktake_sessions(id,company_id,warehouse_id,title,count_date,status,created_by) \
         SELECT 'st-bad','company-demo',w.id,'تست','1405/01/01','unknown','user-demo' \
         FROM warehouses w LIMIT 1",
        [],
    );
    assert!(invalid.is_err(), "CHECK وضعیت دوره کار نمی‌کند");

    // مقدار شمارش منفی هم باید در سطح پایگاه داده رد شود
    conn.execute(
        "INSERT INTO stocktake_sessions(id,company_id,warehouse_id,title,count_date,created_by) \
         SELECT 'st-1','company-demo',w.id,'انبارگردانی نمونه','1405/05/01','user-demo' \
         FROM warehouses w LIMIT 1",
        [],
    )
    .unwrap();
    let negative = conn.execute(
        "INSERT INTO stocktake_lines(id,session_id,product_id,frozen_quantity,counted_quantity) \
         SELECT 'stl-1','st-1',p.id,10,-5 FROM products p LIMIT 1",
        [],
    );
    assert!(negative.is_err(), "CHECK مقدار شمارش کار نمی‌کند");
}
