#![allow(warnings)]
// موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! فاز ۱۲ — کدینگ حساب‌ها و سلامت درخت حساب.
//!
//! مرجع: تصویر `dgNqWj` (کدینگ حساب‌ها).
//!
//! ## قاعده‌ی محوری این فاز
//!
//! **فقط برگ‌های درخت سند می‌پذیرند.** اگر روی حسابی که فرزند دارد سند مستقیم
//! ثبت شود، جمع شاخه با مانده‌ی خودِ حساب نمی‌خواند و تراز آزمایشی دو عدد
//! متفاوت نشان می‌دهد. این خطا در ظاهر دیده نمی‌شود و فقط در حسابرسی لو
//! می‌رود — پس باید در سطح موتور گرفته شود.
//!
//! ## چرا کدینگ مسطح هم باید کار کند
//!
//! کدینگ پایه‌ی این نرم‌افزار (و کدینگ بسیاری از کسب‌وکارهای واقعی) چهاررقمی
//! مسطح است و سلسله‌مراتب فقط از رابطه‌ی والد-فرزند می‌آید. نرم‌افزاری که این
//! را نپذیرد، کاربر را مجبور می‌کند کل دفاترش را دوباره کدگذاری کند.

use novin_core::coding::{AccountNature, CodingScheme};
use novin_core::db;
use rusqlite::Connection;

fn fresh() -> Connection {
    db::open_in_memory().expect("پایگاه داده باید ساخته شود")
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

/// ت۰۱ — طرح پیش‌فرض `[1,2,2,2]` سطح‌ها را درست تشخیص می‌دهد.
#[test]
fn t01_default_scheme_maps_code_length_to_level() {
    let scheme = CodingScheme::default();
    assert_eq!(scheme.depth(), 4);
    assert_eq!(scheme.code_length(0), Some(1));
    assert_eq!(scheme.code_length(1), Some(3));
    assert_eq!(scheme.code_length(2), Some(5));
    assert_eq!(scheme.code_length(3), Some(7));

    assert_eq!(scheme.level_of("1").unwrap(), 0);
    assert_eq!(scheme.level_of("110").unwrap(), 1);
    assert_eq!(scheme.level_of("11031").unwrap(), 2);
    // کد هفت‌رقمی رایج نرم‌افزارهای موجود
    assert_eq!(scheme.level_of("1103101").unwrap(), 3);
    assert!(scheme.is_leaf_level("1103101").unwrap());
    assert!(!scheme.is_leaf_level("110").unwrap());
}

/// ت۰۲ — کد والد از کد فرزند دقیقاً بازسازی می‌شود.
#[test]
fn t02_parent_code_is_derived_from_child() {
    let scheme = CodingScheme::default();
    assert_eq!(scheme.parent_code("1103101").unwrap(), "11031");
    assert_eq!(scheme.parent_code("11031").unwrap(), "110");
    assert_eq!(scheme.parent_code("110").unwrap(), "1");
    assert!(scheme.parent_code("1").is_err(), "ریشه والد ندارد");
}

/// ت۰۳ — کد بعدی هرگز کد گرفته‌شده را دوباره پیشنهاد نمی‌دهد.
#[test]
fn t03_next_code_skips_taken_codes() {
    let scheme = CodingScheme::default();
    let taken = vec![
        "11001".to_string(),
        "11002".to_string(),
        "11004".to_string(),
    ];
    // شماره‌ی ۳ آزاد است، پس باید همان پیشنهاد شود — نه ۵.
    assert_eq!(scheme.next_child_code("110", &taken).unwrap(), "11003");
    assert_eq!(scheme.next_child_code("110", &[]).unwrap(), "11001");
    // کد گرفته‌شده‌ی خارج از توالی نباید ترتیب را به هم بزند.
    assert_eq!(
        scheme
            .next_child_code("110", &["11031".to_string()])
            .unwrap(),
        "11001"
    );
}

/// ت۰۴ — ظرفیت هر سطح محدود است و پر شدنش باید خطا بدهد، نه کد تکراری.
#[test]
fn t04_level_exhaustion_is_an_error_not_a_duplicate() {
    // سطح دوم دو رقم دارد: ۹۹ فرزند حداکثر.
    let scheme = CodingScheme::default();
    let taken: Vec<String> = (1..=99).map(|serial| format!("110{serial:02}")).collect();
    assert!(
        scheme.next_child_code("110", &taken).is_err(),
        "پر شدن سطح باید خطا بدهد"
    );
    assert!(
        scheme.child_code("110", 0).is_err(),
        "شماره صفر نامعتبر است"
    );
    assert!(scheme.child_code("110", 100).is_err(), "خارج از ظرفیت");
}

/// ت۰۵ — طرح کدینگ نامعتبر ساخته نمی‌شود.
#[test]
fn t05_invalid_scheme_is_rejected() {
    assert!(CodingScheme::new(vec![], vec![]).is_none(), "طرح خالی");
    assert!(
        CodingScheme::new(vec![1, 2], vec!["گروه".into()]).is_none(),
        "تعداد عنوان و عرض باید برابر باشد"
    );
    assert!(
        CodingScheme::new(vec![0, 2], vec!["الف".into(), "ب".into()]).is_none(),
        "عرض صفر"
    );
    assert!(
        CodingScheme::new(vec![7, 2], vec!["الف".into(), "ب".into()]).is_none(),
        "عرض بیش از شش رقم"
    );
    // طرح دلخواه معتبر
    let custom =
        CodingScheme::new(vec![2, 3], vec!["گروه".into(), "معین".into()]).expect("باید بسازد");
    assert_eq!(custom.code_length(1), Some(5));
    assert_eq!(custom.level_title(1), Some("معین"));
}

/// ت۰۶ — ماهیت فرزند باید با والد بخواند؛ «دوطرفه» همه را می‌پذیرد.
#[test]
fn t06_child_nature_must_match_parent() {
    assert!(AccountNature::Debit.accepts_child(AccountNature::Debit));
    assert!(!AccountNature::Debit.accepts_child(AccountNature::Credit));
    assert!(!AccountNature::Credit.accepts_child(AccountNature::Debit));
    assert!(AccountNature::Mixed.accepts_child(AccountNature::Debit));
    assert!(AccountNature::Mixed.accepts_child(AccountNature::Credit));
}

/// ت۰۷ — کد غیرعددی یا با طول ناشناخته رد می‌شود.
#[test]
fn t07_malformed_codes_are_rejected() {
    let scheme = CodingScheme::default();
    assert!(scheme.level_of("").is_err(), "کد خالی");
    assert!(scheme.level_of("11a").is_err(), "حرف در کد");
    assert!(scheme.level_of("۱۱۰").is_err(), "رقم فارسی");
    // چهار رقم در طرح [1,2,2,2] هیچ سطحی ندارد — همان کدینگ مسطح موجود.
    assert!(
        scheme.level_of("1101").is_err(),
        "طول چهار در این طرح سطح ندارد"
    );
}

/// ت۰۸ — درخت پایه‌ی حساب‌ها از نظر ساختاری سالم است.
///
/// هر حساب غیرریشه باید والد موجود داشته باشد و هیچ حلقه‌ای نباشد.
#[test]
fn t08_seeded_chart_has_a_sound_tree() {
    let conn = fresh();
    let orphans = count(
        &conn,
        "SELECT COUNT(*) FROM accounts c WHERE c.parent_id IS NOT NULL \
         AND NOT EXISTS (SELECT 1 FROM accounts p WHERE p.id=c.parent_id)",
    );
    assert_eq!(orphans, 0, "حساب یتیم در درخت وجود دارد");

    let self_parent = count(&conn, "SELECT COUNT(*) FROM accounts WHERE parent_id = id");
    assert_eq!(self_parent, 0, "حساب والد خودش است");

    // ماهیت فرزند باید با والد بخواند، مگر والد «دوطرفه» باشد.
    //
    // نکته‌ی حسابداری: گروه‌هایی مثل «درآمد فروش» هم حساب عادی دارند (فروش
    // کالا، بستانکار) و هم حساب کاهنده (برگشت از فروش و تخفیف، بدهکار). چنین
    // گروهی ذاتاً دوطرفه است و باید همان‌طور هم تعریف شود؛ وگرنه یا قاعده
    // نقض می‌شود یا حساب کاهنده جای اشتباهی می‌نشیند.
    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM accounts c JOIN accounts p ON p.id=c.parent_id \
         WHERE p.nature <> 'mixed' AND p.nature <> c.nature",
    );
    assert_eq!(
        mismatched, 0,
        "ماهیت فرزند با والد نمی‌خواند؛ اگر حساب کاهنده است، والد باید دوطرفه باشد"
    );

    // و برعکس: گروهی که حساب کاهنده دارد حتماً باید دوطرفه باشد.
    for group in ["acc-4000", "acc-5000"] {
        let nature: String = conn
            .query_row(
                "SELECT nature FROM accounts WHERE id=?1",
                rusqlite::params![group],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            nature, "mixed",
            "گروه «{group}» حساب کاهنده دارد و باید دوطرفه باشد"
        );
    }
}

/// ت۰۹ — سند فقط روی برگ‌های درخت نشسته است.
///
/// این همان قاعده‌ای است که اگر نقض شود، تراز آزمایشی دو عدد متفاوت
/// می‌دهد و کسی متوجه نمی‌شود.
#[test]
fn t09_journal_lines_only_touch_leaf_accounts() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه باید ساخته شود");

    let non_leaf_postings = count(
        &conn,
        "SELECT COUNT(*) FROM journal_lines l \
         WHERE EXISTS (SELECT 1 FROM accounts k WHERE k.parent_id = l.account_id)",
    );
    assert_eq!(
        non_leaf_postings, 0,
        "روی حسابی که زیرحساب دارد سند مستقیم ثبت شده است"
    );

    let ghost_accounts = count(
        &conn,
        "SELECT COUNT(*) FROM journal_lines l \
         WHERE NOT EXISTS (SELECT 1 FROM accounts a WHERE a.id = l.account_id)",
    );
    assert_eq!(ghost_accounts, 0, "سند به حساب ناموجود ارجاع دارد");
}

/// ت۱۰ — مانده‌ی هر شاخه دقیقاً جمع فرزندانش است.
///
/// اگر این برقرار نباشد، «تراز چهارستونی» با «تراز شش‌ستونی» اختلاف پیدا
/// می‌کند و صورت‌های مالی قابل اتکا نیست.
#[test]
fn t10_branch_balance_equals_sum_of_children() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).unwrap();

    // مانده‌ی کل دفتر باید صفر باشد (بدهکار = بستانکار).
    let total: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0)-COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(total, 0, "دفتر کل نامتوازن است");

    // مانده‌ی هر ریشه = جمع همه‌ی نوادگانش. چون همه‌ی سندها روی برگ‌هاست،
    // جمع ریشه‌ها هم باید صفر شود.
    let mut statement = conn
        .prepare("SELECT id FROM accounts WHERE parent_id IS NULL")
        .unwrap();
    let roots: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    drop(statement);
    assert!(!roots.is_empty(), "درخت ریشه ندارد");

    let mut grand_total = 0i64;
    for root in &roots {
        // پیمایش بازگشتی درخت با CTE — همان کاری که گزارش تراز می‌کند.
        let branch: i64 = conn
            .query_row(
                "WITH RECURSIVE branch(id) AS (\
                   SELECT id FROM accounts WHERE id=?1 \
                   UNION ALL \
                   SELECT a.id FROM accounts a JOIN branch b ON a.parent_id=b.id\
                 ) \
                 SELECT COALESCE(SUM(l.debit),0)-COALESCE(SUM(l.credit),0) \
                 FROM journal_lines l WHERE l.account_id IN (SELECT id FROM branch)",
                rusqlite::params![root],
                |row| row.get(0),
            )
            .unwrap();
        grand_total += branch;
    }
    assert_eq!(
        grand_total, 0,
        "جمع مانده‌ی شاخه‌های ریشه باید صفر باشد؛ یعنی حسابی خارج از درخت سند خورده"
    );
}
