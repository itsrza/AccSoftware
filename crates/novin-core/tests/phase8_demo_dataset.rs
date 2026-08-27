#![allow(warnings)] // موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! # تست داده‌ی نمونه‌ی گسترده
//!
//! داده‌ی نمونه اختیاری است و شکست آن نباید مهاجرت را بشکند؛ ولی باید بدانیم
//! اگر ساخته نشد، **دقیقاً کجا** شکست خورده است. این تست همان را آشکار می‌کند.

use novin_core::db;

#[test]
fn demo_dataset_builds_completely() {
    let conn = db::open_in_memory().expect("پایگاه داده باید ساخته شود");

    // اجرای مستقیم تا در صورت خطا، پیام مرحله‌ی شکست‌خورده دیده شود.
    db::demo::seed_demo_dataset(&conn).expect("ساخت داده‌ی نمونه باید موفق باشد");

    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or(-1)
    };

    assert!(count("products") >= 60, "کالاها: {}", count("products"));
    assert!(count("contacts") >= 50, "اشخاص: {}", count("contacts"));
    assert!(count("warehouses") >= 5, "انبارها: {}", count("warehouses"));
    assert!(
        count("sales_invoices") >= 55,
        "فاکتور فروش: {}",
        count("sales_invoices")
    );
    assert!(
        count("purchase_invoices") >= 25,
        "فاکتور خرید: {}",
        count("purchase_invoices")
    );
    assert!(count("checks") >= 20, "چک‌ها: {}", count("checks"));
    assert!(
        count("treasury_accounts") >= 5,
        "حساب‌های خزانه: {}",
        count("treasury_accounts")
    );

    // یکپارچگی حسابداری داده‌ی نمونه
    let (debit, credit): (f64, f64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(debit, credit, "تراز کل داده‌ی نمونه باید متوازن باشد");

    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let violations = statement.query_map([], |_| Ok(())).unwrap().count();
    assert_eq!(violations, 0, "داده‌ی نمونه نباید کلید خارجی بشکند");

    // اجرای دوباره نباید داده را دوبرابر کند
    let before = count("products");
    db::demo::seed_demo_dataset(&conn).unwrap();
    assert_eq!(
        before,
        count("products"),
        "ساخت داده‌ی نمونه باید idempotent باشد"
    );
}
