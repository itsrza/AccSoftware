//! کدینگ حساب‌ها — درخت چندسطحی گروه، کل، معین و تفصیلی.
//!
//! مرجع: تصویر `dgNqWj` (کدینگ حساب‌ها).
//!
//! ## قواعد حسابداری پیاده‌شده
//!
//! ۱. **فقط حساب سطح آخر قابل ثبت سند است.** حساب گروه و کل جمع فرزندانشان
//!    هستند؛ ثبت سند مستقیم روی آن‌ها تراز را از ساختار درخت جدا می‌کند.
//! ۲. **ماهیت فرزند باید با والد سازگار باشد.** حساب بدهکار نمی‌تواند زیر
//!    حساب بستانکار بنشیند — مگر والد «مختلط» باشد.
//! ۳. **کد از طرح کدینگ می‌آید، نه از دست کاربر.** طول هر سطح در طرح تعریف
//!    شده و کد فرزند از کد والد ساخته می‌شود، پس سلسله‌مراتب از روی کد
//!    قابل بازسازی است.
//! ۴. **حساب دارای گردش حذف نمی‌شود.** غیرفعال می‌شود تا دفاتر سال‌های قبل
//!    سالم بمانند.
//!
//! ## چرا سطح از ستون پایگاه داده خوانده می‌شود، نه فقط از طول کد
//!
//! کدینگ موجود (و کدینگ بسیاری از کسب‌وکارهای واقعی) **مسطح** است: همه‌ی
//! کدها چهار رقمی‌اند و سلسله‌مراتب فقط از رابطه‌ی والد-فرزند می‌آید، نه از
//! طول کد. نرم‌افزاری که این را نپذیرد، کاربر را مجبور می‌کند کل دفاترش را
//! دوباره کدگذاری کند.
//!
//! پس: **درخت از `parent_id` ساخته می‌شود و سطح از ستون `level`.** طرح کدینگ
//! برای *پیشنهاد کد بعدی* و *اعتبارسنجی کدهای تازه* به کار می‌رود، و
//! گزارش «سلامت کدینگ» حساب‌هایی را که با طرح نمی‌خوانند نشان می‌دهد تا
//! کاربر بتواند تدریجی مهاجرت کند.
//!
//! ## طرح کدینگ قابل تنظیم
//!
//! پیش‌فرض `[1,2,2,2]` است — یعنی `1` گروه، `11` کل، `1101` معین،
//! `110101` تفصیلی. این با کدهای هفت‌رقمی رایج مثل `1103101` هم سازگار است
//! اگر طرح `[1,2,2,2]` به `[1,2,2,2]` گسترش یابد؛ طرح در تنظیمات ذخیره
//! می‌شود و همه‌ی محاسبات از همان می‌خوانند.

use novin_core::coding::{AccountNature, CodingScheme};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

use crate::{active_context, audit, conn, require_permission, AppState};

const SCHEME_WIDTHS_KEY: &str = "coding.level_widths";
const SCHEME_TITLES_KEY: &str = "coding.level_titles";

/// یک گره‌ی درخت کدینگ، همراه با گردش و مانده.
#[derive(Debug, Serialize)]
pub struct AccountNodeRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub level: usize,
    pub level_title: String,
    pub parent_id: Option<String>,
    pub nature: String,
    pub nature_label: String,
    pub is_active: bool,
    /// آیا سند مستقیم روی این حساب مجاز است؟
    pub is_postable: bool,
    pub child_count: i64,
    /// گردش بدهکار و بستانکار خودِ حساب (نه فرزندان).
    pub debit: i64,
    pub credit: i64,
    /// مانده‌ی تجمعی شامل همه‌ی فرزندان — همان چیزی که در تراز دیده می‌شود.
    pub rollup_balance: i64,
    pub requires_subsidiary: bool,
    pub subsidiary_group_id: Option<String>,
}

/// طرح کدینگ فعلی، برای نمایش در تنظیمات و ساخت کد بعدی.
#[derive(Debug, Serialize)]
pub struct CodingSchemeInfo {
    pub level_widths: Vec<u8>,
    pub level_titles: Vec<String>,
    /// طول تجمعی کد در هر سطح.
    pub code_lengths: Vec<usize>,
    /// حداکثر تعداد فرزند در هر سطح.
    pub capacities: Vec<u32>,
}

fn setting(tx: &rusqlite::Transaction<'_>, key: &str) -> Option<String> {
    tx.query_row(
        "SELECT value FROM app_settings WHERE key=?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
}

/// خواندن طرح کدینگ از تنظیمات؛ اگر تنظیم نشده باشد، طرح پیش‌فرض.
fn load_scheme(tx: &rusqlite::Transaction<'_>) -> CodingScheme {
    let widths = setting(tx, SCHEME_WIDTHS_KEY).and_then(|raw| {
        raw.split(',')
            .map(|part| part.trim().parse::<u8>().ok())
            .collect::<Option<Vec<u8>>>()
    });
    let titles = setting(tx, SCHEME_TITLES_KEY).map(|raw| {
        raw.split(',')
            .map(|part| part.trim().to_string())
            .collect::<Vec<String>>()
    });
    match (widths, titles) {
        (Some(widths), Some(titles)) => {
            CodingScheme::new(widths, titles).unwrap_or_else(CodingScheme::default)
        }
        _ => CodingScheme::default(),
    }
}

/// سطح حساب از ستون پایگاه داده — منبع حقیقتِ سلسله‌مراتب.
fn level_from_db(level: &str) -> usize {
    match level {
        "group" => 0,
        "general" => 1,
        "subsidiary" => 2,
        _ => 3,
    }
}

fn db_level_name(level: usize) -> &'static str {
    match level {
        0 => "group",
        1 => "general",
        2 => "subsidiary",
        _ => "detail",
    }
}

fn nature_label(nature: &str) -> &'static str {
    match nature {
        "debit" => "بدهکار",
        "credit" => "بستانکار",
        "mixed" => "مختلط",
        _ => "نامشخص",
    }
}

fn parse_nature(value: &str) -> Result<AccountNature, String> {
    match value {
        "debit" => Ok(AccountNature::Debit),
        "credit" => Ok(AccountNature::Credit),
        "mixed" => Ok(AccountNature::Mixed),
        _ => Err("COA-001: ماهیت حساب نامعتبر است".into()),
    }
}

/// طرح کدینگ فعلی.
#[tauri::command]
pub fn get_coding_scheme(state: State<AppState>) -> Result<CodingSchemeInfo, String> {
    let mut c = conn(&state)?;
    require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let scheme = load_scheme(&tx);
    Ok(describe(&scheme))
}

fn describe(scheme: &CodingScheme) -> CodingSchemeInfo {
    let depth = scheme.depth();
    let code_lengths: Vec<usize> = (0..depth)
        .filter_map(|level| scheme.code_length(level))
        .collect();
    let mut capacities = Vec::with_capacity(depth);
    let mut previous = 0usize;
    for length in &code_lengths {
        let width = length - previous;
        capacities.push(10u32.pow(width as u32) - 1);
        previous = *length;
    }
    CodingSchemeInfo {
        level_widths: (0..depth)
            .map(|level| {
                let start = if level == 0 {
                    0
                } else {
                    code_lengths[level - 1]
                };
                (code_lengths[level] - start) as u8
            })
            .collect(),
        level_titles: (0..depth)
            .map(|level| scheme.level_title(level).unwrap_or("سطح").to_string())
            .collect(),
        code_lengths,
        capacities,
    }
}

/// تغییر طرح کدینگ.
///
/// اگر حسابی وجود داشته باشد که با طرح تازه نخواند، تغییر رد می‌شود —
/// وگرنه درخت کدینگ بی‌معنا و گزارش تراز غیرقابل ساخت می‌شود.
#[tauri::command]
pub fn set_coding_scheme(
    state: State<AppState>,
    level_widths: Vec<u8>,
    level_titles: Vec<String>,
) -> Result<CodingSchemeInfo, String> {
    let scheme = CodingScheme::new(level_widths.clone(), level_titles.clone())
        .ok_or_else(|| "COA-002: طرح کدینگ نامعتبر است".to_string())?;
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    // کدهای موجود مانع تغییر طرح نمی‌شوند — کدینگ قدیمی حق دارد باقی بماند.
    // گزارش سلامت کدینگ ناسازگاری‌ها را نشان می‌دهد تا مهاجرت تدریجی ممکن باشد.
    tx.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![
            SCHEME_WIDTHS_KEY,
            level_widths
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![SCHEME_TITLES_KEY, level_titles.join(",")],
    )
    .map_err(|e| e.to_string())?;
    let info = describe(&scheme);
    tx.commit().map_err(|e| e.to_string())?;
    Ok(info)
}

/// درخت کامل کدینگ با گردش و مانده‌ی تجمعی هر شاخه.
///
/// مانده‌ی تجمعی با تطبیق پیشوند کد محاسبه می‌شود، نه با پیمایش بازگشتی؛
/// یک پرس‌وجو به‌جای صدها پرس‌وجوی تودرتو.
#[tauri::command]
pub fn list_account_tree(
    state: State<AppState>,
    include_inactive: Option<bool>,
) -> Result<Vec<AccountNodeRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;
    let scheme = load_scheme(&tx);

    let mut sql = String::from(
        "SELECT a.id,a.code,a.name,a.parent_id,a.nature,a.is_active,\
         COALESCE(a.requires_subsidiary,0),a.subsidiary_group_id,\
         COALESCE(SUM(l.debit),0),COALESCE(SUM(l.credit),0),a.level \
         FROM accounts a LEFT JOIN journal_lines l ON l.account_id=a.id \
         WHERE a.company_id=?1",
    );
    if !include_inactive.unwrap_or(false) {
        sql.push_str(" AND a.is_active=1");
    }
    sql.push_str(" GROUP BY a.id ORDER BY a.code");

    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let raw: Vec<(
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        i64,
        Option<String>,
        i64,
        i64,
        String,
    )> = statement
        .query_map(params![company], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(statement);

    let mut rows = Vec::with_capacity(raw.len());
    // مانده‌ی تجمعی از روی درخت والد-فرزند محاسبه می‌شود، نه از روی پیشوند کد،
    // چون کدینگ ممکن است مسطح باشد. عمق درخت محدود است، پس چند پیمایش کافی است.
    let descendants = |root: &str| -> Vec<usize> {
        let mut selected: Vec<usize> = Vec::new();
        let mut frontier: Vec<String> = vec![root.to_string()];
        while let Some(current) = frontier.pop() {
            for (index, row) in raw.iter().enumerate() {
                if row.0 == current && !selected.contains(&index) {
                    selected.push(index);
                }
                if row.3.as_deref() == Some(current.as_str()) {
                    frontier.push(row.0.clone());
                }
            }
        }
        selected
    };

    for (id, code, name, parent_id, nature, is_active, requires, group, debit, credit, db_level) in
        &raw
    {
        let level = level_from_db(db_level);
        let (roll_debit, roll_credit) = descendants(id)
            .into_iter()
            .fold((0i64, 0i64), |(d, cr), index| {
                (d + raw[index].8, cr + raw[index].9)
            });
        let child_count = raw
            .iter()
            .filter(|other| other.3.as_deref() == Some(id.as_str()))
            .count() as i64;
        rows.push(AccountNodeRow {
            id: id.clone(),
            code: code.clone(),
            name: name.clone(),
            level,
            level_title: scheme.level_title(level).unwrap_or("سطح").to_string(),
            parent_id: parent_id.clone(),
            nature_label: nature_label(nature).to_string(),
            nature: nature.clone(),
            is_active: *is_active == 1,
            // فقط برگ‌های واقعی درخت قابل ثبت سندند. «برگ بودن» یعنی فرزندی
            // ندارد — این تعریف هم با کدینگ سلسله‌مراتبی کار می‌کند هم با مسطح.
            is_postable: child_count == 0,
            child_count,
            debit: *debit,
            credit: *credit,
            rollup_balance: roll_debit - roll_credit,
            requires_subsidiary: *requires == 1,
            subsidiary_group_id: group.clone(),
        });
    }
    Ok(rows)
}

/// نخستین کد آزاد زیر یک حساب — تا کاربر کد را از خودش نسازد.
#[tauri::command]
pub fn suggest_account_code(
    state: State<AppState>,
    parent_id: Option<String>,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;
    let scheme = load_scheme(&tx);

    let mut statement = tx
        .prepare("SELECT code FROM accounts WHERE company_id=?1")
        .map_err(|e| e.to_string())?;
    let codes: Vec<String> = statement
        .query_map(params![company], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(statement);

    match parent_id.filter(|v| !v.trim().is_empty()) {
        Some(parent) => {
            let parent_code: String = tx
                .query_row(
                    "SELECT code FROM accounts WHERE id=?1 AND company_id=?2",
                    params![parent, company],
                    |row| row.get(0),
                )
                .map_err(|_| "COA-004: حساب والد یافت نشد".to_string())?;

            // مسیر اول: طرح کدینگ سلسله‌مراتبی.
            if let Ok(candidate) = scheme.next_child_code(&parent_code, &codes) {
                return Ok(candidate);
            }

            // مسیر دوم (کدینگ مسطح): از روی کد خواهرها ادامه بده.
            let mut siblings: Vec<String> = Vec::new();
            let mut statement = tx
                .prepare(
                    "SELECT code FROM accounts WHERE company_id=?1 AND parent_id=?2 ORDER BY code",
                )
                .map_err(|e| e.to_string())?;
            for value in statement
                .query_map(params![company, parent], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .flatten()
            {
                siblings.push(value);
            }
            drop(statement);

            let width = siblings
                .first()
                .map(String::len)
                .unwrap_or_else(|| parent_code.len());
            let base: i64 = siblings
                .iter()
                .filter_map(|value| value.parse::<i64>().ok())
                .max()
                .unwrap_or_else(|| parent_code.parse::<i64>().unwrap_or(0));
            for step in 1..1000 {
                let candidate = format!("{:0width$}", base + step, width = width);
                if !codes.contains(&candidate) {
                    return Ok(candidate);
                }
            }
            Err("COA-005: کد آزادی زیر این حساب پیدا نشد".into())
        }
        None => {
            // سطح ریشه: نخستین کد آزاد با همان طول کدهای ریشه‌ی موجود.
            let root_width = codes
                .iter()
                .filter(|code| code.ends_with("000"))
                .map(|code| code.len())
                .next()
                .unwrap_or(describe(&scheme).level_widths[0] as usize);
            for serial in 1..1000 {
                let candidate = if root_width > 1 {
                    format!("{}{}", serial, "0".repeat(root_width - 1))
                } else {
                    serial.to_string()
                };
                if !codes.contains(&candidate) {
                    return Ok(candidate);
                }
            }
            Err("COA-006: ظرفیت سطح ریشه پر شده است".into())
        }
    }
}

/// گزارش سلامت کدینگ.
///
/// حساب‌هایی را نشان می‌دهد که با قواعد سالم حسابداری نمی‌خوانند: حساب غیربرگ
/// که سند مستقیم خورده، حساب برگ بدون گردش، و کدی که با طرح کدینگ فعلی
/// سازگار نیست. هیچ‌کدام خطا نیستند — هشدارِ قابل رفع‌اند.
#[derive(Debug, Serialize)]
pub struct CodingIssue {
    pub account_id: String,
    pub code: String,
    pub name: String,
    pub severity: String,
    pub message: String,
}

#[tauri::command]
pub fn audit_coding_health(state: State<AppState>) -> Result<Vec<CodingIssue>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;
    let scheme = load_scheme(&tx);

    let mut statement = tx
        .prepare(
            "SELECT a.id,a.code,a.name,a.level,\
             (SELECT COUNT(*) FROM accounts k WHERE k.parent_id=a.id),\
             (SELECT COUNT(*) FROM journal_lines l WHERE l.account_id=a.id),\
             a.parent_id \
             FROM accounts a WHERE a.company_id=?1 AND a.is_active=1 ORDER BY a.code",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, String, String, String, i64, i64, Option<String>)> = statement
        .query_map(params![company], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(statement);

    let mut issues = Vec::new();
    for (id, code, name, _level, children, lines, parent) in &rows {
        // مهم‌ترین ایراد: سند مستقیم روی حسابی که فرزند دارد.
        if *children > 0 && *lines > 0 {
            issues.push(CodingIssue {
                account_id: id.clone(),
                code: code.clone(),
                name: name.clone(),
                severity: "error".into(),
                message: format!(
                    "این حساب {children} زیرحساب دارد ولی {lines} سطر سند مستقیم روی آن ثبت شده؛ جمع شاخه با مانده نمی‌خواند."
                ),
            });
        }
        if parent.is_none() && *lines > 0 {
            issues.push(CodingIssue {
                account_id: id.clone(),
                code: code.clone(),
                name: name.clone(),
                severity: "error".into(),
                message: "سند مستقیم روی حساب سطح گروه ثبت شده است.".into(),
            });
        }
        if scheme.level_of(code).is_err() {
            issues.push(CodingIssue {
                account_id: id.clone(),
                code: code.clone(),
                name: name.clone(),
                severity: "info".into(),
                message: "طول این کد با طرح کدینگ فعلی نمی‌خواند؛ کدینگ مسطح است و کار می‌کند، ولی پیشنهاد کد خودکار برایش دقیق نیست.".into(),
            });
        }
    }
    Ok(issues)
}

/// ساخت یا ویرایش حساب با رعایت همه‌ی قواعد کدینگ.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn save_account(
    state: State<AppState>,
    id: Option<String>,
    code: String,
    name: String,
    nature: String,
    parent_id: Option<String>,
    requires_subsidiary: Option<bool>,
    subsidiary_group_id: Option<String>,
    is_active: Option<bool>,
) -> Result<String, String> {
    let code = code.trim().to_string();
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("COA-007: نام حساب الزامی است".into());
    }
    let account_nature = parse_nature(&nature)?;

    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;
    let scheme = load_scheme(&tx);

    if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("COA-008: کد حساب باید فقط رقم باشد".into());
    }

    // سطح از والد می‌آید، نه از طول کد — تا کدینگ مسطح موجود هم کار کند.
    let parent = parent_id.filter(|v| !v.trim().is_empty());
    let level = match &parent {
        None => 0usize,
        Some(parent_id) => {
            let (parent_code, parent_nature, parent_level): (String, String, String) = tx
                .query_row(
                    "SELECT code,nature,level FROM accounts WHERE id=?1 AND company_id=?2",
                    params![parent_id, company],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|_| "COA-004: حساب والد یافت نشد".to_string())?;

            // ماهیت فرزند باید با والد سازگار باشد — این قاعده‌ی حسابداری است
            // و به شکل کدینگ ربطی ندارد، پس همیشه اعمال می‌شود.
            let parent_nature_enum = parse_nature(&parent_nature)?;
            if !parent_nature_enum.accepts_child(account_nature) {
                return Err(format!(
                    "COA-013: حساب {} نمی‌تواند زیر حساب {} بنشیند",
                    nature_label(&nature),
                    nature_label(&parent_nature)
                ));
            }

            // اگر هر دو کد با طرح کدینگ بخوانند، پیشوند هم باید بخواند.
            // اگر کدینگ مسطح است، این بررسی نادیده گرفته می‌شود.
            if let (Ok(_), Ok(expected)) =
                (scheme.level_of(&parent_code), scheme.parent_code(&code))
            {
                if expected != parent_code {
                    return Err(format!(
                        "COA-012: کد «{code}» زیرمجموعه‌ی «{parent_code}» نیست؛ باید با «{expected}» شروع شود"
                    ));
                }
            }

            let next = level_from_db(&parent_level) + 1;
            if next > 3 {
                return Err("COA-009: عمق کدینگ بیش از چهار سطح پشتیبانی نمی‌شود".into());
            }
            next
        }
    };

    let duplicate: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE company_id=?1 AND code=?2 AND id<>?3",
            params![company, code, id.clone().unwrap_or_default()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if duplicate > 0 {
        return Err(format!("COA-014: حسابی با کد «{code}» از قبل وجود دارد"));
    }

    let db_level = db_level_name(level);

    let account_id = match &id {
        Some(existing) => {
            // اگر حساب گردش دارد، تغییر ماهیت یا کد آن دفاتر قبلی را بی‌معنا می‌کند.
            let movements: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM journal_lines WHERE account_id=?1",
                    params![existing],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if movements > 0 {
                let (old_code, old_nature): (String, String) = tx
                    .query_row(
                        "SELECT code,nature FROM accounts WHERE id=?1",
                        params![existing],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| e.to_string())?;
                if old_code != code {
                    return Err(format!(
                        "COA-015: این حساب {movements} سطر سند دارد؛ کد آن قابل تغییر نیست"
                    ));
                }
                if old_nature != nature {
                    return Err(format!(
                        "COA-016: این حساب {movements} سطر سند دارد؛ ماهیت آن قابل تغییر نیست"
                    ));
                }
            }
            tx.execute(
                "UPDATE accounts SET code=?1,name=?2,level=?3,parent_id=?4,nature=?5,\
                 requires_subsidiary=?6,subsidiary_group_id=?7,is_active=?8 WHERE id=?9",
                params![
                    code,
                    name,
                    db_level,
                    parent,
                    nature,
                    i64::from(requires_subsidiary.unwrap_or(false)),
                    subsidiary_group_id,
                    i64::from(is_active.unwrap_or(true)),
                    existing
                ],
            )
            .map_err(|e| format!("COA-017: {e}"))?;
            existing.clone()
        }
        None => {
            let new_id = format!("acc-{code}");
            tx.execute(
                "INSERT INTO accounts(id,company_id,code,name,level,parent_id,nature,\
                 requires_subsidiary,subsidiary_group_id,is_active) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    new_id,
                    company,
                    code,
                    name,
                    db_level,
                    parent,
                    nature,
                    i64::from(requires_subsidiary.unwrap_or(false)),
                    subsidiary_group_id,
                    i64::from(is_active.unwrap_or(true))
                ],
            )
            .map_err(|e| format!("COA-018: {e}"))?;
            new_id
        }
    };

    audit(
        &tx,
        &user,
        "accounting.account.save",
        "account",
        &account_id,
        None,
        Some(&format!("{{\"code\":\"{code}\",\"name\":\"{name}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(account_id)
}

/// غیرفعال‌کردن حساب.
///
/// حساب دارای گردش هرگز حذف نمی‌شود؛ حذفش یعنی نابودی دفاتر سال‌های قبل.
/// حسابی که فرزند فعال دارد هم غیرفعال نمی‌شود، وگرنه شاخه‌ی درخت آویزان می‌ماند.
#[tauri::command]
pub fn deactivate_account(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let children: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE parent_id=?1 AND is_active=1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if children > 0 {
        return Err(format!(
            "COA-019: این حساب {children} زیرحساب فعال دارد؛ ابتدا آن‌ها را غیرفعال کنید"
        ));
    }
    let linked: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM treasury_accounts WHERE linked_account_id=?1 AND is_active=1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if linked > 0 {
        return Err("COA-020: یک حساب خزانه‌ی فعال به این حساب وصل است".into());
    }

    let changed = tx
        .execute(
            "UPDATE accounts SET is_active=0 WHERE id=?1 AND company_id=?2",
            params![id, company],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("COA-021: حساب یافت نشد".into());
    }
    audit(
        &tx,
        &user,
        "accounting.account.deactivate",
        "account",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
