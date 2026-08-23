//! مرکز تنظیمات — رجیستری تنظیمات واقعی برنامه.
//!
//! مرجع: تصویر `k51J4O` (مرکز تنظیمات با زبانه‌های متعدد).
//!
//! ## قاعده‌ی این ماژول: هیچ تنظیم تزئینی
//!
//! هر تنظیمی که اینجا اعلام می‌شود **باید جایی در کد خوانده شود**. تنظیمی که
//! هیچ اثری ندارد بدتر از نبودنش است: کاربر آن را عوض می‌کند، انتظار تغییر
//! رفتار دارد و چیزی عوض نمی‌شود.
//!
//! ستون «کجا اثر می‌گذارد» در تعریف هر تنظیم اجباری است و در رابط کاربری هم
//! نمایش داده می‌شود، تا کاربر بداند تغییرش چه چیزی را عوض می‌کند.
//!
//! ## اعتبارسنجی
//!
//! نوع و دامنه‌ی هر تنظیم همین‌جا اعلام و بررسی می‌شود. مقدار نامعتبر هرگز
//! ذخیره نمی‌شود؛ چون تنظیم خراب می‌تواند محاسبه‌ی مالی را خراب کند
//! (مثلاً نرخ مالیات منفی).

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

use crate::{audit, conn, require_login, require_permission, AppState};

/// نوع مقدار یک تنظیم — رابط کاربری از روی همین، کنترل مناسب را می‌سازد.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    Boolean,
    Integer,
    Text,
    Choice,
}

/// یک گزینه در تنظیم انتخابی.
#[derive(Debug, Clone, Serialize)]
pub struct SettingChoice {
    pub value: &'static str,
    pub label: &'static str,
}

/// تعریف کامل یک تنظیم.
#[derive(Debug, Clone, Serialize)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub group: &'static str,
    pub group_label: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// دقیقاً کجای برنامه این تنظیم خوانده می‌شود.
    pub effect: &'static str,
    pub kind: SettingKind,
    pub default_value: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<SettingChoice>,
    /// آیا تغییرش نیاز به مجوز مدیریتی دارد؟
    pub sensitive: bool,
}

fn choice(value: &'static str, label: &'static str) -> SettingChoice {
    SettingChoice { value, label }
}

/// رجیستری تنظیمات.
///
/// هر ورودی این فهرست در کد خوانده می‌شود — فیلد `effect` می‌گوید کجا.
pub fn registry() -> Vec<SettingDefinition> {
    vec![
        // ---------------- انبار ----------------
        SettingDefinition {
            key: "inventory_valuation_method",
            group: "inventory",
            group_label: "انبار",
            label: "روش ارزش‌گذاری موجودی",
            description: "بهای تمام‌شده‌ی کالای خارج‌شده از انبار با کدام روش محاسبه شود.",
            effect: "گزارش ارزش موجودی، بهای تمام‌شده‌ی فروش، سند تعدیل انبارگردانی",
            kind: SettingKind::Choice,
            default_value: "weighted_average",
            min: None,
            max: None,
            choices: vec![
                choice("weighted_average", "میانگین موزون"),
                choice("fifo", "اولین صادره از اولین وارده (FIFO)"),
                choice("lifo", "اولین صادره از آخرین وارده (LIFO)"),
            ],
            sensitive: true,
        },
        SettingDefinition {
            key: "inventory.low_stock_threshold",
            group: "inventory",
            group_label: "انبار",
            label: "حد هشدار کمبود موجودی",
            description: "اگر موجودی کالا از این عدد کمتر شود، در کارت «نزدیک به اتمام موجودی» دیده می‌شود.",
            effect: "داشبورد انبار — کارت نزدیک به اتمام موجودی",
            kind: SettingKind::Integer,
            default_value: "5",
            min: Some(0),
            max: Some(100_000),
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "inventory.recount_threshold_percent",
            group: "inventory",
            group_label: "انبار",
            label: "درصد اختلاف الزام‌آور شمارش مجدد",
            description: "اگر اختلاف شمارش از این درصد بیشتر باشد، شمارش دوم اجباری می‌شود.",
            effect: "انبارگردانی — الزام شمارش مجدد پیش از تأیید اختلاف",
            kind: SettingKind::Integer,
            default_value: "5",
            min: Some(0),
            max: Some(100),
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "inventory.allow_negative_stock",
            group: "inventory",
            group_label: "انبار",
            label: "اجازه‌ی منفی شدن موجودی",
            description: "اگر فعال باشد، فروش بیش از موجودی ممکن می‌شود. برای فروشگاه‌هایی که ثبت ورود کالا با تأخیر انجام می‌دهند.",
            effect: "ثبت فاکتور فروش و رسید تولید — کنترل کفایت موجودی",
            kind: SettingKind::Boolean,
            default_value: "false",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: true,
        },
        // ---------------- فروش و خرید ----------------
        SettingDefinition {
            key: "sales.default_vat_basis_points",
            group: "sales",
            group_label: "فروش و خرید",
            label: "نرخ پیش‌فرض مالیات بر ارزش افزوده",
            description: "بر حسب صدم‌درصد؛ ۹۰۰ یعنی ۹ درصد. مبنای مالیات، مبلغ پس از تخفیف است.",
            effect: "فرم فاکتور فروش، پیش‌فاکتور و سفارش خرید — مقدار اولیه‌ی نرخ مالیات",
            kind: SettingKind::Integer,
            default_value: "900",
            min: Some(0),
            max: Some(10_000),
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "quotes.default_validity_days",
            group: "sales",
            group_label: "فروش و خرید",
            label: "اعتبار پیش‌فرض پیش‌فاکتور (روز)",
            description: "تاریخ اعتبار پیش‌فاکتور به‌طور خودکار این تعداد روز بعد از تاریخ صدور پیشنهاد می‌شود.",
            effect: "فرم پیش‌فاکتور — مقدار اولیه‌ی «اعتبار تا»",
            kind: SettingKind::Integer,
            default_value: "30",
            min: Some(1),
            max: Some(365),
            choices: Vec::new(),
            sensitive: false,
        },
        // ---------------- خزانه و چک ----------------
        SettingDefinition {
            key: "treasury.default_negative_policy",
            group: "treasury",
            group_label: "خزانه و چک",
            label: "سیاست پیش‌فرض منفی شدن موجودی",
            description: "برای حساب خزانه‌ی تازه‌ساخته‌شده. صندوق نقدی معمولاً «خطا» و حساب بانکی «هشدار» می‌گیرد.",
            effect: "فرم تعریف صندوق و بانک — مقدار اولیه‌ی سیاست منفی",
            kind: SettingKind::Choice,
            default_value: "warn",
            min: None,
            max: None,
            choices: vec![
                choice("error", "خطا — عملیات انجام نمی‌شود"),
                choice("warn", "هشدار — انجام می‌شود ولی پیام داده می‌شود"),
                choice("ignore", "بی‌تأثیر"),
            ],
            sensitive: false,
        },
        SettingDefinition {
            key: "checks.due_soon_days",
            group: "treasury",
            group_label: "خزانه و چک",
            label: "بازه‌ی هشدار سررسید چک (روز)",
            description: "چک‌هایی که تا این تعداد روز آینده سررسید می‌شوند، در داشبورد چک هشدار می‌گیرند.",
            effect: "داشبورد چک‌ها — شمارنده‌ی «نزدیک سررسید»",
            kind: SettingKind::Integer,
            default_value: "7",
            min: Some(1),
            max: Some(180),
            choices: Vec::new(),
            sensitive: false,
        },
        // ---------------- اشخاص ----------------
        SettingDefinition {
            key: "parties.require_national_id",
            group: "parties",
            group_label: "اشخاص",
            label: "الزام کد ملی / شناسه ملی",
            description: "اگر فعال باشد، ثبت شخص بدون کد ملی (حقیقی) یا شناسه ملی (حقوقی) ممکن نیست. برای صدور صورتحساب رسمی لازم است.",
            effect: "فرم ثبت شخص — اعتبارسنجی پیش از ذخیره",
            kind: SettingKind::Boolean,
            default_value: "false",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "parties.enforce_credit_limit",
            group: "parties",
            group_label: "اشخاص",
            label: "اعمال سقف اعتبار",
            description: "اگر فعال باشد، فروش نسیه بیش از سقف اعتبار شخص متوقف می‌شود؛ در غیر این صورت فقط هشدار داده می‌شود.",
            effect: "ثبت فاکتور فروش نسیه — کنترل سقف اعتبار",
            kind: SettingKind::Boolean,
            default_value: "true",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: false,
        },
        // ---------------- تولید ----------------
        SettingDefinition {
            key: "production.default_cost_allocation",
            group: "production",
            group_label: "تولید",
            label: "روش پیش‌فرض تخصیص بهای تمام‌شده",
            description: "وقتی یک رسید تولید چند محصول دارد، بهای تمام‌شده با کدام روش تقسیم شود.",
            effect: "فرم رسید تولید — مقدار اولیه‌ی روش تخصیص",
            kind: SettingKind::Choice,
            default_value: "by_quantity",
            min: None,
            max: None,
            choices: vec![
                choice("by_quantity", "بر اساس مقدار"),
                choice("by_market_value", "بر اساس ارزش بازار"),
            ],
            sensitive: false,
        },
        // ---------------- حسابداری ----------------
        SettingDefinition {
            key: "accounting.require_description",
            group: "accounting",
            group_label: "حسابداری",
            label: "الزام شرح در سطر سند",
            description: "اگر فعال باشد، هیچ سطر سندی بدون شرح ثبت نمی‌شود. برای حسابرسی‌پذیری توصیه می‌شود.",
            effect: "ثبت سند حسابداری — اعتبارسنجی سطرها",
            kind: SettingKind::Boolean,
            default_value: "false",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "coding.level_widths",
            group: "accounting",
            group_label: "حسابداری",
            label: "طرح کدینگ (عرض هر سطح)",
            description: "تعداد رقم هر سطح کدینگ، جدا شده با ویرگول. پیش‌فرض ۱،۲،۲،۲ یعنی گروه یک رقم و تفصیلی هفت رقم.",
            effect: "کدینگ حساب‌ها — پیشنهاد کد بعدی و گزارش سلامت کدینگ",
            kind: SettingKind::Text,
            default_value: "1,2,2,2",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: true,
        },
        // ---------------- ظاهر ----------------
        SettingDefinition {
            key: "appearance.dark_mode",
            group: "appearance",
            group_label: "ظاهر",
            label: "تم تاریک",
            description: "حالت نمایش برنامه در شروع.",
            effect: "پوسته‌ی برنامه — تم اولیه هنگام باز شدن",
            kind: SettingKind::Boolean,
            default_value: "false",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "appearance.sidebar_collapsed",
            group: "appearance",
            group_label: "ظاهر",
            label: "منوی جمع‌شده در شروع",
            description: "منوی کناری در شروع جمع‌شده باشد تا فضای بیشتری برای جدول‌ها بماند.",
            effect: "پوسته‌ی برنامه — وضعیت اولیه‌ی منوی کناری",
            kind: SettingKind::Boolean,
            default_value: "false",
            min: None,
            max: None,
            choices: Vec::new(),
            sensitive: false,
        },
        SettingDefinition {
            key: "appearance.rows_per_page",
            group: "appearance",
            group_label: "ظاهر",
            label: "تعداد ردیف در هر صفحه",
            description: "تعداد ردیفی که در جدول‌های بلند یک‌جا نمایش داده می‌شود.",
            effect: "جدول‌های فهرست — اندازه‌ی صفحه",
            kind: SettingKind::Integer,
            default_value: "50",
            min: Some(10),
            max: Some(500),
            choices: Vec::new(),
            sensitive: false,
        },
    ]
}

/// یک تنظیم همراه با مقدار فعلی‌اش.
#[derive(Debug, Serialize)]
pub struct SettingWithValue {
    #[serde(flatten)]
    pub definition: SettingDefinition,
    pub value: String,
    /// آیا کاربر مقدارش را از پیش‌فرض تغییر داده؟
    pub is_customized: bool,
}

/// خواندن یک تنظیم با مقدار پیش‌فرض به‌عنوان پشتیبان.
///
/// این تابع نقطه‌ی واحد خواندن تنظیمات در کل میزبان است.
pub fn read_setting(conn: &rusqlite::Connection, key: &str) -> String {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key=?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten();
    stored.unwrap_or_else(|| {
        registry()
            .into_iter()
            .find(|definition| definition.key == key)
            .map(|definition| definition.default_value.to_string())
            .unwrap_or_default()
    })
}

/// خواندن تنظیم عددی با محدود شدن به دامنه‌ی مجاز.
pub fn read_integer(conn: &rusqlite::Connection, key: &str) -> i64 {
    let raw = read_setting(conn, key);
    let parsed = raw.trim().parse::<i64>().ok();
    let definition = registry().into_iter().find(|item| item.key == key);
    let fallback = definition
        .as_ref()
        .and_then(|item| item.default_value.parse::<i64>().ok())
        .unwrap_or(0);
    let value = parsed.unwrap_or(fallback);
    match definition {
        Some(item) => value
            .max(item.min.unwrap_or(i64::MIN))
            .min(item.max.unwrap_or(i64::MAX)),
        None => value,
    }
}

/// خواندن تنظیم بولی.
pub fn read_boolean(conn: &rusqlite::Connection, key: &str) -> bool {
    matches!(
        read_setting(conn, key).trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

/// همه‌ی تنظیمات با مقدار فعلی.
#[tauri::command]
pub fn list_settings(state: State<AppState>) -> Result<Vec<SettingWithValue>, String> {
    let c = conn(&state)?;
    require_login(&state)?;
    Ok(registry()
        .into_iter()
        .map(|definition| {
            let stored: Option<String> = c
                .query_row(
                    "SELECT value FROM app_settings WHERE key=?1",
                    params![definition.key],
                    |row| row.get(0),
                )
                .optional()
                .ok()
                .flatten();
            let is_customized = stored
                .as_deref()
                .map(|value| value != definition.default_value)
                .unwrap_or(false);
            SettingWithValue {
                value: stored.unwrap_or_else(|| definition.default_value.to_string()),
                definition,
                is_customized,
            }
        })
        .collect())
}

/// اعتبارسنجی مقدار یک تنظیم بر اساس تعریفش.
fn validate(definition: &SettingDefinition, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    match definition.kind {
        SettingKind::Boolean => match trimmed.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Ok("true".into()),
            "false" | "0" | "no" | "off" => Ok("false".into()),
            _ => Err(format!(
                "SET-001: مقدار «{}» برای تنظیم بله/خیر معتبر نیست",
                definition.label
            )),
        },
        SettingKind::Integer => {
            let parsed: i64 = trimmed
                .parse()
                .map_err(|_| format!("SET-002: «{}» باید یک عدد صحیح باشد", definition.label))?;
            if let Some(min) = definition.min {
                if parsed < min {
                    return Err(format!(
                        "SET-003: «{}» نمی‌تواند کمتر از {min} باشد",
                        definition.label
                    ));
                }
            }
            if let Some(max) = definition.max {
                if parsed > max {
                    return Err(format!(
                        "SET-004: «{}» نمی‌تواند بیشتر از {max} باشد",
                        definition.label
                    ));
                }
            }
            Ok(parsed.to_string())
        }
        SettingKind::Choice => {
            if definition
                .choices
                .iter()
                .any(|option| option.value == trimmed)
            {
                Ok(trimmed.to_string())
            } else {
                Err(format!(
                    "SET-005: گزینه‌ی انتخابی برای «{}» معتبر نیست",
                    definition.label
                ))
            }
        }
        SettingKind::Text => {
            if trimmed.is_empty() {
                return Err(format!(
                    "SET-006: «{}» نمی‌تواند خالی باشد",
                    definition.label
                ));
            }
            // طرح کدینگ ساختار مشخصی دارد و باید همان‌جا بررسی شود.
            if definition.key == "coding.level_widths" {
                let widths: Result<Vec<u8>, _> = trimmed
                    .split(',')
                    .map(|part| part.trim().parse::<u8>())
                    .collect();
                let widths = widths.map_err(|_| {
                    "SET-007: طرح کدینگ باید اعداد جداشده با ویرگول باشد".to_string()
                })?;
                if widths.is_empty() || widths.iter().any(|width| *width == 0 || *width > 6) {
                    return Err("SET-008: عرض هر سطح کدینگ باید بین ۱ تا ۶ رقم باشد".into());
                }
            }
            Ok(trimmed.to_string())
        }
    }
}

/// ذخیره‌ی یک تنظیم با اعتبارسنجی کامل.
#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<String, String> {
    let definition = registry()
        .into_iter()
        .find(|item| item.key == key)
        .ok_or_else(|| format!("SET-009: تنظیم «{key}» شناخته نمی‌شود"))?;

    let mut c = conn(&state)?;
    // تنظیمات حساس رفتار مالی را عوض می‌کنند، پس مجوز مدیریتی می‌خواهند.
    let user = if definition.sensitive {
        require_permission(&state, &c, "accounting.journal.create")?
    } else {
        require_login(&state)?
    };

    let normalized = validate(&definition, &value)?;
    let previous = read_setting(&c, &key);

    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,?2) \
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, normalized],
    )
    .map_err(|e| format!("SET-010: {e}"))?;
    audit(
        &tx,
        &user,
        "settings.update",
        "setting",
        &key,
        Some(&format!("{{\"value\":\"{previous}\"}}")),
        Some(&format!("{{\"value\":\"{normalized}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(normalized)
}

/// بازگرداندن یک تنظیم به مقدار پیش‌فرض.
#[tauri::command]
pub fn reset_setting(state: State<AppState>, key: String) -> Result<String, String> {
    let definition = registry()
        .into_iter()
        .find(|item| item.key == key)
        .ok_or_else(|| format!("SET-009: تنظیم «{key}» شناخته نمی‌شود"))?;
    set_setting(state, key, definition.default_value.to_string())
}
