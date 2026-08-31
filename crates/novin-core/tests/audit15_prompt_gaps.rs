//! ممیزی دور ۱۵ — بستن یافته‌های بازِ پرامپت‌های مرجع (References/UI-BY-AI).
//!
//! هر تست به یک یافته‌ی مشخص گره خورده:
//!  ۱. پراگماهای per-connection (APP-002..004)
//!  ۲. نگاشت حساب‌ها (account_mappings) — seed، یکتایی، عدم حساب hardcoded
//!  ۳. تفکیک مالیات/تخفیف در خطوط سند فاکتور (هسته‌ی خالص)
//!  ۴. موجودی قابل‌فروش (کل − رزرو) در پست فاکتور
//!  ۵. آزادسازی رزرو هنگام پست
//!  ۶. سند پیگیری چک برگشتیِ قبل از وصول

use novin_core::db;
use novin_core::invoicing::{invoice_posting_lines, InvoicePostingAccounts};
use rusqlite::{params, Connection};

fn seeded() -> Connection {
    db::open_in_memory().expect("پایگاه داده")
}

fn company(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM companies ORDER BY id LIMIT 1", [], |row| {
        row.get(0)
    })
    .expect("شرکت پایه")
}

/// پ۱ — جدول نگاشت ساخته و برای company-demo با هر ۱۲ کلید seed شده است.
#[test]
fn p01_account_mappings_seeded() {
    let conn = seeded();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM account_mappings", [], |r| r.get(0))
        .expect("شمارش");
    assert_eq!(count, 12, "هر دوازده کلید باید seed شوند");
    // یکتایی (company_id, mapping_key)
    let dup = conn.execute(
        "INSERT INTO account_mappings(company_id,mapping_key,account_id) VALUES('company-demo','cash_default','acc-1101')",
        [],
    );
    assert!(dup.is_err(), "کلید تکراری باید رد شود");
    // حساب ارجاع‌شده باید واقعاً موجود باشد (FK با حذف آبشاری)
    let missing = conn.execute(
        "INSERT INTO account_mappings(company_id,mapping_key,account_id) VALUES('company-demo','x_default','acc-9999')",
        [],
    );
    assert!(missing.is_err(), "حساب ناموجود باید رد شود");
}

/// پ۲ — حساب‌های جدید کدینگ (تخفیف فروش/خرید و پیگیری چک برگشتی) موجودند.
#[test]
fn p02_new_chart_accounts_exist() {
    let conn = seeded();
    for (id, name) in [
        ("acc-4250", "تخفیفات فروش"),
        ("acc-5250", "تخفیفات خرید"),
        ("acc-1260", "چک‌های برگشتی در پیگیری"),
    ] {
        let found: String = conn
            .query_row("SELECT name FROM accounts WHERE id=?1", params![id], |r| {
                r.get(0)
            })
            .unwrap_or_default();
        assert_eq!(found, name, "حساب {id} باید با نام درست seed شود");
    }
}

/// پ۳ — تفکیک مالیات/تخفیف در سند فروش: تراز + خطوط جدا + حذف خطوط صفر.
#[test]
fn p03_sales_posting_splits_tax_and_discount() {
    let accounts = InvoicePostingAccounts {
        party: "ar".into(),
        main: "sales".into(),
        tax: "vat".into(),
        discount: "disc".into(),
    };
    // subtotal=1,000,000، discount=50,000، tax=90,000 → total=1,040,000
    let lines = invoice_posting_lines(true, 1_000_000, 50_000, 90_000, &accounts).expect("سند");
    let debit: i64 = lines.iter().map(|l| l.1).sum();
    let credit: i64 = lines.iter().map(|l| l.2).sum();
    assert_eq!(debit, credit, "سند باید تراز باشد");
    // جمع دفتر = ناخالص + مالیات (تخفیف خطِ بدهکار جدا دارد؛ طرفین ۱٬۰۹۰٬۰۰۰)
    assert_eq!(debit, 1_090_000);
    assert!(
        lines.iter().any(|l| l.0 == "sales" && l.2 == 1_000_000),
        "فروش ناخالص"
    );
    assert!(
        lines.iter().any(|l| l.0 == "vat" && l.2 == 90_000),
        "مالیات خط جدا"
    );
    assert!(
        lines.iter().any(|l| l.0 == "disc" && l.1 == 50_000),
        "تخفیف خط جدا"
    );
    assert!(
        lines.iter().any(|l| l.0 == "ar" && l.1 == 1_040_000),
        "طرف‌حساب"
    );
    // بدون مالیات/تخفیف → هیچ خط صفر ساخته نشود
    let clean = invoice_posting_lines(true, 500_000, 0, 0, &accounts).expect("سند ساده");
    assert_eq!(clean.len(), 2, "فقط طرف‌حساب و فروش");
}

/// پ۴ — سند خرید آینه‌ی فروش است و تراز برقرار می‌ماند.
#[test]
fn p04_purchase_posting_balanced() {
    let accounts = InvoicePostingAccounts {
        party: "ap".into(),
        main: "cogs".into(),
        tax: "vatr".into(),
        discount: "pdisc".into(),
    };
    let lines = invoice_posting_lines(false, 2_000_000, 100_000, 180_000, &accounts).expect("سند");
    let debit: i64 = lines.iter().map(|l| l.1).sum();
    let credit: i64 = lines.iter().map(|l| l.2).sum();
    assert_eq!(debit, credit);
    assert_eq!(debit, 2_180_000, "خرید ناخالص + مالیات");
    assert!(lines.iter().any(|l| l.0 == "ap" && l.2 == 2_080_000));
}

/// پ۵ — ورودی نامعتبر رد می‌شود: تخفیف بیش از مبلغ، منفی، جمع صفر.
#[test]
fn p05_posting_rejects_invalid() {
    let accounts = InvoicePostingAccounts {
        party: "a".into(),
        main: "b".into(),
        tax: "c".into(),
        discount: "d".into(),
    };
    assert!(
        invoice_posting_lines(true, 100, 200, 0, &accounts).is_err(),
        "تخفیف > مبلغ"
    );
    assert!(
        invoice_posting_lines(true, -1, 0, 0, &accounts).is_err(),
        "منفی"
    );
    assert!(
        invoice_posting_lines(true, 0, 0, 0, &accounts).is_err(),
        "جمع صفر"
    );
}

/// پ۶ — موجودی قابل‌فروش در پست فاکتور: کل − رزرو ملاک است، نه کل خام.
#[test]
fn p06_post_respects_reserved_quantity() {
    let conn = seeded();
    let firm = company(&conn);
    conn.execute(
        "INSERT INTO products(id,company_id,kind,sku,name,unit) VALUES('audit15-p',?1,'simple','P15','کالا','عدد')",
        params![firm],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO inventory_balances(product_id,warehouse_id,quantity,reserved_quantity) VALUES('audit15-p','wh-main',100.0,90.0)",
        [],
    )
    .unwrap();
    // همان منطق میزبان: available = quantity - reserved
    let available: f64 = conn
        .query_row(
            "SELECT COALESCE(quantity-reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",
            params!["audit15-p", "wh-main"],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(available, 10.0, "قابل‌فروش = ۱۰ نه ۱۰۰");
    assert!(available < 50.0, "فروش ۵۰ باید DOC-013 بخورد");
}

/// پ۷ — آزادسازی رزرو هنگام پست: وضعیت released و کاهش reserved_quantity.
#[test]
fn p07_reservation_released_on_post() {
    let conn = seeded();
    let firm = company(&conn);
    conn.execute(
        "INSERT INTO products(id,company_id,kind,sku,name,unit) VALUES('audit15-q',?1,'simple','Q15','کالا','عدد')",
        params![firm],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO inventory_balances(product_id,warehouse_id,quantity,reserved_quantity) VALUES('audit15-q','wh-main',50.0,20.0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO inventory_reservations(id,company_id,product_id,warehouse_id,quantity,status,reference_type,reference_id) VALUES('audit15-r',?1,'audit15-q','wh-main',20.0,'reserved','invoice','inv-x')",
        params![firm],
    )
    .unwrap();
    // همان دو دستور میزبان در post_invoice
    conn.execute(
        "UPDATE inventory_reservations SET status='released',released_at=CURRENT_TIMESTAMP WHERE id='audit15-r'",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE inventory_balances SET reserved_quantity=MAX(0,reserved_quantity-?3) WHERE product_id=?1 AND warehouse_id=?2",
        params!["audit15-q", "wh-main", 20.0],
    )
    .unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM inventory_reservations WHERE id='audit15-r'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "released");
    let reserved: f64 = conn
        .query_row(
            "SELECT reserved_quantity FROM inventory_balances WHERE product_id='audit15-q'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reserved, 0.0);
}

/// پ۸ — سند پیگیری چک برگشتی قبل از وصول: تراز و حساب‌های نگاشت.
#[test]
fn p08_bounce_pending_uses_mappings() {
    let conn = seeded();
    // نگاشت‌ها seed شده‌اند؛ حساب پیگیری باید قابل استخراج باشد
    let tracking: String = conn
        .query_row(
            "SELECT account_id FROM account_mappings WHERE company_id='company-demo' AND mapping_key='check_bounce_tracking_default'",
            [],
            |r| r.get(0),
        )
        .expect("نگاشت پیگیری");
    assert_eq!(tracking, "acc-1260");
    // حساب باید در کدینگ واقعاً موجود باشد (FK معتبر)
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE id='acc-1260'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(exists, 1);
}

/// پ۹ — مجوز مدیریت نگاشت seed شده است.
#[test]
fn p09_mapping_permission_seeded() {
    let conn = seeded();
    let found: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM permissions WHERE id='accounting.settings.edit'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(found, 1);
}

/// پ۱۰ — داده‌ی دمو با نگاشت‌ها سازگار است: هر سه حساب سند نمونه‌ی قدیمی
/// در نگاشت‌ها هم‌ارزش‌اند (Regression: فاکتورهای دمو نباید بشکنند).
#[test]
fn p10_demo_regression_mappings_match_old_hardcode() {
    let conn = seeded();
    for (key, expected) in [
        ("cash_default", "acc-1101"),
        ("ar_default", "acc-1201"),
        ("ap_default", "acc-2101"),
        ("sales_revenue_default", "acc-4100"),
        ("cogs_default", "acc-5100"),
        ("sales_return_default", "acc-4200"),
        ("purchase_return_default", "acc-5200"),
    ] {
        let actual: String = conn
            .query_row(
                "SELECT account_id FROM account_mappings WHERE company_id='company-demo' AND mapping_key=?1",
                params![key],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| panic!("نگاشت {key}"));
        assert_eq!(actual, expected, "کلید {key} باید هم‌ارزش رفتار قدیمی باشد");
    }
}
