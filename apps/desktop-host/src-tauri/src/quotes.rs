//! پیش‌فاکتور فروش و سفارش خرید.
//!
//! مرجع: منوی «پیش‌فاکتورها» و «سفارش خرید».
//!
//! ## چرا این‌ها فاکتور نیستند
//!
//! پیش‌فاکتور و سفارش خرید **تعهد** هستند، نه **رویداد مالی**:
//!
//! - هیچ سند حسابداری نمی‌سازند (نه درآمدی محقق شده نه هزینه‌ای)
//! - موجودی انبار را تغییر نمی‌دهند (کالایی جابه‌جا نشده)
//! - در تراز و سود و زیان دیده نمی‌شوند
//!
//! اثر مالی فقط در لحظه‌ی **تبدیل به فاکتور** ایجاد می‌شود. اگر پیش‌فاکتور
//! سند بزند، درآمد تحقق‌نیافته در صورت‌های مالی ظاهر می‌شود — یکی از
//! رایج‌ترین اشتباهات نرم‌افزارهای حسابداری.
//!
//! ## چرخه‌ی وضعیت
//!
//! ```text
//! پیش‌نویس → ارسال‌شده → پذیرفته‌شده → تبدیل‌شده
//!                    ↘ ردشده
//!                    ↘ منقضی (پس از تاریخ اعتبار)
//! ```
//!
//! تبدیل فقط یک بار ممکن است؛ تبدیل دوباره یعنی یک سفارش دو بار فاکتور شود.

use novin_core::jalali::JalaliDate;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{active_context, audit, conn, require_permission, validate_fiscal_date, AppState};

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteLineInput {
    pub product_id: String,
    pub quantity: f64,
    pub unit_price: i64,
    #[serde(default)]
    pub discount: i64,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuoteInput {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: String,
    pub issue_date: String,
    #[serde(default)]
    pub valid_until: Option<String>,
    pub contact_id: String,
    #[serde(default)]
    pub warehouse_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// نرخ مالیات بر حسب صدم‌درصد (۹۰۰ یعنی ۹٪).
    #[serde(default)]
    pub vat_basis_points: i64,
    pub lines: Vec<QuoteLineInput>,
}

#[derive(Debug, Serialize)]
pub struct QuoteRow {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub number: i64,
    pub issue_date: String,
    pub valid_until: Option<String>,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub warehouse_name: Option<String>,
    pub description: Option<String>,
    pub subtotal: i64,
    pub discount: i64,
    pub tax: i64,
    pub total: i64,
    pub status: String,
    pub status_label: String,
    pub converted_invoice_id: Option<String>,
    pub line_count: i64,
    /// آیا تاریخ اعتبار گذشته است؟ محاسبه‌شده، نه ذخیره‌شده.
    pub is_expired: bool,
}

#[derive(Debug, Serialize)]
pub struct QuoteLineRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub unit: String,
    pub quantity: f64,
    pub unit_price: i64,
    pub discount: i64,
    pub tax: i64,
    pub line_total: i64,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuoteDetail {
    pub header: QuoteRow,
    pub lines: Vec<QuoteLineRow>,
}

fn parse_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "sales_quote" => Ok("sales_quote"),
        "purchase_order" => Ok("purchase_order"),
        _ => Err("QT-001: نوع سند باید پیش‌فاکتور یا سفارش خرید باشد".into()),
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "sales_quote" => "پیش‌فاکتور فروش",
        "purchase_order" => "سفارش خرید",
        _ => "نامشخص",
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "draft" => "پیش‌نویس",
        "sent" => "ارسال‌شده",
        "accepted" => "پذیرفته‌شده",
        "rejected" => "ردشده",
        "expired" => "منقضی",
        "converted" => "تبدیل به فاکتور",
        "cancelled" => "باطل‌شده",
        _ => "نامشخص",
    }
}

/// گذارهای مجاز وضعیت — همان چیزی که در تصویر منو هم دیده می‌شود.
fn allowed_next(status: &str) -> &'static [&'static str] {
    match status {
        "draft" => &["sent", "cancelled"],
        "sent" => &["accepted", "rejected", "expired", "cancelled"],
        "accepted" => &["converted", "cancelled"],
        "rejected" => &["draft"],
        "expired" => &["draft"],
        // تبدیل‌شده و باطل‌شده پایانی‌اند.
        _ => &[],
    }
}

/// امروز به شمسی، برای تشخیص انقضا.
fn today_jalali() -> JalaliDate {
    novin_core::jalali::from_gregorian(chrono::Utc::now().date_naive())
}

fn is_expired(valid_until: Option<&str>) -> bool {
    let Some(raw) = valid_until.map(str::trim).filter(|v| !v.is_empty()) else {
        return false;
    };
    let today = today_jalali();
    JalaliDate::parse(raw).map(|date| date < today).unwrap_or(false)
}

/// جمع‌های سند — یک منبع حقیقت برای پیش‌نمایش و ذخیره.
fn totals(lines: &[QuoteLineInput], vat_basis_points: i64) -> Result<(i64, i64, i64, i64), String> {
    if lines.is_empty() {
        return Err("QT-002: سند بدون قلم قابل ثبت نیست".into());
    }
    let mut subtotal = 0i64;
    let mut discount = 0i64;
    for (index, line) in lines.iter().enumerate() {
        let row = index + 1;
        if line.quantity <= 0.0 {
            return Err(format!("QT-003: مقدار سطر {row} باید بیشتر از صفر باشد"));
        }
        if line.unit_price < 0 {
            return Err(format!("QT-004: قیمت سطر {row} نمی‌تواند منفی باشد"));
        }
        let gross = (line.quantity * line.unit_price as f64).round() as i64;
        if line.discount < 0 || line.discount > gross {
            return Err(format!(
                "QT-005: تخفیف سطر {row} نمی‌تواند منفی یا بیشتر از مبلغ سطر باشد"
            ));
        }
        subtotal += gross;
        discount += line.discount;
    }
    if !(0..=10_000).contains(&vat_basis_points) {
        return Err("QT-006: نرخ مالیات نامعتبر است".into());
    }
    let net = subtotal - discount;
    // مالیات روی مبلغ پس از تخفیف محاسبه می‌شود، نه روی مبلغ ناخالص.
    let tax = net * vat_basis_points / 10_000;
    Ok((subtotal, discount, tax, net + tax))
}

/// پیش‌نمایش جمع‌های سند بدون نوشتن در پایگاه داده.
#[derive(Debug, Serialize)]
pub struct QuotePreview {
    pub subtotal: i64,
    pub discount: i64,
    pub net: i64,
    pub tax: i64,
    pub total: i64,
}

#[tauri::command]
pub fn preview_quote(
    lines: Vec<QuoteLineInput>,
    vat_basis_points: i64,
) -> Result<QuotePreview, String> {
    let (subtotal, discount, tax, total) = totals(&lines, vat_basis_points)?;
    Ok(QuotePreview {
        subtotal,
        discount,
        net: subtotal - discount,
        tax,
        total,
    })
}

fn permission_for(kind: &str) -> &'static str {
    if kind == "sales_quote" {
        "sales.invoice.create"
    } else {
        "purchase.invoice.create"
    }
}

/// ذخیره‌ی پیش‌فاکتور یا سفارش خرید.
#[tauri::command]
pub fn save_quote(state: State<AppState>, input: QuoteInput) -> Result<String, String> {
    let kind = parse_kind(&input.kind)?;
    let (subtotal, discount, tax, total) = totals(&input.lines, input.vat_basis_points)?;

    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, permission_for(kind))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &input.issue_date)?;

    // تاریخ اعتبار نباید قبل از تاریخ صدور باشد.
    if let Some(valid) = input.valid_until.as_deref().filter(|v| !v.trim().is_empty()) {
        let issued = JalaliDate::parse(&input.issue_date)
            .map_err(|_| "QT-007: تاریخ صدور شمسی معتبر نیست".to_string())?;
        let until =
            JalaliDate::parse(valid).map_err(|_| "QT-008: تاریخ اعتبار شمسی معتبر نیست".to_string())?;
        if until < issued {
            return Err("QT-009: تاریخ اعتبار نمی‌تواند قبل از تاریخ صدور باشد".into());
        }
    }

    let party_ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",
            params![input.contact_id, company],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if party_ok == 0 {
        return Err("QT-010: طرف حساب معتبر نیست".into());
    }

    let quote_id = match &input.id {
        Some(existing) => {
            let status: Option<String> = tx
                .query_row(
                    "SELECT status FROM quotes WHERE id=?1 AND company_id=?2",
                    params![existing, company],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            match status.as_deref() {
                None => return Err("QT-011: سند یافت نشد".into()),
                // سند تبدیل‌شده تاریخچه است؛ تغییرش یعنی تغییر فاکتور صادرشده.
                Some("converted") => {
                    return Err("QT-012: سند تبدیل‌شده به فاکتور قابل ویرایش نیست".into())
                }
                Some("cancelled") => return Err("QT-013: سند باطل‌شده قابل ویرایش نیست".into()),
                _ => {}
            }
            tx.execute(
                "UPDATE quotes SET issue_date=?1,valid_until=?2,contact_id=?3,warehouse_id=?4,\
                 description=?5,subtotal=?6,discount=?7,tax=?8,total=?9 WHERE id=?10",
                params![
                    input.issue_date,
                    input.valid_until,
                    input.contact_id,
                    input.warehouse_id,
                    input.description,
                    subtotal,
                    discount,
                    tax,
                    total,
                    existing
                ],
            )
            .map_err(|e| format!("QT-014: {e}"))?;
            tx.execute("DELETE FROM quote_lines WHERE quote_id=?1", params![existing])
                .map_err(|e| e.to_string())?;
            existing.clone()
        }
        None => {
            let number: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(number),0)+1 FROM quotes \
                     WHERE company_id=?1 AND fiscal_year_id=?2 AND kind=?3",
                    params![company, fy, kind],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            let new_id = format!(
                "quote-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date,\
                 valid_until,contact_id,warehouse_id,description,subtotal,discount,tax,total,\
                 status,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,'draft',?15)",
                params![
                    new_id,
                    company,
                    fy,
                    kind,
                    number,
                    input.issue_date,
                    input.valid_until,
                    input.contact_id,
                    input.warehouse_id,
                    input.description,
                    subtotal,
                    discount,
                    tax,
                    total,
                    user
                ],
            )
            .map_err(|e| format!("QT-015: {e}"))?;
            new_id
        }
    };

    for (index, line) in input.lines.iter().enumerate() {
        let gross = (line.quantity * line.unit_price as f64).round() as i64;
        let net = gross - line.discount;
        let line_tax = net * input.vat_basis_points / 10_000;
        tx.execute(
            "INSERT INTO quote_lines(id,quote_id,product_id,quantity,unit_price,discount,tax,\
             line_total,description) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                format!("{quote_id}-l{index}"),
                quote_id,
                line.product_id,
                line.quantity,
                line.unit_price,
                line.discount,
                line_tax,
                net + line_tax,
                line.description
            ],
        )
        .map_err(|e| format!("QT-016: {e}"))?;
    }

    audit(
        &tx,
        &user,
        "quote.save",
        kind,
        &quote_id,
        None,
        Some(&format!("{{\"total\":{total}}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(quote_id)
}

/// فهرست پیش‌فاکتورها یا سفارش‌های خرید.
#[tauri::command]
pub fn list_quotes(
    state: State<AppState>,
    kind: String,
    status: Option<String>,
) -> Result<Vec<QuoteRow>, String> {
    let kind = parse_kind(&kind)?;
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, permission_for(kind))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;

    let mut sql = String::from(
        "SELECT q.id,q.kind,q.number,q.issue_date,q.valid_until,q.contact_id,c.name,w.name,\
         q.description,q.subtotal,q.discount,q.tax,q.total,q.status,q.converted_invoice_id,\
         (SELECT COUNT(*) FROM quote_lines l WHERE l.quote_id=q.id) \
         FROM quotes q LEFT JOIN contacts c ON c.id=q.contact_id \
         LEFT JOIN warehouses w ON w.id=q.warehouse_id \
         WHERE q.company_id=?1 AND q.fiscal_year_id=?2 AND q.kind=?3",
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(company),
        Box::new(fy),
        Box::new(kind.to_string()),
    ];
    if let Some(value) = status.filter(|v| !v.trim().is_empty()) {
        values.push(Box::new(value));
        sql.push_str(&format!(" AND q.status=?{}", values.len()));
    }
    sql.push_str(" ORDER BY q.number DESC LIMIT 500");

    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let bound: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let rows = statement
        .query_map(bound.as_slice(), |row| {
            let kind: String = row.get(1)?;
            let status: String = row.get(13)?;
            let valid_until: Option<String> = row.get(4)?;
            // انقضا محاسبه می‌شود، نه ذخیره — چون با گذشت زمان تغییر می‌کند.
            let expired = status != "converted"
                && status != "cancelled"
                && is_expired(valid_until.as_deref());
            Ok(QuoteRow {
                id: row.get(0)?,
                kind_label: kind_label(&kind).to_string(),
                kind,
                number: row.get(2)?,
                issue_date: row.get(3)?,
                valid_until,
                contact_id: row.get(5)?,
                contact_name: row.get(6)?,
                warehouse_name: row.get(7)?,
                description: row.get(8)?,
                subtotal: row.get(9)?,
                discount: row.get(10)?,
                tax: row.get(11)?,
                total: row.get(12)?,
                status_label: status_label(&status).to_string(),
                status,
                converted_invoice_id: row.get(14)?,
                line_count: row.get(15)?,
                is_expired: expired,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// جزئیات یک پیش‌فاکتور یا سفارش خرید.
#[tauri::command]
pub fn get_quote(state: State<AppState>, id: String) -> Result<QuoteDetail, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "sales.invoice.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let header = tx
        .query_row(
            "SELECT q.id,q.kind,q.number,q.issue_date,q.valid_until,q.contact_id,c.name,w.name,\
             q.description,q.subtotal,q.discount,q.tax,q.total,q.status,q.converted_invoice_id,\
             (SELECT COUNT(*) FROM quote_lines l WHERE l.quote_id=q.id) \
             FROM quotes q LEFT JOIN contacts c ON c.id=q.contact_id \
             LEFT JOIN warehouses w ON w.id=q.warehouse_id \
             WHERE q.id=?1 AND q.company_id=?2",
            params![id, company],
            |row| {
                let kind: String = row.get(1)?;
                let status: String = row.get(13)?;
                let valid_until: Option<String> = row.get(4)?;
                let expired = status != "converted"
                    && status != "cancelled"
                    && is_expired(valid_until.as_deref());
                Ok(QuoteRow {
                    id: row.get(0)?,
                    kind_label: kind_label(&kind).to_string(),
                    kind,
                    number: row.get(2)?,
                    issue_date: row.get(3)?,
                    valid_until,
                    contact_id: row.get(5)?,
                    contact_name: row.get(6)?,
                    warehouse_name: row.get(7)?,
                    description: row.get(8)?,
                    subtotal: row.get(9)?,
                    discount: row.get(10)?,
                    tax: row.get(11)?,
                    total: row.get(12)?,
                    status_label: status_label(&status).to_string(),
                    status,
                    converted_invoice_id: row.get(14)?,
                    line_count: row.get(15)?,
                    is_expired: expired,
                })
            },
        )
        .map_err(|_| "QT-011: سند یافت نشد".to_string())?;

    let mut statement = tx
        .prepare(
            "SELECT l.id,l.product_id,p.name,p.unit,l.quantity,l.unit_price,l.discount,l.tax,\
             l.line_total,l.description FROM quote_lines l JOIN products p ON p.id=l.product_id \
             WHERE l.quote_id=?1 ORDER BY l.id",
        )
        .map_err(|e| e.to_string())?;
    let lines = statement
        .query_map(params![id], |row| {
            Ok(QuoteLineRow {
                id: row.get(0)?,
                product_id: row.get(1)?,
                product_name: row.get(2)?,
                unit: row.get(3)?,
                quantity: row.get(4)?,
                unit_price: row.get(5)?,
                discount: row.get(6)?,
                tax: row.get(7)?,
                line_total: row.get(8)?,
                description: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(QuoteDetail { header, lines })
}

/// گذارهای مجاز وضعیت برای یک سند — رابط کاربری از خودش فهرست نمی‌سازد.
#[derive(Debug, Serialize)]
pub struct QuoteTransition {
    pub status: String,
    pub label: String,
}

#[tauri::command]
pub fn quote_transitions(
    state: State<AppState>,
    id: String,
) -> Result<Vec<QuoteTransition>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "sales.invoice.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;
    let status: String = tx
        .query_row(
            "SELECT status FROM quotes WHERE id=?1 AND company_id=?2",
            params![id, company],
            |row| row.get(0),
        )
        .map_err(|_| "QT-011: سند یافت نشد".to_string())?;
    Ok(allowed_next(&status)
        .iter()
        .map(|value| QuoteTransition {
            status: (*value).to_string(),
            label: status_label(value).to_string(),
        })
        .collect())
}

/// تغییر وضعیت پیش‌فاکتور.
///
/// «تبدیل به فاکتور» از این مسیر انجام نمی‌شود؛ برای آن `convert_quote` هست
/// که فاکتور واقعی می‌سازد.
#[tauri::command]
pub fn set_quote_status(
    state: State<AppState>,
    id: String,
    status: String,
) -> Result<(), String> {
    if status == "converted" {
        return Err("QT-017: تبدیل به فاکتور باید از دکمه‌ی تبدیل انجام شود".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "sales.invoice.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let current: String = tx
        .query_row(
            "SELECT status FROM quotes WHERE id=?1 AND company_id=?2",
            params![id, company],
            |row| row.get(0),
        )
        .map_err(|_| "QT-011: سند یافت نشد".to_string())?;
    if !allowed_next(&current).contains(&status.as_str()) {
        return Err(format!(
            "QT-018: تغییر وضعیت از «{}» به «{}» مجاز نیست",
            status_label(&current),
            status_label(&status)
        ));
    }
    tx.execute(
        "UPDATE quotes SET status=?1 WHERE id=?2",
        params![status, id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "quote.status",
        "quote",
        &id,
        Some(&format!("{{\"status\":\"{current}\"}}")),
        Some(&format!("{{\"status\":\"{status}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// تبدیل پیش‌فاکتور پذیرفته‌شده به فاکتور واقعی.
///
/// اینجاست که اثر مالی متولد می‌شود: فاکتور پیش‌نویس ساخته می‌شود تا کاربر
/// آن را بررسی و ثبت قطعی کند. فاکتور مستقیماً «ثبت‌شده» نمی‌شود چون ممکن
/// است بین پیشنهاد و فروش، قیمت یا موجودی تغییر کرده باشد.
#[tauri::command]
pub fn convert_quote(
    state: State<AppState>,
    id: String,
    invoice_date: String,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "sales.invoice.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &invoice_date)?;

    let (kind, status, contact, warehouse, subtotal, discount, tax, total, converted): (
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
        i64,
        Option<String>,
    ) = tx
        .query_row(
            "SELECT kind,status,contact_id,warehouse_id,subtotal,discount,tax,total,\
             converted_invoice_id FROM quotes WHERE id=?1 AND company_id=?2",
            params![id, company],
            |row| {
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
                ))
            },
        )
        .map_err(|_| "QT-011: سند یافت نشد".to_string())?;

    if converted.is_some() || status == "converted" {
        return Err("QT-019: این سند قبلاً به فاکتور تبدیل شده است".into());
    }
    if status != "accepted" {
        return Err("QT-020: فقط سند پذیرفته‌شده به فاکتور تبدیل می‌شود".into());
    }

    let sales = kind == "sales_quote";
    let invoice_table = if sales {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let line_table = if sales {
        "sales_invoice_lines"
    } else {
        "purchase_invoice_lines"
    };

    let number: i64 = tx
        .query_row(
            &format!(
                "SELECT COALESCE(MAX(number),0)+1 FROM {invoice_table} \
                 WHERE company_id=?1 AND fiscal_year_id=?2"
            ),
            params![company, fy],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let invoice_id = format!(
        "invoice-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute(
        &format!(
            "INSERT INTO {invoice_table}(id,company_id,fiscal_year_id,number,invoice_date,\
             contact_id,warehouse_id,status,payment_status,subtotal,discount,tax,total,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,'draft','unpaid',?8,?9,?10,?11,?12)"
        ),
        params![
            invoice_id,
            company,
            fy,
            number,
            invoice_date,
            contact,
            warehouse,
            subtotal,
            discount,
            tax,
            total,
            user
        ],
    )
    .map_err(|e| format!("QT-021: {e}"))?;

    let mut statement = tx
        .prepare("SELECT product_id,quantity,unit_price,discount,tax,line_total FROM quote_lines WHERE quote_id=?1 ORDER BY id")
        .map_err(|e| e.to_string())?;
    let lines: Vec<(String, f64, i64, i64, i64, i64)> = statement
        .query_map(params![id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(statement);
    if lines.is_empty() {
        return Err("QT-002: سند بدون قلم قابل تبدیل نیست".into());
    }

    for (index, (product, quantity, price, line_discount, line_tax, line_total)) in
        lines.iter().enumerate()
    {
        tx.execute(
            &format!(
                "INSERT INTO {line_table}(id,invoice_id,product_id,quantity,unit_price,discount,\
                 tax,line_total) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)"
            ),
            params![
                format!("{invoice_id}-l{index}"),
                invoice_id,
                product,
                quantity,
                price,
                line_discount,
                line_tax,
                line_total
            ],
        )
        .map_err(|e| format!("QT-022: {e}"))?;
    }

    tx.execute(
        "UPDATE quotes SET status='converted',converted_invoice_id=?1 WHERE id=?2",
        params![invoice_id, id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "quote.convert",
        "quote",
        &id,
        None,
        Some(&format!("{{\"invoice\":\"{invoice_id}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(invoice_id)
}
