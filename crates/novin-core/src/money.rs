//! نوع پولی دقیق (بدون خطای ممیز شناور).
//!
//! ## چرا؟
//! نگهداری مبلغ با `f64` در نرم‌افزار حسابداری منجر به خطای انباشته می‌شود
//! (مثال کلاسیک: `0.1 + 0.2 != 0.3`). در پلتفرم نوین پرداز واحد داخلی **ریال**
//! است و به‌صورت عدد صحیح ۶۴ بیتی نگهداری می‌شود. تومان فقط واحد نمایش/ورودی است.
//!
//! ظرفیت `i64` تا حدود ۹.۲ کوینتیلیون ریال است؛ برای هر کسب‌وکار ایرانی کفایت می‌کند.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// نسبت ریال به تومان.
pub const RIALS_PER_TOMAN: i64 = 10;

/// مبلغ مالی بر حسب **ریال**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Money(i64);

/// خطاهای عملیات پولی.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("MON-001: سرریز مبلغ")]
    Overflow,
    #[error("MON-002: مبلغ نامعتبر است")]
    Invalid,
    #[error("MON-003: تعداد نامعتبر است")]
    InvalidQuantity,
}

impl Money {
    pub const ZERO: Money = Money(0);

    #[inline]
    pub const fn from_rials(rials: i64) -> Self {
        Money(rials)
    }

    #[inline]
    pub const fn rials(self) -> i64 {
        self.0
    }

    /// ساخت مبلغ از تومان (واحد نمایش برای کاربر ایرانی).
    pub fn from_tomans(tomans: i64) -> Result<Self, MoneyError> {
        tomans
            .checked_mul(RIALS_PER_TOMAN)
            .map(Money)
            .ok_or(MoneyError::Overflow)
    }

    /// تومان کامل (بخش صحیح) — برای نمایش.
    #[inline]
    pub const fn tomans(self) -> i64 {
        self.0 / RIALS_PER_TOMAN
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[inline]
    pub const fn abs(self) -> Self {
        Money(self.0.abs())
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_add(other.0).map(Money).ok_or(MoneyError::Overflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.0.checked_sub(other.0).map(Money).ok_or(MoneyError::Overflow)
    }

    /// ضرب مبلغ در تعداد (تعداد می‌تواند کسری باشد: ۲.۵ کیلوگرم).
    ///
    /// گرد کردن: نصف به سمت بالا در قدر مطلق (half-away-from-zero) — رایج‌ترین
    /// قاعده در فاکتورهای ایرانی.
    pub fn mul_quantity(self, quantity: f64) -> Result<Self, MoneyError> {
        if !quantity.is_finite() {
            return Err(MoneyError::InvalidQuantity);
        }
        let raw = self.0 as f64 * quantity;
        if !raw.is_finite() || raw.abs() > i64::MAX as f64 {
            return Err(MoneyError::Overflow);
        }
        Ok(Money(round_half_away(raw)))
    }

    /// درصد گرفتن با دقت پایه‌نقطه (basis point): ۹٪ مالیات = ۹۰۰ bp.
    ///
    /// محاسبه با حساب ۱۲۸ بیتی انجام می‌شود تا سرریز میانی رخ ندهد.
    pub fn percent_bp(self, basis_points: i64) -> Result<Self, MoneyError> {
        let raw = (self.0 as i128) * (basis_points as i128);
        let scaled = div_round_half_away(raw, 10_000);
        i64::try_from(scaled).map(Money).map_err(|_| MoneyError::Overflow)
    }

    /// تقسیم مبلغ بین چند سهم بر اساس وزن، **بدون گم‌شدن حتی یک ریال**.
    ///
    /// کاربرد: پخش تخفیف سرجمع فاکتور یا هزینه‌ی حمل روی سطرها.
    /// باقیمانده با روش «بزرگ‌ترین باقیمانده» توزیع می‌شود تا جمع سهم‌ها
    /// دقیقاً برابر مبلغ اصلی بماند.
    pub fn allocate(self, weights: &[i64]) -> Result<Vec<Money>, MoneyError> {
        if weights.is_empty() {
            return Err(MoneyError::Invalid);
        }
        if weights.iter().any(|w| *w < 0) {
            return Err(MoneyError::Invalid);
        }
        let total: i128 = weights.iter().map(|w| *w as i128).sum();
        if total == 0 {
            return Err(MoneyError::Invalid);
        }
        let amount = self.0 as i128;
        let mut shares = Vec::with_capacity(weights.len());
        let mut remainders: Vec<(i128, usize)> = Vec::with_capacity(weights.len());
        let mut distributed: i128 = 0;
        for (index, weight) in weights.iter().enumerate() {
            let numerator = amount * (*weight as i128);
            let share = numerator.div_euclid(total);
            let remainder = numerator.rem_euclid(total);
            shares.push(share);
            remainders.push((remainder, index));
            distributed += share;
        }
        let mut leftover = amount - distributed;
        // باقیمانده همیشه هم‌علامت با مبلغ و کوچک‌تر از تعداد سهم‌هاست.
        remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut cursor = 0usize;
        while leftover != 0 && !remainders.is_empty() {
            let (_, index) = remainders[cursor % remainders.len()];
            let step = if leftover > 0 { 1 } else { -1 };
            shares[index] += step;
            leftover -= step;
            cursor += 1;
        }
        shares
            .into_iter()
            .map(|s| i64::try_from(s).map(Money).map_err(|_| MoneyError::Overflow))
            .collect()
    }

    /// تبدیل رشته‌ی ورودی کاربر به مبلغ ریالی.
    ///
    /// پشتیبانی از ارقام فارسی/عربی، جداکننده‌ی هزارگان و علامت منفی.
    pub fn parse_rials(input: &str) -> Result<Self, MoneyError> {
        let normalized = normalize_digits(input);
        let cleaned: String = normalized
            .chars()
            .filter(|c| !matches!(c, ',' | '٬' | ' ' | '\u{200c}' | '_'))
            .collect();
        let trimmed = cleaned.trim();
        if trimmed.is_empty() {
            return Err(MoneyError::Invalid);
        }
        trimmed.parse::<i64>().map(Money).map_err(|_| MoneyError::Invalid)
    }

    /// نمایش با جداکننده‌ی هزارگان (ارقام لاتین؛ جهت‌دهی با UI است).
    pub fn format_grouped(self) -> String {
        let negative = self.0 < 0;
        let digits = self.0.unsigned_abs().to_string();
        let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
        for (index, ch) in digits.chars().enumerate() {
            if index > 0 && (digits.len() - index) % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        if negative {
            format!("-{out}")
        } else {
            out
        }
    }
}

/// تبدیل ارقام فارسی/عربی به لاتین.
pub fn normalize_digits(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '۰'..='۹' => char::from(b'0' + (c as u32 - '۰' as u32) as u8),
            '٠'..='٩' => char::from(b'0' + (c as u32 - '٠' as u32) as u8),
            other => other,
        })
        .collect()
}

fn round_half_away(value: f64) -> i64 {
    if value >= 0.0 {
        (value + 0.5).floor() as i64
    } else {
        (value - 0.5).ceil() as i64
    }
}

fn div_round_half_away(numerator: i128, denominator: i128) -> i128 {
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

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}
impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}
impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.0 -= rhs.0;
    }
}
impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        Money(iter.map(|m| m.0).sum())
    }
}
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_grouped())
    }
}
impl serde::Serialize for Money {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.0)
    }
}
impl<'de> serde::Deserialize<'de> for Money {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        i64::deserialize(deserializer).map(Money)
    }
}
impl rusqlite::types::FromSql for Money {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        i64::column_result(value).map(Money)
    }
}
impl rusqlite::ToSql for Money {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.0))
    }
}
