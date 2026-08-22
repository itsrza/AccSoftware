//! کاتالوگ کالا و خدمات: انواع کالا، سطوح قیمت، چند واحدی، مالیات و کالای مرکب.
//!
//! مرجع: تصاویر `NztJl5` (فرم تعریف کالا)، `6FM9Ow` (انتخاب نوع کالا) و
//! `8Xmc1p` (لیست کالاها) از نرم‌افزار فعلی — `docs/FEATURE_BASELINE.md` بخش ۳.

use crate::money::{Money, MoneyError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// خطاهای دامنه‌ی کاتالوگ.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CatalogError {
    #[error("CAT-001: سطح قیمت نامعتبر است")]
    UnknownPriceLevel,
    #[error("CAT-002: قیمت نمی‌تواند منفی باشد")]
    NegativePrice,
    #[error("CAT-003: برای این کالا هیچ قیمتی تعریف نشده است")]
    NoPriceDefined,
    #[error("CAT-004: ضریب تبدیل واحد باید بزرگ‌تر از صفر باشد")]
    InvalidUnitFactor,
    #[error("CAT-005: واحد «{unit}» برای این کالا تعریف نشده است")]
    UnknownUnit { unit: String },
    #[error("CAT-006: کالای مرکب باید حداقل یک جزء داشته باشد")]
    EmptyComposite,
    #[error("CAT-007: کالای مرکب نمی‌تواند جزء خودش باشد")]
    SelfReference,
    #[error("CAT-008: مقدار جزء کالای مرکب باید بزرگ‌تر از صفر باشد")]
    InvalidComponentQuantity,
    #[error("CAT-009: کالای تنوع‌دار باید حداقل یک ویژگی داشته باشد")]
    EmptyVariantAttributes,
    #[error("CAT-010: وزن کالای طلا باید بزرگ‌تر از صفر باشد")]
    InvalidWeight,
    #[error("CAT-011: نرخ مالیات نامعتبر است")]
    InvalidTaxRate,
    #[error("CAT-012: گروه کالای والد یافت نشد: {parent}")]
    MissingParentGroup { parent: String },
    #[error("CAT-013: کد گروه کالا تکراری است: {code}")]
    DuplicateGroupCode { code: String },
    #[error("CAT-014: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
}

// ---------------------------------------------------------------------------
// نوع کالا
// ---------------------------------------------------------------------------

/// چهار نوع کالا مطابق دیالوگ «انتخاب نوع کالا» نرم‌افزار فعلی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductKind {
    /// کالای عمومی (ساده) — F1
    Simple,
    /// کالای مرکب — F2
    Composite,
    /// کالای تنوع‌دار — F3
    Variant,
    /// طلا و جواهر — F4
    GoldJewelry,
    /// خدمت (بدون موجودی انبار)
    Service,
}

impl ProductKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProductKind::Simple => "simple",
            ProductKind::Composite => "composite",
            ProductKind::Variant => "variant",
            ProductKind::GoldJewelry => "gold_jewelry",
            ProductKind::Service => "service",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProductKind::Simple => "کالای عمومی (ساده)",
            ProductKind::Composite => "کالای مرکب",
            ProductKind::Variant => "کالای تنوع‌دار",
            ProductKind::GoldJewelry => "طلا و جواهر",
            ProductKind::Service => "خدمت",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "simple" => ProductKind::Simple,
            "composite" => ProductKind::Composite,
            "variant" => ProductKind::Variant,
            "gold_jewelry" => ProductKind::GoldJewelry,
            "service" => ProductKind::Service,
            _ => return None,
        })
    }

    /// آیا این نوع کالا در انبار موجودی دارد؟
    pub fn is_stockable(self) -> bool {
        !matches!(self, ProductKind::Service)
    }
}

// ---------------------------------------------------------------------------
// سطوح قیمت
// ---------------------------------------------------------------------------

/// هفت سطح قیمت مطابق بخش «اطلاعات قیمت‌ها» فرم تعریف کالا.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceLevel {
    /// جزئی
    Retail,
    /// کلی
    Wholesale,
    /// همکار
    Partner,
    /// همکار درجه ۲
    PartnerTier2,
    /// همکار درجه ۳
    PartnerTier3,
    /// فصلی
    Seasonal,
    /// نمایشگاه
    Exhibition,
}

impl PriceLevel {
    /// همه‌ی سطوح به ترتیب نمایش در فرم.
    pub const ALL: [PriceLevel; 7] = [
        PriceLevel::Retail,
        PriceLevel::Wholesale,
        PriceLevel::Partner,
        PriceLevel::PartnerTier2,
        PriceLevel::PartnerTier3,
        PriceLevel::Seasonal,
        PriceLevel::Exhibition,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PriceLevel::Retail => "retail",
            PriceLevel::Wholesale => "wholesale",
            PriceLevel::Partner => "partner",
            PriceLevel::PartnerTier2 => "partner_tier2",
            PriceLevel::PartnerTier3 => "partner_tier3",
            PriceLevel::Seasonal => "seasonal",
            PriceLevel::Exhibition => "exhibition",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PriceLevel::Retail => "جزئی",
            PriceLevel::Wholesale => "کلی",
            PriceLevel::Partner => "همکار",
            PriceLevel::PartnerTier2 => "همکار درجه ۲",
            PriceLevel::PartnerTier3 => "همکار درجه ۳",
            PriceLevel::Seasonal => "فصلی",
            PriceLevel::Exhibition => "نمایشگاه",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CatalogError> {
        Ok(match value {
            "retail" => PriceLevel::Retail,
            "wholesale" => PriceLevel::Wholesale,
            "partner" => PriceLevel::Partner,
            "partner_tier2" => PriceLevel::PartnerTier2,
            "partner_tier3" => PriceLevel::PartnerTier3,
            "seasonal" => PriceLevel::Seasonal,
            "exhibition" => PriceLevel::Exhibition,
            _ => return Err(CatalogError::UnknownPriceLevel),
        })
    }

    /// زنجیره‌ی جایگزینی وقتی این سطح برای کالا تعریف نشده است.
    ///
    /// مثلاً اگر «همکار درجه ۳» تعریف نشده باشد، به ترتیب درجه ۲، همکار و
    /// در نهایت جزئی بررسی می‌شود — رفتار مورد انتظار فروشنده.
    pub fn fallback_chain(self) -> &'static [PriceLevel] {
        match self {
            PriceLevel::Retail => &[PriceLevel::Retail],
            PriceLevel::Wholesale => &[PriceLevel::Wholesale, PriceLevel::Retail],
            PriceLevel::Partner => &[PriceLevel::Partner, PriceLevel::Wholesale, PriceLevel::Retail],
            PriceLevel::PartnerTier2 => &[
                PriceLevel::PartnerTier2,
                PriceLevel::Partner,
                PriceLevel::Wholesale,
                PriceLevel::Retail,
            ],
            PriceLevel::PartnerTier3 => &[
                PriceLevel::PartnerTier3,
                PriceLevel::PartnerTier2,
                PriceLevel::Partner,
                PriceLevel::Wholesale,
                PriceLevel::Retail,
            ],
            PriceLevel::Seasonal => &[PriceLevel::Seasonal, PriceLevel::Retail],
            PriceLevel::Exhibition => &[PriceLevel::Exhibition, PriceLevel::Retail],
        }
    }
}

/// جدول قیمت یک کالا.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceList {
    levels: BTreeMap<String, Money>,
}

impl PriceList {
    pub fn new() -> Self {
        PriceList::default()
    }

    /// تعیین قیمت یک سطح. قیمت منفی پذیرفته نمی‌شود.
    pub fn set(&mut self, level: PriceLevel, price: Money) -> Result<(), CatalogError> {
        if price.is_negative() {
            return Err(CatalogError::NegativePrice);
        }
        self.levels.insert(level.as_str().to_string(), price);
        Ok(())
    }

    /// قیمت دقیقاً همین سطح، بدون جایگزینی.
    pub fn exact(&self, level: PriceLevel) -> Option<Money> {
        self.levels.get(level.as_str()).copied()
    }

    /// قیمت مؤثر با در نظر گرفتن زنجیره‌ی جایگزینی.
    pub fn effective(&self, level: PriceLevel) -> Result<Money, CatalogError> {
        level
            .fallback_chain()
            .iter()
            .find_map(|candidate| self.exact(*candidate))
            .ok_or(CatalogError::NoPriceDefined)
    }

    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub fn defined_levels(&self) -> Vec<PriceLevel> {
        PriceLevel::ALL
            .iter()
            .copied()
            .filter(|level| self.exact(*level).is_some())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// چند واحدی
// ---------------------------------------------------------------------------

/// واحد فرعی با ضریب تبدیل به واحد اصلی.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitDefinition {
    pub name: String,
    /// چند واحد اصلی در یک واحد فرعی است؟ (کارتن = ۱۲ عدد → ۱۲)
    pub factor: f64,
}

/// مجموعه‌ی واحدهای یک کالا.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitSet {
    pub base_unit: String,
    pub alternatives: Vec<UnitDefinition>,
}

impl UnitSet {
    pub fn new(base_unit: impl Into<String>) -> Self {
        UnitSet {
            base_unit: base_unit.into(),
            alternatives: Vec::new(),
        }
    }

    /// افزودن واحد فرعی. ضریب باید مثبت و متناهی باشد.
    pub fn with_unit(
        mut self,
        name: impl Into<String>,
        factor: f64,
    ) -> Result<Self, CatalogError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(CatalogError::InvalidUnitFactor);
        }
        self.alternatives.push(UnitDefinition {
            name: name.into(),
            factor,
        });
        Ok(self)
    }

    fn factor_of(&self, unit: &str) -> Result<f64, CatalogError> {
        if unit == self.base_unit {
            return Ok(1.0);
        }
        self.alternatives
            .iter()
            .find(|definition| definition.name == unit)
            .map(|definition| definition.factor)
            .ok_or_else(|| CatalogError::UnknownUnit {
                unit: unit.to_string(),
            })
    }

    /// تبدیل مقدار از هر واحدی به واحد اصلی.
    pub fn to_base(&self, quantity: f64, unit: &str) -> Result<f64, CatalogError> {
        Ok(quantity * self.factor_of(unit)?)
    }

    /// تبدیل مقدار از واحد اصلی به واحد دلخواه.
    pub fn from_base(&self, quantity: f64, unit: &str) -> Result<f64, CatalogError> {
        Ok(quantity / self.factor_of(unit)?)
    }

    /// تبدیل مستقیم بین دو واحد فرعی.
    pub fn convert(&self, quantity: f64, from: &str, to: &str) -> Result<f64, CatalogError> {
        let base = self.to_base(quantity, from)?;
        self.from_base(base, to)
    }

    /// قیمت واحد فرعی از روی قیمت واحد اصلی.
    pub fn unit_price(&self, base_price: Money, unit: &str) -> Result<Money, CatalogError> {
        Ok(base_price.mul_quantity(self.factor_of(unit)?)?)
    }
}

// ---------------------------------------------------------------------------
// مالیات
// ---------------------------------------------------------------------------

/// اطلاعات مالیاتی کالا (زبانه‌ی «اطلاعات مالیاتی» و سامانه مؤدیان).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxProfile {
    /// نرخ مالیات بر ارزش افزوده بر حسب پایه‌نقطه (۹٪ = ۹۰۰).
    pub vat_basis_points: i64,
    /// نرخ عوارض بر حسب پایه‌نقطه.
    #[serde(default)]
    pub duty_basis_points: i64,
    /// شناسه‌ی کالا در سامانه مؤدیان.
    #[serde(default)]
    pub tax_code: Option<String>,
    #[serde(default)]
    pub tax_title: Option<String>,
    /// کالای معاف از مالیات.
    #[serde(default)]
    pub exempt: bool,
}

impl TaxProfile {
    /// نرخ استاندارد ارزش افزوده‌ی ایران (۹٪).
    pub fn standard() -> Self {
        TaxProfile {
            vat_basis_points: 900,
            ..Default::default()
        }
    }

    pub fn exempt() -> Self {
        TaxProfile {
            exempt: true,
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), CatalogError> {
        if !(0..=10_000).contains(&self.vat_basis_points)
            || !(0..=10_000).contains(&self.duty_basis_points)
        {
            return Err(CatalogError::InvalidTaxRate);
        }
        Ok(())
    }

    /// مالیات و عوارض یک مبلغ.
    pub fn tax_on(&self, amount: Money) -> Result<Money, CatalogError> {
        self.validate()?;
        if self.exempt {
            return Ok(Money::ZERO);
        }
        let vat = amount.percent_bp(self.vat_basis_points)?;
        let duty = amount.percent_bp(self.duty_basis_points)?;
        Ok(vat.checked_add(duty)?)
    }
}

// ---------------------------------------------------------------------------
// کالای مرکب
// ---------------------------------------------------------------------------

/// یک جزء از کالای مرکب.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub product_id: String,
    pub quantity: f64,
    /// بهای واحد جزء (ریال).
    pub unit_cost: Money,
}

/// بهای تمام‌شده‌ی کالای مرکب از روی اجزای آن.
pub fn composite_cost(parent_id: &str, components: &[Component]) -> Result<Money, CatalogError> {
    if components.is_empty() {
        return Err(CatalogError::EmptyComposite);
    }
    let mut total = Money::ZERO;
    for component in components {
        if component.product_id == parent_id {
            return Err(CatalogError::SelfReference);
        }
        if !component.quantity.is_finite() || component.quantity <= 0.0 {
            return Err(CatalogError::InvalidComponentQuantity);
        }
        if component.unit_cost.is_negative() {
            return Err(CatalogError::NegativePrice);
        }
        total = total.checked_add(component.unit_cost.mul_quantity(component.quantity)?)?;
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// کالای تنوع‌دار
// ---------------------------------------------------------------------------

/// یک ویژگی تنوع (رنگ، سایز و…) با مقادیر ممکن.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantAttribute {
    pub name: String,
    pub values: Vec<String>,
}

/// یک ترکیب تنوع تولیدشده.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VariantCombination {
    /// مقادیر به ترتیب ویژگی‌ها: مثلاً ["قرمز", "L"]
    pub values: Vec<String>,
    /// کد یکتای تنوع بر پایه‌ی کد کالای اصلی.
    pub sku: String,
}

/// تولید همه‌ی ترکیب‌های ممکن تنوع (ضرب دکارتی).
///
/// مثال: رنگ (قرمز، آبی) × سایز (M، L) → چهار تنوع با SKUهای یکتا.
pub fn expand_variants(
    base_sku: &str,
    attributes: &[VariantAttribute],
) -> Result<Vec<VariantCombination>, CatalogError> {
    if attributes.is_empty() || attributes.iter().any(|attribute| attribute.values.is_empty()) {
        return Err(CatalogError::EmptyVariantAttributes);
    }
    let mut combinations: Vec<Vec<String>> = vec![Vec::new()];
    for attribute in attributes {
        let mut expanded = Vec::with_capacity(combinations.len() * attribute.values.len());
        for existing in &combinations {
            for value in &attribute.values {
                let mut next = existing.clone();
                next.push(value.clone());
                expanded.push(next);
            }
        }
        combinations = expanded;
    }
    Ok(combinations
        .into_iter()
        .enumerate()
        .map(|(index, values)| VariantCombination {
            sku: format!("{base_sku}-{:03}", index + 1),
            values,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// طلا و جواهر
// ---------------------------------------------------------------------------

/// پارامترهای قیمت‌گذاری طلا.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GoldPricing {
    /// وزن به گرم.
    pub weight_grams: f64,
    /// نرخ هر گرم طلا (ریال).
    pub rate_per_gram: Money,
    /// اجرت ساخت بر حسب پایه‌نقطه (۷٪ = ۷۰۰).
    pub making_charge_bp: i64,
    /// سود فروشنده بر حسب پایه‌نقطه.
    pub profit_bp: i64,
    /// نرخ ارزش افزوده بر حسب پایه‌نقطه.
    pub vat_bp: i64,
}

/// تفکیک قیمت نهایی طلا.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoldPriceBreakdown {
    /// ارزش خود طلا.
    pub metal_value: Money,
    /// اجرت ساخت.
    pub making_charge: Money,
    /// سود فروشنده.
    pub profit: Money,
    /// مالیات بر ارزش افزوده.
    pub vat: Money,
    /// مبلغ قابل پرداخت.
    pub total: Money,
}

/// محاسبه‌ی قیمت طلا مطابق رویه‌ی بازار ایران.
///
/// ```text
/// ارزش طلا = وزن × نرخ هر گرم
/// اجرت     = ارزش طلا × درصد اجرت
/// سود      = (ارزش طلا + اجرت) × درصد سود
/// مالیات   = (اجرت + سود) × نرخ ارزش افزوده     ← فقط بر اجرت و سود
/// جمع      = ارزش طلا + اجرت + سود + مالیات
/// ```
///
/// نکته‌ی مهم: طبق مقررات، ارزش افزوده **بر ارزش خود طلا تعلق نمی‌گیرد** و فقط
/// بر اجرت و سود محاسبه می‌شود.
pub fn gold_price(pricing: GoldPricing) -> Result<GoldPriceBreakdown, CatalogError> {
    if !pricing.weight_grams.is_finite() || pricing.weight_grams <= 0.0 {
        return Err(CatalogError::InvalidWeight);
    }
    if pricing.rate_per_gram.is_negative() {
        return Err(CatalogError::NegativePrice);
    }
    if !(0..=10_000).contains(&pricing.making_charge_bp)
        || !(0..=10_000).contains(&pricing.profit_bp)
        || !(0..=10_000).contains(&pricing.vat_bp)
    {
        return Err(CatalogError::InvalidTaxRate);
    }

    let metal_value = pricing.rate_per_gram.mul_quantity(pricing.weight_grams)?;
    let making_charge = metal_value.percent_bp(pricing.making_charge_bp)?;
    let profit = metal_value
        .checked_add(making_charge)?
        .percent_bp(pricing.profit_bp)?;
    let vat = making_charge.checked_add(profit)?.percent_bp(pricing.vat_bp)?;
    let total = metal_value
        .checked_add(making_charge)?
        .checked_add(profit)?
        .checked_add(vat)?;

    Ok(GoldPriceBreakdown {
        metal_value,
        making_charge,
        profit,
        vat,
        total,
    })
}

// ---------------------------------------------------------------------------
// گروه‌بندی درختی کالا
// ---------------------------------------------------------------------------

/// یک گروه کالا (درخت سمت راست لیست کالاها).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductGroup {
    pub code: String,
    pub title: String,
    #[serde(default)]
    pub parent_code: Option<String>,
}

/// گره‌ی درخت گروه کالا.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProductGroupNode {
    pub group: ProductGroup,
    pub children: Vec<ProductGroupNode>,
}

/// ساخت درخت گروه‌های کالا با کشف کد تکراری و والد گم‌شده.
pub fn build_group_tree(groups: &[ProductGroup]) -> Result<Vec<ProductGroupNode>, CatalogError> {
    let mut by_code: BTreeMap<&str, &ProductGroup> = BTreeMap::new();
    for group in groups {
        if by_code.insert(group.code.as_str(), group).is_some() {
            return Err(CatalogError::DuplicateGroupCode {
                code: group.code.clone(),
            });
        }
    }
    for group in groups {
        if let Some(parent) = &group.parent_code {
            if !by_code.contains_key(parent.as_str()) {
                return Err(CatalogError::MissingParentGroup {
                    parent: parent.clone(),
                });
            }
        }
    }

    fn collect(
        by_code: &BTreeMap<&str, &ProductGroup>,
        parent: Option<&str>,
    ) -> Vec<ProductGroupNode> {
        by_code
            .values()
            .filter(|group| group.parent_code.as_deref() == parent)
            .map(|group| ProductGroupNode {
                group: (*group).clone(),
                children: collect(by_code, Some(&group.code)),
            })
            .collect()
    }

    Ok(collect(&by_code, None))
}

/// مسیر کامل یک گروه از ریشه: `مواد غذایی / لبنیات / پنیر`
pub fn group_path(groups: &[ProductGroup], code: &str) -> Option<String> {
    let by_code: BTreeMap<&str, &ProductGroup> =
        groups.iter().map(|group| (group.code.as_str(), group)).collect();
    let mut parts = Vec::new();
    let mut cursor = Some(code);
    let mut guard = 0;
    while let Some(current) = cursor {
        let group = by_code.get(current)?;
        parts.push(group.title.clone());
        cursor = group.parent_code.as_deref();
        guard += 1;
        if guard > 64 {
            return None; // محافظت در برابر حلقه‌ی والد
        }
    }
    parts.reverse();
    Some(parts.join(" / "))
}
