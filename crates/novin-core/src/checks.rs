//! چرخه‌ی حیات چک و محاسبات مرتبط.
//!
//! مبنای این ماژول تحلیل مستقیم صفحه‌ی «دفتر اسناد دریافتنی/پرداختنی» نرم‌افزار
//! فعلی نوین پرداز است (`docs/FEATURE_BASELINE.md` بخش ۵).
//!
//! دو قاعده‌ی حسابداری که اینجا تضمین می‌شوند:
//!
//! ۱. **چک انتظامی اثر مالی ندارد.** چک تضمینی/امانی در حساب‌ها ثبت نمی‌شود و
//!    فقط به‌صورت یادداشت انتظامی نگهداری می‌گردد.
//! ۲. **گذار وضعیت آزاد نیست.** مثلاً چکی که وصول شده نمی‌تواند دوباره واگذار شود.
//!    همه‌ی گذارها از این ماشین حالت عبور می‌کنند.

use crate::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی چک.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CheckError {
    #[error("CHK-101: گذار وضعیت چک مجاز نیست: از «{from}» به «{to}»")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("CHK-102: این وضعیت برای چک {kind} تعریف نشده است")]
    StatusNotAllowedForKind { kind: &'static str },
    #[error("CHK-103: مبلغ چک باید بزرگ‌تر از صفر باشد")]
    InvalidAmount,
    #[error("CHK-104: سررسید چک نمی‌تواند پیش از تاریخ صدور باشد")]
    DueBeforeIssue,
    #[error("CHK-105: فهرست چک برای راس‌گیری خالی است")]
    EmptyPortfolio,
    #[error("CHK-106: چک انتظامی اثر مالی ندارد و قابل ثبت در خزانه نیست")]
    MemoHasNoFinancialEffect,
}

/// نوع چک.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// چک دریافتی از مشتریان.
    Received,
    /// چک پرداختی (چک شخصی شرکت).
    Issued,
}

impl CheckKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckKind::Received => "received",
            CheckKind::Issued => "issued",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CheckKind::Received => "دریافتی",
            CheckKind::Issued => "پرداختی",
        }
    }
}

/// وضعیت چک — دقیقاً مطابق زبانه‌های نرم‌افزار فعلی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// موجود در صندوق (چک دریافتی).
    InHand,
    /// واگذار شده به بانک جهت وصول.
    Deposited,
    /// وصول شده.
    Collected,
    /// نقد شده.
    Cashed,
    /// خرج شده (ظهرنویسی به شخص ثالث).
    Endorsed,
    /// برگشتی.
    Bounced,
    /// عودت شده به پردازنده / گیرنده.
    Returned,
    /// باطل شده.
    Void,
    /// پرداختی در جریان (چک شخصی صادرشده و تسویه‌نشده).
    Outstanding,
    /// پرداخت شده.
    Paid,
    /// انتظامی موجود — بدون اثر مالی.
    MemoInHand,
    /// انتظامی عودت شده — بدون اثر مالی.
    MemoReturned,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::InHand => "in_hand",
            CheckStatus::Deposited => "deposited",
            CheckStatus::Collected => "collected",
            CheckStatus::Cashed => "cashed",
            CheckStatus::Endorsed => "endorsed",
            CheckStatus::Bounced => "bounced",
            CheckStatus::Returned => "returned",
            CheckStatus::Void => "void",
            CheckStatus::Outstanding => "outstanding",
            CheckStatus::Paid => "paid",
            CheckStatus::MemoInHand => "memo_in_hand",
            CheckStatus::MemoReturned => "memo_returned",
        }
    }

    /// عنوان فارسی — همان برچسب زبانه‌ها در نرم‌افزار فعلی.
    pub fn label(self) -> &'static str {
        match self {
            CheckStatus::InHand => "موجود",
            CheckStatus::Deposited => "واگذار شده",
            CheckStatus::Collected => "وصول شده",
            CheckStatus::Cashed => "نقد شده",
            CheckStatus::Endorsed => "خرج شده",
            CheckStatus::Bounced => "برگشتی",
            CheckStatus::Returned => "عودت شده",
            CheckStatus::Void => "باطل شده",
            CheckStatus::Outstanding => "پرداختی",
            CheckStatus::Paid => "پرداخت شده",
            CheckStatus::MemoInHand => "انتظامی موجود",
            CheckStatus::MemoReturned => "انتظامی عودت شده",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "in_hand" => CheckStatus::InHand,
            "deposited" => CheckStatus::Deposited,
            "collected" => CheckStatus::Collected,
            "cashed" => CheckStatus::Cashed,
            "endorsed" => CheckStatus::Endorsed,
            "bounced" => CheckStatus::Bounced,
            "returned" => CheckStatus::Returned,
            "void" => CheckStatus::Void,
            "outstanding" => CheckStatus::Outstanding,
            "paid" => CheckStatus::Paid,
            "memo_in_hand" => CheckStatus::MemoInHand,
            "memo_returned" => CheckStatus::MemoReturned,
            _ => return None,
        })
    }

    /// آیا این وضعیت انتظامی (بدون اثر مالی) است؟
    pub fn is_memo(self) -> bool {
        matches!(self, CheckStatus::MemoInHand | CheckStatus::MemoReturned)
    }

    /// آیا چرخه‌ی چک در این وضعیت پایان یافته است؟
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            CheckStatus::Collected
                | CheckStatus::Cashed
                | CheckStatus::Paid
                | CheckStatus::Void
                | CheckStatus::Returned
        )
    }

    /// آیا چک هنوز جزو دارایی/بدهی جاری شرکت است؟
    ///
    /// چک‌های انتظامی و وضعیت‌های پایانی در مانده‌ی خزانه دیده نمی‌شوند.
    pub fn is_open(self) -> bool {
        !self.is_terminal() && !self.is_memo()
    }

    /// وضعیت آغازین بر اساس نوع چک.
    pub fn initial(kind: CheckKind, memo: bool) -> Self {
        match (kind, memo) {
            (_, true) => CheckStatus::MemoInHand,
            (CheckKind::Received, false) => CheckStatus::InHand,
            (CheckKind::Issued, false) => CheckStatus::Outstanding,
        }
    }
}

/// گذارهای مجاز از یک وضعیت، بر اساس نوع چک.
pub fn allowed_transitions(kind: CheckKind, status: CheckStatus) -> &'static [CheckStatus] {
    use CheckStatus::*;
    match (kind, status) {
        // --- چک دریافتی ---
        (CheckKind::Received, InHand) => &[Deposited, Endorsed, Cashed, Returned, Void],
        (CheckKind::Received, Deposited) => &[Collected, Bounced],
        (CheckKind::Received, Endorsed) => &[Bounced],
        // چک وصول‌شده هم می‌تواند برگشت بخورد: بانک مبلغ را از حساب کسر می‌کند.
        // این تنها گذار خروجی از وضعیت‌های پایانی است و اثر مالی معکوس دارد.
        (CheckKind::Received, Collected | Cashed) => &[Bounced],
        (CheckKind::Received, Bounced) => &[InHand, Deposited, Returned],
        // --- چک پرداختی ---
        (CheckKind::Issued, Outstanding) => &[Paid, Bounced, Returned, Void],
        (CheckKind::Issued, Bounced) => &[Outstanding, Paid, Returned],
        // --- انتظامی (هر دو نوع) ---
        (_, MemoInHand) => &[MemoReturned, Void],
        (_, MemoReturned) => &[MemoInHand],
        // --- وضعیت‌های پایانی ---
        _ => &[],
    }
}

/// آیا وضعیت برای این نوع چک اصلاً معنا دارد؟
pub fn status_belongs_to_kind(kind: CheckKind, status: CheckStatus) -> bool {
    use CheckStatus::*;
    if status.is_memo() {
        return true;
    }
    match kind {
        CheckKind::Received => matches!(
            status,
            InHand | Deposited | Collected | Cashed | Endorsed | Bounced | Returned | Void
        ),
        CheckKind::Issued => matches!(status, Outstanding | Paid | Bounced | Returned | Void),
    }
}

/// اعمال گذار وضعیت با اعتبارسنجی کامل.
pub fn transition(
    kind: CheckKind,
    from: CheckStatus,
    to: CheckStatus,
) -> Result<CheckStatus, CheckError> {
    if !status_belongs_to_kind(kind, to) {
        return Err(CheckError::StatusNotAllowedForKind {
            kind: kind.as_str(),
        });
    }
    if allowed_transitions(kind, from).contains(&to) {
        Ok(to)
    } else {
        Err(CheckError::InvalidTransition {
            from: from.as_str(),
            to: to.as_str(),
        })
    }
}

/// اثر مالی یک گذار بر خزانه.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TreasuryEffect {
    /// افزایش موجودی (وصول یا نقد کردن چک دریافتی).
    Increase,
    /// کاهش موجودی (پرداخت چک شخصی).
    Decrease,
    /// بدون اثر مالی.
    None,
}

/// اثر خزانه‌ای گذار وضعیت — منبع واحد حقیقت برای صدور سند خودکار.
pub fn treasury_effect(kind: CheckKind, from: CheckStatus, to: CheckStatus) -> TreasuryEffect {
    if from.is_memo() || to.is_memo() {
        return TreasuryEffect::None;
    }
    match (kind, to) {
        (CheckKind::Received, CheckStatus::Collected | CheckStatus::Cashed) => {
            TreasuryEffect::Increase
        }
        (CheckKind::Issued, CheckStatus::Paid) => TreasuryEffect::Decrease,
        // برگشت چکی که قبلاً وصول شده بود، اثر معکوس دارد.
        (CheckKind::Received, CheckStatus::Bounced)
            if matches!(from, CheckStatus::Collected | CheckStatus::Cashed) =>
        {
            TreasuryEffect::Decrease
        }
        _ => TreasuryEffect::None,
    }
}

/// یک قلم چک برای محاسبات سبد.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckItem {
    pub amount: Money,
    pub due_date: NaiveDate,
}

impl CheckItem {
    pub fn new(amount: Money, due_date: NaiveDate) -> Self {
        CheckItem { amount, due_date }
    }

    /// فاصله‌ی روز تا سررسید نسبت به یک تاریخ مبنا (منفی = گذشته).
    pub fn days_to_due(&self, base: NaiveDate) -> i64 {
        (self.due_date - base).num_days()
    }
}

/// نتیجه‌ی راس‌گیری.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MaturityAverage {
    /// راس بر حسب روز نسبت به تاریخ مبنا (منفی = گذشته).
    pub days: i64,
    /// جمع مبالغ سبد.
    pub total_amount: Money,
    /// تعداد فقره.
    pub count: usize,
}

/// راس‌گیری وزنی چک‌ها — معادل ابزار «راس‌گیری» نرم‌افزار فعلی.
///
/// فرمول استاندارد مالی: `راس = Σ(مبلغ × روز) / Σ(مبلغ)`
///
/// نتیجه به نزدیک‌ترین روز گرد می‌شود (نصف به سمت بالا در قدر مطلق) تا با
/// نمایش عدد صحیح روز در رابط کاربری سازگار باشد.
pub fn weighted_maturity(
    base: NaiveDate,
    items: &[CheckItem],
) -> Result<MaturityAverage, CheckError> {
    if items.is_empty() {
        return Err(CheckError::EmptyPortfolio);
    }
    let total: i128 = items.iter().map(|item| item.amount.rials() as i128).sum();
    if total == 0 {
        return Err(CheckError::InvalidAmount);
    }
    let weighted: i128 = items
        .iter()
        .map(|item| item.amount.rials() as i128 * item.days_to_due(base) as i128)
        .sum();
    let days = divide_round_half_away(weighted, total);
    Ok(MaturityAverage {
        days: days as i64,
        total_amount: Money::from_rials(total as i64),
        count: items.len(),
    })
}

/// تاریخ راس سبد چک — تاریخی که پرداخت یک‌جای کل مبلغ در آن، معادل مالی سبد است.
pub fn maturity_date(base: NaiveDate, items: &[CheckItem]) -> Result<NaiveDate, CheckError> {
    let average = weighted_maturity(base, items)?;
    base.checked_add_signed(chrono::Duration::days(average.days))
        .ok_or(CheckError::EmptyPortfolio)
}

fn divide_round_half_away(numerator: i128, denominator: i128) -> i128 {
    let negative = (numerator < 0) ^ (denominator < 0);
    let n = numerator.abs();
    let d = denominator.abs();
    let quotient = (n * 2 + d) / (d * 2);
    if negative {
        -quotient
    } else {
        quotient
    }
}

/// اعتبارسنجی داده‌های پایه‌ی یک چک هنگام ثبت.
pub fn validate_check(
    amount: Money,
    issue_date: NaiveDate,
    due_date: NaiveDate,
) -> Result<(), CheckError> {
    if amount.rials() <= 0 {
        return Err(CheckError::InvalidAmount);
    }
    if due_date < issue_date {
        return Err(CheckError::DueBeforeIssue);
    }
    Ok(())
}

/// چک‌های نزدیک سررسید برای هشدار و پیامک یادآوری.
pub fn due_within(
    base: NaiveDate,
    horizon_days: i64,
    items: &[(CheckItem, CheckStatus)],
) -> Vec<&CheckItem> {
    items
        .iter()
        .filter(|(item, status)| {
            if !status.is_open() {
                return false;
            }
            let days = item.days_to_due(base);
            (0..=horizon_days).contains(&days)
        })
        .map(|(item, _)| item)
        .collect()
}
