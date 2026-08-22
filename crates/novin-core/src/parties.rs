//! اشخاص: انواع شخصیت، نقش‌ها، اعتبارسنجی هویتی ایرانی و مانده‌ی حساب.
//!
//! مرجع: تصاویر `c9pvYl` (لیست اشخاص) و `1zkKV5` (فرم افزودن شخص) و بخش
//! «حساب‌های بانکی» در `p6hT01` — `docs/FEATURE_BASELINE.md` بخش‌های ۲ و ۴.
//!
//! این ماژول شامل الگوریتم‌های رسمی اعتبارسنجی ایران است: کد ملی، شناسه ملی
//! اشخاص حقوقی، شماره شبا و شماره کارت بانکی. ورود داده‌ی هویتی نامعتبر یکی از
//! رایج‌ترین منابع خطا در نرم‌افزارهای حسابداری است و اینجا در مرز ورودی گرفته
//! می‌شود، نه بعداً در گزارش مالیاتی.

use crate::money::Money;
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی اشخاص.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PartyError {
    #[error("PRT-001: نام شخص نمی‌تواند خالی باشد")]
    EmptyName,
    #[error("PRT-002: کد ملی نامعتبر است")]
    InvalidNationalId,
    #[error("PRT-003: شناسه ملی شخص حقوقی نامعتبر است")]
    InvalidLegalId,
    #[error("PRT-004: کد اقتصادی نامعتبر است")]
    InvalidEconomicCode,
    #[error("PRT-005: کد پستی باید ۱۰ رقم باشد")]
    InvalidPostalCode,
    #[error("PRT-006: شماره موبایل نامعتبر است")]
    InvalidMobile,
    #[error("PRT-007: شماره شبا نامعتبر است")]
    InvalidIban,
    #[error("PRT-008: شماره کارت بانکی نامعتبر است")]
    InvalidCardNumber,
    #[error("PRT-009: شخص حقوقی باید نام شرکت داشته باشد")]
    MissingCompanyName,
    #[error("PRT-010: سقف اعتبار نمی‌تواند منفی باشد")]
    NegativeCreditLimit,
    #[error(
        "PRT-011: سقف اعتبار مشتری تمام شده است؛ مانده بدهی: {balance} ریال، سقف: {limit} ریال"
    )]
    CreditLimitExceeded { balance: i64, limit: i64 },
    #[error("PRT-012: شخص باید حداقل یکی از نقش‌های مشتری یا تأمین‌کننده را داشته باشد")]
    NoCommercialRole,
}

// ---------------------------------------------------------------------------
// انواع شخصیت و نقش
// ---------------------------------------------------------------------------

/// نوع شخصیت — مطابق چهار گزینه‌ی رادیویی فرم افزودن شخص.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyType {
    /// حقیقی
    Natural,
    /// حقوقی غیردولتی
    PrivateLegal,
    /// حقوقی دولتی
    GovernmentLegal,
    /// مشارکت مدنی
    CivilPartnership,
}

impl PartyType {
    pub fn as_str(self) -> &'static str {
        match self {
            PartyType::Natural => "natural",
            PartyType::PrivateLegal => "private_legal",
            PartyType::GovernmentLegal => "government_legal",
            PartyType::CivilPartnership => "civil_partnership",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PartyType::Natural => "حقیقی",
            PartyType::PrivateLegal => "حقوقی غیردولتی",
            PartyType::GovernmentLegal => "حقوقی دولتی",
            PartyType::CivilPartnership => "مشارکت مدنی",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "natural" => PartyType::Natural,
            "private_legal" => PartyType::PrivateLegal,
            "government_legal" => PartyType::GovernmentLegal,
            "civil_partnership" => PartyType::CivilPartnership,
            _ => return None,
        })
    }

    /// آیا این نوع شخصیت، حقوقی است؟ (شناسه ملی به‌جای کد ملی)
    pub fn is_legal_entity(self) -> bool {
        !matches!(self, PartyType::Natural)
    }
}

/// نقش سازمانی شخص — مطابق سه گزینه‌ی بالای فرم.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyFunction {
    /// شخص عادی (مشتری / تأمین‌کننده)
    Person,
    /// بازاریاب
    Marketer,
    /// سوپروایزر
    Supervisor,
}

impl PartyFunction {
    pub fn as_str(self) -> &'static str {
        match self {
            PartyFunction::Person => "person",
            PartyFunction::Marketer => "marketer",
            PartyFunction::Supervisor => "supervisor",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PartyFunction::Person => "شخص",
            PartyFunction::Marketer => "بازاریاب",
            PartyFunction::Supervisor => "سوپروایزر",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "person" => PartyFunction::Person,
            "marketer" => PartyFunction::Marketer,
            "supervisor" => PartyFunction::Supervisor,
            _ => return None,
        })
    }

    /// آیا این نقش می‌تواند پورسانت فروش بگیرد؟
    pub fn earns_commission(self) -> bool {
        matches!(self, PartyFunction::Marketer | PartyFunction::Supervisor)
    }
}

// ---------------------------------------------------------------------------
// اعتبارسنجی هویتی ایرانی
// ---------------------------------------------------------------------------

fn digits_only(input: &str) -> String {
    crate::money::normalize_digits(input)
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect()
}

/// اعتبارسنجی **کد ملی** ایرانی (۱۰ رقمی) با رقم کنترل رسمی.
///
/// کدهایی مانند `1111111111` که رقم کنترلشان تصادفاً درست است ولی در واقعیت
/// صادر نمی‌شوند، صریحاً رد می‌شوند.
pub fn national_id_is_valid(input: &str) -> bool {
    let digits = digits_only(input);
    if digits.len() != 10 {
        return false;
    }
    let bytes: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if bytes.iter().all(|digit| *digit == bytes[0]) {
        return false;
    }
    let sum: u32 = (0..9).map(|index| bytes[index] * (10 - index as u32)).sum();
    let remainder = sum % 11;
    let expected = if remainder < 2 {
        remainder
    } else {
        11 - remainder
    };
    bytes[9] == expected
}

/// اعتبارسنجی **شناسه ملی اشخاص حقوقی** (۱۱ رقمی).
pub fn legal_id_is_valid(input: &str) -> bool {
    let digits = digits_only(input);
    if digits.len() != 11 {
        return false;
    }
    let bytes: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if bytes.iter().all(|digit| *digit == bytes[0]) {
        return false;
    }
    let offset = bytes[9] + 2;
    const WEIGHTS: [u32; 10] = [29, 27, 23, 19, 17, 29, 27, 23, 19, 17];
    let sum: u32 = (0..10)
        .map(|index| (bytes[index] + offset) * WEIGHTS[index])
        .sum();
    let remainder = sum % 11;
    let expected = if remainder == 10 { 0 } else { remainder };
    bytes[10] == expected
}

/// اعتبارسنجی کد اقتصادی (۱۲ رقمی طبق فرمت جاری سازمان امور مالیاتی).
pub fn economic_code_is_valid(input: &str) -> bool {
    let digits = digits_only(input);
    digits.len() == 12 && !digits.chars().all(|c| c == digits.as_bytes()[0] as char)
}

/// اعتبارسنجی کد پستی ایران (۱۰ رقم، بدون صفر در ابتدا و بدون رقم تکراری کامل).
pub fn postal_code_is_valid(input: &str) -> bool {
    let digits = digits_only(input);
    digits.len() == 10
        && !digits.starts_with('0')
        && !digits.chars().all(|c| c == digits.as_bytes()[0] as char)
}

/// یکسان‌سازی شماره موبایل به قالب `09xxxxxxxxx`.
///
/// ورودی‌های `+989…`، `00989…`، `989…` و ارقام فارسی همگی پذیرفته می‌شوند.
pub fn normalize_mobile(input: &str) -> Option<String> {
    let digits = digits_only(input);
    let national = if let Some(rest) = digits.strip_prefix("0098") {
        format!("0{rest}")
    } else if let Some(rest) = digits.strip_prefix("98") {
        if digits.len() == 12 {
            format!("0{rest}")
        } else {
            digits.clone()
        }
    } else {
        digits.clone()
    };
    if national.len() == 11 && national.starts_with("09") {
        Some(national)
    } else if national.len() == 10 && national.starts_with('9') {
        Some(format!("0{national}"))
    } else {
        None
    }
}

/// اعتبارسنجی **شماره شبا** با الگوریتم استاندارد mod-97.
///
/// شبای ایران با `IR` شروع می‌شود و مجموعاً ۲۶ نویسه دارد.
pub fn iban_is_valid(input: &str) -> bool {
    let normalized: String = crate::money::normalize_digits(input)
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_uppercase();
    if normalized.len() != 26 || !normalized.starts_with("IR") {
        return false;
    }
    if !normalized[2..].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // چهار نویسه‌ی اول به انتها منتقل می‌شود و حروف به عدد تبدیل می‌شوند.
    let rearranged = format!("{}{}", &normalized[4..], &normalized[..4]);
    let mut remainder: u32 = 0;
    for character in rearranged.chars() {
        let value = if character.is_ascii_digit() {
            character.to_digit(10).unwrap_or(0)
        } else {
            character as u32 - 'A' as u32 + 10
        };
        // اعداد دو رقمی حروف باید رقم‌به‌رقم وارد شوند.
        remainder = if value > 9 {
            ((remainder * 10 + value / 10) % 97 * 10 + value % 10) % 97
        } else {
            (remainder * 10 + value) % 97
        };
    }
    remainder == 1
}

/// اعتبارسنجی شماره کارت بانکی ۱۶ رقمی با الگوریتم Luhn.
pub fn card_number_is_valid(input: &str) -> bool {
    let digits = digits_only(input);
    if digits.len() != 16 {
        return false;
    }
    let sum: u32 = digits
        .chars()
        .rev()
        .filter_map(|c| c.to_digit(10))
        .enumerate()
        .map(|(index, digit)| {
            if index % 2 == 1 {
                let doubled = digit * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                digit
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

// ---------------------------------------------------------------------------
// تعریف شخص
// ---------------------------------------------------------------------------

/// اطلاعات پایه‌ی یک شخص برای اعتبارسنجی.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyDefinition {
    pub code: String,
    pub party_type: PartyType,
    #[serde(default = "default_function")]
    pub function: PartyFunction,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    #[serde(default)]
    pub national_id: Option<String>,
    #[serde(default)]
    pub economic_code: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub is_customer: bool,
    #[serde(default)]
    pub is_supplier: bool,
    /// سقف اعتبار به ریال؛ صفر یعنی بدون محدودیت.
    #[serde(default)]
    pub credit_limit: i64,
    /// مسیر پخش مویرگی.
    #[serde(default)]
    pub route: Option<String>,
    /// بازاریاب مسئول این شخص.
    #[serde(default)]
    pub marketer_code: Option<String>,
}

fn default_function() -> PartyFunction {
    PartyFunction::Person
}

impl PartyDefinition {
    /// نام نمایشی: برای حقیقی «نام + نام خانوادگی»، برای حقوقی نام شرکت.
    pub fn display_name(&self) -> String {
        if self.party_type.is_legal_entity() {
            return self.company_name.clone().unwrap_or_default();
        }
        [self.first_name.as_deref(), self.last_name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    /// اعتبارسنجی کامل شخص پیش از ذخیره.
    pub fn validate(&self) -> Result<(), PartyError> {
        if self.display_name().is_empty() {
            return Err(if self.party_type.is_legal_entity() {
                PartyError::MissingCompanyName
            } else {
                PartyError::EmptyName
            });
        }
        // بازاریاب و سوپروایزر لزوماً طرف حساب تجاری نیستند.
        if self.function == PartyFunction::Person && !self.is_customer && !self.is_supplier {
            return Err(PartyError::NoCommercialRole);
        }
        if let Some(identifier) = self.national_id.as_deref().filter(|v| !v.is_empty()) {
            let valid = if self.party_type.is_legal_entity() {
                legal_id_is_valid(identifier)
            } else {
                national_id_is_valid(identifier)
            };
            if !valid {
                return Err(if self.party_type.is_legal_entity() {
                    PartyError::InvalidLegalId
                } else {
                    PartyError::InvalidNationalId
                });
            }
        }
        if let Some(code) = self.economic_code.as_deref().filter(|v| !v.is_empty()) {
            if !economic_code_is_valid(code) {
                return Err(PartyError::InvalidEconomicCode);
            }
        }
        if let Some(code) = self.postal_code.as_deref().filter(|v| !v.is_empty()) {
            if !postal_code_is_valid(code) {
                return Err(PartyError::InvalidPostalCode);
            }
        }
        if let Some(mobile) = self.mobile.as_deref().filter(|v| !v.is_empty()) {
            if normalize_mobile(mobile).is_none() {
                return Err(PartyError::InvalidMobile);
            }
        }
        if self.credit_limit < 0 {
            return Err(PartyError::NegativeCreditLimit);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// مانده‌ی حساب
// ---------------------------------------------------------------------------

/// وضعیت مانده‌ی یک شخص.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BalanceStatus {
    /// بدهکار
    Debtor,
    /// بستانکار
    Creditor,
    /// بی‌حساب
    Settled,
}

impl BalanceStatus {
    /// نشانگر کوتاه ستون «حساب فعلی» در لیست اشخاص.
    pub fn indicator(self) -> &'static str {
        match self {
            BalanceStatus::Debtor => "بد",
            BalanceStatus::Creditor => "بس",
            BalanceStatus::Settled => "بی حساب",
        }
    }

    pub fn of(balance: Money) -> Self {
        if balance.rials() > 0 {
            BalanceStatus::Debtor
        } else if balance.rials() < 0 {
            BalanceStatus::Creditor
        } else {
            BalanceStatus::Settled
        }
    }
}

/// خلاصه‌ی حساب اشخاص — معادل پنل کناری لیست اشخاص.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BalanceSummary {
    pub debtor_count: usize,
    pub debtor_total: Money,
    pub creditor_count: usize,
    pub creditor_total: Money,
    pub settled_count: usize,
    pub total_count: usize,
    /// خالص مانده (بدهکار مثبت، بستانکار منفی).
    pub net_total: Money,
}

/// محاسبه‌ی خلاصه‌ی حساب از روی مانده‌ی اشخاص.
///
/// قرارداد علامت: مانده‌ی مثبت = بدهکار، منفی = بستانکار.
pub fn summarize_balances(balances: &[Money]) -> BalanceSummary {
    let mut summary = BalanceSummary {
        debtor_count: 0,
        debtor_total: Money::ZERO,
        creditor_count: 0,
        creditor_total: Money::ZERO,
        settled_count: 0,
        total_count: balances.len(),
        net_total: Money::ZERO,
    };
    for balance in balances {
        summary.net_total += *balance;
        match BalanceStatus::of(*balance) {
            BalanceStatus::Debtor => {
                summary.debtor_count += 1;
                summary.debtor_total += *balance;
            }
            BalanceStatus::Creditor => {
                summary.creditor_count += 1;
                summary.creditor_total += balance.abs();
            }
            BalanceStatus::Settled => summary.settled_count += 1,
        }
    }
    summary
}

/// کنترل سقف اعتبار پیش از ثبت فروش نسیه.
///
/// `credit_limit` برابر صفر یعنی بدون محدودیت.
pub fn check_credit_limit(
    current_balance: Money,
    credit_limit: i64,
    additional_debt: Money,
) -> Result<(), PartyError> {
    if credit_limit < 0 {
        return Err(PartyError::NegativeCreditLimit);
    }
    if credit_limit == 0 {
        return Ok(());
    }
    let projected = current_balance.rials() + additional_debt.rials();
    if projected > credit_limit {
        return Err(PartyError::CreditLimitExceeded {
            balance: projected,
            limit: credit_limit,
        });
    }
    Ok(())
}

/// مبلغ باقی‌مانده تا سقف اعتبار (برای نمایش هشدار در فاکتور).
pub fn remaining_credit(current_balance: Money, credit_limit: i64) -> Option<Money> {
    if credit_limit <= 0 {
        return None;
    }
    Some(Money::from_rials(credit_limit - current_balance.rials()))
}
