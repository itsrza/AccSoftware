//! فرم کامل تعریف کالا — چند زبانه.
//!
//! مرجع: تصویر `NztJl5` (فرم تعریف کالا)، `6FM9Ow` (انتخاب نوع کالا) و
//! `8Xmc1p` (لیست کالاها).
//!
//! ## زبانه‌ها
//!
//! ۱. مشخصات عمومی — نوع کالا، کد، نام، نام نمایشی، برند، گروه، واحد اصلی
//! ۲. سطوح قیمت — هفت سطح جزئی تا نمایشگاه
//! ۳. چند واحدی — واحدهای فرعی با ضریب تبدیل به واحد اصلی
//! ۴. اطلاعات مالیاتی — ارزش افزوده، عوارض، معافیت، کد سامانه مؤدیان
//! ۵. موجودی — حداقل، حداکثر و نقطه‌ی سفارش
//! ۶. تخفیف پلکانی — از چه مقداری به بالا، چند درصد
//! ۷. طلا و جواهر — وزن، عیار، اجرت و سود (فقط برای کالای طلا)
//!
//! ## چرا یک تراکنش
//!
//! کالایی که قیمت‌هایش ذخیره شده ولی مالیاتش نه، در فاکتور عدد غلط تولید
//! می‌کند. کل فرم در یک تراکنش می‌نشیند: یا همه یا هیچ.
//!
//! ## چرا اعتبارسنجی در هسته
//!
//! نرخ مالیات، ضریب واحد و قیمت منفی همگی با همان توابعی بررسی می‌شوند که
//! موتور فاکتور استفاده می‌کند (`novin_core::catalog`) — تا داده‌ای ذخیره
//! نشود که بعداً موتور آن را رد کند.

use novin_core::catalog::{PriceLevel, ProductKind, TaxProfile, UnitSet};
use novin_core::money::Money;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{active_context, audit, conn, require_permission, AppState};

// ---------------------------------------------------------------------------
// ورودی
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PriceLevelInput {
    pub level: String,
    /// خالی یعنی این سطح تعریف نشده و از زنجیره‌ی جایگزینی استفاده می‌شود.
    #[serde(default)]
    pub price: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnitInput {
    pub unit_name: String,
    pub factor: f64,
    #[serde(default)]
    pub is_default_sale: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TierInput {
    pub min_quantity: f64,
    pub discount_bp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoldInput {
    pub weight_grams: f64,
    pub carat: i64,
    pub making_charge_bp: i64,
    pub profit_bp: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductInput {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: String,
    pub sku: String,
    #[serde(default)]
    pub barcode: Option<String>,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub unit: String,
    #[serde(default)]
    pub purchase_price: i64,
    #[serde(default)]
    pub min_stock: f64,
    #[serde(default)]
    pub max_stock: f64,
    #[serde(default)]
    pub reorder_point: f64,
    #[serde(default)]
    pub vat_basis_points: i64,
    #[serde(default)]
    pub duty_basis_points: i64,
    #[serde(default)]
    pub tax_code: Option<String>,
    #[serde(default)]
    pub tax_exempt: bool,
    #[serde(default)]
    pub prices: Vec<PriceLevelInput>,
    #[serde(default)]
    pub units: Vec<UnitInput>,
    #[serde(default)]
    pub tiers: Vec<TierInput>,
    #[serde(default)]
    pub gold: Option<GoldInput>,
}

// ---------------------------------------------------------------------------
// خروجی
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PriceLevelRow {
    pub level: String,
    pub label: String,
    pub price: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct UnitRow {
    pub unit_name: String,
    pub factor: f64,
    pub is_default_sale: bool,
}

#[derive(Debug, Serialize)]
pub struct TierRow {
    pub min_quantity: f64,
    pub discount_bp: i64,
}

#[derive(Debug, Serialize)]
pub struct StockRow {
    pub warehouse_id: String,
    pub warehouse_name: String,
    pub quantity: f64,
}

#[derive(Debug, Serialize)]
pub struct ProductDetail {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub display_name: Option<String>,
    pub brand: Option<String>,
    pub group_id: Option<String>,
    pub group_title: Option<String>,
    pub unit: String,
    pub sale_price: i64,
    pub purchase_price: i64,
    pub min_stock: f64,
    pub max_stock: f64,
    pub reorder_point: f64,
    pub vat_basis_points: i64,
    pub duty_basis_points: i64,
    pub tax_code: Option<String>,
    pub tax_exempt: bool,
    pub prices: Vec<PriceLevelRow>,
    pub units: Vec<UnitRow>,
    pub tiers: Vec<TierRow>,
    pub gold: Option<GoldInput>,
    pub stock: Vec<StockRow>,
    pub total_stock: f64,
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

/// فهرست انواع کالا برای دیالوگ «انتخاب نوع کالا».
#[tauri::command]
pub fn list_product_kinds() -> serde_json::Value {
    let kinds = [
        ProductKind::Simple,
        ProductKind::Composite,
        ProductKind::Variant,
        ProductKind::GoldJewelry,
        ProductKind::Service,
    ];
    serde_json::json!({
        "kinds": kinds
            .iter()
            .map(|kind| serde_json::json!({
                "value": kind.as_str(),
                "label": kind.label(),
                "tracks_inventory": kind.tracks_inventory(),
            }))
            .collect::<Vec<_>>(),
        "levels": PriceLevel::ALL
            .iter()
            .map(|level| serde_json::json!({
                "value": level.as_str(),
                "label": level.label(),
            }))
            .collect::<Vec<_>>(),
    })
}

/// خواندن پروفایل کامل یک کالا.
#[tauri::command]
pub fn get_product_profile(state: State<AppState>, id: String) -> Result<ProductDetail, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.create")?;

    let mut detail = c
        .query_row(
            "SELECT p.id, COALESCE(p.kind,'simple'), p.sku, p.barcode, p.name, p.display_name, \
                    p.brand, p.group_id, g.title, p.unit, p.sale_price, p.purchase_price, \
                    p.min_stock, p.max_stock, p.reorder_point, p.vat_basis_points, \
                    p.duty_basis_points, p.tax_code, p.tax_exempt \
             FROM products p LEFT JOIN product_groups g ON g.id = p.group_id WHERE p.id=?1",
            params![id],
            |row| {
                let kind: String = row.get(1)?;
                Ok(ProductDetail {
                    id: row.get(0)?,
                    kind_label: ProductKind::parse(&kind)
                        .map(|value| value.label().to_string())
                        .unwrap_or_else(|| kind.clone()),
                    kind,
                    sku: row.get(2)?,
                    barcode: row.get(3)?,
                    name: row.get(4)?,
                    display_name: row.get(5)?,
                    brand: row.get(6)?,
                    group_id: row.get(7)?,
                    group_title: row.get(8)?,
                    unit: row.get(9)?,
                    sale_price: row.get(10)?,
                    purchase_price: row.get(11)?,
                    min_stock: row.get(12)?,
                    max_stock: row.get(13)?,
                    reorder_point: row.get(14)?,
                    vat_basis_points: row.get(15)?,
                    duty_basis_points: row.get(16)?,
                    tax_code: row.get(17)?,
                    tax_exempt: row.get::<_, i64>(18)? == 1,
                    prices: Vec::new(),
                    units: Vec::new(),
                    tiers: Vec::new(),
                    gold: None,
                    stock: Vec::new(),
                    total_stock: 0.0,
                })
            },
        )
        .map_err(|_| "ITM-001: کالا یافت نشد".to_string())?;

    // --- سطوح قیمت: همیشه هر هفت سطح برگردانده می‌شود، حتی خالی ---
    let mut statement = c
        .prepare("SELECT level, price FROM product_prices WHERE product_id=?1")
        .map_err(|e| e.to_string())?;
    let stored: Vec<(String, i64)> = statement
        .query_map(params![detail.id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    detail.prices = PriceLevel::ALL
        .iter()
        .map(|level| PriceLevelRow {
            level: level.as_str().to_string(),
            label: level.label().to_string(),
            price: stored
                .iter()
                .find(|(key, _)| key == level.as_str())
                .map(|(_, value)| *value),
        })
        .collect();

    let mut statement = c
        .prepare("SELECT unit_name, factor, is_default_sale FROM product_units WHERE product_id=?1 ORDER BY factor")
        .map_err(|e| e.to_string())?;
    detail.units = statement
        .query_map(params![detail.id], |row| {
            Ok(UnitRow {
                unit_name: row.get(0)?,
                factor: row.get(1)?,
                is_default_sale: row.get::<_, i64>(2)? == 1,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let mut statement = c
        .prepare("SELECT min_quantity, discount_bp FROM product_discount_tiers WHERE product_id=?1 ORDER BY min_quantity")
        .map_err(|e| e.to_string())?;
    detail.tiers = statement
        .query_map(params![detail.id], |row| {
            Ok(TierRow {
                min_quantity: row.get(0)?,
                discount_bp: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    detail.gold = c
        .query_row(
            "SELECT weight_grams, carat, making_charge_bp, profit_bp FROM product_gold_specs WHERE product_id=?1",
            params![detail.id],
            |row| {
                Ok(GoldInput {
                    weight_grams: row.get(0)?,
                    carat: row.get(1)?,
                    making_charge_bp: row.get(2)?,
                    profit_bp: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    // --- موجودی به تفکیک انبار (پنل لیست کالاها در تصویر مرجع) ---
    let mut statement = c
        .prepare(
            "SELECT w.id, w.name, COALESCE(b.quantity,0) FROM warehouses w \
             LEFT JOIN inventory_balances b ON b.warehouse_id=w.id AND b.product_id=?1 \
             WHERE w.is_active=1 ORDER BY w.code",
        )
        .map_err(|e| e.to_string())?;
    detail.stock = statement
        .query_map(params![detail.id], |row| {
            Ok(StockRow {
                warehouse_id: row.get(0)?,
                warehouse_name: row.get(1)?,
                quantity: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    detail.total_stock = detail.stock.iter().map(|row| row.quantity).sum();

    Ok(detail)
}

/// اعتبارسنجی ورودی با همان قواعدی که موتور فاکتور دارد.
fn validate(input: &ProductInput) -> Result<ProductKind, String> {
    let kind = ProductKind::parse(&input.kind).ok_or("ITM-002: نوع کالا نامعتبر است")?;

    if input.sku.trim().is_empty() {
        return Err("ITM-003: کد کالا نمی‌تواند خالی باشد".into());
    }
    if input.name.trim().is_empty() {
        return Err("ITM-004: نام کالا نمی‌تواند خالی باشد".into());
    }
    if input.unit.trim().is_empty() {
        return Err("ITM-005: واحد اصلی کالا نمی‌تواند خالی باشد".into());
    }
    if input.purchase_price < 0 {
        return Err("ITM-006: قیمت خرید نمی‌تواند منفی باشد".into());
    }

    // مالیات با همان پروفایلی سنجیده می‌شود که فاکتور استفاده می‌کند.
    let profile = TaxProfile {
        vat_basis_points: input.vat_basis_points,
        duty_basis_points: input.duty_basis_points,
        tax_code: clean(&input.tax_code),
        tax_title: None,
        exempt: input.tax_exempt,
    };
    profile
        .validate()
        .map_err(|error| format!("ITM-007: {error}"))?;

    for level in &input.prices {
        PriceLevel::parse(&level.level).map_err(|error| format!("ITM-008: {error}"))?;
        if let Some(price) = level.price {
            if price < 0 {
                return Err("ITM-009: قیمت سطح نمی‌تواند منفی باشد".into());
            }
        }
    }

    // ضریب واحدهای فرعی با همان ساختار هسته ساخته می‌شود تا ضریب صفر یا
    // منفی همین‌جا رد شود، نه هنگام محاسبه‌ی فاکتور.
    let mut units = UnitSet::new(input.unit.trim());
    for unit in &input.units {
        if unit.unit_name.trim().is_empty() {
            return Err("ITM-010: نام واحد فرعی نمی‌تواند خالی باشد".into());
        }
        if unit.unit_name.trim() == input.unit.trim() {
            return Err("ITM-011: واحد فرعی نمی‌تواند هم‌نام واحد اصلی باشد".into());
        }
        units = units
            .with_unit(unit.unit_name.trim(), unit.factor)
            .map_err(|error| format!("ITM-012: {error}"))?;
    }
    if input
        .units
        .iter()
        .filter(|unit| unit.is_default_sale)
        .count()
        > 1
    {
        return Err("ITM-013: فقط یک واحد می‌تواند واحد پیش‌فرض فروش باشد".into());
    }

    for tier in &input.tiers {
        if tier.min_quantity <= 0.0 {
            return Err("ITM-014: مقدار شروع پله‌ی تخفیف باید بیشتر از صفر باشد".into());
        }
        if !(0..=10_000).contains(&tier.discount_bp) {
            return Err("ITM-015: درصد تخفیف پلکانی نامعتبر است".into());
        }
    }

    if kind == ProductKind::GoldJewelry {
        let gold = input
            .gold
            .as_ref()
            .ok_or("ITM-016: برای کالای طلا، وزن و عیار الزامی است")?;
        if gold.weight_grams <= 0.0 {
            return Err("ITM-017: وزن کالای طلا باید بیشتر از صفر باشد".into());
        }
        if !(1..=24).contains(&gold.carat) {
            return Err("ITM-018: عیار طلا باید بین ۱ تا ۲۴ باشد".into());
        }
    }

    Ok(kind)
}

/// ثبت یا ویرایش کالا با همه‌ی زبانه‌ها در یک تراکنش.
#[tauri::command]
pub fn save_product_profile(state: State<AppState>, input: ProductInput) -> Result<String, String> {
    let kind = validate(&input)?;

    let mut c = conn(&state)?;
    let permission = if input.id.is_some() {
        "products.edit"
    } else {
        "products.create"
    };
    let user = require_permission(&state, &c, permission)?;

    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _fiscal) = active_context(&tx, &user)?;

    // قیمت فروش پایه همان سطح «جزئی» است؛ اگر تعریف نشده باشد صفر می‌ماند.
    let retail = input
        .prices
        .iter()
        .find(|level| level.level == PriceLevel::Retail.as_str())
        .and_then(|level| level.price)
        .unwrap_or(0);

    let id = match &input.id {
        Some(existing) => {
            tx.execute(
                "UPDATE products SET kind=?1, sku=?2, barcode=?3, name=?4, display_name=?5, \
                 brand=?6, group_id=?7, unit=?8, sale_price=?9, purchase_price=?10, \
                 min_stock=?11, max_stock=?12, reorder_point=?13, vat_basis_points=?14, \
                 duty_basis_points=?15, tax_code=?16, tax_exempt=?17, is_service=?18 \
                 WHERE id=?19 AND company_id=?20",
                params![
                    kind.as_str(),
                    input.sku.trim(),
                    clean(&input.barcode),
                    input.name.trim(),
                    clean(&input.display_name),
                    clean(&input.brand),
                    clean(&input.group_id),
                    input.unit.trim(),
                    retail,
                    input.purchase_price,
                    input.min_stock,
                    input.max_stock,
                    input.reorder_point,
                    input.vat_basis_points,
                    input.duty_basis_points,
                    clean(&input.tax_code),
                    i64::from(input.tax_exempt),
                    i64::from(!kind.tracks_inventory()),
                    existing,
                    company
                ],
            )
            .map_err(|e| format!("ITM-019: ذخیره‌ی کالا انجام نشد: {e}"))?;
            existing.clone()
        }
        None => {
            let new_id = format!(
                "product-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO products(id, company_id, kind, sku, barcode, name, display_name, \
                 brand, group_id, unit, sale_price, purchase_price, min_stock, max_stock, \
                 reorder_point, vat_basis_points, duty_basis_points, tax_code, tax_exempt, is_service) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
                params![
                    new_id,
                    company,
                    kind.as_str(),
                    input.sku.trim(),
                    clean(&input.barcode),
                    input.name.trim(),
                    clean(&input.display_name),
                    clean(&input.brand),
                    clean(&input.group_id),
                    input.unit.trim(),
                    retail,
                    input.purchase_price,
                    input.min_stock,
                    input.max_stock,
                    input.reorder_point,
                    input.vat_basis_points,
                    input.duty_basis_points,
                    clean(&input.tax_code),
                    i64::from(input.tax_exempt),
                    i64::from(!kind.tracks_inventory())
                ],
            )
            .map_err(|e| format!("ITM-020: کد کالا تکراری است یا ثبت انجام نشد: {e}"))?;
            new_id
        }
    };

    // --- سطوح قیمت: پاک و دوباره‌نویسی، چون خالی کردن یک سطح هم معنادار است ---
    tx.execute(
        "DELETE FROM product_prices WHERE product_id=?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    for level in &input.prices {
        if let Some(price) = level.price {
            tx.execute(
                "INSERT INTO product_prices(product_id, level, price) VALUES(?1,?2,?3)",
                params![id, level.level, price],
            )
            .map_err(|e| format!("ITM-021: ذخیره‌ی سطح قیمت انجام نشد: {e}"))?;
        }
    }

    tx.execute("DELETE FROM product_units WHERE product_id=?1", params![id])
        .map_err(|e| e.to_string())?;
    for (index, unit) in input.units.iter().enumerate() {
        tx.execute(
            "INSERT INTO product_units(id, product_id, unit_name, factor, is_default_sale) \
             VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{id}-u{index}"),
                id,
                unit.unit_name.trim(),
                unit.factor,
                i64::from(unit.is_default_sale)
            ],
        )
        .map_err(|e| format!("ITM-022: ذخیره‌ی واحد فرعی انجام نشد: {e}"))?;
    }

    tx.execute(
        "DELETE FROM product_discount_tiers WHERE product_id=?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    for (index, tier) in input.tiers.iter().enumerate() {
        tx.execute(
            "INSERT INTO product_discount_tiers(id, product_id, min_quantity, discount_bp) \
             VALUES(?1,?2,?3,?4)",
            params![
                format!("{id}-t{index}"),
                id,
                tier.min_quantity,
                tier.discount_bp
            ],
        )
        .map_err(|e| format!("ITM-023: ذخیره‌ی پله‌ی تخفیف انجام نشد: {e}"))?;
    }

    tx.execute(
        "DELETE FROM product_gold_specs WHERE product_id=?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    if let Some(gold) = &input.gold {
        if kind == ProductKind::GoldJewelry {
            tx.execute(
                "INSERT INTO product_gold_specs(product_id, weight_grams, carat, making_charge_bp, profit_bp) \
                 VALUES(?1,?2,?3,?4,?5)",
                params![id, gold.weight_grams, gold.carat, gold.making_charge_bp, gold.profit_bp],
            )
            .map_err(|e| format!("ITM-024: ذخیره‌ی مشخصات طلا انجام نشد: {e}"))?;
        }
    }

    audit(
        &tx,
        &user,
        if input.id.is_some() {
            "update"
        } else {
            "create"
        },
        "product",
        &id,
        None,
        Some(&format!(
            "{} — {} ({})",
            input.sku.trim(),
            input.name.trim(),
            kind.label()
        )),
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

/// فهرست کالاها برای صفحه‌ی لیست — با گروه، موجودی و دو سطح قیمت پرکاربرد.
///
/// مرجع ستون‌ها: تصویر `8Xmc1p` — کد، نام، موجودی، واحد، گروه، جزئی، همکار.
#[derive(Debug, Serialize)]
pub struct ProductListRow {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub unit: String,
    pub group_title: Option<String>,
    pub quantity: f64,
    pub retail_price: i64,
    pub partner_price: i64,
    pub purchase_price: i64,
    pub min_stock: f64,
    pub vat_basis_points: i64,
    pub tax_exempt: bool,
}

#[tauri::command]
pub fn list_products_detailed(state: State<AppState>) -> Result<Vec<ProductListRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.create")?;
    let mut statement = c
        .prepare(
            "SELECT p.id, COALESCE(p.kind,'simple'), p.sku, p.barcode, p.name, p.unit, g.title, \
                    COALESCE((SELECT SUM(b.quantity) FROM inventory_balances b WHERE b.product_id=p.id),0), \
                    COALESCE((SELECT price FROM product_prices pp WHERE pp.product_id=p.id AND pp.level='retail'), p.sale_price), \
                    COALESCE((SELECT price FROM product_prices pp WHERE pp.product_id=p.id AND pp.level='partner'), 0), \
                    p.purchase_price, p.min_stock, p.vat_basis_points, p.tax_exempt \
             FROM products p \
             LEFT JOIN product_groups g ON g.id = p.group_id \
             ORDER BY p.sku",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let kind: String = row.get(1)?;
            Ok(ProductListRow {
                id: row.get(0)?,
                kind_label: ProductKind::parse(&kind)
                    .map(|value| value.label().to_string())
                    .unwrap_or_else(|| kind.clone()),
                kind,
                sku: row.get(2)?,
                barcode: row.get(3)?,
                name: row.get(4)?,
                unit: row.get(5)?,
                group_title: row.get(6)?,
                quantity: row.get(7)?,
                retail_price: row.get(8)?,
                partner_price: row.get(9)?,
                purchase_price: row.get(10)?,
                min_stock: row.get(11)?,
                vat_basis_points: row.get(12)?,
                tax_exempt: row.get::<_, i64>(13)? == 1,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// محاسبه‌ی قیمت پیشنهادی کالای طلا با نرخ روز.
///
/// چرا اینجا و نه در رابط کاربری: فرمول قیمت طلا (وزن × نرخ + اجرت + سود +
/// ارزش افزوده) بخشی از منطق مالی است و باید همان‌جایی محاسبه شود که موتور
/// فاکتور از آن استفاده می‌کند.
#[tauri::command]
pub fn preview_gold_price(
    state: State<AppState>,
    id: String,
    rate_per_gram: i64,
) -> Result<serde_json::Value, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.create")?;
    let (weight, making, profit, vat): (f64, i64, i64, i64) = c
        .query_row(
            "SELECT s.weight_grams, s.making_charge_bp, s.profit_bp, p.vat_basis_points \
             FROM product_gold_specs s JOIN products p ON p.id = s.product_id WHERE s.product_id=?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| "ITM-025: مشخصات طلای این کالا ثبت نشده است".to_string())?;

    let breakdown = novin_core::catalog::gold_price(novin_core::catalog::GoldPricing {
        weight_grams: weight,
        rate_per_gram: Money::from_rials(rate_per_gram),
        making_charge_bp: making,
        profit_bp: profit,
        vat_bp: vat,
    })
    .map_err(|error| format!("ITM-026: {error}"))?;

    Ok(serde_json::json!({
        "metal_value": breakdown.metal_value.rials(),
        "making_charge": breakdown.making_charge.rials(),
        "profit": breakdown.profit.rials(),
        "vat": breakdown.vat.rials(),
        "total": breakdown.total.rials(),
    }))
}
