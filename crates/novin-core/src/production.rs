//! تولید و فرمول تولید (BOM) و محاسبه‌ی بهای تمام‌شده.
//!
//! مرجع: تصویر `3qTCnS` (رسید تولید).
//!
//! ## معادله‌ی محوری بهای تمام‌شده
//!
//! ```text
//! جمع مواد مصرفی + جمع هزینه‌های تولید = جمع بهای تمام‌شده‌ی کالاهای تولیدشده
//! ```
//!
//! این معادله **همیشه** باید برقرار باشد. اگر نباشد، یعنی یا ارزشی از هوا
//! ساخته شده یا ارزشی ناپدید شده — و هر دو در صورت‌های مالی به سود یا زیان
//! ساختگی تبدیل می‌شوند.
//!
//! ## سند حسابداری تولید
//!
//! ```text
//! بدهکار  موجودی کالای ساخته‌شده     بهای تمام‌شده‌ی محصولات
//! بستانکار موجودی مواد اولیه          ارزش مواد مصرفی
//! بستانکار حساب‌های هزینه‌ی تولید      دستمزد، سربار و…
//! ```
//!
//! تولید **سود نمی‌سازد**؛ فقط شکل دارایی عوض می‌شود. سود در لحظه‌ی فروش
//! محقق می‌شود، نه در لحظه‌ی تولید.
//!
//! ## تخصیص بهای تمام‌شده بین چند محصول
//!
//! وقتی یک رسید تولید چند محصول دارد (مثلاً محصول اصلی و محصول فرعی)، کل
//! بهای تمام‌شده باید بین آن‌ها تقسیم شود. دو روش پشتیبانی می‌شود:
//!
//! - **بر اساس مقدار**: مناسب محصولات همگن
//! - **بر اساس ارزش بازار**: مناسب محصول اصلی و فرعی با ارزش خیلی متفاوت
//!
//! تقسیم با الگوریتم «بزرگ‌ترین باقیمانده» انجام می‌شود تا جمع اجزا دقیقاً
//! برابر کل باشد و حتی یک ریال هم گم نشود.

use crate::money::{Money, MoneyError};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProductionError {
    #[error("رسید تولید باید حداقل یک محصول داشته باشد")]
    NoOutput,
    #[error("رسید تولید باید حداقل یک ماده‌ی مصرفی داشته باشد")]
    NoInput,
    #[error("مقدار باید بیشتر از صفر باشد")]
    NonPositiveQuantity,
    #[error("بهای واحد نمی‌تواند منفی باشد")]
    NegativeCost,
    #[error("مبلغ هزینه نمی‌تواند منفی باشد")]
    NegativeExpense,
    #[error("محصول «{product}» در فرمول تکرار شده است")]
    DuplicateComponent { product: String },
    #[error("کالای تولیدشده نمی‌تواند در مواد مصرفی خودش باشد: {product}")]
    CircularFormula { product: String },
    #[error(
        "معادله‌ی بهای تمام‌شده برقرار نیست: مواد {inputs} + هزینه {expenses} \
         در برابر محصولات {outputs}"
    )]
    UnbalancedCost {
        inputs: i64,
        expenses: i64,
        outputs: i64,
    },
    #[error("ضریب مصرف فرمول باید بیشتر از صفر باشد")]
    NonPositiveRatio,
    #[error("خطای مبلغ: {0}")]
    Money(#[from] MoneyError),
}

/// روش تخصیص بهای تمام‌شده بین چند محصول یک رسید.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostAllocation {
    /// به نسبت مقدار تولیدشده — مناسب محصولات همگن.
    ByQuantity,
    /// به نسبت ارزش بازار — مناسب محصول اصلی و فرعی.
    ByMarketValue,
}

impl CostAllocation {
    pub fn as_str(self) -> &'static str {
        match self {
            CostAllocation::ByQuantity => "by_quantity",
            CostAllocation::ByMarketValue => "by_market_value",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CostAllocation::ByQuantity => "بر اساس مقدار",
            CostAllocation::ByMarketValue => "بر اساس ارزش بازار",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "by_quantity" => Some(CostAllocation::ByQuantity),
            "by_market_value" => Some(CostAllocation::ByMarketValue),
            _ => None,
        }
    }

    /// توضیح سه‌جمله‌ای برای نمایش کنار گزینه در فرم.
    pub fn explanation(self) -> &'static str {
        match self {
            CostAllocation::ByQuantity => {
                "کل بهای تمام‌شده به نسبت مقدار هر محصول تقسیم می‌شود. \
                 مناسب وقتی محصولات از یک جنس و هم‌ارزش‌اند. \
                 اگر ارزش محصولات خیلی متفاوت باشد، این روش بهای محصول ارزان را بالا می‌برد."
            }
            CostAllocation::ByMarketValue => {
                "کل بهای تمام‌شده به نسبت ارزش فروش هر محصول تقسیم می‌شود. \
                 مناسب وقتی یک محصول اصلی و یک محصول فرعی دارید. \
                 حاشیه‌ی سود هر دو محصول برابر درمی‌آید."
            }
        }
    }
}

/// یک ماده‌ی مصرفی در رسید تولید.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsumedMaterial {
    pub product_id: String,
    pub quantity: f64,
    /// بهای تمام‌شده‌ی واحد در لحظه‌ی مصرف.
    pub unit_cost: Money,
}

impl ConsumedMaterial {
    pub fn total(&self) -> Result<Money, ProductionError> {
        if self.quantity <= 0.0 {
            return Err(ProductionError::NonPositiveQuantity);
        }
        if self.unit_cost.rials() < 0 {
            return Err(ProductionError::NegativeCost);
        }
        Ok(Money::from_rials(
            (self.quantity * self.unit_cost.rials() as f64).round() as i64,
        ))
    }
}

/// یک هزینه‌ی تولید (دستمزد، سربار، انرژی و…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionExpense {
    /// حساب هزینه در کدینگ.
    pub account_id: String,
    pub title: String,
    pub amount: Money,
}

/// یک محصول تولیدشده.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProducedItem {
    pub product_id: String,
    pub quantity: f64,
    /// ارزش بازار واحد — فقط برای تخصیص بر اساس ارزش لازم است.
    #[serde(default)]
    pub market_unit_price: Option<Money>,
}

/// نتیجه‌ی محاسبه‌ی بهای تمام‌شده‌ی یک رسید تولید.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductionCosting {
    pub materials_total: Money,
    pub expenses_total: Money,
    /// جمع کل که باید بین محصولات تقسیم شود.
    pub total_cost: Money,
    pub outputs: Vec<ProducedCost>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProducedCost {
    pub product_id: String,
    pub quantity: f64,
    /// سهم این محصول از کل بهای تمام‌شده.
    pub allocated_cost: Money,
    /// بهای تمام‌شده‌ی هر واحد.
    pub unit_cost: Money,
}

/// محاسبه‌ی بهای تمام‌شده‌ی رسید تولید و تخصیص آن بین محصولات.
///
/// ریال آخر هم گم نمی‌شود: تخصیص با «بزرگ‌ترین باقیمانده» انجام می‌شود تا
/// جمع سهم‌ها دقیقاً برابر کل باشد.
pub fn calculate_costing(
    materials: &[ConsumedMaterial],
    expenses: &[ProductionExpense],
    outputs: &[ProducedItem],
    allocation: CostAllocation,
) -> Result<ProductionCosting, ProductionError> {
    if outputs.is_empty() {
        return Err(ProductionError::NoOutput);
    }
    if materials.is_empty() {
        return Err(ProductionError::NoInput);
    }

    // یک محصول نمی‌تواند هم ورودی باشد هم خروجی — وگرنه بهای تمام‌شده‌اش
    // به خودش وابسته می‌شود و محاسبه بی‌معنا است.
    for output in outputs {
        if materials
            .iter()
            .any(|material| material.product_id == output.product_id)
        {
            return Err(ProductionError::CircularFormula {
                product: output.product_id.clone(),
            });
        }
    }

    let mut seen: Vec<&str> = Vec::with_capacity(materials.len());
    let mut materials_total = Money::ZERO;
    for material in materials {
        if seen.contains(&material.product_id.as_str()) {
            return Err(ProductionError::DuplicateComponent {
                product: material.product_id.clone(),
            });
        }
        seen.push(&material.product_id);
        materials_total = materials_total.checked_add(material.total()?)?;
    }

    let mut expenses_total = Money::ZERO;
    for expense in expenses {
        if expense.amount.rials() < 0 {
            return Err(ProductionError::NegativeExpense);
        }
        expenses_total = expenses_total.checked_add(expense.amount)?;
    }

    let total_cost = materials_total.checked_add(expenses_total)?;

    // وزن تخصیص به‌ازای هر محصول.
    let weights: Vec<i64> = match allocation {
        CostAllocation::ByQuantity => outputs
            .iter()
            .map(|output| {
                if output.quantity <= 0.0 {
                    Err(ProductionError::NonPositiveQuantity)
                } else {
                    // مقدار اعشاری به وزن صحیح تبدیل می‌شود تا تخصیص دقیق بماند.
                    Ok((output.quantity * 1_000_000.0).round() as i64)
                }
            })
            .collect::<Result<_, _>>()?,
        CostAllocation::ByMarketValue => outputs
            .iter()
            .map(|output| {
                if output.quantity <= 0.0 {
                    return Err(ProductionError::NonPositiveQuantity);
                }
                let price = output.market_unit_price.unwrap_or(Money::ZERO);
                if price.rials() < 0 {
                    return Err(ProductionError::NegativeCost);
                }
                Ok((output.quantity * price.rials() as f64).round().max(0.0) as i64)
            })
            .collect::<Result<_, _>>()?,
    };

    // اگر همه‌ی وزن‌ها صفر باشند (مثلاً ارزش بازار وارد نشده)، به‌جای خطای
    // تقسیم بر صفر، مساوی تقسیم می‌کنیم.
    let weights = if weights.iter().all(|weight| *weight == 0) {
        vec![1i64; outputs.len()]
    } else {
        weights
    };

    let shares = total_cost.allocate(&weights)?;
    let mut allocated = Vec::with_capacity(outputs.len());
    for (output, share) in outputs.iter().zip(shares.iter()) {
        let unit = if output.quantity > 0.0 {
            Money::from_rials((share.rials() as f64 / output.quantity).round() as i64)
        } else {
            Money::ZERO
        };
        allocated.push(ProducedCost {
            product_id: output.product_id.clone(),
            quantity: output.quantity,
            allocated_cost: *share,
            unit_cost: unit,
        });
    }

    Ok(ProductionCosting {
        materials_total,
        expenses_total,
        total_cost,
        outputs: allocated,
    })
}

/// بررسی برقراری معادله‌ی بهای تمام‌شده.
///
/// آخرین خط دفاع پیش از صدور سند: اگر جمع سهم محصولات با جمع مواد و هزینه
/// برابر نباشد، سند صادر نمی‌شود.
pub fn assert_cost_balance(costing: &ProductionCosting) -> Result<(), ProductionError> {
    let allocated: i64 = costing
        .outputs
        .iter()
        .map(|output| output.allocated_cost.rials())
        .sum();
    if allocated != costing.total_cost.rials() {
        return Err(ProductionError::UnbalancedCost {
            inputs: costing.materials_total.rials(),
            expenses: costing.expenses_total.rials(),
            outputs: allocated,
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// فرمول تولید (BOM)
// ---------------------------------------------------------------------------

/// یک جزء در فرمول تولید.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormulaComponent {
    pub product_id: String,
    /// مقدار مصرف برای تولید یک واحد محصول.
    pub quantity_per_unit: f64,
    /// درصد ضایعات مجاز — مصرف واقعی از این بیشتر می‌شود.
    #[serde(default)]
    pub waste_percent: f64,
}

impl FormulaComponent {
    /// مصرف واقعی برای تولید مقدار مشخصی محصول، با احتساب ضایعات.
    ///
    /// ضایعات بخشی از بهای تمام‌شده است، نه هزینه‌ی جداگانه: ماده‌ای که
    /// ضایع می‌شود هم پول شرکت را مصرف کرده است.
    pub fn required_for(&self, output_quantity: f64) -> Result<f64, ProductionError> {
        if self.quantity_per_unit <= 0.0 {
            return Err(ProductionError::NonPositiveRatio);
        }
        if output_quantity <= 0.0 {
            return Err(ProductionError::NonPositiveQuantity);
        }
        let base = self.quantity_per_unit * output_quantity;
        Ok(base * (1.0 + self.waste_percent / 100.0))
    }
}

/// فرمول تولید یک محصول.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionFormula {
    pub product_id: String,
    pub title: String,
    /// مقداری که این فرمول تولید می‌کند (معمولاً ۱).
    pub output_quantity: f64,
    pub components: Vec<FormulaComponent>,
}

impl ProductionFormula {
    /// اعتبارسنجی فرمول پیش از ذخیره.
    pub fn validate(&self) -> Result<(), ProductionError> {
        if self.output_quantity <= 0.0 {
            return Err(ProductionError::NonPositiveQuantity);
        }
        if self.components.is_empty() {
            return Err(ProductionError::NoInput);
        }
        let mut seen: Vec<&str> = Vec::with_capacity(self.components.len());
        for component in &self.components {
            if component.product_id == self.product_id {
                return Err(ProductionError::CircularFormula {
                    product: component.product_id.clone(),
                });
            }
            if seen.contains(&component.product_id.as_str()) {
                return Err(ProductionError::DuplicateComponent {
                    product: component.product_id.clone(),
                });
            }
            if component.quantity_per_unit <= 0.0 {
                return Err(ProductionError::NonPositiveRatio);
            }
            if component.waste_percent < 0.0 || component.waste_percent >= 100.0 {
                return Err(ProductionError::NonPositiveRatio);
            }
            seen.push(&component.product_id);
        }
        Ok(())
    }

    /// گسترش فرمول برای تولید مقدار مشخص: چه مقدار از هر ماده لازم است.
    pub fn expand(&self, output_quantity: f64) -> Result<Vec<(String, f64)>, ProductionError> {
        self.validate()?;
        if output_quantity <= 0.0 {
            return Err(ProductionError::NonPositiveQuantity);
        }
        // فرمول ممکن است برای چند واحد نوشته شده باشد.
        let batches = output_quantity / self.output_quantity;
        self.components
            .iter()
            .map(|component| {
                component
                    .required_for(batches * self.output_quantity)
                    .map(|quantity| (component.product_id.clone(), quantity))
            })
            .collect()
    }
}

/// بیشترین مقداری که با موجودی فعلی مواد قابل تولید است.
///
/// کاربرد: پیش از شروع تولید، به کاربر بگوییم اصلاً چقدر می‌تواند بسازد.
/// ماده‌ای که کمترین ظرفیت را می‌دهد، گلوگاه است.
pub fn producible_quantity(
    formula: &ProductionFormula,
    stock: &[(String, f64)],
) -> Result<f64, ProductionError> {
    formula.validate()?;
    let mut limit = f64::INFINITY;
    for component in &formula.components {
        let available = stock
            .iter()
            .find(|(product, _)| *product == component.product_id)
            .map(|(_, quantity)| *quantity)
            .unwrap_or(0.0);
        // مصرف هر واحد محصول، با ضایعات.
        let per_unit = component.quantity_per_unit * (1.0 + component.waste_percent / 100.0)
            / formula.output_quantity;
        if per_unit <= 0.0 {
            return Err(ProductionError::NonPositiveRatio);
        }
        limit = limit.min(available / per_unit);
    }
    Ok(if limit.is_finite() { limit.max(0.0) } else { 0.0 })
}
