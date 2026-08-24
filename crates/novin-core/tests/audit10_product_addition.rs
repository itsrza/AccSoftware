//! ممیزی دور ۱۰ — خط لوله‌ی کامل «افزودن کالا».
//!
//! مرجع: تصاویر `6FM9Ow` (انتخاب نوع کالا)، `NztJl5` (فرم تعریف کالا) و
//! `8Xmc1p` (لیست کالاها) از نرم‌افزار فعلی — `docs/FEATURE_BASELINE.md` بخش ۳.
//!
//! ## چرا این تست‌ها وجود دارند
//!
//! دور ۷ قواعد را تکه‌تکه سنجید (یک سطح قیمت، یک واحد، یک پله). دور ۱۰
//! **خط لوله‌ی کامل ذخیره‌ی فرم** را می‌سنجد: همان دنباله‌ای از دستورات
//! SQL که `save_product_profile` میزبان در یک تراکنش اجرا می‌کند — درج
//! کالا، بازنویسی سطوح قیمت، واحدهای فرعی، پله‌های تخفیف و مشخصات طلا.
//!
//! اگر روزی یکی از این قواعد سست شود (مثلاً CHECK از اسکیمای جدید حذف
//! شود یا تراکنش «یا همه یا هیچ» بشکند)، این پرونده قرمز می‌شود، نه آنکه
//! خطا جلوی مشتری در لحظه‌ی صدور فاکتور ظاهر شود.

use novin_core::catalog::{
    build_group_tree, expand_variants, gold_price, group_path, GoldPricing, PriceLevel, PriceList,
    ProductGroup, ProductKind, UnitSet, VariantAttribute,
};
use novin_core::db;
use novin_core::money::Money;
use rusqlite::{params, Connection};

fn seeded() -> Connection {
    db::open_in_memory().expect("پایگاه داده")
}

fn rials(value: i64) -> Money {
    Money::from_rials(value)
}

/// شرکت فعال دمو — همان شرکتی که میزبان با `active_context` برمی‌گرداند.
fn company(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM companies ORDER BY id LIMIT 1", [], |row| {
        row.get(0)
    })
    .expect("شرکت پایه")
}

/// درج کالا با همان ستون‌هایی که فرم میزبان می‌نویسد.
fn insert_product(conn: &Connection, company: &str, id: &str, sku: &str, kind: ProductKind) {
    conn.execute(
        "INSERT INTO products(id, company_id, kind, sku, barcode, name, display_name, brand, \
         unit, purchase_price, min_stock, is_service) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            id,
            company,
            kind.as_str(),
            sku,
            format!("690{sku:0>10}"),
            format!("کالای {sku}"),
            format!("نمایشی {sku}"),
            "برند نمونه",
            "عدد",
            1_200_000_i64,
            2.0_f64,
            i64::from(!kind.tracks_inventory())
        ],
    )
    .expect("درج کالا");
}

/// خواندن یک عدد صحیح از پرس‌وجو.
fn count(conn: &Connection, sql: &str, id: &str) -> i64 {
    conn.query_row(sql, params![id], |row| row.get(0))
        .expect("شمارش")
}

// ---------------------------------------------------------------------------
// خط لوله‌ی کامل
// ---------------------------------------------------------------------------

/// ک۱۹ — هر هفت جدول فرم، مقادیر را دقیقاً همان‌طور که ذخیره شده‌اند برمی‌گردانند.
#[test]
fn k19_full_addition_pipeline_round_trip() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-p1", "A100", ProductKind::Simple);
    conn.execute(
        "UPDATE products SET vat_basis_points=900, duty_basis_points=50, tax_code='TAX-100', \
         tax_exempt=0, max_stock=500, reorder_point=10, group_id=NULL WHERE id='audit10-p1'",
        [],
    )
    .unwrap();

    for (level, price) in [
        (PriceLevel::Retail, 2_500_000_i64),
        (PriceLevel::Wholesale, 2_350_000_i64),
        (PriceLevel::Partner, 2_200_000_i64),
    ] {
        conn.execute(
            "INSERT INTO product_prices(product_id, level, price) VALUES(?1,?2,?3)",
            params!["audit10-p1", level.as_str(), price],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor, is_default_sale) \
         VALUES('audit10-p1-u0','audit10-p1','کارتن',12.0,1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor, is_default_sale) \
         VALUES('audit10-p1-u1','audit10-p1','دسته',0.5,0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-p1-t0','audit10-p1',10.0,300)",
        [],
    )
    .unwrap();

    // --- خواندن برگشت ---
    let (kind, sku, barcode, name, unit, purchase, is_service): (
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
    ) = conn
        .query_row(
            "SELECT kind, sku, barcode, name, unit, purchase_price, is_service FROM products \
             WHERE id='audit10-p1'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(kind, "simple");
    assert_eq!(sku, "A100");
    assert_eq!(barcode, "69000000A100");
    assert_eq!(name, "کالای A100");
    assert_eq!(unit, "عدد");
    assert_eq!(purchase, 1_200_000);
    assert_eq!(is_service, 0, "کالای ساده موجودی دارد");

    let prices: Vec<(String, i64)> = {
        let mut statement = conn
            .prepare("SELECT level, price FROM product_prices WHERE product_id=?1 ORDER BY level")
            .unwrap();
        statement
            .query_map(params!["audit10-p1"], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    assert_eq!(prices.len(), 3);
    assert!(prices.contains(&("partner".into(), 2_200_000)));
    assert!(prices.contains(&("retail".into(), 2_500_000)));

    let (factor, is_default): (f64, i64) = conn
        .query_row(
            "SELECT factor, is_default_sale FROM product_units WHERE product_id='audit10-p1' \
             AND unit_name='کارتن'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(factor, 12.0);
    assert_eq!(is_default, 1);

    let (min_q, discount): (f64, i64) = conn
        .query_row(
            "SELECT min_quantity, discount_bp FROM product_discount_tiers \
             WHERE product_id='audit10-p1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(min_q, 10.0);
    assert_eq!(discount, 300);
}

/// ک۲۰ — کد کالای تکراری در همان شرکت رد می‌شود و متن خطا صریح است.
#[test]
fn k20_duplicate_sku_rejected_within_company() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-d1", "DUP", ProductKind::Simple);
    let duplicate = conn.execute(
        "INSERT INTO products(id, company_id, kind, sku, name, unit) \
         VALUES('audit10-d2',?1,'simple','DUP','تکراری','عدد')",
        params![company],
    );
    let message = duplicate.expect_err("کد تکراری باید رد شود").to_string();
    assert!(
        message.contains("UNIQUE"),
        "پیام خطا باید منشأ UNIQUE باشد: {message}"
    );
    // کد متفاوت در همان شرکت آزاد است.
    conn.execute(
        "INSERT INTO products(id, company_id, kind, sku, name, unit) \
         VALUES('audit10-d3',?1,'simple','DUP2','مجاز','عدد')",
        params![company],
    )
    .expect("کد متفاوت باید پذیرفته شود");
}

/// ک۲۱ — پایگاه داده خودش فضای خالی را نمی‌تراشد؛ به همین دلیل میزبان trim می‌کند.
#[test]
fn k21_database_does_not_trim_sku_so_host_must() {
    let conn = seeded();
    let company = company(&conn);
    // میزبان پیش از درج `sku.trim()` می‌زند؛ اگر نزند، « SK1 » و «SK1»
    // دو کالای متفاوت‌اند و جستجو با کد، دومی را نمی‌بیند.
    conn.execute(
        "INSERT INTO products(id, company_id, kind, sku, name, unit) \
         VALUES('audit10-w1',?1,'simple',' SK1 ','با فاصله','عدد')",
        params![company],
    )
    .unwrap();
    let found: Result<String, _> = conn.query_row(
        "SELECT id FROM products WHERE company_id=?1 AND sku='SK1'",
        params![company],
        |row| row.get(0),
    );
    assert!(
        found.is_err(),
        "بدون trim، جستجوی کدِ تریم‌شده ردیف را پیدا نمی‌کند"
    );
    let raw: String = conn
        .query_row(
            "SELECT sku FROM products WHERE id='audit10-w1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(raw, " SK1 ", "مقدار خام بدون تغییر ذخیره می‌شود");
}

/// ک۲۲ — بازنویسی سطوح قیمت «پاک و دوباره‌نویس» است و PK تکرار را نمی‌پذیرد.
#[test]
fn k22_price_rewrite_semantics() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-pr", "PR1", ProductKind::Simple);

    let write_levels = |levels: &[(&str, i64)]| {
        conn.execute(
            "DELETE FROM product_prices WHERE product_id='audit10-pr'",
            [],
        )
        .unwrap();
        for (level, price) in levels {
            conn.execute(
                "INSERT INTO product_prices(product_id, level, price) VALUES(?1,?2,?3)",
                params!["audit10-pr", level, price],
            )
            .unwrap();
        }
    };

    // هر هفت سطح
    let all = [
        ("retail", 1_000_000_i64),
        ("wholesale", 950_000),
        ("partner", 900_000),
        ("partner_tier2", 850_000),
        ("partner_tier3", 800_000),
        ("seasonal", 700_000),
        ("exhibition", 600_000),
    ];
    write_levels(&all);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM product_prices WHERE product_id=?1",
            "audit10-pr"
        ),
        7
    );

    // بازنویسی با فقط دو سطح → بقیه باید حذف شده باشند (خالی کردن سطح معنادار است)
    write_levels(&[("retail", 1_100_000), ("wholesale", 1_000_000)]);
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM product_prices WHERE product_id=?1",
            "audit10-pr"
        ),
        2,
        "بازنویسی باید سطوح قبلی را پاک کند"
    );
    let seasonal: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM product_prices WHERE product_id=?1 AND level='seasonal'",
        "audit10-pr",
    );
    assert_eq!(seasonal, 0);

    // درج مستقیم سطح تکراری در همان تراکنش باید به PK بخورد
    let duplicate = conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-pr','retail',5)",
        [],
    );
    assert!(
        duplicate.is_err(),
        "PK(product_id, level) تکرار را رد می‌کند"
    );
}

/// ک۲۳ — مرزهای قیمت: صفر مجاز، منفی ممنوع، مبالغ بزرگ بدون خطای گردکردن.
#[test]
fn k23_price_boundaries_and_large_values() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-b", "B1", ProductKind::Simple);

    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-b','retail',0)",
        [],
    )
    .expect("قیمت صفر (کالای رایگان/استعلامی) مجاز است");

    let negative = conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-b','wholesale',-1)",
        [],
    );
    assert!(negative.is_err(), "CHECK(price >= 0) قیمت منفی را رد می‌کند");

    // ۹ همت ریال — بالای محدوده‌ی واقعی کسب‌وکار، داخل محدوده‌ی i64
    let huge = 9_000_000_000_000_i64;
    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-b','partner',?1)",
        params![huge],
    )
    .unwrap();
    let stored: i64 = conn
        .query_row(
            "SELECT price FROM product_prices WHERE product_id='audit10-b' AND level='partner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, huge, "مبلغ بزرگ باید بدون تغییر ذخیره شود");
}

/// ک۲۴ — ستون sale_price همیشه آینه‌ی سطح «جزئی» است: خالی یعنی صفر.
#[test]
fn k24_sale_price_mirrors_retail_level() {
    let conn = seeded();
    let company = company(&conn);

    // کالای با قیمت جزئی → sale_price همان است
    insert_product(&conn, &company, "audit10-r1", "R1", ProductKind::Simple);
    conn.execute(
        "UPDATE products SET sale_price=2500000 WHERE id='audit10-r1'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-r1','retail',2500000)",
        [],
    )
    .unwrap();

    // کالای فقط با قیمت کلی → sale_price صفر می‌ماند
    insert_product(&conn, &company, "audit10-r2", "R2", ProductKind::Simple);
    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-r2','wholesale',900000)",
        [],
    )
    .unwrap();

    // نامعادلی روی کل داده (شامل دمو) — نسبت یک‌به‌یک retail↔sale_price
    let mismatched: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM products p \
             LEFT JOIN product_prices pr ON pr.product_id=p.id AND pr.level='retail' \
             WHERE p.sale_price <> COALESCE(pr.price, 0)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        mismatched, 0,
        "هر کالا یا قیمت جزئی هم‌ارزش sale_price دارد یا sale_price=0 بدون سطح جزئی"
    );
}

// ---------------------------------------------------------------------------
// واحدهای فرعی
// ---------------------------------------------------------------------------

/// ک۲۵ — ضریب تبدیل باید مثبت باشد؛ کسری هم مجاز است (نیم‌دسته، وزنه).
#[test]
fn k25_unit_factor_constraints() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-u", "U1", ProductKind::Simple);

    for bad in [0.0_f64, -2.5_f64] {
        let rejected = conn.execute(
            "INSERT INTO product_units(id, product_id, unit_name, factor) \
             VALUES('audit10-u-x','audit10-u','بد',?1)",
            params![bad],
        );
        assert!(rejected.is_err(), "ضریب {bad} باید رد شود");
    }

    for (suffix, factor) in [("carton", 12.0_f64), ("half", 0.5_f64)] {
        conn.execute(
            "INSERT INTO product_units(id, product_id, unit_name, factor) VALUES(?1,'audit10-u',?2,?3)",
            params![format!("audit10-u-{suffix}"), suffix, factor],
        )
        .unwrap_or_else(|error| panic!("ضریب {factor} مجاز است: {error}"));
    }

    let factors: Vec<f64> = {
        let mut statement = conn
            .prepare(
                "SELECT factor FROM product_units WHERE product_id='audit10-u' ORDER BY factor",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    assert_eq!(factors, vec![0.5, 12.0]);
}

/// ک۲۶ — نام واحد فرعی تکراری برای یک کالا رد می‌شود (UNIQUE).
#[test]
fn k26_duplicate_unit_name_rejected() {
    let conn = seeded();
    let company = company(&conn);
    insert_product(&conn, &company, "audit10-un", "UN1", ProductKind::Simple);
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor) \
         VALUES('audit10-un-0','audit10-un','کارتن',12.0)",
        [],
    )
    .unwrap();
    let duplicate = conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor) \
         VALUES('audit10-un-1','audit10-un','کارتن',24.0)",
        [],
    );
    assert!(
        duplicate.is_err(),
        "UNIQUE(product_id, unit_name) واحد هم‌نام را رد می‌کند"
    );
    // همان نام برای کالای دیگر مشکلی ندارد
    insert_product(&conn, &company, "audit10-un2", "UN2", ProductKind::Simple);
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor) \
         VALUES('audit10-un2-0','audit10-un2','کارتن',6.0)",
        [],
    )
    .expect("نام واحد برای کالای دیگر آزاد است");
}

/// ک۲۷ — ستون‌هایی که فرم میزبان می‌نویسد باید در اسکیما موجود باشند.
#[test]
fn k27_addition_pipeline_schema_contract() {
    let conn = seeded();
    let expected: &[(&str, &[&str])] = &[
        (
            "products",
            &[
                "kind",
                "barcode",
                "display_name",
                "brand",
                "group_id",
                "max_stock",
                "reorder_point",
                "vat_basis_points",
                "duty_basis_points",
                "tax_code",
                "tax_exempt",
            ],
        ),
        ("product_units", &["unit_name", "factor", "is_default_sale"]),
        ("product_discount_tiers", &["min_quantity", "discount_bp"]),
        (
            "product_gold_specs",
            &["weight_grams", "carat", "making_charge_bp", "profit_bp"],
        ),
        ("product_variants", &["sku", "attribute_values", "barcode"]),
    ];
    for (table, columns) in expected {
        let actual: Vec<String> = {
            let mut statement = conn
                .prepare(&format!("PRAGMA table_info({table})"))
                .unwrap();
            statement
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect()
        };
        for column in *columns {
            assert!(
                actual.contains(&column.to_string()),
                "جدول {table} ستون {column} را ندارد"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// تخفیف پلکانی
// ---------------------------------------------------------------------------

/// ک۲۸ — مرزهای پله‌ی تخفیف: مقدار صفر ممنوع، درصد ۰ تا ۱۰۰، آستانه‌ی تکراری ممنوع.
#[test]
fn k28_tier_constraint_boundaries() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-t", "T1", ProductKind::Simple);

    let zero_min = conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-t-0','audit10-t',0,300)",
        [],
    );
    assert!(zero_min.is_err(), "CHECK(min_quantity > 0) صفر را رد می‌کند");

    let over_discount = conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-t-1','audit10-t',5,10001)",
        [],
    );
    assert!(over_discount.is_err(), "تخفیف بیش از ۱۰۰٪ رد می‌شود");

    // مرزهای مجاز: ۰٪ و ۱۰۰٪
    conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-t-2','audit10-t',1,0)",
        [],
    )
    .expect("تخفیف صفر مجاز است");
    conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-t-3','audit10-t',50,10000)",
        [],
    )
    .expect("تخفیف ۱۰۰٪ در مرز مجاز است");

    let duplicate_threshold = conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-t-4','audit10-t',1,500)",
        [],
    );
    assert!(
        duplicate_threshold.is_err(),
        "UNIQUE(product_id, min_quantity) آستانه‌ی تکراری را رد می‌کند"
    );
}

// ---------------------------------------------------------------------------
// طلا و جواهر
// ---------------------------------------------------------------------------

/// ک۲۹ — مشخصات طلا در پایگاه داده و پیش‌نمایش قیمت با هم می‌خوانند.
#[test]
fn k29_gold_preview_pipeline_from_database() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-g", "G1", ProductKind::GoldJewelry);
    conn.execute(
        "UPDATE products SET vat_basis_points=900 WHERE id='audit10-g'",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_gold_specs(product_id, weight_grams, carat, making_charge_bp, profit_bp) \
         VALUES('audit10-g', 10.0, 18, 700, 500)",
        [],
    )
    .unwrap();

    // همان پرس‌وجوی preview_gold_price میزبان: وزن و اجرت و سود از جدول طلا،
    // نرخ ارزش افزوده از خود کالا.
    let (weight, making, profit, vat): (f64, i64, i64, i64) = conn
        .query_row(
            "SELECT s.weight_grams, s.making_charge_bp, s.profit_bp, p.vat_basis_points \
             FROM product_gold_specs s JOIN products p ON p.id = s.product_id \
             WHERE s.product_id='audit10-g'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!((weight as i64, making, profit, vat), (10, 700, 500, 900));

    let breakdown = gold_price(GoldPricing {
        weight_grams: weight,
        rate_per_gram: rials(50_000_000),
        making_charge_bp: making,
        profit_bp: profit,
        vat_bp: vat,
    })
    .unwrap();

    // ارزش فلز = ۱۰ × ۵۰م = ۵۰۰م؛ اجرت ۷٪ = ۳۵م؛ سود ۵٪ از ۵۳۵م = ۲۶٫۷۵م؛
    // ارزش افزوده ۹٪ فقط از اجرت+سود = ۵٬۵۵۷٬۵۰۰؛ جمع = ۵۶۷٬۳۰۷٬۵۰۰.
    assert_eq!(breakdown.metal_value, rials(500_000_000));
    assert_eq!(breakdown.making_charge, rials(35_000_000));
    assert_eq!(breakdown.profit, rials(26_750_000));
    assert_eq!(breakdown.vat, rials(5_557_500));
    assert_eq!(breakdown.total, rials(567_307_500));
}

/// ک۳۰ — وزن طلا باید مثبت باشد (CHECK) و مقدار اعشاری دقیق برمی‌گردد.
#[test]
fn k30_gold_weight_constraints() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-g2", "G2", ProductKind::GoldJewelry);
    for bad in [0.0_f64, -0.1_f64] {
        let rejected = conn.execute(
            "INSERT INTO product_gold_specs(product_id, weight_grams, carat) \
             VALUES('audit10-g2',?1,18)",
            params![bad],
        );
        assert!(rejected.is_err(), "وزن {bad} باید رد شود");
    }
    // حلقه‌ی ظریف: ۲٫۴۲ گرم — باید بدون تغییر برگردد
    conn.execute(
        "INSERT INTO product_gold_specs(product_id, weight_grams, carat) \
         VALUES('audit10-g2',2.42,18)",
        [],
    )
    .unwrap();
    let weight: f64 = conn
        .query_row(
            "SELECT weight_grams FROM product_gold_specs WHERE product_id='audit10-g2'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(weight, 2.42);
}

// ---------------------------------------------------------------------------
// حذف و پرچم خدمت
// ---------------------------------------------------------------------------

/// ک۳۱ — حذف کالا همه‌ی وابستگی‌هایش را هم پاک می‌کند (ON DELETE CASCADE).
#[test]
fn k31_delete_product_cascades() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-c", "C1", ProductKind::Simple);
    for level in [PriceLevel::Retail, PriceLevel::Wholesale] {
        conn.execute(
            "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-c',?1,1000)",
            params![level.as_str()],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor) \
         VALUES('audit10-c-u0','audit10-c','کارتن',12.0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
         VALUES('audit10-c-t0','audit10-c',5,300)",
        [],
    )
    .unwrap();

    conn.execute("DELETE FROM products WHERE id='audit10-c'", [])
        .unwrap();

    for table in ["product_prices", "product_units", "product_discount_tiers"] {
        let left: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE product_id='audit10-c'"),
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(left, 0, "{table} باید با حذف کالا خالی شود");
    }
}

/// ک۳۲ — پرچم is_service دقیقاً معکوس tracks_inventory نوع کالاست.
#[test]
fn k32_service_flag_matches_kind() {
    let conn = seeded();
    let firm = company(&conn);
    for (id, sku, kind) in [
        ("audit10-s1", "S1", ProductKind::Simple),
        ("audit10-s2", "S2", ProductKind::Service),
        ("audit10-s3", "S3", ProductKind::GoldJewelry),
    ] {
        insert_product(&conn, &firm, id, sku, kind);
    }
    let flags: Vec<(String, i64)> = {
        let mut statement = conn
            .prepare("SELECT id, is_service FROM products WHERE id LIKE 'audit10-s%' ORDER BY id")
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    assert_eq!(
        flags,
        vec![
            ("audit10-s1".to_string(), 0),
            ("audit10-s2".to_string(), 1),
            ("audit10-s3".to_string(), 0),
        ]
    );
}

// ---------------------------------------------------------------------------
// کالای تنوع‌دار و مرکب
// ---------------------------------------------------------------------------

/// ک۳۳ — تنوع‌ها با SKU یکتا ذخیره می‌شوند و ترکیب مقادیر باقی می‌ماند.
#[test]
fn k33_variant_expansion_persists() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-v", "V1", ProductKind::Variant);
    conn.execute(
        "INSERT INTO product_attributes(id, product_id, name, values_csv) \
         VALUES('audit10-v-a0','audit10-v','رنگ','قرمز,آبی')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_attributes(id, product_id, name, values_csv) \
         VALUES('audit10-v-a1','audit10-v','سایز','S,M,L')",
        [],
    )
    .unwrap();

    let combinations = expand_variants(
        "SHIRT",
        &[
            VariantAttribute {
                name: "رنگ".into(),
                values: vec!["قرمز".into(), "آبی".into()],
            },
            VariantAttribute {
                name: "سایز".into(),
                values: vec!["S".into(), "M".into(), "L".into()],
            },
        ],
    )
    .unwrap();
    assert_eq!(combinations.len(), 6, "۲ رنگ × ۳ سایز = ۶ تنوع");

    for (index, combination) in combinations.iter().enumerate() {
        conn.execute(
            "INSERT INTO product_variants(id, product_id, sku, attribute_values, barcode) \
             VALUES(?1,'audit10-v',?2,?3,?4)",
            params![
                format!("audit10-v-v{index}"),
                combination.sku,
                combination.values.join(", "),
                format!("690{index:0>9}")
            ],
        )
        .unwrap();
    }
    let duplicate = conn.execute(
        "INSERT INTO product_variants(id, product_id, sku, attribute_values) \
         VALUES('audit10-v-dup','audit10-v','SHIRT-001','تکراری')",
        [],
    );
    assert!(
        duplicate.is_err(),
        "UNIQUE(product_id, sku) تکرار را رد می‌کند"
    );

    let stored: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM product_variants WHERE product_id=?1",
        "audit10-v",
    );
    assert_eq!(stored, 6);
    let first: (String, String) = conn
        .query_row(
            "SELECT sku, attribute_values FROM product_variants \
             WHERE product_id='audit10-v' AND id='audit10-v-v0'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(first, ("SHIRT-001".to_string(), "قرمز, S".to_string()));
}

/// ک۳۴ — جزء کالای مرکب: خود‌ارجاعی و مقدار نامعتبر در پایگاه داده رد می‌شود.
#[test]
fn k34_composite_components_constraints() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-m", "M1", ProductKind::Composite);
    insert_product(&conn, &firm, "audit10-mc", "M1-C1", ProductKind::Simple);

    let self_reference = conn.execute(
        "INSERT INTO product_components(parent_id, component_id, quantity) \
         VALUES('audit10-m','audit10-m',1)",
        [],
    );
    assert!(
        self_reference.is_err(),
        "CHECK(parent <> component) خود‌ارجاعی را رد می‌کند"
    );

    let zero_quantity = conn.execute(
        "INSERT INTO product_components(parent_id, component_id, quantity) \
         VALUES('audit10-m','audit10-mc',0)",
        [],
    );
    assert!(
        zero_quantity.is_err(),
        "CHECK(quantity > 0) مقدار صفر را رد می‌کند"
    );

    conn.execute(
        "INSERT INTO product_components(parent_id, component_id, quantity) \
         VALUES('audit10-m','audit10-mc',2.5)",
        [],
    )
    .expect("جزء معتبر باید پذیرفته شود");
}

// ---------------------------------------------------------------------------
// بازیابی منطق هسته از داده‌ی ذخیره‌شده
// ---------------------------------------------------------------------------

/// ک۳۵ — زنجیره‌ی جایگزینی قیمت بعد از ذخیره و بارگذاری مجدد هم کار می‌کند.
#[test]
fn k35_effective_price_after_reload() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-e", "E1", ProductKind::Simple);
    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-e','retail',1000000)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-e','wholesale',950000)",
        [],
    )
    .unwrap();

    // همان کاری که موتور فاکتور بعد از خواندن از پایگاه داده می‌کند
    let mut list = PriceList::new();
    let mut statement = conn
        .prepare("SELECT level, price FROM product_prices WHERE product_id='audit10-e'")
        .unwrap();
    let rows: Vec<(String, i64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    for (level, price) in rows {
        list.set(PriceLevel::parse(&level).unwrap(), rials(price))
            .unwrap();
    }

    // همکار درجه۳ تعریف نشده → باید تا کلی عقب بکشد و همانجا بایستد
    assert_eq!(
        list.effective(PriceLevel::PartnerTier3).unwrap(),
        rials(950_000),
        "زنجیره‌ی t3←t2←partner←wholesale باید روی wholesale بایستد"
    );
    assert_eq!(
        list.effective(PriceLevel::Seasonal).unwrap(),
        rials(1_000_000)
    );
    assert_eq!(
        list.effective(PriceLevel::Retail).unwrap(),
        rials(1_000_000)
    );
}

/// ک۳۶ — تبدیل واحد و قیمت واحد فرعی، از ضریب ذخیره‌شده ساخته می‌شود.
#[test]
fn k36_unit_price_after_reload() {
    let conn = seeded();
    let firm = company(&conn);
    insert_product(&conn, &firm, "audit10-u2", "U2", ProductKind::Simple);
    conn.execute(
        "INSERT INTO product_units(id, product_id, unit_name, factor, is_default_sale) \
         VALUES('audit10-u2-0','audit10-u2','کارتن',12.0,1)",
        [],
    )
    .unwrap();

    let mut statement = conn
        .prepare("SELECT unit_name, factor FROM product_units WHERE product_id='audit10-u2'")
        .unwrap();
    let mut units = UnitSet::new("عدد");
    {
        let rows: Vec<(String, f64)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        for (name, factor) in rows {
            units = units.with_unit(&name, factor).unwrap();
        }
    }

    assert_eq!(units.to_base(2.0, "کارتن").unwrap(), 24.0);
    assert_eq!(units.from_base(24.0, "کارتن").unwrap(), 2.0);
    // قیمت کارتن = قیمت عدد × ۱۲ — همان چیزی که فاکتور باید بزند
    assert_eq!(
        units.unit_price(rials(120_000), "کارتن").unwrap(),
        rials(1_440_000)
    );
    assert!(
        units.to_base(1.0, "بسته").is_err(),
        "واحد تعریف‌نشده خطا می‌دهد"
    );
}

/// ک۳۷ — درخت گروه کالا از پایگاه داده با همان ساختار میزبان ساخته می‌شود.
#[test]
fn k37_group_tree_from_database() {
    let conn = seeded();
    let firm = company(&conn);
    for (id, code, title, parent) in [
        ("audit10-g1", "950", "گروه ممیزی", None),
        ("audit10-g2", "9501", "زیرگروه", Some("audit10-g1")),
        ("audit10-g3", "950101", "زیر زیرگروه", Some("audit10-g2")),
    ] {
        conn.execute(
            "INSERT INTO product_groups(id, company_id, code, title, parent_id) VALUES(?1,?2,?3,?4,?5)",
            params![id, firm, code, title, parent],
        )
        .unwrap();
    }

    // همان نگاشت list_product_groups میزبان: parent_id → parent_code
    let mut statement = conn
        .prepare("SELECT code, title, parent_id FROM product_groups WHERE id LIKE 'audit10-g%'")
        .unwrap();
    let rows: Vec<(String, String, Option<String>)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    let by_id: std::collections::BTreeMap<String, String> = [
        ("audit10-g1".to_string(), "950".to_string()),
        ("audit10-g2".to_string(), "9501".to_string()),
        ("audit10-g3".to_string(), "950101".to_string()),
    ]
    .into_iter()
    .collect();
    let groups: Vec<ProductGroup> = rows
        .into_iter()
        .map(|(code, title, parent_id)| ProductGroup {
            code,
            title,
            parent_code: parent_id.map(|parent| by_id[&parent].clone()),
        })
        .collect();

    let tree = build_group_tree(&groups).unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(tree[0].children[0].children.len(), 1);
    assert_eq!(
        group_path(&groups, "950101").as_deref(),
        Some("گروه ممیزی / زیرگروه / زیر زیرگروه")
    );

    // کد گروه تکراری در همان شرکت رد می‌شود
    let duplicate = conn.execute(
        "INSERT INTO product_groups(id, company_id, code, title) \
         VALUES('audit10-g4',?1,'950','تکراری')",
        params![firm],
    );
    assert!(
        duplicate.is_err(),
        "UNIQUE(company_id, code) کد تکراری را رد می‌کند"
    );
}

/// ک۳۸ — خط لوله‌ی ذخیره «یا همه یا هیچ» است: خطا در هر جدول، کل تراکنش را برمی‌گرداند.
#[test]
fn k38_all_or_nothing_transaction() {
    let mut conn = seeded();
    let firm = company(&conn);

    let before_products: i64 = conn
        .query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))
        .unwrap();
    let before_prices: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_prices", [], |r| r.get(0))
        .unwrap();

    {
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO products(id, company_id, kind, sku, name, unit) \
             VALUES('audit10-tx',?1,'simple','TX1','تراکنشی','عدد')",
            params![firm],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO product_prices(product_id, level, price) VALUES('audit10-tx','retail',100)",
            [],
        )
        .unwrap();
        // پله‌ی نامعتبر → کل تراکنش باید برگردد
        let failure = tx.execute(
            "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
             VALUES('audit10-tx-t0','audit10-tx',0,300)",
            [],
        );
        assert!(failure.is_err(), "پله‌ی نامعتبر باید خطا بدهد");
        // بدون commit → drop = rollback
    }

    let after_products: i64 = conn
        .query_row("SELECT COUNT(*) FROM products", [], |r| r.get(0))
        .unwrap();
    let after_prices: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_prices", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after_products, before_products, "کالا نباید جا بماند");
    assert_eq!(after_prices, before_prices, "قیمت نباید جا بماند");
    let leftover: i64 = count(
        &conn,
        "SELECT COUNT(*) FROM products WHERE id=?1",
        "audit10-tx",
    );
    assert_eq!(leftover, 0);
}
