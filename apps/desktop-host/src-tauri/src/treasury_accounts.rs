//! تعریف و مدیریت صندوق‌ها و حساب‌های بانکی.
//!
//! ## چرا یک ماژول برای هر دو
//!
//! صندوق، بانک و تنخواه از نظر حسابداری یک چیزند: **حساب خزانه**. تفاوتشان
//! فقط در فیلدهای تکمیلی است (بانک شبا و شعبه دارد، صندوق ندارد). ساختن دو
//! جدول یا دو منطق جدا، همان اشتباهی است که بعداً گزارش موجودی نقد را
//! دوتکه می‌کند.
//!
//! ## سیاست منفی شدن موجودی
//!
//! مرجع: بخش «هشدار منفی شدن موجودی» در فرم تعریف بانک و صندوق.
//!
//! | سیاست | رفتار |
//! |---|---|
//! | `error` | برداشت بیش از موجودی انجام نمی‌شود |
//! | `warn` | انجام می‌شود ولی هشدار داده می‌شود |
//! | `ignore` | بی‌تفاوت |
//!
//! از نظر حسابداری، صندوقِ منفی بی‌معناست (نمی‌شود پولی که نیست را پرداخت
//! کرد)، ولی حساب بانکی می‌تواند اضافه‌برداشت داشته باشد؛ پس سیاست باید
//! به‌ازای هر حساب قابل تنظیم باشد، نه سراسری.

use novin_core::parties::{card_number_is_valid, iban_is_valid};
use novin_core::treasury::NegativeBalancePolicy;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::{active_context, audit, conn, require_permission, AppState};

/// یک حساب خزانه با همه‌ی اطلاعات و مانده‌ی محاسبه‌شده.
#[derive(Debug, Serialize)]
pub struct TreasuryAccountRow {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub account_type_label: String,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub card_number: Option<String>,
    pub branch_name: Option<String>,
    pub branch_code: Option<String>,
    pub holder_name: Option<String>,
    pub has_pos_terminal: bool,
    pub negative_policy: String,
    pub negative_policy_label: String,
    pub linked_account_id: Option<String>,
    pub linked_account_name: Option<String>,
    pub is_active: bool,
    /// مانده = مجموع دریافت‌ها منهای پرداخت‌ها.
    pub balance: i64,
    pub inflow: i64,
    pub outflow: i64,
    pub transaction_count: i64,
}

fn type_label(kind: &str) -> &'static str {
    match kind {
        "cash" => "صندوق",
        "bank" => "حساب بانکی",
        "petty_cash" => "تنخواه",
        _ => "نامشخص",
    }
}

/// ورودی ذخیره‌ی حساب — همان فیلدهای فرم تعریف بانک و صندوق.
#[derive(Debug, serde::Deserialize)]
pub struct TreasuryAccountInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub account_type: String,
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub card_number: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub branch_code: Option<String>,
    #[serde(default)]
    pub holder_name: Option<String>,
    #[serde(default)]
    pub has_pos_terminal: bool,
    #[serde(default)]
    pub negative_policy: Option<String>,
    #[serde(default)]
    pub linked_account_id: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// فهرست حساب‌های خزانه با مانده‌ی واقعی، قابل فیلتر بر اساس نوع.
///
/// مانده در همان پرس‌وجو و با تجمیع محاسبه می‌شود تا جدول به‌ازای هر ردیف
/// پرس‌وجوی جداگانه نزند.
#[tauri::command]
pub fn list_treasury_account_details(
    state: State<AppState>,
    account_type: Option<String>,
    include_inactive: Option<bool>,
) -> Result<Vec<TreasuryAccountRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.view")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let mut sql = String::from(
        "SELECT t.id,t.name,t.account_type,t.account_number,t.iban,t.card_number,t.branch_name,\
         t.branch_code,t.holder_name,t.has_pos_terminal,t.negative_policy,t.linked_account_id,\
         a.name,t.is_active,\
         COALESCE(SUM(CASE WHEN x.transaction_type='receipt' THEN x.amount ELSE 0 END),0),\
         COALESCE(SUM(CASE WHEN x.transaction_type='payment' THEN x.amount ELSE 0 END),0),\
         COUNT(x.id) \
         FROM treasury_accounts t \
         LEFT JOIN accounts a ON a.id=t.linked_account_id \
         LEFT JOIN treasury_transactions x ON x.treasury_account_id=t.id \
         WHERE t.company_id=?1",
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(company.clone())];
    if let Some(kind) = account_type.filter(|v| !v.trim().is_empty()) {
        if !["cash", "bank", "petty_cash"].contains(&kind.as_str()) {
            return Err("TACC-001: نوع حساب خزانه نامعتبر است".into());
        }
        values.push(Box::new(kind));
        sql.push_str(&format!(" AND t.account_type=?{}", values.len()));
    }
    if !include_inactive.unwrap_or(false) {
        sql.push_str(" AND t.is_active=1");
    }
    sql.push_str(" GROUP BY t.id ORDER BY t.account_type, t.name");

    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let bound: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let rows = statement
        .query_map(bound.as_slice(), |row| {
            let account_type: String = row.get(2)?;
            let policy: String = row.get(10)?;
            let inflow: i64 = row.get(14)?;
            let outflow: i64 = row.get(15)?;
            Ok(TreasuryAccountRow {
                id: row.get(0)?,
                name: row.get(1)?,
                account_type_label: type_label(&account_type).to_string(),
                account_type,
                account_number: row.get(3)?,
                iban: row.get(4)?,
                card_number: row.get(5)?,
                branch_name: row.get(6)?,
                branch_code: row.get(7)?,
                holder_name: row.get(8)?,
                has_pos_terminal: row.get::<_, i64>(9)? == 1,
                negative_policy_label: NegativeBalancePolicy::parse(&policy).label().to_string(),
                negative_policy: policy,
                linked_account_id: row.get(11)?,
                linked_account_name: row.get(12)?,
                is_active: row.get::<_, i64>(13)? == 1,
                balance: inflow - outflow,
                inflow,
                outflow,
                transaction_count: row.get(16)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// ذخیره‌ی حساب خزانه (ایجاد یا ویرایش) با اعتبارسنجی کامل.
///
/// شبا و شماره کارت با همان الگوریتم‌های رسمی بررسی می‌شوند (کد کنترلی
/// mod-97 برای شبا و Luhn برای کارت)، چون شماره‌ی اشتباه در حواله یعنی پول
/// گم‌شده.
#[tauri::command]
pub fn save_treasury_account(
    state: State<AppState>,
    input: TreasuryAccountInput,
) -> Result<String, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("TACC-002: نام حساب الزامی است".into());
    }
    if !["cash", "bank", "petty_cash"].contains(&input.account_type.as_str()) {
        return Err("TACC-001: نوع حساب خزانه نامعتبر است".into());
    }
    // اگر کاربر سیاست نداده، پیش‌فرض از مرکز تنظیمات می‌آید.
    let default_policy = {
        let probe = conn(&state)?;
        crate::settings::read_setting(&probe, "treasury.default_negative_policy")
    };
    let policy = input
        .negative_policy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_policy.as_str());
    if !["error", "warn", "ignore"].contains(&policy) {
        return Err("TACC-003: سیاست منفی شدن موجودی نامعتبر است".into());
    }
    let iban = clean(&input.iban);
    if let Some(value) = &iban {
        if !iban_is_valid(value) {
            return Err("TACC-004: شماره شبا معتبر نیست".into());
        }
    }
    let card = clean(&input.card_number);
    if let Some(value) = &card {
        if !card_number_is_valid(value) {
            return Err("TACC-005: شماره کارت معتبر نیست".into());
        }
    }
    // صندوق و تنخواه شبا و شعبه ندارند؛ پذیرفتن آن‌ها یعنی داده‌ی بی‌معنا.
    if input.account_type != "bank" && (iban.is_some() || input.branch_name.is_some()) {
        return Err("TACC-006: شبا و شعبه فقط برای حساب بانکی معنا دارد".into());
    }

    let mut c = conn(&state)?;
    let permission = if input.id.is_some() {
        "treasury.account.update"
    } else {
        "treasury.account.create"
    };
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    if let Some(account) = clean(&input.linked_account_id) {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2 AND is_active=1",
                params![account, company],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("TACC-007: حساب حسابداری متصل معتبر نیست".into());
        }
    }

    // نام تکراری در همان شرکت مجاز نیست — قید پایگاه داده هم همین را می‌گوید،
    // ولی پیام گویا بهتر از خطای خام است.
    let duplicate: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM treasury_accounts WHERE company_id=?1 AND name=?2 AND id<>?3",
            params![company, name, input.id.clone().unwrap_or_default()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if duplicate > 0 {
        return Err("TACC-008: حسابی با این نام از قبل وجود دارد".into());
    }

    let id = match &input.id {
        Some(existing) => {
            let owned: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM treasury_accounts WHERE id=?1 AND company_id=?2",
                    params![existing, company],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if owned == 0 {
                return Err("TACC-009: حساب خزانه یافت نشد".into());
            }
            tx.execute(
                "UPDATE treasury_accounts SET name=?1,account_type=?2,account_number=?3,iban=?4,\
                 card_number=?5,branch_name=?6,branch_code=?7,holder_name=?8,has_pos_terminal=?9,\
                 negative_policy=?10,linked_account_id=?11,is_active=?12 WHERE id=?13",
                params![
                    name,
                    input.account_type,
                    clean(&input.account_number),
                    iban,
                    card,
                    clean(&input.branch_name),
                    clean(&input.branch_code),
                    clean(&input.holder_name),
                    i64::from(input.has_pos_terminal),
                    policy,
                    clean(&input.linked_account_id),
                    i64::from(input.is_active),
                    existing
                ],
            )
            .map_err(|e| format!("TACC-010: {e}"))?;
            existing.clone()
        }
        None => {
            let id = format!(
                "treasury-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO treasury_accounts(id,company_id,name,account_type,account_number,iban,\
                 card_number,branch_name,branch_code,holder_name,has_pos_terminal,negative_policy,\
                 linked_account_id,is_active) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    id,
                    company,
                    name,
                    input.account_type,
                    clean(&input.account_number),
                    iban,
                    card,
                    clean(&input.branch_name),
                    clean(&input.branch_code),
                    clean(&input.holder_name),
                    i64::from(input.has_pos_terminal),
                    policy,
                    clean(&input.linked_account_id),
                    i64::from(input.is_active)
                ],
            )
            .map_err(|e| format!("TACC-011: {e}"))?;
            id
        }
    };

    audit(
        &tx,
        &user,
        "treasury.account.save",
        "treasury_account",
        &id,
        None,
        Some(&format!(
            "{{\"name\":\"{name}\",\"type\":\"{}\"}}",
            input.account_type
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

/// غیرفعال‌کردن حساب خزانه.
///
/// حذف نمی‌کنیم: حسابی که سند دارد باید برای همیشه در دفاتر بماند. حذف آن
/// یعنی از بین رفتن ردپای حسابرسی.
#[tauri::command]
pub fn deactivate_treasury_account(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.account.update")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let balance: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN transaction_type='receipt' THEN amount ELSE -amount END),0) \
             FROM treasury_transactions WHERE treasury_account_id=?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if balance != 0 {
        return Err(format!(
            "TACC-012: این حساب {balance} ریال مانده دارد؛ ابتدا مانده را تسویه یا منتقل کنید"
        ));
    }
    let open_checks: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM checks WHERE treasury_account_id=?1 \
             AND status IN ('in_hand','deposited','endorsed','outstanding')",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if open_checks > 0 {
        return Err("TACC-013: چک باز روی این حساب وجود دارد".into());
    }

    let changed = tx
        .execute(
            "UPDATE treasury_accounts SET is_active=0 WHERE id=?1 AND company_id=?2",
            params![id, company],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("TACC-009: حساب خزانه یافت نشد".into());
    }
    audit(
        &tx,
        &user,
        "treasury.account.deactivate",
        "treasury_account",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// سیاست‌های منفی شدن موجودی با برچسب و توضیح فارسی.
#[derive(Debug, Serialize)]
pub struct PolicyInfo {
    pub value: String,
    pub label: String,
    pub explanation: String,
}

#[tauri::command]
pub fn list_negative_policies() -> Vec<PolicyInfo> {
    vec![
        PolicyInfo {
            value: "error".into(),
            label: NegativeBalancePolicy::Error.label().into(),
            explanation: "اگر برداشت باعث منفی شدن موجودی شود، عملیات انجام نمی‌شود. مناسب صندوق نقدی، چون پولی که در صندوق نیست قابل پرداخت نیست.".into(),
        },
        PolicyInfo {
            value: "warn".into(),
            label: NegativeBalancePolicy::Warn.label().into(),
            explanation: "عملیات انجام می‌شود ولی هشدار داده می‌شود. مناسب حساب بانکی که ممکن است اضافه‌برداشت داشته باشد.".into(),
        },
        PolicyInfo {
            value: "ignore".into(),
            label: NegativeBalancePolicy::Ignore.label().into(),
            explanation: "هیچ بررسی‌ای انجام نمی‌شود. فقط وقتی استفاده کنید که مانده‌ی این حساب را جای دیگری کنترل می‌کنید.".into(),
        },
    ]
}
