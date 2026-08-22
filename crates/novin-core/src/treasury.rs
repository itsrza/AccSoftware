//! خزانه‌داری: سند دریافت و پرداخت چندروشی، بانک، صندوق و دسته‌چک.
//!
//! مرجع: تصاویر `MZlUiD` (افزودن سند دریافت)، `p6hT01` (حساب‌های بانکی)،
//! `WLumbs` (صندوق‌ها) و منوی «اطلاعات پایه ← دسته چک».
//!
//! ## نکته‌ی کلیدی که در نسخه‌ی قبلی نبود
//!
//! در نرم‌افزار فعلی، یک سند دریافت می‌تواند **هم‌زمان** شامل نقد، چک، حواله و
//! کارتخوان باشد؛ جدول سطرهای سند دقیقاً برای همین است. این یعنی یک سند دریافت
//! یک سند حسابداری چندسطری تولید می‌کند، نه یک سطر ساده.

use crate::accounting::{validate_journal, AccountingError, JournalLine};
use crate::money::{Money, MoneyError};
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی خزانه.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TreasuryError {
    #[error("TRS-001: سند بدون سطر قابل ثبت نیست")]
    EmptyDocument,
    #[error("TRS-002: مبلغ هر سطر باید بزرگ‌تر از صفر باشد")]
    NonPositiveAmount,
    #[error("TRS-003: برای سطر چک، شماره و سررسید چک الزامی است")]
    MissingCheckDetails,
    #[error("TRS-004: برای این روش، انتخاب حساب خزانه الزامی است")]
    MissingTreasuryAccount,
    #[error("TRS-005: برای سطر کارتخوان، انتخاب پایانه الزامی است")]
    MissingTerminal,
    #[error("TRS-006: موجودی {account} منفی می‌شود: مانده {balance} ریال، برداشت {amount} ریال")]
    NegativeBalance {
        account: String,
        balance: i64,
        amount: i64,
    },
    #[error("TRS-007: شماره‌ی چک خارج از محدوده‌ی دسته‌چک است")]
    SerialOutOfRange,
    #[error("TRS-008: دسته‌چک تمام شده است")]
    CheckbookExhausted,
    #[error("TRS-009: این شماره چک قبلاً استفاده شده است: {serial}")]
    SerialAlreadyUsed { serial: i64 },
    #[error("TRS-010: محدوده‌ی دسته‌چک نامعتبر است")]
    InvalidCheckbookRange,
    #[error("TRS-011: طرف حساب سند مشخص نشده است")]
    MissingParty,
    #[error("TRS-012: خطای محاسبه‌ی مبلغ")]
    Money(#[from] MoneyError),
    #[error("TRS-013: {0}")]
    Accounting(#[from] AccountingError),
}

// ---------------------------------------------------------------------------
// روش‌های دریافت و پرداخت
// ---------------------------------------------------------------------------

/// روش تسویه در سطر سند — مطابق ستون «نوع عملیات» نرم‌افزار فعلی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    /// نقد (صندوق)
    Cash,
    /// چک
    Check,
    /// حواله بانکی
    BankTransfer,
    /// کارتخوان (پایانه فروشگاهی)
    CardTerminal,
    /// تخفیف نقدی هنگام تسویه
    Discount,
    /// تهاتر با حساب دیگر
    Offset,
}

impl PaymentMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentMethod::Cash => "cash",
            PaymentMethod::Check => "check",
            PaymentMethod::BankTransfer => "bank_transfer",
            PaymentMethod::CardTerminal => "card_terminal",
            PaymentMethod::Discount => "discount",
            PaymentMethod::Offset => "offset",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PaymentMethod::Cash => "نقد",
            PaymentMethod::Check => "چک",
            PaymentMethod::BankTransfer => "حواله",
            PaymentMethod::CardTerminal => "کارتخوان",
            PaymentMethod::Discount => "تخفیف",
            PaymentMethod::Offset => "تهاتر",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "cash" => PaymentMethod::Cash,
            "check" => PaymentMethod::Check,
            "bank_transfer" => PaymentMethod::BankTransfer,
            "card_terminal" => PaymentMethod::CardTerminal,
            "discount" => PaymentMethod::Discount,
            "offset" => PaymentMethod::Offset,
            _ => return None,
        })
    }

    /// آیا این روش موجودی حساب خزانه را جابه‌جا می‌کند؟
    ///
    /// تخفیف و تهاتر پولی جابه‌جا نمی‌کنند؛ فقط مانده‌ی طرف حساب را تغییر می‌دهند.
    pub fn moves_treasury(self) -> bool {
        matches!(
            self,
            PaymentMethod::Cash | PaymentMethod::BankTransfer | PaymentMethod::CardTerminal
        )
    }

    /// آیا این روش نیازمند حساب خزانه (صندوق/بانک) است؟
    pub fn requires_treasury_account(self) -> bool {
        self.moves_treasury()
    }
}

/// نوع سند خزانه.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    /// سند دریافت — پول یا تعهد از طرف حساب گرفته می‌شود.
    Receipt,
    /// سند پرداخت.
    Payment,
}

impl DocumentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentKind::Receipt => "receipt",
            DocumentKind::Payment => "payment",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DocumentKind::Receipt => "سند دریافت",
            DocumentKind::Payment => "سند پرداخت",
        }
    }
}

// ---------------------------------------------------------------------------
// سطر سند
// ---------------------------------------------------------------------------

/// مشخصات چک در سطر سند.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckDetails {
    pub serial: String,
    /// سررسید شمسی.
    pub due_date: String,
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub sayad_id: Option<String>,
}

/// یک سطر سند خزانه.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentLine {
    pub method: PaymentMethod,
    pub amount: Money,
    #[serde(default)]
    pub description: Option<String>,
    /// حساب خزانه (صندوق یا بانک) برای روش‌های نقدی.
    #[serde(default)]
    pub treasury_account: Option<String>,
    /// شناسه‌ی پایانه فروشگاهی برای کارتخوان.
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub check: Option<CheckDetails>,
}

impl DocumentLine {
    pub fn new(method: PaymentMethod, amount: Money) -> Self {
        DocumentLine {
            method,
            amount,
            description: None,
            treasury_account: None,
            terminal_id: None,
            check: None,
        }
    }

    pub fn with_account(mut self, account: impl Into<String>) -> Self {
        self.treasury_account = Some(account.into());
        self
    }
}

/// اعتبارسنجی یک سطر سند.
pub fn validate_line(line: &DocumentLine) -> Result<(), TreasuryError> {
    if line.amount.rials() <= 0 {
        return Err(TreasuryError::NonPositiveAmount);
    }
    if line.method.requires_treasury_account()
        && line
            .treasury_account
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(TreasuryError::MissingTreasuryAccount);
    }
    if line.method == PaymentMethod::CardTerminal
        && line
            .terminal_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(TreasuryError::MissingTerminal);
    }
    if line.method == PaymentMethod::Check {
        match &line.check {
            Some(details)
                if !details.serial.trim().is_empty() && !details.due_date.trim().is_empty() => {}
            _ => return Err(TreasuryError::MissingCheckDetails),
        }
    }
    Ok(())
}

/// جمع سند و تفکیک به‌ازای هر روش.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DocumentTotals {
    pub cash: Money,
    pub check: Money,
    pub bank_transfer: Money,
    pub card_terminal: Money,
    pub discount: Money,
    pub offset: Money,
    pub total: Money,
    /// جمع بخشی که واقعاً پول جابه‌جا می‌کند.
    pub treasury_movement: Money,
}

/// محاسبه‌ی جمع سند با اعتبارسنجی همه‌ی سطرها.
pub fn calculate_totals(lines: &[DocumentLine]) -> Result<DocumentTotals, TreasuryError> {
    if lines.is_empty() {
        return Err(TreasuryError::EmptyDocument);
    }
    let mut totals = DocumentTotals {
        cash: Money::ZERO,
        check: Money::ZERO,
        bank_transfer: Money::ZERO,
        card_terminal: Money::ZERO,
        discount: Money::ZERO,
        offset: Money::ZERO,
        total: Money::ZERO,
        treasury_movement: Money::ZERO,
    };
    for line in lines {
        validate_line(line)?;
        let bucket = match line.method {
            PaymentMethod::Cash => &mut totals.cash,
            PaymentMethod::Check => &mut totals.check,
            PaymentMethod::BankTransfer => &mut totals.bank_transfer,
            PaymentMethod::CardTerminal => &mut totals.card_terminal,
            PaymentMethod::Discount => &mut totals.discount,
            PaymentMethod::Offset => &mut totals.offset,
        };
        *bucket = bucket.checked_add(line.amount)?;
        totals.total = totals.total.checked_add(line.amount)?;
        if line.method.moves_treasury() {
            totals.treasury_movement = totals.treasury_movement.checked_add(line.amount)?;
        }
    }
    Ok(totals)
}

// ---------------------------------------------------------------------------
// سند حسابداری خزانه
// ---------------------------------------------------------------------------

/// حساب‌های موردنیاز برای صدور سند خزانه.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreasuryAccounts {
    /// حساب طرف حساب (دریافتنی یا پرداختنی).
    pub party_account: String,
    /// حساب اسناد دریافتنی (چک‌های دریافتی نزد صندوق).
    pub notes_receivable: String,
    /// حساب اسناد پرداختنی (چک‌های صادرشده).
    pub notes_payable: String,
    /// حساب تخفیف نقدی.
    pub discount_account: String,
}

/// ساخت سند حسابداری برای یک سند دریافت یا پرداخت چندروشی.
///
/// ```text
/// سند دریافت:  بدهکار صندوق/بانک/اسناد دریافتنی/تخفیف  ← بستانکار طرف حساب
/// سند پرداخت:  بدهکار طرف حساب  ← بستانکار صندوق/بانک/اسناد پرداختنی
/// ```
///
/// هر سطر سند خزانه یک سطر سند حسابداری می‌سازد؛ طرف مقابل یک سطر تجمیعی است.
pub fn build_journal(
    kind: DocumentKind,
    lines: &[DocumentLine],
    accounts: &TreasuryAccounts,
) -> Result<Vec<JournalLine>, TreasuryError> {
    let totals = calculate_totals(lines)?;
    if accounts.party_account.trim().is_empty() {
        return Err(TreasuryError::MissingParty);
    }

    let mut journal = Vec::with_capacity(lines.len() + 1);
    for line in lines {
        let account = match line.method {
            PaymentMethod::Cash | PaymentMethod::BankTransfer | PaymentMethod::CardTerminal => line
                .treasury_account
                .clone()
                .ok_or(TreasuryError::MissingTreasuryAccount)?,
            PaymentMethod::Check => match kind {
                DocumentKind::Receipt => accounts.notes_receivable.clone(),
                DocumentKind::Payment => accounts.notes_payable.clone(),
            },
            PaymentMethod::Discount => accounts.discount_account.clone(),
            PaymentMethod::Offset => accounts.party_account.clone(),
        };
        journal.push(match kind {
            DocumentKind::Receipt => JournalLine::debit(account, line.amount),
            DocumentKind::Payment => JournalLine::credit(account, line.amount),
        });
    }

    journal.push(match kind {
        DocumentKind::Receipt => JournalLine::credit(&accounts.party_account, totals.total),
        DocumentKind::Payment => JournalLine::debit(&accounts.party_account, totals.total),
    });

    validate_journal(&journal)?;
    Ok(journal)
}

// ---------------------------------------------------------------------------
// سیاست منفی شدن موجودی
// ---------------------------------------------------------------------------

/// رفتار سیستم هنگام منفی شدن موجودی صندوق یا بانک.
///
/// مرجع: بخش «هشدار منفی شدن موجودی» در فرم تعریف بانک و صندوق.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NegativeBalancePolicy {
    /// خطا — عملیات انجام نمی‌شود.
    Error,
    /// هشدار — عملیات انجام می‌شود ولی پیام داده می‌شود.
    Warn,
    /// بی‌تأثیر.
    Ignore,
}

impl NegativeBalancePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            NegativeBalancePolicy::Error => "error",
            NegativeBalancePolicy::Warn => "warn",
            NegativeBalancePolicy::Ignore => "ignore",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NegativeBalancePolicy::Error => "خطا",
            NegativeBalancePolicy::Warn => "هشدار",
            NegativeBalancePolicy::Ignore => "بی‌تأثیر",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "error" => NegativeBalancePolicy::Error,
            "ignore" => NegativeBalancePolicy::Ignore,
            _ => NegativeBalancePolicy::Warn,
        }
    }
}

/// نتیجه‌ی بررسی موجودی.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BalanceCheck {
    /// مجاز و بدون هشدار.
    Allowed,
    /// مجاز ولی با هشدار برای نمایش به کاربر.
    Warning(String),
}

/// بررسی برداشت از حساب خزانه بر اساس سیاست تعریف‌شده.
pub fn check_withdrawal(
    account_name: &str,
    current_balance: Money,
    amount: Money,
    policy: NegativeBalancePolicy,
) -> Result<BalanceCheck, TreasuryError> {
    if amount.rials() <= 0 {
        return Err(TreasuryError::NonPositiveAmount);
    }
    let projected = current_balance.checked_sub(amount)?;
    if projected.rials() >= 0 {
        return Ok(BalanceCheck::Allowed);
    }
    match policy {
        NegativeBalancePolicy::Error => Err(TreasuryError::NegativeBalance {
            account: account_name.to_string(),
            balance: current_balance.rials(),
            amount: amount.rials(),
        }),
        NegativeBalancePolicy::Warn => Ok(BalanceCheck::Warning(format!(
            "موجودی {account_name} پس از این عملیات {} ریال منفی می‌شود.",
            projected.abs().rials()
        ))),
        NegativeBalancePolicy::Ignore => Ok(BalanceCheck::Allowed),
    }
}

// ---------------------------------------------------------------------------
// دسته‌چک
// ---------------------------------------------------------------------------

/// یک دسته‌چک با محدوده‌ی شماره‌ی سریال.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkbook {
    pub id: String,
    pub bank_account_id: String,
    pub serial_from: i64,
    pub serial_to: i64,
    /// شماره‌های استفاده‌شده (صادرشده یا باطل‌شده).
    #[serde(default)]
    pub used_serials: Vec<i64>,
}

impl Checkbook {
    pub fn validate(&self) -> Result<(), TreasuryError> {
        if self.serial_from <= 0 || self.serial_to < self.serial_from {
            return Err(TreasuryError::InvalidCheckbookRange);
        }
        Ok(())
    }

    /// تعداد کل برگ‌های دسته‌چک.
    pub fn capacity(&self) -> i64 {
        (self.serial_to - self.serial_from + 1).max(0)
    }

    /// تعداد برگ‌های باقی‌مانده.
    pub fn remaining(&self) -> i64 {
        self.capacity() - self.used_serials.len() as i64
    }

    /// نخستین شماره‌ی آزاد دسته‌چک.
    pub fn next_serial(&self) -> Result<i64, TreasuryError> {
        self.validate()?;
        let used: std::collections::BTreeSet<i64> = self.used_serials.iter().copied().collect();
        (self.serial_from..=self.serial_to)
            .find(|serial| !used.contains(serial))
            .ok_or(TreasuryError::CheckbookExhausted)
    }

    /// ثبت استفاده از یک شماره‌ی مشخص.
    pub fn use_serial(&mut self, serial: i64) -> Result<(), TreasuryError> {
        self.validate()?;
        if serial < self.serial_from || serial > self.serial_to {
            return Err(TreasuryError::SerialOutOfRange);
        }
        if self.used_serials.contains(&serial) {
            return Err(TreasuryError::SerialAlreadyUsed { serial });
        }
        self.used_serials.push(serial);
        Ok(())
    }
}
