//! فرم کامل ثبت و ویرایش شخص — هفت زبانه.
//!
//! مرجع: تصویر `1zkKV5` (فرم افزودن شخص) و `c9pvYl` (لیست اشخاص).
//!
//! ## زبانه‌ها
//!
//! ۱. مشخصات عمومی — نوع شخصیت، نقش، کد، گروه، مسیر، بازاریاب، شناسه‌ها
//! ۲. مشخصات ارتباطی — تلفن‌های چندگانه، آدرس، ایمیل، وب‌سایت
//! ۳. حساب‌های بانکی — چند حساب با شبا و کارت، یکی پیش‌فرض
//! ۴. تصاویر — مسیر فایل، یکی اصلی
//! ۵. مشخصات کاربری — نام کاربری فروشگاه اینترنتی (رمز هش می‌شود)
//! ۶. سایر مشخصات — شغل، نحوه آشنایی، سقف اعتبار، یادداشت
//! ۷. تقویم مناسبت‌ها — تاریخ شمسی تکرارشونده با یادآوری
//!
//! ## چرا یک تراکنش برای همه‌ی زبانه‌ها
//!
//! شخصی که نصف اطلاعاتش ذخیره شده باشد از شخص ذخیره‌نشده بدتر است. کل فرم
//! در یک تراکنش می‌نشیند: یا همه‌چیز ثبت می‌شود یا هیچ‌چیز.
//!
//! ## اعتبارسنجی
//!
//! کد ملی، شناسه ملی حقوقی، کد اقتصادی، کد پستی، موبایل، شبا و شماره کارت
//! همگی با الگوریتم رسمی خودشان بررسی می‌شوند — نه با «طولش درست است».

use novin_core::parties::{
    card_number_is_valid, iban_is_valid, normalize_mobile, PartyDefinition, PartyFunction,
    PartyType,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{active_context, audit, conn, require_permission, AppState};

// ---------------------------------------------------------------------------
// ساختارهای ورودی — یکی به‌ازای هر زبانه
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct PhoneInput {
    #[serde(default)]
    pub title: Option<String>,
    pub number: String,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankAccountInput {
    pub bank_name: String,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub account_number: Option<String>,
    #[serde(default)]
    pub iban: Option<String>,
    #[serde(default)]
    pub card_number: Option<String>,
    #[serde(default)]
    pub holder_name: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageInput {
    #[serde(default)]
    pub title: Option<String>,
    pub file_path: String,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OccasionInput {
    pub title: String,
    pub jalali_month: i64,
    pub jalali_day: i64,
    #[serde(default)]
    pub remind_days_before: i64,
}

/// کل فرم شخص در یک ساختار.
#[derive(Debug, Deserialize)]
pub struct PartyInput {
    #[serde(default)]
    pub id: Option<String>,
    // --- زبانه ۱: مشخصات عمومی ---
    #[serde(default)]
    pub code: Option<String>,
    pub party_type: String,
    pub party_function: String,
    #[serde(default)]
    pub title_prefix: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub national_id: Option<String>,
    #[serde(default)]
    pub economic_code: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub route_id: Option<String>,
    #[serde(default)]
    pub marketer_id: Option<String>,
    #[serde(default)]
    pub opening_date: Option<String>,
    #[serde(default)]
    pub is_customer: bool,
    #[serde(default)]
    pub is_supplier: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    // --- زبانه ۲: مشخصات ارتباطی ---
    #[serde(default)]
    pub mobile: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub province: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub phones: Vec<PhoneInput>,
    // --- زبانه ۳ ---
    #[serde(default)]
    pub bank_accounts: Vec<BankAccountInput>,
    // --- زبانه ۴ ---
    #[serde(default)]
    pub images: Vec<ImageInput>,
    // --- زبانه ۵ ---
    #[serde(default)]
    pub portal_username: Option<String>,
    /// رمز خام؛ فقط هش‌شده ذخیره می‌شود و هرگز برگردانده نمی‌شود.
    #[serde(default)]
    pub portal_password: Option<String>,
    // --- زبانه ۶ ---
    #[serde(default)]
    pub job_title: Option<String>,
    #[serde(default)]
    pub introduction: Option<String>,
    #[serde(default)]
    pub credit_limit: i64,
    #[serde(default)]
    pub note: Option<String>,
    // --- زبانه ۷ ---
    #[serde(default)]
    pub occasions: Vec<OccasionInput>,
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

// ---------------------------------------------------------------------------
// ساختارهای خروجی
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct PartyGroupRow {
    pub id: String,
    pub code: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PhoneRow {
    pub id: String,
    pub title: Option<String>,
    pub number: String,
    pub is_primary: bool,
}

#[derive(Debug, Serialize)]
pub struct BankAccountRow {
    pub id: String,
    pub bank_name: String,
    pub branch_name: Option<String>,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub card_number: Option<String>,
    pub holder_name: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct ImageRow {
    pub id: String,
    pub title: Option<String>,
    pub file_path: String,
    pub is_primary: bool,
}

#[derive(Debug, Serialize)]
pub struct OccasionRow {
    pub id: String,
    pub title: String,
    pub jalali_month: i64,
    pub jalali_day: i64,
    pub remind_days_before: i64,
}

/// جزئیات کامل یک شخص برای بازکردن در فرم ویرایش.
#[derive(Debug, Serialize)]
pub struct PartyDetail {
    pub id: String,
    pub code: Option<String>,
    pub party_type: String,
    pub party_type_label: String,
    pub party_function: String,
    pub party_function_label: String,
    pub title_prefix: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub display_name: String,
    pub national_id: Option<String>,
    pub economic_code: Option<String>,
    pub group_id: Option<String>,
    pub route_id: Option<String>,
    pub marketer_id: Option<String>,
    pub opening_date: Option<String>,
    pub is_customer: bool,
    pub is_supplier: bool,
    pub is_active: bool,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub job_title: Option<String>,
    pub introduction: Option<String>,
    pub credit_limit: i64,
    pub note: Option<String>,
    pub portal_username: Option<String>,
    /// آیا رمز فروشگاه تنظیم شده؟ خود رمز هرگز برنمی‌گردد.
    pub has_portal_password: bool,
    pub phones: Vec<PhoneRow>,
    pub bank_accounts: Vec<BankAccountRow>,
    pub images: Vec<ImageRow>,
    pub occasions: Vec<OccasionRow>,
}

// ---------------------------------------------------------------------------
// اعتبارسنجی
// ---------------------------------------------------------------------------

fn parse_party_type(value: &str) -> Result<PartyType, String> {
    PartyType::parse(value).ok_or_else(|| "PRT-001: نوع شخصیت نامعتبر است".to_string())
}

fn parse_party_function(value: &str) -> Result<PartyFunction, String> {
    PartyFunction::parse(value).ok_or_else(|| "PRT-002: نقش شخص نامعتبر است".to_string())
}

/// اعتبارسنجی زبانه‌های تکمیلی که هسته از آن‌ها خبر ندارد.
fn validate_tabs(input: &PartyInput) -> Result<(), String> {
    // تلفن‌ها
    let mut primary_phones = 0;
    for (index, phone) in input.phones.iter().enumerate() {
        if phone.number.trim().is_empty() {
            return Err(format!("PRT-010: شماره تلفن ردیف {} خالی است", index + 1));
        }
        if phone.is_primary {
            primary_phones += 1;
        }
    }
    if primary_phones > 1 {
        return Err("PRT-011: فقط یک شماره می‌تواند پیش‌فرض باشد".into());
    }

    // حساب‌های بانکی
    let mut default_accounts = 0;
    for (index, account) in input.bank_accounts.iter().enumerate() {
        let row = index + 1;
        if account.bank_name.trim().is_empty() {
            return Err(format!("PRT-012: نام بانک ردیف {row} خالی است"));
        }
        if let Some(iban) = clean(&account.iban) {
            if !iban_is_valid(&iban) {
                return Err(format!("PRT-013: شماره شبای ردیف {row} معتبر نیست"));
            }
        }
        if let Some(card) = clean(&account.card_number) {
            if !card_number_is_valid(&card) {
                return Err(format!("PRT-014: شماره کارت ردیف {row} معتبر نیست"));
            }
        }
        if account.is_default {
            default_accounts += 1;
        }
    }
    if default_accounts > 1 {
        return Err("PRT-015: فقط یک حساب بانکی می‌تواند پیش‌فرض باشد".into());
    }

    // تصاویر
    let mut primary_images = 0;
    for image in &input.images {
        if image.file_path.trim().is_empty() {
            return Err("PRT-016: مسیر تصویر خالی است".into());
        }
        if image.is_primary {
            primary_images += 1;
        }
    }
    if primary_images > 1 {
        return Err("PRT-017: فقط یک تصویر می‌تواند اصلی باشد".into());
    }

    // مناسبت‌ها — روز ۳۱ فقط در شش ماه اول سال شمسی وجود دارد.
    for (index, occasion) in input.occasions.iter().enumerate() {
        let row = index + 1;
        if occasion.title.trim().is_empty() {
            return Err(format!("PRT-018: عنوان مناسبت ردیف {row} خالی است"));
        }
        if !(1..=12).contains(&occasion.jalali_month) {
            return Err(format!("PRT-019: ماه مناسبت ردیف {row} نامعتبر است"));
        }
        let max_day = if occasion.jalali_month <= 6 { 31 } else { 30 };
        if occasion.jalali_day < 1 || occasion.jalali_day > max_day {
            return Err(format!(
                "PRT-020: روز مناسبت ردیف {row} در ماه انتخابی وجود ندارد"
            ));
        }
        if occasion.remind_days_before < 0 || occasion.remind_days_before > 365 {
            return Err(format!("PRT-021: روز یادآوری ردیف {row} نامعتبر است"));
        }
    }

    // ایمیل — بررسی حداقلی ولی واقعی
    if let Some(email) = clean(&input.email) {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || !parts[1].contains('.') {
            return Err("PRT-022: نشانی ایمیل معتبر نیست".into());
        }
    }

    if let Some(username) = clean(&input.portal_username) {
        if username.chars().count() < 4 {
            return Err("PRT-023: نام کاربری باید حداقل چهار نویسه باشد".into());
        }
        if let Some(password) = clean(&input.portal_password) {
            if password.chars().count() < 8 {
                return Err("PRT-024: رمز عبور باید حداقل هشت نویسه باشد".into());
            }
        }
    } else if clean(&input.portal_password).is_some() {
        return Err("PRT-025: برای تعیین رمز، نام کاربری هم لازم است".into());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// فرمان‌ها
// ---------------------------------------------------------------------------

/// درخت گروه‌های اشخاص با تعداد اعضای هر گروه.
#[tauri::command]
pub fn list_party_groups(state: State<AppState>) -> Result<Vec<PartyGroupRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let mut statement = tx
        .prepare(
            "SELECT g.id,g.code,g.title,g.parent_id,\
             (SELECT COUNT(*) FROM contacts c WHERE c.group_id=g.id) \
             FROM party_groups g WHERE g.company_id=?1 AND g.is_active=1 ORDER BY g.code",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![company], |row| {
            Ok(PartyGroupRow {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                parent_id: row.get(3)?,
                member_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// ساخت یا ویرایش گروه اشخاص.
#[tauri::command]
pub fn save_party_group(
    state: State<AppState>,
    id: Option<String>,
    code: String,
    title: String,
    parent_id: Option<String>,
) -> Result<String, String> {
    if code.trim().is_empty() || title.trim().is_empty() {
        return Err("PRT-030: کد و عنوان گروه الزامی است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    // گروه نمی‌تواند والد خودش باشد — حلقه در درخت، صفحه را قفل می‌کند.
    if let (Some(existing), Some(parent)) = (&id, &parent_id) {
        if existing == parent {
            return Err("PRT-031: گروه نمی‌تواند والد خودش باشد".into());
        }
    }

    let group_id = match &id {
        Some(existing) => {
            tx.execute(
                "UPDATE party_groups SET code=?1,title=?2,parent_id=?3 WHERE id=?4 AND company_id=?5",
                params![code.trim(), title.trim(), parent_id, existing, company],
            )
            .map_err(|e| format!("PRT-032: {e}"))?;
            existing.clone()
        }
        None => {
            let new_id = format!(
                "pgroup-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO party_groups(id,company_id,code,title,parent_id) VALUES(?1,?2,?3,?4,?5)",
                params![new_id, company, code.trim(), title.trim(), parent_id],
            )
            .map_err(|e| format!("PRT-033: {e}"))?;
            new_id
        }
    };
    tx.commit().map_err(|e| e.to_string())?;
    Ok(group_id)
}

/// خواندن کامل یک شخص با همه‌ی زبانه‌ها.
#[tauri::command]
pub fn get_party(state: State<AppState>, id: String) -> Result<PartyDetail, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let mut detail = tx
        .query_row(
            "SELECT id,code,party_type,party_function,title_prefix,first_name,last_name,\
             company_name,name,national_id,economic_code,group_id,route_id,marketer_id,\
             opening_date,is_customer,is_supplier,is_active,mobile,email,website,province,city,\
             address,postal_code,job_title,introduction,credit_limit,note,portal_username,\
             portal_password_hash FROM contacts WHERE id=?1 AND company_id=?2",
            params![id, company],
            |row| {
                let party_type: String = row.get(2)?;
                let party_function: String = row.get(3)?;
                let password_hash: Option<String> = row.get(30)?;
                Ok(PartyDetail {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    party_type_label: PartyType::parse(&party_type)
                        .map(|t| t.label().to_string())
                        .unwrap_or_else(|| party_type.clone()),
                    party_type,
                    party_function_label: PartyFunction::parse(&party_function)
                        .map(|f| f.label().to_string())
                        .unwrap_or_else(|| party_function.clone()),
                    party_function,
                    title_prefix: row.get(4)?,
                    first_name: row.get(5)?,
                    last_name: row.get(6)?,
                    company_name: row.get(7)?,
                    display_name: row.get(8)?,
                    national_id: row.get(9)?,
                    economic_code: row.get(10)?,
                    group_id: row.get(11)?,
                    route_id: row.get(12)?,
                    marketer_id: row.get(13)?,
                    opening_date: row.get(14)?,
                    is_customer: row.get::<_, i64>(15)? == 1,
                    is_supplier: row.get::<_, i64>(16)? == 1,
                    is_active: row.get::<_, i64>(17)? == 1,
                    mobile: row.get(18)?,
                    email: row.get(19)?,
                    website: row.get(20)?,
                    province: row.get(21)?,
                    city: row.get(22)?,
                    address: row.get(23)?,
                    postal_code: row.get(24)?,
                    job_title: row.get(25)?,
                    introduction: row.get(26)?,
                    credit_limit: row.get(27)?,
                    note: row.get(28)?,
                    portal_username: row.get(29)?,
                    has_portal_password: password_hash.is_some(),
                    phones: Vec::new(),
                    bank_accounts: Vec::new(),
                    images: Vec::new(),
                    occasions: Vec::new(),
                })
            },
        )
        .map_err(|_| "PRT-003: شخص یافت نشد".to_string())?;

    {
        let mut statement = tx
            .prepare(
                "SELECT id,title,number,is_primary FROM party_phones WHERE contact_id=?1 ORDER BY is_primary DESC,id",
            )
            .map_err(|e| e.to_string())?;
        detail.phones = statement
            .query_map(params![id], |row| {
                Ok(PhoneRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    number: row.get(2)?,
                    is_primary: row.get::<_, i64>(3)? == 1,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
    }
    {
        let mut statement = tx
            .prepare(
                "SELECT id,bank_name,branch_name,account_number,iban,card_number,holder_name,\
                 is_default FROM party_bank_accounts WHERE contact_id=?1 ORDER BY is_default DESC,id",
            )
            .map_err(|e| e.to_string())?;
        detail.bank_accounts = statement
            .query_map(params![id], |row| {
                Ok(BankAccountRow {
                    id: row.get(0)?,
                    bank_name: row.get(1)?,
                    branch_name: row.get(2)?,
                    account_number: row.get(3)?,
                    iban: row.get(4)?,
                    card_number: row.get(5)?,
                    holder_name: row.get(6)?,
                    is_default: row.get::<_, i64>(7)? == 1,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
    }
    {
        let mut statement = tx
            .prepare(
                "SELECT id,title,file_path,is_primary FROM party_images WHERE contact_id=?1 ORDER BY is_primary DESC,id",
            )
            .map_err(|e| e.to_string())?;
        detail.images = statement
            .query_map(params![id], |row| {
                Ok(ImageRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    file_path: row.get(2)?,
                    is_primary: row.get::<_, i64>(3)? == 1,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
    }
    {
        let mut statement = tx
            .prepare(
                "SELECT id,title,jalali_month,jalali_day,remind_days_before FROM party_occasions \
                 WHERE contact_id=?1 ORDER BY jalali_month,jalali_day",
            )
            .map_err(|e| e.to_string())?;
        detail.occasions = statement
            .query_map(params![id], |row| {
                Ok(OccasionRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    jalali_month: row.get(2)?,
                    jalali_day: row.get(3)?,
                    remind_days_before: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
    }

    Ok(detail)
}

/// ذخیره‌ی کامل شخص با همه‌ی هفت زبانه، در یک تراکنش.
#[tauri::command]
pub fn save_party(state: State<AppState>, input: PartyInput) -> Result<String, String> {
    let party_type = parse_party_type(&input.party_type)?;
    let party_function = parse_party_function(&input.party_function)?;

    // اعتبارسنجی هسته: نام، شناسه‌ها، موبایل، سقف اعتبار و نقش تجاری.
    let definition = PartyDefinition {
        code: input.code.clone().unwrap_or_default(),
        party_type,
        function: party_function,
        first_name: clean(&input.first_name),
        last_name: clean(&input.last_name),
        company_name: clean(&input.company_name),
        national_id: clean(&input.national_id),
        economic_code: clean(&input.economic_code),
        postal_code: clean(&input.postal_code),
        mobile: clean(&input.mobile),
        is_customer: input.is_customer,
        is_supplier: input.is_supplier,
        credit_limit: input.credit_limit,
        route: clean(&input.route_id),
        marketer_code: clean(&input.marketer_id),
    };
    definition
        .validate()
        .map_err(|error| format!("PRT-004: {error}"))?;
    validate_tabs(&input)?;

    let display_name = definition.display_name();
    // موبایل به شکل استاندارد ۱۱ رقمی ذخیره می‌شود تا جستجو و تشخیص تکراری کار کند.
    let mobile = clean(&input.mobile).and_then(|value| normalize_mobile(&value));

    let mut c = conn(&state)?;
    let permission = if input.id.is_some() {
        "contacts.update"
    } else {
        "contacts.create"
    };
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    // ارجاع‌های اختیاری باید متعلق به همین شرکت باشند.
    for (table, value, error) in [
        ("party_groups", clean(&input.group_id), "PRT-005: گروه معتبر نیست"),
        ("party_routes", clean(&input.route_id), "PRT-006: مسیر معتبر نیست"),
        ("contacts", clean(&input.marketer_id), "PRT-007: بازاریاب معتبر نیست"),
    ] {
        if let Some(reference) = value {
            let sql = format!("SELECT COUNT(*) FROM {table} WHERE id=?1 AND company_id=?2");
            let ok: i64 = tx
                .query_row(&sql, params![reference, company], |row| row.get(0))
                .unwrap_or(0);
            if ok == 0 {
                return Err(error.into());
            }
        }
    }

    // کد شخص در هر شرکت یکتاست؛ اگر خالی بود، خودکار ساخته می‌شود.
    let code = match clean(&input.code) {
        Some(value) => {
            let duplicate: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM contacts WHERE company_id=?1 AND code=?2 AND id<>?3",
                    params![company, value, input.id.clone().unwrap_or_default()],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if duplicate > 0 {
                return Err("PRT-008: کد شخص تکراری است".into());
            }
            value
        }
        None => {
            let next: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(CAST(code AS INTEGER)),1000)+1 FROM contacts \
                     WHERE company_id=?1 AND code GLOB '[0-9]*'",
                    params![company],
                    |row| row.get(0),
                )
                .unwrap_or(1001);
            next.to_string()
        }
    };

    let kind = if party_type.is_legal_entity() {
        "company"
    } else {
        "person"
    };
    let password_hash = clean(&input.portal_password).map(|value| db_hash(&value));

    let contact_id = match &input.id {
        Some(existing) => {
            let owned: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",
                    params![existing, company],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if owned == 0 {
                return Err("PRT-003: شخص یافت نشد".into());
            }
            tx.execute(
                "UPDATE contacts SET code=?1,kind=?2,name=?3,party_type=?4,party_function=?5,\
                 title_prefix=?6,first_name=?7,last_name=?8,company_name=?9,national_id=?10,\
                 economic_code=?11,group_id=?12,route_id=?13,marketer_id=?14,opening_date=?15,\
                 is_customer=?16,is_supplier=?17,is_active=?18,mobile=?19,email=?20,website=?21,\
                 province=?22,city=?23,address=?24,postal_code=?25,job_title=?26,introduction=?27,\
                 credit_limit=?28,note=?29,portal_username=?30 WHERE id=?31",
                params![
                    code,
                    kind,
                    display_name,
                    party_type.as_str(),
                    party_function.as_str(),
                    clean(&input.title_prefix),
                    clean(&input.first_name),
                    clean(&input.last_name),
                    clean(&input.company_name),
                    clean(&input.national_id),
                    clean(&input.economic_code),
                    clean(&input.group_id),
                    clean(&input.route_id),
                    clean(&input.marketer_id),
                    clean(&input.opening_date),
                    i64::from(input.is_customer),
                    i64::from(input.is_supplier),
                    i64::from(input.is_active),
                    mobile,
                    clean(&input.email),
                    clean(&input.website),
                    clean(&input.province),
                    clean(&input.city),
                    clean(&input.address),
                    clean(&input.postal_code),
                    clean(&input.job_title),
                    clean(&input.introduction),
                    input.credit_limit,
                    clean(&input.note),
                    clean(&input.portal_username),
                    existing
                ],
            )
            .map_err(|e| format!("PRT-009: {e}"))?;
            // رمز فقط وقتی عوض می‌شود که کاربر رمز تازه وارد کرده باشد.
            if let Some(hash) = &password_hash {
                tx.execute(
                    "UPDATE contacts SET portal_password_hash=?1 WHERE id=?2",
                    params![hash, existing],
                )
                .map_err(|e| e.to_string())?;
            }
            existing.clone()
        }
        None => {
            let new_id = format!(
                "contact-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO contacts(id,company_id,code,kind,name,party_type,party_function,\
                 title_prefix,first_name,last_name,company_name,national_id,economic_code,group_id,\
                 route_id,marketer_id,opening_date,is_customer,is_supplier,is_active,mobile,email,\
                 website,province,city,address,postal_code,job_title,introduction,credit_limit,\
                 note,portal_username,portal_password_hash) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,\
                 ?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33)",
                params![
                    new_id,
                    company,
                    code,
                    kind,
                    display_name,
                    party_type.as_str(),
                    party_function.as_str(),
                    clean(&input.title_prefix),
                    clean(&input.first_name),
                    clean(&input.last_name),
                    clean(&input.company_name),
                    clean(&input.national_id),
                    clean(&input.economic_code),
                    clean(&input.group_id),
                    clean(&input.route_id),
                    clean(&input.marketer_id),
                    clean(&input.opening_date),
                    i64::from(input.is_customer),
                    i64::from(input.is_supplier),
                    i64::from(input.is_active),
                    mobile,
                    clean(&input.email),
                    clean(&input.website),
                    clean(&input.province),
                    clean(&input.city),
                    clean(&input.address),
                    clean(&input.postal_code),
                    clean(&input.job_title),
                    clean(&input.introduction),
                    input.credit_limit,
                    clean(&input.note),
                    clean(&input.portal_username),
                    password_hash
                ],
            )
            .map_err(|e| format!("PRT-009: {e}"))?;
            new_id
        }
    };

    // زبانه‌های چندردیفی: جایگزینی کامل ساده‌تر و امن‌تر از تطبیق ردیف‌هاست،
    // چون هیچ سند حسابداری‌ای به این ردیف‌ها ارجاع نمی‌دهد.
    for table in [
        "party_phones",
        "party_bank_accounts",
        "party_images",
        "party_occasions",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE contact_id=?1"),
            params![contact_id],
        )
        .map_err(|e| e.to_string())?;
    }

    for (index, phone) in input.phones.iter().enumerate() {
        tx.execute(
            "INSERT INTO party_phones(id,contact_id,title,number,is_primary) VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{contact_id}-phone-{index}"),
                contact_id,
                clean(&phone.title),
                phone.number.trim(),
                i64::from(phone.is_primary)
            ],
        )
        .map_err(|e| format!("PRT-026: {e}"))?;
    }
    for (index, account) in input.bank_accounts.iter().enumerate() {
        tx.execute(
            "INSERT INTO party_bank_accounts(id,contact_id,bank_name,branch_name,account_number,\
             iban,card_number,holder_name,is_default) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                format!("{contact_id}-bank-{index}"),
                contact_id,
                account.bank_name.trim(),
                clean(&account.branch_name),
                clean(&account.account_number),
                clean(&account.iban),
                clean(&account.card_number),
                clean(&account.holder_name),
                i64::from(account.is_default)
            ],
        )
        .map_err(|e| format!("PRT-027: {e}"))?;
    }
    for (index, image) in input.images.iter().enumerate() {
        tx.execute(
            "INSERT INTO party_images(id,contact_id,title,file_path,is_primary) VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{contact_id}-image-{index}"),
                contact_id,
                clean(&image.title),
                image.file_path.trim(),
                i64::from(image.is_primary)
            ],
        )
        .map_err(|e| format!("PRT-028: {e}"))?;
    }
    for (index, occasion) in input.occasions.iter().enumerate() {
        tx.execute(
            "INSERT INTO party_occasions(id,contact_id,title,jalali_month,jalali_day,\
             remind_days_before) VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{contact_id}-occasion-{index}"),
                contact_id,
                occasion.title.trim(),
                occasion.jalali_month,
                occasion.jalali_day,
                occasion.remind_days_before
            ],
        )
        .map_err(|e| format!("PRT-029: {e}"))?;
    }

    audit(
        &tx,
        &user,
        "contacts.save",
        "contact",
        &contact_id,
        None,
        Some(&format!("{{\"name\":\"{display_name}\",\"code\":\"{code}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(contact_id)
}

/// هش رمز فروشگاه با همان الگوریتمی که برای کاربران سیستم استفاده می‌شود.
fn db_hash(password: &str) -> String {
    novin_core::db::hash_password(password)
}

/// انواع شخصیت و نقش‌ها با برچسب فارسی — تا فرم فهرست را از backend بگیرد.
#[derive(Debug, Serialize)]
pub struct LabelledOption {
    pub value: String,
    pub label: String,
}

#[tauri::command]
pub fn list_party_options() -> serde_json::Value {
    let types: Vec<LabelledOption> = [
        PartyType::Natural,
        PartyType::PrivateLegal,
        PartyType::GovernmentLegal,
        PartyType::CivilPartnership,
    ]
    .into_iter()
    .map(|value| LabelledOption {
        value: value.as_str().to_string(),
        label: value.label().to_string(),
    })
    .collect();
    let functions: Vec<LabelledOption> = [
        PartyFunction::Person,
        PartyFunction::Marketer,
        PartyFunction::Supervisor,
    ]
    .into_iter()
    .map(|value| LabelledOption {
        value: value.as_str().to_string(),
        label: value.label().to_string(),
    })
    .collect();
    serde_json::json!({ "party_types": types, "party_functions": functions })
}

/// یافتن مناسبت‌های نزدیک — پایه‌ی یادآوری تولد و سالگرد مشتریان.
#[derive(Debug, Serialize)]
pub struct UpcomingOccasion {
    pub contact_id: String,
    pub contact_name: String,
    pub title: String,
    pub jalali_month: i64,
    pub jalali_day: i64,
    pub remind_days_before: i64,
}

#[tauri::command]
pub fn list_upcoming_occasions(
    state: State<AppState>,
    jalali_month: i64,
) -> Result<Vec<UpcomingOccasion>, String> {
    if !(1..=12).contains(&jalali_month) {
        return Err("PRT-019: ماه نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let mut statement = tx
        .prepare(
            "SELECT o.contact_id,c.name,o.title,o.jalali_month,o.jalali_day,o.remind_days_before \
             FROM party_occasions o JOIN contacts c ON c.id=o.contact_id \
             WHERE c.company_id=?1 AND o.jalali_month=?2 ORDER BY o.jalali_day",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![company, jalali_month], |row| {
            Ok(UpcomingOccasion {
                contact_id: row.get(0)?,
                contact_name: row.get(1)?,
                title: row.get(2)?,
                jalali_month: row.get(3)?,
                jalali_day: row.get(4)?,
                remind_days_before: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// غیرفعال‌کردن شخص — حذف نمی‌کنیم چون سند به او ارجاع دارد.
#[tauri::command]
pub fn deactivate_party(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.update")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let open_checks: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM checks WHERE party_id=?1 \
             AND status IN ('in_hand','deposited','endorsed','outstanding')",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if open_checks > 0 {
        return Err("PRT-034: این شخص چک باز دارد و غیرفعال نمی‌شود".into());
    }
    let unpaid: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sales_invoices WHERE contact_id=?1 AND payment_status<>'paid' \
             AND status='posted'",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if unpaid > 0 {
        return Err("PRT-035: این شخص فاکتور تسویه‌نشده دارد".into());
    }

    let changed = tx
        .execute(
            "UPDATE contacts SET is_active=0 WHERE id=?1 AND company_id=?2",
            params![id, company],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("PRT-003: شخص یافت نشد".into());
    }
    audit(&tx, &user, "contacts.deactivate", "contact", &id, None, None)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// تشخیص شخص تکراری بر اساس موبایل یا کد ملی — پیش از ذخیره.
#[tauri::command]
pub fn find_duplicate_party(
    state: State<AppState>,
    mobile: Option<String>,
    national_id: Option<String>,
    exclude_id: Option<String>,
) -> Result<Option<String>, String> {
    let normalized = mobile.as_deref().and_then(normalize_mobile);
    let identifier = national_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    if normalized.is_none() && identifier.is_none() {
        return Ok(None);
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let found: Option<String> = tx
        .query_row(
            "SELECT name FROM contacts WHERE company_id=?1 AND id<>?2 \
             AND ((?3 IS NOT NULL AND mobile=?3) OR (?4 IS NOT NULL AND national_id=?4)) LIMIT 1",
            params![
                company,
                exclude_id.unwrap_or_default(),
                normalized,
                identifier
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(found)
}
