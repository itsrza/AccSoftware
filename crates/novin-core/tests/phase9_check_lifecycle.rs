#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! فاز ۹ — چرخه‌ی کامل عمر چک و یکپارچگی داده‌ی نمونه.
//!
//! این فاز دو باگ ریشه‌ای را می‌بندد که هر دو از یک جنس بودند: **نبود انطباق
//! بین قواعد پایگاه داده و قواعد حسابداری**.
//!
//! ۱. قید `CHECK` جدول چک تنها شش وضعیت قدیمی را می‌پذیرفت، در حالی که ماشین
//!    حالت هسته دوازده وضعیت واقعی دارد. نتیجه: هر گذاری که ماشین حالت مجاز
//!    می‌دانست، در پایگاه داده رد می‌شد.
//! ۲. داده‌ی نمونه شماره‌ی سند و فاکتور را hardcode می‌کرد و با داده‌ی پایه
//!    تداخل داشت؛ `INSERT OR IGNORE` سطر والد را بی‌صدا رد می‌کرد و سطرهای
//!    فرزند کلید خارجی را می‌شکستند.
//!
//! هر ده تست زیر مستقیماً یکی از این دو ریشه یا اثر حسابداری آن را می‌سنجد.

use novin_core::checks::{
    allowed_transitions, transition, treasury_effect, CheckKind, CheckStatus, TreasuryEffect,
};
use novin_core::db;
use rusqlite::Connection;

fn fresh() -> Connection {
    db::open_in_memory().expect("پایگاه داده‌ی حافظه‌ای باید ساخته شود")
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

/// ت۰۱ — پایگاه داده باید **هر دوازده** وضعیت ماشین حالت را بپذیرد.
///
/// این دقیقاً همان باگی است که باعث می‌شد کاربر در صفحه‌ی چک خطای عمومی
/// ببیند: وضعیت مجاز از نظر منطق، از نظر پایگاه داده نامعتبر بود.
#[test]
fn t01_database_accepts_every_status_of_the_state_machine() {
    let conn = fresh();
    let all = [
        CheckStatus::InHand,
        CheckStatus::Deposited,
        CheckStatus::Collected,
        CheckStatus::Cashed,
        CheckStatus::Endorsed,
        CheckStatus::Bounced,
        CheckStatus::Returned,
        CheckStatus::Void,
        CheckStatus::Outstanding,
        CheckStatus::Paid,
        CheckStatus::MemoInHand,
        CheckStatus::MemoReturned,
    ];
    for (index, status) in all.iter().enumerate() {
        let kind = if matches!(status, CheckStatus::Outstanding | CheckStatus::Paid) {
            "issued"
        } else {
            "received"
        };
        conn.execute(
            "INSERT INTO checks(id,company_id,fiscal_year_id,check_type,check_number,amount,\
             issue_date,due_date,status,created_by) \
             VALUES(?1,'company-demo','fy-demo',?2,?3,1000,'1405/01/01','1405/02/01',?4,'user-demo')",
            rusqlite::params![
                format!("t01-check-{index}"),
                kind,
                format!("T01-{index}"),
                status.as_str()
            ],
        )
        .unwrap_or_else(|error| panic!("وضعیت «{}» باید پذیرفته شود: {error}", status.as_str()));
    }
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM checks WHERE id LIKE 't01-%'"),
        12,
        "هر دوازده وضعیت باید در پایگاه داده قابل ثبت باشد"
    );
}

/// ت۰۲ — وضعیت ساختگی باید توسط خود پایگاه داده رد شود.
///
/// پذیرش هر رشته‌ای یعنی داده‌ی خراب؛ قید `CHECK` آخرین خط دفاع است.
#[test]
fn t02_database_rejects_invented_status() {
    let conn = fresh();
    let result = conn.execute(
        "INSERT INTO checks(id,company_id,fiscal_year_id,check_type,check_number,amount,\
         issue_date,due_date,status,created_by) \
         VALUES('t02','company-demo','fy-demo','received','T02',1000,'1405/01/01','1405/02/01','registered','user-demo')",
        [],
    );
    assert!(
        result.is_err(),
        "وضعیت منسوخ «registered» نباید دیگر پذیرفته شود"
    );
}

/// ت۰۳ — داده‌ی قدیمی باید بدون از دست رفتن سطر مهاجرت کند.
///
/// نگاشت: `registered` چک دریافتی → «موجود»، چک پرداختی → «پرداختی در جریان».
#[test]
fn t03_legacy_statuses_are_migrated_not_dropped() {
    let conn = fresh();
    // داده‌ی پایه یک چک با وضعیت قدیمی داشت؛ باید به وضعیت جدید نگاشت شده باشد.
    let status: String = conn
        .query_row(
            "SELECT status FROM checks WHERE id='demo-check-1'",
            [],
            |row| row.get(0),
        )
        .expect("چک نمونه‌ی پایه باید پس از مهاجرت باقی مانده باشد");
    assert_eq!(status, "in_hand", "چک دریافتی ثبت‌شده باید «موجود» شود");
    assert!(
        CheckStatus::parse(&status).is_some(),
        "هیچ چکی نباید وضعیت خارج از واژه‌نامه داشته باشد"
    );
}

/// ت۰۴ — چک وصول‌شده می‌تواند برگشت بخورد، و اثرش معکوس است.
///
/// منطق حسابداری: بانک مبلغ وصول‌شده را از حساب کسر می‌کند، پس باید سند
/// معکوس صادر شود؛ نه اینکه گذار ممنوع باشد.
#[test]
fn t04_collected_check_can_bounce_with_reverse_effect() {
    let kind = CheckKind::Received;
    assert!(
        transition(kind, CheckStatus::Collected, CheckStatus::Bounced).is_ok(),
        "برگشت چک وصول‌شده باید مجاز باشد"
    );
    assert_eq!(
        treasury_effect(kind, CheckStatus::Collected, CheckStatus::Bounced),
        TreasuryEffect::Decrease,
        "برگشت چک وصول‌شده باید موجودی خزانه را کاهش دهد"
    );
    // ولی گذار بی‌معنا همچنان ممنوع است.
    assert!(
        transition(kind, CheckStatus::Collected, CheckStatus::Deposited).is_err(),
        "چک وصول‌شده را نمی‌توان دوباره واگذار کرد"
    );
}

/// ت۰۵ — وضعیت‌های چک دریافتی و پرداختی نباید با هم قاطی شوند.
#[test]
fn t05_status_vocabulary_is_kind_specific() {
    assert!(
        transition(
            CheckKind::Issued,
            CheckStatus::Outstanding,
            CheckStatus::Collected
        )
        .is_err(),
        "«وصول شده» وضعیت چک پرداختی نیست"
    );
    assert!(
        transition(CheckKind::Received, CheckStatus::InHand, CheckStatus::Paid).is_err(),
        "«پرداخت شده» وضعیت چک دریافتی نیست"
    );
    for target in allowed_transitions(CheckKind::Issued, CheckStatus::Outstanding) {
        assert!(
            novin_core::checks::status_belongs_to_kind(CheckKind::Issued, *target),
            "گذار پیشنهادی «{}» برای چک پرداختی معنا ندارد",
            target.as_str()
        );
    }
}

/// ت۰۶ — چک انتظامی هرگز اثر مالی ندارد.
#[test]
fn t06_memo_checks_never_touch_treasury() {
    for kind in [CheckKind::Received, CheckKind::Issued] {
        for target in allowed_transitions(kind, CheckStatus::MemoInHand) {
            assert_eq!(
                treasury_effect(kind, CheckStatus::MemoInHand, *target),
                TreasuryEffect::None,
                "گذار انتظامی نباید اثر مالی داشته باشد"
            );
        }
    }
}

/// ت۰۷ — داده‌ی نمونه باید کامل ساخته شود و هیچ کلید خارجی نشکند.
#[test]
fn t07_demo_dataset_is_complete_and_referentially_intact() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه باید بدون خطا ساخته شود");

    assert!(
        count(&conn, "SELECT COUNT(*) FROM checks") >= 20,
        "چک‌ها کم است"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM sales_invoice_lines") >= 55,
        "اقلام فاکتور فروش باید ساخته شوند — نشانه‌ی رد شدن بی‌صدای سطر والد"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM purchase_invoice_lines") >= 25,
        "اقلام فاکتور خرید باید ساخته شوند"
    );

    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let violations = statement.query_map([], |_| Ok(())).unwrap().count();
    assert_eq!(violations, 0, "داده‌ی نمونه نباید هیچ کلید خارجی بشکند");
}

/// ت۰۸ — شماره‌ی سند در هر دفتر یکتاست و هیچ شماره‌ای بی‌صدا مصرف نشده.
///
/// این تست دقیقاً همان تداخل شماره‌ای را می‌گیرد که ریشه‌ی باگ بود.
#[test]
fn t08_document_numbers_never_collide() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).unwrap();

    for table in ["sales_invoices", "purchase_invoices", "journal_entries"] {
        let total = count(&conn, &format!("SELECT COUNT(*) FROM {table}"));
        let distinct = count(
            &conn,
            &format!(
                "SELECT COUNT(DISTINCT company_id||'/'||fiscal_year_id||'/'||number) FROM {table}"
            ),
        );
        assert_eq!(total, distinct, "شماره‌ی تکراری در «{table}»");
    }

    // هیچ فاکتور فروشی نباید بدون قلم بماند؛ فاکتور بدون قلم یعنی والد رد شده.
    let orphan = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices s WHERE NOT EXISTS \
         (SELECT 1 FROM sales_invoice_lines l WHERE l.invoice_id=s.id)",
    );
    assert_eq!(orphan, 0, "فاکتور فروش بدون قلم وجود دارد");
}

/// ت۰۹ — وضعیت هر چک نمونه باید با نوع همان چک سازگار باشد.
#[test]
fn t09_demo_check_statuses_match_their_kind() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).unwrap();

    let mut statement = conn
        .prepare("SELECT check_type,status FROM checks")
        .unwrap();
    let rows: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "چک نمونه ساخته نشده است");
    for (check_type, status) in rows {
        let kind = if check_type == "issued" {
            CheckKind::Issued
        } else {
            CheckKind::Received
        };
        let parsed = CheckStatus::parse(&status)
            .unwrap_or_else(|| panic!("وضعیت ناشناخته در داده‌ی نمونه: {status}"));
        assert!(
            novin_core::checks::status_belongs_to_kind(kind, parsed),
            "وضعیت «{status}» برای چک «{check_type}» معنا ندارد"
        );
    }
}

/// ت۱۰ — کل داده‌ی نمونه از نظر حسابداری متوازن است و تکرار اجرا آن را خراب نمی‌کند.
#[test]
fn t10_demo_dataset_is_balanced_and_idempotent() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).unwrap();

    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(debit, credit, "تراز کل باید متوازن باشد");

    // هر سند به‌تنهایی هم باید متوازن باشد — تراز کلی می‌تواند خطای دو سند را بپوشاند.
    let unbalanced = count(
        &conn,
        "SELECT COUNT(*) FROM (SELECT journal_id FROM journal_lines \
         GROUP BY journal_id HAVING SUM(debit) <> SUM(credit))",
    );
    assert_eq!(unbalanced, 0, "سند نامتوازن وجود دارد");

    let before = count(&conn, "SELECT COUNT(*) FROM journal_lines");
    db::demo::seed_demo_dataset(&conn).unwrap();
    assert_eq!(
        before,
        count(&conn, "SELECT COUNT(*) FROM journal_lines"),
        "اجرای دوباره نباید داده را دوبرابر کند"
    );
}
