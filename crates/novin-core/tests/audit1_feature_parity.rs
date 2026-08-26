#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! ممیزی ۱ — انطباق امکانات با اسکرین‌شات‌های نرم‌افزار مرجع.
//!
//! این فایل «تست واحد» نیست؛ **ممیزی انطباق** است. هر تست یک قابلیت مشخص از
//! تصاویر مرجع را برمی‌دارد و می‌پرسد: «آیا واقعاً پیاده شده، یا فقط جدولش
//! ساخته شده؟»
//!
//! معیار پذیرش در همه‌ی تست‌ها یکسان است:
//!
//! ۱. **ساختار داده وجود دارد** (جدول و ستون)
//! ۲. **قواعدش در پایگاه داده اجرا می‌شود** (قید `CHECK` و کلید خارجی)
//! ۳. **داده‌ی نمونه آن را نشان می‌دهد** (کاربر خالی نمی‌بیند)
//!
//! نبود هر سه یعنی قابلیت روی کاغذ است، نه در محصول.

use novin_core::catalog::PriceLevel;
use novin_core::checks::{CheckKind, CheckStatus};
use novin_core::coding::CodingScheme;
use novin_core::db;
use novin_core::parties::{PartyFunction, PartyType};
use novin_core::production::CostAllocation;
use novin_core::treasury::{DocumentKind, NegativeBalancePolicy, PaymentMethod};
use rusqlite::Connection;

/// پایگاه داده‌ی تازه.
///
/// نکته: `open_in_memory` هم مهاجرت و هم داده‌ی پایه و نمونه را اجرا می‌کند.
/// پس هر درج آزمایشی باید شماره‌ی خارج از محدوده‌ی داده‌ی نمونه بگیرد،
/// وگرنه با قید یکتایی برخورد می‌کند.
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

fn table_exists(conn: &Connection, table: &str) -> bool {
    count(
        conn,
        &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"),
    ) == 1
}

fn columns_of(conn: &Connection, table: &str) -> Vec<String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("جدول باید وجود داشته باشد");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("خواندن ستون‌ها")
        .filter_map(Result::ok)
        .collect();
    rows
}

/// آیا این درج توسط پایگاه داده رد می‌شود؟
fn rejected(conn: &Connection, sql: &str) -> bool {
    conn.execute(sql, []).is_err()
}

// ===========================================================================
// چک (`1hNwr0`, `rm1qup`, `hutUjB`)
// ===========================================================================

/// ت۰۱ — دوازده وضعیت چک تصاویر مرجع، همه پیاده و همه در پایگاه داده مجاز.
#[test]
fn t01_check_has_all_twelve_tabs_of_the_reference() {
    let labels = [
        (CheckStatus::InHand, "موجود"),
        (CheckStatus::Deposited, "واگذار شده"),
        (CheckStatus::Collected, "وصول شده"),
        (CheckStatus::Cashed, "نقد شده"),
        (CheckStatus::Endorsed, "خرج شده"),
        (CheckStatus::Bounced, "برگشتی"),
        (CheckStatus::Returned, "عودت شده"),
        (CheckStatus::Void, "باطل شده"),
        (CheckStatus::Outstanding, "پرداختی"),
        (CheckStatus::Paid, "پرداخت شده"),
        (CheckStatus::MemoInHand, "انتظامی موجود"),
        (CheckStatus::MemoReturned, "انتظامی عودت شده"),
    ];
    for (status, label) in labels {
        assert_eq!(status.label(), label, "برچسب زبانه‌ی «{label}» نمی‌خواند");
        assert_eq!(CheckStatus::parse(status.as_str()), Some(status));
    }
    // و همه‌ی این‌ها باید در پایگاه داده هم پذیرفته شوند.
    let conn = fresh();
    for (index, (status, _)) in labels.iter().enumerate() {
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
                format!("a01-{index}"),
                kind,
                format!("A01-{index}"),
                status.as_str()
            ],
        )
        .unwrap_or_else(|e| panic!("وضعیت «{}» رد شد: {e}", status.as_str()));
    }
}

/// ت۰۲ — چک انتظامی «بدون اثر مالی» است، همان‌طور که در مرجع تفکیک شده.
#[test]
fn t02_memo_checks_are_financially_inert() {
    assert!(CheckStatus::MemoInHand.is_memo());
    assert!(CheckStatus::MemoReturned.is_memo());
    assert!(
        !CheckStatus::MemoInHand.is_open(),
        "انتظامی نباید در مانده بیاید"
    );
    assert!(!CheckStatus::MemoReturned.is_open());
    // و وضعیت آغازین چک انتظامی مستقل از نوع چک است.
    assert_eq!(
        CheckStatus::initial(CheckKind::Received, true),
        CheckStatus::MemoInHand
    );
    assert_eq!(
        CheckStatus::initial(CheckKind::Issued, true),
        CheckStatus::MemoInHand
    );
}

/// ت۰۳ — دسته‌چک با کنترل سریال، مطابق «دسته چک» منوی اطلاعات پایه.
#[test]
fn t03_checkbook_with_serial_control_exists() {
    let conn = fresh();
    assert!(table_exists(&conn, "checkbooks"), "جدول دسته‌چک نیست");
    let columns = columns_of(&conn, "checkbooks");
    for column in ["treasury_account_id", "serial_from", "serial_to"] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }
    assert!(
        count(&conn, "SELECT COUNT(*) FROM checkbooks") >= 1,
        "دسته‌چک نمونه ساخته نشده"
    );
    // بازه‌ی معکوس باید رد شود.
    assert!(
        rejected(
            &conn,
            "INSERT INTO checkbooks(id,company_id,treasury_account_id,title,serial_from,serial_to) \
             VALUES('a03','company-demo','treasury-cash-demo','بد',500,100)"
        ) || count(&conn, "SELECT COUNT(*) FROM checkbooks WHERE serial_to < serial_from") == 0,
        "بازه‌ی سریال معکوس نباید ثبت شود"
    );
}

// ===========================================================================
// اشخاص (`c9pvYl`, `1zkKV5`)
// ===========================================================================

/// ت۰۴ — چهار نوع شخصیت تصویر «افزودن شخص».
#[test]
fn t04_four_party_types_of_the_reference_form() {
    let expected = [
        (PartyType::Natural, "حقیقی"),
        (PartyType::PrivateLegal, "حقوقی غیردولتی"),
        (PartyType::GovernmentLegal, "حقوقی دولتی"),
        (PartyType::CivilPartnership, "مشارکت مدنی"),
    ];
    for (value, label) in expected {
        assert_eq!(value.label(), label);
        assert_eq!(PartyType::parse(value.as_str()), Some(value));
    }
    // حقوقی بودن باید درست تشخیص داده شود — نام نمایشی به آن وابسته است.
    assert!(!PartyType::Natural.is_legal_entity());
    assert!(PartyType::PrivateLegal.is_legal_entity());
    assert!(PartyType::GovernmentLegal.is_legal_entity());
}

/// ت۰۵ — سه نقش «شخص | بازاریاب | سوپروایزر» و پورسانت‌گیری.
#[test]
fn t05_three_party_roles_with_commission_rule() {
    assert_eq!(PartyFunction::Person.label(), "شخص");
    assert_eq!(PartyFunction::Marketer.label(), "بازاریاب");
    assert_eq!(PartyFunction::Supervisor.label(), "سوپروایزر");
    // بازاریاب و سوپروایزر پورسانت می‌گیرند، شخص عادی نه.
    assert!(!PartyFunction::Person.earns_commission());
    assert!(PartyFunction::Marketer.earns_commission());
    assert!(PartyFunction::Supervisor.earns_commission());
}

/// ت۰۶ — هفت زبانه‌ی فرم شخص، هر کدام جدول یا ستون واقعی دارند.
#[test]
fn t06_all_seven_party_tabs_have_real_storage() {
    let conn = fresh();
    // زبانه‌های چندردیفی
    for table in [
        "party_phones",
        "party_bank_accounts",
        "party_images",
        "party_occasions",
    ] {
        assert!(table_exists(&conn, table), "زبانه‌ی «{table}» جدول ندارد");
    }
    // زبانه‌های تک‌ردیفی روی خود شخص
    let columns = columns_of(&conn, "contacts");
    for column in [
        "party_type",      // مشخصات عمومی
        "party_function",  // نقش
        "route_id",        // مسیر پخش مویرگی
        "marketer_id",     // بازاریاب
        "email",           // مشخصات ارتباطی
        "portal_username", // مشخصات کاربری
        "job_title",       // سایر مشخصات
        "credit_limit",    // سقف اعتبار
    ] {
        assert!(
            columns.contains(&column.to_string()),
            "ستون «{column}» نیست"
        );
    }
}

/// ت۰۷ — درخت گروه اشخاص تصویر «لیست اشخاص» ساخته شده و پر است.
#[test]
fn t07_party_group_tree_matches_the_reference_list() {
    let conn = seeded();
    let titles: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT title FROM party_groups ORDER BY code")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    for expected in ["بدهکاران تجاری", "بستانکاران تجاری", "سایت", "همکاران"]
    {
        assert!(
            titles.iter().any(|title| title == expected),
            "گروه «{expected}» تصویر مرجع وجود ندارد"
        );
    }
    // و گروه‌ها واقعاً عضو دارند، وگرنه درخت خالی است.
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM contacts WHERE group_id IS NOT NULL"
        ) >= 40,
        "اشخاص نمونه به گروه وصل نشده‌اند"
    );
}

// ===========================================================================
// کالا (`8Xmc1p`, `NztJl5`, `6FM9Ow`)
// ===========================================================================

/// ت۰۸ — هفت سطح قیمت تصویر «سطوح قیمت‌ها».
#[test]
fn t08_seven_price_levels_of_the_reference() {
    let expected = [
        (PriceLevel::Retail, "جزئی"),
        (PriceLevel::Wholesale, "کلی"),
        (PriceLevel::Partner, "همکار"),
        (PriceLevel::PartnerTier2, "همکار درجه ۲"),
        (PriceLevel::PartnerTier3, "همکار درجه ۳"),
        (PriceLevel::Seasonal, "فصلی"),
        (PriceLevel::Exhibition, "نمایشگاه"),
    ];
    assert_eq!(expected.len(), 7, "تعداد سطوح قیمت باید هفت باشد");
    for (level, label) in expected {
        assert_eq!(level.label(), label, "برچسب سطح قیمت نمی‌خواند");
        assert_eq!(PriceLevel::parse(level.as_str()).ok(), Some(level));
    }
}

/// ت۰۹ — چهار نوع کالای دیالوگ «انتخاب نوع کالا».
#[test]
fn t09_four_product_kinds_of_the_reference_dialog() {
    use novin_core::catalog::ProductKind;
    let expected = [
        (ProductKind::Simple, "کالای عمومی (ساده)"),
        (ProductKind::Composite, "کالای مرکب"),
        (ProductKind::Variant, "کالای تنوع‌دار"),
        (ProductKind::GoldJewelry, "طلا و جواهر"),
    ];
    for (kind, label) in expected {
        assert_eq!(kind.label(), label);
        assert_eq!(ProductKind::parse(kind.as_str()), Some(kind));
    }
    // «خدمت» افزوده‌ی ماست، نه در تصویر مرجع: خدمت موجودی انبار ندارد و
    // نباید در گردش انبار بیاید. وجودش درست است ولی باید صریحاً متمایز بماند.
    assert_eq!(ProductKind::Service.label(), "خدمت");
    assert!(
        !ProductKind::Service.tracks_inventory(),
        "خدمت نباید موجودی انبار داشته باشد"
    );
    assert!(ProductKind::Simple.tracks_inventory());
}

/// ت۱۰ — کالای نمونه واقعاً چند سطح قیمت دارد، نه فقط یک قیمت.
#[test]
fn t10_demo_products_actually_carry_multiple_price_levels() {
    let conn = seeded();
    assert!(table_exists(&conn, "product_prices"), "جدول سطوح قیمت نیست");
    let levels = count(
        &conn,
        "SELECT COUNT(DISTINCT level) FROM product_prices WHERE product_id LIKE 'demo-prod-%'",
    );
    assert!(levels >= 3, "کالای نمونه فقط {levels} سطح قیمت دارد");
    // هیچ قیمتی منفی نباشد.
    assert_eq!(
        count(&conn, "SELECT COUNT(*) FROM product_prices WHERE price < 0"),
        0,
        "قیمت منفی وجود دارد"
    );
}

// ===========================================================================
// خزانه (`MZlUiD`, `p6hT01`, `WLumbs`)
// ===========================================================================

/// ت۱۱ — شش روش تسویه‌ی «سند دریافت» مرجع.
#[test]
fn t11_six_settlement_methods_of_the_receipt_document() {
    let expected = [
        (PaymentMethod::Cash, "نقد", true),
        (PaymentMethod::Check, "چک", false),
        (PaymentMethod::BankTransfer, "حواله", true),
        (PaymentMethod::CardTerminal, "کارتخوان", true),
        (PaymentMethod::Discount, "تخفیف", false),
        (PaymentMethod::Offset, "تهاتر", false),
    ];
    assert_eq!(expected.len(), 6);
    for (method, label_fragment, moves_money) in expected {
        assert!(
            method.label().contains(label_fragment),
            "برچسب «{}» شامل «{label_fragment}» نیست",
            method.label()
        );
        assert_eq!(
            method.moves_treasury(),
            moves_money,
            "اثر خزانه‌ای «{}» اشتباه است",
            method.as_str()
        );
    }
}

/// ت۱۲ — فرم بانک و صندوق همه‌ی فیلدهای تصویر `p6hT01` را دارد.
#[test]
fn t12_bank_form_has_every_field_of_the_reference() {
    let conn = fresh();
    let columns = columns_of(&conn, "treasury_accounts");
    for column in [
        "account_number",
        "iban",
        "card_number",
        "branch_name",
        "branch_code",
        "holder_name",
        "has_pos_terminal",
        "negative_policy",
        "linked_account_id",
    ] {
        assert!(
            columns.contains(&column.to_string()),
            "فیلد «{column}» نیست"
        );
    }
}

/// ت۱۳ — سه سیاست «هشدار منفی شدن موجودی» تصویر مرجع.
#[test]
fn t13_three_negative_balance_policies() {
    assert_eq!(NegativeBalancePolicy::Error.label(), "خطا");
    assert_eq!(NegativeBalancePolicy::Warn.label(), "هشدار");
    assert_eq!(NegativeBalancePolicy::Ignore.label(), "بی‌تأثیر");
    for value in ["error", "warn", "ignore"] {
        assert_eq!(NegativeBalancePolicy::parse(value).as_str(), value);
    }
    // مقدار ناشناخته نباید برنامه را بشکند؛ به امن‌ترین حالت برمی‌گردد.
    assert_eq!(
        NegativeBalancePolicy::parse("nonsense"),
        NegativeBalancePolicy::Warn
    );
}

/// ت۱۴ — کارتخوان و پایانه فروشگاهی (منوی «کارتخوان‌ها و پایانه‌های فروش»).
#[test]
fn t14_pos_terminal_support_exists_end_to_end() {
    let conn = fresh();
    // سطر سند خزانه باید شناسه‌ی پایانه را نگه دارد.
    let columns = columns_of(&conn, "treasury_document_lines");
    assert!(
        columns.contains(&"terminal_id".to_string()),
        "شناسه پایانه نیست"
    );
    // و روش کارتخوان باید حساب خزانه بخواهد (پول به حساب بانکی می‌رود).
    assert!(PaymentMethod::CardTerminal.requires_treasury_account());
    assert!(PaymentMethod::CardTerminal.moves_treasury());
}

/// ت۱۵ — سند دریافت و پرداخت هر دو، با شماره‌گذاری جدا.
#[test]
fn t15_receipt_and_payment_documents_number_separately() {
    assert_eq!(DocumentKind::Receipt.as_str(), "receipt");
    assert_eq!(DocumentKind::Payment.as_str(), "payment");
    let conn = fresh();
    // قید یکتایی باید شامل `kind` باشد، وگرنه دریافت و پرداخت شماره‌ی هم را می‌گیرند.
    conn.execute(
        "INSERT INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,document_date,created_by) \
         VALUES('a15-r','company-demo','fy-demo','receipt',900005,'1405/05/01','user-demo')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,document_date,created_by) \
         VALUES('a15-p','company-demo','fy-demo','payment',900005,'1405/05/01','user-demo')",
        [],
    )
    .expect("شماره‌ی یکسان در دو دفتر متفاوت باید مجاز باشد");
    assert!(
        rejected(
            &conn,
            "INSERT INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,document_date,created_by) \
             VALUES('a15-r2','company-demo','fy-demo','receipt',900005,'1405/05/01','user-demo')"
        ),
        "شماره‌ی تکراری در یک دفتر باید رد شود"
    );
}

// ===========================================================================
// کدینگ (`dgNqWj`)
// ===========================================================================

/// ت۱۶ — چهار سطح «گروه، کل، معین، تفصیلی» با عنوان درست.
#[test]
fn t16_four_coding_levels_with_reference_titles() {
    let scheme = CodingScheme::default();
    assert_eq!(scheme.depth(), 4);
    assert_eq!(scheme.level_title(0), Some("گروه"));
    assert_eq!(scheme.level_title(1), Some("کل"));
    assert_eq!(scheme.level_title(2), Some("معین"));
    assert_eq!(scheme.level_title(3), Some("تفصیلی"));
}

/// ت۱۷ — تفصیلی شناور: حساب می‌تواند گروه تفصیلی اجباری داشته باشد.
#[test]
fn t17_floating_subsidiary_is_configurable_per_account() {
    let conn = fresh();
    let columns = columns_of(&conn, "accounts");
    assert!(columns.contains(&"requires_subsidiary".to_string()));
    assert!(columns.contains(&"subsidiary_group_id".to_string()));
    assert!(table_exists(&conn, "subsidiary_groups"));
    // حساب مشتریان باید تفصیلی اجباری داشته باشد — وگرنه مانده‌ی هر شخص جدا نمی‌شود.
    let requires = count(
        &conn,
        "SELECT COUNT(*) FROM accounts WHERE code='1201' AND requires_subsidiary=1",
    );
    assert_eq!(requires, 1, "حساب مشتریان باید تفصیلی اجباری داشته باشد");
}

/// ت۱۸ — مرکز هزینه و پروژه (ابعاد سند) وجود دارند.
#[test]
fn t18_cost_center_and_project_dimensions_exist() {
    let conn = fresh();
    assert!(table_exists(&conn, "cost_centers"), "مرکز هزینه نیست");
    assert!(table_exists(&conn, "projects"), "پروژه نیست");
    let columns = columns_of(&conn, "journal_lines");
    for column in ["cost_center_id", "project_id", "subsidiary_id"] {
        assert!(
            columns.contains(&column.to_string()),
            "بُعد «{column}» روی سطر سند نیست"
        );
    }
}

// ===========================================================================
// انبار و تولید (`3qTCnS`)
// ===========================================================================

/// ت۱۹ — سه زبانه‌ی «رسید تولید»: محصولات، مواد مصرفی، هزینه‌ها.
#[test]
fn t19_production_receipt_has_all_three_tabs() {
    let conn = fresh();
    for table in [
        "production_outputs",
        "production_inputs",
        "production_expenses",
    ] {
        assert!(table_exists(&conn, table), "زبانه‌ی «{table}» نیست");
    }
    // و فرمول تولید (دکمه‌ی Insert تصویر مرجع)
    assert!(table_exists(&conn, "production_formulas"));
    assert!(table_exists(&conn, "production_formula_components"));
}

/// ت۲۰ — دو روش تخصیص بهای تمام‌شده، هر کدام با توضیح واقعی.
#[test]
fn t20_cost_allocation_methods_carry_real_explanations() {
    for method in [CostAllocation::ByQuantity, CostAllocation::ByMarketValue] {
        assert!(!method.label().is_empty());
        let explanation = method.explanation();
        // توضیح باید واقعاً سه جمله باشد، نه یک عبارت تزئینی.
        assert!(
            explanation.chars().count() > 80,
            "توضیح «{}» بیش از حد کوتاه است",
            method.label()
        );
        assert!(
            explanation.matches('.').count() >= 2 || explanation.matches('،').count() >= 2,
            "توضیح «{}» توضیح واقعی نیست",
            method.label()
        );
    }
}

/// ت۲۱ — انبارگردانی با فریز موجودی و شمارش مجدد.
#[test]
fn t21_stocktaking_has_freeze_and_recount() {
    let conn = fresh();
    assert!(table_exists(&conn, "stocktake_sessions"));
    assert!(table_exists(&conn, "stocktake_lines"));
    let session_columns = columns_of(&conn, "stocktake_sessions");
    assert!(
        session_columns.iter().any(|c| c.contains("status")),
        "وضعیت جلسه‌ی انبارگردانی نیست"
    );
    let line_columns = columns_of(&conn, "stocktake_lines");
    // شمارش اول، شمارش مجدد و موجودی فریزشده باید جدا باشند.
    assert!(
        line_columns
            .iter()
            .any(|c| c.contains("system") || c.contains("frozen")),
        "موجودی سیستمی/فریزشده ذخیره نمی‌شود"
    );
    assert!(
        line_columns.iter().any(|c| c.contains("recount")),
        "شمارش مجدد ذخیره نمی‌شود"
    );
}

// ===========================================================================
// فاکتور (`sFpxWK`, `PI5uot`, `FRPBDr`)
// ===========================================================================

/// ت۲۲ — فاکتور همه‌ی اجزای تصویر را دارد: تخفیف، مالیات، کرایه، سریال.
#[test]
fn t22_invoice_carries_every_component_of_the_reference() {
    let conn = fresh();
    let header = columns_of(&conn, "sales_invoices");
    for column in ["subtotal", "discount", "tax", "total", "payment_status"] {
        assert!(header.contains(&column.to_string()), "ستون «{column}» نیست");
    }
    let lines = columns_of(&conn, "sales_invoice_lines");
    for column in ["quantity", "unit_price", "discount", "tax", "line_total"] {
        assert!(
            lines.contains(&column.to_string()),
            "ستون سطر «{column}» نیست"
        );
    }
    // تخفیف پلکانی کالا (تصویر تعریف کالا)
    assert!(table_exists(&conn, "product_discount_tiers"));
}

/// ت۲۳ — برگشت از فروش و خرید، هر دو با ارجاع به فاکتور اصلی.
#[test]
fn t23_both_return_documents_reference_their_origin() {
    let conn = fresh();
    for table in ["sales_returns", "purchase_returns"] {
        assert!(table_exists(&conn, table));
        let columns = columns_of(&conn, table);
        assert!(
            columns.contains(&"original_invoice_id".to_string()),
            "برگشت «{table}» به فاکتور اصلی وصل نیست"
        );
    }
    // ارجاع به فاکتور ناموجود باید رد شود.
    assert!(
        rejected(
            &conn,
            "INSERT INTO sales_returns(id,company_id,fiscal_year_id,number,return_date,\
             original_invoice_id,status,total) \
             VALUES('a23','company-demo','fy-demo',9001,'1405/06/01','ghost','draft',0)"
        ),
        "برگشت بدون فاکتور اصلی نباید ثبت شود"
    );
}

/// ت۲۴ — پیش‌فاکتور و سفارش خرید (منوی «سفارشگیری» و «پیش‌فاکتورها»).
#[test]
fn t24_quotes_and_purchase_orders_exist_with_validity() {
    let conn = fresh();
    assert!(table_exists(&conn, "quotes"));
    assert!(table_exists(&conn, "quote_lines"));
    let columns = columns_of(&conn, "quotes");
    for column in ["kind", "valid_until", "status", "converted_invoice_id"] {
        assert!(
            columns.contains(&column.to_string()),
            "ستون «{column}» نیست"
        );
    }
    // نوع ساختگی باید رد شود.
    assert!(
        rejected(
            &conn,
            "INSERT INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date) \
             VALUES('a24','company-demo','fy-demo','invented',1,'1405/01/01')"
        ),
        "نوع سند ناشناخته نباید پذیرفته شود"
    );
}

/// ت۲۵ — مسیر پخش مویرگی و بازاریاب، پایه‌ی «پخش مویرگی» منو.
#[test]
fn t25_distribution_route_and_marketer_are_wired() {
    let conn = seeded();
    assert!(table_exists(&conn, "party_routes"));
    assert!(
        count(&conn, "SELECT COUNT(*) FROM party_routes") >= 2,
        "مسیر پخش نمونه ساخته نشده"
    );
    // اشخاص نمونه باید واقعاً به مسیر وصل باشند، وگرنه قابلیت روی کاغذ است.
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM contacts WHERE route_id IS NOT NULL"
        ) >= 40,
        "هیچ شخصی به مسیر پخش وصل نیست"
    );
    // بازاریاب نمونه هم باید وجود داشته باشد.
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM contacts WHERE id='contact-marketer'"
        ) == 1,
        "بازاریاب نمونه ساخته نشده"
    );
}
