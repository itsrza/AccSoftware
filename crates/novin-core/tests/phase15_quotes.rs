#![allow(warnings)]
// موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! فاز ۱۵ — پیش‌فاکتور فروش و سفارش خرید.
//!
//! ## قاعده‌ی محوری: تعهد، رویداد مالی نیست
//!
//! پیش‌فاکتور و سفارش خرید **هیچ اثر مالی و انباری ندارند**:
//!
//! - سند حسابداری نمی‌سازند (درآمدی محقق نشده، هزینه‌ای انجام نشده)
//! - موجودی انبار را تغییر نمی‌دهند (کالایی جابه‌جا نشده)
//! - در تراز و صورت سود و زیان دیده نمی‌شوند
//!
//! اگر پیش‌فاکتور سند بزند، **درآمد تحقق‌نیافته** در صورت‌های مالی ظاهر
//! می‌شود — یکی از رایج‌ترین و پرهزینه‌ترین اشتباهات نرم‌افزارهای حسابداری،
//! چون مالیات بر درآمدی محاسبه می‌شود که هنوز وجود ندارد.
//!
//! اثر مالی فقط در لحظه‌ی **تبدیل به فاکتور** متولد می‌شود، و فاکتور حاصل
//! هم **پیش‌نویس** است نه ثبت‌شده — چون بین پیشنهاد و فروش ممکن است قیمت یا
//! موجودی تغییر کرده باشد.

use novin_core::db;
use novin_core::jalali::JalaliDate;
use rusqlite::Connection;

fn fresh() -> Connection {
    db::open_in_memory().expect("پایگاه داده باید ساخته شود")
}

fn seeded() -> Connection {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه باید ساخته شود");
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

/// ت۰۱ — جدول‌ها با قیدهای درست ساخته شده‌اند.
#[test]
fn t01_quote_schema_rejects_invalid_data() {
    let conn = fresh();
    for table in ["quotes", "quote_lines"] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "جدول «{table}» نیست");
    }

    // نوع ساختگی
    assert!(
        conn.execute(
            "INSERT INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date) \
             VALUES('t01a','company-demo','fy-demo','wish_list',1,'1405/01/01')",
            [],
        )
        .is_err(),
        "نوع ناشناخته باید رد شود"
    );
    // وضعیت ساختگی
    assert!(
        conn.execute(
            "INSERT INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date,status) \
             VALUES('t01b','company-demo','fy-demo','sales_quote',1,'1405/01/01','maybe')",
            [],
        )
        .is_err(),
        "وضعیت ناشناخته باید رد شود"
    );
    // مبلغ منفی
    assert!(
        conn.execute(
            "INSERT INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date,total) \
             VALUES('t01c','company-demo','fy-demo','sales_quote',1,'1405/01/01',-1)",
            [],
        )
        .is_err(),
        "مبلغ منفی باید رد شود"
    );
}

/// ت۰۲ — پیش‌فاکتور هیچ سند حسابداری نمی‌سازد.
///
/// این مهم‌ترین قاعده‌ی این فاز است.
#[test]
fn t02_quotes_never_create_journal_entries() {
    let conn = seeded();
    assert!(
        count(&conn, "SELECT COUNT(*) FROM quotes") >= 10,
        "پیش‌فاکتور نمونه ساخته نشده"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_entries \
             WHERE source_type IN ('sales_quote','purchase_order','quote')"
        ),
        0,
        "پیش‌فاکتور سند حسابداری ساخته است — درآمد تحقق‌نیافته"
    );
}

/// ت۰۳ — پیش‌فاکتور موجودی انبار را تغییر نمی‌دهد.
#[test]
fn t03_quotes_never_touch_inventory() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements \
             WHERE reference_type IN ('sales_quote','purchase_order','quote')"
        ),
        0,
        "پیش‌فاکتور گردش انبار ساخته است"
    );
    // و هیچ گردشی نباید به شناسه‌ی یک پیش‌فاکتور ارجاع بدهد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements m \
             WHERE EXISTS (SELECT 1 FROM quotes q WHERE q.id = m.reference_id)"
        ),
        0,
        "گردش انباری به پیش‌فاکتور ارجاع دارد"
    );
}

/// ت۰۴ — مالیات روی مبلغ **پس از تخفیف** محاسبه می‌شود.
///
/// محاسبه‌ی مالیات روی مبلغ ناخالص، مالیات اضافی می‌سازد و مبلغ فاکتور را
/// بالاتر از واقع نشان می‌دهد.
#[test]
fn t04_tax_is_calculated_after_discount() {
    let conn = seeded();
    let mut statement = conn
        .prepare("SELECT id,subtotal,discount,tax,total FROM quotes")
        .unwrap();
    let rows: Vec<(String, i64, i64, i64, i64)> = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "پیش‌فاکتوری وجود ندارد");

    for (id, subtotal, discount, tax, total) in rows {
        let net = subtotal - discount;
        let expected_tax = net * 900 / 10_000;
        assert_eq!(tax, expected_tax, "مالیات «{id}» روی مبلغ خالص نیست");
        assert_eq!(total, net + tax, "جمع کل «{id}» نمی‌خواند");
        // اگر مالیات روی ناخالص محاسبه می‌شد، برای اسناد دارای تخفیف بیشتر می‌شد.
        if discount > 0 {
            assert!(
                tax < subtotal * 900 / 10_000,
                "مالیات «{id}» روی مبلغ ناخالص محاسبه شده"
            );
        }
    }
}

/// ت۰۵ — جمع هدر با جمع اقلام می‌خواند.
#[test]
fn t05_header_totals_match_line_totals() {
    let conn = seeded();
    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM quotes q WHERE q.total <> COALESCE(\
           (SELECT SUM(l.line_total) FROM quote_lines l WHERE l.quote_id=q.id),0)",
    );
    assert_eq!(mismatched, 0, "جمع هدر با اقلام نمی‌خواند");

    // و هیچ پیش‌فاکتوری بدون قلم نمانده باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM quotes q WHERE NOT EXISTS \
             (SELECT 1 FROM quote_lines l WHERE l.quote_id=q.id)"
        ),
        0,
        "پیش‌فاکتور بدون قلم وجود دارد"
    );
}

/// ت۰۶ — تخفیف سطر هرگز از مبلغ همان سطر بیشتر نیست.
#[test]
fn t06_line_discount_never_exceeds_line_amount() {
    let conn = seeded();
    let violations = count(
        &conn,
        "SELECT COUNT(*) FROM quote_lines \
         WHERE discount > ROUND(quantity * unit_price) OR discount < 0",
    );
    assert_eq!(violations, 0, "تخفیف سطر بیشتر از مبلغ سطر است");
}

/// ت۰۷ — تاریخ اعتبار هرگز قبل از تاریخ صدور نیست.
#[test]
fn t07_validity_never_precedes_issue_date() {
    let conn = seeded();
    let mut statement = conn
        .prepare("SELECT id,issue_date,valid_until FROM quotes WHERE valid_until IS NOT NULL")
        .unwrap();
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "سند دارای تاریخ اعتبار وجود ندارد");
    for (id, issue, valid) in rows {
        let issued = JalaliDate::parse(&issue).expect("تاریخ صدور شمسی");
        let until = JalaliDate::parse(&valid).expect("تاریخ اعتبار شمسی");
        assert!(
            until >= issued,
            "اعتبار «{id}» ({valid}) قبل از صدور ({issue}) است"
        );
    }
}

/// ت۰۸ — شماره در هر دفتر (نوع سند) یکتاست.
///
/// پیش‌فاکتور و سفارش خرید دفتر شماره‌گذاری جدا دارند؛ شماره‌ی یکسان در دو
/// نوع مختلف مجاز است ولی در یک نوع نه.
#[test]
fn t08_numbering_is_unique_per_document_kind() {
    let conn = seeded();
    let total = count(&conn, "SELECT COUNT(*) FROM quotes");
    let distinct = count(
        &conn,
        "SELECT COUNT(DISTINCT company_id||'/'||fiscal_year_id||'/'||kind||'/'||number) FROM quotes",
    );
    assert_eq!(total, distinct, "شماره‌ی تکراری در یک دفتر وجود دارد");

    // هر دو نوع باید نمونه داشته باشند.
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM quotes WHERE kind='sales_quote'"
        ) > 0,
        "پیش‌فاکتور فروش نمونه ندارد"
    );
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM quotes WHERE kind='purchase_order'"
        ) > 0,
        "سفارش خرید نمونه ندارد"
    );
}

/// ت۰۹ — فقط سند «تبدیل‌شده» شناسه‌ی فاکتور دارد.
///
/// شناسه‌ی فاکتور روی سندی که تبدیل نشده، یعنی یک سفارش دو بار فاکتور شده.
#[test]
fn t09_only_converted_quotes_carry_an_invoice_reference() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM quotes WHERE converted_invoice_id IS NOT NULL \
             AND status <> 'converted'"
        ),
        0,
        "سند تبدیل‌نشده شناسه‌ی فاکتور دارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM quotes WHERE status='converted' \
             AND converted_invoice_id IS NULL"
        ),
        0,
        "سند تبدیل‌شده بدون شناسه‌ی فاکتور است"
    );
    // یک فاکتور نباید حاصل دو پیش‌فاکتور باشد.
    let referenced = count(
        &conn,
        "SELECT COUNT(*) FROM quotes WHERE converted_invoice_id IS NOT NULL",
    );
    let unique = count(
        &conn,
        "SELECT COUNT(DISTINCT converted_invoice_id) FROM quotes \
         WHERE converted_invoice_id IS NOT NULL",
    );
    assert_eq!(referenced, unique, "دو پیش‌فاکتور به یک فاکتور اشاره دارند");
}

/// ت۱۰ — داده‌ی نمونه از نظر ارجاعی سالم است و تکرار اجرا آن را خراب نمی‌کند.
#[test]
fn t10_quote_dataset_is_intact_and_idempotent() {
    let conn = seeded();
    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    assert_eq!(
        statement.query_map([], |_| Ok(())).unwrap().count(),
        0,
        "کلید خارجی شکسته است"
    );
    drop(statement);

    let before = count(&conn, "SELECT COUNT(*) FROM quote_lines");
    db::demo::seed_demo_dataset(&conn).unwrap();
    assert_eq!(
        before,
        count(&conn, "SELECT COUNT(*) FROM quote_lines"),
        "اجرای دوباره اقلام را دوبرابر کرد"
    );

    // پس از همه‌ی این‌ها، دفتر کل باید همچنان متوازن باشد.
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(debit, credit, "دفتر کل نامتوازن شد");
}
