//! برگشت از فروش و برگشت از خرید.
//!
//! مرجع: تصویر `FRPBDr` (برگشت از فروش).
//!
//! ## دو باگ حسابداری که این ماژول می‌بندد
//!
//! ۱. **مالیات برگشت داده نمی‌شد.** سند برگشت فقط مبلغ خالص را معکوس می‌کرد،
//!    پس حساب «مالیات بر ارزش افزوده» متورم می‌ماند و اظهارنامه اشتباه
//!    درمی‌آمد. حالا مالیات به نسبت مبلغ برگشتی معکوس می‌شود.
//! ۲. **تاریخ سند میلادیِ امروز بود.** سند برگشت با تاریخ روز ثبت می‌شد، نه
//!    تاریخ برگشت — پس ممکن بود در سال مالی دیگری بیفتد. حالا تاریخ برگشت
//!    استفاده و با سال مالی اعتبارسنجی می‌شود.
//!
//! ## سند برگشت از فروش
//!
//! ```text
//! بدهکار  برگشت از فروش (کاهنده‌ی درآمد)   مبلغ خالص
//! بدهکار  مالیات بر ارزش افزوده             مالیات متناسب
//! بستانکار حساب مشتریان                     جمع کل
//! ```
//!
//! برگشت از خرید دقیقاً معکوس، با «برگشت از خرید» به‌عنوان کاهنده‌ی بهای
//! تمام‌شده.
//!
//! ## قاعده‌ی مقدار
//!
//! مجموع برگشت‌های یک قلم هرگز نباید از مقدار فاکتور اصلی بیشتر شود؛ وگرنه
//! کالایی برگشت می‌خورد که اصلاً فروخته نشده است.

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::State;

use crate::{active_context, audit, conn, next_journal_number, require_permission, AppState};

/// یک قلم قابل برگشت از فاکتور اصلی.
#[derive(Debug, Serialize)]
pub struct ReturnableLine {
    pub product_id: String,
    pub product_name: String,
    pub unit: String,
    pub invoiced_quantity: f64,
    /// مقداری که تا الان برگشت خورده (شامل پیش‌نویس‌ها).
    pub returned_quantity: f64,
    /// باقیمانده‌ی قابل برگشت.
    pub returnable_quantity: f64,
    pub unit_price: i64,
}

#[derive(Debug, Serialize)]
pub struct ReturnRow {
    pub id: String,
    pub number: i64,
    pub return_date: String,
    pub original_invoice_id: String,
    pub original_invoice_number: Option<i64>,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub warehouse_name: Option<String>,
    pub status: String,
    pub status_label: String,
    pub total: i64,
    pub tax: i64,
    pub grand_total: i64,
    pub journal_id: Option<String>,
    pub line_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ReturnLineRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub quantity: f64,
    pub unit_price: i64,
    pub line_total: i64,
}

#[derive(Debug, Serialize)]
pub struct ReturnDetail {
    pub header: ReturnRow,
    pub lines: Vec<ReturnLineRow>,
}

struct Tables {
    invoices: &'static str,
    invoice_lines: &'static str,
    returns: &'static str,
    return_lines: &'static str,
}

fn tables(sale: bool) -> Tables {
    if sale {
        Tables {
            invoices: "sales_invoices",
            invoice_lines: "sales_invoice_lines",
            returns: "sales_returns",
            return_lines: "sales_return_lines",
        }
    } else {
        Tables {
            invoices: "purchase_invoices",
            invoice_lines: "purchase_invoice_lines",
            returns: "purchase_returns",
            return_lines: "purchase_return_lines",
        }
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "draft" => "پیش‌نویس",
        "posted" => "ثبت‌شده",
        "cancelled" => "باطل‌شده",
        _ => "نامشخص",
    }
}

/// نسبت مالیات فاکتور اصلی — برای معکوس‌کردن متناسب مالیات در سند برگشت.
///
/// از خود فاکتور خوانده می‌شود، نه از تنظیمات فعلی؛ چون نرخ مالیات ممکن است
/// از زمان صدور فاکتور تغییر کرده باشد و برگشت باید همان نرخ روز فروش را
/// برگرداند.
fn invoice_tax_ratio(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    invoice_id: &str,
) -> Result<(i64, i64), String> {
    let sql = format!("SELECT subtotal, tax FROM {table} WHERE id=?1");
    tx.query_row(&sql, params![invoice_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })
    .map_err(|_| "RET-001: فاکتور اصلی یافت نشد".to_string())
}

/// اقلام قابل برگشت یک فاکتور، با کسر برگشت‌های قبلی.
#[tauri::command]
pub fn list_returnable_lines(
    state: State<AppState>,
    sale: bool,
    invoice_id: String,
) -> Result<Vec<ReturnableLine>, String> {
    let t = tables(sale);
    let mut c = conn(&state)?;
    let permission = if sale {
        "sales.return.create"
    } else {
        "purchase.return.create"
    };
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let owned: i64 = tx
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE id=?1 AND company_id=?2",
                t.invoices
            ),
            params![invoice_id, company],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if owned == 0 {
        return Err("RET-001: فاکتور اصلی یافت نشد".into());
    }

    let sql = format!(
        "SELECT l.product_id, p.name, p.unit, SUM(l.quantity), MAX(l.unit_price), \
         COALESCE((SELECT SUM(rl.quantity) FROM {rl} rl JOIN {rt} r ON r.id=rl.return_id \
                   WHERE r.original_invoice_id=?1 AND r.status<>'cancelled' \
                   AND rl.product_id=l.product_id),0) \
         FROM {il} l JOIN products p ON p.id=l.product_id \
         WHERE l.invoice_id=?1 GROUP BY l.product_id ORDER BY p.name",
        rl = t.return_lines,
        rt = t.returns,
        il = t.invoice_lines
    );
    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![invoice_id], |row| {
            let invoiced: f64 = row.get(3)?;
            let returned: f64 = row.get(5)?;
            Ok(ReturnableLine {
                product_id: row.get(0)?,
                product_name: row.get(1)?,
                unit: row.get(2)?,
                invoiced_quantity: invoiced,
                returned_quantity: returned,
                // منفی نمی‌شود حتی اگر داده‌ی قدیمی ناسازگار باشد.
                returnable_quantity: (invoiced - returned).max(0.0),
                unit_price: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// فهرست برگشت‌ها با فیلتر وضعیت.
#[tauri::command]
pub fn list_returns(
    state: State<AppState>,
    sale: bool,
    status: Option<String>,
) -> Result<Vec<ReturnRow>, String> {
    let t = tables(sale);
    let mut c = conn(&state)?;
    let permission = if sale {
        "sales.return.create"
    } else {
        "purchase.return.create"
    };
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;

    let mut sql = format!(
        "SELECT r.id,r.number,r.return_date,r.original_invoice_id,i.number,r.contact_id,c.name,\
         w.name,r.status,r.total,r.journal_id,\
         (SELECT COUNT(*) FROM {rl} l WHERE l.return_id=r.id),\
         COALESCE(i.subtotal,0),COALESCE(i.tax,0) \
         FROM {rt} r \
         LEFT JOIN {inv} i ON i.id=r.original_invoice_id \
         LEFT JOIN contacts c ON c.id=r.contact_id \
         LEFT JOIN warehouses w ON w.id=r.warehouse_id \
         WHERE r.company_id=?1 AND r.fiscal_year_id=?2",
        rl = t.return_lines,
        rt = t.returns,
        inv = t.invoices
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(company), Box::new(fy)];
    if let Some(value) = status.filter(|v| !v.trim().is_empty()) {
        values.push(Box::new(value));
        sql.push_str(&format!(" AND r.status=?{}", values.len()));
    }
    sql.push_str(" ORDER BY r.number DESC LIMIT 500");

    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let bound: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let rows = statement
        .query_map(bound.as_slice(), |row| {
            let status: String = row.get(8)?;
            let total: i64 = row.get(9)?;
            let subtotal: i64 = row.get(12)?;
            let invoice_tax: i64 = row.get(13)?;
            // مالیات برگشت به نسبت مبلغ برگشتی از مالیات فاکتور اصلی.
            let tax = if subtotal > 0 {
                (total as i128 * invoice_tax as i128 / subtotal as i128) as i64
            } else {
                0
            };
            Ok(ReturnRow {
                id: row.get(0)?,
                number: row.get(1)?,
                return_date: row.get(2)?,
                original_invoice_id: row.get(3)?,
                original_invoice_number: row.get(4)?,
                contact_id: row.get(5)?,
                contact_name: row.get(6)?,
                warehouse_name: row.get(7)?,
                status_label: status_label(&status).to_string(),
                status,
                total,
                tax,
                grand_total: total + tax,
                journal_id: row.get(10)?,
                line_count: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// جزئیات یک برگشت.
#[tauri::command]
pub fn get_return(state: State<AppState>, sale: bool, id: String) -> Result<ReturnDetail, String> {
    let t = tables(sale);
    let all = list_returns(state.clone(), sale, None)?;
    let header = all
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| "RET-006: برگشت یافت نشد".to_string())?;

    let mut c = conn(&state)?;
    let permission = if sale {
        "sales.return.create"
    } else {
        "purchase.return.create"
    };
    require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let mut statement = tx
        .prepare(&format!(
            "SELECT l.id,l.product_id,p.name,l.quantity,l.unit_price,l.line_total \
             FROM {rl} l JOIN products p ON p.id=l.product_id WHERE l.return_id=?1 ORDER BY p.name",
            rl = t.return_lines
        ))
        .map_err(|e| e.to_string())?;
    let lines = statement
        .query_map(params![id], |row| {
            Ok(ReturnLineRow {
                id: row.get(0)?,
                product_id: row.get(1)?,
                product_name: row.get(2)?,
                quantity: row.get(3)?,
                unit_price: row.get(4)?,
                line_total: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(ReturnDetail { header, lines })
}

/// ثبت قطعی برگشت: بروزرسانی موجودی، صدور سند حسابداری با مالیات معکوس.
///
/// این تابع جایگزین نسخه‌ی قبلی است که مالیات را برنمی‌گرداند و تاریخ سند را
/// از ساعت سیستم می‌گرفت.
fn post(state: &State<AppState>, sale: bool, return_id: &str) -> Result<(), String> {
    let t = tables(sale);
    let permission = if sale {
        "sales.return.post"
    } else {
        "purchase.return.post"
    };
    let mut c = conn(state)?;
    let user = require_permission(state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;

    let (company, fy, status, warehouse, total, return_date, invoice_id): (
        String,
        String,
        String,
        Option<String>,
        i64,
        String,
        String,
    ) = tx
        .query_row(
            &format!(
                "SELECT company_id,fiscal_year_id,status,warehouse_id,total,return_date,\
                 original_invoice_id FROM {rt} WHERE id=?1",
                rt = t.returns
            ),
            params![return_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|_| "RET-006: برگشت یافت نشد".to_string())?;

    if status != "draft" {
        return Err("RET-007: فقط برگشت پیش‌نویس قابل ثبت است".into());
    }
    let warehouse = warehouse
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "RET-008: انبار برگشت مشخص نیست".to_string())?;

    // تاریخ سند = تاریخ برگشت، و باید داخل سال مالی باز باشد.
    crate::validate_fiscal_date(&tx, &fy, &return_date)?;

    let mut statement = tx
        .prepare(&format!(
            "SELECT product_id,quantity,unit_price FROM {rl} WHERE return_id=?1",
            rl = t.return_lines
        ))
        .map_err(|e| e.to_string())?;
    let items: Vec<(String, f64, i64)> = statement
        .query_map(params![return_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(statement);
    if items.is_empty() {
        return Err("RET-010: برگشت بدون قلم قابل ثبت نیست".into());
    }

    for (product, quantity, price) in &items {
        let current: f64 = tx
            .query_row(
                "SELECT COALESCE(quantity,0) FROM inventory_balances \
                 WHERE product_id=?1 AND warehouse_id=?2",
                params![product, warehouse],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        // برگشت از خرید کالا را از انبار خارج می‌کند؛ اگر موجودی نباشد یعنی
        // کالا قبلاً فروخته شده و برگشت به فروشنده ممکن نیست.
        if !sale && current < *quantity {
            return Err(format!(
                "RET-009: موجودی «{product}» برای برگشت خرید کافی نیست (موجودی {current})"
            ));
        }
        let updated = if sale {
            current + *quantity
        } else {
            current - *quantity
        };
        tx.execute(
            "INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?1,?2,?3) \
             ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,\
             updated_at=CURRENT_TIMESTAMP",
            params![product, warehouse, updated],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR REPLACE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
             movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,'invoice_return',?8,?9,?10)",
            params![
                format!("return-stock-{return_id}-{product}"),
                company,
                product,
                warehouse,
                if sale { "receipt" } else { "issue" },
                quantity,
                price,
                return_id,
                if sale {
                    "برگشت از فروش"
                } else {
                    "برگشت از خرید"
                },
                user
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // مالیات متناسب از فاکتور اصلی.
    let (invoice_subtotal, invoice_tax) = invoice_tax_ratio(&tx, t.invoices, &invoice_id)?;
    let tax = if invoice_subtotal > 0 {
        (total as i128 * invoice_tax as i128 / invoice_subtotal as i128) as i64
    } else {
        0
    };
    let grand_total = total + tax;

    let journal_id = format!("journal-return-{return_id}");
    let number = next_journal_number(&tx, &company, &fy)?;
    let description = if sale {
        "برگشت از فروش"
    } else {
        "برگشت از خرید"
    };
    tx.execute(
        "INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,\
         status,source_type,source_id,created_by) VALUES(?1,?2,?3,?4,?5,?6,'posted','invoice_return',?7,?8)",
        params![
            journal_id,
            company,
            fy,
            number,
            return_date,
            description,
            return_id,
            user
        ],
    )
    .map_err(|e| format!("RET-011: {e}"))?;

    // سند برگشت از فروش: کاهنده‌ی درآمد و مالیات بدهکار، مشتری بستانکار.
    // سند برگشت از خرید: تأمین‌کننده بدهکار، کاهنده‌ی خرید و مالیات بستانکار.
    let mut journal_lines: Vec<(&str, i64, i64)> = Vec::with_capacity(3);
    if sale {
        journal_lines.push(("acc-4200", total, 0));
        if tax > 0 {
            journal_lines.push(("acc-2401", tax, 0));
        }
        journal_lines.push(("acc-1201", 0, grand_total));
    } else {
        journal_lines.push(("acc-2101", grand_total, 0));
        journal_lines.push(("acc-5200", 0, total));
        if tax > 0 {
            journal_lines.push(("acc-2401", 0, tax));
        }
    }

    // بررسی نهایی توازن پیش از نوشتن — سند نامتوازن هرگز نباید ثبت شود.
    let debit: i64 = journal_lines.iter().map(|(_, d, _)| *d).sum();
    let credit: i64 = journal_lines.iter().map(|(_, _, c)| *c).sum();
    if debit != credit {
        return Err(format!(
            "RET-012: سند برگشت متوازن نیست (بدهکار {debit} در برابر بستانکار {credit})"
        ));
    }

    for (index, (account, line_debit, line_credit)) in journal_lines.iter().enumerate() {
        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1",
                params![account],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if exists == 0 {
            return Err(format!("RET-013: حساب «{account}» در کدینگ تعریف نشده است"));
        }
        tx.execute(
            "INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{journal_id}-l{index}"),
                journal_id,
                account,
                line_debit,
                line_credit,
                description
            ],
        )
        .map_err(|e| format!("RET-014: {e}"))?;
    }

    tx.execute(
        &format!(
            "UPDATE {rt} SET status='posted',journal_id=?1 WHERE id=?2",
            rt = t.returns
        ),
        params![journal_id, return_id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        permission,
        if sale {
            "sales_return"
        } else {
            "purchase_return"
        },
        return_id,
        Some("{\"status\":\"draft\"}"),
        Some(&format!(
            "{{\"status\":\"posted\",\"total\":{grand_total},\"tax\":{tax}}}"
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn post_sales_return_v2(state: State<AppState>, id: String) -> Result<(), String> {
    post(&state, true, &id)
}

#[tauri::command]
pub fn post_purchase_return_v2(state: State<AppState>, id: String) -> Result<(), String> {
    post(&state, false, &id)
}

/// ابطال برگشت پیش‌نویس. برگشت ثبت‌شده باطل نمی‌شود — باید سند معکوس بخورد.
#[tauri::command]
pub fn cancel_return(state: State<AppState>, sale: bool, id: String) -> Result<(), String> {
    let t = tables(sale);
    let mut c = conn(&state)?;
    let permission = if sale {
        "sales.return.create"
    } else {
        "purchase.return.create"
    };
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let status: Option<String> = tx
        .query_row(
            &format!(
                "SELECT status FROM {rt} WHERE id=?1 AND company_id=?2",
                rt = t.returns
            ),
            params![id, company],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    match status.as_deref() {
        None => return Err("RET-006: برگشت یافت نشد".into()),
        Some("posted") => {
            return Err("RET-015: برگشت ثبت‌شده باطل نمی‌شود؛ سند معکوس صادر کنید".into())
        }
        Some("cancelled") => return Ok(()),
        _ => {}
    }
    tx.execute(
        &format!(
            "UPDATE {rt} SET status='cancelled' WHERE id=?1",
            rt = t.returns
        ),
        params![id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        permission,
        "return",
        &id,
        None,
        Some("{\"status\":\"cancelled\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
