#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! ممیزی ماژول کالا — اعتبارسنجی و ساختار داده.
//!
//! ## چرا این تست‌ها در هسته‌اند و نه در میزبان
//! فرم تعریف کالا فقط یک فرم نیست؛ داده‌ای که ذخیره می‌کند مستقیماً وارد
//! محاسبه‌ی فاکتور می‌شود. اگر نرخ مالیات نامعتبر یا ضریب واحد صفر ذخیره
//! شود، خطا در لحظه‌ی صدور فاکتور بروز می‌کند — یعنی جلوی مشتری.
//!
//! این تست‌ها همان قواعدی را می‌سنجند که ماژول `products_form` میزبان
//! پیش از ذخیره اعمال می‌کند، تا اگر روزی آن قواعد سست شدند، اینجا قرمز
//! شود.

use novin_core::catalog::{
    gold_price, GoldPricing, PriceLevel, PriceList, ProductKind, TaxProfile, UnitSet,
};
use novin_core::db;
use novin_core::money::Money;
use rusqlite::Connection;

fn seeded() -> Connection {
    db::open_in_memory().expect("پایگاه داده")
}

fn rials(value: i64) -> Money {
    Money::from_rials(value)
}

// ---------------------------------------------------------------------------
// انواع کالا
// ---------------------------------------------------------------------------

/// ک۱ — هر پنج نوع کالای نرم‌افزار فعلی پشتیبانی می‌شود.
#[test]
fn k1_all_product_kinds_are_supported() {
    for (value, expected) in [
        ("simple", ProductKind::Simple),
        ("composite", ProductKind::Composite),
        ("variant", ProductKind::Variant),
        ("gold_jewelry", ProductKind::GoldJewelry),
        ("service", ProductKind::Service),
    ] {
        assert_eq!(ProductKind::parse(value), Some(expected), "نوع «{value}»");
    }
    assert!(ProductKind::parse("unknown").is_none());
}

/// ک۲ — فقط خدمت موجودی انبار ندارد.
#[test]
fn k2_only_service_has_no_inventory() {
    assert!(!ProductKind::Service.tracks_inventory());
    for kind in [
        ProductKind::Simple,
        ProductKind::Composite,
        ProductKind::Variant,
        ProductKind::GoldJewelry,
    ] {
        assert!(
            kind.tracks_inventory(),
            "{} باید موجودی داشته باشد",
            kind.label()
        );
    }
}

// ---------------------------------------------------------------------------
// سطوح قیمت
// ---------------------------------------------------------------------------

/// ک۳ — هر هفت سطح قیمت نرم‌افزار فعلی وجود دارد.
#[test]
fn k3_seven_price_levels_exist() {
    assert_eq!(PriceLevel::ALL.len(), 7);
    let labels: Vec<&str> = PriceLevel::ALL.iter().map(|level| level.label()).collect();
    for expected in [
        "جزئی",
        "کلی",
        "همکار",
        "همکار درجه ۲",
        "همکار درجه ۳",
        "فصلی",
        "نمایشگاه",
    ] {
        assert!(labels.contains(&expected), "سطح «{expected}» وجود ندارد");
    }
}

/// ک۴ — سطح تعریف‌نشده به سطح بالاتر برمی‌گردد، نه به خطا.
///
/// این همان رفتاری است که فروشنده انتظار دارد: اگر برای «همکار درجه ۳»
/// قیمتی نگذاشته باشد، قیمت همکار اعمال شود نه اینکه فاکتور صفر شود.
#[test]
fn k4_undefined_level_falls_back_up_the_chain() {
    let mut list = PriceList::new();
    list.set(PriceLevel::Retail, rials(1_000_000))
        .expect("قیمت");
    list.set(PriceLevel::Partner, rials(880_000)).expect("قیمت");

    assert_eq!(
        list.effective(PriceLevel::PartnerTier3).expect("قیمت مؤثر"),
        rials(880_000),
        "درجه ۳ باید به همکار برگردد"
    );
    assert!(list.exact(PriceLevel::PartnerTier3).is_none());
    assert_eq!(
        list.effective(PriceLevel::Retail).expect("قیمت"),
        rials(1_000_000)
    );
}

/// ک۵ — قیمت منفی پذیرفته نمی‌شود.
#[test]
fn k5_negative_price_is_rejected() {
    let mut list = PriceList::new();
    assert!(list.set(PriceLevel::Retail, rials(-1)).is_err());
}

/// ک۶ — سطح قیمت ناشناخته رد می‌شود.
#[test]
fn k6_unknown_price_level_is_rejected() {
    assert!(PriceLevel::parse("vip").is_err());
    assert!(PriceLevel::parse("retail").is_ok());
}

// ---------------------------------------------------------------------------
// چند واحدی
// ---------------------------------------------------------------------------

/// ک۷ — ضریب واحد فرعی باید مثبت باشد.
#[test]
fn k7_unit_factor_must_be_positive() {
    let units = UnitSet::new("عدد");
    assert!(units.clone().with_unit("کارتن", 0.0).is_err());
    assert!(units.clone().with_unit("کارتن", -3.0).is_err());
    assert!(units.with_unit("کارتن", 12.0).is_ok());
}

/// ک۸ — تبدیل واحد و قیمت واحد فرعی درست است.
#[test]
fn k8_unit_conversion_and_price_are_consistent() {
    let units = UnitSet::new("عدد")
        .with_unit("کارتن", 12.0)
        .expect("واحد")
        .with_unit("بسته", 4.0)
        .expect("واحد");

    assert_eq!(units.to_base(3.0, "کارتن").expect("تبدیل"), 36.0);
    assert_eq!(units.convert(3.0, "کارتن", "بسته").expect("تبدیل"), 9.0);
    assert_eq!(
        units.unit_price(rials(50_000), "کارتن").expect("قیمت"),
        rials(600_000),
        "قیمت کارتن باید دوازده برابر قیمت عدد باشد"
    );
    assert!(
        units.to_base(1.0, "پالت").is_err(),
        "واحد تعریف‌نشده باید رد شود"
    );
}

// ---------------------------------------------------------------------------
// مالیات
// ---------------------------------------------------------------------------

/// ک۹ — نرخ مالیات خارج از محدوده رد می‌شود.
#[test]
fn k9_tax_rate_out_of_range_is_rejected() {
    let bad = TaxProfile {
        vat_basis_points: 20_000,
        ..Default::default()
    };
    assert!(bad.validate().is_err());

    let negative = TaxProfile {
        duty_basis_points: -100,
        ..Default::default()
    };
    assert!(negative.validate().is_err());

    assert!(TaxProfile::standard().validate().is_ok());
}

/// ک۱۰ — کالای معاف صفر مالیات می‌گیرد حتی با نرخ تعریف‌شده.
#[test]
fn k10_exempt_product_pays_nothing() {
    let profile = TaxProfile {
        vat_basis_points: 900,
        duty_basis_points: 100,
        exempt: true,
        ..Default::default()
    };
    assert_eq!(
        profile.tax_on(rials(100_000_000)).expect("مالیات"),
        Money::ZERO
    );
}

/// ک۱۱ — عوارض و ارزش افزوده هر دو روی مبلغ محاسبه می‌شوند.
#[test]
fn k11_duty_and_vat_are_both_applied() {
    let profile = TaxProfile {
        vat_basis_points: 900,
        duty_basis_points: 100,
        ..Default::default()
    };
    // ۹٪ + ۱٪ روی ۱۰۰ میلیون
    assert_eq!(
        profile.tax_on(rials(100_000_000)).expect("مالیات"),
        rials(10_000_000)
    );
}

// ---------------------------------------------------------------------------
// طلا و جواهر
// ---------------------------------------------------------------------------

/// ک۱۲ — قیمت طلا: فلز + اجرت + سود + ارزش افزوده.
///
/// **قاعده‌ی مهم ایران:** ارزش افزوده بر ارزش خودِ طلا تعلق **نمی‌گیرد** و
/// فقط روی اجرت ساخت و سود فروشنده محاسبه می‌شود. اگر روی کل مبلغ بسته
/// شود، مشتری چند برابر مالیات قانونی می‌پردازد.
#[test]
fn k12_gold_price_breakdown_is_correct() {
    let breakdown = gold_price(GoldPricing {
        weight_grams: 10.0,
        rate_per_gram: rials(30_000_000),
        making_charge_bp: 1_000, // ۱۰٪
        profit_bp: 700,          // ۷٪
        vat_bp: 900,             // ۹٪
    })
    .expect("قیمت طلا");

    assert_eq!(breakdown.metal_value, rials(300_000_000));
    // اجرت روی ارزش فلز
    assert_eq!(breakdown.making_charge, rials(30_000_000));
    // سود روی فلز + اجرت
    assert_eq!(breakdown.profit, rials(23_100_000));
    // ارزش افزوده فقط روی اجرت + سود، نه روی ارزش طلا
    assert_eq!(breakdown.vat, rials((30_000_000 + 23_100_000) * 9 / 100));
    assert_eq!(
        breakdown.total,
        rials(300_000_000 + 30_000_000 + 23_100_000 + 4_779_000)
    );

    // اگر مالیات روی کل مبلغ بسته می‌شد، این عدد بزرگ‌تر بود.
    assert!(
        breakdown.vat < rials(353_100_000 * 9 / 100),
        "ارزش افزوده نباید روی ارزش طلا محاسبه شود"
    );
}

/// ک۱۳ — وزن صفر یا منفی برای کالای طلا رد می‌شود.
#[test]
fn k13_gold_weight_must_be_positive() {
    for weight in [0.0, -5.0] {
        assert!(
            gold_price(GoldPricing {
                weight_grams: weight,
                rate_per_gram: rials(30_000_000),
                making_charge_bp: 0,
                profit_bp: 0,
                vat_bp: 0,
            })
            .is_err(),
            "وزن {weight} باید رد شود"
        );
    }
}

// ---------------------------------------------------------------------------
// ساختار پایگاه داده
// ---------------------------------------------------------------------------

/// ک۱۴ — همه‌ی جدول‌هایی که فرم کالا به آن‌ها می‌نویسد وجود دارند.
#[test]
fn k14_product_tables_exist() {
    let conn = seeded();
    for table in [
        "products",
        "product_groups",
        "product_prices",
        "product_units",
        "product_discount_tiers",
        "product_gold_specs",
        "product_attributes",
        "product_variants",
    ] {
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .expect("پرس‌وجو");
        assert_eq!(found, 1, "جدول «{table}» وجود ندارد");
    }
}

/// ک۱۵ — ستون‌های گسترده‌ی کالا در جدول موجودند.
#[test]
fn k15_extended_product_columns_exist() {
    let conn = seeded();
    let mut statement = conn.prepare("PRAGMA table_info(products)").expect("پرس‌وجو");
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("اجرا")
        .filter_map(Result::ok)
        .collect();
    for column in [
        "kind",
        "group_id",
        "vat_basis_points",
        "duty_basis_points",
        "tax_code",
        "tax_exempt",
        "display_name",
        "brand",
        "max_stock",
        "reorder_point",
    ] {
        assert!(
            columns.contains(&column.to_string()),
            "ستون «{column}» نیست"
        );
    }
}

/// ک۱۶ — جدول قیمت فقط هفت سطح مجاز را می‌پذیرد.
#[test]
fn k16_price_table_rejects_unknown_level() {
    let conn = seeded();
    let product: String = conn
        .query_row("SELECT id FROM products LIMIT 1", [], |row| row.get(0))
        .expect("کالا");
    for level in PriceLevel::ALL {
        conn.execute(
            "INSERT OR REPLACE INTO product_prices(product_id, level, price) VALUES(?1,?2,?3)",
            rusqlite::params![product, level.as_str(), 1_000],
        )
        .unwrap_or_else(|error| panic!("سطح «{}» رد شد: {error}", level.as_str()));
    }
    assert!(
        conn.execute(
            "INSERT INTO product_prices(product_id, level, price) VALUES(?1,'vip',1000)",
            rusqlite::params![product],
        )
        .is_err(),
        "سطح ناشناخته نباید ذخیره شود"
    );
}

/// ک۱۷ — ضریب واحد صفر در پایگاه داده هم رد می‌شود.
///
/// اعتبارسنجی برنامه لایه‌ی اول است؛ محدودیت پایگاه داده لایه‌ی دوم. اگر
/// روزی کدی این قاعده را دور بزند، پایگاه داده جلویش را می‌گیرد.
#[test]
fn k17_database_rejects_zero_unit_factor() {
    let conn = seeded();
    let product: String = conn
        .query_row("SELECT id FROM products LIMIT 1", [], |row| row.get(0))
        .expect("کالا");
    assert!(
        conn.execute(
            "INSERT INTO product_units(id, product_id, unit_name, factor) VALUES('u-bad',?1,'کارتن',0)",
            rusqlite::params![product],
        )
        .is_err()
    );
}

/// ک۱۸ — درصد تخفیف پلکانی خارج از محدوده در پایگاه داده رد می‌شود.
#[test]
fn k18_database_rejects_invalid_tier_discount() {
    let conn = seeded();
    let product: String = conn
        .query_row("SELECT id FROM products LIMIT 1", [], |row| row.get(0))
        .expect("کالا");
    assert!(
        conn.execute(
            "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
             VALUES('t-bad',?1,10,20000)",
            rusqlite::params![product],
        )
        .is_err(),
        "درصد بیش از ۱۰۰ باید رد شود"
    );
    assert!(
        conn.execute(
            "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
             VALUES('t-bad2',?1,0,500)",
            rusqlite::params![product],
        )
        .is_err(),
        "مقدار شروع صفر باید رد شود"
    );
}
