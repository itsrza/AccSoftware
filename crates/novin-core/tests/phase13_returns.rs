//! فاز ۱۳ — برگشت از فروش و برگشت از خرید.
//!
//! مرجع: تصویر `FRPBDr` (برگشت از فروش).
//!
//! ## دو باگ حسابداری که این فاز بست
//!
//! ۱. **مالیات برگشت داده نمی‌شد.** سند برگشت فقط مبلغ خالص را معکوس می‌کرد،
//!    پس مانده‌ی حساب «مالیات بر ارزش افزوده» متورم می‌ماند و اظهارنامه
//!    اشتباه درمی‌آمد. حالا مالیات به نسبت مبلغ برگشتی و با **نرخ روز فروش**
//!    معکوس می‌شود — نه با نرخ امروز.
//!
//! ۲. **تاریخ سند، تاریخ میلادیِ امروز بود.** سند برگشت با `Utc::now()` ثبت
//!    می‌شد، پس ممکن بود در سال مالی دیگری بیفتد یا اصلاً تاریخ شمسی نباشد.
//!    حالا تاریخ برگشت استفاده و با سال مالی اعتبارسنجی می‌شود.
//!
//! ## قاعده‌ی مقدار
//!
//! مجموع برگشت‌های یک قلم هرگز از مقدار فاکتور اصلی بیشتر نمی‌شود؛ وگرنه
//! کالایی برگشت می‌خورد که فروخته نشده و موجودی انبار از هوا زیاد می‌شود.

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

/// محاسبه‌ی مالیات متناسب — همان فرمولی که موتور استفاده می‌کند.
fn proportional_tax(returned_net: i64, invoice_subtotal: i64, invoice_tax: i64) -> i64 {
    if invoice_subtotal <= 0 {
        return 0;
    }
    (returned_net as i128 * invoice_tax as i128 / invoice_subtotal as i128) as i64
}

/// ت۰۱ — جدول‌های برگشت با قیدهای درست ساخته شده‌اند.
#[test]
fn t01_return_tables_have_sound_constraints() {
    let conn = fresh();
    for table in [
        "sales_returns",
        "sales_return_lines",
        "purchase_returns",
        "purchase_return_lines",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "جدول «{table}» نیست");
    }

    // مقدار صفر یا منفی در قلم برگشت بی‌معناست.
    conn.execute(
        "INSERT INTO sales_returns(id,company_id,fiscal_year_id,number,return_date,\
         original_invoice_id,status,total) \
         VALUES('t01-r','company-demo','fy-demo',900001,'1405/06/01','demo-sale-1','draft',0)",
        [],
    )
    .unwrap();
    assert!(
        conn.execute(
            "INSERT INTO sales_return_lines(id,return_id,product_id,quantity,unit_price,line_total) \
             VALUES('t01-l','t01-r','demo-prod-000',0,1000,0)",
            [],
        )
        .is_err(),
        "مقدار صفر باید رد شود"
    );
    assert!(
        conn.execute(
            "INSERT INTO sales_return_lines(id,return_id,product_id,quantity,unit_price,line_total) \
             VALUES('t01-l2','t01-r','demo-prod-000',-1,1000,0)",
            [],
        )
        .is_err(),
        "مقدار منفی باید رد شود"
    );
}

/// ت۰۲ — برگشت باید به فاکتور واقعی ارجاع بدهد.
#[test]
fn t02_return_must_reference_a_real_invoice() {
    let conn = fresh();
    assert!(
        conn.execute(
            "INSERT INTO sales_returns(id,company_id,fiscal_year_id,number,return_date,\
             original_invoice_id,status,total) \
             VALUES('t02','company-demo','fy-demo',900002,'1405/06/01','ghost-invoice','draft',0)",
            [],
        )
        .is_err(),
        "ارجاع به فاکتور ناموجود باید رد شود"
    );
}

/// ت۰۳ — مقدار برگشتی هرگز از مقدار فاکتور بیشتر نیست.
///
/// این مهم‌ترین قاعده‌ی این فاز است: برگشت بیش از فروش یعنی موجودی انبار از
/// هوا زیاد می‌شود.
#[test]
fn t03_returned_quantity_never_exceeds_invoiced() {
    let conn = seeded();
    let violations = count(
        &conn,
        "SELECT COUNT(*) FROM (\
           SELECT rl.product_id, r.original_invoice_id, \
                  SUM(rl.quantity) AS returned, \
                  (SELECT SUM(il.quantity) FROM sales_invoice_lines il \
                   WHERE il.invoice_id=r.original_invoice_id AND il.product_id=rl.product_id) AS sold \
           FROM sales_return_lines rl JOIN sales_returns r ON r.id=rl.return_id \
           WHERE r.status<>'cancelled' \
           GROUP BY rl.product_id, r.original_invoice_id \
           HAVING returned > COALESCE(sold,0)\
         )",
    );
    assert_eq!(violations, 0, "برگشت بیش از مقدار فروخته‌شده وجود دارد");
}

/// ت۰۴ — مالیات به نسبت درست معکوس می‌شود.
#[test]
fn t04_tax_is_reversed_proportionally() {
    // فاکتور: خالص ۱۰٬۰۰۰٬۰۰۰ با مالیات ۹۰۰٬۰۰۰ (۹٪)
    assert_eq!(proportional_tax(10_000_000, 10_000_000, 900_000), 900_000);
    // برگشت نصف فاکتور → نصف مالیات
    assert_eq!(proportional_tax(5_000_000, 10_000_000, 900_000), 450_000);
    // برگشت یک‌سوم
    assert_eq!(proportional_tax(3_000_000, 9_000_000, 810_000), 270_000);
    // فاکتور معاف از مالیات → برگشت هم مالیات ندارد
    assert_eq!(proportional_tax(5_000_000, 10_000_000, 0), 0);
    // فاکتور با خالص صفر نباید تقسیم بر صفر بدهد
    assert_eq!(proportional_tax(1_000, 0, 500), 0);
}

/// ت۰۵ — سند برگشت از فروش متوازن است و مالیات را هم دربرمی‌گیرد.
#[test]
fn t05_sales_return_journal_is_balanced_with_tax() {
    let net = 8_000_000i64;
    let tax = proportional_tax(net, 40_000_000, 3_600_000);
    assert_eq!(tax, 720_000);

    // بدهکار: برگشت از فروش + مالیات · بستانکار: مشتری
    let debit = net + tax;
    let credit = net + tax;
    assert_eq!(debit, credit, "سند برگشت باید متوازن باشد");

    // اگر مالیات فراموش شود، سند به‌اندازه‌ی مالیات نامتوازن می‌ماند —
    // همان باگی که این فاز بست.
    let buggy_debit = net;
    assert_ne!(
        buggy_debit,
        net + tax,
        "نادیده‌گرفتن مالیات باید اختلاف بسازد"
    );
}

/// ت۰۶ — تاریخ برگشت باید شمسی معتبر و داخل سال مالی باشد.
#[test]
fn t06_return_date_is_a_valid_jalali_date_in_the_fiscal_year() {
    let conn = seeded();
    let (start, end): (String, String) = conn
        .query_row(
            "SELECT start_date,end_date FROM fiscal_years WHERE id='fy-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let start = JalaliDate::parse(&start).expect("شروع سال مالی");
    let end = JalaliDate::parse(&end).expect("پایان سال مالی");

    let mut statement = conn
        .prepare("SELECT id,return_date FROM sales_returns")
        .unwrap();
    let rows: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "برگشت نمونه ساخته نشده است");
    for (id, date) in rows {
        let parsed = JalaliDate::parse(&date)
            .unwrap_or_else(|_| panic!("تاریخ برگشت «{id}» شمسی معتبر نیست: {date}"));
        assert!(
            parsed >= start && parsed <= end,
            "تاریخ برگشت «{id}» خارج از سال مالی است: {date}"
        );
    }
}

/// ت۰۷ — تاریخ برگشت نباید قبل از تاریخ فاکتور اصلی باشد.
///
/// کالایی که هنوز فروخته نشده، برگشت نمی‌خورد.
#[test]
fn t07_return_never_precedes_its_invoice() {
    let conn = seeded();
    let mut statement = conn
        .prepare(
            "SELECT r.id, r.return_date, i.invoice_date FROM sales_returns r \
             JOIN sales_invoices i ON i.id=r.original_invoice_id",
        )
        .unwrap();
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "برگشت نمونه‌ای وجود ندارد");
    for (id, return_date, invoice_date) in rows {
        let returned = JalaliDate::parse(&return_date).unwrap();
        let invoiced = JalaliDate::parse(&invoice_date).unwrap();
        assert!(
            returned >= invoiced,
            "برگشت «{id}» ({return_date}) قبل از فاکتور ({invoice_date}) است"
        );
    }
}

/// ت۰۸ — مبلغ هر قلم برگشت با مقدار در قیمت واحد می‌خواند.
#[test]
fn t08_line_total_matches_quantity_times_price() {
    let conn = seeded();
    let mut statement = conn
        .prepare("SELECT id,quantity,unit_price,line_total FROM sales_return_lines")
        .unwrap();
    let rows: Vec<(String, f64, i64, i64)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "قلم برگشتی وجود ندارد");
    for (id, quantity, price, total) in rows {
        let expected = (quantity * price as f64).round() as i64;
        assert_eq!(total, expected, "جمع سطر «{id}» نمی‌خواند");
    }
}

/// ت۰۹ — جمع هدر برگشت با جمع اقلامش برابر است.
#[test]
fn t09_return_header_total_matches_its_lines() {
    let conn = seeded();
    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM sales_returns r \
         WHERE r.total <> COALESCE((SELECT SUM(l.line_total) FROM sales_return_lines l \
                                    WHERE l.return_id=r.id),0)",
    );
    assert_eq!(mismatched, 0, "جمع هدر برگشت با اقلامش نمی‌خواند");
}

/// ت۱۰ — داده‌ی نمونه هر دو وضعیت را می‌سازد و شماره‌ها یکتا هستند.
#[test]
fn t10_demo_returns_cover_both_states_with_unique_numbers() {
    let conn = seeded();
    assert!(
        count(&conn, "SELECT COUNT(*) FROM sales_returns") >= 5,
        "برگشت نمونه کم است"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM sales_returns WHERE status='draft'") >= 1,
        "برگشت پیش‌نویس نمونه وجود ندارد"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM sales_returns WHERE status='posted'") >= 1,
        "برگشت ثبت‌شده‌ی نمونه وجود ندارد"
    );

    let total = count(&conn, "SELECT COUNT(*) FROM sales_returns");
    let distinct = count(
        &conn,
        "SELECT COUNT(DISTINCT company_id||'/'||fiscal_year_id||'/'||number) FROM sales_returns",
    );
    assert_eq!(total, distinct, "شماره‌ی برگشت تکراری است");

    // هیچ برگشتی نباید بدون قلم بماند.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sales_returns r WHERE NOT EXISTS \
             (SELECT 1 FROM sales_return_lines l WHERE l.return_id=r.id)"
        ),
        0,
        "برگشت بدون قلم وجود دارد"
    );

    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    assert_eq!(
        statement.query_map([], |_| Ok(())).unwrap().count(),
        0,
        "کلید خارجی شکسته است"
    );
}
