//! انبارگردانی بر پایه‌ی منطق حسابداری انبار.
//!
//! مرجع: منوی «عملیات ← عملیات انبار» و لیست کالاهای نرم‌افزار فعلی
//! (`docs/FEATURE_BASELINE.md` بخش‌های ۱۷ و ۱۸).
//!
//! ## چرا انبارگردانی صرفاً «شمارش» نیست
//!
//! انبارگردانی یک **رویداد مالی** است، نه یک فهرست شمارش. سه قاعده‌ی حسابداری
//! که این ماژول تضمین می‌کند:
//!
//! ۱. **فریز منطقی موجودی.** لحظه‌ی شروع شمارش، موجودی سیستمی هر کالا عکس‌برداری
//!    می‌شود. اگر حین شمارش فروشی انجام شود، مبنای مقایسه به‌هم نمی‌ریزد.
//! ۲. **اختلاف باید تأیید شود.** ثبت مستقیم اختلاف بدون تأیید، راه فرار از کنترل
//!    داخلی است. مسیر اجباری: شمارش ← بازبینی ← تأیید ← ثبت.
//! ۳. **اختلاف سند مالی می‌خواهد.** کسری هزینه است و اضافی درآمد؛ هر دو باید به
//!    بهای تمام‌شده‌ی واقعی (همان موتور ارزش‌گذاری) ارزش‌گذاری و سند بخورند.

use crate::accounting::{validate_journal, AccountingError, JournalLine};
use crate::money::{Money, MoneyError};
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی انبارگردانی.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StocktakeError {
    #[error("STK-001: گذار وضعیت انبارگردانی مجاز نیست: از «{from}» به «{to}»")]
    InvalidTransition { from: &'static str, to: &'static str },
    #[error("STK-002: دوره‌ی انبارگردانی بدون قلم قابل شروع نیست")]
    EmptySession,
    #[error("STK-003: شمارش همه‌ی اقلام کامل نشده است: {remaining} قلم باقی مانده")]
    IncompleteCount { remaining: usize },
    #[error("STK-004: مقدار شمارش نمی‌تواند منفی باشد")]
    NegativeCount,
    #[error("STK-005: اختلاف تأییدنشده وجود دارد: {count} قلم")]
    UnapprovedVariance { count: usize },
    #[error("STK-006: دوره‌ی بسته‌شده قابل تغییر نیست")]
    SessionLocked,
    #[error("STK-007: در دوره‌ی فریزشده نمی‌توان قلم جدید افزود")]
    FrozenSession,
    #[error("STK-008: حساب‌های کسری و اضافی انبار تعریف نشده‌اند")]
    MissingVarianceAccounts,
    #[error("STK-009: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
    #[error("STK-010: {0}")]
    Accounting(#[from] AccountingError),
}

// ---------------------------------------------------------------------------
// چرخه‌ی وضعیت
// ---------------------------------------------------------------------------

/// وضعیت دوره‌ی انبارگردانی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StocktakeStatus {
    /// پیش‌نویس — هنوز اقلام قابل افزودن و حذف‌اند.
    Draft,
    /// فریزشده — موجودی سیستمی عکس‌برداری شده و شمارش آغاز شده است.
    Counting,
    /// بازبینی — شمارش کامل شده و اختلاف‌ها در انتظار تأیید هستند.
    Review,
    /// ثبت‌شده — سند تعدیل صادر شده و دوره قفل است.
    Posted,
    /// ابطال‌شده.
    Cancelled,
}

impl StocktakeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            StocktakeStatus::Draft => "draft",
            StocktakeStatus::Counting => "counting",
            StocktakeStatus::Review => "review",
            StocktakeStatus::Posted => "posted",
            StocktakeStatus::Cancelled => "cancelled",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            StocktakeStatus::Draft => "پیش‌نویس",
            StocktakeStatus::Counting => "در حال شمارش",
            StocktakeStatus::Review => "بازبینی اختلاف",
            StocktakeStatus::Posted => "ثبت‌شده",
            StocktakeStatus::Cancelled => "ابطال‌شده",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "draft" => StocktakeStatus::Draft,
            "counting" => StocktakeStatus::Counting,
            "review" => StocktakeStatus::Review,
            "posted" => StocktakeStatus::Posted,
            "cancelled" => StocktakeStatus::Cancelled,
            _ => return None,
        })
    }

    /// آیا دوره قفل است؟ (دیگر هیچ تغییری نمی‌پذیرد)
    pub fn is_locked(self) -> bool {
        matches!(self, StocktakeStatus::Posted | StocktakeStatus::Cancelled)
    }

    /// آیا موجودی سیستمی فریز شده است؟
    pub fn is_frozen(self) -> bool {
        !matches!(self, StocktakeStatus::Draft)
    }
}

/// گذارهای مجاز وضعیت دوره.
pub fn allowed_transitions(status: StocktakeStatus) -> &'static [StocktakeStatus] {
    use StocktakeStatus::*;
    match status {
        Draft => &[Counting, Cancelled],
        Counting => &[Review, Cancelled],
        // بازگشت به شمارش برای شمارش مجدد مجاز است.
        Review => &[Counting, Posted, Cancelled],
        Posted | Cancelled => &[],
    }
}

/// اعمال گذار وضعیت با اعتبارسنجی.
pub fn transition(
    from: StocktakeStatus,
    to: StocktakeStatus,
) -> Result<StocktakeStatus, StocktakeError> {
    if allowed_transitions(from).contains(&to) {
        Ok(to)
    } else {
        Err(StocktakeError::InvalidTransition {
            from: from.as_str(),
            to: to.as_str(),
        })
    }
}

// ---------------------------------------------------------------------------
// اقلام شمارش
// ---------------------------------------------------------------------------

/// یک قلم در دوره‌ی انبارگردانی.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountLine {
    pub product_id: String,
    /// موجودی سیستمی در لحظه‌ی فریز — مبنای مقایسه.
    pub frozen_quantity: f64,
    /// شمارش اول.
    #[serde(default)]
    pub counted_quantity: Option<f64>,
    /// شمارش مجدد (در صورت وجود، بر شمارش اول ارجحیت دارد).
    #[serde(default)]
    pub recount_quantity: Option<f64>,
    /// تأیید اختلاف توسط مسئول.
    #[serde(default)]
    pub variance_approved: bool,
    /// بهای واحد برای ارزش‌گذاری اختلاف (از موتور ارزش‌گذاری می‌آید).
    #[serde(default)]
    pub unit_cost: Money,
}

impl CountLine {
    pub fn new(product_id: impl Into<String>, frozen_quantity: f64, unit_cost: Money) -> Self {
        CountLine {
            product_id: product_id.into(),
            frozen_quantity,
            counted_quantity: None,
            recount_quantity: None,
            variance_approved: false,
            unit_cost,
        }
    }

    /// مقدار نهایی شمارش: شمارش مجدد در اولویت است.
    pub fn final_quantity(&self) -> Option<f64> {
        self.recount_quantity.or(self.counted_quantity)
    }

    /// آیا شمارش این قلم انجام شده است؟
    pub fn is_counted(&self) -> bool {
        self.final_quantity().is_some()
    }

    /// اختلاف: مثبت = اضافی، منفی = کسری.
    pub fn variance(&self) -> Option<f64> {
        self.final_quantity().map(|value| value - self.frozen_quantity)
    }

    /// آیا این قلم اختلاف دارد؟
    pub fn has_variance(&self) -> bool {
        self.variance().map(|value| value.abs() > 1e-9).unwrap_or(false)
    }

    /// ارزش ریالی اختلاف (مثبت = اضافی).
    pub fn variance_value(&self) -> Result<Money, StocktakeError> {
        let variance = self.variance().unwrap_or(0.0);
        Ok(self.unit_cost.mul_quantity(variance)?)
    }
}

/// خلاصه‌ی یک دوره‌ی انبارگردانی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StocktakeSummary {
    pub total_lines: usize,
    pub counted_lines: usize,
    pub uncounted_lines: usize,
    pub surplus_lines: usize,
    pub shortage_lines: usize,
    pub unapproved_variances: usize,
    /// ارزش کل اضافی (مثبت).
    pub surplus_value: Money,
    /// ارزش کل کسری (مثبت).
    pub shortage_value: Money,
    /// اثر خالص بر ارزش موجودی (اضافی − کسری).
    pub net_value: Money,
}

/// محاسبه‌ی خلاصه‌ی دوره.
pub fn summarize(lines: &[CountLine]) -> Result<StocktakeSummary, StocktakeError> {
    let mut summary = StocktakeSummary {
        total_lines: lines.len(),
        counted_lines: 0,
        uncounted_lines: 0,
        surplus_lines: 0,
        shortage_lines: 0,
        unapproved_variances: 0,
        surplus_value: Money::ZERO,
        shortage_value: Money::ZERO,
        net_value: Money::ZERO,
    };
    for line in lines {
        if line.is_counted() {
            summary.counted_lines += 1;
        } else {
            summary.uncounted_lines += 1;
            continue;
        }
        if !line.has_variance() {
            continue;
        }
        if !line.variance_approved {
            summary.unapproved_variances += 1;
        }
        let value = line.variance_value()?;
        if value.rials() > 0 {
            summary.surplus_lines += 1;
            summary.surplus_value = summary.surplus_value.checked_add(value)?;
        } else if value.rials() < 0 {
            summary.shortage_lines += 1;
            summary.shortage_value = summary.shortage_value.checked_add(value.abs())?;
        }
    }
    summary.net_value = summary.surplus_value.checked_sub(summary.shortage_value)?;
    Ok(summary)
}

/// بررسی آمادگی دوره برای ثبت نهایی.
///
/// دو شرط: همه‌ی اقلام شمارش شده باشند و همه‌ی اختلاف‌ها تأیید شده باشند.
pub fn ensure_postable(lines: &[CountLine]) -> Result<StocktakeSummary, StocktakeError> {
    if lines.is_empty() {
        return Err(StocktakeError::EmptySession);
    }
    for line in lines {
        for quantity in [line.counted_quantity, line.recount_quantity].into_iter().flatten() {
            if !quantity.is_finite() || quantity < 0.0 {
                return Err(StocktakeError::NegativeCount);
            }
        }
    }
    let summary = summarize(lines)?;
    if summary.uncounted_lines > 0 {
        return Err(StocktakeError::IncompleteCount {
            remaining: summary.uncounted_lines,
        });
    }
    if summary.unapproved_variances > 0 {
        return Err(StocktakeError::UnapprovedVariance {
            count: summary.unapproved_variances,
        });
    }
    Ok(summary)
}

/// اقلامی که نیاز به شمارش مجدد دارند.
///
/// قاعده‌ی کنترل داخلی: اختلاف بزرگ‌تر از آستانه باید دوباره شمرده شود، نه اینکه
/// مستقیم تأیید گردد.
pub fn lines_needing_recount(lines: &[CountLine], threshold_percent: f64) -> Vec<&CountLine> {
    lines
        .iter()
        .filter(|line| {
            if line.recount_quantity.is_some() || !line.has_variance() {
                return false;
            }
            let variance = line.variance().unwrap_or(0.0).abs();
            if line.frozen_quantity.abs() < 1e-9 {
                return variance > 0.0;
            }
            (variance / line.frozen_quantity.abs()) * 100.0 >= threshold_percent
        })
        .collect()
}

// ---------------------------------------------------------------------------
// سند تعدیل
// ---------------------------------------------------------------------------

/// حساب‌های موردنیاز برای سند تعدیل انبارگردانی.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarianceAccounts {
    /// حساب موجودی کالا (دارایی).
    pub inventory: String,
    /// حساب کسری و ضایعات انبار (هزینه).
    pub shortage_expense: String,
    /// حساب اضافات انبار (درآمد).
    pub surplus_income: String,
}

impl VarianceAccounts {
    fn validate(&self) -> Result<(), StocktakeError> {
        if self.inventory.trim().is_empty()
            || self.shortage_expense.trim().is_empty()
            || self.surplus_income.trim().is_empty()
        {
            return Err(StocktakeError::MissingVarianceAccounts);
        }
        Ok(())
    }
}

/// ساخت سند تعدیل انبارگردانی.
///
/// ```text
/// اضافی:  بدهکار موجودی کالا      / بستانکار اضافات انبار
/// کسری:   بدهکار کسری و ضایعات   / بستانکار موجودی کالا
/// ```
///
/// اگر هم اضافی و هم کسری وجود داشته باشد، هر دو در یک سند با خالص اثر روی
/// حساب موجودی ثبت می‌شوند. سند خروجی همیشه متعادل است.
pub fn build_adjustment_journal(
    lines: &[CountLine],
    accounts: &VarianceAccounts,
) -> Result<Vec<JournalLine>, StocktakeError> {
    accounts.validate()?;
    let summary = ensure_postable(lines)?;

    if summary.surplus_value.is_zero() && summary.shortage_value.is_zero() {
        // انبارگردانی بدون اختلاف سند نمی‌خواهد.
        return Ok(Vec::new());
    }

    let mut journal = Vec::new();
    let net = summary.net_value;
    if net.rials() > 0 {
        journal.push(JournalLine::debit(&accounts.inventory, net));
    } else if net.rials() < 0 {
        journal.push(JournalLine::credit(&accounts.inventory, net.abs()));
    }
    if !summary.surplus_value.is_zero() {
        journal.push(JournalLine::credit(
            &accounts.surplus_income,
            summary.surplus_value,
        ));
    }
    if !summary.shortage_value.is_zero() {
        journal.push(JournalLine::debit(
            &accounts.shortage_expense,
            summary.shortage_value,
        ));
    }

    validate_journal(&journal)?;
    Ok(journal)
}

// ---------------------------------------------------------------------------
// عملیات جمعی روی کالا
// ---------------------------------------------------------------------------

/// نوع تغییر جمعی قیمت.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkPriceChange {
    /// افزایش/کاهش درصدی (پایه‌نقطه؛ منفی = کاهش).
    Percent(i64),
    /// افزایش/کاهش مبلغ ثابت.
    Amount(Money),
    /// جایگزینی با مبلغ مشخص.
    Set(Money),
}

/// خطاهای عملیات جمعی.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BulkError {
    #[error("BLK-001: نتیجه‌ی تغییر قیمت منفی می‌شود: کالای {product_id}")]
    NegativeResult { product_id: String },
    #[error("BLK-002: درصد تغییر نامعتبر است")]
    InvalidPercent,
    #[error("BLK-003: هیچ کالایی انتخاب نشده است")]
    EmptySelection,
    #[error("BLK-004: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
}

/// نتیجه‌ی پیش‌نمایش تغییر جمعی قیمت.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BulkPriceResult {
    pub product_id: String,
    pub old_price: Money,
    pub new_price: Money,
    pub difference: Money,
}

/// محاسبه‌ی پیش‌نمایش تغییر جمعی قیمت — پیش از اعمال، همیشه نمایش داده می‌شود.
///
/// قیمت منفی هرگز تولید نمی‌شود؛ اگر تغییر باعث منفی شدن شود، کل عملیات رد
/// می‌گردد (نه اینکه بی‌صدا صفر شود).
pub fn preview_bulk_price(
    products: &[(String, Money)],
    change: BulkPriceChange,
    /// گرد کردن به نزدیک‌ترین مضرب (مثلاً ۱۰۰۰ ریال). صفر یعنی بدون گرد کردن.
    round_to: i64,
) -> Result<Vec<BulkPriceResult>, BulkError> {
    if products.is_empty() {
        return Err(BulkError::EmptySelection);
    }
    if let BulkPriceChange::Percent(bp) = change {
        if !(-10_000..=100_000).contains(&bp) {
            return Err(BulkError::InvalidPercent);
        }
    }

    let mut results = Vec::with_capacity(products.len());
    for (product_id, old_price) in products {
        let raw = match change {
            BulkPriceChange::Percent(bp) => {
                let delta = old_price.percent_bp(bp.abs())?;
                if bp >= 0 {
                    old_price.checked_add(delta)?
                } else {
                    old_price.checked_sub(delta)?
                }
            }
            BulkPriceChange::Amount(amount) => old_price.checked_add(amount)?,
            BulkPriceChange::Set(amount) => amount,
        };
        if raw.is_negative() {
            return Err(BulkError::NegativeResult {
                product_id: product_id.clone(),
            });
        }
        let new_price = if round_to > 1 {
            let remainder = raw.rials() % round_to;
            let rounded = if remainder * 2 >= round_to {
                raw.rials() - remainder + round_to
            } else {
                raw.rials() - remainder
            };
            Money::from_rials(rounded)
        } else {
            raw
        };
        results.push(BulkPriceResult {
            product_id: product_id.clone(),
            old_price: *old_price,
            new_price,
            difference: new_price.checked_sub(*old_price)?,
        });
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// هشدار کم‌موجودی
// ---------------------------------------------------------------------------

/// یک قلم کم‌موجودی برای کارت «نزدیک به اتمام موجودی».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LowStockItem {
    pub product_id: String,
    pub product_name: String,
    pub quantity: f64,
    /// حد سفارش خود کالا (در صورت تعریف).
    pub reorder_point: f64,
}

/// یافتن کالاهای نزدیک به اتمام موجودی.
///
/// آستانه دو منبع دارد: حد سفارش خود کالا (اگر تعریف شده باشد) و آستانه‌ی
/// عمومی تنظیمات. هرکدام بزرگ‌تر باشد ملاک است تا هشدار زودتر داده شود.
pub fn low_stock_items(
    products: &[(String, String, f64, f64)],
    global_threshold: f64,
) -> Vec<LowStockItem> {
    let mut items: Vec<LowStockItem> = products
        .iter()
        .filter_map(|(id, name, quantity, reorder_point)| {
            let threshold = reorder_point.max(global_threshold);
            if *quantity <= threshold {
                Some(LowStockItem {
                    product_id: id.clone(),
                    product_name: name.clone(),
                    quantity: *quantity,
                    reorder_point: *reorder_point,
                })
            } else {
                None
            }
        })
        .collect();
    // کم‌موجودترین‌ها اول
    items.sort_by(|a, b| {
        a.quantity
            .partial_cmp(&b.quantity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
}
