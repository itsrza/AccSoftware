#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! # تست‌های سخت‌گیرانه‌ی فاز ۰ — یکپارچگی مالی و پایگاه داده
//!
//! این مجموعه شامل ۱۰ تست است که هرکدام یک **ادعای غیرقابل مذاکره** درباره‌ی
//! صحت نرم‌افزار را می‌سنجد. اگر هرکدام قرمز شود، محصول قابل انتشار نیست.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | پول | هیچ ریالی در محاسبات گم یا خلق نمی‌شود |
//! | ۲ | پول | تخفیف سرجمع بدون خطای گردکردن پخش می‌شود |
//! | ۳ | تقویم | تبدیل شمسی ↔ میلادی روی ۲۰۰ سال بی‌نقص است |
//! | ۴ | حسابداری | سند نامتعادل هرگز پذیرفته نمی‌شود |
//! | ۵ | حسابداری | سند خودکار فاکتور فروش همیشه متعادل است |
//! | ۶ | حسابداری | سند برگشتی دقیقاً اثر سند اصلی را خنثی می‌کند |
//! | ۷ | انبار | FIFO بهای تمام‌شده را دقیق مصرف می‌کند |
//! | ۸ | انبار | میانگین متحرک با تغییر قیمت درست به‌روز می‌شود |
//! | ۹ | پایگاه داده | اسکیمای واقعی، مهاجرت و کلیدهای خارجی سالم‌اند |
//! | ۱۰ | داده‌ی نمونه | داده‌ی دمو از نظر حسابداری واقعاً متعادل است |

use novin_core::accounting::{
    build_reversal, calculate_invoice, sales_invoice_journal, validate_journal, AccountingError,
    InvoiceLineInput, JournalLine,
};
use novin_core::inventory::{
    can_issue, consume_fifo, fifo_layers, valuate, InventoryError, Layer, Movement, MovementKind,
    ValuationMethod,
};
use novin_core::jalali::{self, JalaliDate};
use novin_core::money::{Money, MoneyError};

// ---------------------------------------------------------------------------
// تست ۱ — پول: هیچ ریالی گم یا خلق نمی‌شود
// ---------------------------------------------------------------------------
#[test]
fn t01_money_is_exact_and_never_loses_rials() {
    // خطای کلاسیک ممیز شناور نباید رخ دهد.
    let a = Money::from_rials(1);
    let sum: Money = [a; 10].into_iter().sum();
    assert_eq!(sum, Money::from_rials(10));

    // تبدیل تومان ↔ ریال
    assert_eq!(Money::from_tomans(1_250_000).unwrap().rials(), 12_500_000);
    assert_eq!(Money::from_rials(12_500_005).tomans(), 1_250_000);

    // ضرب در تعداد کسری با گرد کردن نصف به سمت بالا
    assert_eq!(
        Money::from_rials(333).mul_quantity(1.5).unwrap(),
        Money::from_rials(500) // 499.5 -> 500
    );
    assert_eq!(
        Money::from_rials(-333).mul_quantity(1.5).unwrap(),
        Money::from_rials(-500)
    );

    // مالیات ۹٪ روی مبلغی که گرد نمی‌شود
    assert_eq!(
        Money::from_rials(1_234_567).percent_bp(900).unwrap(),
        Money::from_rials(111_111) // 111111.03
    );

    // ورودی کاربر با ارقام فارسی و جداکننده
    assert_eq!(
        Money::parse_rials("۱۲٬۵۰۰,۰۰۰").unwrap(),
        Money::from_rials(12_500_000)
    );
    assert_eq!(Money::parse_rials("abc"), Err(MoneyError::Invalid));
    assert_eq!(
        Money::from_rials(-12_500_000).format_grouped(),
        "-12,500,000"
    );

    // سرریز باید خطا بدهد، نه wrap شود
    assert_eq!(
        Money::from_rials(i64::MAX).checked_add(Money::from_rials(1)),
        Err(MoneyError::Overflow)
    );
}

// ---------------------------------------------------------------------------
// تست ۲ — پخش تخفیف بدون خطای گردکردن
// ---------------------------------------------------------------------------
#[test]
fn t02_allocation_preserves_every_rial() {
    // مبلغی که بر تعداد سهم‌ها بخش‌پذیر نیست
    let amount = Money::from_rials(100);
    let shares = amount.allocate(&[1, 1, 1]).unwrap();
    assert_eq!(shares.len(), 3);
    let total: Money = shares.iter().copied().sum();
    assert_eq!(total, amount, "جمع سهم‌ها باید دقیقاً برابر مبلغ اصلی باشد");
    assert_eq!(
        shares,
        vec![
            Money::from_rials(34),
            Money::from_rials(33),
            Money::from_rials(33)
        ]
    );

    // وزن‌های نامتوازن روی مبلغ بزرگ
    let big = Money::from_rials(1_000_000_007);
    let weights = [3, 5, 11, 2, 7];
    let shares = big.allocate(&weights).unwrap();
    let total: Money = shares.iter().copied().sum();
    assert_eq!(total, big);
    assert!(shares.iter().all(|s| !s.is_negative()));

    // مبلغ منفی (برگشت از فروش) هم باید کامل توزیع شود
    let negative = Money::from_rials(-100);
    let shares = negative.allocate(&[1, 1, 1]).unwrap();
    let total: Money = shares.iter().copied().sum();
    assert_eq!(total, negative);

    // وزن نامعتبر
    assert_eq!(
        Money::from_rials(10).allocate(&[]),
        Err(MoneyError::Invalid)
    );
    assert_eq!(
        Money::from_rials(10).allocate(&[0, 0]),
        Err(MoneyError::Invalid)
    );
}

// ---------------------------------------------------------------------------
// تست ۳ — تقویم شمسی: صحت روی ۲۰۰ سال
// ---------------------------------------------------------------------------
#[test]
fn t03_jalali_calendar_is_exact_over_two_centuries() {
    // نقاط مرجع تاریخی
    let anchors = [
        ((1404, 5, 30), (2025, 8, 21)),
        ((1405, 1, 1), (2026, 3, 21)),
        ((1403, 1, 1), (2024, 3, 20)),
        ((1357, 11, 22), (1979, 2, 11)), // ۲۲ بهمن ۱۳۵۷
        ((1300, 1, 1), (1921, 3, 21)),
    ];
    for ((jy, jm, jd), (gy, gm, gd)) in anchors {
        let jalali = JalaliDate::new(jy, jm, jd).unwrap();
        let gregorian = chrono::NaiveDate::from_ymd_opt(gy, gm, gd).unwrap();
        assert_eq!(jalali.to_gregorian().unwrap(), gregorian, "شمسی → میلادی");
        assert_eq!(jalali::from_gregorian(gregorian), jalali, "میلادی → شمسی");
    }

    // رفت و برگشت روی هر روز از ۱۳۰۰ تا ۱۵۰۰
    let start = JalaliDate::new(1300, 1, 1).unwrap().to_gregorian().unwrap();
    let end = JalaliDate::new(1500, 1, 1).unwrap().to_gregorian().unwrap();
    let mut cursor = start;
    let mut checked = 0u32;
    while cursor < end {
        let jalali = jalali::from_gregorian(cursor);
        assert!(
            jalali.is_valid(),
            "تاریخ شمسی تولیدشده باید معتبر باشد: {jalali:?}"
        );
        assert_eq!(jalali.to_gregorian().unwrap(), cursor, "رفت‌وبرگشت ناسازگار");
        cursor = cursor.succ_opt().expect("تاریخ بعدی باید معتبر باشد");
        checked += 1;
    }
    assert!(checked > 73_000, "پوشش تست کافی نیست: {checked}");

    // کبیسه و اعتبارسنجی
    assert!(jalali::is_jalali_leap(1403));
    assert!(!jalali::is_jalali_leap(1404));
    assert_eq!(jalali::days_in_jalali_month(1403, 12), 30);
    assert_eq!(jalali::days_in_jalali_month(1404, 12), 29);
    assert!(
        JalaliDate::new(1404, 12, 30).is_err(),
        "۳۰ اسفند سال غیرکبیسه"
    );
    assert!(JalaliDate::new(1403, 12, 30).is_ok());
    assert!(JalaliDate::new(1404, 13, 1).is_err());
    assert!(JalaliDate::new(1404, 7, 31).is_err(), "مهر ۳۰ روز است");

    // خواندن ورودی فارسی
    assert_eq!(
        JalaliDate::parse("۱۴۰۴/۰۵/۳۰").unwrap(),
        JalaliDate::new(1404, 5, 30).unwrap()
    );
    assert_eq!(
        JalaliDate::parse("1404-05-30").unwrap().format(),
        "1404/05/30"
    );
}

// ---------------------------------------------------------------------------
// تست ۴ — سند نامتعادل هرگز پذیرفته نمی‌شود
// ---------------------------------------------------------------------------
#[test]
fn t04_unbalanced_journal_is_always_rejected() {
    let balanced = vec![
        JournalLine::debit("acc-1101", Money::from_rials(5_000_000)),
        JournalLine::credit("acc-4101", Money::from_rials(3_000_000)),
        JournalLine::credit("acc-4102", Money::from_rials(2_000_000)),
    ];
    let totals = validate_journal(&balanced).unwrap();
    assert_eq!(totals.total_debit, totals.total_credit);

    // اختلاف حتی یک ریال هم مردود است
    let off_by_one_rial = vec![
        JournalLine::debit("acc-1101", Money::from_rials(5_000_000)),
        JournalLine::credit("acc-4101", Money::from_rials(4_999_999)),
    ];
    assert_eq!(
        validate_journal(&off_by_one_rial),
        Err(AccountingError::Unbalanced { difference: 1 })
    );

    // سایر حالت‌های مردود
    assert_eq!(validate_journal(&[]), Err(AccountingError::EmptyJournal));
    assert_eq!(
        validate_journal(&[JournalLine::debit("acc-1", Money::from_rials(10))]),
        Err(AccountingError::SingleLine)
    );
    assert_eq!(
        validate_journal(&[
            JournalLine {
                account_id: "acc-1".into(),
                subsidiary_id: None,
                cost_center_id: None,
                project_id: None,
                debit: Money::from_rials(10),
                credit: Money::from_rials(10),
                description: None,
            },
            JournalLine::credit("acc-2", Money::from_rials(10)),
        ]),
        Err(AccountingError::BothSidesOnLine)
    );
    assert_eq!(
        validate_journal(&[
            JournalLine::debit("acc-1", Money::ZERO),
            JournalLine::credit("acc-2", Money::ZERO),
        ]),
        Err(AccountingError::ZeroLine)
    );
    assert_eq!(
        validate_journal(&[
            JournalLine::debit("acc-1", Money::from_rials(-10)),
            JournalLine::credit("acc-2", Money::from_rials(-10)),
        ]),
        Err(AccountingError::NegativeAmount)
    );
    assert_eq!(
        validate_journal(&[
            JournalLine::debit("   ", Money::from_rials(10)),
            JournalLine::credit("acc-2", Money::from_rials(10)),
        ]),
        Err(AccountingError::MissingAccount)
    );
}

// ---------------------------------------------------------------------------
// تست ۵ — سند خودکار فاکتور فروش همیشه متعادل است
// ---------------------------------------------------------------------------
#[test]
fn t05_sales_invoice_journal_is_always_balanced() {
    // فاکتور با اعداد «بدخیم»: تعداد کسری، قیمت فرد، تخفیف سطری و سرجمع، مالیات ۹٪
    let lines = vec![
        InvoiceLineInput {
            product_id: "p-1".into(),
            quantity: 2.5,
            unit_price: Money::from_rials(333_333),
            line_discount: Money::from_rials(11_111),
            tax_basis_points: 900,
        },
        InvoiceLineInput {
            product_id: "p-2".into(),
            quantity: 3.0,
            unit_price: Money::from_rials(777_777),
            line_discount: Money::ZERO,
            tax_basis_points: 900,
        },
        InvoiceLineInput {
            product_id: "p-3".into(),
            quantity: 1.0,
            unit_price: Money::from_rials(1_000_001),
            line_discount: Money::ZERO,
            tax_basis_points: 0, // کالای معاف از مالیات
        },
    ];
    let totals = calculate_invoice(&lines, Money::from_rials(97)).unwrap();

    // جمع سطرها باید دقیقاً برابر جمع فاکتور باشد
    let sum_gross: Money = totals.lines.iter().map(|l| l.gross).sum();
    let sum_discount: Money = totals.lines.iter().map(|l| l.discount).sum();
    let sum_tax: Money = totals.lines.iter().map(|l| l.tax).sum();
    let sum_total: Money = totals.lines.iter().map(|l| l.total).sum();
    assert_eq!(sum_gross, totals.subtotal);
    assert_eq!(sum_discount, totals.discount);
    assert_eq!(sum_tax, totals.tax);
    assert_eq!(sum_total, totals.total);

    // تخفیف سرجمع کامل اعمال شده باشد (۹۷ ریال + تخفیف سطری)
    assert_eq!(totals.discount, Money::from_rials(11_111 + 97));
    // کالای معاف نباید مالیات بگیرد
    assert_eq!(totals.lines[2].tax, Money::ZERO);
    // معادله‌ی بنیادی فاکتور
    assert_eq!(
        totals.total,
        totals.subtotal - totals.discount + totals.tax,
        "جمع کل = ناخالص − تخفیف + مالیات"
    );

    // سند خودکار باید متعادل باشد
    let journal = sales_invoice_journal("acc-1101", "acc-4101", "acc-2401", &totals).unwrap();
    let journal_totals = validate_journal(&journal).unwrap();
    assert_eq!(journal_totals.total_debit, totals.total);
    assert_eq!(journal_totals.total_debit, journal_totals.total_credit);

    // ورودی‌های نامعتبر
    assert_eq!(
        calculate_invoice(
            &[InvoiceLineInput {
                product_id: "p".into(),
                quantity: 0.0,
                unit_price: Money::from_rials(1),
                line_discount: Money::ZERO,
                tax_basis_points: 0,
            }],
            Money::ZERO
        ),
        Err(AccountingError::InvalidQuantity)
    );
    assert_eq!(
        calculate_invoice(
            &[InvoiceLineInput {
                product_id: "p".into(),
                quantity: 1.0,
                unit_price: Money::from_rials(1_000),
                line_discount: Money::from_rials(2_000),
                tax_basis_points: 0,
            }],
            Money::ZERO
        ),
        Err(AccountingError::DiscountTooLarge)
    );
}

// ---------------------------------------------------------------------------
// تست ۶ — سند برگشتی اثر سند اصلی را دقیقاً خنثی می‌کند
// ---------------------------------------------------------------------------
#[test]
fn t06_reversal_exactly_neutralises_original() {
    let original = vec![
        JournalLine::debit("acc-1101", Money::from_rials(7_777_777)),
        JournalLine::credit("acc-4101", Money::from_rials(7_142_915)),
        JournalLine::credit("acc-2401", Money::from_rials(634_862)),
    ];
    let reversal = build_reversal(&original).unwrap();
    validate_journal(&reversal).unwrap();

    // اثر خالص هر حساب پس از ثبت هر دو سند باید صفر باشد
    let mut net: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for line in original.iter().chain(reversal.iter()) {
        *net.entry(line.account_id.clone()).or_default() +=
            line.debit.rials() - line.credit.rials();
    }
    assert!(
        net.values().all(|v| *v == 0),
        "اثر مالی سند برگشتی باید کاملاً خنثی باشد: {net:?}"
    );

    // سند نامتعادل اصلاً برگشت‌پذیر نیست
    assert!(build_reversal(&[
        JournalLine::debit("acc-1", Money::from_rials(10)),
        JournalLine::credit("acc-2", Money::from_rials(9)),
    ])
    .is_err());
}

// ---------------------------------------------------------------------------
// تست ۷ — FIFO بهای تمام‌شده را دقیق مصرف می‌کند
// ---------------------------------------------------------------------------
#[test]
fn t07_fifo_costing_is_precise() {
    // ۱۰ عدد به ۱۰۰۰ ریال، سپس ۱۰ عدد به ۲۰۰۰ ریال
    let mut layers = vec![
        Layer {
            quantity: 10.0,
            unit_cost: 1_000,
        },
        Layer {
            quantity: 10.0,
            unit_cost: 2_000,
        },
    ];
    // خروج ۱۵ عدد: ۱۰×۱۰۰۰ + ۵×۲۰۰۰ = ۲۰٬۰۰۰
    assert_eq!(consume_fifo(&mut layers, 15.0).unwrap(), 20_000);
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].quantity, 5.0);
    assert_eq!(layers[0].unit_cost, 2_000);

    // برداشت بیش از موجودی مردود است
    assert_eq!(
        consume_fifo(&mut layers, 6.0),
        Err(InventoryError::InsufficientStock)
    );
    assert_eq!(
        consume_fifo(&mut layers, 0.0),
        Err(InventoryError::InvalidQuantity)
    );

    // ارزش‌گذاری از روی گردش‌ها
    let movements = vec![
        Movement::new(MovementKind::Receipt, 10.0, 1_000),
        Movement::new(MovementKind::Receipt, 10.0, 2_000),
        Movement::new(MovementKind::Issue, 15.0, 0),
    ];
    let valuation = valuate(&movements, ValuationMethod::Fifo).unwrap();
    assert_eq!(valuation.quantity, 5.0);
    assert_eq!(valuation.unit_cost, 2_000, "باقی‌مانده از لایه‌ی گران‌تر است");
    assert_eq!(valuation.total_value, 10_000);
    assert_eq!(fifo_layers(&movements).len(), 1);

    // انتقال بین انبار: خروج از مبدأ لایه‌ها را مصرف می‌کند
    let transfer = vec![
        Movement::new(MovementKind::Receipt, 5.0, 500),
        Movement::new(MovementKind::TransferOut, 5.0, 0),
    ];
    let valuation = valuate(&transfer, ValuationMethod::Fifo).unwrap();
    assert_eq!(valuation.quantity, 0.0);
    assert_eq!(valuation.total_value, 0);

    // کنترل موجودی رزروشده
    assert!(can_issue(10.0, 4.0, 6.0).is_ok());
    assert_eq!(
        can_issue(10.0, 4.0, 7.0),
        Err(InventoryError::InsufficientStock),
        "موجودی رزروشده نباید قابل برداشت باشد"
    );
}

// ---------------------------------------------------------------------------
// تست ۸ — میانگین متحرک با تغییر قیمت
// ---------------------------------------------------------------------------
#[test]
fn t08_moving_average_tracks_price_changes() {
    // ۱۰×۱۰۰۰ سپس ۱۰×۲۰۰۰ → میانگین ۱۵۰۰
    let movements = vec![
        Movement::new(MovementKind::Receipt, 10.0, 1_000),
        Movement::new(MovementKind::Receipt, 10.0, 2_000),
    ];
    let valuation = valuate(&movements, ValuationMethod::MovingAverage).unwrap();
    assert_eq!(valuation.unit_cost, 1_500);
    assert_eq!(valuation.quantity, 20.0);
    assert_eq!(valuation.total_value, 30_000);

    // خروج نباید میانگین را تغییر دهد
    let mut with_issue = movements.clone();
    with_issue.push(Movement::new(MovementKind::Issue, 12.0, 0));
    let valuation = valuate(&with_issue, ValuationMethod::MovingAverage).unwrap();
    assert_eq!(
        valuation.unit_cost, 1_500,
        "خروج نباید بهای میانگین را جابه‌جا کند"
    );
    assert_eq!(valuation.quantity, 8.0);
    assert_eq!(valuation.total_value, 12_000);

    // ورود بعدی با قیمت جدید، میانگین را وزنی به‌روز می‌کند: (8×1500 + 2×5000)/10
    let mut with_new_receipt = with_issue.clone();
    with_new_receipt.push(Movement::new(MovementKind::Receipt, 2.0, 5_000));
    let valuation = valuate(&with_new_receipt, ValuationMethod::MovingAverage).unwrap();
    assert_eq!(valuation.unit_cost, 2_200);
    assert_eq!(valuation.quantity, 10.0);

    // انبار خالی نباید بهای واحد منفی یا NaN بدهد
    let empty = valuate(&[], ValuationMethod::MovingAverage).unwrap();
    assert_eq!(empty.quantity, 0.0);
    assert_eq!(empty.unit_cost, 0);

    // روش نامعتبر
    assert_eq!(
        ValuationMethod::parse("lifo"),
        Err(InventoryError::InvalidMethod),
        "LIFO در استاندارد حسابداری ایران مجاز نیست"
    );
    // تعداد منفی در گردش انبار
    assert_eq!(
        valuate(
            &[Movement::new(MovementKind::Receipt, -1.0, 100)],
            ValuationMethod::Fifo
        ),
        Err(InventoryError::InvalidQuantity)
    );
}

// ---------------------------------------------------------------------------
// تست ۹ — اسکیمای واقعی پایگاه داده
// ---------------------------------------------------------------------------
#[test]
fn t09_database_schema_is_sound() {
    let conn = novin_core::db::open_in_memory().expect("پایگاه داده باید ساخته شود");

    // جدول‌های حیاتی موجود باشند
    let required = [
        "companies",
        "fiscal_years",
        "users",
        "roles",
        "permissions",
        "accounts",
        "journal_entries",
        "journal_lines",
        "contacts",
        "products",
        "warehouses",
        "inventory_balances",
        "inventory_movements",
        "sales_invoices",
        "purchase_invoices",
        "treasury_accounts",
        "checks",
        "audit_logs",
    ];
    for table in required {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "جدول ضروری وجود ندارد: {table}");
    }

    // یکپارچگی و کلیدهای خارجی
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let mut stmt = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let violations = stmt.query_map([], |_| Ok(())).unwrap().count();
    assert_eq!(violations, 0, "نقض کلید خارجی در داده‌ی اولیه");

    // مهاجرت باید idempotent باشد (اجرای دوباره نباید خطا بدهد یا داده را دوبرابر کند)
    let accounts_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
        .unwrap();
    novin_core::db::migrate(&conn).expect("اجرای دوباره‌ی مهاجرت باید بی‌خطر باشد");
    let accounts_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        accounts_before, accounts_after,
        "مهاجرت باید idempotent باشد"
    );
    assert!(accounts_before > 0, "کدینگ حساب‌ها باید مقداردهی اولیه شود");

    // رمز کاربر باید با Argon2 هش شده باشد، نه متن ساده
    let hash: String = conn
        .query_row(
            "SELECT password_hash FROM users WHERE username='admin'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        hash.starts_with("$argon2"),
        "رمز عبور باید با Argon2 هش شود"
    );
    assert!(
        !hash.contains("demo"),
        "رمز نباید به‌صورت متن ساده ذخیره شود"
    );

    // نقش مدیر باید مجوزها را داشته باشد و مجوزها granular باشند
    let permission_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM permissions", [], |r| r.get(0))
        .unwrap();
    assert!(permission_count >= 20, "مجوزها باید ریزدانه باشند");
    let admin_permissions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM role_permissions WHERE role_id='role-admin'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(admin_permissions, permission_count);
}

// ---------------------------------------------------------------------------
// تست ۱۰ — داده‌ی نمونه از نظر حسابداری واقعاً متعادل است
// ---------------------------------------------------------------------------
#[test]
fn t10_demo_data_is_accounting_consistent() {
    let conn = novin_core::db::open_in_memory().unwrap();

    // هر سند دمو باید متعادل باشد
    let mut stmt = conn
        .prepare(
            "SELECT j.id, COALESCE(SUM(l.debit),0), COALESCE(SUM(l.credit),0), COUNT(l.id) \
             FROM journal_entries j LEFT JOIN journal_lines l ON l.journal_id=j.id GROUP BY j.id",
        )
        .unwrap();
    let rows: Vec<(String, f64, f64, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "داده‌ی نمونه باید شامل سند حسابداری باشد");
    for (id, debit, credit, line_count) in &rows {
        assert!(*line_count >= 2, "سند {id} باید حداقل دو سطر داشته باشد");
        assert_eq!(debit, credit, "سند نمونه‌ی نامتعادل: {id}");
    }

    // تراز کل سیستم: مجموع بدهکار = مجموع بستانکار
    let (total_debit, total_credit): (f64, f64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        total_debit, total_credit,
        "تراز آزمایشی کل باید متوازن باشد"
    );
    assert!(total_debit > 0.0, "داده‌ی نمونه نباید خالی باشد");

    // هر سطر سند باید به حساب موجود ارجاع دهد (نه حساب یتیم)
    let orphan_lines: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM journal_lines l \
             LEFT JOIN accounts a ON a.id=l.account_id WHERE a.id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orphan_lines, 0, "سطر سند با حساب ناموجود");

    // داده‌ی نمونه باید واقعاً به هم متصل باشد (نه UI ساختگی)
    for (table, minimum) in [
        ("contacts", 1i64),
        ("products", 1),
        ("warehouses", 1),
        ("treasury_accounts", 1),
        ("checks", 1),
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert!(count >= minimum, "داده‌ی نمونه برای {table} وجود ندارد");
    }

    // چک نمونه باید وضعیت معتبر و سررسید پس از تاریخ صدور داشته باشد
    let mut stmt = conn
        .prepare("SELECT status, issue_date, due_date FROM checks")
        .unwrap();
    let checks: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    for (status, issue_date, due_date) in checks {
        assert!(
            novin_core::checks::CheckStatus::parse(&status).is_some(),
            "وضعیت چک نامعتبر: {status}"
        );
        let issued = JalaliDate::parse(&issue_date).expect("تاریخ صدور چک باید شمسی معتبر باشد");
        let due = JalaliDate::parse(&due_date).expect("سررسید چک باید شمسی معتبر باشد");
        assert!(due >= issued, "سررسید چک نباید قبل از تاریخ صدور باشد");
    }
}
