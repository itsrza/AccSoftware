//! صدور فاکتور: تخفیف پلکانی، پورسانت، عوارض، کرایه، سود فاکتور و اقساط.
//!
//! مرجع: تصاویر `sFpxWK` (فاکتور فروش)، `PI5uot` (فاکتور خرید) و `FRPBDr`
//! (برگشت از فروش) — `docs/FEATURE_BASELINE.md` بخش ۶.
//!
//! این ماژول روی `accounting` سوار است و همان تضمین بنیادی را حفظ می‌کند:
//! **جمع اجزای هر سطر دقیقاً برابر جمع فاکتور است و حتی یک ریال گم نمی‌شود.**

use crate::jalali;
use crate::money::{Money, MoneyError};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی فاکتور.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvoiceError {
    #[error("INV-201: فاکتور بدون سطر قابل ثبت نیست")]
    EmptyInvoice,
    #[error("INV-202: مقدار سطر فاکتور باید بزرگ‌تر از صفر باشد")]
    InvalidQuantity,
    #[error("INV-203: مبلغ منفی در فاکتور مجاز نیست")]
    NegativeAmount,
    #[error("INV-204: تخفیف از مبلغ سطر بیشتر است")]
    DiscountTooLarge,
    #[error("INV-205: نرخ درصدی نامعتبر است")]
    InvalidRate,
    #[error("INV-206: تعداد سریال‌های واردشده با مقدار سطر برابر نیست: {expected} مورد لازم است، {actual} وارد شده")]
    SerialCountMismatch { expected: usize, actual: usize },
    #[error("INV-207: سریال تکراری در فاکتور: {serial}")]
    DuplicateSerial { serial: String },
    #[error("INV-208: کوپن تخفیف برای این فاکتور معتبر نیست")]
    CouponNotApplicable,
    #[error("INV-209: تعداد اقساط باید بین ۱ تا ۱۲۰ باشد")]
    InvalidInstallmentCount,
    #[error("INV-210: پیش‌پرداخت نمی‌تواند از مبلغ فاکتور بیشتر باشد")]
    DownPaymentTooLarge,
    #[error("INV-211: تاریخ سررسید قسط نامعتبر است")]
    InvalidInstallmentDate,
    #[error("INV-212: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
}

// ---------------------------------------------------------------------------
// تخفیف پلکانی
// ---------------------------------------------------------------------------

/// یک پله‌ی تخفیف: از این مقدار به بالا، این درصد تخفیف اعمال می‌شود.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DiscountTier {
    pub min_quantity: f64,
    /// درصد تخفیف بر حسب پایه‌نقطه (۵٪ = ۵۰۰).
    pub discount_bp: i64,
}

/// یافتن درصد تخفیف پلکانی مناسب برای یک مقدار.
///
/// بالاترین پله‌ای که مقدار به آن رسیده باشد انتخاب می‌شود؛ ترتیب ورودی مهم نیست.
pub fn resolve_tier_discount(tiers: &[DiscountTier], quantity: f64) -> i64 {
    tiers
        .iter()
        .filter(|tier| {
            tier.min_quantity.is_finite() && quantity + f64::EPSILON >= tier.min_quantity
        })
        .max_by(|a, b| {
            a.min_quantity
                .partial_cmp(&b.min_quantity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|tier| tier.discount_bp)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// کوپن تخفیف
// ---------------------------------------------------------------------------

/// نوع کوپن تخفیف.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouponKind {
    /// درصدی (بر حسب پایه‌نقطه)
    Percent(i64),
    /// مبلغ ثابت
    Amount(Money),
}

/// کوپن تخفیف — بخش «اعمال کوپن تخفیف» فاکتور فروش.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coupon {
    pub code: String,
    pub kind: CouponKind,
    /// حداقل مبلغ فاکتور برای فعال شدن کوپن.
    #[serde(default)]
    pub minimum_invoice: Option<Money>,
    /// سقف تخفیف کوپن درصدی.
    #[serde(default)]
    pub maximum_discount: Option<Money>,
}

impl Coupon {
    /// مبلغ تخفیف این کوپن روی یک مبلغ پایه.
    pub fn discount_for(&self, base: Money) -> Result<Money, InvoiceError> {
        if let Some(minimum) = self.minimum_invoice {
            if base < minimum {
                return Err(InvoiceError::CouponNotApplicable);
            }
        }
        let raw = match self.kind {
            CouponKind::Percent(bp) => {
                if !(0..=10_000).contains(&bp) {
                    return Err(InvoiceError::InvalidRate);
                }
                base.percent_bp(bp)?
            }
            CouponKind::Amount(amount) => {
                if amount.is_negative() {
                    return Err(InvoiceError::NegativeAmount);
                }
                amount
            }
        };
        let capped = match self.maximum_discount {
            Some(maximum) if raw > maximum => maximum,
            _ => raw,
        };
        Ok(if capped > base { base } else { capped })
    }
}

// ---------------------------------------------------------------------------
// سطر فاکتور
// ---------------------------------------------------------------------------

/// نحوه‌ی برخورد با کرایه حمل.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreightMode {
    /// کرایه پس از محاسبه‌ی مالیات به جمع فاکتور اضافه می‌شود.
    ///
    /// یعنی **روی کرایه مالیات بسته نمی‌شود**. مناسب وقتی کرایه را شرکت
    /// حمل‌ونقل مستقیم از خریدار می‌گیرد و شما فقط واسط هستید.
    AddToTotal,
    /// کرایه به نسبت مبلغ، روی سطرها سرشکن و وارد مأخذ مالیات می‌شود.
    ///
    /// یعنی **روی کرایه هم مالیات بسته می‌شود**. مناسب وقتی کرایه بخشی از
    /// بهای فروش است و در صورتحساب رسمی باید مشمول ارزش افزوده باشد.
    ///
    /// انتخاب اشتباه بین این دو، مبلغ مالیات اظهارنامه را عوض می‌کند.
    AllocateToLines,
}

/// یک سطر فاکتور پیش از محاسبه.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub product_id: String,
    pub quantity: f64,
    pub unit_price: Money,
    /// تخفیف سطری به‌صورت مبلغ.
    #[serde(default)]
    pub discount_amount: Money,
    /// تخفیف سطری به‌صورت درصد (پایه‌نقطه) — با مبلغ جمع می‌شود.
    #[serde(default)]
    pub discount_bp: i64,
    /// پله‌های تخفیف بر اساس مقدار.
    #[serde(default)]
    pub tiers: Vec<DiscountTier>,
    /// نرخ ارزش افزوده (پایه‌نقطه).
    #[serde(default)]
    pub vat_bp: i64,
    /// نرخ عوارض (پایه‌نقطه).
    #[serde(default)]
    pub duty_bp: i64,
    /// درصد پورسانت بازاریاب (پایه‌نقطه).
    #[serde(default)]
    pub commission_bp: i64,
    /// بهای تمام‌شده‌ی واحد برای محاسبه‌ی سود.
    #[serde(default)]
    pub unit_cost: Money,
    /// سریال‌های کالا (در صورت سریال‌دار بودن).
    #[serde(default)]
    pub serials: Vec<String>,
    /// آیا این کالا سریال‌دار است؟
    #[serde(default)]
    pub serial_tracked: bool,
}

impl InvoiceLine {
    pub fn new(product_id: impl Into<String>, quantity: f64, unit_price: Money) -> Self {
        InvoiceLine {
            product_id: product_id.into(),
            quantity,
            unit_price,
            discount_amount: Money::ZERO,
            discount_bp: 0,
            tiers: Vec::new(),
            vat_bp: 0,
            duty_bp: 0,
            commission_bp: 0,
            unit_cost: Money::ZERO,
            serials: Vec::new(),
            serial_tracked: false,
        }
    }
}

/// سربرگ فاکتور.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceInput {
    pub lines: Vec<InvoiceLine>,
    /// تخفیف سرجمع فاکتور.
    #[serde(default)]
    pub header_discount: Money,
    #[serde(default)]
    pub coupon: Option<Coupon>,
    #[serde(default)]
    pub freight: Money,
    #[serde(default = "default_freight_mode")]
    pub freight_mode: FreightMode,
}

fn default_freight_mode() -> FreightMode {
    FreightMode::AddToTotal
}

/// سطر فاکتور پس از محاسبه — تفکیک کامل برای نمایش و صدور سند.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ComputedLine {
    /// مبلغ ناخالص = مقدار × فی واحد
    pub gross: Money,
    /// تخفیف پلکانی
    pub tier_discount: Money,
    /// تخفیف سطری (مبلغ + درصد)
    pub line_discount: Money,
    /// سهم این سطر از تخفیف سرجمع
    pub header_discount_share: Money,
    /// سهم این سطر از کوپن
    pub coupon_share: Money,
    /// جمع همه‌ی تخفیف‌های این سطر
    pub total_discount: Money,
    /// مبلغ خالص پس از تخفیف
    pub net: Money,
    /// سهم کرایه حمل (در حالت سرشکن)
    pub freight_share: Money,
    pub duty: Money,
    pub vat: Money,
    /// مبلغ نهایی سطر
    pub total: Money,
    /// پورسانت بازاریاب
    pub commission: Money,
    /// بهای تمام‌شده
    pub cost: Money,
    /// سود ناخالص سطر
    pub profit: Money,
}

/// جمع‌های فاکتور.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvoiceResult {
    pub lines: Vec<ComputedLine>,
    pub subtotal: Money,
    pub discount_total: Money,
    pub net_total: Money,
    pub freight: Money,
    pub duty_total: Money,
    pub vat_total: Money,
    pub total: Money,
    pub commission_total: Money,
    pub cost_total: Money,
    /// سود ناخالص فاکتور — دکمه‌ی «محاسبه سود فاکتور».
    pub profit: Money,
    /// حاشیه‌ی سود بر حسب پایه‌نقطه نسبت به فروش خالص.
    pub profit_margin_bp: i64,
}

fn validate_rate(rate: i64) -> Result<(), InvoiceError> {
    if !(0..=10_000).contains(&rate) {
        return Err(InvoiceError::InvalidRate);
    }
    Ok(())
}

/// اعتبارسنجی سریال‌های فاکتور: تعداد برابر مقدار و بدون تکرار در کل فاکتور.
pub fn validate_serials(lines: &[InvoiceLine]) -> Result<(), InvoiceError> {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for line in lines {
        if !line.serial_tracked {
            continue;
        }
        let expected = line.quantity.round() as usize;
        if line.serials.len() != expected {
            return Err(InvoiceError::SerialCountMismatch {
                expected,
                actual: line.serials.len(),
            });
        }
        for serial in &line.serials {
            if !seen.insert(serial.as_str()) {
                return Err(InvoiceError::DuplicateSerial {
                    serial: serial.clone(),
                });
            }
        }
    }
    Ok(())
}

/// محاسبه‌ی کامل فاکتور.
///
/// ترتیب اعمال — مطابق رویه‌ی نرم‌افزارهای حسابداری ایرانی:
/// ۱. مبلغ ناخالص  ۲. تخفیف پلکانی  ۳. تخفیف سطری  ۴. تخفیف سرجمع (سرشکن)
/// ۵. کوپن (سرشکن)  ۶. کرایه  ۷. عوارض  ۸. ارزش افزوده
pub fn calculate(input: &InvoiceInput) -> Result<InvoiceResult, InvoiceError> {
    if input.lines.is_empty() {
        return Err(InvoiceError::EmptyInvoice);
    }
    if input.header_discount.is_negative() || input.freight.is_negative() {
        return Err(InvoiceError::NegativeAmount);
    }
    validate_serials(&input.lines)?;

    // --- گام ۱ تا ۳: ناخالص و تخفیف‌های سطری ---
    let mut gross_values = Vec::with_capacity(input.lines.len());
    let mut tier_discounts = Vec::with_capacity(input.lines.len());
    let mut line_discounts = Vec::with_capacity(input.lines.len());
    let mut after_line = Vec::with_capacity(input.lines.len());

    for line in &input.lines {
        if !line.quantity.is_finite() || line.quantity <= 0.0 {
            return Err(InvoiceError::InvalidQuantity);
        }
        if line.unit_price.is_negative()
            || line.discount_amount.is_negative()
            || line.unit_cost.is_negative()
        {
            return Err(InvoiceError::NegativeAmount);
        }
        validate_rate(line.vat_bp)?;
        validate_rate(line.duty_bp)?;
        validate_rate(line.commission_bp)?;
        validate_rate(line.discount_bp)?;

        let gross = line.unit_price.mul_quantity(line.quantity)?;
        let tier_bp = resolve_tier_discount(&line.tiers, line.quantity);
        validate_rate(tier_bp)?;
        let tier_discount = gross.percent_bp(tier_bp)?;
        let percent_discount = gross.percent_bp(line.discount_bp)?;
        let line_discount = percent_discount.checked_add(line.discount_amount)?;
        let combined = tier_discount.checked_add(line_discount)?;
        if combined > gross {
            return Err(InvoiceError::DiscountTooLarge);
        }
        gross_values.push(gross);
        tier_discounts.push(tier_discount);
        line_discounts.push(line_discount);
        after_line.push(gross.checked_sub(combined)?);
    }

    // --- گام ۴: تخفیف سرجمع، سرشکن به نسبت مبلغ خالص سطرها ---
    let net_sum: Money = after_line.iter().copied().sum();
    if input.header_discount > net_sum {
        return Err(InvoiceError::DiscountTooLarge);
    }
    let weights: Vec<i64> = after_line.iter().map(|value| value.rials()).collect();
    let header_shares = allocate_or_zero(input.header_discount, &weights)?;

    // --- گام ۵: کوپن روی مبلغ پس از تخفیف‌ها ---
    let after_header: Vec<Money> = after_line
        .iter()
        .zip(&header_shares)
        .map(|(value, share)| *value - *share)
        .collect();
    let coupon_base: Money = after_header.iter().copied().sum();
    let coupon_discount = match &input.coupon {
        Some(coupon) => coupon.discount_for(coupon_base)?,
        None => Money::ZERO,
    };
    let coupon_weights: Vec<i64> = after_header.iter().map(|value| value.rials()).collect();
    let coupon_shares = allocate_or_zero(coupon_discount, &coupon_weights)?;

    // --- گام ۶: کرایه حمل ---
    let freight_shares = match input.freight_mode {
        FreightMode::AllocateToLines => allocate_or_zero(input.freight, &coupon_weights)?,
        FreightMode::AddToTotal => vec![Money::ZERO; input.lines.len()],
    };

    // --- گام ۷ و ۸: عوارض، ارزش افزوده و جمع‌ها ---
    let mut computed = Vec::with_capacity(input.lines.len());
    let mut subtotal = Money::ZERO;
    let mut discount_total = Money::ZERO;
    let mut duty_total = Money::ZERO;
    let mut vat_total = Money::ZERO;
    let mut line_total_sum = Money::ZERO;
    let mut commission_total = Money::ZERO;
    let mut cost_total = Money::ZERO;
    let mut profit_total = Money::ZERO;
    let mut net_total = Money::ZERO;

    for (index, line) in input.lines.iter().enumerate() {
        let gross = gross_values[index];
        let tier_discount = tier_discounts[index];
        let line_discount = line_discounts[index];
        let header_share = header_shares[index];
        let coupon_share = coupon_shares[index];
        let freight_share = freight_shares[index];

        let total_discount = tier_discount
            .checked_add(line_discount)?
            .checked_add(header_share)?
            .checked_add(coupon_share)?;
        let net = gross.checked_sub(total_discount)?;
        let taxable = net.checked_add(freight_share)?;
        let duty = taxable.percent_bp(line.duty_bp)?;
        let vat = taxable.checked_add(duty)?.percent_bp(line.vat_bp)?;
        let total = taxable.checked_add(duty)?.checked_add(vat)?;
        let commission = net.percent_bp(line.commission_bp)?;
        let cost = line.unit_cost.mul_quantity(line.quantity)?;
        let profit = net.checked_sub(cost)?.checked_sub(commission)?;

        subtotal = subtotal.checked_add(gross)?;
        discount_total = discount_total.checked_add(total_discount)?;
        net_total = net_total.checked_add(net)?;
        duty_total = duty_total.checked_add(duty)?;
        vat_total = vat_total.checked_add(vat)?;
        line_total_sum = line_total_sum.checked_add(total)?;
        commission_total = commission_total.checked_add(commission)?;
        cost_total = cost_total.checked_add(cost)?;
        profit_total = profit_total.checked_add(profit)?;

        computed.push(ComputedLine {
            gross,
            tier_discount,
            line_discount,
            header_discount_share: header_share,
            coupon_share,
            total_discount,
            net,
            freight_share,
            duty,
            vat,
            total,
            commission,
            cost,
            profit,
        });
    }

    let total = match input.freight_mode {
        FreightMode::AddToTotal => line_total_sum.checked_add(input.freight)?,
        FreightMode::AllocateToLines => line_total_sum,
    };

    let profit_margin_bp = if net_total.rials() == 0 {
        0
    } else {
        (profit_total.rials() as i128 * 10_000 / net_total.rials() as i128) as i64
    };

    Ok(InvoiceResult {
        lines: computed,
        subtotal,
        discount_total,
        net_total,
        freight: input.freight,
        duty_total,
        vat_total,
        total,
        commission_total,
        cost_total,
        profit: profit_total,
        profit_margin_bp,
    })
}

fn allocate_or_zero(amount: Money, weights: &[i64]) -> Result<Vec<Money>, InvoiceError> {
    if amount.is_zero() || weights.iter().all(|weight| *weight == 0) {
        return Ok(vec![Money::ZERO; weights.len()]);
    }
    Ok(amount.allocate(weights)?)
}

// ---------------------------------------------------------------------------
// اقساط
// ---------------------------------------------------------------------------

/// یک قسط.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Installment {
    pub number: usize,
    pub due_date: NaiveDate,
    /// سررسید شمسی برای نمایش.
    pub due_date_jalali: String,
    pub amount: Money,
}

/// تولید جدول اقساط — بخش «قسط» فاکتور فروش.
///
/// سررسیدها **ماه شمسی** جلو می‌روند (نه ۳۰ روزه) و مجموع اقساط دقیقاً برابر
/// مبلغ باقی‌مانده پس از پیش‌پرداخت است؛ حتی یک ریال گرد نمی‌شود.
pub fn installment_plan(
    total: Money,
    down_payment: Money,
    count: usize,
    first_due: NaiveDate,
) -> Result<Vec<Installment>, InvoiceError> {
    if !(1..=120).contains(&count) {
        return Err(InvoiceError::InvalidInstallmentCount);
    }
    if down_payment.is_negative() || total.is_negative() {
        return Err(InvoiceError::NegativeAmount);
    }
    if down_payment > total {
        return Err(InvoiceError::DownPaymentTooLarge);
    }
    let remaining = total.checked_sub(down_payment)?;
    let weights = vec![1i64; count];
    let shares = remaining.allocate(&weights)?;

    let mut plan = Vec::with_capacity(count);
    for (index, amount) in shares.into_iter().enumerate() {
        let due_date = jalali::add_jalali_months(first_due, index as i32)
            .map_err(|_| InvoiceError::InvalidInstallmentDate)?;
        plan.push(Installment {
            number: index + 1,
            due_date,
            due_date_jalali: jalali::jalali_string(due_date),
            amount,
        });
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------
// نمایش زنده‌ی مانده‌ی طرف حساب
// ---------------------------------------------------------------------------

/// وضعیت مانده‌ی طرف حساب حین ثبت فاکتور — نوار پایین فرم فاکتور.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BalanceView {
    /// مانده پیش از فاکتور
    pub before: Money,
    /// اثر این فاکتور
    pub invoice_effect: Money,
    /// دریافتی همراه فاکتور
    pub received: Money,
    /// مانده پس از فاکتور
    pub after: Money,
    /// مانده‌ی خود فاکتور (تسویه‌نشده)
    pub invoice_remainder: Money,
}

/// محاسبه‌ی مانده‌ی زنده‌ی طرف حساب.
///
/// قرارداد علامت: مثبت = بدهکار.
pub fn balance_view(before: Money, invoice_total: Money, received: Money) -> BalanceView {
    BalanceView {
        before,
        invoice_effect: invoice_total,
        received,
        after: before + invoice_total - received,
        invoice_remainder: invoice_total - received,
    }
}

/// تفکیک روش‌های تسویه‌ی فاکتور (نقد، چک، حواله، کارتخوان).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementBreakdown {
    #[serde(default)]
    pub cash: Money,
    #[serde(default)]
    pub check: Money,
    #[serde(default)]
    pub transfer: Money,
    #[serde(default)]
    pub card: Money,
}

impl SettlementBreakdown {
    pub fn total(&self) -> Money {
        self.cash + self.check + self.transfer + self.card
    }

    /// اعتبارسنجی: هیچ روشی منفی نباشد و جمع از مبلغ فاکتور بیشتر نشود.
    pub fn validate(&self, invoice_total: Money) -> Result<(), InvoiceError> {
        for amount in [self.cash, self.check, self.transfer, self.card] {
            if amount.is_negative() {
                return Err(InvoiceError::NegativeAmount);
            }
        }
        if self.total() > invoice_total {
            return Err(InvoiceError::DiscountTooLarge);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// خطوط سند حسابداری فاکتور — با تفکیک مالیات و تخفیف
// ---------------------------------------------------------------------------

/// حساب‌های درگیر در ثبت فاکتور — همه از account_mappings می‌آیند.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoicePostingAccounts {
    /// حساب طرف حساب (مشتری/تأمین‌کننده)
    pub party: String,
    /// فروش (بستانکار) یا خرید (بدهکار)
    pub main: String,
    /// مالیات (پرداختنی در فروش / دریافتنی در خرید)
    pub tax: String,
    /// تخفیف (کاهنده فروش / کاهنده خرید)
    pub discount: String,
}

/// خط سند: (شناسه حساب، بدهکار، بستانکار)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingLine(pub String, pub i64, pub i64);

/// ساخت خطوط سندِ پست فاکتور با تفکیک فروش خالص/مالیات/تخفیف.
///
/// ```text
/// فروش:  بدهکار طرف‌حساب (total) │ بستانکار فروش (subtotal) + مالیات (tax)
///        + بدهکار تخفیف (discount) — درصورت مثبت‌بودن
/// خرید:  بدهکار خرید (subtotal) + مالیات (tax) │ بستانکار تخفیف + طرف‌حساب (total)
/// ```
///
/// چرا تخفیف خط جدا دارد: بدون آن، تخفیف در دل حساب فروش گم می‌شود و
/// گزارش سود و زیان و اظهارنامه مالیاتی از دیتابیس قابل استخراج نیست.
/// خطوط صفر ساخته نمی‌شوند و تراز بدهکار/بستانکار تضمین می‌شود.
pub fn invoice_posting_lines(
    sale: bool,
    subtotal: i64,
    discount: i64,
    tax: i64,
    accounts: &InvoicePostingAccounts,
) -> Result<Vec<PostingLine>, String> {
    if subtotal < 0 || discount < 0 || tax < 0 {
        return Err("ACC-021: مبالغ فاکتور نمی‌تواند منفی باشد".into());
    }
    if discount > subtotal {
        return Err("ACC-022: تخفیف بیشتر از مبلغ فاکتور است".into());
    }
    let total = subtotal - discount + tax;
    if total <= 0 {
        return Err("ACC-023: جمع فاکتور باید مثبت باشد".into());
    }
    let mut lines = Vec::new();
    if sale {
        lines.push(PostingLine(accounts.party.clone(), total, 0));
        lines.push(PostingLine(accounts.main.clone(), 0, subtotal));
        if tax > 0 {
            lines.push(PostingLine(accounts.tax.clone(), 0, tax));
        }
        if discount > 0 {
            lines.push(PostingLine(accounts.discount.clone(), discount, 0));
        }
    } else {
        lines.push(PostingLine(accounts.main.clone(), subtotal, 0));
        if tax > 0 {
            lines.push(PostingLine(accounts.tax.clone(), tax, 0));
        }
        if discount > 0 {
            lines.push(PostingLine(accounts.discount.clone(), 0, discount));
        }
        lines.push(PostingLine(accounts.party.clone(), 0, total));
    }
    let debit: i64 = lines.iter().map(|line| line.1).sum();
    let credit: i64 = lines.iter().map(|line| line.2).sum();
    if debit != credit {
        return Err(format!("ACC-002: جمع بدهکار ({debit}) با بستانکار ({credit}) برابر نیست"));
    }
    Ok(lines)
}
