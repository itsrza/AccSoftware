#![allow(warnings)] // موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! ممیزی داده‌ی نمونه‌ی بخش‌های پشتیبان.
//!
//! ## چرا این تست‌ها لازم‌اند
//! داده‌ی نمونه فقط «پر کردن جدول» نیست؛ کاربر با آن نرم‌افزار را قضاوت
//! می‌کند. اگر رسید تولیدی بسازیم که معادله‌ی بهای تمام‌شده‌اش برقرار نباشد،
//! یا برگشت خریدی که به فاکتور ناموجود اشاره کند، کاربر یک نرم‌افزار
//! «غلط» می‌بیند — بدتر از نرم‌افزار خالی.
//!
//! پس هر بخش نمونه از سه زاویه سنجیده می‌شود: **وجود دارد**، **به داده‌ی
//! واقعی وصل است**، و **از نظر حسابداری/انباری درست است**.

use novin_core::db;
use rusqlite::Connection;

fn seeded() -> Connection {
    db::open_in_memory().expect("پایگاه داده")
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// تولید
// ---------------------------------------------------------------------------

/// د۱ — فرمول ساخت با اجزای واقعی وجود دارد.
#[test]
fn d1_production_formulas_exist_with_components() {
    let conn = seeded();
    let formulas = count(&conn, "SELECT COUNT(*) FROM production_formulas");
    assert!(formulas >= 2, "فرمول ساخت نمونه ساخته نشده است");

    let orphan_components = count(
        &conn,
        "SELECT COUNT(*) FROM production_formula_components c \
         LEFT JOIN production_formulas f ON f.id=c.formula_id WHERE f.id IS NULL",
    );
    assert_eq!(orphan_components, 0, "جزء فرمول بدون فرمول والد");

    let empty = count(
        &conn,
        "SELECT COUNT(*) FROM production_formulas f \
         WHERE NOT EXISTS(SELECT 1 FROM production_formula_components c WHERE c.formula_id=f.id)",
    );
    assert_eq!(empty, 0, "فرمول بدون هیچ ماده‌ی مصرفی بی‌معناست");
}

/// د۲ — هر جزء فرمول به کالای موجود اشاره می‌کند.
#[test]
fn d2_formula_components_reference_real_products() {
    let conn = seeded();
    let broken = count(
        &conn,
        "SELECT COUNT(*) FROM production_formula_components c \
         LEFT JOIN products p ON p.id=c.product_id WHERE p.id IS NULL",
    );
    assert_eq!(broken, 0, "جزء فرمول به کالای ناموجود اشاره دارد");
}

/// د۳ — رسید تولید نمونه وجود دارد و ورودی و خروجی دارد.
#[test]
fn d3_production_orders_have_inputs_and_outputs() {
    let conn = seeded();
    let orders = count(&conn, "SELECT COUNT(*) FROM production_orders");
    assert!(orders >= 2, "رسید تولید نمونه ساخته نشده است");

    let without_input = count(
        &conn,
        "SELECT COUNT(*) FROM production_orders o \
         WHERE NOT EXISTS(SELECT 1 FROM production_inputs i WHERE i.order_id=o.id)",
    );
    assert_eq!(without_input, 0, "رسید تولید بدون ماده‌ی مصرفی");

    let without_output = count(
        &conn,
        "SELECT COUNT(*) FROM production_orders o \
         WHERE NOT EXISTS(SELECT 1 FROM production_outputs u WHERE u.order_id=o.id)",
    );
    assert_eq!(without_output, 0, "رسید تولید بدون محصول");
}

/// د۴ — **معادله‌ی بهای تمام‌شده در هر رسید تولید دقیقاً برقرار است.**
///
/// مواد مصرفی + هزینه‌ها = بهای تمام‌شده‌ی محصولات. یک ریال اختلاف یعنی
/// موجودی یا سود خطا دارد.
#[test]
fn d4_production_cost_equation_balances_to_the_rial() {
    let conn = seeded();
    let mut statement = conn
        .prepare("SELECT id, materials_total, expenses_total, total_cost FROM production_orders")
        .expect("پرس‌وجو");
    let rows: Vec<(String, i64, i64, i64)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("اجرا")
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "رسید تولیدی وجود ندارد");

    for (id, materials, expenses, total) in rows {
        assert_eq!(
            materials + expenses,
            total,
            "رسید «{id}»: مواد + هزینه با بهای تمام‌شده نمی‌خواند"
        );

        let inputs_sum: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(line_total),0) FROM production_inputs WHERE order_id=?1",
                [&id],
                |row| row.get(0),
            )
            .expect("جمع ورودی");
        assert_eq!(inputs_sum, materials, "رسید «{id}»: جمع سطر مواد غلط است");

        let expenses_sum: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount),0) FROM production_expenses WHERE order_id=?1",
                [&id],
                |row| row.get(0),
            )
            .expect("جمع هزینه");
        assert_eq!(expenses_sum, expenses, "رسید «{id}»: جمع هزینه‌ها غلط است");

        let allocated: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(allocated_cost),0) FROM production_outputs WHERE order_id=?1",
                [&id],
                |row| row.get(0),
            )
            .expect("جمع تخصیص");
        assert_eq!(
            allocated, total,
            "رسید «{id}»: بهای تخصیص‌یافته به محصولات با کل برابر نیست"
        );
    }
}

/// د۵ — محصول تولیدشده در مواد مصرفی همان رسید نیست (فرمول دوری ممنوع).
#[test]
fn d5_no_product_is_both_input_and_output_of_the_same_order() {
    let conn = seeded();
    let circular = count(
        &conn,
        "SELECT COUNT(*) FROM production_inputs i \
         JOIN production_outputs o ON o.order_id=i.order_id AND o.product_id=i.product_id",
    );
    assert_eq!(circular, 0, "کالایی هم ورودی است هم خروجی همان رسید");
}

/// د۶ — هزینه‌های تولید به حساب واقعی و از جنس هزینه وصل‌اند.
#[test]
fn d6_production_expenses_point_to_real_expense_accounts() {
    let conn = seeded();
    let broken = count(
        &conn,
        "SELECT COUNT(*) FROM production_expenses e \
         LEFT JOIN accounts a ON a.id=e.account_id WHERE a.id IS NULL",
    );
    assert_eq!(broken, 0, "هزینه‌ی تولید به حساب ناموجود اشاره دارد");

    let wrong_nature = count(
        &conn,
        "SELECT COUNT(*) FROM production_expenses e \
         JOIN accounts a ON a.id=e.account_id WHERE a.nature <> 'debit'",
    );
    assert_eq!(wrong_nature, 0, "حساب هزینه باید ماهیت بدهکار داشته باشد");
}

// ---------------------------------------------------------------------------
// انبارگردانی
// ---------------------------------------------------------------------------

/// د۷ — یک دوره‌ی انبارگردانی برای تمرین چرخه وجود دارد.
#[test]
fn d7_a_stocktake_session_is_available_for_practice() {
    let conn = seeded();
    let sessions = count(&conn, "SELECT COUNT(*) FROM inventory_count_sessions");
    assert!(sessions >= 1, "دوره‌ی انبارگردانی نمونه وجود ندارد");

    let status: String = conn
        .query_row(
            "SELECT status FROM inventory_count_sessions WHERE id='demo-count-001'",
            [],
            |row| row.get(0),
        )
        .expect("وضعیت دوره");
    assert_eq!(
        status, "counting",
        "دوره باید نیمه‌کاره باشد تا کاربر بتواند چرخه را تمرین کند"
    );
}

/// د۸ — اقلام شمارش هم قلم شمرده‌شده دارند هم نشمرده، و اختلاف واقعی است.
#[test]
fn d8_stocktake_lines_have_real_variance() {
    let conn = seeded();
    let lines = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_count_lines WHERE session_id='demo-count-001'",
    );
    assert!(lines >= 8, "اقلام انبارگردانی کم است");

    let counted = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_count_lines \
         WHERE session_id='demo-count-001' AND counted_quantity IS NOT NULL",
    );
    let pending = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_count_lines \
         WHERE session_id='demo-count-001' AND counted_quantity IS NULL",
    );
    assert!(
        counted > 0 && pending > 0,
        "دوره باید هم شمرده داشته باشد هم نشمرده"
    );

    // اختلاف ذخیره‌شده باید دقیقاً «شمارش − سیستم» باشد.
    let wrong = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_count_lines \
         WHERE counted_quantity IS NOT NULL \
         AND ABS(COALESCE(variance,0) - (counted_quantity - system_quantity)) > 0.000001",
    );
    assert_eq!(wrong, 0, "مقدار اختلاف با شمارش و موجودی سیستمی نمی‌خواند");
}

// ---------------------------------------------------------------------------
// سری/سریال و برگشت از خرید
// ---------------------------------------------------------------------------

/// د۹ — سری ساخت و شماره سریال هر دو نمونه دارند.
#[test]
fn d9_batch_and_serial_lots_both_exist() {
    let conn = seeded();
    let batches = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_lots WHERE lot_type='batch'",
    );
    let serials = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_lots WHERE lot_type='serial'",
    );
    assert!(batches >= 2, "سری ساخت نمونه وجود ندارد");
    assert!(serials >= 2, "سریال نمونه وجود ندارد");

    let missing_serial = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_lots WHERE lot_type='serial' AND serial_number IS NULL",
    );
    assert_eq!(missing_serial, 0, "قلم سریال‌دار بدون شماره سریال");
}

/// د۱۰ — برگشت از خرید به فاکتور خرید واقعی وصل است و مبلغش با سطرش می‌خواند.
#[test]
fn d10_purchase_returns_are_linked_and_consistent() {
    let conn = seeded();
    let returns = count(&conn, "SELECT COUNT(*) FROM purchase_returns");
    assert!(returns >= 1, "برگشت از خرید نمونه وجود ندارد");

    let orphan = count(
        &conn,
        "SELECT COUNT(*) FROM purchase_returns r \
         LEFT JOIN purchase_invoices i ON i.id=r.original_invoice_id WHERE i.id IS NULL",
    );
    assert_eq!(orphan, 0, "برگشت خرید به فاکتور ناموجود اشاره دارد");

    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM purchase_returns r WHERE r.total <> \
         (SELECT COALESCE(SUM(line_total),0) FROM purchase_return_lines l WHERE l.return_id=r.id)",
    );
    assert_eq!(mismatched, 0, "جمع برگشت خرید با سطرهایش نمی‌خواند");

    // برگشت هرگز نمی‌تواند از خود فاکتور بزرگ‌تر باشد.
    let over = count(
        &conn,
        "SELECT COUNT(*) FROM purchase_returns r \
         JOIN purchase_invoices i ON i.id=r.original_invoice_id WHERE r.total > i.total",
    );
    assert_eq!(over, 0, "مبلغ برگشت از مبلغ فاکتور اصلی بیشتر است");
}

// ---------------------------------------------------------------------------
// ابزارها
// ---------------------------------------------------------------------------

/// د۱۱ — قالب چاپ برای هر چهار کاربرد اصلی آماده است.
#[test]
fn d11_print_templates_cover_the_main_document_kinds() {
    let conn = seeded();
    for kind in ["invoice", "receipt", "journal", "label"] {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM print_templates WHERE template_type=?1",
                [kind],
                |row| row.get(0),
            )
            .expect("شمارش قالب");
        assert!(found >= 1, "قالب چاپ «{kind}» وجود ندارد");
    }
}

/// د۱۲ — گزارش‌های ذخیره‌شده پیکربندی JSON معتبر و منبع شناخته‌شده دارند.
#[test]
fn d12_saved_reports_have_valid_source_and_config() {
    let conn = seeded();
    let mut statement = conn
        .prepare("SELECT name, source, config_json FROM custom_reports")
        .expect("پرس‌وجو");
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("اجرا")
        .filter_map(Result::ok)
        .collect();
    assert!(rows.len() >= 3, "گزارش ذخیره‌شده‌ی نمونه وجود ندارد");

    for (name, source, config) in rows {
        assert!(
            ["sales", "purchase", "inventory", "ledger"].contains(&source.as_str()),
            "گزارش «{name}» منبع ناشناخته دارد: {source}"
        );
        assert!(
            config.trim_start().starts_with('{') && config.trim_end().ends_with('}'),
            "پیکربندی گزارش «{name}» JSON نیست"
        );
        assert!(
            config.contains("\"columns\""),
            "گزارش «{name}» بدون ستون قابل اجرا نیست"
        );
    }
}

/// د۱۳ — اتصال API نمونه دامنه‌ی مجاز دارد (بدون آن، کنترل دسترسی بی‌معناست).
#[test]
fn d13_api_profiles_declare_allowed_domains() {
    let conn = seeded();
    let profiles = count(&conn, "SELECT COUNT(*) FROM api_profiles");
    assert!(profiles >= 3, "اتصال API نمونه وجود ندارد");

    let without_domain = count(
        &conn,
        "SELECT COUNT(*) FROM api_profiles WHERE TRIM(allowed_domains)=''",
    );
    assert_eq!(without_domain, 0, "اتصال بدون دامنه‌ی مجاز خطر امنیتی است");

    // حداقل یکی غیرفعال باشد تا کاربر تفاوت وضعیت را ببیند.
    let disabled = count(&conn, "SELECT COUNT(*) FROM api_profiles WHERE enabled=0");
    assert!(
        disabled >= 1,
        "همه‌ی اتصال‌ها فعالند؛ تفاوت وضعیت دیده نمی‌شود"
    );
}

/// د۱۴ — افزونه‌ی نمونه مجوزهای اعلام‌شده دارد.
#[test]
fn d14_plugins_declare_permissions() {
    let conn = seeded();
    let plugins = count(&conn, "SELECT COUNT(*) FROM plugins");
    assert!(plugins >= 2, "افزونه‌ی نمونه وجود ندارد");

    let without_permission = count(
        &conn,
        "SELECT COUNT(*) FROM plugins p \
         WHERE NOT EXISTS(SELECT 1 FROM plugin_permissions pp WHERE pp.plugin_id=p.id)",
    );
    assert_eq!(without_permission, 0, "افزونه بدون مجوز اعلام‌شده");
}

// ---------------------------------------------------------------------------
// یکپارچگی کلی
// ---------------------------------------------------------------------------

/// د۱۵ — افزودن داده‌ی پشتیبان توازن دفتر کل را به هم نزده است.
#[test]
fn d15_ledger_remains_balanced_after_supporting_data() {
    let conn = seeded();
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("جمع");
    assert_eq!(debit, credit, "دفتر کل پس از داده‌ی پشتیبان نامتوازن شد");
}

/// د۱۶ — اجرای دوباره‌ی داده‌ی نمونه چیزی را دوبار درج نمی‌کند.
#[test]
fn d16_seeding_twice_is_idempotent() {
    let conn = seeded();
    let before = (
        count(&conn, "SELECT COUNT(*) FROM production_orders"),
        count(&conn, "SELECT COUNT(*) FROM inventory_count_lines"),
        count(&conn, "SELECT COUNT(*) FROM print_templates"),
        count(&conn, "SELECT COUNT(*) FROM purchase_returns"),
        count(&conn, "SELECT COUNT(*) FROM inventory_lots"),
    );
    db::demo::seed_demo_dataset(&conn).expect("اجرای دوباره");
    let after = (
        count(&conn, "SELECT COUNT(*) FROM production_orders"),
        count(&conn, "SELECT COUNT(*) FROM inventory_count_lines"),
        count(&conn, "SELECT COUNT(*) FROM print_templates"),
        count(&conn, "SELECT COUNT(*) FROM purchase_returns"),
        count(&conn, "SELECT COUNT(*) FROM inventory_lots"),
    );
    assert_eq!(before, after, "اجرای دوباره رکورد تکراری ساخت");
}

/// د۱۷ — هیچ بخش قابل مشاهده‌ای در نرم‌افزار خالی نمی‌ماند.
///
/// این تست فهرست «صفحه‌هایی که کاربر باز می‌کند» را به جدول پشتیبانش
/// نگاشت می‌کند. اگر روزی صفحه‌ای بدون داده‌ی نمونه بماند، همین‌جا قرمز
/// می‌شود — نه در دست کاربر.
#[test]
fn d17_every_visible_section_has_sample_content() {
    let conn = seeded();
    let sections: [(&str, &str); 16] = [
        ("کالاها", "products"),
        ("اشخاص", "contacts"),
        ("انبارها", "warehouses"),
        ("موجودی انبار", "inventory_balances"),
        ("فاکتور فروش", "sales_invoices"),
        ("فاکتور خرید", "purchase_invoices"),
        ("برگشت از فروش", "sales_returns"),
        ("برگشت از خرید", "purchase_returns"),
        ("پیش‌فاکتور و سفارش", "quotes"),
        ("چک‌ها", "checks"),
        ("خزانه", "treasury_accounts"),
        ("سند دریافت و پرداخت", "treasury_documents"),
        ("انتقال بین انبارها", "inventory_transfer_orders"),
        ("تولید", "production_orders"),
        ("انبارگردانی", "inventory_count_sessions"),
        ("قالب‌های چاپ", "print_templates"),
    ];
    for (title, table) in sections {
        let rows = count(&conn, &format!("SELECT COUNT(*) FROM {table}"));
        assert!(
            rows > 0,
            "بخش «{title}» بدون داده‌ی نمونه است (جدول {table})"
        );
    }
}
