//! موتور حسابداری دوطرفه (Double Entry) — منطق خالص و قابل تست.
//!
//! هیچ سندی نباید نامتعادل ثبت شود. این ماژول تنها مرجع اعتبارسنجی سند و
//! محاسبه‌ی مبالغ فاکتور است؛ لایه‌ی IPC فقط آن را صدا می‌زند.

use crate::money::{Money, MoneyError};
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی حسابداری. کد خطا برای پشتیبانی، متن برای کاربر.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AccountingError {
    #[error("ACC-001: سند بدون سطر قابل ثبت نیست")]
    EmptyJournal,
    #[error("ACC-002: سند باید حداقل دو سطر داشته باشد")]
    SingleLine,
    #[error("ACC-003: هر سطر باید فقط بدهکار یا فقط بستانکار باشد")]
    BothSidesOnLine,
    #[error("ACC-004: سطر بدون مبلغ مجاز نیست")]
    ZeroLine,
    #[error("ACC-005: مبلغ منفی در سطر سند مجاز نیست")]
    NegativeAmount,
    #[error("ACC-006: سند نامتعادل است؛ اختلاف بدهکار و بستانکار: {difference} ریال")]
    Unbalanced { difference: i64 },
    #[error("ACC-007: حساب سطر سند مشخص نشده است")]
    MissingAccount,
    #[error("ACC-008: تعداد در سطر فاکتور باید بزرگ‌تر از صفر باشد")]
    InvalidQuantity,
    #[error("ACC-009: تخفیف نمی‌تواند از مبلغ ناخالص سطر بیشتر باشد")]
    DiscountTooLarge,
    #[error("ACC-010: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
}

/// یک سطر سند حسابداری.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLine {
    pub account_id: String,
    /// تفصیلی شناور (اختیاری): شخص، پروژه، مرکز هزینه و…
    #[serde(default)]
    pub subsidiary_id: Option<String>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub debit: Money,
    pub credit: Money,
    #[serde(default)]
    pub description: Option<String>,
}

impl JournalLine {
    pub fn debit(account_id: impl Into<String>, amount: Money) -> Self {
        JournalLine {
            account_id: account_id.into(),
            subsidiary_id: None,
            cost_center_id: None,
            project_id: None,
            debit: amount,
            credit: Money::ZERO,
            description: None,
        }
    }

    pub fn credit(account_id: impl Into<String>, amount: Money) -> Self {
        JournalLine {
            account_id: account_id.into(),
            subsidiary_id: None,
            cost_center_id: None,
            project_id: None,
            debit: Money::ZERO,
            credit: amount,
            description: None,
        }
    }
}

/// جمع‌های یک سند.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct JournalTotals {
    pub total_debit: Money,
    pub total_credit: Money,
}

/// اعتبارسنجی کامل سند: ساختار سطرها + تعادل بدهکار/بستانکار.
///
/// این تابع دروازه‌ی اجباری ثبت هر سند است.
pub fn validate_journal(lines: &[JournalLine]) -> Result<JournalTotals, AccountingError> {
    if lines.is_empty() {
        return Err(AccountingError::EmptyJournal);
    }
    if lines.len() < 2 {
        return Err(AccountingError::SingleLine);
    }
    let mut total_debit = Money::ZERO;
    let mut total_credit = Money::ZERO;
    for line in lines {
        if line.account_id.trim().is_empty() {
            return Err(AccountingError::MissingAccount);
        }
        if line.debit.is_negative() || line.credit.is_negative() {
            return Err(AccountingError::NegativeAmount);
        }
        if !line.debit.is_zero() && !line.credit.is_zero() {
            return Err(AccountingError::BothSidesOnLine);
        }
        if line.debit.is_zero() && line.credit.is_zero() {
            return Err(AccountingError::ZeroLine);
        }
        total_debit = total_debit.checked_add(line.debit)?;
        total_credit = total_credit.checked_add(line.credit)?;
    }
    if total_debit != total_credit {
        return Err(AccountingError::Unbalanced {
            difference: total_debit.rials() - total_credit.rials(),
        });
    }
    Ok(JournalTotals {
        total_debit,
        total_credit,
    })
}

/// ساخت سند برگشتی (معکوس) — تنها راه مجاز اصلاح سند ثبت‌شده.
///
/// سند اصلی هرگز حذف یا ویرایش نمی‌شود؛ عکس آن ثبت می‌شود و اثر مالی خنثی می‌گردد.
pub fn build_reversal(lines: &[JournalLine]) -> Result<Vec<JournalLine>, AccountingError> {
    validate_journal(lines)?;
    Ok(lines
        .iter()
        .map(|line| JournalLine {
            account_id: line.account_id.clone(),
            subsidiary_id: line.subsidiary_id.clone(),
            cost_center_id: line.cost_center_id.clone(),
            project_id: line.project_id.clone(),
            debit: line.credit,
            credit: line.debit,
            description: line.description.clone(),
        })
        .collect())
}

/// سطر فاکتور (فروش یا خرید) پیش از محاسبه.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceLineInput {
    pub product_id: String,
    pub quantity: f64,
    pub unit_price: Money,
    /// تخفیف سطری (مبلغ ریالی).
    #[serde(default)]
    pub line_discount: Money,
    /// نرخ مالیات بر ارزش افزوده بر حسب پایه‌نقطه (۹٪ = ۹۰۰).
    #[serde(default)]
    pub tax_basis_points: i64,
}

/// سطر فاکتور پس از محاسبه.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvoiceLineTotals {
    pub gross: Money,
    pub discount: Money,
    pub net: Money,
    pub tax: Money,
    pub total: Money,
}

/// جمع‌های فاکتور.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvoiceTotals {
    pub lines: Vec<InvoiceLineTotals>,
    pub subtotal: Money,
    pub discount: Money,
    pub tax: Money,
    pub total: Money,
}

/// محاسبه‌ی کامل فاکتور شامل تخفیف سطری، تخفیف سرجمع و مالیات.
///
/// تخفیف سرجمع به نسبت مبلغ خالص سطرها و **بدون گم شدن ریال** پخش می‌شود؛
/// مالیات پس از کسر تخفیف‌ها محاسبه می‌گردد (مطابق رویه‌ی مالیاتی ایران).
pub fn calculate_invoice(
    lines: &[InvoiceLineInput],
    header_discount: Money,
) -> Result<InvoiceTotals, AccountingError> {
    if lines.is_empty() {
        return Err(AccountingError::EmptyJournal);
    }
    if header_discount.is_negative() {
        return Err(AccountingError::NegativeAmount);
    }

    let mut gross_values = Vec::with_capacity(lines.len());
    let mut net_after_line_discount = Vec::with_capacity(lines.len());
    for line in lines {
        if !line.quantity.is_finite() || line.quantity <= 0.0 {
            return Err(AccountingError::InvalidQuantity);
        }
        if line.unit_price.is_negative() || line.line_discount.is_negative() {
            return Err(AccountingError::NegativeAmount);
        }
        let gross = line.unit_price.mul_quantity(line.quantity)?;
        if line.line_discount > gross {
            return Err(AccountingError::DiscountTooLarge);
        }
        gross_values.push(gross);
        net_after_line_discount.push(gross.checked_sub(line.line_discount)?);
    }

    let net_sum: Money = net_after_line_discount.iter().copied().sum();
    if header_discount > net_sum {
        return Err(AccountingError::DiscountTooLarge);
    }

    let header_shares = if header_discount.is_zero() {
        vec![Money::ZERO; lines.len()]
    } else {
        let weights: Vec<i64> = net_after_line_discount.iter().map(|m| m.rials()).collect();
        if weights.iter().all(|w| *w == 0) {
            vec![Money::ZERO; lines.len()]
        } else {
            header_discount.allocate(&weights)?
        }
    };

    let mut computed = Vec::with_capacity(lines.len());
    let mut subtotal = Money::ZERO;
    let mut discount_total = Money::ZERO;
    let mut tax_total = Money::ZERO;
    let mut grand_total = Money::ZERO;

    for (index, line) in lines.iter().enumerate() {
        let gross = gross_values[index];
        let discount = line.line_discount.checked_add(header_shares[index])?;
        let net = gross.checked_sub(discount)?;
        let tax = net.percent_bp(line.tax_basis_points)?;
        let total = net.checked_add(tax)?;
        subtotal = subtotal.checked_add(gross)?;
        discount_total = discount_total.checked_add(discount)?;
        tax_total = tax_total.checked_add(tax)?;
        grand_total = grand_total.checked_add(total)?;
        computed.push(InvoiceLineTotals {
            gross,
            discount,
            net,
            tax,
            total,
        });
    }

    Ok(InvoiceTotals {
        lines: computed,
        subtotal,
        discount: discount_total,
        tax: tax_total,
        total: grand_total,
    })
}

/// سند حسابداری خودکار فاکتور فروش.
///
/// بدهکار: حساب دریافتنی (مشتری) — بستانکار: درآمد فروش و مالیات پرداختنی.
pub fn sales_invoice_journal(
    receivable_account: &str,
    revenue_account: &str,
    tax_payable_account: &str,
    totals: &InvoiceTotals,
) -> Result<Vec<JournalLine>, AccountingError> {
    let net = totals.subtotal.checked_sub(totals.discount)?;
    let mut lines = vec![
        JournalLine::debit(receivable_account, totals.total),
        JournalLine::credit(revenue_account, net),
    ];
    if !totals.tax.is_zero() {
        lines.push(JournalLine::credit(tax_payable_account, totals.tax));
    }
    validate_journal(&lines)?;
    Ok(lines)
}

/// سند حسابداری خودکار فاکتور خرید.
pub fn purchase_invoice_journal(
    inventory_account: &str,
    tax_receivable_account: &str,
    payable_account: &str,
    totals: &InvoiceTotals,
) -> Result<Vec<JournalLine>, AccountingError> {
    let net = totals.subtotal.checked_sub(totals.discount)?;
    let mut lines = vec![JournalLine::debit(inventory_account, net)];
    if !totals.tax.is_zero() {
        lines.push(JournalLine::debit(tax_receivable_account, totals.tax));
    }
    lines.push(JournalLine::credit(payable_account, totals.total));
    validate_journal(&lines)?;
    Ok(lines)
}
