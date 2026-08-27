//! ممیزی دور ۱۳ — عینک امنیت: گارد نام جدول در SQL داینامیک.
//!
//! ## زمینه
//!
//! میزبان در نقاط ثبت/پست فاکتور، برگشت و گزارش‌های مانده، نام جدول را با
//! `format!` داخل SQL می‌نشیند. مقادیر فعلی همه ثابت‌اند (`if sale {...}`)،
//! اما بدون گارد، اولین فراخوانی آینده با ورودی کاربر یعنی تزریق کامل SQL.
//! `db::validated_table` فقط ۸ نام مجاز اسکیمای سند را قبول می‌کند.

use novin_core::db;

/// ا۱ — هر هشت جدول مجاز سند، عیناً برگردانده می‌شوند.
#[test]
fn s01_all_document_tables_accepted() {
    for name in [
        "sales_invoices",
        "purchase_invoices",
        "sales_invoice_lines",
        "purchase_invoice_lines",
        "sales_returns",
        "purchase_returns",
        "sales_return_lines",
        "purchase_return_lines",
    ] {
        assert_eq!(db::validated_table(name), Ok(name), "جدول {name} مجاز است");
    }
}

/// ا۲ — رشته‌های تزریق کلاسیک رد می‌شوند.
#[test]
fn s02_injection_strings_rejected() {
    let attacks = [
        "sales_invoices; DROP TABLE products--",
        "sales_invoices WHERE 1=1",
        "sales_invoices'",
        "users",
        "user_accounts",
        "",
        " sales_invoices",
        "sales_invoices ",
        "SALES_INVOICES",
        "sales_invoices;--",
        "$(rm -rf)",
        "sales_invoices\u{0000}",
    ];
    for attack in attacks {
        let result = db::validated_table(attack);
        assert!(
            result.is_err(),
            "رشته‌ی «{attack:?}» باید رد شود، نه این‌که {result:?} بدهد"
        );
        let message = result.unwrap_err();
        assert!(
            message.contains("SQLGUARD-001"),
            "خطا باید کد پیگیری داشته باشد: {message}"
        );
    }
}

/// ا۳ — خروجی از نوع مرجع ایستاست و کل فهرست allowlist دقیقاً ۸ عضو دارد.
#[test]
fn s03_allowlist_is_exact() {
    let accepted: Vec<&str> = [
        "sales_invoices",
        "purchase_invoice_lines",
        "sales_return_lines",
    ]
    .into_iter()
    .filter_map(|name| db::validated_table(name).ok())
    .collect();
    assert_eq!(accepted.len(), 3);
    // جدول‌های حساس خارج از فهرست — هرگز نباید قبول شوند
    for forbidden in ["users", "company_users", "audit_logs", "journal_entries"] {
        assert!(
            db::validated_table(forbidden).is_err(),
            "جدول حساس {forbidden} نباید در فهرست سند باشد"
        );
    }
}

/// ا۴ — گزارش سن بدهکاران از ایندکس تسویه استفاده می‌کند (EXPLAIN QUERY PLAN).
#[test]
fn s04_settlement_aging_index_used() {
    let conn = db::open_in_memory().expect("پایگاه داده");
    let plan: String = conn
        .query_row(
            "EXPLAIN QUERY PLAN SELECT invoice_id, SUM(amount) FROM invoice_settlements \
             WHERE company_id=?1 AND invoice_type=?2 AND settlement_date<=?3 GROUP BY invoice_id",
            ["company-demo", "sales", "2026-01-01"],
            |row| row.get(3),
        )
        .expect("برنامه‌ی اجرا");
    assert!(
        plan.contains("idx_invoice_settlements_company_type_date"),
        "کوئری سن باید از ایندکس بگذرد، نه اسکن کامل: {plan}"
    );
}
