//! فاز ۱۱ — فرم کامل شخص، گروه‌بندی و زبانه‌های تکمیلی.
//!
//! مرجع: تصویر `1zkKV5` (فرم افزودن شخص) و `c9pvYl` (لیست اشخاص).
//!
//! این فاز اطلاعات جانبی شخص را اضافه می‌کند: تلفن‌های چندگانه، حساب‌های
//! بانکی، تصاویر، مشخصات کاربری و تقویم مناسبت‌ها. هیچ‌کدام اثر حسابداری
//! مستقیم ندارند، ولی **یکپارچگی‌شان حیاتی است**: شماره شبای اشتباه یعنی
//! حواله‌ی گم‌شده، و شخص تکراری یعنی مانده‌ی حساب دوتکه.

use novin_core::db;
use novin_core::parties::{
    card_number_is_valid, economic_code_is_valid, iban_is_valid, legal_id_is_valid,
    national_id_is_valid, normalize_mobile, postal_code_is_valid, PartyDefinition, PartyFunction,
    PartyType,
};
use rusqlite::Connection;

fn fresh() -> Connection {
    db::open_in_memory().expect("پایگاه داده باید ساخته شود")
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

fn natural(first: &str, last: &str) -> PartyDefinition {
    PartyDefinition {
        code: "1001".into(),
        party_type: PartyType::Natural,
        function: PartyFunction::Person,
        first_name: Some(first.into()),
        last_name: Some(last.into()),
        company_name: None,
        national_id: None,
        economic_code: None,
        postal_code: None,
        mobile: None,
        is_customer: true,
        is_supplier: false,
        credit_limit: 0,
        route: None,
        marketer_code: None,
    }
}

/// ت۰۱ — جدول‌های زبانه‌ها و ستون‌های تازه واقعاً ساخته می‌شوند.
///
/// مهاجرت ناقص یعنی فرم هنگام ذخیره می‌شکند — و کاربر خطای عمومی می‌بیند.
#[test]
fn t01_schema_has_every_tab_of_the_form() {
    let conn = fresh();
    for table in [
        "party_groups",
        "party_phones",
        "party_bank_accounts",
        "party_images",
        "party_occasions",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "جدول «{table}» ساخته نشده است");
    }

    let mut statement = conn.prepare("PRAGMA table_info(contacts)").unwrap();
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    for column in [
        "code",
        "group_id",
        "is_active",
        "title_prefix",
        "email",
        "website",
        "city",
        "province",
        "portal_username",
        "portal_password_hash",
        "note",
    ] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }
}

/// ت۰۲ — درخت گروه‌بندی اشخاص با گروه‌های واقعی ساخته می‌شود.
#[test]
fn t02_party_group_tree_is_seeded() {
    let conn = fresh();
    assert!(
        count(&conn, "SELECT COUNT(*) FROM party_groups") >= 6,
        "گروه‌های پیش‌فرض ساخته نشده‌اند"
    );
    // گروه فرزند باید به والد موجود اشاره کند، وگرنه درخت شکسته است.
    let orphans = count(
        &conn,
        "SELECT COUNT(*) FROM party_groups c WHERE c.parent_id IS NOT NULL \
         AND NOT EXISTS (SELECT 1 FROM party_groups p WHERE p.id=c.parent_id)",
    );
    assert_eq!(orphans, 0, "گروه یتیم در درخت وجود دارد");
}

/// ت۰۳ — شخص حقوقی بدون نام شرکت و شخص حقیقی بدون نام رد می‌شوند.
#[test]
fn t03_name_rules_differ_by_party_type() {
    let mut legal = natural("", "");
    legal.party_type = PartyType::PrivateLegal;
    legal.first_name = Some("محمد".into());
    legal.last_name = Some("رضایی".into());
    assert!(
        legal.validate().is_err(),
        "شخص حقوقی بدون نام شرکت نباید پذیرفته شود"
    );
    legal.company_name = Some("بازرگانی نمونه".into());
    assert!(legal.validate().is_ok());
    assert_eq!(legal.display_name(), "بازرگانی نمونه");

    let mut person = natural("", "");
    assert!(person.validate().is_err(), "شخص حقیقی بدون نام");
    person.first_name = Some("زهرا".into());
    person.last_name = Some("کریمی".into());
    assert!(person.validate().is_ok());
    assert_eq!(person.display_name(), "زهرا کریمی");
}

/// ت۰۴ — شخص با نقش «شخص» باید حداقل مشتری یا تأمین‌کننده باشد.
///
/// شخصی که هیچ نقش تجاری ندارد در هیچ فاکتوری قابل انتخاب نیست؛ ثبتش یعنی
/// داده‌ی مرده. ولی بازاریاب و سوپروایزر از این قاعده مستثنا هستند.
#[test]
fn t04_commercial_role_required_except_for_marketers() {
    let mut party = natural("علی", "احمدی");
    party.is_customer = false;
    party.is_supplier = false;
    assert!(party.validate().is_err(), "شخص بدون نقش تجاری");

    party.function = PartyFunction::Marketer;
    assert!(
        party.validate().is_ok(),
        "بازاریاب لازم نیست مشتری یا تأمین‌کننده باشد"
    );
    assert!(PartyFunction::Marketer.earns_commission());
}

/// ت۰۵ — شناسه‌ها با الگوریتم رسمی بررسی می‌شوند، نه با طول.
#[test]
fn t05_identifiers_use_real_check_digits() {
    // کد ملی حقیقی
    assert!(national_id_is_valid("0499370899"));
    assert!(national_id_is_valid("0012345679"));
    assert!(
        !national_id_is_valid("1111111111"),
        "ارقام یکسان باید رد شود"
    );
    assert!(!national_id_is_valid("0499370898"), "رقم کنترلی غلط");
    assert!(!national_id_is_valid("049937089"), "طول کوتاه");

    // شناسه ملی حقوقی
    assert!(legal_id_is_valid("10293847568"));
    assert!(legal_id_is_valid("10630617911"));
    assert!(!legal_id_is_valid("10293847569"));

    // کد اقتصادی و کد پستی
    assert!(!economic_code_is_valid("123"));
    assert!(!postal_code_is_valid("12345"));
    assert!(postal_code_is_valid("1234567890"));
}

/// ت۰۶ — شبا و شماره کارت با mod-97 و Luhn بررسی می‌شوند.
#[test]
fn t06_bank_identifiers_are_validated() {
    assert!(iban_is_valid("IR280620000000001234567891"));
    assert!(
        !iban_is_valid("IR280620000000001234567892"),
        "کد کنترلی غلط"
    );
    assert!(!iban_is_valid("IR2806200000000012345678"), "طول نادرست");

    assert!(card_number_is_valid("6037991234567893"));
    assert!(!card_number_is_valid("6037991234567894"), "Luhn غلط");
    assert!(!card_number_is_valid("60379912345678"), "طول نادرست");
}

/// ت۰۷ — موبایل به شکل یکسان ذخیره می‌شود تا تشخیص تکراری کار کند.
///
/// اگر یک شخص با `+98912…` و دیگری با `0912…` ثبت شود، مانده‌ی حسابش دوتکه
/// می‌شود و هیچ گزارشی درست درنمی‌آید.
#[test]
fn t07_mobile_is_normalized_to_one_shape() {
    let expected = Some("09121234567".to_string());
    for input in [
        "09121234567",
        "9121234567",
        "+989121234567",
        "0098 912 123 4567",
        "۰۹۱۲۱۲۳۴۵۶۷",
        "0912-123-4567",
    ] {
        assert_eq!(
            normalize_mobile(input),
            expected,
            "یکسان‌سازی «{input}» درست نیست"
        );
    }
    assert_eq!(
        normalize_mobile("02122334455"),
        None,
        "تلفن ثابت موبایل نیست"
    );
    assert_eq!(normalize_mobile("091212345"), None, "طول کوتاه");
}

/// ت۰۸ — سقف اعتبار منفی بی‌معناست و باید رد شود.
#[test]
fn t08_credit_limit_cannot_be_negative() {
    let mut party = natural("مهدی", "نوری");
    party.credit_limit = -1;
    assert!(party.validate().is_err(), "سقف اعتبار منفی");
    party.credit_limit = 0;
    assert!(party.validate().is_ok(), "صفر یعنی بدون محدودیت");
    party.credit_limit = 500_000_000;
    assert!(party.validate().is_ok());
}

/// ت۰۹ — داده‌ی نمونه زبانه‌های شخص را هم پر می‌کند.
///
/// بدون این، کاربر فرم را باز می‌کند و همه‌ی زبانه‌ها خالی‌اند — همان
/// «دموی ناقص» که کارفرما به آن ایراد گرفت.
#[test]
fn t09_demo_dataset_fills_the_party_tabs() {
    let conn = fresh();
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه باید ساخته شود");

    assert!(
        count(&conn, "SELECT COUNT(*) FROM party_phones") >= 50,
        "تلفن اشخاص ساخته نشده"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM party_bank_accounts") >= 15,
        "حساب بانکی اشخاص ساخته نشده"
    );
    assert!(
        count(&conn, "SELECT COUNT(*) FROM party_occasions") >= 12,
        "مناسبت اشخاص ساخته نشده"
    );
    // هر شخص نمونه باید کد و گروه داشته باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM contacts WHERE id LIKE 'demo-contact-%' \
             AND (code IS NULL OR group_id IS NULL)"
        ),
        0,
        "شخص نمونه بدون کد یا گروه وجود دارد"
    );
}

/// ت۱۰ — قیدهای پایگاه داده داده‌ی بی‌معنا را رد می‌کنند.
///
/// اعتبارسنجی برنامه ممکن است دور زده شود؛ پایگاه داده آخرین خط دفاع است.
#[test]
fn t10_database_rejects_impossible_tab_data() {
    let conn = fresh();
    conn.execute(
        "INSERT INTO contacts(id,company_id,kind,name) \
         VALUES('t10-contact','company-demo','person','آزمون')",
        [],
    )
    .unwrap();

    // ماه ۱۳ وجود ندارد
    assert!(
        conn.execute(
            "INSERT INTO party_occasions(id,contact_id,title,jalali_month,jalali_day) \
             VALUES('t10-o1','t10-contact','تولد',13,1)",
            [],
        )
        .is_err(),
        "ماه ۱۳ باید رد شود"
    );
    // روز ۳۲ وجود ندارد
    assert!(
        conn.execute(
            "INSERT INTO party_occasions(id,contact_id,title,jalali_month,jalali_day) \
             VALUES('t10-o2','t10-contact','تولد',1,32)",
            [],
        )
        .is_err(),
        "روز ۳۲ باید رد شود"
    );
    // مناسبت بدون شخص موجود
    assert!(
        conn.execute(
            "INSERT INTO party_occasions(id,contact_id,title,jalali_month,jalali_day) \
             VALUES('t10-o3','ghost','تولد',1,1)",
            [],
        )
        .is_err(),
        "ارجاع به شخص ناموجود باید رد شود"
    );
    // حذف شخص باید زبانه‌هایش را هم پاک کند (بدون ردیف یتیم)
    conn.execute(
        "INSERT INTO party_phones(id,contact_id,number) VALUES('t10-p','t10-contact','02100000000')",
        [],
    )
    .unwrap();
    conn.execute("DELETE FROM contacts WHERE id='t10-contact'", [])
        .unwrap();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM party_phones WHERE contact_id='t10-contact'"
        ),
        0,
        "تلفن یتیم پس از حذف شخص باقی مانده است"
    );
}
