//! کدینگ حساب‌ها، تفصیلی شناور و ابعاد مالی.
//!
//! مرجع: منوی «اطلاعات پایه ← کدینگ حساب‌ها» و زبانه‌ی «تنظیمات کدینگ» در
//! نرم‌افزار فعلی (`docs/FEATURE_BASELINE.md` بخش‌های ۱۰ و ۱۱).
//!
//! ## دو تصمیم معماری
//!
//! ۱. **تعداد سطوح Hard-code نمی‌شود.** طرح کدینگ یک داده‌ی پیکربندی است
//!    (`CodingScheme`) تا شرکت‌های مختلف بتوانند ساختار خودشان را داشته باشند و
//!    افزودن سطح پنجم در آینده نیازمند تغییر کد نباشد.
//!
//! ۲. **تفصیلی «شناور» است.** تفصیلی زیرمجموعه‌ی درخت حساب‌ها نیست؛ یک بُعد
//!    مستقل با کدینگ خودش است که به هر حسابی می‌تواند بچسبد — دقیقاً مانند
//!    «گروه تفصیلی: بانک‌ها / صندوق‌ها» در نرم‌افزار فعلی. همین ویژگی امکان
//!    افزودن مرکز هزینه، پروژه و شعبه را بدون تغییر ساختار سند فراهم می‌کند.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// خطاهای کدینگ و ابعاد مالی.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodingError {
    #[error("COD-001: کد حساب باید فقط رقم باشد")]
    NonNumericCode,
    #[error("COD-002: طول کد حساب با هیچ سطحی از طرح کدینگ سازگار نیست: {code}")]
    UnknownLevel { code: String },
    #[error("COD-003: حساب سطح ریشه والد ندارد")]
    RootHasNoParent,
    #[error("COD-004: حساب والد یافت نشد: {parent}")]
    MissingParent { parent: String },
    #[error("COD-005: کد حساب تکراری است: {code}")]
    DuplicateCode { code: String },
    #[error("COD-006: ماهیت حساب با والد آن سازگار نیست: {code}")]
    NatureConflict { code: String },
    #[error("COD-007: ظرفیت شماره‌گذاری این سطح تکمیل شده است")]
    LevelExhausted,
    #[error("COD-008: ثبت سند فقط روی حساب سطح آخر مجاز است: {code}")]
    NotPostable { code: String },
    #[error("COD-009: برای این حساب انتخاب تفصیلی الزامی است")]
    SubsidiaryRequired,
    #[error("COD-010: تفصیلی انتخاب‌شده به گروه تفصیلی مجاز این حساب تعلق ندارد")]
    SubsidiaryGroupMismatch,
    #[error("COD-011: برای این حساب انتخاب مرکز هزینه الزامی است")]
    CostCenterRequired,
    #[error("COD-012: برای این حساب انتخاب پروژه الزامی است")]
    ProjectRequired,
    #[error("COD-013: این حساب تفصیلی نمی‌پذیرد")]
    SubsidiaryNotAllowed,
}

/// ماهیت حساب.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountNature {
    Debit,
    Credit,
    Mixed,
}

impl AccountNature {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountNature::Debit => "debit",
            AccountNature::Credit => "credit",
            AccountNature::Mixed => "mixed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccountNature::Debit => "بدهکار",
            AccountNature::Credit => "بستانکار",
            AccountNature::Mixed => "دوطرفه",
        }
    }

    /// آیا ماهیت فرزند با ماهیت والد سازگار است؟
    ///
    /// والد دوطرفه هر ماهیتی را می‌پذیرد؛ در غیر این صورت باید یکسان باشند.
    pub fn accepts_child(self, child: AccountNature) -> bool {
        self == AccountNature::Mixed || self == child
    }
}

/// طرح کدینگ: عرض هر سطح بر حسب تعداد رقم.
///
/// پیش‌فرض `[1, 2, 2, 2]` یعنی گروه ۱ رقم، کل ۳ رقم، معین ۵ رقم، تفصیلی ۷ رقم —
/// سازگار با کدینگ نرم‌افزار فعلی (نمونه: `1103101`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingScheme {
    level_widths: Vec<u8>,
    level_titles: Vec<String>,
}

impl Default for CodingScheme {
    fn default() -> Self {
        CodingScheme {
            level_widths: vec![1, 2, 2, 2],
            level_titles: vec![
                "گروه".to_string(),
                "کل".to_string(),
                "معین".to_string(),
                "تفصیلی".to_string(),
            ],
        }
    }
}

impl CodingScheme {
    /// ساخت طرح دلخواه. حداقل یک سطح و هر عرض بین ۱ تا ۶ رقم.
    pub fn new(level_widths: Vec<u8>, level_titles: Vec<String>) -> Option<Self> {
        if level_widths.is_empty()
            || level_widths.len() != level_titles.len()
            || level_widths.iter().any(|w| *w == 0 || *w > 6)
        {
            return None;
        }
        Some(CodingScheme {
            level_widths,
            level_titles,
        })
    }

    /// تعداد سطوح طرح.
    pub fn depth(&self) -> usize {
        self.level_widths.len()
    }

    /// طول کد در هر سطح (تجمعی).
    pub fn code_length(&self, level: usize) -> Option<usize> {
        if level >= self.depth() {
            return None;
        }
        Some(
            self.level_widths[..=level]
                .iter()
                .map(|w| *w as usize)
                .sum(),
        )
    }

    /// عنوان سطح، برای نمایش در رابط کاربری.
    pub fn level_title(&self, level: usize) -> Option<&str> {
        self.level_titles.get(level).map(String::as_str)
    }

    /// تشخیص سطح یک کد از روی طول آن.
    pub fn level_of(&self, code: &str) -> Result<usize, CodingError> {
        if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(CodingError::NonNumericCode);
        }
        (0..self.depth())
            .find(|level| self.code_length(*level) == Some(code.len()))
            .ok_or_else(|| CodingError::UnknownLevel {
                code: code.to_string(),
            })
    }

    /// آیا این کد در آخرین سطح طرح است؟ فقط این حساب‌ها قابل ثبت سند هستند.
    pub fn is_leaf_level(&self, code: &str) -> Result<bool, CodingError> {
        Ok(self.level_of(code)? == self.depth() - 1)
    }

    /// کد والد یک حساب.
    pub fn parent_code(&self, code: &str) -> Result<String, CodingError> {
        let level = self.level_of(code)?;
        if level == 0 {
            return Err(CodingError::RootHasNoParent);
        }
        let parent_length = self
            .code_length(level - 1)
            .ok_or(CodingError::RootHasNoParent)?;
        Ok(code[..parent_length].to_string())
    }

    /// ساخت کد فرزند از روی کد والد و شماره‌ی ترتیبی.
    pub fn child_code(&self, parent: &str, serial: u32) -> Result<String, CodingError> {
        let parent_level = self.level_of(parent)?;
        let child_level = parent_level + 1;
        let width = *self
            .level_widths
            .get(child_level)
            .ok_or(CodingError::UnknownLevel {
                code: parent.to_string(),
            })? as usize;
        let capacity = 10u32.pow(width as u32);
        if serial == 0 || serial >= capacity {
            return Err(CodingError::LevelExhausted);
        }
        Ok(format!("{parent}{serial:0width$}"))
    }

    /// نخستین کد آزاد زیر یک والد.
    pub fn next_child_code(
        &self,
        parent: &str,
        existing: &[String],
    ) -> Result<String, CodingError> {
        let taken: BTreeSet<&str> = existing.iter().map(String::as_str).collect();
        let parent_level = self.level_of(parent)?;
        let width = *self
            .level_widths
            .get(parent_level + 1)
            .ok_or(CodingError::UnknownLevel {
                code: parent.to_string(),
            })? as usize;
        let capacity = 10u32.pow(width as u32);
        (1..capacity)
            .map(|serial| self.child_code(parent, serial))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .find(|candidate| !taken.contains(candidate.as_str()))
            .ok_or(CodingError::LevelExhausted)
    }
}

/// یک حساب در درخت کدینگ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountDefinition {
    pub code: String,
    pub title: String,
    pub nature: AccountNature,
    /// آیا انتخاب تفصیلی برای ثبت روی این حساب الزامی است؟
    #[serde(default)]
    pub requires_subsidiary: bool,
    /// گروه تفصیلی مجاز (تفصیلی شناور)؛ خالی یعنی هر گروهی مجاز است.
    #[serde(default)]
    pub subsidiary_group: Option<String>,
    #[serde(default)]
    pub requires_cost_center: bool,
    #[serde(default)]
    pub requires_project: bool,
}

impl AccountDefinition {
    pub fn new(code: impl Into<String>, title: impl Into<String>, nature: AccountNature) -> Self {
        AccountDefinition {
            code: code.into(),
            title: title.into(),
            nature,
            requires_subsidiary: false,
            subsidiary_group: None,
            requires_cost_center: false,
            requires_project: false,
        }
    }

    pub fn with_subsidiary_group(mut self, group: impl Into<String>) -> Self {
        self.subsidiary_group = Some(group.into());
        self.requires_subsidiary = true;
        self
    }
}

/// گره‌ی درخت حساب‌ها به‌همراه فرزندان.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountNode {
    pub account: AccountDefinition,
    pub level: usize,
    pub children: Vec<AccountNode>,
}

impl AccountNode {
    /// آیا این گره قابل ثبت سند است؟ (برگ درخت در آخرین سطح)
    pub fn is_postable(&self, scheme: &CodingScheme) -> bool {
        self.children.is_empty() && self.level == scheme.depth() - 1
    }
}

/// اعتبارسنجی کامل مجموعه حساب‌ها و ساخت درخت.
///
/// خطاهای کشف‌شده: کد نامعتبر، کد تکراری، والد گم‌شده، ناسازگاری ماهیت.
pub fn build_tree(
    scheme: &CodingScheme,
    accounts: &[AccountDefinition],
) -> Result<Vec<AccountNode>, CodingError> {
    let mut by_code: BTreeMap<&str, &AccountDefinition> = BTreeMap::new();
    for account in accounts {
        scheme.level_of(&account.code)?;
        if by_code.insert(account.code.as_str(), account).is_some() {
            return Err(CodingError::DuplicateCode {
                code: account.code.clone(),
            });
        }
    }

    for account in accounts {
        let level = scheme.level_of(&account.code)?;
        if level == 0 {
            continue;
        }
        let parent_code = scheme.parent_code(&account.code)?;
        let parent = by_code
            .get(parent_code.as_str())
            .ok_or(CodingError::MissingParent {
                parent: parent_code.clone(),
            })?;
        if !parent.nature.accepts_child(account.nature) {
            return Err(CodingError::NatureConflict {
                code: account.code.clone(),
            });
        }
    }

    fn collect(
        scheme: &CodingScheme,
        by_code: &BTreeMap<&str, &AccountDefinition>,
        parent: Option<&str>,
        level: usize,
    ) -> Result<Vec<AccountNode>, CodingError> {
        let mut nodes = Vec::new();
        for (code, account) in by_code {
            if scheme.level_of(code)? != level {
                continue;
            }
            let belongs = match parent {
                None => true,
                Some(parent_code) => scheme.parent_code(code)? == parent_code,
            };
            if !belongs {
                continue;
            }
            nodes.push(AccountNode {
                account: (*account).clone(),
                level,
                children: collect(scheme, by_code, Some(code), level + 1)?,
            });
        }
        Ok(nodes)
    }

    collect(scheme, &by_code, None, 0)
}

/// یک تفصیلی شناور (شخص، صندوق، بانک، مرکز هزینه، پروژه و…).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subsidiary {
    pub code: String,
    pub title: String,
    /// گروه تفصیلی: بانک‌ها، صندوق‌ها، اشخاص، مراکز هزینه و…
    pub group: String,
}

/// ابعاد مالی انتخاب‌شده روی یک سطر سند.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    #[serde(default)]
    pub subsidiary: Option<Subsidiary>,
    #[serde(default)]
    pub cost_center: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

impl Dimensions {
    pub fn with_subsidiary(subsidiary: Subsidiary) -> Self {
        Dimensions {
            subsidiary: Some(subsidiary),
            ..Default::default()
        }
    }
}

/// اعتبارسنجی ثبت‌پذیری حساب و ابعاد مالی همراه آن.
///
/// این تابع دروازه‌ی اجباری هر سطر سند است و سه قاعده را تضمین می‌کند:
/// - سند فقط روی حساب سطح آخر ثبت می‌شود (نه گروه/کل/معین میانی).
/// - اگر حساب تفصیلی الزامی دارد، تفصیلی باید انتخاب شده باشد.
/// - تفصیلی انتخاب‌شده باید به گروه تفصیلی مجاز آن حساب تعلق داشته باشد.
pub fn validate_posting(
    scheme: &CodingScheme,
    account: &AccountDefinition,
    dimensions: &Dimensions,
) -> Result<(), CodingError> {
    if !scheme.is_leaf_level(&account.code)? {
        return Err(CodingError::NotPostable {
            code: account.code.clone(),
        });
    }
    validate_dimensions(account, dimensions)
}

/// اعتبارسنجی فقط ابعاد مالی، بدون بررسی سطح کد.
///
/// برای پایگاه داده‌هایی که سطح حساب را در ستون جداگانه نگهداری می‌کنند
/// (به‌جای استنتاج از طول کد) همین تابع استفاده می‌شود تا قواعد ابعاد مالی
/// یک منبع حقیقت واحد داشته باشند.
pub fn validate_dimensions(
    account: &AccountDefinition,
    dimensions: &Dimensions,
) -> Result<(), CodingError> {
    match (&account.subsidiary_group, &dimensions.subsidiary) {
        (Some(group), Some(subsidiary)) => {
            if &subsidiary.group != group {
                return Err(CodingError::SubsidiaryGroupMismatch);
            }
        }
        (Some(_), None) => return Err(CodingError::SubsidiaryRequired),
        (None, Some(_)) if !account.requires_subsidiary => {
            return Err(CodingError::SubsidiaryNotAllowed)
        }
        (None, None) if account.requires_subsidiary => return Err(CodingError::SubsidiaryRequired),
        _ => {}
    }
    if account.requires_cost_center && dimensions.cost_center.is_none() {
        return Err(CodingError::CostCenterRequired);
    }
    if account.requires_project && dimensions.project.is_none() {
        return Err(CodingError::ProjectRequired);
    }
    Ok(())
}
