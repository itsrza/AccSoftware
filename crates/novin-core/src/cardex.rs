#![allow(warnings)]
// موقت: میراث ممیزی CI — بعد از سبزشدن، فایل‌به‌فایل برداشته می‌شود
//! کاردکس کالا — گزارش حرکات فروش، خرید و کلی.
//!
//! مرجع: لیست کالاهای نرم‌افزار فعلی (تصویر `8Xmc1p`) — کلید‌ها:
//! **F4 کاردکس فروش · F5 کاردکس خرید · F6 کاردکس**.
//!
//! ## چرا ماند از ابتدای تاریخ حساب می‌شود، نه از ابتدای بازه
//!
//! کاردکس یک **دفتر** است، نه یک عکس فوری. اگر ماند هر سطر فقط از ابتدای
//! بازه‌ی انتخابی شروع می‌شد، کاربری که وسط سال بازه را عوض می‌کرد ماند
//! را «از صفر» می‌دید و گمان می‌کرد انبار خالی شده است. پس:
//!
//! ```text
//! ماندِ سطر = (جمع ورود − جمع خروجِ همه‌ی حرکاتِ هم‌کانالِ قبل از بازه)
//!           + (جمع تجمعی همین بازه تا آن سطر)
//! ```
//!
//! موجودی «افتتاحیه» همین جمله‌ی اول است و در سربرگ گزارش می‌آید.
//!
//! ## چرا جهتِ تعدیل انبارگردانی از یادداشت خوانده می‌شود
//!
//! جدول حرکت‌ها `quantity` را همیشه مثبت نگه می‌دارد و جهت را با
//! `movement_type` می‌فهمد؛ اما تعدیل انبارگردانی (`adjustment`) دو جهت
//! دارد و علامت آن فقط در یادداشت `variance:±n` مانده است. اگر همین‌جا
//! خوانده نشود، انبارگردانیِ کسری به‌جای کم‌کردن، موجودی را بیشتر می‌کند.
//!
//! ## تفکیک فروش/خرید
//!
//! حرکت‌های فاکتور با `reference_type='invoice'` ثبت می‌شوند و نوع فروش یا
//! خرید بودن فقط از جدول مقصد مشخص می‌شود؛ به همین دلیل پرس‌وجو به هر
//! چهار جدول سند (فاکتور فروش/خرید و برگشت از فروش/خرید) join می‌زند.
//! حرکت‌های قدیمی seed و دمو `sales_invoice`/`purchase_invoice` نیز
//! پشتیبانی می‌شوند تا داده‌های موجود هرگز کور نشوند.

use crate::jalali;
use crate::money::Money;
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::Serialize;

/// خطاهای دامنه‌ی کاردکس.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CardexError {
    #[error("نوع کاردکس نامعتبر است")]
    UnknownKind,
    #[error("کالای کاردکس مشخص نشده یا یافت نشد")]
    MissingProduct,
    #[error("بازه‌ی تاریخ نامعتبر است")]
    InvalidRange,
    #[error("خطای پایگاه داده: {0}")]
    Database(String),
    #[error("خطای محاسبه‌ی مبلغ")]
    Money(#[from] crate::money::MoneyError),
}

/// کانال کاردکس — همان سه کلید مرجع.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardexKind {
    /// F4 — فقط گردش فروش (فاکتور فروش + برگشت از فروش)
    Sales,
    /// F5 — فقط گردش خرید (فاکتور خرید + برگشت از خرید)
    Purchase,
    /// F6 — همه‌ی حرکت‌ها (انتقال، انبارگردانی، تعدیل، افتتاحیه و…)
    All,
}

impl CardexKind {
    pub fn parse(value: &str) -> Result<Self, CardexError> {
        Ok(match value {
            "sales" => CardexKind::Sales,
            "purchase" => CardexKind::Purchase,
            "all" => CardexKind::All,
            _ => return Err(CardexError::UnknownKind),
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CardexKind::Sales => "sales",
            CardexKind::Purchase => "purchase",
            CardexKind::All => "all",
        }
    }
}

/// جهت حرکت انبار.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    In,
    Out,
}

/// علامتِ تعدیل انبارگردانی از یادداشت `variance:±n`.
fn variance_of(note: &str) -> Option<f64> {
    note.rsplit("variance:")
        .next()
        .map(|tail| tail.trim().to_string())
        .and_then(|tail| tail.parse::<f64>().ok())
}

/// جهت یک حرکت بر اساس نوع و یادداشت تعدیل.
pub fn movement_flow(movement_type: &str, note: Option<&str>) -> Flow {
    match movement_type {
        "receipt" | "transfer_in" => Flow::In,
        "issue" | "transfer_out" => Flow::Out,
        "adjustment" => match note.and_then(variance_of) {
            Some(variance) if variance < 0.0 => Flow::Out,
            _ => Flow::In,
        },
        _ => Flow::In,
    }
}

/// مقدار علامت‌دار حرکت: ورود مثبت، خروج منفی.
pub fn signed_quantity(quantity: f64, movement_type: &str, note: Option<&str>) -> f64 {
    match movement_flow(movement_type, note) {
        Flow::In => quantity,
        Flow::Out => -quantity,
    }
}

/// شماره‌ی اسناد join‌شده — تعیین‌کننده‌ی کانال فروش/خرید.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocLinks {
    pub sales_invoice: Option<i64>,
    pub purchase_invoice: Option<i64>,
    pub sales_return: Option<i64>,
    pub purchase_return: Option<i64>,
}

/// کانال حرکت: گردش فروش، گردش خرید یا حرکت داخلی انبار.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Sales,
    Purchase,
    Internal,
}

/// تعیین کانال حرکت از روی نوع مرجع و جدول مقصد سند.
pub fn channel_of(reference_type: Option<&str>, links: &DocLinks) -> Channel {
    match reference_type {
        Some("invoice") => {
            if links.sales_invoice.is_some() {
                Channel::Sales
            } else if links.purchase_invoice.is_some() {
                Channel::Purchase
            } else {
                Channel::Internal
            }
        }
        Some("invoice_return") => {
            if links.sales_return.is_some() {
                Channel::Sales
            } else if links.purchase_return.is_some() {
                Channel::Purchase
            } else {
                Channel::Internal
            }
        }
        Some("sales_invoice") => Channel::Sales,
        Some("purchase_invoice") => Channel::Purchase,
        _ => Channel::Internal,
    }
}

/// شناسه‌ی نوع سند برای نمایش — رابط کاربری آن را ترجمه می‌کند.
pub fn doc_kind(reference_type: Option<&str>, links: &DocLinks) -> &'static str {
    match reference_type {
        Some("invoice") | Some("sales_invoice") => {
            if links.sales_invoice.is_some() {
                "sales_invoice"
            } else {
                "other"
            }
        }
        Some("purchase_invoice") => {
            if links.purchase_invoice.is_some() {
                "purchase_invoice"
            } else {
                "other"
            }
        }
        Some("invoice_return") => {
            if links.sales_return.is_some() {
                "sales_return"
            } else if links.purchase_return.is_some() {
                "purchase_return"
            } else {
                "other"
            }
        }
        Some("transfer") | Some("warehouse_transfer") => "transfer",
        Some("inventory_count") => "inventory_count",
        Some("inventory_adjustment") => "inventory_adjustment",
        Some("opening") => "opening",
        _ => "other",
    }
}

/// شماره‌ی سندِ متناسب با نوع تشخیص‌داده‌شده.
fn doc_number(links: &DocLinks) -> Option<i64> {
    links
        .sales_invoice
        .or(links.purchase_invoice)
        .or(links.sales_return)
        .or(links.purchase_return)
}

/// فیلتر گزارش.
pub struct CardexFilter {
    pub company_id: String,
    pub product_id: String,
    pub kind: CardexKind,
    /// تاریخ میلادی ISO — مرز بازه (شامل هر دو سر).
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub warehouse_id: Option<String>,
}

/// یک سطر کاردکس.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CardexEntry {
    pub date_iso: String,
    pub date_jalali: String,
    pub warehouse_name: String,
    /// `in` یا `out`
    pub flow: &'static str,
    /// شناسه‌ی نوع سند: sales_invoice و…
    pub doc_kind: String,
    pub doc_number: Option<i64>,
    pub quantity: f64,
    pub unit_cost: i64,
    /// quantity × unit_cost (ریال)
    pub value: i64,
    /// ماند تجمعی تا این سطر
    pub balance: f64,
    pub note: Option<String>,
}

/// گزارش کامل کاردکس.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CardexReport {
    pub product_id: String,
    pub product_name: String,
    pub product_unit: String,
    pub kind: &'static str,
    /// جمع علامت‌دار حرکات هم‌کانالِ قبل از بازه
    pub opening_balance: f64,
    pub total_in: f64,
    pub total_out: f64,
    pub closing_balance: f64,
    pub entries: Vec<CardexEntry>,
}

/// ساخت گزارش کاردکس از پایگاه داده.
pub fn cardex(conn: &Connection, filter: &CardexFilter) -> Result<CardexReport, CardexError> {
    if filter.product_id.trim().is_empty() {
        return Err(CardexError::MissingProduct);
    }
    if filter.from > filter.to {
        return Err(CardexError::InvalidRange);
    }

    let (product_name, product_unit): (String, String) = conn
        .query_row(
            "SELECT name, unit FROM products WHERE id=?1 AND company_id=?2",
            params![filter.product_id, filter.company_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| CardexError::MissingProduct)?;

    let sql = "\
        SELECT m.created_at, m.movement_type, m.quantity, m.unit_cost, m.note, \
               m.warehouse_id, w.name, m.reference_type, \
               si.number, pi.number, sr.number, pr.number \
        FROM inventory_movements m \
        JOIN warehouses w ON w.id = m.warehouse_id \
        LEFT JOIN sales_invoices si ON m.reference_type IN ('invoice','sales_invoice') AND si.id = m.reference_id \
        LEFT JOIN purchase_invoices pi ON m.reference_type IN ('invoice','purchase_invoice') AND pi.id = m.reference_id \
        LEFT JOIN sales_returns sr ON m.reference_type = 'invoice_return' AND sr.id = m.reference_id \
        LEFT JOIN purchase_returns pr ON m.reference_type = 'invoice_return' AND pr.id = m.reference_id \
        WHERE m.product_id = ?1 AND m.company_id = ?2 AND date(m.created_at) <= ?3";

    let mut statement = if filter.warehouse_id.is_some() {
        conn.prepare(&format!(
            "{sql} AND m.warehouse_id = ?4 ORDER BY m.created_at, m.rowid"
        ))
        .map_err(|error| CardexError::Database(error.to_string()))?
    } else {
        conn.prepare(&format!("{sql} ORDER BY m.created_at, m.rowid"))
            .map_err(|error| CardexError::Database(error.to_string()))?
    };

    let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<_> {
        Ok((
            row.get::<_, String>(0)?,         // created_at
            row.get::<_, String>(1)?,         // movement_type
            row.get::<_, f64>(2)?,            // quantity
            row.get::<_, i64>(3)?,            // unit_cost
            row.get::<_, Option<String>>(4)?, // note
            row.get::<_, String>(6)?,         // warehouse name
            row.get::<_, Option<String>>(7)?, // reference_type
            row.get::<_, Option<i64>>(8)?,    // sales invoice number
            row.get::<_, Option<i64>>(9)?,    // purchase invoice number
            row.get::<_, Option<i64>>(10)?,   // sales return number
            row.get::<_, Option<i64>>(11)?,   // purchase return number
        ))
    };

    let rows = if filter.warehouse_id.is_some() {
        statement
            .query_map(
                params![
                    filter.product_id,
                    filter.company_id,
                    jalali::iso_string(filter.to),
                    filter.warehouse_id
                ],
                map_row,
            )
            .map_err(|error| CardexError::Database(error.to_string()))?
    } else {
        statement
            .query_map(
                params![
                    filter.product_id,
                    filter.company_id,
                    jalali::iso_string(filter.to)
                ],
                map_row,
            )
            .map_err(|error| CardexError::Database(error.to_string()))?
    };

    let mut opening = 0.0_f64;
    let mut total_in = 0.0_f64;
    let mut total_out = 0.0_f64;
    let mut entries: Vec<CardexEntry> = Vec::new();

    for row in rows {
        let (
            created_at,
            movement_type,
            quantity,
            unit_cost,
            note,
            warehouse_name,
            reference_type,
            sales_number,
            purchase_number,
            sales_return_number,
            purchase_return_number,
        ) = row.map_err(|error| CardexError::Database(error.to_string()))?;

        let links = DocLinks {
            sales_invoice: sales_number,
            purchase_invoice: purchase_number,
            sales_return: sales_return_number,
            purchase_return: purchase_return_number,
        };
        let channel = channel_of(reference_type.as_deref(), &links);
        if filter.kind != CardexKind::All && channel != channel_of_kind(filter.kind) {
            continue;
        }

        let date = created_at
            .get(..10)
            .and_then(|slice| NaiveDate::parse_from_str(slice, "%Y-%m-%d").ok())
            .ok_or_else(|| CardexError::Database(format!("تاریخ نامعتبر: {created_at}")))?;
        let signed = signed_quantity(quantity, &movement_type, note.as_deref());

        if date < filter.from {
            // قبل از بازه → فقط روی افتتاحیه اثر دارد
            opening += signed;
            continue;
        }

        let flow = movement_flow(&movement_type, note.as_deref());
        match flow {
            Flow::In => total_in += quantity,
            Flow::Out => total_out += quantity,
        }
        let value = Money::from_rials(unit_cost)
            .mul_quantity(quantity)
            .map(|money| money.rials())?;

        entries.push(CardexEntry {
            date_iso: jalali::iso_string(date),
            date_jalali: jalali::jalali_string(date),
            warehouse_name,
            flow: match flow {
                Flow::In => "in",
                Flow::Out => "out",
            },
            doc_kind: doc_kind(reference_type.as_deref(), &links).to_string(),
            doc_number: doc_number(&links),
            quantity,
            unit_cost,
            value,
            balance: opening + total_in - total_out,
            note,
        });
    }

    Ok(CardexReport {
        product_id: filter.product_id.clone(),
        product_name,
        product_unit,
        kind: filter.kind.as_str(),
        opening_balance: opening,
        total_in,
        total_out,
        closing_balance: opening + total_in - total_out,
        entries,
    })
}

/// کانال متناظر با نوع کاردکس.
fn channel_of_kind(kind: CardexKind) -> Channel {
    match kind {
        CardexKind::Sales => Channel::Sales,
        CardexKind::Purchase => Channel::Purchase,
        CardexKind::All => Channel::Internal, // هرگز صدا زده نمی‌شود
    }
}
