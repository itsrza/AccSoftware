#![allow(warnings)]
// موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! # تست‌های سخت‌گیرانه‌ی فاز ۳ — کاتالوگ کالا
//!
//! مرجع: تصاویر `NztJl5` (فرم تعریف کالا)، `6FM9Ow` (انتخاب نوع کالا) و
//! `8Xmc1p` (لیست کالاها) — با قیمت‌های واقعی همان لیست.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | انواع کالا | هر چهار نوع + خدمت، با رفتار انبار درست |
//! | ۲ | سطوح قیمت | هفت سطح، ذخیره و بازیابی دقیق |
//! | ۳ | سطوح قیمت | زنجیره‌ی جایگزینی هیچ‌وقت قیمت اشتباه نمی‌دهد |
//! | ۴ | چند واحدی | تبدیل واحد و قیمت واحد فرعی دقیق است |
//! | ۵ | چند واحدی | ضریب نامعتبر و واحد ناشناخته رد می‌شوند |
//! | ۶ | مالیات | ارزش افزوده و عوارض و معافیت درست محاسبه می‌شوند |
//! | ۷ | کالای مرکب | بهای تمام‌شده و کشف ارجاع به خود |
//! | ۸ | کالای تنوع‌دار | ضرب دکارتی ویژگی‌ها با SKU یکتا |
//! | ۹ | طلا و جواهر | فرمول بازار ایران: مالیات فقط بر اجرت و سود |
//! | ۱۰ | پایگاه داده | جدول‌ها، ستون‌ها و داده‌ی پایه‌ی کاتالوگ |

use novin_core::catalog::{
    build_group_tree, composite_cost, expand_variants, gold_price, group_path, CatalogError,
    Component, GoldPricing, PriceLevel, PriceList, ProductGroup, ProductKind, TaxProfile, UnitSet,
    VariantAttribute,
};
use novin_core::money::Money;

// ---------------------------------------------------------------------------
// تست ۱ — انواع کالا
// ---------------------------------------------------------------------------
#[test]
fn t01_product_kinds_match_legacy_dialog() {
    // چهار گزینه‌ی دیالوگ «انتخاب نوع کالا» + خدمت
    assert_eq!(ProductKind::Simple.label(), "کالای عمومی (ساده)");
    assert_eq!(ProductKind::Composite.label(), "کالای مرکب");
    assert_eq!(ProductKind::Variant.label(), "کالای تنوع‌دار");
    assert_eq!(ProductKind::GoldJewelry.label(), "طلا و جواهر");

    for kind in [
        ProductKind::Simple,
        ProductKind::Composite,
        ProductKind::Variant,
        ProductKind::GoldJewelry,
        ProductKind::Service,
    ] {
        assert_eq!(ProductKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(ProductKind::parse("unknown"), None);

    // خدمت موجودی انبار ندارد، بقیه دارند
    assert!(!ProductKind::Service.is_stockable());
    assert!(ProductKind::Simple.is_stockable());
    assert!(ProductKind::GoldJewelry.is_stockable());
}

// ---------------------------------------------------------------------------
// تست ۲ — هفت سطح قیمت
// ---------------------------------------------------------------------------
#[test]
fn t02_seven_price_levels_are_stored_exactly() {
    assert_eq!(PriceLevel::ALL.len(), 7);
    let labels: Vec<&str> = PriceLevel::ALL.iter().map(|level| level.label()).collect();
    assert_eq!(
        labels,
        vec![
            "جزئی",
            "کلی",
            "همکار",
            "همکار درجه ۲",
            "همکار درجه ۳",
            "فصلی",
            "نمایشگاه"
        ]
    );

    // قیمت‌های واقعی «iPhone SE 2022» از لیست کالاها: جزئی ۱۱۱٬۱۱۱ و همکار ۲۲۲٬۲۲۲
    let mut prices = PriceList::new();
    prices
        .set(PriceLevel::Retail, Money::from_rials(111_111))
        .unwrap();
    prices
        .set(PriceLevel::Partner, Money::from_rials(222_222))
        .unwrap();
    assert_eq!(
        prices.exact(PriceLevel::Retail),
        Some(Money::from_rials(111_111))
    );
    assert_eq!(
        prices.exact(PriceLevel::Partner),
        Some(Money::from_rials(222_222))
    );
    assert_eq!(prices.exact(PriceLevel::Exhibition), None);
    assert_eq!(
        prices.defined_levels(),
        vec![PriceLevel::Retail, PriceLevel::Partner]
    );

    // به‌روزرسانی قیمت باید جایگزین شود نه اضافه
    prices
        .set(PriceLevel::Retail, Money::from_rials(120_000))
        .unwrap();
    assert_eq!(
        prices.exact(PriceLevel::Retail),
        Some(Money::from_rials(120_000))
    );
    assert_eq!(prices.defined_levels().len(), 2);

    // قیمت منفی ممنوع
    assert_eq!(
        prices.set(PriceLevel::Wholesale, Money::from_rials(-1)),
        Err(CatalogError::NegativePrice)
    );
    // قیمت صفر مجاز است (کالای رایگان/هدیه)
    assert!(prices.set(PriceLevel::Seasonal, Money::ZERO).is_ok());

    for level in PriceLevel::ALL {
        assert_eq!(PriceLevel::parse(level.as_str()), Ok(level));
    }
    assert_eq!(
        PriceLevel::parse("vip"),
        Err(CatalogError::UnknownPriceLevel)
    );
}

// ---------------------------------------------------------------------------
// تست ۳ — زنجیره‌ی جایگزینی سطح قیمت
// ---------------------------------------------------------------------------
#[test]
fn t03_price_fallback_never_returns_wrong_tier() {
    let mut prices = PriceList::new();
    prices
        .set(PriceLevel::Retail, Money::from_rials(100_000))
        .unwrap();
    prices
        .set(PriceLevel::Wholesale, Money::from_rials(95_000))
        .unwrap();
    prices
        .set(PriceLevel::Partner, Money::from_rials(90_000))
        .unwrap();

    // سطح تعریف‌شده مستقیم برمی‌گردد
    assert_eq!(
        prices.effective(PriceLevel::Partner).unwrap(),
        Money::from_rials(90_000)
    );
    // همکار درجه ۳ تعریف نشده → به همکار می‌رسد، نه جزئی
    assert_eq!(
        prices.effective(PriceLevel::PartnerTier3).unwrap(),
        Money::from_rials(90_000)
    );
    // فصلی تعریف نشده → مستقیم به جزئی (نه به قیمت همکار!)
    assert_eq!(
        prices.effective(PriceLevel::Seasonal).unwrap(),
        Money::from_rials(100_000)
    );
    assert_eq!(
        prices.effective(PriceLevel::Exhibition).unwrap(),
        Money::from_rials(100_000)
    );

    // هیچ سطحی تعریف نشده
    let empty = PriceList::new();
    assert!(empty.is_empty());
    assert_eq!(
        empty.effective(PriceLevel::Retail),
        Err(CatalogError::NoPriceDefined)
    );

    // زنجیره همیشه با خود سطح شروع می‌شود
    for level in PriceLevel::ALL {
        assert_eq!(level.fallback_chain()[0], level);
        assert_eq!(*level.fallback_chain().last().unwrap(), PriceLevel::Retail);
    }
}

// ---------------------------------------------------------------------------
// تست ۴ — تبدیل واحد
// ---------------------------------------------------------------------------
#[test]
fn t04_multi_unit_conversion_is_exact() {
    // «برنجی ایرانی عنبربو» با واحد کیلوگرم از لیست کالاها
    let units = UnitSet::new("کیلوگرم")
        .with_unit("گرم", 0.001)
        .unwrap()
        .with_unit("کیسه", 10.0)
        .unwrap()
        .with_unit("تن", 1000.0)
        .unwrap();

    assert_eq!(units.to_base(3.0, "کیسه").unwrap(), 30.0);
    assert_eq!(units.to_base(500.0, "گرم").unwrap(), 0.5);
    assert_eq!(units.to_base(2.5, "کیلوگرم").unwrap(), 2.5);
    assert_eq!(units.from_base(30.0, "کیسه").unwrap(), 3.0);
    assert_eq!(units.convert(2.0, "تن", "کیسه").unwrap(), 200.0);
    assert_eq!(units.convert(1.0, "کیسه", "گرم").unwrap(), 10_000.0);

    // قیمت واحد فرعی: کیسه = ۱۰ کیلو × ۹۷۰٬۰۰۰ ریال
    let base_price = Money::from_rials(970_000);
    assert_eq!(
        units.unit_price(base_price, "کیسه").unwrap(),
        Money::from_rials(9_700_000)
    );
    assert_eq!(
        units.unit_price(base_price, "گرم").unwrap(),
        Money::from_rials(970)
    );
    assert_eq!(units.unit_price(base_price, "کیلوگرم").unwrap(), base_price);
}

// ---------------------------------------------------------------------------
// تست ۵ — واحد نامعتبر
// ---------------------------------------------------------------------------
#[test]
fn t05_invalid_units_are_rejected() {
    let units = UnitSet::new("عدد").with_unit("کارتن", 12.0).unwrap();

    assert_eq!(
        units.to_base(1.0, "بسته"),
        Err(CatalogError::UnknownUnit {
            unit: "بسته".into()
        })
    );
    assert!(units.unit_price(Money::from_rials(1000), "پالت").is_err());

    // ضریب‌های نامعتبر
    assert_eq!(
        UnitSet::new("عدد").with_unit("خراب", 0.0).err(),
        Some(CatalogError::InvalidUnitFactor)
    );
    assert_eq!(
        UnitSet::new("عدد").with_unit("منفی", -3.0).err(),
        Some(CatalogError::InvalidUnitFactor)
    );
    assert_eq!(
        UnitSet::new("عدد")
            .with_unit("بی‌نهایت", f64::INFINITY)
            .err(),
        Some(CatalogError::InvalidUnitFactor)
    );
    assert_eq!(
        UnitSet::new("عدد").with_unit("نامعین", f64::NAN).err(),
        Some(CatalogError::InvalidUnitFactor)
    );

    // واحد اصلی همیشه ضریب ۱ دارد
    assert_eq!(units.to_base(7.0, "عدد").unwrap(), 7.0);
    assert_eq!(units.to_base(2.0, "کارتن").unwrap(), 24.0);
}

// ---------------------------------------------------------------------------
// تست ۶ — مالیات و عوارض
// ---------------------------------------------------------------------------
#[test]
fn t06_tax_profile_calculations() {
    let standard = TaxProfile::standard();
    assert_eq!(standard.vat_basis_points, 900);
    // ۹٪ روی ۱۲٬۵۰۰٬۰۰۰ = ۱٬۱۲۵٬۰۰۰
    assert_eq!(
        standard.tax_on(Money::from_rials(12_500_000)).unwrap(),
        Money::from_rials(1_125_000)
    );

    // مالیات + عوارض
    let with_duty = TaxProfile {
        vat_basis_points: 900,
        duty_basis_points: 100,
        ..Default::default()
    };
    assert_eq!(
        with_duty.tax_on(Money::from_rials(1_000_000)).unwrap(),
        Money::from_rials(100_000) // ۹٪ + ۱٪
    );

    // کالای معاف
    assert_eq!(
        TaxProfile::exempt()
            .tax_on(Money::from_rials(999_999))
            .unwrap(),
        Money::ZERO
    );

    // گرد کردن روی مبلغ فرد
    assert_eq!(
        standard.tax_on(Money::from_rials(1_234_567)).unwrap(),
        Money::from_rials(111_111)
    );

    // نرخ نامعتبر
    let invalid = TaxProfile {
        vat_basis_points: 20_000,
        ..Default::default()
    };
    assert_eq!(invalid.validate(), Err(CatalogError::InvalidTaxRate));
    assert!(invalid.tax_on(Money::from_rials(1000)).is_err());
    let negative = TaxProfile {
        vat_basis_points: -100,
        ..Default::default()
    };
    assert_eq!(negative.validate(), Err(CatalogError::InvalidTaxRate));
}

// ---------------------------------------------------------------------------
// تست ۷ — کالای مرکب
// ---------------------------------------------------------------------------
#[test]
fn t07_composite_cost_is_precise() {
    let components = vec![
        Component {
            product_id: "prod-1".into(),
            quantity: 2.0,
            unit_cost: Money::from_rials(9_500_000),
        },
        Component {
            product_id: "prod-2".into(),
            quantity: 1.5,
            unit_cost: Money::from_rials(6_500_000),
        },
    ];
    // ۲×۹٬۵۰۰٬۰۰۰ + ۱٫۵×۶٬۵۰۰٬۰۰۰ = ۲۸٬۷۵۰٬۰۰۰
    assert_eq!(
        composite_cost("pack-1", &components).unwrap(),
        Money::from_rials(28_750_000)
    );

    assert_eq!(
        composite_cost("pack-1", &[]),
        Err(CatalogError::EmptyComposite)
    );
    // ارجاع به خود
    assert_eq!(
        composite_cost(
            "pack-1",
            &[Component {
                product_id: "pack-1".into(),
                quantity: 1.0,
                unit_cost: Money::from_rials(100),
            }]
        ),
        Err(CatalogError::SelfReference)
    );
    // مقدار نامعتبر
    for quantity in [0.0, -1.0, f64::NAN] {
        assert_eq!(
            composite_cost(
                "pack-1",
                &[Component {
                    product_id: "prod-1".into(),
                    quantity,
                    unit_cost: Money::from_rials(100),
                }]
            ),
            Err(CatalogError::InvalidComponentQuantity)
        );
    }
}

// ---------------------------------------------------------------------------
// تست ۸ — کالای تنوع‌دار
// ---------------------------------------------------------------------------
#[test]
fn t08_variant_expansion_is_complete_and_unique() {
    // «پیراهن مردانه دو یقه» با رنگ و سایز
    let attributes = vec![
        VariantAttribute {
            name: "رنگ".into(),
            values: vec!["سفید".into(), "آبی".into(), "مشکی".into()],
        },
        VariantAttribute {
            name: "سایز".into(),
            values: vec!["M".into(), "L".into()],
        },
    ];
    let variants = expand_variants("SHIRT-44", &attributes).unwrap();

    assert_eq!(variants.len(), 6, "۳ رنگ × ۲ سایز");
    assert_eq!(variants[0].values, vec!["سفید", "M"]);
    assert_eq!(variants[0].sku, "SHIRT-44-001");
    assert_eq!(variants[5].values, vec!["مشکی", "L"]);
    assert_eq!(variants[5].sku, "SHIRT-44-006");

    // همه‌ی SKUها یکتا هستند
    let mut skus: Vec<&str> = variants
        .iter()
        .map(|variant| variant.sku.as_str())
        .collect();
    skus.sort_unstable();
    skus.dedup();
    assert_eq!(skus.len(), 6);

    // همه‌ی ترکیب‌ها یکتا هستند
    let mut combos: Vec<String> = variants
        .iter()
        .map(|variant| variant.values.join("|"))
        .collect();
    combos.sort();
    combos.dedup();
    assert_eq!(combos.len(), 6);

    // تک‌ویژگی
    let single = expand_variants(
        "RING",
        &[VariantAttribute {
            name: "عیار".into(),
            values: vec!["18".into(), "24".into()],
        }],
    )
    .unwrap();
    assert_eq!(single.len(), 2);

    // ورودی نامعتبر
    assert_eq!(
        expand_variants("X", &[]),
        Err(CatalogError::EmptyVariantAttributes)
    );
    assert_eq!(
        expand_variants(
            "X",
            &[VariantAttribute {
                name: "رنگ".into(),
                values: vec![],
            }]
        ),
        Err(CatalogError::EmptyVariantAttributes)
    );
}

// ---------------------------------------------------------------------------
// تست ۹ — قیمت‌گذاری طلا و جواهر
// ---------------------------------------------------------------------------
#[test]
fn t09_gold_pricing_follows_iranian_market_rules() {
    // ۵ گرم طلا، نرخ ۳۰٬۰۰۰٬۰۰۰ ریال بر گرم، اجرت ۱۰٪، سود ۷٪، ارزش افزوده ۹٪
    let breakdown = gold_price(GoldPricing {
        weight_grams: 5.0,
        rate_per_gram: Money::from_rials(30_000_000),
        making_charge_bp: 1_000,
        profit_bp: 700,
        vat_bp: 900,
    })
    .unwrap();

    assert_eq!(breakdown.metal_value, Money::from_rials(150_000_000));
    assert_eq!(breakdown.making_charge, Money::from_rials(15_000_000));
    // سود روی (ارزش طلا + اجرت) = ۷٪ × ۱۶۵٬۰۰۰٬۰۰۰
    assert_eq!(breakdown.profit, Money::from_rials(11_550_000));
    // مالیات فقط روی اجرت و سود = ۹٪ × ۲۶٬۵۵۰٬۰۰۰
    assert_eq!(breakdown.vat, Money::from_rials(2_389_500));
    assert_eq!(breakdown.total, Money::from_rials(178_939_500));

    // قاعده‌ی کلیدی: مالیات نباید روی ارزش خود طلا محاسبه شود
    let naive_vat = Money::from_rials(178_939_500 - 2_389_500)
        .percent_bp(900)
        .unwrap();
    assert!(
        breakdown.vat < naive_vat,
        "ارزش افزوده نباید بر ارزش طلا تعلق بگیرد"
    );

    // جمع اجزا باید دقیقاً برابر کل باشد
    assert_eq!(
        breakdown.metal_value + breakdown.making_charge + breakdown.profit + breakdown.vat,
        breakdown.total
    );

    // بدون اجرت و سود، مالیاتی هم نیست
    let plain = gold_price(GoldPricing {
        weight_grams: 1.0,
        rate_per_gram: Money::from_rials(30_000_000),
        making_charge_bp: 0,
        profit_bp: 0,
        vat_bp: 900,
    })
    .unwrap();
    assert_eq!(plain.vat, Money::ZERO);
    assert_eq!(plain.total, Money::from_rials(30_000_000));

    // ورودی نامعتبر
    for weight in [0.0, -1.0, f64::NAN] {
        assert_eq!(
            gold_price(GoldPricing {
                weight_grams: weight,
                rate_per_gram: Money::from_rials(1),
                making_charge_bp: 0,
                profit_bp: 0,
                vat_bp: 0,
            }),
            Err(CatalogError::InvalidWeight)
        );
    }
    assert_eq!(
        gold_price(GoldPricing {
            weight_grams: 1.0,
            rate_per_gram: Money::from_rials(1),
            making_charge_bp: 50_000,
            profit_bp: 0,
            vat_bp: 0,
        }),
        Err(CatalogError::InvalidTaxRate)
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — درخت گروه کالا و پایگاه داده
// ---------------------------------------------------------------------------
#[test]
fn t10_product_groups_and_catalog_schema() {
    // درخت گروه‌ها مطابق لیست کالاهای نرم‌افزار فعلی
    let groups = vec![
        ProductGroup {
            code: "1".into(),
            title: "مواد غذایی".into(),
            parent_code: None,
        },
        ProductGroup {
            code: "106".into(),
            title: "رستورانی".into(),
            parent_code: Some("1".into()),
        },
        ProductGroup {
            code: "2".into(),
            title: "کالاهای متفرقه".into(),
            parent_code: None,
        },
    ];
    let tree = build_group_tree(&groups).unwrap();
    assert_eq!(tree.len(), 2, "دو گروه ریشه");
    let food = tree.iter().find(|node| node.group.code == "1").unwrap();
    assert_eq!(food.children.len(), 1);
    assert_eq!(food.children[0].group.title, "رستورانی");
    assert_eq!(
        group_path(&groups, "106").as_deref(),
        Some("مواد غذایی / رستورانی")
    );
    assert_eq!(group_path(&groups, "404"), None);

    // خطاهای ساختاری
    assert_eq!(
        build_group_tree(&[ProductGroup {
            code: "9".into(),
            title: "یتیم".into(),
            parent_code: Some("404".into()),
        }]),
        Err(CatalogError::MissingParentGroup {
            parent: "404".into()
        })
    );
    assert!(matches!(
        build_group_tree(&[
            ProductGroup {
                code: "1".into(),
                title: "الف".into(),
                parent_code: None
            },
            ProductGroup {
                code: "1".into(),
                title: "ب".into(),
                parent_code: None
            },
        ]),
        Err(CatalogError::DuplicateGroupCode { .. })
    ));

    // --- پایگاه داده ---
    let conn = novin_core::db::open_in_memory().unwrap();
    for table in [
        "product_groups",
        "product_prices",
        "product_units",
        "product_components",
        "product_attributes",
        "product_variants",
        "product_gold_specs",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "جدول {table} ساخته نشده است");
    }

    let columns: Vec<String> = {
        let mut statement = conn.prepare("PRAGMA table_info(products)").unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    for column in [
        "kind",
        "group_id",
        "vat_basis_points",
        "tax_code",
        "reorder_point",
    ] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }

    // داده‌ی پایه: گروه‌ها و سطوح قیمت کالاهای نمونه
    let group_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_groups", [], |row| row.get(0))
        .unwrap();
    assert!(group_count >= 5, "درخت گروه کالا مقداردهی نشده");

    let retail_prices: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM product_prices WHERE level='retail'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(retail_prices >= 1, "قیمت جزئی کالاهای نمونه ثبت نشده");

    // سطح قیمت نامعتبر باید توسط خود پایگاه داده رد شود
    let invalid = conn.execute(
        "INSERT INTO product_prices(product_id,level,price) \
         SELECT id,'vip',1 FROM products LIMIT 1",
        [],
    );
    assert!(invalid.is_err(), "CHECK سطح قیمت کار نمی‌کند");

    // مهاجرت idempotent
    novin_core::db::migrate(&conn).unwrap();
    let group_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM product_groups", [], |row| row.get(0))
        .unwrap();
    assert_eq!(group_count, group_count_after);
}
