#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use argon2::{Argon2, PasswordVerifier};
use novin_core::catalog::{PriceLevel, ProductKind};
use novin_core::checks::{
    transition as check_transition, treasury_effect, CheckKind, CheckStatus, TreasuryEffect,
};
use novin_core::coding::{
    validate_dimensions, AccountDefinition, AccountNature, Dimensions, Subsidiary,
};
use novin_core::db;
use novin_core::inventory::{self as core_inventory, MovementKind, ValuationMethod};
use novin_core::invoicing::{
    self, DiscountTier, FreightMode, InvoiceInput as CoreInvoiceInput,
    InvoiceLine as CoreInvoiceLine,
};
use novin_core::jalali;
use novin_core::parties::{self, BalanceStatus, PartyDefinition, PartyFunction, PartyType};
use novin_core::stocktaking::{
    self, BulkPriceChange, CountLine, StocktakeStatus, VarianceAccounts,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Manager, State};

mod chart_of_accounts;
mod parties_form;
mod production;
mod quotes;
mod returns;
mod settings;
mod treasury_accounts;
mod treasury_docs;

pub(crate) struct AppState {
    db_path: Mutex<PathBuf>,
    user_id: Mutex<Option<String>>,
}

#[derive(Serialize)]
struct Company {
    id: String,
    name: String,
    national_id: Option<String>,
}
#[derive(Serialize)]
struct User {
    id: String,
    username: String,
    display_name: String,
}
#[derive(Serialize)]
struct Account {
    id: String,
    code: String,
    name: String,
    level: String,
    parent_id: Option<String>,
    nature: String,
}
#[derive(Serialize)]
struct Contact {
    id: String,
    name: String,
    kind: String,
    mobile: Option<String>,
    is_customer: bool,
    is_supplier: bool,
}
#[derive(Serialize)]
struct Product {
    id: String,
    sku: String,
    barcode: Option<String>,
    name: String,
    unit: String,
    sale_price: i64,
    purchase_price: i64,
    min_stock: f64,
}
#[derive(Serialize)]
struct Journal {
    id: String,
    number: i64,
    entry_date: String,
    description: String,
    status: String,
    total_debit: i64,
    total_credit: i64,
}
#[derive(Serialize)]
struct Permission {
    id: String,
    name: String,
}
#[derive(Serialize)]
struct Warehouse {
    id: String,
    name: String,
    code: String,
    is_active: bool,
}
#[derive(Serialize)]
struct StockBalance {
    product_id: String,
    warehouse_id: String,
    quantity: f64,
    reserved_quantity: f64,
    available_quantity: f64,
}

pub(crate) fn conn(state: &State<AppState>) -> Result<Connection, String> {
    Connection::open(
        state
            .db_path
            .lock()
            .map_err(|_| "APP-001: قفل پایگاه داده در دسترس نیست".to_string())?
            .clone(),
    )
    .map_err(|e| e.to_string())
}

fn require_login(state: &State<AppState>) -> Result<String, String> {
    state
        .user_id
        .lock()
        .map_err(|_| "AUTH-001: وضعیت ورود در دسترس نیست".to_string())?
        .clone()
        .ok_or_else(|| "AUTH-002: ابتدا وارد حساب کاربری شوید".into())
}

fn has_permission(c: &Connection, user_id: &str, permission: &str) -> Result<bool, String> {
    let count:i64=c.query_row("SELECT COUNT(*) FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.name=?2",params![user_id,permission],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok(count > 0)
}

pub(crate) fn require_permission(
    state: &State<AppState>,
    c: &Connection,
    permission: &str,
) -> Result<String, String> {
    let user = require_login(state)?;
    if !has_permission(c, &user, permission)? {
        return Err(format!(
            "AUTH-403: مجوز لازم برای این عملیات وجود ندارد: {permission}"
        ));
    }
    Ok(user)
}

pub(crate) fn audit(
    tx: &rusqlite::Transaction<'_>,
    user_id: &str,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<(), String> {
    tx.execute("INSERT INTO audit_logs(id,user_id,action,entity_type,entity_id,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![format!("audit-{action}-{entity_id}-{}",chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()),user_id,action,entity_type,entity_id,before,after]).map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command]
fn login(state: State<AppState>, username: String, password: String) -> Result<User, String> {
    let c = conn(&state)?;
    let row= c.query_row("SELECT id,username,display_name,password_hash FROM users WHERE username=?1 AND is_active=1",params![username],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|_|"AUTH-003: نام کاربری یا رمز عبور نادرست است".to_string())?;
    let parsed = argon2::PasswordHash::new(&row.3)
        .map_err(|_| "AUTH-004: اطلاعات ورود نامعتبر است".to_string())?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| "AUTH-003: نام کاربری یا رمز عبور نادرست است".to_string())?;
    *state
        .user_id
        .lock()
        .map_err(|_| "AUTH-001: وضعیت ورود در دسترس نیست".to_string())? = Some(row.0.clone());
    Ok(User {
        id: row.0,
        username: row.1,
        display_name: row.2,
    })
}

#[tauri::command]
fn logout(state: State<AppState>) -> Result<(), String> {
    *state
        .user_id
        .lock()
        .map_err(|_| "AUTH-001: وضعیت ورود در دسترس نیست".to_string())? = None;
    Ok(())
}

#[tauri::command]
fn current_user(state: State<AppState>) -> Result<Option<User>, String> {
    let Some(id) = state
        .user_id
        .lock()
        .map_err(|_| "AUTH-001: وضعیت ورود در دسترس نیست".to_string())?
        .clone()
    else {
        return Ok(None);
    };
    let c = conn(&state)?;
    c.query_row(
        "SELECT id,username,display_name FROM users WHERE id=?1",
        params![id],
        |r| {
            Ok(User {
                id: r.get(0)?,
                username: r.get(1)?,
                display_name: r.get(2)?,
            })
        },
    )
    .map(Some)
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_company(state: State<AppState>) -> Result<Company, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    c.query_row("SELECT c.id,c.name,c.national_id FROM companies c JOIN company_users cu ON cu.company_id=c.id WHERE cu.user_id=?1 AND cu.is_active=1 LIMIT 1",params![user],|r|Ok(Company{id:r.get(0)?,name:r.get(1)?,national_id:r.get(2)?})).map_err(|e|e.to_string())
}

#[tauri::command]
fn list_accounts(state: State<AppState>) -> Result<Vec<Account>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT a.id,a.code,a.name,a.level,a.parent_id,a.nature FROM accounts a JOIN company_users cu ON cu.company_id=a.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY a.code").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Account {
                id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                level: r.get(3)?,
                parent_id: r.get(4)?,
                nature: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_contacts(state: State<AppState>) -> Result<Vec<Contact>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT x.id,x.name,x.kind,x.mobile,x.is_customer,x.is_supplier FROM contacts x JOIN company_users cu ON cu.company_id=x.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY x.name").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Contact {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                mobile: r.get(3)?,
                is_customer: r.get::<_, i64>(4)? != 0,
                is_supplier: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_products(state: State<AppState>) -> Result<Vec<Product>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT p.id,p.sku,p.barcode,p.name,p.unit,p.sale_price,p.purchase_price,p.min_stock FROM products p JOIN company_users cu ON cu.company_id=p.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY p.name").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Product {
                id: r.get(0)?,
                sku: r.get(1)?,
                barcode: r.get(2)?,
                name: r.get(3)?,
                unit: r.get(4)?,
                sale_price: r.get(5)?,
                purchase_price: r.get(6)?,
                min_stock: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_permissions(state: State<AppState>) -> Result<Vec<Permission>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT DISTINCT p.id,p.name FROM permissions p JOIN role_permissions rp ON rp.permission_id=p.id JOIN user_roles ur ON ur.role_id=rp.role_id WHERE ur.user_id=?1 ORDER BY p.name").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Permission {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_journals(state: State<AppState>) -> Result<Vec<Journal>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT j.id,j.number,j.entry_date,j.description,j.status,COALESCE(SUM(l.debit),0),COALESCE(SUM(l.credit),0) FROM journal_entries j JOIN company_users cu ON cu.company_id=j.company_id LEFT JOIN journal_lines l ON l.journal_id=j.id WHERE cu.user_id=?1 AND cu.is_active=1 GROUP BY j.id ORDER BY j.number DESC").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Journal {
                id: r.get(0)?,
                number: r.get(1)?,
                entry_date: r.get(2)?,
                description: r.get(3)?,
                status: r.get(4)?,
                total_debit: r.get(5)?,
                total_credit: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_warehouses(state: State<AppState>) -> Result<Vec<Warehouse>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT w.id,w.name,w.code,w.is_active FROM warehouses w JOIN company_users cu ON cu.company_id=w.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY w.code").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(Warehouse {
                id: r.get(0)?,
                name: r.get(1)?,
                code: r.get(2)?,
                is_active: r.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_contact(
    state: State<AppState>,
    name: String,
    kind: String,
    mobile: Option<String>,
    is_customer: bool,
    is_supplier: bool,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("CONTACT-001: نام شخص الزامی است".into());
    }
    if kind != "person" && kind != "company" {
        return Err("CONTACT-002: نوع شخص نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.create")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "CONTACT-003: شرکت فعال یافت نشد".to_string())?;
    let id = format!(
        "contact-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    c.execute("INSERT INTO contacts(id,company_id,kind,name,mobile,is_customer,is_supplier) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,company,kind,name,mobile,is_customer as i64,is_supplier as i64]).map_err(|e|format!("CONTACT-004: {e}"))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "contact.create",
        "contact",
        &id,
        None,
        Some(&format!("{{\"name\":\"{}\"}}", name.replace('"', "'"))),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn update_contact(
    state: State<AppState>,
    id: String,
    name: String,
    kind: String,
    mobile: Option<String>,
    is_customer: bool,
    is_supplier: bool,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("CONTACT-001: نام شخص الزامی است".into());
    }
    if kind != "person" && kind != "company" {
        return Err("CONTACT-002: نوع شخص نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.edit")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "CONTACT-003: شرکت فعال یافت نشد".to_string())?;
    let before:String=c.query_row("SELECT json_object('name',name,'kind',kind,'mobile',mobile,'is_customer',is_customer,'is_supplier',is_supplier) FROM contacts WHERE id=?1 AND company_id=?2",params![id,company],|r|r.get(0)).map_err(|_|"CONTACT-005: شخص یافت نشد".to_string())?;
    c.execute("UPDATE contacts SET name=?1,kind=?2,mobile=?3,is_customer=?4,is_supplier=?5 WHERE id=?6 AND company_id=?7",params![name,kind,mobile,is_customer as i64,is_supplier as i64,id,company]).map_err(|e|format!("CONTACT-006: {e}"))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "contact.edit",
        "contact",
        &id,
        Some(&before),
        Some(&format!(
            "{{\"name\":\"{}\",\"kind\":\"{}\"}}",
            name.replace('"', "'"),
            kind
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_contact(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.delete")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "CONTACT-003: شرکت فعال یافت نشد".to_string())?;
    let before:String=c.query_row("SELECT json_object('name',name,'kind',kind) FROM contacts WHERE id=?1 AND company_id=?2",params![id,company],|r|r.get(0)).map_err(|_|"CONTACT-005: شخص یافت نشد".to_string())?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM contacts WHERE id=?1 AND company_id=?2",
        params![id, company],
    )
    .map_err(|e| format!("CONTACT-007: {e}"))?;
    audit(
        &tx,
        &user,
        "contact.delete",
        "contact",
        &id,
        Some(&before),
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn create_product(
    state: State<AppState>,
    sku: String,
    barcode: Option<String>,
    name: String,
    unit: String,
    sale_price: i64,
    purchase_price: i64,
    min_stock: f64,
) -> Result<String, String> {
    if sku.trim().is_empty() || name.trim().is_empty() || unit.trim().is_empty() {
        return Err("PRODUCT-001: کد، نام و واحد کالا الزامی است".into());
    }
    if sale_price < 0 || purchase_price < 0 || min_stock < 0.0 {
        return Err("PRODUCT-002: مقادیر قیمت و حداقل موجودی نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "products.create")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "PRODUCT-003: شرکت فعال یافت نشد".to_string())?;
    let id = format!(
        "product-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    c.execute("INSERT INTO products(id,company_id,sku,barcode,name,unit,sale_price,purchase_price,min_stock) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,company,sku,barcode,name,unit,sale_price,purchase_price,min_stock]).map_err(|e|format!("PRODUCT-004: {e}"))?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "product.create",
        "product",
        &id,
        None,
        Some(&format!(
            "{{\"sku\":\"{}\",\"name\":\"{}\"}}",
            sku.replace('"', "'"),
            name.replace('"', "'")
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[derive(Serialize)]
struct InventoryAdvanced {
    product_id: String,
    warehouse_id: String,
    quantity: f64,
    reserved_quantity: f64,
    in_transit_quantity: f64,
    available_quantity: f64,
    valuation_method: String,
    average_cost: i64,
    inventory_value: i64,
    expiring_quantity: f64,
}
#[derive(Serialize)]
struct InventoryLot {
    id: String,
    product_id: String,
    warehouse_id: String,
    lot_number: String,
    lot_type: String,
    serial_number: Option<String>,
    manufacture_date: Option<String>,
    expiry_date: Option<String>,
    quantity: f64,
    unit_cost: i64,
    status: String,
}
#[derive(Serialize)]
struct InventoryCount {
    id: String,
    warehouse_id: String,
    title: String,
    count_date: String,
    status: String,
    line_count: i64,
    variance_count: i64,
}
#[derive(Serialize)]
struct InventoryTransferOrder {
    id: String,
    product_id: String,
    from_warehouse_id: String,
    to_warehouse_id: String,
    quantity: f64,
    unit_cost: i64,
    status: String,
    note: Option<String>,
}

fn inventory_method(c: &Connection, company: &str) -> String {
    let scoped = format!("inventory_valuation_method:{}", company);
    c.query_row(
        "SELECT value FROM app_settings WHERE key=?1",
        params![scoped],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| {
        c.query_row(
            "SELECT value FROM app_settings WHERE key='inventory_valuation_method'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "weighted_average".to_string())
    })
}

#[tauri::command]
fn get_inventory_valuation_method(state: State<AppState>) -> Result<String, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-100: شرکت فعال یافت نشد".to_string())?;
    Ok(inventory_method(&c, &company))
}

#[tauri::command]
fn set_inventory_valuation_method(state: State<AppState>, method: String) -> Result<(), String> {
    let method = ValuationMethod::parse(&method)
        .map_err(|_| "INV-101: روش ارزش‌گذاری نامعتبر است".to_string())?
        .as_str()
        .to_string();
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.valuation.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-102: شرکت فعال یافت نشد".to_string())?;
    let key = format!("inventory_valuation_method:{}", company);
    tx.execute("INSERT INTO app_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,method]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.valuation.method",
        "inventory_valuation",
        "method",
        None,
        Some(&format!("{{\"method\":\"{}\"}}", method)),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_inventory_advanced(state: State<AppState>) -> Result<Vec<InventoryAdvanced>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-102: شرکت فعال یافت نشد".to_string())?;
    let method = inventory_method(&c, &company);
    let mut st=c.prepare("SELECT b.product_id,b.warehouse_id,b.quantity,b.reserved_quantity,b.in_transit_quantity,p.purchase_price,COALESCE(SUM(CASE WHEN l.expiry_date IS NOT NULL AND l.expiry_date<=date('now','+30 day') AND l.status='active' THEN l.quantity ELSE 0 END),0) FROM inventory_balances b JOIN products p ON p.id=b.product_id LEFT JOIN inventory_lots l ON l.product_id=b.product_id AND l.warehouse_id=b.warehouse_id WHERE p.company_id=?1 GROUP BY b.product_id,b.warehouse_id,b.quantity,b.reserved_quantity,p.purchase_price ORDER BY p.name") .map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, f64>(3)?,
                r.get::<_, f64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, f64>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (pid, wid, qty, reserved, in_transit, purchase, exp) = row;
        let avg = valuation_cost(&c, &company, &pid, &wid, &method).unwrap_or(purchase);
        out.push(InventoryAdvanced {
            product_id: pid,
            warehouse_id: wid,
            quantity: qty,
            reserved_quantity: reserved,
            in_transit_quantity: in_transit,
            available_quantity: (qty - reserved).max(0.0),
            valuation_method: method.clone(),
            average_cost: avg,
            inventory_value: (qty * avg as f64).round() as i64,
            expiring_quantity: exp,
        });
    }
    Ok(out)
}

fn valuation_cost(
    c: &Connection,
    company: &str,
    product: &str,
    warehouse: &str,
    method: &str,
) -> Result<i64, String> {
    Ok(valuation_of(c, company, product, warehouse, method)?.unit_cost)
}

/// ارزش‌گذاری کامل موجودی یک کالا در یک انبار با تفویض به هسته‌ی مالی.
fn valuation_of(
    c: &Connection,
    company: &str,
    product: &str,
    warehouse: &str,
    method: &str,
) -> Result<core_inventory::Valuation, String> {
    let method = ValuationMethod::parse(method).map_err(|e| e.to_string())?;
    let mut st = c
        .prepare(
            "SELECT movement_type,quantity,unit_cost FROM inventory_movements \
             WHERE company_id=?1 AND product_id=?2 AND warehouse_id=?3 \
             ORDER BY created_at ASC,id ASC",
        )
        .map_err(|e| e.to_string())?;
    let movements = st
        .query_map(params![company, product, warehouse], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, f64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|(kind, quantity, unit_cost)| {
            MovementKind::parse(&kind)
                .map(|kind| core_inventory::Movement::new(kind, quantity, unit_cost))
        })
        .collect::<Vec<_>>();
    core_inventory::valuate(&movements, method).map_err(|e| e.to_string())
}

#[tauri::command]
fn reserve_inventory(
    state: State<AppState>,
    product_id: String,
    warehouse_id: String,
    quantity: f64,
    reference_type: Option<String>,
    reference_id: Option<String>,
) -> Result<String, String> {
    if quantity <= 0.0 {
        return Err("INV-110: مقدار رزرو باید بیشتر از صفر باشد".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.reserve")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM products WHERE id=?1 AND is_service=0",
            params![product_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-111: کالا یافت نشد".to_string())?;
    let ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![company, user],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if ok == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    let available:f64=tx.query_row("SELECT COALESCE(quantity-reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![product_id,warehouse_id],|r|r.get(0)).unwrap_or(0.0);
    if available < quantity {
        return Err("INV-112: موجودی قابل رزرو کافی نیست".into());
    }
    let id = format!(
        "reservation-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_reservations(id,company_id,product_id,warehouse_id,quantity,status,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,'reserved',?,?,?)",params![id,company,product_id,warehouse_id,quantity,reference_type,reference_id,user]).map_err(|e|e.to_string())?;
    tx.execute("UPDATE inventory_balances SET reserved_quantity=reserved_quantity+?,updated_at=CURRENT_TIMESTAMP WHERE product_id=? AND warehouse_id=?",params![quantity,product_id,warehouse_id]).map_err(|e|e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.reserve",
        "inventory_reservation",
        &id,
        None,
        Some(&format!(
            "{{\"quantity\":{},\"product_id\":\"{}\"}}",
            quantity, product_id
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn release_inventory(state: State<AppState>, reservation_id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.reserve")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let row:(String,String,String,f64)=tx.query_row("SELECT company_id,product_id,warehouse_id,quantity FROM inventory_reservations WHERE id=?1 AND status='reserved'",params![reservation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|"INV-113: رزرو فعال یافت نشد".to_string())?;
    let allowed: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![row.0, user],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if allowed == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    tx.execute("UPDATE inventory_reservations SET status='released',released_at=CURRENT_TIMESTAMP WHERE id=?1",params![reservation_id]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE inventory_balances SET reserved_quantity=MAX(0,reserved_quantity-?),updated_at=CURRENT_TIMESTAMP WHERE product_id=? AND warehouse_id=?",params![row.3,row.1,row.2]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.reserve.release",
        "inventory_reservation",
        &reservation_id,
        None,
        Some("{\"status\":\"released\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn create_inventory_lot(
    state: State<AppState>,
    product_id: String,
    warehouse_id: String,
    lot_number: String,
    lot_type: String,
    serial_number: Option<String>,
    manufacture_date: Option<String>,
    expiry_date: Option<String>,
    quantity: f64,
    unit_cost: i64,
) -> Result<String, String> {
    if lot_number.trim().is_empty() || quantity < 0.0 || unit_cost < 0 {
        return Err("INV-120: اطلاعات سری/بچ نامعتبر است".into());
    }
    if lot_type == "serial"
        && (serial_number.as_deref().unwrap_or("").trim().is_empty() || quantity != 1.0)
    {
        return Err("INV-121: سریال باید شماره یکتا و مقدار دقیقاً ۱ داشته باشد".into());
    }
    if lot_type != "batch" && lot_type != "serial" {
        return Err("INV-122: نوع Lot نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.lot.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM products WHERE id=?1 AND is_service=0",
            params![product_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-123: کالا یافت نشد".to_string())?;
    let wh: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM warehouses WHERE id=?1 AND company_id=?2 AND is_active=1",
            params![warehouse_id, company],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if wh == 0 {
        return Err("INV-124: انبار معتبر نیست".into());
    }
    let id = format!(
        "lot-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_lots(id,company_id,product_id,warehouse_id,lot_number,lot_type,serial_number,manufacture_date,expiry_date,quantity,unit_cost,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![id,company,product_id,warehouse_id,lot_number,lot_type,serial_number,manufacture_date,expiry_date,quantity,unit_cost,user]).map_err(|e|if e.to_string().contains("UNIQUE"){"INV-125: شماره سریال/بچ تکراری است".into()}else{e.to_string()})?;
    audit(
        &tx,
        &user,
        "inventory.lot.create",
        "inventory_lot",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn list_inventory_lots(
    state: State<AppState>,
    product_id: Option<String>,
    warehouse_id: Option<String>,
) -> Result<Vec<InventoryLot>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-126: شرکت فعال یافت نشد".to_string())?;
    let mut sql=String::from("SELECT l.id,l.product_id,l.warehouse_id,l.lot_number,l.lot_type,l.serial_number,l.manufacture_date,l.expiry_date,l.quantity,l.unit_cost,l.status FROM inventory_lots l WHERE l.company_id=?1");
    let mut vals: Vec<String> = vec![company];
    if let Some(x) = product_id {
        sql.push_str(" AND l.product_id=?2");
        vals.push(x)
    }
    if let Some(x) = warehouse_id {
        sql.push_str(if vals.len() == 2 {
            " AND l.warehouse_id=?3"
        } else {
            " AND l.warehouse_id=?2"
        });
        vals.push(x)
    }
    sql.push_str(" ORDER BY l.expiry_date IS NULL,l.expiry_date,l.lot_number");
    let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|x| x as &dyn rusqlite::ToSql).collect();
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = st
        .query_map(&*refs, |r| {
            Ok(InventoryLot {
                id: r.get(0)?,
                product_id: r.get(1)?,
                warehouse_id: r.get(2)?,
                lot_number: r.get(3)?,
                lot_type: r.get(4)?,
                serial_number: r.get(5)?,
                manufacture_date: r.get(6)?,
                expiry_date: r.get(7)?,
                quantity: r.get(8)?,
                unit_cost: r.get(9)?,
                status: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_inventory_count(
    state: State<AppState>,
    warehouse_id: String,
    title: String,
    count_date: String,
) -> Result<String, String> {
    if title.trim().is_empty() {
        return Err("INV-130: عنوان انبارگردانی الزامی است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1 AND is_active=1",
            params![warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-131: انبار معتبر نیست".to_string())?;
    let id = format!(
        "count-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_count_sessions(id,company_id,warehouse_id,title,count_date,status,created_by) VALUES(?,?,?,?,?,'draft',?)",params![id,company,warehouse_id,title,count_date,user]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO inventory_count_lines(id,session_id,product_id,system_quantity) SELECT 'count-line-'||?1||'-'||p.id,?1,p.id,COALESCE(ib.quantity,0) FROM products p LEFT JOIN inventory_balances ib ON ib.product_id=p.id AND ib.warehouse_id=?2 WHERE p.company_id=?3 AND p.is_service=0",params![id,warehouse_id,company]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.count.create",
        "inventory_count",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn list_inventory_counts(state: State<AppState>) -> Result<Vec<InventoryCount>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-132: شرکت فعال یافت نشد".to_string())?;
    let mut st=c.prepare("SELECT s.id,s.warehouse_id,s.title,s.count_date,s.status,COUNT(l.id),SUM(CASE WHEN ABS(COALESCE(l.variance,0))>0 THEN 1 ELSE 0 END) FROM inventory_count_sessions s LEFT JOIN inventory_count_lines l ON l.session_id=s.id WHERE s.company_id=?1 GROUP BY s.id ORDER BY s.created_at DESC").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(InventoryCount {
                id: r.get(0)?,
                warehouse_id: r.get(1)?,
                title: r.get(2)?,
                count_date: r.get(3)?,
                status: r.get(4)?,
                line_count: r.get(5)?,
                variance_count: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn set_inventory_count_line(
    state: State<AppState>,
    line_id: String,
    counted_quantity: f64,
    recount_quantity: Option<f64>,
    note: Option<String>,
) -> Result<(), String> {
    if counted_quantity < 0.0 {
        return Err("INV-133: مقدار شمارش نمی‌تواند منفی باشد".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let session:String=tx.query_row("SELECT s.id FROM inventory_count_lines l JOIN inventory_count_sessions s ON s.id=l.session_id JOIN company_users cu ON cu.company_id=s.company_id WHERE l.id=?1 AND cu.user_id=?2 AND cu.is_active=1 AND s.status IN ('draft','counting','review')",params![line_id,user],|r|r.get(0)).map_err(|_|"INV-134: سطر انبارگردانی معتبر نیست".to_string())?;
    tx.execute("UPDATE inventory_count_lines SET counted_quantity=?,recount_quantity=?,variance=?-system_quantity,note=?,status=CASE WHEN ? IS NULL THEN 'counted' ELSE 'recounted' END WHERE id=?",params![counted_quantity,recount_quantity,counted_quantity,note,recount_quantity,line_id]).map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE inventory_count_sessions SET status='counting' WHERE id=? AND status='draft'",
        params![session],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.count.line",
        "inventory_count_line",
        &line_id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn post_inventory_count(state: State<AppState>, session_id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.post")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company:String=tx.query_row("SELECT company_id FROM inventory_count_sessions WHERE id=?1 AND status IN ('counting','review')",params![session_id],|r|r.get(0)).map_err(|_|"INV-135: دوره انبارگردانی قابل ثبت نیست".to_string())?;
    let incomplete:i64=tx.query_row("SELECT COUNT(*) FROM inventory_count_lines WHERE session_id=?1 AND counted_quantity IS NULL",params![session_id],|r|r.get(0)).map_err(|e| e.to_string())?;
    if incomplete > 0 {
        return Err("INV-136: همه اقلام باید شمارش شوند".into());
    }
    let wh: String = tx
        .query_row(
            "SELECT warehouse_id FROM inventory_count_sessions WHERE id=?1",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<(String, f64, f64, f64, String)> = {
        let mut st=tx.prepare("SELECT product_id,system_quantity,COALESCE(recount_quantity,counted_quantity),COALESCE(variance,0),id FROM inventory_count_lines WHERE session_id=?1").map_err(|e|e.to_string())?;
        let rows = st
            .query_map(params![session_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, f64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };
    for (pid, _sys, counted, var, lid) in items {
        if var.abs() > 0.0000001 {
            tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![pid,wh,counted]).map_err(|e| e.to_string())?;
            tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![format!("count-move-{}",lid),company,pid,wh,"adjustment",var.abs(),0,"inventory_count",session_id,format!("variance:{}",var),user]).map_err(|e| e.to_string())?;
        }
    }
    tx.execute("UPDATE inventory_count_sessions SET status='posted',posted_at=CURRENT_TIMESTAMP WHERE id=?",params![session_id]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.count.post",
        "inventory_count",
        &session_id,
        None,
        Some("{\"status\":\"posted\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn create_inventory_transfer_order(
    state: State<AppState>,
    product_id: String,
    from_warehouse_id: String,
    to_warehouse_id: String,
    quantity: f64,
    unit_cost: i64,
    note: Option<String>,
) -> Result<String, String> {
    if from_warehouse_id == to_warehouse_id || quantity <= 0.0 {
        return Err("INV-140: مقصد و مبدأ یا مقدار انتقال نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1 AND is_active=1",
            params![from_warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-141: انبار مبدأ معتبر نیست".to_string())?;
    let dest: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1 AND is_active=1",
            params![to_warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-142: انبار مقصد معتبر نیست".to_string())?;
    if company != dest {
        return Err("INV-143: انبارها متعلق به یک شرکت نیستند".into());
    }
    let avail:f64=tx.query_row("SELECT COALESCE(quantity-reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![product_id,from_warehouse_id],|r|r.get(0)).unwrap_or(0.0);
    if avail < quantity {
        return Err("INV-144: موجودی قابل انتقال کافی نیست".into());
    }
    let id = format!(
        "transfer-order-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_transfer_orders(id,company_id,product_id,from_warehouse_id,to_warehouse_id,quantity,unit_cost,status,note,created_by) VALUES(?,?,?,?,?,?,?,'in_transit',?,?)",params![id,company,product_id,from_warehouse_id,to_warehouse_id,quantity,unit_cost,note,user]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE inventory_balances SET quantity=quantity-?,in_transit_quantity=in_transit_quantity+?,updated_at=CURRENT_TIMESTAMP WHERE product_id=? AND warehouse_id=?",params![quantity,quantity,product_id,from_warehouse_id]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.transfer.create",
        "inventory_transfer",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn list_inventory_transfer_orders(
    state: State<AppState>,
) -> Result<Vec<InventoryTransferOrder>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "INV-147: شرکت فعال یافت نشد".to_string())?;
    let mut st=c.prepare("SELECT id,product_id,from_warehouse_id,to_warehouse_id,quantity,unit_cost,status,note FROM inventory_transfer_orders WHERE company_id=?1 ORDER BY created_at DESC").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(InventoryTransferOrder {
                id: r.get(0)?,
                product_id: r.get(1)?,
                from_warehouse_id: r.get(2)?,
                to_warehouse_id: r.get(3)?,
                quantity: r.get(4)?,
                unit_cost: r.get(5)?,
                status: r.get(6)?,
                note: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn receive_inventory_transfer(state: State<AppState>, transfer_id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer.receive")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let row:(String,String,String,String,String,f64,i64)=tx.query_row("SELECT company_id,product_id,from_warehouse_id,to_warehouse_id,status,quantity,unit_cost FROM inventory_transfer_orders WHERE id=?1",params![transfer_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).map_err(|_|"INV-145: انتقال یافت نشد".to_string())?;
    if row.4 != "in_transit" {
        return Err("INV-146: انتقال در وضعیت قابل دریافت نیست".into());
    }
    let ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![row.0, user],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if ok == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    tx.execute("UPDATE inventory_balances SET in_transit_quantity=MAX(0,in_transit_quantity-?),updated_at=CURRENT_TIMESTAMP WHERE product_id=? AND warehouse_id=?",params![row.5,row.1,row.2]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=quantity+excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![row.1,row.3,row.5]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![format!("transfer-out-{}",transfer_id),row.0,row.1,row.2,"transfer_out",row.5,row.6,"transfer",transfer_id,user]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![format!("transfer-in-{}",transfer_id),row.0,row.1,row.3,"transfer_in",row.5,row.6,"transfer",transfer_id,user]).map_err(|e| e.to_string())?;
    tx.execute("UPDATE inventory_transfer_orders SET status='received',received_at=CURRENT_TIMESTAMP WHERE id=?",params![transfer_id]).map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.transfer.receive",
        "inventory_transfer",
        &transfer_id,
        None,
        Some("{\"status\":\"received\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_stock_balances(state: State<AppState>) -> Result<Vec<StockBalance>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut s=c.prepare("SELECT b.product_id,b.warehouse_id,b.quantity,b.reserved_quantity,b.quantity-b.reserved_quantity FROM inventory_balances b JOIN company_users cu ON cu.company_id=(SELECT company_id FROM products WHERE id=b.product_id) WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY b.product_id,b.warehouse_id").map_err(|e|e.to_string())?;
    let rows = s
        .query_map(params![user], |r| {
            Ok(StockBalance {
                product_id: r.get(0)?,
                warehouse_id: r.get(1)?,
                quantity: r.get(2)?,
                reserved_quantity: r.get(3)?,
                available_quantity: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn inventory_move(
    state: &State<AppState>,
    product_id: &str,
    warehouse_id: &str,
    movement_type: &str,
    quantity: f64,
    unit_cost: i64,
    note: Option<&str>,
) -> Result<String, String> {
    if quantity <= 0.0 {
        return Err("INV-001: مقدار حرکت باید بیشتر از صفر باشد".into());
    }
    let mut c = conn(state)?;
    let user = require_permission(
        state,
        &c,
        if movement_type == "receipt" {
            "inventory.receive"
        } else {
            "inventory.issue"
        },
    )?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM products WHERE id=?1 AND is_service=0",
            params![product_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-002: کالا یافت نشد".to_string())?;
    let allowed: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![company, user],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if allowed == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    let wh_ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM warehouses WHERE id=?1 AND company_id=?2 AND is_active=1",
            params![warehouse_id, company],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if wh_ok == 0 {
        return Err("INV-003: انبار معتبر نیست".into());
    }
    let current: f64 = tx
        .query_row(
            "SELECT quantity FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",
            params![product_id, warehouse_id],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let reserved:f64=tx.query_row("SELECT reserved_quantity FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![product_id,warehouse_id],|r|r.get(0)).unwrap_or(0.0);
    if movement_type == "issue" && current - reserved < quantity {
        return Err("INV-004: موجودی قابل فروش کافی نیست".into());
    }
    let new_qty = if movement_type == "receipt" {
        current + quantity
    } else {
        current - quantity
    };
    tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?1,?2,?3) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![product_id,warehouse_id,new_qty]).map_err(|e|e.to_string())?;
    let id = format!(
        "stock-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,note,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,company,product_id,warehouse_id,movement_type,quantity,unit_cost,note,user]).map_err(|e|e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.move",
        "inventory",
        &id,
        None,
        Some(&format!(
            "{{\"type\":\"{movement_type}\",\"quantity\":{quantity}}}"
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn update_product(
    state: State<AppState>,
    id: String,
    sku: String,
    barcode: Option<String>,
    name: String,
    unit: String,
    sale_price: i64,
    purchase_price: i64,
    min_stock: f64,
) -> Result<(), String> {
    if sku.trim().is_empty() || name.trim().is_empty() || unit.trim().is_empty() {
        return Err("PRODUCT-001: SKU، نام و واحد الزامی هستند".into());
    }
    if sale_price < 0 || purchase_price < 0 || min_stock < 0.0 {
        return Err("PRODUCT-002: مقادیر قیمت و حداقل موجودی نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "products.edit")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "PRODUCT-003: شرکت فعال یافت نشد".to_string())?;
    let before:String=c.query_row("SELECT json_object('sku',sku,'barcode',barcode,'name',name,'unit',unit,'sale_price',sale_price,'purchase_price',purchase_price,'min_stock',min_stock) FROM products WHERE id=?1 AND company_id=?2",params![id,company],|r|r.get(0)).map_err(|_|"PRODUCT-004: کالا یافت نشد".to_string())?;
    let result=c.execute("UPDATE products SET sku=?1,barcode=?2,name=?3,unit=?4,sale_price=?5,purchase_price=?6,min_stock=?7 WHERE id=?8 AND company_id=?9",params![sku,barcode,name,unit,sale_price,purchase_price,min_stock,id,company]);
    if let Err(e) = result {
        return Err(if e.to_string().contains("UNIQUE") {
            "PRODUCT-005: SKU یا بارکد تکراری است".into()
        } else {
            format!("PRODUCT-006: {e}")
        });
    }
    let tx = c.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "product.edit",
        "product",
        &id,
        Some(&before),
        Some(&format!(
            "{{\"sku\":\"{}\",\"name\":\"{}\"}}",
            sku.replace('"', "'"),
            name.replace('"', "'")
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_product(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "products.delete")?;
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "PRODUCT-003: شرکت فعال یافت نشد".to_string())?;
    let before: String = c
        .query_row(
            "SELECT json_object('sku',sku,'name',name) FROM products WHERE id=?1 AND company_id=?2",
            params![id, company],
            |r| r.get(0),
        )
        .map_err(|_| "PRODUCT-004: کالا یافت نشد".to_string())?;
    let refs:i64=c.query_row("SELECT (SELECT COUNT(*) FROM inventory_balances WHERE product_id=?1)+(SELECT COUNT(*) FROM inventory_movements WHERE product_id=?1)",params![id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if refs > 0 {
        return Err("PRODUCT-007: این کالا سابقه انبار دارد و حذف مستقیم مجاز نیست".into());
    }
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM products WHERE id=?1 AND company_id=?2",
        params![id, company],
    )
    .map_err(|e| format!("PRODUCT-008: {e}"))?;
    audit(
        &tx,
        &user,
        "product.delete",
        "product",
        &id,
        Some(&before),
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn receive_stock(
    state: State<AppState>,
    product_id: String,
    warehouse_id: String,
    quantity: f64,
    unit_cost: i64,
    note: Option<String>,
) -> Result<String, String> {
    inventory_move(
        &state,
        &product_id,
        &warehouse_id,
        "receipt",
        quantity,
        unit_cost,
        note.as_deref(),
    )
}

#[tauri::command]
fn transfer_stock(
    state: State<AppState>,
    product_id: String,
    from_warehouse_id: String,
    to_warehouse_id: String,
    quantity: f64,
    note: Option<String>,
) -> Result<String, String> {
    if from_warehouse_id == to_warehouse_id {
        return Err("INV-010: انبار مبدأ و مقصد باید متفاوت باشند".into());
    }
    if quantity <= 0.0 {
        return Err("INV-002: مقدار باید بیشتر از صفر باشد".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1",
            params![from_warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-003: انبار مبدأ یافت نشد".to_string())?;
    let dest_company: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1",
            params![to_warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-004: انبار مقصد یافت نشد".to_string())?;
    if company != dest_company {
        return Err("INV-005: انبارها متعلق به یک شرکت نیستند".into());
    }
    let available:f64=tx.query_row("SELECT COALESCE(quantity-reserved_quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![product_id,from_warehouse_id],|r|r.get(0)).unwrap_or(0.0);
    if available < quantity {
        return Err("INV-006: موجودی قابل انتقال کافی نیست".into());
    }
    let cost:i64=tx.query_row("SELECT COALESCE(CAST(ROUND(SUM(quantity*unit_cost)/NULLIF(SUM(quantity),0)) AS INTEGER),0) FROM inventory_movements WHERE product_id=?1 AND company_id=?2 AND movement_type IN ('receipt','transfer_in','adjustment')",params![product_id,company],|r|r.get(0)).unwrap_or(0);
    let out_id = format!(
        "movement-transfer-out-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let in_id = format!(
        "movement-transfer-in-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() + 1
    );
    tx.execute("UPDATE inventory_balances SET quantity=quantity-?,updated_at=CURRENT_TIMESTAMP WHERE product_id=? AND warehouse_id=?",params![quantity,product_id,from_warehouse_id]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=quantity+excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![product_id,to_warehouse_id,quantity]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![out_id,company,product_id,from_warehouse_id,"transfer_out",quantity,cost,"warehouse_transfer",in_id,note,user]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![in_id,company,product_id,to_warehouse_id,"transfer_in",quantity,cost,"warehouse_transfer",out_id,note,user]).map_err(|e|e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.transfer",
        "inventory",
        &out_id,
        None,
        Some(&format!(
            "{{\"product_id\":\"{}\",\"quantity\":{},\"from\":\"{}\",\"to\":\"{}\"}}",
            product_id, quantity, from_warehouse_id, to_warehouse_id
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(out_id)
}

#[tauri::command]
fn adjust_stock(
    state: State<AppState>,
    product_id: String,
    warehouse_id: String,
    new_quantity: f64,
    note: String,
) -> Result<String, String> {
    if new_quantity < 0.0 {
        return Err("INV-011: موجودی جدید نمی‌تواند منفی باشد".into());
    }
    if note.trim().is_empty() {
        return Err("INV-012: شرح اصلاح موجودی الزامی است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.adjust")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM warehouses WHERE id=?1",
            params![warehouse_id],
            |r| r.get(0),
        )
        .map_err(|_| "INV-004: انبار یافت نشد".to_string())?;
    let old:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![product_id,warehouse_id],|r|r.get(0)).unwrap_or(0.0);
    let delta = new_quantity - old;
    if delta.abs() < f64::EPSILON {
        return Err("INV-013: موجودی جدید با موجودی فعلی تفاوتی ندارد".into());
    }
    let cost:i64=tx.query_row("SELECT COALESCE(CAST(ROUND(SUM(quantity*unit_cost)/NULLIF(SUM(quantity),0)) AS INTEGER),0) FROM inventory_movements WHERE product_id=?1 AND company_id=?2 AND unit_cost>0",params![product_id,company],|r|r.get(0)).unwrap_or(0);
    tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![product_id,warehouse_id,new_quantity]).map_err(|e|e.to_string())?;
    let id = format!(
        "movement-adjustment-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,note,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![id,company,product_id,warehouse_id,"adjustment",delta.abs(),cost,"inventory_adjustment",note,user]).map_err(|e|e.to_string())?;
    audit(
        &tx,
        &user,
        "inventory.adjust",
        "inventory",
        &id,
        None,
        Some(&format!(
            "{{\"old_quantity\":{},\"new_quantity\":{},\"delta\":{}}}",
            old, new_quantity, delta
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn issue_stock(
    state: State<AppState>,
    product_id: String,
    warehouse_id: String,
    quantity: f64,
    note: Option<String>,
) -> Result<String, String> {
    inventory_move(
        &state,
        &product_id,
        &warehouse_id,
        "issue",
        quantity,
        0,
        note.as_deref(),
    )
}

fn create_journal_internal(
    state: &State<AppState>,
    entry_date: &str,
    description: &str,
    lines: &[(String, i64, i64)],
    status: &str,
) -> Result<String, String> {
    if lines.is_empty() {
        return Err("ACC-001: سند بدون سطر قابل ثبت نیست".into());
    }
    let debit: i64 = lines.iter().map(|x| x.1).sum();
    let credit: i64 = lines.iter().map(|x| x.2).sum();
    if debit <= 0 || debit != credit {
        return Err("ACC-002: جمع بدهکار و بستانکار باید برابر و بزرگتر از صفر باشد".into());
    }
    for (acc, d, c) in lines {
        if *d < 0 || *c < 0 || (*d > 0 && *c > 0) || (*d == 0 && *c == 0) {
            return Err(format!("ACC-003: سطر حساب {acc} نامعتبر است"));
        }
    }
    let mut c = conn(state)?;
    let user = require_permission(state, &c, "accounting.journal.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (fy,company):(String,String)=tx.query_row("SELECT fy.id,fy.company_id FROM fiscal_years fy JOIN company_users cu ON cu.company_id=fy.company_id WHERE cu.user_id=?1 AND cu.is_active=1 AND fy.is_closed=0 ORDER BY fy.start_date DESC LIMIT 1",params![user],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"ACC-004: سال مالی باز برای کاربر یافت نشد".to_string())?;
    let valid_date: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM fiscal_years WHERE id=?1 AND ?2 BETWEEN start_date AND end_date",
            params![fy, entry_date],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if valid_date == 0 {
        return Err("ACC-005: تاریخ سند خارج از سال مالی است".into());
    }
    for (acc, _, _) in lines {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2 AND is_active=1",
                params![acc, company],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if ok == 0 {
            return Err(format!("ACC-006: حساب معتبر نیست: {acc}"));
        }
    }
    let number: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE fiscal_year_id=?1",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let id = format!("journal-{number}-{}", chrono::Utc::now().timestamp_millis());
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,'manual',?8)",params![id,company,fy,number,entry_date,description,status,user]).map_err(|e|e.to_string())?;
    for (i, (acc, d, cr)) in lines.iter().enumerate() {
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit) VALUES(?1,?2,?3,?4,?5)",params![format!("{id}-line-{i}"),id,acc,d,cr]).map_err(|e|e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "journal.create",
        "journal",
        &id,
        None,
        Some(&format!(
            "{{\"status\":\"{status}\",\"debit\":{debit},\"credit\":{credit}}}"
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

// ===========================================================================
// فاز ۲ — ابعاد مالی و سند یک‌سطری
// مرجع: تصاویر dgNqWj (کدینگ حساب‌ها) و Rb2xiG (صدور سند یک‌سطری)
// ===========================================================================

#[derive(Serialize)]
struct SubsidiaryGroupRow {
    id: String,
    code: String,
    title: String,
}

#[derive(Serialize)]
struct DimensionRow {
    id: String,
    code: String,
    title: String,
}

/// حساب قابل ثبت به‌همراه الزامات ابعاد مالی آن.
#[derive(Serialize)]
struct PostableAccount {
    id: String,
    code: String,
    name: String,
    nature: String,
    requires_subsidiary: bool,
    subsidiary_group_id: Option<String>,
    requires_cost_center: bool,
    requires_project: bool,
}

#[tauri::command]
fn list_subsidiary_groups(state: State<AppState>) -> Result<Vec<SubsidiaryGroupRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "accounting.journal.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare("SELECT id,code,title FROM subsidiary_groups WHERE company_id=?1 ORDER BY code")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(SubsidiaryGroupRow {
                id: r.get(0)?,
                code: r.get(1)?,
                title: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn list_cost_centers(state: State<AppState>) -> Result<Vec<DimensionRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "accounting.journal.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare(
            "SELECT id,code,title FROM cost_centers WHERE company_id=?1 AND is_active=1 ORDER BY code",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(DimensionRow {
                id: r.get(0)?,
                code: r.get(1)?,
                title: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<DimensionRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "accounting.journal.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare(
            "SELECT id,code,title FROM projects WHERE company_id=?1 AND is_active=1 AND status='open' ORDER BY code",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(DimensionRow {
                id: r.get(0)?,
                code: r.get(1)?,
                title: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// فهرست حساب‌های قابل ثبت سند (سطح آخر) به‌همراه الزامات ابعاد مالی.
#[tauri::command]
fn list_postable_accounts(state: State<AppState>) -> Result<Vec<PostableAccount>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "accounting.journal.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare(
            "SELECT id,code,name,nature,requires_subsidiary,subsidiary_group_id,\
                    requires_cost_center,requires_project \
             FROM accounts \
             WHERE company_id=?1 AND is_active=1 AND level='detail' ORDER BY code",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(PostableAccount {
                id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                nature: r.get(3)?,
                requires_subsidiary: r.get::<_, i64>(4)? != 0,
                subsidiary_group_id: r.get(5)?,
                requires_cost_center: r.get::<_, i64>(6)? != 0,
                requires_project: r.get::<_, i64>(7)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// خواندن تعریف حساب و ساخت ابعاد مالی برای اعتبارسنجی توسط هسته.
fn load_account_for_posting(
    c: &Connection,
    company: &str,
    account_id: &str,
) -> Result<AccountDefinition, String> {
    let row: (String, String, String, i64, Option<String>, i64, i64) = c
        .query_row(
            "SELECT code,name,nature,requires_subsidiary,subsidiary_group_id,\
                    requires_cost_center,requires_project \
             FROM accounts WHERE id=?1 AND company_id=?2 AND is_active=1 AND level='detail'",
            params![account_id, company],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .map_err(|_| format!("COD-008: ثبت سند فقط روی حساب سطح آخر مجاز است: {account_id}"))?;
    let nature = match row.2.as_str() {
        "credit" => AccountNature::Credit,
        "mixed" => AccountNature::Mixed,
        _ => AccountNature::Debit,
    };
    let mut account = AccountDefinition::new(row.0, row.1, nature);
    account.requires_subsidiary = row.3 != 0;
    account.subsidiary_group = row.4;
    account.requires_cost_center = row.5 != 0;
    account.requires_project = row.6 != 0;
    Ok(account)
}

/// خواندن تفصیلی شناور و گروه آن.
fn load_subsidiary(c: &Connection, company: &str, id: &str) -> Result<Subsidiary, String> {
    c.query_row(
        "SELECT code,title,group_id FROM subsidiaries WHERE id=?1 AND company_id=?2 AND is_active=1",
        params![id, company],
        |r| {
            Ok(Subsidiary {
                code: r.get(0)?,
                title: r.get(1)?,
                group: r.get(2)?,
            })
        },
    )
    .map_err(|_| "COD-014: تفصیلی انتخاب‌شده معتبر نیست".to_string())
}

#[allow(clippy::too_many_arguments)]
fn build_side_dimensions(
    c: &Connection,
    company: &str,
    subsidiary_id: Option<String>,
    cost_center_id: Option<String>,
    project_id: Option<String>,
) -> Result<Dimensions, String> {
    let subsidiary = match subsidiary_id {
        Some(id) if !id.is_empty() => Some(load_subsidiary(c, company, &id)?),
        _ => None,
    };
    Ok(Dimensions {
        subsidiary,
        cost_center: cost_center_id.filter(|value| !value.is_empty()),
        project: project_id.filter(|value| !value.is_empty()),
    })
}

/// یک طرف سند یک‌سطری، همان‌طور که از رابط کاربری می‌آید.
#[derive(serde::Deserialize)]
struct SinglePostingInput {
    account_id: String,
    #[serde(default)]
    subsidiary_id: Option<String>,
    #[serde(default)]
    cost_center_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
}

/// **صدور سند حسابداری یک‌سطری** — معادل فرم `Rb2xiG` نرم‌افزار فعلی.
///
/// یک مبلغ، یک شرح، یک طرف بدهکار و یک طرف بستانکار. اعتبارسنجی ابعاد مالی و
/// تعادل سند توسط هسته‌ی مالی انجام می‌شود، سپس سند ثبت نهایی می‌گردد.
#[tauri::command]
fn create_single_line_journal(
    state: State<AppState>,
    entry_date: String,
    description: String,
    amount: i64,
    debit: SinglePostingInput,
    credit: SinglePostingInput,
) -> Result<String, String> {
    if amount <= 0 {
        return Err("ACC-011: مبلغ سند باید بزرگ‌تر از صفر باشد".into());
    }
    if debit.account_id == credit.account_id && debit.subsidiary_id == credit.subsidiary_id {
        return Err("ACC-012: حساب بدهکار و بستانکار نمی‌توانند یکی باشند".into());
    }

    let (debit_dimensions, credit_dimensions) = {
        let c = conn(&state)?;
        require_permission(&state, &c, "accounting.journal.create")?;
        let (company, _) = active_company(&state, &c)?;

        let debit_account = load_account_for_posting(&c, &company, &debit.account_id)?;
        let credit_account = load_account_for_posting(&c, &company, &credit.account_id)?;
        let debit_dimensions = build_side_dimensions(
            &c,
            &company,
            debit.subsidiary_id.clone(),
            debit.cost_center_id.clone(),
            debit.project_id.clone(),
        )?;
        let credit_dimensions = build_side_dimensions(
            &c,
            &company,
            credit.subsidiary_id.clone(),
            credit.cost_center_id.clone(),
            credit.project_id.clone(),
        )?;
        validate_dimensions(&debit_account, &debit_dimensions).map_err(|e| e.to_string())?;
        validate_dimensions(&credit_account, &credit_dimensions).map_err(|e| e.to_string())?;
        (debit_dimensions, credit_dimensions)
    };

    let lines = vec![
        (debit.account_id.clone(), amount, 0i64),
        (credit.account_id.clone(), 0i64, amount),
    ];
    let journal_id = create_journal_internal(&state, &entry_date, &description, &lines, "draft")?;

    // ثبت ابعاد مالی روی سطرهای ایجادشده.
    {
        let c = conn(&state)?;
        for (index, dimensions) in [(0usize, &debit_dimensions), (1usize, &credit_dimensions)] {
            c.execute(
                "UPDATE journal_lines SET subsidiary_id=?1, cost_center_id=?2, project_id=?3 \
                 WHERE id=?4",
                params![
                    dimensions.subsidiary.as_ref().map(|s| s.code.clone()),
                    dimensions.cost_center,
                    dimensions.project,
                    format!("{journal_id}-line-{index}")
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    post_journal(state, journal_id.clone())?;
    Ok(journal_id)
}

// ===========================================================================
// فاز ۳ — کاتالوگ کالا: گروه درختی و هفت سطح قیمت
// مرجع: تصاویر 8Xmc1p (لیست کالاها ← قیمت کالاها) و NztJl5 (اطلاعات قیمت‌ها)
// ===========================================================================

#[derive(Serialize)]
struct ProductGroupRow {
    id: String,
    code: String,
    title: String,
    parent_id: Option<String>,
    product_count: i64,
}

#[derive(Serialize)]
struct PriceLevelRow {
    level: String,
    label: String,
    price: Option<i64>,
}

#[derive(Serialize)]
struct ProductPriceRow {
    id: String,
    sku: String,
    name: String,
    kind: String,
    kind_label: String,
    group_title: Option<String>,
    prices: Vec<PriceLevelRow>,
}

#[tauri::command]
fn list_product_groups(state: State<AppState>) -> Result<Vec<ProductGroupRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare(
            "SELECT g.id,g.code,g.title,g.parent_id,\
                    (SELECT COUNT(*) FROM products p WHERE p.group_id=g.id) \
             FROM product_groups g WHERE g.company_id=?1 AND g.is_active=1 ORDER BY g.code",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(ProductGroupRow {
                id: r.get(0)?,
                code: r.get(1)?,
                title: r.get(2)?,
                parent_id: r.get(3)?,
                product_count: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// فهرست کالاها به‌همراه هر هفت سطح قیمت.
#[tauri::command]
fn list_product_prices(state: State<AppState>) -> Result<Vec<ProductPriceRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.create")?;
    let (company, _) = active_company(&state, &c)?;

    let mut st = c
        .prepare(
            "SELECT p.id,p.sku,p.name,p.kind,g.title \
             FROM products p LEFT JOIN product_groups g ON g.id=p.group_id \
             WHERE p.company_id=?1 ORDER BY p.sku",
        )
        .map_err(|e| e.to_string())?;
    let products: Vec<(String, String, String, String, Option<String>)> = st
        .query_map(params![company], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let mut price_stmt = c
        .prepare("SELECT level,price FROM product_prices WHERE product_id=?1")
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::with_capacity(products.len());
    for (id, sku, name, kind, group_title) in products {
        let stored: Vec<(String, i64)> = price_stmt
            .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        let prices = PriceLevel::ALL
            .iter()
            .map(|level| PriceLevelRow {
                level: level.as_str().to_string(),
                label: level.label().to_string(),
                price: stored
                    .iter()
                    .find(|(stored_level, _)| stored_level == level.as_str())
                    .map(|(_, price)| *price),
            })
            .collect();
        let kind_label = ProductKind::parse(&kind)
            .unwrap_or(ProductKind::Simple)
            .label()
            .to_string();
        rows.push(ProductPriceRow {
            id,
            sku,
            name,
            kind,
            kind_label,
            group_title,
            prices,
        });
    }
    Ok(rows)
}

/// ثبت یا حذف قیمت یک سطح مشخص برای یک کالا.
///
/// مقدار خالی یعنی «این سطح برای این کالا تعریف نشده» و باعث حذف رکورد می‌شود
/// تا زنجیره‌ی جایگزینی سطح قیمت در هسته درست عمل کند.
#[tauri::command]
fn set_product_price(
    state: State<AppState>,
    product_id: String,
    level: String,
    price: Option<i64>,
) -> Result<(), String> {
    let level = PriceLevel::parse(&level).map_err(|e| e.to_string())?;
    if let Some(value) = price {
        if value < 0 {
            return Err("CAT-002: قیمت نمی‌تواند منفی باشد".into());
        }
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "products.edit")?;
    let (company, _) = active_company(&state, &c)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;

    let owned: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM products WHERE id=?1 AND company_id=?2",
            params![product_id, company],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if owned == 0 {
        return Err("CAT-015: کالا یافت نشد".into());
    }

    match price {
        Some(value) => {
            tx.execute(
                "INSERT INTO product_prices(product_id,level,price) VALUES(?1,?2,?3) \
                 ON CONFLICT(product_id,level) DO UPDATE SET price=excluded.price,\
                 updated_at=CURRENT_TIMESTAMP",
                params![product_id, level.as_str(), value],
            )
            .map_err(|e| e.to_string())?;
            // سطح جزئی، قیمت فروش پایه‌ی کالا را هم به‌روز نگه می‌دارد.
            if level == PriceLevel::Retail {
                tx.execute(
                    "UPDATE products SET sale_price=?1 WHERE id=?2",
                    params![value, product_id],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        None => {
            tx.execute(
                "DELETE FROM product_prices WHERE product_id=?1 AND level=?2",
                params![product_id, level.as_str()],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    audit(
        &tx,
        &user,
        "product.price.set",
        "product",
        &product_id,
        None,
        Some(&format!(
            "{{\"level\":\"{}\",\"price\":{}}}",
            level.as_str(),
            price
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".into())
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ===========================================================================
// فاز ۴ — اشخاص: نقش‌ها، مسیر، اعتبارسنجی هویتی و خلاصه‌ی حساب
// مرجع: تصاویر c9pvYl (لیست اشخاص) و 1zkKV5 (فرم افزودن شخص)
// ===========================================================================

#[derive(Serialize)]
struct PartyRow {
    id: String,
    code: String,
    display_name: String,
    party_type: String,
    party_type_label: String,
    party_function: String,
    party_function_label: String,
    group_title: String,
    is_customer: bool,
    is_supplier: bool,
    mobile: Option<String>,
    route_title: Option<String>,
    marketer_name: Option<String>,
    credit_limit: i64,
    balance: i64,
    balance_status: String,
    balance_indicator: String,
}

#[derive(Serialize)]
struct PartySummary {
    debtor_count: usize,
    debtor_total: i64,
    creditor_count: usize,
    creditor_total: i64,
    settled_count: usize,
    total_count: usize,
    net_total: i64,
}

#[derive(Serialize)]
struct PartyListResult {
    rows: Vec<PartyRow>,
    summary: PartySummary,
}

#[derive(Serialize)]
struct RouteRow {
    id: String,
    code: String,
    title: String,
}

/// مانده‌ی حساب یک شخص از روی سطرهای سند ثبت‌شده.
///
/// قرارداد علامت: مثبت = بدهکار، منفی = بستانکار.
fn party_balance(c: &Connection, company: &str, contact_id: &str) -> i64 {
    c.query_row(
        "SELECT COALESCE(SUM(jl.debit - jl.credit),0) \
         FROM journal_lines jl JOIN journal_entries je ON je.id=jl.journal_id \
         WHERE je.company_id=?1 AND je.status='posted' AND jl.subsidiary_id=?2",
        params![company, contact_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

/// فهرست اشخاص به‌همراه خلاصه‌ی حساب — معادل صفحه‌ی «لیست اشخاص».
#[tauri::command]
fn list_parties(state: State<AppState>) -> Result<PartyListResult, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "contacts.create")?;
    let (company, _) = active_company(&state, &c)?;

    let mut st = c
        .prepare(
            "SELECT ct.id, ct.name, ct.party_type, ct.party_function, ct.is_customer, \
                    ct.is_supplier, ct.mobile, ct.credit_limit, r.title, m.name \
             FROM contacts ct \
             LEFT JOIN party_routes r ON r.id=ct.route_id \
             LEFT JOIN contacts m ON m.id=ct.marketer_id \
             WHERE ct.company_id=?1 ORDER BY ct.name",
        )
        .map_err(|e| e.to_string())?;

    let raw: Vec<(
        String,
        String,
        String,
        String,
        i64,
        i64,
        Option<String>,
        i64,
        Option<String>,
        Option<String>,
    )> = st
        .query_map(params![company], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
                r.get(9)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let mut rows = Vec::with_capacity(raw.len());
    let mut balances = Vec::with_capacity(raw.len());
    for (
        id,
        name,
        party_type,
        party_function,
        is_customer,
        is_supplier,
        mobile,
        credit_limit,
        route_title,
        marketer_name,
    ) in raw
    {
        let balance = party_balance(&c, &company, &id);
        let money = novin_core::money::Money::from_rials(balance);
        let status = BalanceStatus::of(money);
        balances.push(money);
        let kind = PartyType::parse(&party_type).unwrap_or(PartyType::Natural);
        let function = PartyFunction::parse(&party_function).unwrap_or(PartyFunction::Person);
        let group_title = if is_customer != 0 && is_supplier != 0 {
            "مشتری و تأمین‌کننده"
        } else if is_customer != 0 {
            "بدهکاران تجاری"
        } else if is_supplier != 0 {
            "بستانکاران تجاری"
        } else {
            function.label()
        };
        rows.push(PartyRow {
            code: id.clone(),
            id,
            display_name: name,
            party_type: kind.as_str().to_string(),
            party_type_label: kind.label().to_string(),
            party_function: function.as_str().to_string(),
            party_function_label: function.label().to_string(),
            group_title: group_title.to_string(),
            is_customer: is_customer != 0,
            is_supplier: is_supplier != 0,
            mobile,
            route_title,
            marketer_name,
            credit_limit,
            balance,
            balance_status: format!("{status:?}").to_lowercase(),
            balance_indicator: status.indicator().to_string(),
        });
    }

    let summary = parties::summarize_balances(&balances);
    Ok(PartyListResult {
        rows,
        summary: PartySummary {
            debtor_count: summary.debtor_count,
            debtor_total: summary.debtor_total.rials(),
            creditor_count: summary.creditor_count,
            creditor_total: summary.creditor_total.rials(),
            settled_count: summary.settled_count,
            total_count: summary.total_count,
            net_total: summary.net_total.rials(),
        },
    })
}

#[tauri::command]
fn list_party_routes(state: State<AppState>) -> Result<Vec<RouteRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "contacts.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare("SELECT id,code,title FROM party_routes WHERE company_id=?1 AND is_active=1 ORDER BY code")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(RouteRow {
                id: r.get(0)?,
                code: r.get(1)?,
                title: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// اعتبارسنجی هویتی پیش از ذخیره — بدون نوشتن در پایگاه داده.
///
/// رابط کاربری می‌تواند این را حین تایپ صدا بزند تا خطا را فوری نشان دهد.
#[tauri::command]
fn validate_party_identity(
    party_type: String,
    national_id: Option<String>,
    economic_code: Option<String>,
    postal_code: Option<String>,
    mobile: Option<String>,
    iban: Option<String>,
    card_number: Option<String>,
) -> Result<Vec<String>, String> {
    let kind = PartyType::parse(&party_type).unwrap_or(PartyType::Natural);
    let mut problems = Vec::new();
    if let Some(value) = national_id.as_deref().filter(|v| !v.trim().is_empty()) {
        let valid = if kind.is_legal_entity() {
            parties::legal_id_is_valid(value)
        } else {
            parties::national_id_is_valid(value)
        };
        if !valid {
            problems.push(if kind.is_legal_entity() {
                "PRT-003: شناسه ملی شخص حقوقی نامعتبر است".to_string()
            } else {
                "PRT-002: کد ملی نامعتبر است".to_string()
            });
        }
    }
    if let Some(value) = economic_code.as_deref().filter(|v| !v.trim().is_empty()) {
        if !parties::economic_code_is_valid(value) {
            problems.push("PRT-004: کد اقتصادی نامعتبر است".to_string());
        }
    }
    if let Some(value) = postal_code.as_deref().filter(|v| !v.trim().is_empty()) {
        if !parties::postal_code_is_valid(value) {
            problems.push("PRT-005: کد پستی باید ۱۰ رقم باشد".to_string());
        }
    }
    if let Some(value) = mobile.as_deref().filter(|v| !v.trim().is_empty()) {
        if parties::normalize_mobile(value).is_none() {
            problems.push("PRT-006: شماره موبایل نامعتبر است".to_string());
        }
    }
    if let Some(value) = iban.as_deref().filter(|v| !v.trim().is_empty()) {
        if !parties::iban_is_valid(value) {
            problems.push("PRT-007: شماره شبا نامعتبر است".to_string());
        }
    }
    if let Some(value) = card_number.as_deref().filter(|v| !v.trim().is_empty()) {
        if !parties::card_number_is_valid(value) {
            problems.push("PRT-008: شماره کارت بانکی نامعتبر است".to_string());
        }
    }
    Ok(problems)
}

/// به‌روزرسانی مشخصات تکمیلی شخص (نوع، نقش، مسیر، سقف اعتبار و هویت).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn update_party_profile(
    state: State<AppState>,
    contact_id: String,
    party_type: String,
    party_function: String,
    national_id: Option<String>,
    economic_code: Option<String>,
    postal_code: Option<String>,
    credit_limit: i64,
    route_id: Option<String>,
    marketer_id: Option<String>,
) -> Result<(), String> {
    let kind = PartyType::parse(&party_type).ok_or("PRT-013: نوع شخصیت نامعتبر است")?;
    let function = PartyFunction::parse(&party_function).ok_or("PRT-014: نقش شخص نامعتبر است")?;

    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "contacts.edit")?;
    let (company, _) = active_company(&state, &c)?;

    let (name, is_customer, is_supplier): (String, i64, i64) = c
        .query_row(
            "SELECT name,is_customer,is_supplier FROM contacts WHERE id=?1 AND company_id=?2",
            params![contact_id, company],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "PRT-015: شخص یافت نشد".to_string())?;

    let definition = PartyDefinition {
        code: contact_id.clone(),
        party_type: kind,
        function,
        first_name: if kind.is_legal_entity() {
            None
        } else {
            Some(name.clone())
        },
        last_name: None,
        company_name: if kind.is_legal_entity() {
            Some(name)
        } else {
            None
        },
        national_id: national_id.clone(),
        economic_code: economic_code.clone(),
        postal_code: postal_code.clone(),
        mobile: None,
        is_customer: is_customer != 0,
        is_supplier: is_supplier != 0,
        credit_limit,
        route: route_id.clone(),
        marketer_code: marketer_id.clone(),
    };
    definition.validate().map_err(|e| e.to_string())?;

    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE contacts SET party_type=?1, party_function=?2, national_id=?3, \
         economic_code=?4, postal_code=?5, credit_limit=?6, route_id=?7, marketer_id=?8 \
         WHERE id=?9 AND company_id=?10",
        params![
            kind.as_str(),
            function.as_str(),
            national_id,
            economic_code,
            postal_code,
            credit_limit,
            route_id,
            marketer_id,
            contact_id,
            company
        ],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "party.profile.update",
        "contact",
        &contact_id,
        None,
        Some(&format!(
            "{{\"type\":\"{}\",\"function\":\"{}\",\"credit_limit\":{}}}",
            kind.as_str(),
            function.as_str(),
            credit_limit
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ===========================================================================
// فاز ۵ — محاسبه‌ی زنده‌ی فاکتور و اقساط
// مرجع: تصاویر sFpxWK، PI5uot، FRPBDr
// ===========================================================================

#[derive(serde::Deserialize)]
struct InvoiceLineInput {
    product_id: String,
    quantity: f64,
    unit_price: i64,
    #[serde(default)]
    discount_amount: i64,
    #[serde(default)]
    discount_bp: i64,
    #[serde(default)]
    vat_bp: i64,
    #[serde(default)]
    duty_bp: i64,
    #[serde(default)]
    commission_bp: i64,
    #[serde(default)]
    unit_cost: i64,
    #[serde(default)]
    serials: Vec<String>,
    #[serde(default)]
    serial_tracked: bool,
}

#[derive(Serialize)]
struct ComputedLineRow {
    gross: i64,
    tier_discount: i64,
    line_discount: i64,
    header_discount_share: i64,
    coupon_share: i64,
    total_discount: i64,
    net: i64,
    freight_share: i64,
    duty: i64,
    vat: i64,
    total: i64,
    commission: i64,
    cost: i64,
    profit: i64,
}

#[derive(Serialize)]
struct InvoicePreview {
    lines: Vec<ComputedLineRow>,
    subtotal: i64,
    discount_total: i64,
    net_total: i64,
    freight: i64,
    duty_total: i64,
    vat_total: i64,
    total: i64,
    commission_total: i64,
    cost_total: i64,
    profit: i64,
    profit_margin_bp: i64,
    balance_before: i64,
    balance_after: i64,
    invoice_remainder: i64,
}

#[derive(Serialize)]
struct InstallmentRow {
    number: usize,
    due_date: String,
    due_date_jalali: String,
    amount: i64,
}

/// تخفیف پلکانی ذخیره‌شده برای یک کالا (فعلاً از تنظیمات کالا خوانده می‌شود).
fn product_tiers(c: &Connection, product_id: &str) -> Vec<DiscountTier> {
    let mut tiers = Vec::new();
    if let Ok(mut st) = c.prepare(
        "SELECT min_quantity,discount_bp FROM product_discount_tiers \
         WHERE product_id=?1 ORDER BY min_quantity",
    ) {
        if let Ok(rows) = st.query_map(params![product_id], |r| {
            Ok(DiscountTier {
                min_quantity: r.get(0)?,
                discount_bp: r.get(1)?,
            })
        }) {
            tiers.extend(rows.flatten());
        }
    }
    tiers
}

/// محاسبه‌ی زنده‌ی فاکتور — بدون ذخیره‌سازی.
///
/// رابط کاربری این را با هر تغییر سطر صدا می‌زند تا جمع‌ها، سود فاکتور و مانده‌ی
/// طرف حساب دقیقاً با همان موتوری محاسبه شود که هنگام ثبت نهایی اجرا می‌شود؛
/// یعنی چیزی که کاربر می‌بیند هرگز با چیزی که ثبت می‌شود فرق ندارد.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn preview_invoice(
    state: State<AppState>,
    lines: Vec<InvoiceLineInput>,
    header_discount: i64,
    freight: i64,
    freight_allocated: bool,
    contact_id: Option<String>,
    received: i64,
) -> Result<InvoicePreview, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "sales.invoice.create")?;
    let (company, _) = active_company(&state, &c)?;

    let core_lines: Vec<CoreInvoiceLine> = lines
        .into_iter()
        .map(|line| CoreInvoiceLine {
            tiers: product_tiers(&c, &line.product_id),
            product_id: line.product_id,
            quantity: line.quantity,
            unit_price: novin_core::money::Money::from_rials(line.unit_price),
            discount_amount: novin_core::money::Money::from_rials(line.discount_amount),
            discount_bp: line.discount_bp,
            vat_bp: line.vat_bp,
            duty_bp: line.duty_bp,
            commission_bp: line.commission_bp,
            unit_cost: novin_core::money::Money::from_rials(line.unit_cost),
            serials: line.serials,
            serial_tracked: line.serial_tracked,
        })
        .collect();

    let input = CoreInvoiceInput {
        lines: core_lines,
        header_discount: novin_core::money::Money::from_rials(header_discount),
        coupon: None,
        freight: novin_core::money::Money::from_rials(freight),
        freight_mode: if freight_allocated {
            FreightMode::AllocateToLines
        } else {
            FreightMode::AddToTotal
        },
    };

    let result = invoicing::calculate(&input).map_err(|e| e.to_string())?;

    let balance_before = match contact_id {
        Some(id) if !id.is_empty() => party_balance(&c, &company, &id),
        _ => 0,
    };
    let view = invoicing::balance_view(
        novin_core::money::Money::from_rials(balance_before),
        result.total,
        novin_core::money::Money::from_rials(received),
    );

    Ok(InvoicePreview {
        lines: result
            .lines
            .iter()
            .map(|line| ComputedLineRow {
                gross: line.gross.rials(),
                tier_discount: line.tier_discount.rials(),
                line_discount: line.line_discount.rials(),
                header_discount_share: line.header_discount_share.rials(),
                coupon_share: line.coupon_share.rials(),
                total_discount: line.total_discount.rials(),
                net: line.net.rials(),
                freight_share: line.freight_share.rials(),
                duty: line.duty.rials(),
                vat: line.vat.rials(),
                total: line.total.rials(),
                commission: line.commission.rials(),
                cost: line.cost.rials(),
                profit: line.profit.rials(),
            })
            .collect(),
        subtotal: result.subtotal.rials(),
        discount_total: result.discount_total.rials(),
        net_total: result.net_total.rials(),
        freight: result.freight.rials(),
        duty_total: result.duty_total.rials(),
        vat_total: result.vat_total.rials(),
        total: result.total.rials(),
        commission_total: result.commission_total.rials(),
        cost_total: result.cost_total.rials(),
        profit: result.profit.rials(),
        profit_margin_bp: result.profit_margin_bp,
        balance_before: view.before.rials(),
        balance_after: view.after.rials(),
        invoice_remainder: view.invoice_remainder.rials(),
    })
}

/// تولید جدول اقساط برای فاکتور.
#[tauri::command]
fn build_installment_plan(
    total: i64,
    down_payment: i64,
    count: usize,
    first_due_jalali: String,
) -> Result<Vec<InstallmentRow>, String> {
    let first_due = novin_core::jalali::JalaliDate::parse(&first_due_jalali)
        .and_then(|date| date.to_gregorian())
        .map_err(|e| e.to_string())?;
    let plan = invoicing::installment_plan(
        novin_core::money::Money::from_rials(total),
        novin_core::money::Money::from_rials(down_payment),
        count,
        first_due,
    )
    .map_err(|e| e.to_string())?;
    Ok(plan
        .into_iter()
        .map(|item| InstallmentRow {
            number: item.number,
            due_date: item.due_date.to_string(),
            due_date_jalali: item.due_date_jalali,
            amount: item.amount.rials(),
        })
        .collect())
}

// ===========================================================================
// فاز ۶ — انبارگردانی اصولی و عملیات جمعی
// مرجع: منوی «عملیات انبار» نرم‌افزار فعلی + بازخورد کارفرما
// ===========================================================================

#[derive(Serialize)]
struct StocktakeSessionRow {
    id: String,
    title: String,
    warehouse_name: String,
    count_date: String,
    status: String,
    status_label: String,
    total_lines: i64,
    counted_lines: i64,
    variance_lines: i64,
}

#[derive(Serialize)]
struct StocktakeLineRow {
    id: String,
    product_id: String,
    product_name: String,
    sku: String,
    frozen_quantity: f64,
    counted_quantity: Option<f64>,
    recount_quantity: Option<f64>,
    final_quantity: Option<f64>,
    variance: Option<f64>,
    variance_value: i64,
    variance_approved: bool,
    needs_recount: bool,
    unit_cost: i64,
}

#[derive(Serialize)]
struct StocktakeDetail {
    id: String,
    title: String,
    status: String,
    status_label: String,
    warehouse_name: String,
    count_date: String,
    lines: Vec<StocktakeLineRow>,
    total_lines: usize,
    counted_lines: usize,
    uncounted_lines: usize,
    surplus_lines: usize,
    shortage_lines: usize,
    unapproved_variances: usize,
    surplus_value: i64,
    shortage_value: i64,
    net_value: i64,
    recount_threshold_percent: f64,
    can_post: bool,
    blocking_reason: Option<String>,
}

fn setting_value(c: &Connection, key: &str, fallback: &str) -> String {
    c.query_row(
        "SELECT value FROM app_settings WHERE key=?1",
        params![key],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| fallback.to_string())
}

/// خواندن اقلام یک دوره به‌همراه محاسبات هسته.
fn load_stocktake_lines(
    c: &Connection,
    session_id: &str,
) -> Result<(Vec<StocktakeLineRow>, Vec<CountLine>), String> {
    let mut st = c
        .prepare(
            "SELECT l.id,l.product_id,p.name,p.sku,l.frozen_quantity,l.counted_quantity,\
                    l.recount_quantity,l.unit_cost,l.variance_approved \
             FROM stocktake_lines l JOIN products p ON p.id=l.product_id \
             WHERE l.session_id=?1 ORDER BY p.sku",
        )
        .map_err(|e| e.to_string())?;
    let raw: Vec<(
        String,
        String,
        String,
        String,
        f64,
        Option<f64>,
        Option<f64>,
        i64,
        i64,
    )> = st
        .query_map(params![session_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                r.get(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let core: Vec<CountLine> = raw
        .iter()
        .map(|row| CountLine {
            product_id: row.1.clone(),
            frozen_quantity: row.4,
            counted_quantity: row.5,
            recount_quantity: row.6,
            variance_approved: row.8 != 0,
            unit_cost: novin_core::money::Money::from_rials(row.7),
        })
        .collect();

    let threshold: f64 = setting_value(c, "inventory.recount_threshold_percent", "5")
        .parse()
        .unwrap_or(5.0);
    let needing: std::collections::BTreeSet<String> =
        stocktaking::lines_needing_recount(&core, threshold)
            .into_iter()
            .map(|line| line.product_id.clone())
            .collect();

    let rows = raw
        .iter()
        .zip(&core)
        .map(|(row, line)| StocktakeLineRow {
            id: row.0.clone(),
            product_id: row.1.clone(),
            product_name: row.2.clone(),
            sku: row.3.clone(),
            frozen_quantity: row.4,
            counted_quantity: row.5,
            recount_quantity: row.6,
            final_quantity: line.final_quantity(),
            variance: line.variance(),
            variance_value: line.variance_value().map(|v| v.rials()).unwrap_or(0),
            variance_approved: row.8 != 0,
            needs_recount: needing.contains(&row.1),
            unit_cost: row.7,
        })
        .collect();
    Ok((rows, core))
}

#[tauri::command]
fn list_stocktakes(state: State<AppState>) -> Result<Vec<StocktakeSessionRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "inventory.count.create")?;
    let (company, _) = active_company(&state, &c)?;
    let mut st = c
        .prepare(
            "SELECT s.id,s.title,w.name,s.count_date,s.status,\
                    (SELECT COUNT(*) FROM stocktake_lines l WHERE l.session_id=s.id),\
                    (SELECT COUNT(*) FROM stocktake_lines l WHERE l.session_id=s.id \
                       AND COALESCE(l.recount_quantity,l.counted_quantity) IS NOT NULL),\
                    (SELECT COUNT(*) FROM stocktake_lines l WHERE l.session_id=s.id \
                       AND COALESCE(l.recount_quantity,l.counted_quantity) IS NOT NULL \
                       AND ABS(COALESCE(l.recount_quantity,l.counted_quantity)-l.frozen_quantity)>0.000001) \
             FROM stocktake_sessions s JOIN warehouses w ON w.id=s.warehouse_id \
             WHERE s.company_id=?1 ORDER BY s.created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            let status: String = r.get(4)?;
            Ok(StocktakeSessionRow {
                id: r.get(0)?,
                title: r.get(1)?,
                warehouse_name: r.get(2)?,
                count_date: r.get(3)?,
                status_label: StocktakeStatus::parse(&status)
                    .map(|s| s.label())
                    .unwrap_or("نامشخص")
                    .to_string(),
                status,
                total_lines: r.get(5)?,
                counted_lines: r.get(6)?,
                variance_lines: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// ایجاد دوره‌ی انبارگردانی و **فریز موجودی** در همان لحظه.
///
/// موجودی سیستمی و بهای واحد هر کالا با موتور ارزش‌گذاری فعلی عکس‌برداری
/// می‌شود تا فروش حین شمارش، مبنای مقایسه را خراب نکند.
#[tauri::command]
fn create_stocktake(
    state: State<AppState>,
    warehouse_id: String,
    title: String,
    count_date: String,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.create")?;
    let (company, _) = active_company(&state, &c)?;
    let method = inventory_method(&c, &company);

    // بهای واحد هر کالا پیش از باز کردن تراکنش محاسبه می‌شود.
    let products: Vec<(String, f64)> = {
        let mut st = c
            .prepare(
                "SELECT p.id, COALESCE(ib.quantity,0) FROM products p \
                 LEFT JOIN inventory_balances ib ON ib.product_id=p.id AND ib.warehouse_id=?2 \
                 WHERE p.company_id=?1 AND p.is_service=0 ORDER BY p.sku",
            )
            .map_err(|e| e.to_string())?;
        // نتیجه پیش از پایان بلوک به متغیر بسته می‌شود تا `st` زودتر از
        // مصرف‌کننده‌ی خود از بین نرود.
        let rows: Vec<(String, f64)> = st
            .query_map(params![company, warehouse_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        rows
    };
    if products.is_empty() {
        return Err("STK-011: کالایی برای انبارگردانی یافت نشد".into());
    }
    let mut costs = Vec::with_capacity(products.len());
    for (product_id, quantity) in &products {
        let cost = valuation_cost(&c, &company, product_id, &warehouse_id, &method).unwrap_or(0);
        costs.push((product_id.clone(), *quantity, cost));
    }

    let session_id = format!("stocktake-{}", chrono::Utc::now().timestamp_millis());
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO stocktake_sessions(id,company_id,warehouse_id,title,count_date,status,\
         frozen_at,created_by) VALUES(?1,?2,?3,?4,?5,'counting',CURRENT_TIMESTAMP,?6)",
        params![session_id, company, warehouse_id, title, count_date, user],
    )
    .map_err(|e| e.to_string())?;
    for (index, (product_id, quantity, cost)) in costs.iter().enumerate() {
        tx.execute(
            "INSERT INTO stocktake_lines(id,session_id,product_id,frozen_quantity,unit_cost) \
             VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{session_id}-line-{index}"),
                session_id,
                product_id,
                quantity,
                cost
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "stocktake.create",
        "stocktake",
        &session_id,
        None,
        Some(&format!("{{\"lines\":{}}}", costs.len())),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(session_id)
}

#[tauri::command]
fn get_stocktake(state: State<AppState>, session_id: String) -> Result<StocktakeDetail, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "inventory.count.create")?;
    let (company, _) = active_company(&state, &c)?;

    let (title, status, warehouse_name, count_date, threshold): (
        String,
        String,
        String,
        String,
        f64,
    ) = c
        .query_row(
            "SELECT s.title,s.status,w.name,s.count_date,s.recount_threshold_percent \
             FROM stocktake_sessions s JOIN warehouses w ON w.id=s.warehouse_id \
             WHERE s.id=?1 AND s.company_id=?2",
            params![session_id, company],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|_| "STK-012: دوره‌ی انبارگردانی یافت نشد".to_string())?;

    let (rows, core) = load_stocktake_lines(&c, &session_id)?;
    let summary = stocktaking::summarize(&core).map_err(|e| e.to_string())?;
    let postable = stocktaking::ensure_postable(&core);
    let blocking_reason = postable.as_ref().err().map(|e| e.to_string());

    Ok(StocktakeDetail {
        id: session_id,
        title,
        status_label: StocktakeStatus::parse(&status)
            .map(|s| s.label())
            .unwrap_or("نامشخص")
            .to_string(),
        status,
        warehouse_name,
        count_date,
        lines: rows,
        total_lines: summary.total_lines,
        counted_lines: summary.counted_lines,
        uncounted_lines: summary.uncounted_lines,
        surplus_lines: summary.surplus_lines,
        shortage_lines: summary.shortage_lines,
        unapproved_variances: summary.unapproved_variances,
        surplus_value: summary.surplus_value.rials(),
        shortage_value: summary.shortage_value.rials(),
        net_value: summary.net_value.rials(),
        recount_threshold_percent: threshold,
        can_post: postable.is_ok(),
        blocking_reason,
    })
}

/// ثبت شمارش یک قلم (شمارش اول یا مجدد) و تأیید اختلاف.
#[tauri::command]
fn set_stocktake_count(
    state: State<AppState>,
    line_id: String,
    quantity: Option<f64>,
    is_recount: bool,
    approve: Option<bool>,
) -> Result<(), String> {
    if let Some(value) = quantity {
        if !value.is_finite() || value < 0.0 {
            return Err("STK-004: مقدار شمارش نمی‌تواند منفی باشد".into());
        }
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.create")?;
    let (company, _) = active_company(&state, &c)?;

    let status: String = c
        .query_row(
            "SELECT s.status FROM stocktake_lines l \
             JOIN stocktake_sessions s ON s.id=l.session_id \
             WHERE l.id=?1 AND s.company_id=?2",
            params![line_id, company],
            |r| r.get(0),
        )
        .map_err(|_| "STK-013: سطر انبارگردانی یافت نشد".to_string())?;
    let parsed = StocktakeStatus::parse(&status).unwrap_or(StocktakeStatus::Draft);
    if parsed.is_locked() {
        return Err("STK-006: دوره‌ی بسته‌شده قابل تغییر نیست".into());
    }

    let tx = c.transaction().map_err(|e| e.to_string())?;
    if is_recount {
        tx.execute(
            "UPDATE stocktake_lines SET recount_quantity=?1 WHERE id=?2",
            params![quantity, line_id],
        )
    } else {
        tx.execute(
            "UPDATE stocktake_lines SET counted_quantity=?1 WHERE id=?2",
            params![quantity, line_id],
        )
    }
    .map_err(|e| e.to_string())?;

    if let Some(approved) = approve {
        tx.execute(
            "UPDATE stocktake_lines SET variance_approved=?1, approved_by=?2 WHERE id=?3",
            params![i64::from(approved), user, line_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// تأیید گروهی همه‌ی اختلاف‌های یک دوره.
#[tauri::command]
fn approve_all_variances(state: State<AppState>, session_id: String) -> Result<usize, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.post")?;
    let (company, _) = active_company(&state, &c)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let affected = tx
        .execute(
            "UPDATE stocktake_lines SET variance_approved=1, approved_by=?1 \
             WHERE session_id=?2 AND session_id IN \
               (SELECT id FROM stocktake_sessions WHERE company_id=?3 AND status IN ('counting','review'))",
            params![user, session_id, company],
        )
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(affected)
}

/// ثبت نهایی انبارگردانی: اصلاح موجودی + سند تعدیل.
#[tauri::command]
fn post_stocktake(state: State<AppState>, session_id: String) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.count.post")?;
    let (company, _) = active_company(&state, &c)?;

    let (status, warehouse_id): (String, String) = c
        .query_row(
            "SELECT status,warehouse_id FROM stocktake_sessions WHERE id=?1 AND company_id=?2",
            params![session_id, company],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "STK-012: دوره‌ی انبارگردانی یافت نشد".to_string())?;
    let current = StocktakeStatus::parse(&status).unwrap_or(StocktakeStatus::Draft);
    // گذار وضعیت از طریق ماشین حالت هسته اعتبارسنجی می‌شود.
    let target = if current == StocktakeStatus::Counting {
        stocktaking::transition(current, StocktakeStatus::Review).map_err(|e| e.to_string())?
    } else {
        current
    };
    stocktaking::transition(target, StocktakeStatus::Posted).map_err(|e| e.to_string())?;

    let (_, core) = load_stocktake_lines(&c, &session_id)?;
    let accounts = VarianceAccounts {
        inventory: account_id_by_code(&c, &company, "1300")?,
        shortage_expense: account_id_by_code(&c, &company, "6300")?,
        surplus_income: account_id_by_code(&c, &company, "4300")?,
    };
    let journal_lines =
        stocktaking::build_adjustment_journal(&core, &accounts).map_err(|e| e.to_string())?;

    // اصلاح موجودی انبار بر اساس اختلاف تأییدشده
    let tx = c.transaction().map_err(|e| e.to_string())?;
    for line in &core {
        let variance = line.variance().unwrap_or(0.0);
        if variance.abs() < 1e-9 {
            continue;
        }
        tx.execute(
            "INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,\
             quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,'adjustment',?5,?6,'stocktake',?7,?8,?9)",
            params![
                format!("stk-move-{}-{}", session_id, line.product_id),
                company,
                line.product_id,
                warehouse_id,
                variance.abs(),
                line.unit_cost.rials(),
                session_id,
                format!("variance:{variance}"),
                user
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?1,?2,?3) \
             ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=?3",
            params![
                line.product_id,
                warehouse_id,
                line.final_quantity().unwrap_or(line.frozen_quantity)
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "UPDATE stocktake_sessions SET status='posted',posted_at=CURRENT_TIMESTAMP,approved_by=?1 \
         WHERE id=?2",
        params![user, session_id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "stocktake.post",
        "stocktake",
        &session_id,
        None,
        Some(&format!("{{\"journal_lines\":{}}}", journal_lines.len())),
    )?;
    tx.commit().map_err(|e| e.to_string())?;

    // سند تعدیل پس از اصلاح موجودی صادر می‌شود.
    if journal_lines.is_empty() {
        return Ok(String::new());
    }
    let lines: Vec<(String, i64, i64)> = journal_lines
        .iter()
        .map(|line| {
            (
                line.account_id.clone(),
                line.debit.rials(),
                line.credit.rials(),
            )
        })
        .collect();
    let journal_id = create_journal_internal(
        &state,
        &jalali::jalali_string(chrono::Local::now().date_naive()),
        &format!("سند تعدیل انبارگردانی {session_id}"),
        &lines,
        "draft",
    )?;
    {
        let c = conn(&state)?;
        c.execute(
            "UPDATE stocktake_sessions SET journal_id=?1 WHERE id=?2",
            params![journal_id, session_id],
        )
        .map_err(|e| e.to_string())?;
    }
    post_journal(state, journal_id.clone())?;
    Ok(journal_id)
}

fn account_id_by_code(c: &Connection, company: &str, code: &str) -> Result<String, String> {
    c.query_row(
        "SELECT id FROM accounts WHERE company_id=?1 AND code=?2",
        params![company, code],
        |r| r.get(0),
    )
    .map_err(|_| format!("STK-014: حساب با کد {code} تعریف نشده است"))
}

/// پیش‌نمایش تغییر جمعی قیمت — پیش از اعمال همیشه نمایش داده می‌شود.
#[derive(Serialize)]
struct BulkPriceRow {
    product_id: String,
    product_name: String,
    old_price: i64,
    new_price: i64,
    difference: i64,
}

/// محاسبه‌ی مشترک تغییر جمعی قیمت — هم برای پیش‌نمایش و هم برای اعمال.
fn compute_bulk_price(
    c: &Connection,
    company: &str,
    product_ids: &[String],
    mode: &str,
    value: i64,
    round_to: i64,
) -> Result<Vec<BulkPriceRow>, String> {
    if product_ids.is_empty() {
        return Err("BLK-003: هیچ کالایی انتخاب نشده است".into());
    }
    let mut names = std::collections::BTreeMap::new();
    let mut products = Vec::new();
    for id in product_ids {
        let row: Result<(String, i64), _> = c.query_row(
            "SELECT name,sale_price FROM products WHERE id=?1 AND company_id=?2",
            params![id, company],
            |r| Ok((r.get(0)?, r.get(1)?)),
        );
        if let Ok((name, price)) = row {
            names.insert(id.clone(), name);
            products.push((id.clone(), novin_core::money::Money::from_rials(price)));
        }
    }

    let change = match mode {
        "percent" => BulkPriceChange::Percent(value),
        "amount" => BulkPriceChange::Amount(novin_core::money::Money::from_rials(value)),
        "set" => BulkPriceChange::Set(novin_core::money::Money::from_rials(value)),
        _ => return Err("BLK-005: نوع تغییر نامعتبر است".into()),
    };
    let results =
        stocktaking::preview_bulk_price(&products, change, round_to).map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|item| BulkPriceRow {
            product_name: names.get(&item.product_id).cloned().unwrap_or_default(),
            product_id: item.product_id,
            old_price: item.old_price.rials(),
            new_price: item.new_price.rials(),
            difference: item.difference.rials(),
        })
        .collect())
}

#[tauri::command]
fn preview_bulk_price_change(
    state: State<AppState>,
    product_ids: Vec<String>,
    mode: String,
    value: i64,
    round_to: i64,
) -> Result<Vec<BulkPriceRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "products.edit")?;
    let (company, _) = active_company(&state, &c)?;
    compute_bulk_price(&c, &company, &product_ids, &mode, value, round_to)
}

/// اعمال تغییر جمعی قیمت پس از تأیید کاربر.
#[tauri::command]
fn apply_bulk_price_change(
    state: State<AppState>,
    product_ids: Vec<String>,
    mode: String,
    value: i64,
    round_to: i64,
) -> Result<usize, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "products.edit")?;
    let (company, _) = active_company(&state, &c)?;
    let preview = compute_bulk_price(&c, &company, &product_ids, &mode, value, round_to)?;

    let tx = c.transaction().map_err(|e| e.to_string())?;
    for row in &preview {
        tx.execute(
            "UPDATE products SET sale_price=?1 WHERE id=?2 AND company_id=?3",
            params![row.new_price, row.product_id, company],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO product_prices(product_id,level,price) VALUES(?1,'retail',?2) \
             ON CONFLICT(product_id,level) DO UPDATE SET price=excluded.price",
            params![row.product_id, row.new_price],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT INTO bulk_operations(id,company_id,operation,payload,affected_count,performed_by) \
         VALUES(?1,?2,'price_change',?3,?4,?5)",
        params![
            format!("bulk-{}", chrono::Utc::now().timestamp_millis()),
            company,
            format!("{{\"mode\":\"{mode}\",\"value\":{value},\"round_to\":{round_to}}}"),
            preview.len() as i64,
            user
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(preview.len())
}

/// کالاهای نزدیک به اتمام موجودی با آستانه‌ی قابل تنظیم.
#[derive(Serialize)]
struct LowStockRow {
    product_id: String,
    product_name: String,
    sku: String,
    quantity: f64,
    reorder_point: f64,
}

#[tauri::command]
fn get_low_stock(state: State<AppState>) -> Result<Vec<LowStockRow>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "reports.view")?;
    let (company, _) = active_company(&state, &c)?;
    let threshold: f64 = setting_value(&c, "inventory.low_stock_threshold", "5")
        .parse()
        .unwrap_or(5.0);

    let mut st = c
        .prepare(
            "SELECT p.id,p.name,p.sku,COALESCE(SUM(ib.quantity),0),\
                    MAX(COALESCE(p.reorder_point,0),COALESCE(p.min_stock,0)) \
             FROM products p LEFT JOIN inventory_balances ib ON ib.product_id=p.id \
             WHERE p.company_id=?1 AND p.is_service=0 GROUP BY p.id",
        )
        .map_err(|e| e.to_string())?;
    let raw: Vec<(String, String, String, f64, f64)> = st
        .query_map(params![company], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    let tuples: Vec<(String, String, f64, f64)> = raw
        .iter()
        .map(|row| (row.0.clone(), row.1.clone(), row.3, row.4))
        .collect();
    let items = stocktaking::low_stock_items(&tuples, threshold);
    let sku_of: std::collections::BTreeMap<&str, &str> = raw
        .iter()
        .map(|row| (row.0.as_str(), row.2.as_str()))
        .collect();

    Ok(items
        .into_iter()
        .map(|item| LowStockRow {
            sku: sku_of
                .get(item.product_id.as_str())
                .copied()
                .unwrap_or("")
                .to_string(),
            product_id: item.product_id,
            product_name: item.product_name,
            quantity: item.quantity,
            reorder_point: item.reorder_point,
        })
        .collect())
}

/// توضیح ساده‌ی روش‌های ارزش‌گذاری برای نمایش در تنظیمات انبار.
#[derive(Serialize)]
struct ValuationInfo {
    method: String,
    label: String,
    explanation: String,
    is_active: bool,
}

#[tauri::command]
fn list_valuation_methods(state: State<AppState>) -> Result<Vec<ValuationInfo>, String> {
    let c = conn(&state)?;
    require_permission(&state, &c, "inventory.valuation.manage")?;
    let (company, _) = active_company(&state, &c)?;
    let active = inventory_method(&c, &company);
    Ok([
        ValuationMethod::Fifo,
        ValuationMethod::MovingAverage,
        ValuationMethod::WeightedAverage,
    ]
    .into_iter()
    .map(|method| ValuationInfo {
        is_active: method.as_str() == active,
        method: method.as_str().to_string(),
        label: method.label().to_string(),
        explanation: method.plain_explanation().to_string(),
    })
    .collect())
}

#[tauri::command]
fn create_journal_draft(
    state: State<AppState>,
    entry_date: String,
    description: String,
    lines: Vec<(String, i64, i64)>,
) -> Result<String, String> {
    create_journal_internal(&state, &entry_date, &description, &lines, "draft")
}

#[tauri::command]
fn create_journal(
    state: State<AppState>,
    entry_date: String,
    description: String,
    lines: Vec<(String, i64, i64)>,
) -> Result<String, String> {
    let id = create_journal_internal(&state, &entry_date, &description, &lines, "draft")?;
    post_journal(state, id.clone())?;
    Ok(id)
}

#[tauri::command]
fn post_journal(state: State<AppState>, journal_id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.journal.post")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let row:(String,i64,i64,String)=tx.query_row("SELECT status,(SELECT COALESCE(SUM(debit),0) FROM journal_lines WHERE journal_id=j.id),(SELECT COALESCE(SUM(credit),0) FROM journal_lines WHERE journal_id=j.id),company_id FROM journal_entries j WHERE id=?1",params![journal_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|_|"ACC-007: سند یافت نشد".to_string())?;
    if row.0 != "draft" && row.0 != "validated" {
        return Err("ACC-008: فقط سند پیش‌نویس یا تأییدشده قابل ثبت نهایی است".into());
    }
    let fiscal_id: String = tx
        .query_row(
            "SELECT fiscal_year_id FROM journal_entries WHERE id=?1",
            params![journal_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let entry_date: String = tx
        .query_row(
            "SELECT entry_date FROM journal_entries WHERE id=?1",
            params![journal_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    validate_fiscal_date(&tx, &fiscal_id, &entry_date)?;
    if row.1 <= 0 || row.1 != row.2 {
        return Err("ACC-002: سند نامتوازن است".into());
    }
    let allowed: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![row.3, user],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if allowed == 0 {
        return Err("AUTH-403: دسترسی به این شرکت وجود ندارد".into());
    }
    tx.execute(
        "UPDATE journal_entries SET status='posted' WHERE id=?1",
        params![journal_id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "journal.post",
        "journal",
        &journal_id,
        Some("{\"status\":\"draft\"}"),
        Some("{\"status\":\"posted\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())
}

#[tauri::command]
fn reverse_journal(
    state: State<AppState>,
    journal_id: String,
    entry_date: String,
    description: String,
) -> Result<String, String> {
    let user = require_permission(&state, &conn(&state)?, "accounting.journal.reverse")?;
    let mut c = conn(&state)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let status: String = tx
        .query_row(
            "SELECT status FROM journal_entries WHERE id=?1",
            params![journal_id],
            |r| r.get(0),
        )
        .map_err(|_| "ACC-007: سند یافت نشد".to_string())?;
    if status != "posted" {
        return Err("ACC-009: فقط سند ثبت‌شده قابل برگشت است".into());
    }
    // تاریخ سند برگشت باید داخل سال مالی باز باشد.
    //
    // بدون این بررسی، سند معکوس می‌توانست در دوره‌ی بسته بنشیند و تراز
    // سال قبل را — که صورت‌های مالی‌اش صادر شده — عوض کند.
    {
        let fiscal_year: String = tx
            .query_row(
                "SELECT fiscal_year_id FROM journal_entries WHERE id=?1",
                params![journal_id],
                |r| r.get(0),
            )
            .map_err(|_| "ACC-007: سند یافت نشد".to_string())?;
        validate_fiscal_date(&tx, &fiscal_year, &entry_date)?;
    }
    let mut stmt = tx
        .prepare(
            "SELECT account_id,debit,credit FROM journal_lines WHERE journal_id=?1 ORDER BY rowid",
        )
        .map_err(|e| e.to_string())?;
    let lines: Vec<(String, i64, i64)> = stmt
        .query_map(params![journal_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|(a, d, c)| (a, c, d))
        .collect();
    drop(stmt);
    let fy_company: (String, String) = tx
        .query_row(
            "SELECT fiscal_year_id,company_id FROM journal_entries WHERE id=?1",
            params![journal_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let number:i64=tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![fy_company.1,fy_company.0],|r|r.get(0)).map_err(|e|e.to_string())?;
    let id = format!(
        "journal-reversal-{number}-{}",
        chrono::Utc::now().timestamp_millis()
    );
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?1,?2,?3,?4,?5,?6,'posted','reversal',?7,?8)",params![id,fy_company.1,fy_company.0,number,entry_date,description,journal_id,user]).map_err(|e|e.to_string())?;
    for (i, (acc, d, c)) in lines.iter().enumerate() {
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit) VALUES(?1,?2,?3,?4,?5)",params![format!("{id}-line-{i}"),id,acc,d,c]).map_err(|e|e.to_string())?;
    }
    tx.execute(
        "UPDATE journal_entries SET status='reversed' WHERE id=?1",
        params![journal_id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "journal.reverse",
        "journal",
        &journal_id,
        Some("{\"status\":\"posted\"}"),
        Some(&format!(
            "{{\"status\":\"reversed\",\"reversal_id\":\"{id}\"}}"
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[derive(Serialize)]
struct BackupInfo {
    name: String,
    size_bytes: u64,
}

fn backup_dir(state: &State<AppState>) -> Result<PathBuf, String> {
    let db = state
        .db_path
        .lock()
        .map_err(|_| "BACKUP-001: مسیر پایگاه داده در دسترس نیست".to_string())?
        .clone();
    let dir = db
        .parent()
        .ok_or_else(|| "BACKUP-002: مسیر پشتیبان نامعتبر است".to_string())?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
fn list_backups(state: State<AppState>) -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir(&state)?;
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) == Some("sqlite") {
            let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
            out.push(BackupInfo {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                size_bytes: meta.len(),
            });
        }
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

#[tauri::command]
fn backup_database(state: State<AppState>) -> Result<BackupInfo, String> {
    let c = conn(&state)?;
    let user = require_permission(&state, &c, "backup.create")?;
    c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("BACKUP-003: {e}"))?;
    drop(c);
    let source = state
        .db_path
        .lock()
        .map_err(|_| "BACKUP-001: مسیر پایگاه داده در دسترس نیست".to_string())?
        .clone();
    let dir = backup_dir(&state)?;
    let name = format!(
        "novin-accounting-{}.sqlite",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let target = dir.join(&name);
    std::fs::copy(&source, &target).map_err(|e| format!("BACKUP-004: {e}"))?;
    let check = Connection::open(&target).map_err(|e| format!("BACKUP-005: {e}"))?;
    let integrity: String = check
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        let _ = std::fs::remove_file(&target);
        return Err("BACKUP-006: بررسی سلامت نسخه پشتیبان ناموفق بود".into());
    }
    let size = std::fs::metadata(&target).map_err(|e| e.to_string())?.len();
    let mut audit_conn = conn(&state)?;
    let tx = audit_conn.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "backup.create",
        "database",
        &name,
        None,
        Some(&format!("{{\"size_bytes\":{size}}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(BackupInfo {
        name,
        size_bytes: size,
    })
}

#[tauri::command]
fn restore_database(state: State<AppState>, name: String) -> Result<(), String> {
    let c = conn(&state)?;
    let user = require_permission(&state, &c, "backup.restore")?;
    drop(c);
    if name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !name.ends_with(".sqlite")
    {
        return Err("BACKUP-007: نام نسخه پشتیبان نامعتبر است".into());
    }
    let source = backup_dir(&state)?.join(&name);
    if !source.is_file() {
        return Err("BACKUP-008: نسخه پشتیبان پیدا نشد".into());
    }
    let check = Connection::open(&source).map_err(|e| format!("BACKUP-005: {e}"))?;
    let integrity: String = check
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err("BACKUP-006: نسخه پشتیبان سالم نیست".into());
    };
    drop(check);
    let target = state
        .db_path
        .lock()
        .map_err(|_| "BACKUP-001: مسیر پایگاه داده در دسترس نیست".to_string())?
        .clone();
    let safety = target.with_extension("pre-restore.sqlite");
    std::fs::copy(&target, &safety).map_err(|e| format!("BACKUP-009: {e}"))?;
    if let Err(e) = std::fs::copy(&source, &target) {
        let _ = std::fs::copy(&safety, &target);
        return Err(format!("BACKUP-010: {e}"));
    }
    let mut audit_conn = conn(&state)?;
    let tx = audit_conn.transaction().map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "backup.restore",
        "database",
        &name,
        None,
        Some("{\"restored\":true}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(safety);
    Ok(())
}

#[tauri::command]
fn get_demo_status(state: State<AppState>) -> Result<bool, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    if !has_permission(&c, &user, "security.role.manage")? {
        return Err("AUTH-403: مجوز مدیریت داده‌های نمونه وجود ندارد".into());
    }
    let v: String = c
        .query_row(
            "SELECT COALESCE((SELECT value FROM app_settings WHERE key='demo_data'),'false')",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "false".into());
    Ok(v == "true")
}

#[tauri::command]
fn delete_demo_data(state: State<AppState>) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_login(&state)?;
    if !has_permission(&c, &user, "security.role.manage")? {
        return Err("AUTH-403: مجوز مدیریت داده‌های نمونه وجود ندارد".into());
    }
    let tx = c.transaction().map_err(|e| e.to_string())?;
    // Keep the demo company, fiscal year and admin login. Remove all business/demo content.
    for sql in [
      "DELETE FROM invoice_settlements WHERE company_id='company-demo'",
      "DELETE FROM sales_return_lines WHERE return_id IN (SELECT id FROM sales_returns WHERE company_id='company-demo')",
      "DELETE FROM purchase_return_lines WHERE return_id IN (SELECT id FROM purchase_returns WHERE company_id='company-demo')",
      "DELETE FROM sales_returns WHERE company_id='company-demo'",
      "DELETE FROM purchase_returns WHERE company_id='company-demo'",
      "DELETE FROM sales_invoice_lines WHERE invoice_id IN (SELECT id FROM sales_invoices WHERE company_id='company-demo')",
      "DELETE FROM purchase_invoice_lines WHERE invoice_id IN (SELECT id FROM purchase_invoices WHERE company_id='company-demo')",
      "DELETE FROM sales_invoices WHERE company_id='company-demo'",
      "DELETE FROM purchase_invoices WHERE company_id='company-demo'",
      "DELETE FROM treasury_transactions WHERE company_id='company-demo'",
      "DELETE FROM checks WHERE company_id='company-demo'",
      "DELETE FROM inventory_movements WHERE company_id='company-demo'",
      "DELETE FROM inventory_reservations WHERE company_id='company-demo'",
      "DELETE FROM inventory_lots WHERE company_id='company-demo'",
      "DELETE FROM inventory_transfer_orders WHERE company_id='company-demo'",
      "DELETE FROM inventory_counts WHERE company_id='company-demo'",
      "DELETE FROM inventory_balances WHERE product_id IN (SELECT id FROM products WHERE company_id='company-demo')",
      "DELETE FROM journal_lines WHERE journal_id IN (SELECT id FROM journal_entries WHERE company_id='company-demo')",
      "DELETE FROM journal_entries WHERE company_id='company-demo'",
      "DELETE FROM custom_reports WHERE company_id='company-demo'",
      "DELETE FROM print_templates WHERE company_id='company-demo'",
      "DELETE FROM import_batches WHERE company_id='company-demo'",
      "DELETE FROM treasury_accounts WHERE company_id='company-demo'",
      "DELETE FROM products WHERE company_id='company-demo'",
      "DELETE FROM contacts WHERE company_id='company-demo'",
      "DELETE FROM warehouses WHERE company_id='company-demo'"
    ] { let _=tx.execute(sql,[]); }
    tx.execute(
        "UPDATE app_settings SET value='false' WHERE key='demo_data'",
        [],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "demo.delete",
        "system",
        "demo-data",
        Some("{\"demo_data\":true}"),
        Some("{\"demo_data\":false}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
struct PrintTemplate {
    id: String,
    name: String,
    template_type: String,
    content_html: String,
    is_default: bool,
}

#[tauri::command]
fn list_print_templates(state: State<AppState>) -> Result<Vec<PrintTemplate>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "printing.template.view")?;
    let mut st=c.prepare("SELECT t.id,t.name,t.template_type,t.content_html,t.is_default FROM print_templates t JOIN company_users cu ON cu.company_id=t.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY t.template_type,t.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(PrintTemplate {
                id: r.get(0)?,
                name: r.get(1)?,
                template_type: r.get(2)?,
                content_html: r.get(3)?,
                is_default: r.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn save_print_template(
    state: State<AppState>,
    id: Option<String>,
    name: String,
    template_type: String,
    content_html: String,
    is_default: bool,
) -> Result<String, String> {
    if name.trim().is_empty() || content_html.trim().is_empty() {
        return Err("PRINT-001: نام و محتوای قالب الزامی است".into());
    }
    if !["invoice", "receipt", "journal", "report", "label"].contains(&template_type.as_str()) {
        return Err("PRINT-002: نوع قالب نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "printing.template.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "PRINT-003: شرکت فعال یافت نشد".to_string())?;
    let rid = id.filter(|x| !x.trim().is_empty()).unwrap_or_else(|| {
        format!(
            "tpl-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });
    if is_default {
        tx.execute(
            "UPDATE print_templates SET is_default=0 WHERE company_id=?1 AND template_type=?2",
            params![company, template_type],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute("INSERT INTO print_templates(id,company_id,name,template_type,content_html,is_default,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET name=excluded.name,template_type=excluded.template_type,content_html=excluded.content_html,is_default=excluded.is_default,updated_at=CURRENT_TIMESTAMP",params![rid,company,name,template_type,content_html,is_default as i64,user]).map_err(|e|format!("PRINT-004: ذخیره قالب انجام نشد: {e}"))?;
    audit(
        &tx,
        &user,
        "print.template.save",
        "print_template",
        &rid,
        None,
        Some(&content_html),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(rid)
}

#[tauri::command]
fn delete_print_template(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "printing.template.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM print_templates WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",params![id,user]).map_err(|e|e.to_string())?;
    audit(
        &tx,
        &user,
        "print.template.delete",
        "print_template",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_data(
    state: State<AppState>,
    entity_type: String,
    rows_json: String,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(
        &state,
        &c,
        if entity_type == "contacts" {
            "contacts.create"
        } else {
            "products.create"
        },
    )?;
    if entity_type != "contacts" && entity_type != "products" {
        return Err("IMPORT-001: فقط ورود اشخاص و کالاها در این نسخه فعال است".into());
    }
    let rows: Vec<serde_json::Value> = serde_json::from_str(&rows_json)
        .map_err(|_| "IMPORT-002: فایل/داده ورودی معتبر نیست".to_string())?;
    if rows.is_empty() {
        return Err("IMPORT-003: داده‌ای برای ورود وجود ندارد".into());
    }
    if rows.len() > 10000 {
        return Err("IMPORT-004: حداکثر 10000 ردیف مجاز است".into());
    }
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company: String = tx
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "IMPORT-005: شرکت فعال یافت نشد".to_string())?;
    let batch = format!(
        "import-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO import_batches(id,company_id,entity_type,row_count,status,created_by) VALUES(?1,?2,?3,?4,'started',?5)",params![batch,company,entity_type,rows.len() as i64,user]).map_err(|e|e.to_string())?;
    let result = (|| -> Result<(), String> {
        for (i, row) in rows.iter().enumerate() {
            if entity_type == "contacts" {
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if name.is_empty() {
                    return Err(format!("IMPORT-006: نام شخص در ردیف {} الزامی است", i + 1));
                }
                let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("person");
                if kind != "person" && kind != "company" {
                    return Err(format!("IMPORT-007: نوع شخص در ردیف {} نامعتبر است", i + 1));
                }
                let mobile = row.get("mobile").and_then(|v| v.as_str());
                let id = format!("contact-import-{}-{}", batch, i);
                let cust = row
                    .get("is_customer")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let supp = row
                    .get("is_supplier")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                tx.execute("INSERT INTO contacts(id,company_id,kind,name,mobile,is_customer,is_supplier) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,company,kind,name,mobile,cust as i64,supp as i64]).map_err(|e|format!("IMPORT-008: ردیف {}: {e}",i+1))?;
            } else {
                let sku = row.get("sku").and_then(|v| v.as_str()).unwrap_or("").trim();
                let name = row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let unit = row.get("unit").and_then(|v| v.as_str()).unwrap_or("عدد");
                if sku.is_empty() || name.is_empty() {
                    return Err(format!(
                        "IMPORT-009: SKU و نام کالا در ردیف {} الزامی است",
                        i + 1
                    ));
                }
                let sale = row.get("sale_price").and_then(|v| v.as_i64()).unwrap_or(0);
                let purchase = row
                    .get("purchase_price")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let min = row.get("min_stock").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let barcode = row.get("barcode").and_then(|v| v.as_str());
                let id = format!("product-import-{}-{}", batch, i);
                if sale < 0 || purchase < 0 || min < 0.0 {
                    return Err(format!(
                        "IMPORT-010: مبلغ/حداقل موجودی در ردیف {} نامعتبر است",
                        i + 1
                    ));
                }
                tx.execute("INSERT INTO products(id,company_id,sku,barcode,name,unit,sale_price,purchase_price,min_stock) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,company,sku,barcode,name,unit,sale,purchase,min]).map_err(|e|format!("IMPORT-011: ردیف {}: {e}",i+1))?;
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        tx.execute(
            "UPDATE import_batches SET status='failed',error_message=?2 WHERE id=?1",
            params![batch, e.clone()],
        )
        .ok();
        return Err(e);
    }
    tx.execute(
        "UPDATE import_batches SET status='completed' WHERE id=?1",
        params![batch],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "data.import",
        "import_batch",
        &batch,
        None,
        Some(&format!(
            "{{\"entity\":\"{}\",\"rows\":{}}}",
            entity_type,
            rows.len()
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(batch)
}

#[derive(serde::Serialize)]
struct InvoiceSummary {
    id: String,
    number: i64,
    invoice_date: String,
    contact_id: Option<String>,
    warehouse_id: Option<String>,
    status: String,
    payment_status: String,
    subtotal: i64,
    discount: i64,
    tax: i64,
    total: i64,
}

fn next_invoice_number(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    company: &str,
    fy: &str,
) -> Result<i64, String> {
    let sql = format!(
        "SELECT COALESCE(MAX(number),0)+1 FROM {table} WHERE company_id=?1 AND fiscal_year_id=?2"
    );
    tx.query_row(&sql, params![company, fy], |r| r.get(0))
        .map_err(|e| e.to_string())
}

pub(crate) fn active_context(
    tx: &rusqlite::Transaction<'_>,
    user: &str,
) -> Result<(String, String), String> {
    let company: String = tx
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "DOC-001: شرکت فعال یافت نشد".to_string())?;
    let fy:String=tx.query_row("SELECT id FROM fiscal_years WHERE company_id=?1 AND is_closed=0 ORDER BY start_date DESC LIMIT 1",params![company],|r|r.get(0)).map_err(|_|"DOC-002: سال مالی باز یافت نشد".to_string())?;
    Ok((company, fy))
}

fn invoice_total(lines: &[(String, f64, i64, i64, i64)]) -> Result<(i64, i64, i64, i64), String> {
    if lines.is_empty() {
        return Err("DOC-003: فاکتور باید حداقل یک قلم داشته باشد".into());
    }
    let mut subtotal = 0i64;
    let mut discount = 0i64;
    let mut tax = 0i64;
    for (_, q, p, d, t) in lines {
        if *q <= 0.0 || *p < 0 || *d < 0 || *t < 0 {
            return Err("DOC-004: مقدار یکی از اقلام نامعتبر است".into());
        }
        let gross = (*q * (*p as f64)).round() as i64;
        if *d > gross {
            return Err("DOC-005: تخفیف بیشتر از مبلغ قلم است".into());
        }
        subtotal += gross;
        discount += *d;
        tax += *t;
    }
    Ok((subtotal, discount, tax, subtotal - discount + tax))
}

fn create_invoice_common(
    state: &State<AppState>,
    sale: bool,
    date: String,
    contact_id: Option<String>,
    warehouse_id: Option<String>,
    lines: Vec<(String, f64, i64, i64, i64)>,
) -> Result<String, String> {
    let permission = if sale {
        "sales.invoice.create"
    } else {
        "purchase.invoice.create"
    };
    let mut c = conn(state)?;
    let user = require_permission(state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &date)?;
    let (subtotal, discount, tax, total) = invoice_total(&lines)?;
    if let Some(cid) = &contact_id {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",
                params![cid, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("DOC-006: شخص معتبر نیست".into());
        }
    }
    if let Some(wid) = &warehouse_id {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM warehouses WHERE id=?1 AND company_id=?2 AND is_active=1",
                params![wid, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("DOC-007: انبار معتبر نیست".into());
        }
    }
    let table = if sale {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let prefix = if sale { "sale" } else { "purchase" };
    let number = next_invoice_number(&tx, table, &company, &fy)?;
    let id = format!(
        "{prefix}-invoice-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let sql=format!("INSERT INTO {table}(id,company_id,fiscal_year_id,number,invoice_date,contact_id,warehouse_id,status,subtotal,discount,tax,total,created_by) VALUES(?,?,?,?,?,?,?,'draft',?,?,?,?,?)");
    tx.execute(
        &sql,
        params![
            id,
            company,
            fy,
            number,
            date,
            contact_id,
            warehouse_id,
            subtotal,
            discount,
            tax,
            total,
            user
        ],
    )
    .map_err(|e| format!("DOC-008: {e}"))?;
    let line_table = if sale {
        "sales_invoice_lines"
    } else {
        "purchase_invoice_lines"
    };
    for (i, (pid, q, p, d, t)) in lines.iter().enumerate() {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id=?1 AND company_id=?2 AND is_service=0",
                params![pid, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err(format!("DOC-009: کالا در قلم {} معتبر نیست", i + 1));
        }
        let lid = format!("{id}-line-{}", i + 1);
        let line_total = ((*q * (*p as f64)).round() as i64) - *d + *t;
        let sql=format!("INSERT INTO {line_table}(id,invoice_id,product_id,quantity,unit_price,discount,tax,line_total) VALUES(?,?,?,?,?,?,?,?)");
        tx.execute(&sql, params![lid, id, pid, q, p, d, t, line_total])
            .map_err(|e| e.to_string())?;
    }
    audit(
        &tx,
        &user,
        if sale {
            "sales.invoice.create"
        } else {
            "purchase.invoice.create"
        },
        if sale {
            "sales_invoice"
        } else {
            "purchase_invoice"
        },
        &id,
        None,
        Some(&format!("{{\"number\":{},\"total\":{}}}", number, total)),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn create_sales_invoice(
    state: State<AppState>,
    invoice_date: String,
    contact_id: Option<String>,
    warehouse_id: Option<String>,
    lines: Vec<(String, f64, i64, i64, i64)>,
) -> Result<String, String> {
    create_invoice_common(&state, true, invoice_date, contact_id, warehouse_id, lines)
}
#[tauri::command]
fn create_purchase_invoice(
    state: State<AppState>,
    invoice_date: String,
    contact_id: Option<String>,
    warehouse_id: Option<String>,
    lines: Vec<(String, f64, i64, i64, i64)>,
) -> Result<String, String> {
    create_invoice_common(&state, false, invoice_date, contact_id, warehouse_id, lines)
}

#[tauri::command]
fn list_sales_invoices(state: State<AppState>) -> Result<Vec<InvoiceSummary>, String> {
    list_invoices(&state, true)
}
#[tauri::command]
fn list_purchase_invoices(state: State<AppState>) -> Result<Vec<InvoiceSummary>, String> {
    list_invoices(&state, false)
}
fn list_invoices(state: &State<AppState>, sale: bool) -> Result<Vec<InvoiceSummary>, String> {
    let user = require_login(state)?;
    let c = conn(state)?;
    let table = if sale {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let sql=format!("SELECT d.id,d.number,d.invoice_date,d.contact_id,d.warehouse_id,d.status,d.payment_status,d.subtotal,d.discount,d.tax,d.total FROM {table} d JOIN company_users cu ON cu.company_id=d.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY d.number DESC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(InvoiceSummary {
                id: r.get(0)?,
                number: r.get(1)?,
                invoice_date: r.get(2)?,
                contact_id: r.get(3)?,
                warehouse_id: r.get(4)?,
                status: r.get(5)?,
                payment_status: r.get(6)?,
                subtotal: r.get(7)?,
                discount: r.get(8)?,
                tax: r.get(9)?,
                total: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn post_invoice(state: &State<AppState>, id: String, sale: bool) -> Result<(), String> {
    let permission = if sale {
        "sales.invoice.post"
    } else {
        "purchase.invoice.post"
    };
    let mut c = conn(state)?;
    let user = require_permission(state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let table = if sale {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let lt = if sale {
        "sales_invoice_lines"
    } else {
        "purchase_invoice_lines"
    };
    let row: (
        String,
        String,
        String,
        String,
        i64,
        Option<String>,
        Option<String>,
        i64,
    ) = {
        let sql=format!("SELECT company_id,fiscal_year_id,invoice_date,status,total,contact_id,warehouse_id,number FROM {table} WHERE id=?1");
        tx.query_row(&sql, params![id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })
        .map_err(|_| "DOC-010: فاکتور یافت نشد".to_string())?
    };
    if row.3 != "draft" {
        return Err("DOC-011: فقط فاکتور پیش‌نویس قابل ثبت است".into());
    }
    let user_company: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![row.0, user],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if user_company == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    let wid = row
        .6
        .clone()
        .ok_or("DOC-012: برای ثبت فاکتور انبار الزامی است".to_string())?;
    let mut st=tx.prepare(&format!("SELECT product_id,quantity,unit_price,line_total FROM {lt} WHERE invoice_id=?1 ORDER BY rowid")).map_err(|e|e.to_string())?;
    let mut items = Vec::new();
    let iter = st
        .query_map(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, f64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for x in iter {
        items.push(x.map_err(|e| e.to_string())?)
    }
    drop(st);
    for (pid, q, p, _lt) in &items {
        let current:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0);
        if sale && current < *q {
            return Err("DOC-013: موجودی یکی از کالاها کافی نیست".into());
        }
        let newq = if sale { current - *q } else { current + *q };
        tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![pid,wid,newq]).map_err(|e|e.to_string())?;
        let mid = format!("invoice-stock-{}-{}", id, pid);
        let typ = if sale { "issue" } else { "receipt" };
        tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![mid,row.0,pid,wid,typ,q,if sale{0}else{*p},"invoice",id,user]).map_err(|e|e.to_string())?;
    }
    let cash_acc = "acc-1101".to_string();
    let party_acc = if sale { "acc-1201" } else { "acc-2101" }.to_string();
    let main_acc = if sale { "acc-4100" } else { "acc-5100" }.to_string();
    let lines = if sale {
        vec![(party_acc.clone(), row.4, 0), (main_acc, 0, row.4)]
    } else {
        vec![(main_acc, row.4, 0), (party_acc.clone(), 0, row.4)]
    };
    let debit: i64 = lines.iter().map(|x| x.1).sum();
    let credit: i64 = lines.iter().map(|x| x.2).sum();
    if debit != credit {
        return Err("ACC-002: سند اتوماتیک نامتوازن است".into());
    }
    let journal_id = format!("journal-invoice-{id}");
    let number:i64=tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,'ثبت خودکار فاکتور', 'posted','invoice',?,?)",params![journal_id,row.0,row.1,number,row.2,id,user]).map_err(|e|e.to_string())?;
    for (i, (acc, d, cr)) in lines.iter().enumerate() {
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{journal_id}-line-{}",i+1),journal_id,acc,d,cr,"ثبت خودکار فاکتور"]).map_err(|e|e.to_string())?;
    }
    tx.execute(
        &format!("UPDATE {table} SET status='posted',journal_id=?1 WHERE id=?2"),
        params![journal_id, id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        if sale {
            "sales.invoice.post"
        } else {
            "purchase.invoice.post"
        },
        if sale {
            "sales_invoice"
        } else {
            "purchase_invoice"
        },
        &id,
        None,
        Some("{\"status\":\"posted\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn post_sales_invoice(state: State<AppState>, id: String) -> Result<(), String> {
    post_invoice(&state, id, true)
}
#[tauri::command]
fn post_purchase_invoice(state: State<AppState>, id: String) -> Result<(), String> {
    post_invoice(&state, id, false)
}

#[tauri::command]
fn settle_invoice(
    state: State<AppState>,
    invoice_id: String,
    sale: bool,
    amount: i64,
    settlement_date: String,
) -> Result<String, String> {
    if amount <= 0 {
        return Err("SET-001: مبلغ تسویه باید بیشتر از صفر باشد".into());
    }
    let permission = if sale {
        "treasury.receipt.create"
    } else {
        "treasury.payment.create"
    };
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let table = if sale {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let invoice_type = if sale { "sales" } else { "purchase" };
    let row: (String, String, String, i64, String) = {
        let sql = format!(
            "SELECT company_id,fiscal_year_id,status,total,payment_status FROM {table} WHERE id=?1"
        );
        tx.query_row(&sql, params![invoice_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(|_| "SET-002: فاکتور یافت نشد".to_string())?
    };
    if row.2 != "posted" {
        return Err("SET-003: فقط فاکتور ثبت‌شده قابل تسویه است".into());
    }
    // تاریخ تسویه باید داخل سال مالی باز فاکتور باشد؛ وگرنه سند دریافت
    // در دوره‌ای می‌نشیند که دفاترش بسته شده است.
    validate_fiscal_date(&tx, &row.1, &settlement_date)?;
    let allowed: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM company_users WHERE company_id=?1 AND user_id=?2 AND is_active=1",
            params![row.0, user],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if allowed == 0 {
        return Err("AUTH-403: دسترسی به شرکت وجود ندارد".into());
    }
    let settled:i64=tx.query_row("SELECT COALESCE(SUM(amount),0) FROM invoice_settlements WHERE invoice_id=?1 AND invoice_type=?2",params![invoice_id,invoice_type],|r|r.get(0)).unwrap_or(0);
    let remaining = row.3 - settled;
    if remaining <= 0 {
        return Err("SET-004: فاکتور قبلاً به‌طور کامل تسویه شده است".into());
    }
    if amount > remaining {
        return Err(format!(
            "SET-005: مبلغ تسویه از مانده فاکتور بیشتر است. مانده: {remaining}"
        ));
    }
    let journal_id = format!(
        "journal-settlement-{}-{}",
        invoice_id,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let number:i64=tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?;
    let description = if sale {
        "دریافت بابت فاکتور فروش"
    } else {
        "پرداخت بابت فاکتور خرید"
    };
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,?,'posted','settlement',?,?)",params![journal_id,row.0,row.1,number,settlement_date,description,invoice_id,user]).map_err(|e|e.to_string())?;
    let (a1d, a1c, a2d, a2c) = if sale {
        (amount, 0, 0, amount)
    } else {
        (0, amount, amount, 0)
    };
    let party = if sale { "acc-1201" } else { "acc-2101" };
    tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{journal_id}-cash"),journal_id,"acc-1101",a1d,a1c,description]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{journal_id}-party"),journal_id,party,a2d,a2c,description]).map_err(|e|e.to_string())?;
    let sid = format!(
        "settlement-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO invoice_settlements(id,company_id,fiscal_year_id,invoice_id,invoice_type,amount,settlement_date,journal_id,created_by) VALUES(?,?,?,?,?,?,?,?,?)",params![sid,row.0,row.1,invoice_id,invoice_type,amount,settlement_date,journal_id,user]).map_err(|e|e.to_string())?;
    let new_status = if amount == remaining {
        "paid"
    } else {
        "partial"
    };
    tx.execute(
        &format!("UPDATE {table} SET payment_status=?1 WHERE id=?2"),
        params![new_status, invoice_id],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        permission,
        if sale {
            "sales_invoice_settlement"
        } else {
            "purchase_invoice_settlement"
        },
        &invoice_id,
        None,
        Some(&format!(
            "{{\"amount\":{},\"payment_status\":\"{}\"}}",
            amount, new_status
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(sid)
}
#[derive(Serialize)]
struct TreasuryAccount {
    id: String,
    name: String,
    account_type: String,
    account_number: Option<String>,
    iban: Option<String>,
    is_active: bool,
}
#[derive(Serialize)]
struct TreasuryTransaction {
    id: String,
    transaction_type: String,
    amount: i64,
    transaction_date: String,
    description: String,
    treasury_account_id: String,
    reference_type: Option<String>,
    reference_id: Option<String>,
}
#[derive(Serialize)]
struct TreasuryStatementLine {
    id: String,
    transaction_type: String,
    amount: i64,
    transaction_date: String,
    description: String,
    running_balance: i64,
    reference_type: Option<String>,
    reference_id: Option<String>,
}
#[derive(Serialize)]
struct TreasurySummary {
    id: String,
    name: String,
    account_type: String,
    balance: i64,
    inflow: i64,
    outflow: i64,
    transaction_count: i64,
    linked_account_id: Option<String>,
}
#[derive(Serialize)]
struct CheckDashboard {
    total_received: i64,
    total_issued: i64,
    received_count: i64,
    issued_count: i64,
    due_soon_count: i64,
    overdue_count: i64,
    bounced_count: i64,
}
#[derive(Serialize)]
struct CheckSummary {
    id: String,
    check_type: String,
    check_number: String,
    party_id: Option<String>,
    amount: i64,
    issue_date: String,
    due_date: String,
    status: String,
    bank_name: Option<String>,
    treasury_account_id: Option<String>,
}

#[tauri::command]
fn list_treasury_accounts(state: State<AppState>) -> Result<Vec<TreasuryAccount>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut st=c.prepare("SELECT t.id,t.name,t.account_type,t.account_number,t.iban,t.is_active FROM treasury_accounts t JOIN company_users cu ON cu.company_id=t.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY t.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(TreasuryAccount {
                id: r.get(0)?,
                name: r.get(1)?,
                account_type: r.get(2)?,
                account_number: r.get(3)?,
                iban: r.get(4)?,
                is_active: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_treasury_account(
    state: State<AppState>,
    name: String,
    account_type: String,
    account_number: Option<String>,
    iban: Option<String>,
    linked_account_id: Option<String>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("TRE-001: نام حساب الزامی است".into());
    }
    if !["cash", "bank", "petty_cash"].contains(&account_type.as_str()) {
        return Err("TRE-002: نوع حساب خزانه نامعتبر است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.account.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _fy) = active_context(&tx, &user)?;
    if let Some(a) = &linked_account_id {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2",
                params![a, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("TRE-003: حساب حسابداری معتبر نیست".into());
        }
    }
    let id = format!(
        "treasury-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute("INSERT INTO treasury_accounts(id,company_id,name,account_type,account_number,iban,linked_account_id) VALUES(?,?,?,?,?,?,?)",params![id,company,name,account_type,account_number,iban,linked_account_id]).map_err(|e|format!("TRE-004: {e}"))?;
    audit(
        &tx,
        &user,
        "treasury.account.create",
        "treasury_account",
        &id,
        None,
        Some(&format!(
            "{{\"name\":\"{}\",\"type\":\"{}\"}}",
            name, account_type
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn update_treasury_account(
    state: State<AppState>,
    id: String,
    name: String,
    account_number: Option<String>,
    iban: Option<String>,
    linked_account_id: Option<String>,
    is_active: bool,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("TRE-005: نام حساب الزامی است".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.account.edit")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let company:String=tx.query_row("SELECT company_id FROM treasury_accounts WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",params![id,user],|r|r.get(0)).map_err(|_|"TRE-006: حساب خزانه یافت نشد".to_string())?;
    if let Some(a) = &linked_account_id {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2",
                params![a, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("TRE-003: حساب حسابداری معتبر نیست".into());
        }
    }
    let before:String=tx.query_row("SELECT json_object('name',name,'account_number',account_number,'iban',iban,'linked_account_id',linked_account_id,'is_active',is_active) FROM treasury_accounts WHERE id=?1",params![id],|r|r.get(0)).map_err(|e|e.to_string())?;
    tx.execute("UPDATE treasury_accounts SET name=?1,account_number=?2,iban=?3,linked_account_id=?4,is_active=?5 WHERE id=?6 AND company_id=?7",params![name,account_number,iban,linked_account_id,is_active as i64,id,company]).map_err(|e|format!("TRE-007: {e}"))?;
    audit(
        &tx,
        &user,
        "treasury.account.update",
        "treasury_account",
        &id,
        Some(&before),
        Some(&format!(
            "{{\"name\":\"{}\",\"is_active\":{}}}",
            name.replace('"', "'"),
            is_active
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_treasury_transactions_filtered(
    state: State<AppState>,
    treasury_account_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<TreasuryTransaction>, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.account.view")?;
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut sql=String::from("SELECT t.id,t.transaction_type,t.amount,t.transaction_date,t.description,t.treasury_account_id,t.reference_type,t.reference_id FROM treasury_transactions t WHERE t.company_id=?1 AND t.fiscal_year_id=?2");
    if treasury_account_id.is_some() {
        sql.push_str(" AND t.treasury_account_id=?3");
    }
    if from_date.is_some() {
        sql.push_str(if treasury_account_id.is_some() {
            " AND t.transaction_date>=?4"
        } else {
            " AND t.transaction_date>=?3"
        });
    }
    if to_date.is_some() {
        let n = 3 + (treasury_account_id.is_some() as usize) + (from_date.is_some() as usize);
        sql.push_str(&format!(" AND t.transaction_date<=?{}", n));
    }
    sql.push_str(" ORDER BY t.transaction_date DESC,t.created_at DESC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy];
    if let Some(ref x) = treasury_account_id {
        pv.push(x);
    }
    if let Some(ref x) = from_date {
        pv.push(x);
    }
    if let Some(ref x) = to_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            Ok(TreasuryTransaction {
                id: r.get(0)?,
                transaction_type: r.get(1)?,
                amount: r.get(2)?,
                transaction_date: r.get(3)?,
                description: r.get(4)?,
                treasury_account_id: r.get(5)?,
                reference_type: r.get(6)?,
                reference_id: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_treasury_statement(
    state: State<AppState>,
    treasury_account_id: String,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<TreasuryStatementLine>, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.account.view")?;
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let exists: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM treasury_accounts WHERE id=?1 AND company_id=?2",
            params![treasury_account_id, company],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Err("TRE-008: حساب خزانه یافت نشد".into());
    }
    let mut sql=String::from("SELECT id,transaction_type,amount,transaction_date,description,reference_type,reference_id FROM treasury_transactions WHERE company_id=?1 AND fiscal_year_id=?2 AND treasury_account_id=?3");
    if from_date.is_some() {
        sql.push_str(" AND transaction_date>=?4");
    }
    if to_date.is_some() {
        sql.push_str(if from_date.is_some() {
            " AND transaction_date<=?5"
        } else {
            " AND transaction_date<=?4"
        });
    }
    sql.push_str(" ORDER BY transaction_date ASC,created_at ASC,id ASC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy, &treasury_account_id];
    if let Some(ref x) = from_date {
        pv.push(x);
    }
    if let Some(ref x) = to_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut balance: i64 = 0;
    let mut out = Vec::new();
    for r in rows.filter_map(Result::ok) {
        let sign = if matches!(r.1.as_str(), "receipt" | "transfer_in") {
            1
        } else {
            -1
        };
        balance += sign * r.2;
        out.push(TreasuryStatementLine {
            id: r.0,
            transaction_type: r.1,
            amount: r.2,
            transaction_date: r.3,
            description: r.4,
            running_balance: balance,
            reference_type: r.5,
            reference_id: r.6,
        });
    }
    Ok(out)
}

#[tauri::command]
fn get_treasury_summary(state: State<AppState>) -> Result<Vec<TreasurySummary>, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.account.view")?;
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut st=c.prepare("SELECT t.id,t.name,t.account_type,COALESCE(SUM(CASE WHEN x.transaction_type IN ('receipt','transfer_in') THEN x.amount ELSE -x.amount END),0),COALESCE(SUM(CASE WHEN x.transaction_type IN ('receipt','transfer_in') THEN x.amount ELSE 0 END),0),COALESCE(SUM(CASE WHEN x.transaction_type IN ('payment','transfer_out') THEN x.amount ELSE 0 END),0),COUNT(x.id),t.linked_account_id FROM treasury_accounts t LEFT JOIN treasury_transactions x ON x.treasury_account_id=t.id AND x.company_id=t.company_id AND x.fiscal_year_id=?2 WHERE t.company_id=?1 GROUP BY t.id ORDER BY t.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy], |r| {
            Ok(TreasurySummary {
                id: r.get(0)?,
                name: r.get(1)?,
                account_type: r.get(2)?,
                balance: r.get(3)?,
                inflow: r.get(4)?,
                outflow: r.get(5)?,
                transaction_count: r.get(6)?,
                linked_account_id: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn jalali_date_for(date: chrono::NaiveDate) -> String {
    jalali::jalali_string(date)
}

fn current_jalali_date() -> String {
    jalali_date_for(chrono::Local::now().date_naive())
}
fn jalali_date_after_days(days: i64) -> String {
    jalali_date_for(chrono::Local::now().date_naive() + chrono::Duration::days(days))
}

#[tauri::command]
fn get_check_dashboard(state: State<AppState>) -> Result<CheckDashboard, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.check.view")?;
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let today = current_jalali_date();
    let received:i64=c.query_row("SELECT COALESCE(SUM(amount),0) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND check_type='received' AND status NOT IN ('void','memo_in_hand','memo_returned')",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let issued:i64=c.query_row("SELECT COALESCE(SUM(amount),0) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND check_type='issued' AND status NOT IN ('void','memo_in_hand','memo_returned')",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let rc:i64=c.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND check_type='received' AND status NOT IN ('void','memo_in_hand','memo_returned')",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let ic:i64=c.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND check_type='issued' AND status NOT IN ('void','memo_in_hand','memo_returned')",params![company,fy],|r|r.get(0)).unwrap_or(0);
    // بازه‌ی هشدار سررسید از تنظیمات خوانده می‌شود، نه هفته‌ی ثابت.
    let horizon = jalali_date_after_days(settings::read_integer(&c, "checks.due_soon_days"));
    let due:i64=c.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND status IN ('in_hand','deposited','endorsed','outstanding') AND due_date>=?3 AND due_date<=?4",params![company,fy,today,horizon],|r|r.get(0)).unwrap_or(0);
    let overdue:i64=c.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND status IN ('in_hand','deposited','endorsed','outstanding') AND due_date<?3",params![company,fy,today],|r|r.get(0)).unwrap_or(0);
    let bounced:i64=c.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND fiscal_year_id=?2 AND status='bounced'",params![company,fy],|r|r.get(0)).unwrap_or(0);
    Ok(CheckDashboard {
        total_received: received,
        total_issued: issued,
        received_count: rc,
        issued_count: ic,
        due_soon_count: due,
        overdue_count: overdue,
        bounced_count: bounced,
    })
}

#[tauri::command]
fn list_checks_filtered(
    state: State<AppState>,
    check_type: Option<String>,
    status: Option<String>,
    from_due_date: Option<String>,
    to_due_date: Option<String>,
) -> Result<Vec<CheckSummary>, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.check.view")?;
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut sql=String::from("SELECT k.id,k.check_type,k.check_number,k.party_id,k.amount,k.issue_date,k.due_date,k.status,k.bank_name,k.treasury_account_id FROM checks k WHERE k.company_id=?1 AND k.fiscal_year_id=?2");
    if check_type.is_some() {
        sql.push_str(" AND k.check_type=?3");
    }
    if status.is_some() {
        sql.push_str(if check_type.is_some() {
            " AND k.status=?4"
        } else {
            " AND k.status=?3"
        });
    }
    if from_due_date.is_some() {
        let n = 3 + (check_type.is_some() as usize) + (status.is_some() as usize);
        sql.push_str(&format!(" AND k.due_date>=?{}", n));
    }
    if to_due_date.is_some() {
        let n = 3
            + (check_type.is_some() as usize)
            + (status.is_some() as usize)
            + (from_due_date.is_some() as usize);
        sql.push_str(&format!(" AND k.due_date<=?{}", n));
    }
    sql.push_str(" ORDER BY k.due_date ASC,k.check_number ASC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy];
    if let Some(ref x) = check_type {
        pv.push(x);
    }
    if let Some(ref x) = status {
        pv.push(x);
    }
    if let Some(ref x) = from_due_date {
        pv.push(x);
    }
    if let Some(ref x) = to_due_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            Ok(CheckSummary {
                id: r.get(0)?,
                check_type: r.get(1)?,
                check_number: r.get(2)?,
                party_id: r.get(3)?,
                amount: r.get(4)?,
                issue_date: r.get(5)?,
                due_date: r.get(6)?,
                status: r.get(7)?,
                bank_name: r.get(8)?,
                treasury_account_id: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_treasury_transactions(state: State<AppState>) -> Result<Vec<TreasuryTransaction>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut st=c.prepare("SELECT t.id,t.transaction_type,t.amount,t.transaction_date,t.description,t.treasury_account_id,t.reference_type,t.reference_id FROM treasury_transactions t JOIN company_users cu ON cu.company_id=t.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY t.transaction_date DESC,t.created_at DESC").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(TreasuryTransaction {
                id: r.get(0)?,
                transaction_type: r.get(1)?,
                amount: r.get(2)?,
                transaction_date: r.get(3)?,
                description: r.get(4)?,
                treasury_account_id: r.get(5)?,
                reference_type: r.get(6)?,
                reference_id: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn list_checks(state: State<AppState>) -> Result<Vec<CheckSummary>, String> {
    let user = require_permission(&state, &conn(&state)?, "treasury.check.view")?;
    let c = conn(&state)?;
    let mut st=c.prepare("SELECT k.id,k.check_type,k.check_number,k.party_id,k.amount,k.issue_date,k.due_date,k.status,k.bank_name,k.treasury_account_id FROM checks k JOIN company_users cu ON cu.company_id=k.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY k.due_date").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(CheckSummary {
                id: r.get(0)?,
                check_type: r.get(1)?,
                check_number: r.get(2)?,
                party_id: r.get(3)?,
                amount: r.get(4)?,
                issue_date: r.get(5)?,
                due_date: r.get(6)?,
                status: r.get(7)?,
                bank_name: r.get(8)?,
                treasury_account_id: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn create_check(
    state: State<AppState>,
    check_type: String,
    check_number: String,
    party_id: Option<String>,
    treasury_account_id: Option<String>,
    amount: i64,
    issue_date: String,
    due_date: String,
    bank_name: Option<String>,
    description: Option<String>,
) -> Result<String, String> {
    if !["received", "issued"].contains(&check_type.as_str()) {
        return Err("CHK-001: نوع چک نامعتبر است".into());
    }
    if check_number.trim().is_empty() {
        return Err("CHK-002: شماره چک الزامی است".into());
    }
    if amount <= 0 {
        return Err("CHK-003: مبلغ چک باید بیشتر از صفر باشد".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &issue_date)?;
    validate_fiscal_date(&tx, &fy, &due_date)?;
    if issue_date > due_date {
        return Err("CHK-004: تاریخ سررسید نمی‌تواند قبل از تاریخ صدور باشد".into());
    }
    if let Some(p) = &party_id {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",
                params![p, company],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err("CHK-004: شخص معتبر نیست".into());
        }
    }
    if let Some(t) = &treasury_account_id {
        let ok:i64=tx.query_row("SELECT COUNT(*) FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![t,company],|r|r.get(0)).unwrap_or(0);
        if ok == 0 {
            return Err("CHK-005: حساب خزانه معتبر نیست".into());
        }
    }
    // شماره‌ی چک باطل‌شده آزاد می‌شود؛ بقیه‌ی وضعیت‌ها شماره را اشغال نگه می‌دارند.
    let duplicate:i64=tx.query_row("SELECT COUNT(*) FROM checks WHERE company_id=?1 AND check_type=?2 AND check_number=?3 AND status<>'void'",params![company,check_type,check_number],|r|r.get(0)).unwrap_or(0);
    if duplicate > 0 {
        return Err("CHK-006: شماره چک تکراری است".into());
    }
    let id = format!(
        "check-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    // وضعیت آغازین را نوع چک تعیین می‌کند: دریافتی «موجود»، پرداختی «پرداختی».
    let kind = if check_type == "issued" {
        CheckKind::Issued
    } else {
        CheckKind::Received
    };
    let initial_status = CheckStatus::initial(kind, false).as_str();
    tx.execute("INSERT INTO checks(id,company_id,fiscal_year_id,check_type,check_number,party_id,treasury_account_id,amount,issue_date,due_date,status,bank_name,description,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)",params![id,company,fy,check_type,check_number,party_id,treasury_account_id,amount,issue_date,due_date,initial_status,bank_name,description,user]).map_err(|e|format!("CHK-007: {e}"))?;
    audit(
        &tx,
        &user,
        "treasury.check.create",
        "check",
        &id,
        None,
        Some(&format!("{{\"status\":\"{initial_status}\"}}")),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

/// یک گذار مجاز برای یک چک، همراه با برچسب فارسی و اثر مالی آن.
#[derive(serde::Serialize)]
struct CheckTransitionOption {
    status: String,
    label: String,
    /// اثر بر خزانه: `increase`، `decrease` یا `none`.
    treasury_effect: String,
}

/// گذارهای مجاز یک چک — منبع حقیقت، ماشین حالت هسته است.
///
/// رابط کاربری هیچ فهرست وضعیتی از خودش ندارد؛ هر دکمه‌ای که نشان می‌دهد
/// حتماً در پایگاه داده هم پذیرفته می‌شود.
#[tauri::command]
fn check_transition_options(
    state: State<AppState>,
    check_id: String,
) -> Result<Vec<CheckTransitionOption>, String> {
    let c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.view")?;
    let (status, check_type): (String, String) = c
        .query_row(
            "SELECT status,check_type FROM checks WHERE id=?1 AND company_id IN \
             (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",
            params![check_id, user],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "CHK-009: چک یافت نشد".to_string())?;
    let kind = if check_type == "issued" {
        CheckKind::Issued
    } else {
        CheckKind::Received
    };
    let current = CheckStatus::parse(&status)
        .ok_or_else(|| format!("CHK-019: وضعیت ثبت‌شده‌ی «{status}» شناخته نمی‌شود"))?;
    Ok(novin_core::checks::allowed_transitions(kind, current)
        .iter()
        .map(|target| CheckTransitionOption {
            status: target.as_str().to_string(),
            label: target.label().to_string(),
            treasury_effect: match treasury_effect(kind, current, *target) {
                TreasuryEffect::Increase => "increase",
                TreasuryEffect::Decrease => "decrease",
                TreasuryEffect::None => "none",
            }
            .to_string(),
        })
        .collect())
}

#[tauri::command]
fn update_check_status(
    state: State<AppState>,
    check_id: String,
    new_status: String,
) -> Result<(), String> {
    let target = CheckStatus::parse(&new_status)
        .ok_or_else(|| "CHK-008: وضعیت چک نامعتبر است".to_string())?;
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.update")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let row:(String,String,String,i64,Option<String>,Option<String>,String,String,Option<String>)=tx.query_row(
        "SELECT status,check_type,company_id,amount,treasury_account_id,clearing_journal_id,fiscal_year_id,due_date,party_id FROM checks WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",params![check_id,user],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))
    ).map_err(|_|"CHK-009: چک یافت نشد".to_string())?;
    let old = &row.0;
    // ماشین حالت چک تنها یک منبع حقیقت دارد: هسته‌ی مالی.
    let kind = if row.1 == "issued" {
        CheckKind::Issued
    } else {
        CheckKind::Received
    };
    let current = CheckStatus::parse(old)
        .ok_or_else(|| format!("CHK-019: وضعیت ثبت‌شده‌ی «{old}» شناخته نمی‌شود"))?;
    check_transition(kind, current, target).map_err(|_| {
        format!(
            "CHK-010: انتقال وضعیت «{}» به «{}» مجاز نیست",
            current.label(),
            target.label()
        )
    })?;
    validate_fiscal_date(&tx, &row.6, &row.7)?;
    // اثر خزانه‌ای گذار را هم هسته تعیین می‌کند، نه شرط‌های پراکنده.
    let effect = treasury_effect(kind, current, target);
    let settles =
        matches!(effect, TreasuryEffect::Increase | TreasuryEffect::Decrease) && row.5.is_none();
    let reverses = target == CheckStatus::Bounced && row.5.is_some();
    if settles {
        let treasury_id = row
            .4
            .as_ref()
            .ok_or("CHK-012: برای وصول چک باید حساب خزانه مشخص باشد".to_string())?;
        let treasury_account:Option<String>=tx.query_row("SELECT linked_account_id FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![treasury_id,row.2],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        let treasury_account = treasury_account
            .ok_or_else(|| "CHK-013: حساب خزانه به حسابداری متصل نیست".to_string())?;
        let offset_account = if row.1 == "received" {
            "acc-1201"
        } else {
            "acc-2101"
        };
        let (debit, credit) = if row.1 == "received" {
            (treasury_account.as_str(), offset_account)
        } else {
            (offset_account, treasury_account.as_str())
        };
        let jid = format!("journal-check-clear-{}", check_id);
        let n = next_journal_number(&tx, &row.2, &row.6)?;
        let desc = if row.1 == "received" {
            "وصول چک دریافتی"
        } else {
            "وصول چک پرداختی"
        };
        tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,?, 'posted','check_clear',?,?)",params![jid,row.2,row.6,n,row.7,desc,check_id,user]).map_err(|e|format!("CHK-014: {e}"))?;
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-debit"),jid,debit,row.3,0,desc]).map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-credit"),jid,credit,0,row.3,desc]).map_err(|e|e.to_string())?;
        let tid = format!("treasury-check-clear-{}", check_id);
        let typ = if row.1 == "received" {
            "receipt"
        } else {
            "payment"
        };
        tx.execute("INSERT INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,transaction_type,amount,transaction_date,description,reference_type,reference_id,journal_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![tid,row.2,row.6,treasury_id,typ,row.3,row.7,desc,"check",check_id,jid,user]).map_err(|e|format!("CHK-015: {e}"))?;
        tx.execute(
            "UPDATE checks SET clearing_journal_id=?1,status=?2 WHERE id=?3",
            params![jid, target.as_str(), check_id],
        )
        .map_err(|e| e.to_string())?;
    } else if reverses {
        let original = row.5.ok_or("CHK-016: سند وصول چک یافت نشد".to_string())?;
        let mut st=tx.prepare("SELECT account_id,debit,credit FROM journal_lines WHERE journal_id=?1 ORDER BY rowid").map_err(|e|e.to_string())?;
        let lines: Vec<(String, i64, i64)> = st
            .query_map(params![original], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|(a, d, cr)| (a, cr, d))
            .collect();
        drop(st);
        let jid = format!("journal-check-bounce-{}", check_id);
        let n = next_journal_number(&tx, &row.2, &row.6)?;
        tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,?, 'posted','check_bounce',?,?)",params![jid,row.2,row.6,n,row.7,"برگشت چک",check_id,user]).map_err(|e|format!("CHK-017: {e}"))?;
        for (i, (acc, d, cr)) in lines.iter().enumerate() {
            tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-line-{i}"),jid,acc,d,cr,"برگشت چک"]).map_err(|e|e.to_string())?;
        }
        if let Some(treasury_id) = &row.4 {
            let tid = format!("treasury-check-bounce-{}", check_id);
            let typ = if row.1 == "received" {
                "payment"
            } else {
                "receipt"
            };
            tx.execute("INSERT INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,transaction_type,amount,transaction_date,description,reference_type,reference_id,journal_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![tid,row.2,row.6,treasury_id,typ,row.3,row.7,"برگشت چک","check",check_id,jid,user]).map_err(|e|format!("CHK-018: {e}"))?;
        }
        tx.execute(
            "UPDATE checks SET status='bounced',clearing_journal_id=NULL WHERE id=?1",
            params![check_id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "UPDATE checks SET status=?1 WHERE id=?2",
            params![target.as_str(), check_id],
        )
        .map_err(|e| e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "treasury.check.update",
        "check",
        &check_id,
        Some(&format!("{{\"status\":\"{}\"}}", old)),
        Some(&format!("{{\"status\":\"{}\"}}", new_status)),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn create_return_common(
    state: &State<AppState>,
    sale: bool,
    original_invoice_id: String,
    return_date: String,
    lines: Vec<(String, f64, i64)>,
) -> Result<String, String> {
    let permission = if sale {
        "sales.return.create"
    } else {
        "purchase.return.create"
    };
    let mut c = conn(state)?;
    let user = require_permission(state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let table = if sale {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let line_table = if sale {
        "sales_invoice_lines"
    } else {
        "purchase_invoice_lines"
    };
    let return_table = if sale {
        "sales_returns"
    } else {
        "purchase_returns"
    };
    let return_line = if sale {
        "sales_return_lines"
    } else {
        "purchase_return_lines"
    };
    let row: (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    ) = {
        let sql=format!("SELECT company_id,fiscal_year_id,status,contact_id,warehouse_id,payment_status FROM {table} WHERE id=?1");
        tx.query_row(&sql, params![original_invoice_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(|_| "RET-001: فاکتور اصلی یافت نشد".to_string())?
    };
    if row.2 != "posted" {
        return Err("RET-002: فقط فاکتور ثبت‌شده قابل برگشت است".into());
    }
    let wid = row
        .4
        .clone()
        .ok_or("RET-003: فاکتور اصلی انبار ندارد".to_string())?;
    let mut total = 0i64;
    for (pid, q, p) in &lines {
        if *q <= 0.0 {
            return Err("RET-004: مقدار برگشتی نامعتبر است".into());
        }
        let original:f64=tx.query_row(&format!("SELECT COALESCE(SUM(quantity),0) FROM {line_table} WHERE invoice_id=?1 AND product_id=?2"),params![original_invoice_id,pid],|r|r.get(0)).unwrap_or(0.0);
        let returned:f64=tx.query_row(&format!("SELECT COALESCE(SUM(rl.quantity),0) FROM {return_line} rl JOIN {return_table} rh ON rh.id=rl.return_id WHERE rh.original_invoice_id=?1 AND rl.product_id=?2 AND rh.status='posted'"),params![original_invoice_id,pid],|r|r.get(0)).unwrap_or(0.0);
        if *q > original - returned {
            return Err("RET-005: مقدار برگشتی بیشتر از مقدار قابل برگشت است".into());
        }
        total += (*q * (*p as f64)).round() as i64;
    }
    let prefix = if sale {
        "sales-return"
    } else {
        "purchase-return"
    };
    let number:i64=tx.query_row(&format!("SELECT COALESCE(MAX(number),0)+1 FROM {return_table} WHERE company_id=?1 AND fiscal_year_id=?2"),params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?;
    let id = format!(
        "{prefix}-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute(&format!("INSERT INTO {return_table}(id,company_id,fiscal_year_id,number,return_date,original_invoice_id,contact_id,warehouse_id,status,total,created_by) VALUES(?,?,?,?,?,?,?,?, 'draft',?,?)"),params![id,row.0,row.1,number,return_date,original_invoice_id,row.3,row.4,total,user]).map_err(|e|e.to_string())?;
    for (i, (pid, q, p)) in lines.iter().enumerate() {
        tx.execute(&format!("INSERT INTO {return_line}(id,return_id,product_id,quantity,unit_price,line_total) VALUES(?,?,?,?,?,?)"),params![format!("{id}-line-{}",i+1),id,pid,q,p,(*q*(*p as f64)).round() as i64]).map_err(|e|e.to_string())?;
    }
    audit(
        &tx,
        &user,
        permission,
        if sale {
            "sales_return"
        } else {
            "purchase_return"
        },
        &id,
        None,
        Some(&format!("{{\"total\":{}}}", total)),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn create_sales_return(
    state: State<AppState>,
    original_invoice_id: String,
    return_date: String,
    lines: Vec<(String, f64, i64)>,
) -> Result<String, String> {
    create_return_common(&state, true, original_invoice_id, return_date, lines)
}
#[tauri::command]
fn create_purchase_return(
    state: State<AppState>,
    original_invoice_id: String,
    return_date: String,
    lines: Vec<(String, f64, i64)>,
) -> Result<String, String> {
    create_return_common(&state, false, original_invoice_id, return_date, lines)
}

fn post_return(state: &State<AppState>, return_id: String, sale: bool) -> Result<(), String> {
    let permission = if sale {
        "sales.return.post"
    } else {
        "purchase.return.post"
    };
    let mut c = conn(state)?;
    let user = require_permission(state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let rt = if sale {
        "sales_returns"
    } else {
        "purchase_returns"
    };
    let rl = if sale {
        "sales_return_lines"
    } else {
        "purchase_return_lines"
    };
    let row:(String,String,String,String,Option<String>,i64)=tx.query_row(&format!("SELECT company_id,fiscal_year_id,status,warehouse_id,journal_id,total FROM {rt} WHERE id=?1"),params![return_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(|_|"RET-006: برگشت یافت نشد".to_string())?;
    if row.2 != "draft" {
        return Err("RET-007: فقط برگشت پیش‌نویس قابل ثبت است".into());
    }
    let wid = row.3.clone();
    if wid.trim().is_empty() {
        return Err("RET-008: انبار برگشت مشخص نیست".to_string());
    }
    let mut st = tx
        .prepare(&format!(
            "SELECT product_id,quantity,unit_price FROM {rl} WHERE return_id=?1"
        ))
        .map_err(|e| e.to_string())?;
    let items: Vec<(String, f64, i64)> = st
        .query_map(params![return_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    drop(st);
    for (pid, q, p) in &items {
        let current:f64=tx.query_row("SELECT COALESCE(quantity,0) FROM inventory_balances WHERE product_id=?1 AND warehouse_id=?2",params![pid,wid],|r|r.get(0)).unwrap_or(0.0);
        if !sale && current < *q {
            return Err("RET-009: موجودی برای برگشت خرید کافی نیست".into());
        }
        let newq = if sale { current + *q } else { current - *q };
        tx.execute("INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?,?,?) ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![pid,wid,newq]).map_err(|e|e.to_string())?;
        let typ = if sale { "receipt" } else { "issue" };
        tx.execute("INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?)",params![format!("return-stock-{}-{}",return_id,pid),row.0,pid,wid,typ,q,p,"invoice_return",return_id,user]).map_err(|e|e.to_string())?;
    }
    let jid = format!("journal-return-{return_id}");
    let n:i64=tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![row.0,row.1],|r|r.get(0)).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,?,'ثبت خودکار برگشت فاکتور','posted','invoice_return',?,?)",params![jid,row.0,row.1,n,chrono::Utc::now().format("%Y/%m/%d").to_string(),return_id,user]).map_err(|e|e.to_string())?;
    let (a, b) = if sale {
        ("acc-4200", "acc-1201")
    } else {
        ("acc-2101", "acc-5200")
    };
    let lines = if sale {
        vec![(a, row.5, 0), (b, 0, row.5)]
    } else {
        vec![(a, row.5, 0), (b, 0, row.5)]
    };
    for (i, (acc, d, cr)) in lines.iter().enumerate() {
        tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{jid}-line-{}",i+1),jid,acc,d,cr,"ثبت خودکار برگشت فاکتور"]).map_err(|e|e.to_string())?;
    }
    tx.execute(
        &format!("UPDATE {rt} SET status='posted',journal_id=?1 WHERE id=?2"),
        params![jid, return_id],
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
        &return_id,
        Some("{\"status\":\"draft\"}"),
        Some("{\"status\":\"posted\"}"),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
fn post_sales_return(state: State<AppState>, id: String) -> Result<(), String> {
    post_return(&state, id, true)
}
#[tauri::command]
fn post_purchase_return(state: State<AppState>, id: String) -> Result<(), String> {
    post_return(&state, id, false)
}

#[derive(Serialize)]
struct AccountBalance {
    id: String,
    code: String,
    name: String,
    debit: i64,
    credit: i64,
    balance: i64,
    nature: String,
}
#[derive(Serialize)]
struct TrialBalance {
    total_debit: i64,
    total_credit: i64,
    accounts: Vec<AccountBalance>,
}
#[derive(Serialize)]
struct TreasuryBalance {
    id: String,
    name: String,
    account_type: String,
    balance: i64,
    linked_account_id: Option<String>,
}
#[derive(Serialize)]
struct LedgerLine {
    date: String,
    journal_number: i64,
    journal_id: String,
    description: String,
    account_id: String,
    debit: i64,
    credit: i64,
    running_balance: i64,
}
#[derive(Serialize)]
struct PartyBalance {
    contact_id: String,
    contact_name: String,
    invoice_count: i64,
    invoiced: i64,
    settled: i64,
    remaining: i64,
}
#[derive(Serialize)]
struct CashPosition {
    total: i64,
    accounts: Vec<TreasuryBalance>,
}
#[derive(Serialize)]
struct PeriodStatus {
    id: String,
    title: String,
    start_date: String,
    end_date: String,
    is_closed: bool,
    draft_journals: i64,
    posted_journals: i64,
}

pub(crate) fn validate_fiscal_date(
    tx: &rusqlite::Transaction<'_>,
    fy: &str,
    date: &str,
) -> Result<(), String> {
    let ok:i64=tx.query_row("SELECT COUNT(*) FROM fiscal_years WHERE id=?1 AND is_closed=0 AND ?2 BETWEEN start_date AND end_date",params![fy,date],|r|r.get(0)).map_err(|e|e.to_string())?;
    if ok == 0 {
        return Err("FY-001: تاریخ خارج از سال مالی باز است".into());
    }
    Ok(())
}

pub(crate) fn next_journal_number(
    tx: &rusqlite::Transaction<'_>,
    company: &str,
    fy: &str,
) -> Result<i64, String> {
    tx.query_row("SELECT COALESCE(MAX(number),0)+1 FROM journal_entries WHERE company_id=?1 AND fiscal_year_id=?2",params![company,fy],|r|r.get(0)).map_err(|e|e.to_string())
}

fn create_treasury_journal(
    tx: &rusqlite::Transaction<'_>,
    company: &str,
    fy: &str,
    date: &str,
    description: &str,
    debit_account: &str,
    credit_account: &str,
    amount: i64,
    user: &str,
    source_type: &str,
    source_id: &str,
) -> Result<String, String> {
    if amount <= 0 {
        return Err("TRE-101: مبلغ باید بیشتر از صفر باشد".into());
    }
    for acc in [debit_account, credit_account] {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2 AND is_active=1",
                params![acc, company],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if ok == 0 {
            return Err("TRE-102: حساب معین معتبر نیست".into());
        }
    }
    let id = format!(
        "journal-treasury-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let n = next_journal_number(tx, company, fy)?;
    tx.execute("INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES(?,?,?,?,? ,?,'posted',?,?,?)",params![id,company,fy,n,date,description,source_type,source_id,user]).map_err(|e|format!("TRE-103: {e}"))?;
    tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{id}-d"),id,debit_account,amount,0,description]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO journal_lines(id,journal_id,account_id,debit,credit,description) VALUES(?,?,?,?,?,?)",params![format!("{id}-c"),id,credit_account,0,amount,description]).map_err(|e|e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn create_treasury_transaction(
    state: State<AppState>,
    transaction_type: String,
    treasury_account_id: String,
    offset_account_id: String,
    amount: i64,
    transaction_date: String,
    description: String,
) -> Result<String, String> {
    if !["receipt", "payment"].contains(&transaction_type.as_str()) {
        return Err("TRE-001: نوع تراکنش خزانه نامعتبر است".into());
    }
    if amount <= 0 {
        return Err("TRE-002: مبلغ باید بیشتر از صفر باشد".into());
    }
    if description.trim().is_empty() {
        return Err("TRE-003: شرح تراکنش الزامی است".into());
    }
    let permission = if transaction_type == "receipt" {
        "treasury.receipt.create"
    } else {
        "treasury.payment.create"
    };
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &transaction_date)?;
    let linked:Option<String>=tx.query_row("SELECT linked_account_id FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![treasury_account_id,company],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let treasury_gl =
        linked.ok_or_else(|| "TRE-004: حساب خزانه معتبر یا متصل به حسابداری نیست".to_string())?;
    let id = format!(
        "treasury-tx-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let journal = if transaction_type == "receipt" {
        create_treasury_journal(
            &tx,
            &company,
            &fy,
            &transaction_date,
            &description,
            &treasury_gl,
            &offset_account_id,
            amount,
            &user,
            "treasury_receipt",
            &id,
        )
    } else {
        create_treasury_journal(
            &tx,
            &company,
            &fy,
            &transaction_date,
            &description,
            &offset_account_id,
            &treasury_gl,
            amount,
            &user,
            "treasury_payment",
            &id,
        )
    }?;
    tx.execute("INSERT INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,transaction_type,amount,transaction_date,description,reference_type,journal_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![id,company,fy,treasury_account_id,transaction_type,amount,transaction_date,description,"manual",journal,user]).map_err(|e|format!("TRE-005: {e}"))?;
    audit(
        &tx,
        &user,
        "treasury.transaction.create",
        "treasury_transaction",
        &id,
        None,
        Some(&format!(
            "{{\"amount\":{},\"type\":\"{}\"}}",
            amount, transaction_type
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn create_treasury_transfer(
    state: State<AppState>,
    from_account_id: String,
    to_account_id: String,
    amount: i64,
    transaction_date: String,
    description: String,
) -> Result<String, String> {
    if amount <= 0 {
        return Err("TRE-006: مبلغ باید بیشتر از صفر باشد".into());
    }
    if from_account_id == to_account_id {
        return Err("TRE-007: حساب مبدأ و مقصد نباید یکسان باشد".into());
    }
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.payment.create")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &transaction_date)?;
    let from:Option<String>=tx.query_row("SELECT linked_account_id FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![from_account_id,company],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let to:Option<String>=tx.query_row("SELECT linked_account_id FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",params![to_account_id,company],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let from = from.ok_or("TRE-008: حساب مبدأ معتبر یا متصل نیست")?;
    let to = to.ok_or("TRE-009: حساب مقصد معتبر یا متصل نیست")?;
    let source = format!(
        "transfer-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let journal = create_treasury_journal(
        &tx,
        &company,
        &fy,
        &transaction_date,
        &description,
        &to,
        &from,
        amount,
        &user,
        "treasury_transfer",
        &source,
    )?;
    let out_id = format!("{source}-out");
    let in_id = format!("{source}-in");
    for (id, account, typ) in [
        (&out_id, &from, "transfer_out"),
        (&in_id, &to, "transfer_in"),
    ] {
        tx.execute("INSERT INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,transaction_type,amount,transaction_date,description,reference_type,reference_id,journal_id,created_by) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![id,company,fy,if typ=="transfer_out"{&from_account_id}else{&to_account_id},typ,amount,transaction_date,description,"treasury_transfer",source,journal,user]).map_err(|e|e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "treasury.transfer.create",
        "treasury_transfer",
        &source,
        None,
        Some(&format!(
            "{{\"amount\":{},\"from\":\"{}\",\"to\":\"{}\"}}",
            amount, from_account_id, to_account_id
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(source)
}

#[tauri::command]
fn list_treasury_balances(state: State<AppState>) -> Result<Vec<TreasuryBalance>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut st=c.prepare("SELECT t.id,t.name,t.account_type,COALESCE(SUM(CASE WHEN x.transaction_type IN ('receipt','transfer_in') THEN x.amount ELSE -x.amount END),0),t.linked_account_id FROM treasury_accounts t JOIN company_users cu ON cu.company_id=t.company_id LEFT JOIN treasury_transactions x ON x.treasury_account_id=t.id WHERE cu.user_id=?1 AND cu.is_active=1 AND t.is_active=1 GROUP BY t.id ORDER BY t.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(TreasuryBalance {
                id: r.get(0)?,
                name: r.get(1)?,
                account_type: r.get(2)?,
                balance: r.get(3)?,
                linked_account_id: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_trial_balance(state: State<AppState>) -> Result<TrialBalance, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut st=c.prepare("SELECT a.id,a.code,a.name,a.nature,COALESCE(SUM(l.debit),0),COALESCE(SUM(l.credit),0) FROM accounts a LEFT JOIN journal_lines l ON l.account_id=a.id LEFT JOIN journal_entries j ON j.id=l.journal_id AND j.status='posted' AND j.company_id=?1 AND j.fiscal_year_id=?2 WHERE a.company_id=?1 AND a.is_active=1 GROUP BY a.id ORDER BY a.code").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy], |r| {
            let d: i64 = r.get(4)?;
            let cr: i64 = r.get(5)?;
            Ok(AccountBalance {
                id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                nature: r.get(3)?,
                debit: d,
                credit: cr,
                balance: d - cr,
            })
        })
        .map_err(|e| e.to_string())?;
    let accounts: Vec<_> = rows.filter_map(Result::ok).collect();
    Ok(TrialBalance {
        total_debit: accounts.iter().map(|x| x.debit).sum(),
        total_credit: accounts.iter().map(|x| x.credit).sum(),
        accounts,
    })
}

#[tauri::command]
fn get_account_ledger(
    state: State<AppState>,
    account_id: String,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<LedgerLine>, String> {
    let user = require_permission(&state, &conn(&state)?, "reporting.view")?;
    let mut c = conn(&state)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    let ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2 AND is_active=1",
            params![account_id, company],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if ok == 0 {
        return Err("RPT-001: حساب معتبر نیست".into());
    }
    let from = from_date.unwrap_or_else(|| {
        tx.query_row(
            "SELECT start_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .unwrap_or_default()
    });
    let to = to_date.unwrap_or_else(|| {
        tx.query_row(
            "SELECT end_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .unwrap_or_default()
    });
    validate_fiscal_date(&tx, &fy, &from)?;
    validate_fiscal_date(&tx, &fy, &to)?;
    if from > to {
        return Err("RPT-002: بازه تاریخ نامعتبر است".into());
    }
    let opening:i64=tx.query_row("SELECT COALESCE(SUM(l.debit-l.credit),0) FROM journal_lines l JOIN journal_entries j ON j.id=l.journal_id WHERE l.account_id=?1 AND j.company_id=?2 AND j.fiscal_year_id=?3 AND j.status='posted' AND j.entry_date < ?4",params![account_id,company,fy,from],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut st=tx.prepare("SELECT j.entry_date,j.number,j.id,j.description,l.debit,l.credit FROM journal_lines l JOIN journal_entries j ON j.id=l.journal_id WHERE l.account_id=?1 AND j.company_id=?2 AND j.fiscal_year_id=?3 AND j.status='posted' AND j.entry_date BETWEEN ?4 AND ?5 ORDER BY j.entry_date,j.number,rowid").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![account_id, company, fy, from, to], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut running = opening;
    let mut out = Vec::new();
    for row in rows.filter_map(Result::ok) {
        running += row.4 - row.5;
        out.push(LedgerLine {
            date: row.0,
            journal_number: row.1,
            journal_id: row.2,
            description: row.3,
            account_id: account_id.clone(),
            debit: row.4,
            credit: row.5,
            running_balance: running,
        });
    }
    Ok(out)
}

#[tauri::command]
fn get_receivables(state: State<AppState>) -> Result<Vec<PartyBalance>, String> {
    get_party_balances(&state, true)
}
#[tauri::command]
fn get_payables(state: State<AppState>) -> Result<Vec<PartyBalance>, String> {
    get_party_balances(&state, false)
}
fn get_party_balances(state: &State<AppState>, sales: bool) -> Result<Vec<PartyBalance>, String> {
    let user = require_permission(state, &conn(state)?, "reporting.view")?;
    let c = conn(state)?;
    let table = if sales {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let inv_type = if sales { "sales" } else { "purchase" };
    let sql=format!("SELECT c.id,c.name,COUNT(i.id),COALESCE(SUM(i.total),0),COALESCE(SUM(COALESCE(s.settled,0)),0) FROM contacts c JOIN company_users cu ON cu.company_id=c.company_id LEFT JOIN {table} i ON i.contact_id=c.id AND i.status='posted' LEFT JOIN (SELECT invoice_id,SUM(amount) AS settled FROM invoice_settlements WHERE invoice_type='{inv_type}' GROUP BY invoice_id) s ON s.invoice_id=i.id WHERE cu.user_id=?1 AND cu.is_active=1 GROUP BY c.id,c.name HAVING COALESCE(SUM(i.total),0)-COALESCE(SUM(COALESCE(s.settled,0)),0)>0 ORDER BY (COALESCE(SUM(i.total),0)-COALESCE(SUM(COALESCE(s.settled,0)),0)) DESC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            let invoiced: i64 = r.get(3)?;
            let settled: i64 = r.get(4)?;
            Ok(PartyBalance {
                contact_id: r.get(0)?,
                contact_name: r.get(1)?,
                invoice_count: r.get(2)?,
                invoiced,
                settled,
                remaining: invoiced - settled,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows
        .filter_map(Result::ok)
        .filter(|x| x.remaining > 0)
        .collect())
}

#[tauri::command]
fn get_cash_position(state: State<AppState>) -> Result<CashPosition, String> {
    let accounts = list_treasury_balances(state)?;
    Ok(CashPosition {
        total: accounts.iter().map(|x| x.balance).sum(),
        accounts,
    })
}

#[tauri::command]
fn get_fiscal_period_status(state: State<AppState>) -> Result<PeriodStatus, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    let mut st=c.prepare("SELECT f.id,f.title,f.start_date,f.end_date,f.is_closed,(SELECT COUNT(*) FROM journal_entries j WHERE j.fiscal_year_id=f.id AND j.status='draft'),(SELECT COUNT(*) FROM journal_entries j WHERE j.fiscal_year_id=f.id AND j.status='posted') FROM fiscal_years f JOIN company_users cu ON cu.company_id=f.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY f.start_date DESC").map_err(|e|e.to_string())?;
    st.query_row(params![user], |r| {
        Ok(PeriodStatus {
            id: r.get(0)?,
            title: r.get(1)?,
            start_date: r.get(2)?,
            end_date: r.get(3)?,
            is_closed: r.get::<_, i64>(4)? != 0,
            draft_journals: r.get(5)?,
            posted_journals: r.get(6)?,
        })
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn close_fiscal_year(state: State<AppState>) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "accounting.period.close")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    let drafts: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM journal_entries WHERE fiscal_year_id=?1 AND status='draft'",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if drafts > 0 {
        return Err("FY-002: قبل از بستن سال مالی، همه اسناد پیش‌نویس را تعیین تکلیف کنید".into());
    }
    let (debit,credit):(i64,i64)=tx.query_row("SELECT COALESCE(SUM(l.debit),0),COALESCE(SUM(l.credit),0) FROM journal_lines l JOIN journal_entries j ON j.id=l.journal_id WHERE j.fiscal_year_id=?1 AND j.status='posted'",params![fy],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    if debit != credit {
        return Err("FY-003: سال مالی نامتوازن است".into());
    }
    tx.execute(
        "UPDATE fiscal_years SET is_closed=1 WHERE id=?1 AND company_id=?2",
        params![fy, company],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "accounting.period.close",
        "fiscal_year",
        &fy,
        None,
        Some("{\"closed\":true}"),
    )?;
    tx.commit().map_err(|e| e.to_string())
}

#[tauri::command]
fn verify_backup_file(state: State<AppState>, name: String) -> Result<String, String> {
    let dir = backup_dir(&state)?;
    let path = dir.join(&name);
    if !path.exists() {
        return Err("BACKUP-007: فایل پشتیبان یافت نشد".into());
    }
    let c = Connection::open(&path).map_err(|e| format!("BACKUP-008: {e}"))?;
    let integrity: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err("BACKUP-009: integrity check ناموفق است".into());
    }
    let mut fk = c
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?;
    let mut rows = fk.query([]).map_err(|e| e.to_string())?;
    if rows.next().map_err(|e| e.to_string())?.is_some() {
        return Err("BACKUP-010: foreign key check ناموفق است".into());
    }
    Ok("Backup verified".into())
}

#[derive(Serialize)]
struct DashboardKpi {
    sales: i64,
    purchases: i64,
    gross_profit: i64,
    receivables: i64,
    payables: i64,
    cash: i64,
    inventory_value: i64,
    low_stock_count: i64,
}
#[derive(Serialize)]
struct SalesTrend {
    period: String,
    sales: i64,
    purchases: i64,
}
#[derive(Serialize)]
struct TopProduct {
    product_id: String,
    name: String,
    quantity: f64,
    revenue: i64,
}
#[derive(Serialize)]
struct LowStock {
    product_id: String,
    name: String,
    quantity: f64,
    min_stock: f64,
    warehouse_count: i64,
}
#[derive(Serialize)]
struct RecentInvoice {
    id: String,
    number: i64,
    invoice_date: String,
    contact_name: Option<String>,
    total: i64,
    payment_status: String,
    invoice_type: String,
}
#[derive(Serialize)]
struct ReportLine {
    code: String,
    name: String,
    amount: i64,
    nature: String,
}
#[derive(Serialize)]
struct FinancialStatement {
    title: String,
    as_of: String,
    lines: Vec<ReportLine>,
    total: i64,
}
#[derive(Serialize)]
struct JournalBookLine {
    date: String,
    number: i64,
    description: String,
    account_code: String,
    account_name: String,
    debit: i64,
    credit: i64,
}
#[derive(Serialize)]
struct PartyAging {
    contact_id: String,
    contact_name: String,
    current: i64,
    days_1_30: i64,
    days_31_60: i64,
    days_61_90: i64,
    over_90: i64,
    total: i64,
}
#[derive(Serialize)]
struct ProfitLoss {
    revenue: i64,
    sales_returns: i64,
    net_revenue: i64,
    cogs: i64,
    gross_profit: i64,
    gross_margin_percent: f64,
}

fn active_company(state: &State<AppState>, c: &Connection) -> Result<(String, String), String> {
    let user = require_login(state)?;
    c.query_row("SELECT company_id, (SELECT id FROM fiscal_years fy WHERE fy.company_id=cu.company_id AND fy.is_closed=0 ORDER BY fy.start_date DESC LIMIT 1) FROM company_users cu WHERE cu.user_id=?1 AND cu.is_active=1 LIMIT 1",params![user],|r|Ok((r.get(0)?,r.get::<_,String>(1)?))).map_err(|_|"REPORT-001: شرکت یا سال مالی فعال یافت نشد".to_string())
}

#[tauri::command]
fn get_dashboard_kpis(state: State<AppState>) -> Result<DashboardKpi, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let sales:i64=c.query_row("SELECT COALESCE(SUM(total),0) FROM sales_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted'",params![company,fy],|r|r.get(0)).map_err(|e|e.to_string())?;
    let purchases:i64=c.query_row("SELECT COALESCE(SUM(total),0) FROM purchase_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted'",params![company,fy],|r|r.get(0)).map_err(|e|e.to_string())?;
    let gross_profit:i64=c.query_row("SELECT COALESCE(SUM(CASE WHEN je.source_type='sales_invoice' THEN jl.credit-jl.debit ELSE 0 END),0) - COALESCE(SUM(CASE WHEN je.source_type='purchase_invoice' THEN jl.debit-jl.credit ELSE 0 END),0) FROM journal_entries je JOIN journal_lines jl ON jl.journal_id=je.id WHERE je.company_id=?1 AND je.fiscal_year_id=?2 AND je.status='posted' AND jl.account_id IN (SELECT id FROM accounts WHERE code IN ('4100','5100'))",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let receivables = get_party_balances_for_company(&c, &company, true)?;
    let payables = get_party_balances_for_company(&c, &company, false)?;
    let cash:i64=c.query_row("SELECT COALESCE(SUM(CASE WHEN tt.transaction_type='receipt' THEN tt.amount WHEN tt.transaction_type='payment' THEN -tt.amount ELSE 0 END),0) FROM treasury_transactions tt WHERE tt.company_id=?1 AND tt.fiscal_year_id=?2",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let inventory_value:f64=c.query_row("SELECT COALESCE(SUM(ib.quantity*p.purchase_price),0) FROM inventory_balances ib JOIN products p ON p.id=ib.product_id WHERE p.company_id=?1",params![company],|r|r.get(0)).unwrap_or(0.0);
    let low_stock_count:i64=c.query_row("SELECT COUNT(*) FROM (SELECT p.id FROM products p LEFT JOIN inventory_balances ib ON ib.product_id=p.id WHERE p.company_id=?1 GROUP BY p.id HAVING COALESCE(SUM(ib.quantity),0)<=p.min_stock)",params![company],|r|r.get(0)).unwrap_or(0);
    Ok(DashboardKpi {
        sales,
        purchases,
        gross_profit,
        receivables: receivables.iter().map(|x| x.5).sum(),
        payables: payables.iter().map(|x| x.5).sum(),
        cash,
        inventory_value: inventory_value.round() as i64,
        low_stock_count,
    })
}

fn get_party_balances_for_company(
    c: &Connection,
    company: &str,
    sales: bool,
) -> Result<Vec<(String, String, i64, i64, i64, i64)>, String> {
    let (table, kind) = if sales {
        ("sales_invoices", "is_customer")
    } else {
        ("purchase_invoices", "is_supplier")
    };
    let sql=format!("SELECT x.contact_id,COALESCE(c.name,'بدون شخص'),COUNT(x.id),COALESCE(SUM(x.total),0),COALESCE(SUM(CASE WHEN x.payment_status='paid' THEN x.total WHEN x.payment_status='partial' THEN COALESCE((SELECT SUM(s.amount) FROM invoice_settlements s WHERE s.invoice_id=x.id AND s.invoice_type=CASE WHEN ?2='sale' THEN 'sales' ELSE 'purchase' END),0) ELSE 0 END),0) FROM {table} x LEFT JOIN contacts c ON c.id=x.contact_id WHERE x.company_id=?1 AND x.status='posted' AND c.{kind}=1 GROUP BY x.contact_id,c.name");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let invoice_type = if sales { "sale" } else { "purchase" };
    let rows = st
        .query_map(params![company, invoice_type], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get::<_, i64>(3)? - r.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_sales_trend(state: State<AppState>) -> Result<Vec<SalesTrend>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut out = Vec::new();
    let mut st=c.prepare("SELECT substr(invoice_date,1,7) p, COALESCE(SUM(CASE WHEN typ='sale' THEN total ELSE 0 END),0), COALESCE(SUM(CASE WHEN typ='purchase' THEN total ELSE 0 END),0) FROM (SELECT invoice_date,total,'sale' typ FROM sales_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted' UNION ALL SELECT invoice_date,total,'purchase' FROM purchase_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted') GROUP BY p ORDER BY p").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy], |r| {
            Ok(SalesTrend {
                period: r.get(0)?,
                sales: r.get(1)?,
                purchases: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    for r in rows.filter_map(Result::ok) {
        out.push(r)
    }
    Ok(out)
}

#[tauri::command]
fn get_top_products(state: State<AppState>) -> Result<Vec<TopProduct>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut st=c.prepare("SELECT p.id,p.name,COALESCE(SUM(l.quantity),0),COALESCE(SUM(l.line_total),0) FROM sales_invoice_lines l JOIN sales_invoices i ON i.id=l.invoice_id JOIN products p ON p.id=l.product_id WHERE i.company_id=?1 AND i.fiscal_year_id=?2 AND i.status='posted' GROUP BY p.id,p.name ORDER BY SUM(l.line_total) DESC LIMIT 10").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy], |r| {
            Ok(TopProduct {
                product_id: r.get(0)?,
                name: r.get(1)?,
                quantity: r.get(2)?,
                revenue: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_low_stock_report(state: State<AppState>) -> Result<Vec<LowStock>, String> {
    let c = conn(&state)?;
    let company = active_company(&state, &c)?.0;
    let mut st=c.prepare("SELECT p.id,p.name,COALESCE(SUM(ib.quantity),0),p.min_stock,COUNT(DISTINCT ib.warehouse_id) FROM products p LEFT JOIN inventory_balances ib ON ib.product_id=p.id WHERE p.company_id=?1 GROUP BY p.id,p.name,p.min_stock HAVING COALESCE(SUM(ib.quantity),0)<=p.min_stock ORDER BY (p.min_stock-COALESCE(SUM(ib.quantity),0)) DESC").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok(LowStock {
                product_id: r.get(0)?,
                name: r.get(1)?,
                quantity: r.get(2)?,
                min_stock: r.get(3)?,
                warehouse_count: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_recent_invoices(state: State<AppState>) -> Result<Vec<RecentInvoice>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut st=c.prepare("SELECT id,number,invoice_date,contact_name,total,payment_status,invoice_type FROM (SELECT s.id,s.number,s.invoice_date,c.name contact_name,s.total,s.payment_status,'sale' invoice_type FROM sales_invoices s LEFT JOIN contacts c ON c.id=s.contact_id WHERE s.company_id=?1 AND s.fiscal_year_id=?2 AND s.status='posted' UNION ALL SELECT p.id,p.number,p.invoice_date,c.name,p.total,p.payment_status,'purchase' FROM purchase_invoices p LEFT JOIN contacts c ON c.id=p.contact_id WHERE p.company_id=?1 AND p.fiscal_year_id=?2 AND p.status='posted') ORDER BY invoice_date DESC,number DESC LIMIT 8").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy], |r| {
            Ok(RecentInvoice {
                id: r.get(0)?,
                number: r.get(1)?,
                invoice_date: r.get(2)?,
                contact_name: r.get(3)?,
                total: r.get(4)?,
                payment_status: r.get(5)?,
                invoice_type: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_journal_book(
    state: State<AppState>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<JournalBookLine>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let fy_start: String = c
        .query_row(
            "SELECT start_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let fy_end: String = c
        .query_row(
            "SELECT end_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let from = from_date.unwrap_or(fy_start);
    let to = to_date.unwrap_or(fy_end);
    if from > to {
        return Err("RPT-002: بازه تاریخ نامعتبر است".into());
    }
    let mut st=c.prepare("SELECT j.entry_date,j.number,j.description,a.code,a.name,l.debit,l.credit FROM journal_entries j JOIN journal_lines l ON l.journal_id=j.id JOIN accounts a ON a.id=l.account_id WHERE j.company_id=?1 AND j.fiscal_year_id=?2 AND j.status='posted' AND j.entry_date BETWEEN ?3 AND ?4 ORDER BY j.entry_date,j.number,a.code").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company, fy, from, to], |r| {
            Ok(JournalBookLine {
                date: r.get(0)?,
                number: r.get(1)?,
                description: r.get(2)?,
                account_code: r.get(3)?,
                account_name: r.get(4)?,
                debit: r.get(5)?,
                credit: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_financial_statement(
    state: State<AppState>,
    statement: String,
    as_of: Option<String>,
) -> Result<FinancialStatement, String> {
    if statement != "balance_sheet" && statement != "income_statement" {
        return Err("RPT-010: نوع صورت مالی نامعتبر است".into());
    }
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let end: String = c
        .query_row(
            "SELECT end_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let date = as_of.unwrap_or(end);
    let (filter, title) = if statement == "balance_sheet" {
        ("substr(a.code,1,1) IN ('1','2','3')", "ترازنامه")
    } else {
        ("substr(a.code,1,1) IN ('4','5','6')", "صورت سود و زیان")
    };
    let sql=format!("SELECT a.code,a.name,a.nature,COALESCE(SUM(l.debit-l.credit),0) FROM accounts a LEFT JOIN journal_lines l ON l.account_id=a.id LEFT JOIN journal_entries j ON j.id=l.journal_id AND j.status='posted' AND j.company_id=?1 AND j.fiscal_year_id=?2 AND j.entry_date<=?3 WHERE a.company_id=?1 AND a.is_active=1 AND {filter} GROUP BY a.id,a.code,a.name,a.nature ORDER BY a.code");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company, fy, date], |r| {
            Ok(ReportLine {
                code: r.get(0)?,
                name: r.get(1)?,
                nature: r.get(2)?,
                amount: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let lines: Vec<_> = rows
        .filter_map(Result::ok)
        .filter(|x| x.amount != 0)
        .collect();
    let total = lines.iter().map(|x| x.amount.abs()).sum();
    Ok(FinancialStatement {
        title: title.into(),
        as_of: date,
        lines,
        total,
    })
}

#[tauri::command]
fn get_party_aging(
    state: State<AppState>,
    sales: bool,
    as_of: Option<String>,
) -> Result<Vec<PartyAging>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let end: String = c
        .query_row(
            "SELECT end_date FROM fiscal_years WHERE id=?1",
            params![fy],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let date = as_of.unwrap_or(end);
    let table = if sales {
        "sales_invoices"
    } else {
        "purchase_invoices"
    };
    let inv_type = if sales { "sales" } else { "purchase" };
    let sql=format!("SELECT c.id,c.name,i.invoice_date,i.total-COALESCE(s.settled,0) FROM contacts c JOIN {table} i ON i.contact_id=c.id AND i.company_id=?1 AND i.fiscal_year_id=?2 AND i.status='posted' AND i.invoice_date<=?3 LEFT JOIN (SELECT invoice_id,SUM(amount) settled FROM invoice_settlements WHERE company_id=?1 AND fiscal_year_id=?2 AND invoice_type='{inv_type}' AND settlement_date<=?3 GROUP BY invoice_id) s ON s.invoice_id=i.id WHERE c.company_id=?1 AND (i.total-COALESCE(s.settled,0))>0 ORDER BY c.name,i.invoice_date");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = st
        .query_map(params![company, fy, date], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    use std::collections::HashMap;
    let mut map: HashMap<String, PartyAging> = HashMap::new();
    let d0 = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| "RPT-011: تاریخ گزارش نامعتبر است".to_string())?;
    for row in rows.filter_map(Result::ok) {
        let invd = chrono::NaiveDate::parse_from_str(&row.2, "%Y-%m-%d").unwrap_or(d0);
        let days = (d0 - invd).num_days().max(0);
        let e = map.entry(row.0.clone()).or_insert(PartyAging {
            contact_id: row.0.clone(),
            contact_name: row.1.clone(),
            current: 0,
            days_1_30: 0,
            days_31_60: 0,
            days_61_90: 0,
            over_90: 0,
            total: 0,
        });
        e.total += row.3;
        if days == 0 {
            e.current += row.3
        } else if days <= 30 {
            e.days_1_30 += row.3
        } else if days <= 60 {
            e.days_31_60 += row.3
        } else if days <= 90 {
            e.days_61_90 += row.3
        } else {
            e.over_90 += row.3
        }
    }
    Ok(map.into_values().collect())
}

#[tauri::command]
fn get_profit_loss(state: State<AppState>) -> Result<ProfitLoss, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let revenue:i64=c.query_row("SELECT COALESCE(SUM(total),0) FROM sales_invoices WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted'",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let sales_returns:i64=c.query_row("SELECT COALESCE(SUM(total),0) FROM sales_returns WHERE company_id=?1 AND fiscal_year_id=?2 AND status='posted'",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let cogs:i64=c.query_row("SELECT COALESCE(SUM(jl.debit),0) FROM journal_entries je JOIN journal_lines jl ON jl.journal_id=je.id JOIN accounts a ON a.id=jl.account_id WHERE je.company_id=?1 AND je.fiscal_year_id=?2 AND je.status='posted' AND a.code='5100'",params![company,fy],|r|r.get(0)).unwrap_or(0);
    let net = revenue - sales_returns;
    let gross = net - cogs;
    let margin = if net == 0 {
        0.0
    } else {
        gross as f64 * 100.0 / net as f64
    };
    Ok(ProfitLoss {
        revenue,
        sales_returns,
        net_revenue: net,
        cogs,
        gross_profit: gross,
        gross_margin_percent: margin,
    })
}

#[derive(Serialize)]
struct StockCardLine {
    date: String,
    movement_type: String,
    quantity: f64,
    unit_cost: i64,
    balance: f64,
    reference_type: Option<String>,
    note: Option<String>,
}
#[derive(Serialize)]
struct InventoryValuation {
    product_id: String,
    product_name: String,
    warehouse_id: String,
    warehouse_name: String,
    quantity: f64,
    average_cost: i64,
    value: i64,
}
#[derive(Serialize)]
struct SalesReportRow {
    date: String,
    invoice_number: i64,
    contact_name: Option<String>,
    subtotal: i64,
    discount: i64,
    tax: i64,
    total: i64,
    payment_status: String,
}
#[derive(Serialize)]
struct PurchaseReportRow {
    date: String,
    invoice_number: i64,
    contact_name: Option<String>,
    subtotal: i64,
    discount: i64,
    tax: i64,
    total: i64,
    payment_status: String,
}
#[derive(Serialize)]
struct AccountLedgerSummary {
    account_id: String,
    code: String,
    name: String,
    debit: i64,
    credit: i64,
    balance: i64,
}

#[tauri::command]
fn get_stock_card(
    state: State<AppState>,
    product_id: String,
    warehouse_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<StockCardLine>, String> {
    let c = conn(&state)?;
    let company = active_company(&state, &c)?.0;
    let mut sql=String::from("SELECT created_at,movement_type,quantity,unit_cost,reference_type,note FROM inventory_movements WHERE company_id=?1 AND product_id=?2");
    if warehouse_id.is_some() {
        sql.push_str(" AND warehouse_id=?3");
    }
    if from_date.is_some() {
        sql.push_str(if warehouse_id.is_some() {
            " AND substr(created_at,1,10)>=?4"
        } else {
            " AND substr(created_at,1,10)>=?3"
        });
    }
    if to_date.is_some() {
        sql.push_str(if warehouse_id.is_some() {
            if from_date.is_some() {
                " AND substr(created_at,1,10)<=?5"
            } else {
                " AND substr(created_at,1,10)<=?4"
            }
        } else {
            if from_date.is_some() {
                " AND substr(created_at,1,10)<=?4"
            } else {
                " AND substr(created_at,1,10)<=?3"
            }
        });
    }
    sql.push_str(" ORDER BY created_at ASC,id ASC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&company, &product_id];
    if let Some(ref x) = warehouse_id {
        params_vec.push(x);
    }
    if let Some(ref x) = from_date {
        params_vec.push(x);
    }
    if let Some(ref x) = to_date {
        params_vec.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(params_vec), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut balance = 0.0;
    let mut out = Vec::new();
    for row in rows.filter_map(Result::ok) {
        let sign = if row.1 == "receipt" || row.1 == "transfer_in" {
            1.0
        } else if row.1 == "issue" || row.1 == "transfer_out" {
            -1.0
        } else {
            if row.2 > 0.0 {
                1.0
            } else {
                -1.0
            }
        };
        balance += row.2 * sign;
        out.push(StockCardLine {
            date: row.0,
            movement_type: row.1,
            quantity: row.2,
            unit_cost: row.3,
            balance,
            reference_type: row.4,
            note: row.5,
        });
    }
    Ok(out)
}

#[tauri::command]
fn get_inventory_valuation(state: State<AppState>) -> Result<Vec<InventoryValuation>, String> {
    let c = conn(&state)?;
    let company = active_company(&state, &c)?.0;
    let mut st=c.prepare("SELECT p.id,p.name,w.id,w.name,COALESCE(ib.quantity,0) FROM products p JOIN warehouses w ON w.company_id=p.company_id LEFT JOIN inventory_balances ib ON ib.product_id=p.id AND ib.warehouse_id=w.id WHERE p.company_id=?1 AND w.is_active=1 AND COALESCE(ib.quantity,0)>0 ORDER BY p.name,w.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![company], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows.filter_map(Result::ok) {
        let mut avg = 0.0;
        let mut qty = 0.0;
        let mut mv=c.prepare("SELECT movement_type,quantity,unit_cost FROM inventory_movements WHERE company_id=?1 AND product_id=?2 AND warehouse_id=?3 ORDER BY created_at ASC,id ASC").map_err(|e|e.to_string())?;
        let mvs = mv
            .query_map(params![company, &row.0, &row.2], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for m in mvs.filter_map(Result::ok) {
            match m.0.as_str() {
                "receipt" | "transfer_in" => {
                    let new_qty = qty + m.1;
                    if m.1 > 0.0 {
                        avg = ((avg * qty) + (m.2 as f64 * m.1)) / new_qty;
                    }
                    qty = new_qty;
                }
                "issue" | "transfer_out" => {
                    qty = (qty - m.1).max(0.0);
                }
                "adjustment" => {
                    if m.1 > 0.0 {
                        qty += m.1;
                        if m.2 > 0 {
                            avg = ((avg * (qty - m.1)) + (m.2 as f64 * m.1)) / qty;
                        }
                    } else {
                        qty = (qty - m.1).max(0.0);
                    }
                }
                _ => {}
            }
        }
        let current = row.4;
        let value = (current * avg).round() as i64;
        out.push(InventoryValuation {
            product_id: row.0,
            product_name: row.1,
            warehouse_id: row.2,
            warehouse_name: row.3,
            quantity: current,
            average_cost: avg.round() as i64,
            value,
        });
    }
    Ok(out)
}

#[tauri::command]
fn get_sales_report(
    state: State<AppState>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<SalesReportRow>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut sql=String::from("SELECT s.invoice_date,s.number,c.name,s.subtotal,s.discount,s.tax,s.total,s.payment_status FROM sales_invoices s LEFT JOIN contacts c ON c.id=s.contact_id WHERE s.company_id=?1 AND s.fiscal_year_id=?2 AND s.status='posted'");
    if from_date.is_some() {
        sql.push_str(" AND s.invoice_date>=?3");
    }
    if to_date.is_some() {
        sql.push_str(if from_date.is_some() {
            " AND s.invoice_date<=?4"
        } else {
            " AND s.invoice_date<=?3"
        });
    }
    sql.push_str(" ORDER BY s.invoice_date DESC,s.number DESC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy];
    if let Some(ref x) = from_date {
        pv.push(x);
    }
    if let Some(ref x) = to_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            Ok(SalesReportRow {
                date: r.get(0)?,
                invoice_number: r.get(1)?,
                contact_name: r.get(2)?,
                subtotal: r.get(3)?,
                discount: r.get(4)?,
                tax: r.get(5)?,
                total: r.get(6)?,
                payment_status: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_purchase_report(
    state: State<AppState>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<PurchaseReportRow>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut sql=String::from("SELECT s.invoice_date,s.number,c.name,s.subtotal,s.discount,s.tax,s.total,s.payment_status FROM purchase_invoices s LEFT JOIN contacts c ON c.id=s.contact_id WHERE s.company_id=?1 AND s.fiscal_year_id=?2 AND s.status='posted'");
    if from_date.is_some() {
        sql.push_str(" AND s.invoice_date>=?3");
    }
    if to_date.is_some() {
        sql.push_str(if from_date.is_some() {
            " AND s.invoice_date<=?4"
        } else {
            " AND s.invoice_date<=?3"
        });
    }
    sql.push_str(" ORDER BY s.invoice_date DESC,s.number DESC");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy];
    if let Some(ref x) = from_date {
        pv.push(x);
    }
    if let Some(ref x) = to_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            Ok(PurchaseReportRow {
                date: r.get(0)?,
                invoice_number: r.get(1)?,
                contact_name: r.get(2)?,
                subtotal: r.get(3)?,
                discount: r.get(4)?,
                tax: r.get(5)?,
                total: r.get(6)?,
                payment_status: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn get_account_ledger_summary(
    state: State<AppState>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<AccountLedgerSummary>, String> {
    let c = conn(&state)?;
    let (company, fy) = active_company(&state, &c)?;
    let mut sql=String::from("SELECT a.id,a.code,a.name,COALESCE(SUM(jl.debit),0),COALESCE(SUM(jl.credit),0) FROM accounts a LEFT JOIN journal_lines jl ON jl.account_id=a.id LEFT JOIN journal_entries je ON je.id=jl.journal_id AND je.status='posted' AND je.company_id=a.company_id AND je.fiscal_year_id=?2 WHERE a.company_id=?1");
    if from_date.is_some() {
        sql.push_str(" AND (je.entry_date IS NULL OR je.entry_date>=?3)");
    }
    if to_date.is_some() {
        sql.push_str(if from_date.is_some() {
            " AND (je.entry_date IS NULL OR je.entry_date<=?4)"
        } else {
            " AND (je.entry_date IS NULL OR je.entry_date<=?3)"
        });
    }
    sql.push_str(" GROUP BY a.id,a.code,a.name ORDER BY a.code");
    let mut st = c.prepare(&sql).map_err(|e| e.to_string())?;
    let mut pv: Vec<&dyn rusqlite::ToSql> = vec![&company, &fy];
    if let Some(ref x) = from_date {
        pv.push(x);
    }
    if let Some(ref x) = to_date {
        pv.push(x);
    }
    let rows = st
        .query_map(rusqlite::params_from_iter(pv), |r| {
            let d: i64 = r.get(3)?;
            let cr: i64 = r.get(4)?;
            Ok(AccountLedgerSummary {
                account_id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                debit: d,
                credit: cr,
                balance: d - cr,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[derive(Serialize)]
struct PluginInfo {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    enabled: bool,
    permissions: Vec<String>,
}
#[derive(Serialize)]
struct ApiProfile {
    id: String,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: Option<String>,
    timeout_ms: i64,
    enabled: bool,
    allowed_domains: String,
}
#[derive(Serialize)]
struct ApiResponse {
    status: u16,
    body: String,
    content_type: Option<String>,
}

fn plugin_root(state: &State<AppState>) -> Result<PathBuf, String> {
    let db = state
        .db_path
        .lock()
        .map_err(|_| "PLUGIN-001: مسیر برنامه در دسترس نیست".to_string())?;
    let root = db
        .parent()
        .ok_or_else(|| "PLUGIN-002: مسیر برنامه نامعتبر است".to_string())?
        .join("plugins");
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("PLUGIN-003: ایجاد پوشه Plugin انجام نشد: {e}"))?;
    Ok(root)
}

fn validate_plugin_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("PLUGIN-004: شناسه Plugin نامعتبر است".into());
    }
    Ok(())
}

#[tauri::command]
fn list_plugins(state: State<AppState>) -> Result<Vec<PluginInfo>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "plugins.view")?;
    let mut st=c.prepare("SELECT p.id,p.name,p.version,p.description,p.enabled FROM plugins p LEFT JOIN company_users cu ON cu.company_id=p.company_id WHERE (p.company_id IS NULL OR (cu.user_id=?1 AND cu.is_active=1)) ORDER BY p.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, i64>(4)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows.filter_map(Result::ok) {
        let mut ps = c
            .prepare(
                "SELECT permission FROM plugin_permissions WHERE plugin_id=?1 ORDER BY permission",
            )
            .map_err(|e| e.to_string())?;
        let perms = ps
            .query_map(params![row.0.clone()], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .collect();
        out.push(PluginInfo {
            id: row.0,
            name: row.1,
            version: row.2,
            description: row.3,
            enabled: row.4,
            permissions: perms,
        });
    }
    Ok(out)
}

#[tauri::command]
fn register_plugin(
    state: State<AppState>,
    manifest_json: String,
    executable_path: String,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "plugins.manage")?;
    let v: serde_json::Value = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("PLUGIN-005: Manifest نامعتبر است: {e}"))?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "PLUGIN-006: id الزامی است".to_string())?;
    validate_plugin_id(id)?;
    let name = v
        .get("name")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "PLUGIN-007: name الزامی است".to_string())?;
    let version = v
        .get("version")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "PLUGIN-008: version الزامی است".to_string())?;
    let entry = v
        .get("entrypoint")
        .and_then(|x| x.as_str())
        .unwrap_or("worker");
    if entry.contains('/') || entry.contains('\\') || entry == "." || entry == ".." {
        return Err("PLUGIN-009: entrypoint نامعتبر است".into());
    }
    let perms = v
        .get("permissions")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let allowed = [
        "network",
        "native.execute",
        "database.read",
        "database.write",
        "filesystem.read",
        "filesystem.write",
        "plugins.execute",
    ];
    for p in &perms {
        let Some(ps) = p.as_str() else {
            return Err("PLUGIN-010: Permission نامعتبر است".into());
        };
        if !allowed.contains(&ps) {
            return Err(format!("PLUGIN-011: Permission پشتیبانی‌نشده: {ps}"));
        }
    }
    let src = std::path::PathBuf::from(&executable_path);
    if !src.is_file() {
        return Err("PLUGIN-012: فایل Worker پیدا نشد".into());
    }
    let root = plugin_root(&state)?;
    let dir = root.join(id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("PLUGIN-013: ایجاد پوشه Plugin انجام نشد: {e}"))?;
    let target = dir.join(entry);
    std::fs::copy(&src, &target).map_err(|e| format!("PLUGIN-014: نصب Worker انجام نشد: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&target)
            .map_err(|e| e.to_string())?
            .permissions();
        perm.set_mode(0o700);
        std::fs::set_permissions(&target, perm).map_err(|e| e.to_string())?;
    }
    let company = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get::<_, String>(0),
        )
        .map_err(|_| "PLUGIN-015: شرکت فعال یافت نشد".to_string())?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute("INSERT OR REPLACE INTO plugins(id,company_id,name,version,description,entrypoint,manifest_json,enabled) VALUES(?1,?2,?3,?4,?5,?6,?7,0)",params![id,company,name,version,v.get("description").and_then(|x|x.as_str()),entry,manifest_json]) .map_err(|e|format!("PLUGIN-016: ثبت Plugin انجام نشد: {e}"))?;
    tx.execute(
        "DELETE FROM plugin_permissions WHERE plugin_id=?1",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    for p in perms {
        // مجوز باید رشته باشد. افزونه‌ای که عدد یا شیء بفرستد، پیش از این
        // برنامه را کرش می‌کرد — یعنی یک فایل افزونه‌ی خراب کافی بود تا
        // نرم‌افزار حسابداری بسته شود.
        let permission = p
            .as_str()
            .ok_or_else(|| "PLUGIN-015: فهرست مجوزهای افزونه باید متنی باشد".to_string())?;
        tx.execute(
            "INSERT INTO plugin_permissions(plugin_id,permission) VALUES(?1,?2)",
            params![id, permission],
        )
        .map_err(|e| e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "plugin.register",
        "plugin",
        id,
        None,
        Some(&manifest_json),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id.to_string())
}

#[tauri::command]
fn set_plugin_enabled(
    state: State<AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "plugins.manage")?;
    let old:i64=c.query_row("SELECT p.enabled FROM plugins p LEFT JOIN company_users cu ON cu.company_id=p.company_id WHERE p.id=?1 AND cu.user_id=?2 AND cu.is_active=1",params![plugin_id,user],|r|r.get(0)).map_err(|_|"PLUGIN-017: Plugin پیدا نشد".to_string())?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE plugins SET enabled=?2 WHERE id=?1",
        params![plugin_id, enabled as i64],
    )
    .map_err(|e| e.to_string())?;
    audit(
        &tx,
        &user,
        "plugin.enable",
        "plugin",
        &plugin_id,
        Some(&old.to_string()),
        Some(&(enabled as i64).to_string()),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn execute_plugin(
    state: State<AppState>,
    plugin_id: String,
    payload: String,
) -> Result<String, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "plugins.execute")?;
    let enabled:i64=c.query_row("SELECT p.enabled FROM plugins p LEFT JOIN company_users cu ON cu.company_id=p.company_id WHERE p.id=?1 AND cu.user_id=?2 AND cu.is_active=1",params![plugin_id,user],|r|r.get(0)).map_err(|_|"PLUGIN-018: Plugin پیدا نشد".to_string())?;
    if enabled == 0 {
        return Err("PLUGIN-019: Plugin فعال نیست".into());
    }
    let native_ok = has_permission(&c, &user, "native.execute")?;
    if !native_ok {
        return Err("PLUGIN-020: مجوز اجرای Native Worker وجود ندارد".into());
    }
    let manifest_native: i64=c.query_row("SELECT COUNT(*) FROM plugin_permissions WHERE plugin_id=?1 AND permission='native.execute'",params![plugin_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if manifest_native == 0 {
        return Err("PLUGIN-020: Plugin مجوز native.execute درخواست نکرده است".into());
    }
    let entry:String = c.query_row("SELECT p.entrypoint FROM plugins p LEFT JOIN company_users cu ON cu.company_id=p.company_id WHERE p.id=?1 AND cu.user_id=?2 AND cu.is_active=1",params![plugin_id,user],|r|r.get(0)).map_err(|_|"PLUGIN-021: اطلاعات Worker یافت نشد".to_string())?;
    let root = plugin_root(&state)?;
    let dir = root.join(&plugin_id);
    let exe = dir.join(&entry);
    if !exe.is_file() {
        return Err("PLUGIN-022: Worker نصب‌شده پیدا نشد".into());
    }
    let mut child = std::process::Command::new(&exe)
        .current_dir(&dir)
        .env_clear()
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("PLUGIN-023: اجرای Worker انجام نشد: {e}"))?;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.as_bytes())
            .map_err(|e| format!("PLUGIN-024: ارسال داده به Worker انجام نشد: {e}"))?;
    }
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let out = child.wait_with_output().map_err(|e| e.to_string())?;
            if !status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                return Err(format!(
                    "PLUGIN-025: Worker با خطا متوقف شد: {}",
                    err.trim()
                ));
            }
            return Ok(String::from_utf8_lossy(&out.stdout)
                .chars()
                .take(1_000_000)
                .collect());
        }
        if started.elapsed() > std::time::Duration::from_secs(15) {
            let _ = child.kill();
            return Err("PLUGIN-026: زمان اجرای Worker بیشتر از ۱۵ ثانیه شد".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[derive(Serialize)]
struct SavedReport {
    id: String,
    name: String,
    source: String,
    config_json: String,
    created_at: String,
    updated_at: String,
}

#[tauri::command]
fn list_custom_reports(state: State<AppState>) -> Result<Vec<SavedReport>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "reports.view")?;
    let mut st=c.prepare("SELECT r.id,r.name,r.source,r.config_json,r.created_at,r.updated_at FROM custom_reports r JOIN company_users cu ON cu.company_id=r.company_id WHERE r.created_by=?1 AND cu.user_id=?1 AND cu.is_active=1 ORDER BY r.updated_at DESC").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(SavedReport {
                id: r.get(0)?,
                name: r.get(1)?,
                source: r.get(2)?,
                config_json: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

#[tauri::command]
fn save_custom_report(
    state: State<AppState>,
    id: Option<String>,
    name: String,
    source: String,
    config_json: String,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "reports.builder.manage")?;
    if name.trim().is_empty() {
        return Err("REP-001: نام گزارش الزامی است".into());
    }
    if name.chars().count() > 120 {
        return Err("REP-002: نام گزارش بیش از حد طولانی است".into());
    }
    if config_json.len() > 100_000 {
        return Err("REP-003: تنظیمات گزارش بیش از حد بزرگ است".into());
    }
    let allowed = ["sales", "purchase", "inventory", "ledger", "trial"];
    if !allowed.contains(&source.as_str()) {
        return Err("REP-004: منبع گزارش نامعتبر است".into());
    }
    let _: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|_| "REP-005: تنظیمات گزارش JSON نامعتبر است".to_string())?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _fy) = active_context(&tx, &user)?;
    let updating = id.as_ref().map(|x| !x.trim().is_empty()).unwrap_or(false);
    let rid = id.filter(|x| !x.trim().is_empty()).unwrap_or_else(|| {
        format!(
            "report-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });
    if updating {
        let n=tx.execute("UPDATE custom_reports SET name=?2,source=?3,config_json=?4,updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND company_id=?5 AND created_by=?6",params![rid,name,source,config_json,company,user]).map_err(|e|e.to_string())?;
        if n == 0 {
            return Err("REP-006: گزارش برای ویرایش پیدا نشد".into());
        }
    } else {
        tx.execute("INSERT INTO custom_reports(id,company_id,name,source,config_json,created_by) VALUES(?1,?2,?3,?4,?5,?6)",params![rid,company,name,source,config_json,user]).map_err(|e|format!("REP-007: ذخیره گزارش انجام نشد: {e}"))?;
    }
    audit(
        &tx,
        &user,
        "report.builder.save",
        "custom_report",
        &rid,
        None,
        Some(&config_json),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(rid)
}

#[tauri::command]
fn delete_custom_report(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "reports.builder.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let n=tx.execute("DELETE FROM custom_reports WHERE id=?1 AND created_by=?2 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?2 AND is_active=1)",params![id,user]).map_err(|e|e.to_string())?;
    if n == 0 {
        return Err("REP-008: گزارش پیدا نشد".into());
    }
    audit(
        &tx,
        &user,
        "report.builder.delete",
        "custom_report",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn list_api_profiles(state: State<AppState>) -> Result<Vec<ApiProfile>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "integrations.view")?;
    let mut st=c.prepare("SELECT p.id,p.name,p.base_url,p.auth_type,p.auth_header,p.timeout_ms,p.enabled,p.allowed_domains FROM api_profiles p JOIN company_users cu ON cu.company_id=p.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY p.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(ApiProfile {
                id: r.get(0)?,
                name: r.get(1)?,
                base_url: r.get(2)?,
                auth_type: r.get(3)?,
                auth_header: r.get(4)?,
                timeout_ms: r.get(5)?,
                enabled: r.get::<_, i64>(6)? != 0,
                allowed_domains: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn api_secret_key(profile_id: &str) -> String {
    format!("api-profile:{profile_id}")
}

#[tauri::command]
fn create_api_profile(
    state: State<AppState>,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: Option<String>,
    timeout_ms: i64,
    allowed_domains: String,
    secret: Option<String>,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "integrations.manage")?;
    if name.trim().is_empty() {
        return Err("API-001: نام اتصال الزامی است".into());
    }
    if !matches!(auth_type.as_str(), "none" | "api_key" | "bearer" | "basic") {
        return Err("API-002: نوع احراز هویت نامعتبر است".into());
    }
    let base =
        reqwest::Url::parse(&base_url).map_err(|_| "API-003: آدرس پایه نامعتبر است".to_string())?;
    if base.scheme() != "https" {
        return Err("API-004: فقط HTTPS برای اتصال خارجی مجاز است".into());
    }
    let host = base
        .host_str()
        .ok_or_else(|| "API-005: دامنه آدرس مشخص نیست".to_string())?;
    let domains = if allowed_domains.trim().is_empty() {
        host.to_string()
    } else {
        allowed_domains
    };
    if !domains.split(',').map(str::trim).any(|d| d == host) {
        return Err("API-006: دامنه Base URL باید در Allowed Domains باشد".into());
    }
    if !(1000..=120000).contains(&timeout_ms) {
        return Err("API-007: Timeout باید بین ۱ تا ۱۲۰ ثانیه باشد".into());
    }
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "API-008: شرکت فعال یافت نشد".to_string())?;
    let id = format!(
        "api-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    c.execute("INSERT INTO api_profiles(id,company_id,name,base_url,auth_type,auth_header,timeout_ms,allowed_domains) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,company,name,base_url,auth_type,auth_header,timeout_ms,domains]).map_err(|e|format!("API-009: ثبت اتصال انجام نشد: {e}"))?;
    if let Some(secret) = secret {
        if !secret.is_empty() {
            let entry = keyring::Entry::new("novin-pardaz-accounting", &api_secret_key(&id))
                .map_err(|e| format!("API-010: دسترسی Secret Storage ممکن نیست: {e}"))?;
            entry
                .set_password(&secret)
                .map_err(|e| format!("API-011: ذخیره Secret انجام نشد: {e}"))?;
        }
    }
    Ok(id)
}

#[tauri::command]
fn execute_api_request(
    state: State<AppState>,
    profile_id: String,
    method: String,
    path: String,
    headers_json: Option<String>,
    body: Option<String>,
) -> Result<ApiResponse, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "integrations.execute")?;
    let p:(String,String,String,Option<String>,i64,bool)=c.query_row("SELECT base_url,auth_type,allowed_domains,auth_header,timeout_ms,enabled FROM api_profiles p JOIN company_users cu ON cu.company_id=p.company_id WHERE p.id=?1 AND cu.user_id=?2 AND cu.is_active=1",params![profile_id,user],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get::<_, i32>(5)? != 0))).map_err(|_|"API-012: اتصال API پیدا نشد".to_string())?;
    if !p.5 {
        return Err("API-013: اتصال API غیرفعال است".into());
    }
    let base =
        reqwest::Url::parse(&p.0).map_err(|_| "API-014: Base URL نامعتبر است".to_string())?;
    let url = base
        .join(path.trim_start_matches('/'))
        .map_err(|_| "API-015: مسیر درخواست نامعتبر است".to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "API-016: دامنه مقصد مشخص نیست".to_string())?;
    if !p.2.split(',').map(str::trim).any(|d| d == host) {
        return Err("API-017: دامنه مقصد در Allowlist نیست".into());
    }
    let m = match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        _ => return Err("API-018: HTTP Method پشتیبانی نمی‌شود".into()),
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(p.4 as u64))
        .build()
        .map_err(|e| format!("API-019: ساخت Client انجام نشد: {e}"))?;
    let mut req = client.request(m, url);
    if let Some(h) = headers_json {
        let hv: serde_json::Value = serde_json::from_str(&h)
            .map_err(|_| "API-020: Headers JSON نامعتبر است".to_string())?;
        if let Some(obj) = hv.as_object() {
            for (k, v) in obj {
                if matches!(
                    k.to_lowercase().as_str(),
                    "host" | "authorization" | "cookie"
                ) {
                    return Err(format!("API-021: Header حساس مجاز نیست: {k}"));
                }
                if let Some(val) = v.as_str() {
                    req = req.header(k, val);
                }
            }
        }
    }
    if p.1 != "none" {
        let entry = keyring::Entry::new("novin-pardaz-accounting", &api_secret_key(&profile_id))
            .map_err(|e| format!("API-022: Secret Storage در دسترس نیست: {e}"))?;
        let secret = entry
            .get_password()
            .map_err(|_| "API-023: Secret این اتصال پیدا نشد".to_string())?;
        match p.1.as_str() {
            "api_key" => {
                let h =
                    p.3.ok_or_else(|| "API-024: نام Header برای API Key مشخص نشده".to_string())?;
                req = req.header(h, secret)
            }
            "bearer" => req = req.bearer_auth(secret),
            "basic" => {
                let parts = secret.splitn(2, ':').collect::<Vec<_>>();
                if parts.len() != 2 {
                    return Err("API-025: Secret نوع Basic باید username:password باشد".into());
                }
                req = req.basic_auth(parts[0], Some(parts[1]))
            }
            _ => {}
        }
    }
    if let Some(b) = body {
        req = req.body(b).header("content-type", "application/json");
    }
    let resp = req
        .send()
        .map_err(|e| format!("API-026: درخواست ناموفق بود: {e}"))?;
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let text = resp.text().unwrap_or_default();
    let body = text.chars().take(1_000_000).collect();
    Ok(ApiResponse {
        status,
        body,
        content_type: ct,
    })
}

#[tauri::command]
fn set_api_profile_enabled(
    state: State<AppState>,
    profile_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "integrations.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let n=tx.execute("UPDATE api_profiles SET enabled=?2 WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?3 AND is_active=1)",params![profile_id,enabled as i64,user]).map_err(|e|e.to_string())?;
    if n == 0 {
        return Err("API-027: اتصال API پیدا نشد".into());
    }
    audit(
        &tx,
        &user,
        "api_profile.enable",
        "api_profile",
        &profile_id,
        None,
        Some(&(enabled as i64).to_string()),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&dir).expect("create app data dir");
            let path = dir.join("novin-accounting.sqlite");
            db::open(&path).expect("database initialization failed");
            app.manage(AppState {
                db_path: Mutex::new(path),
                user_id: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            current_user,
            get_company,
            list_accounts,
            list_contacts,
            list_products,
            list_permissions,
            list_warehouses,
            list_inventory_advanced,
            get_inventory_valuation_method,
            set_inventory_valuation_method,
            reserve_inventory,
            release_inventory,
            create_inventory_lot,
            list_inventory_lots,
            create_inventory_count,
            list_inventory_counts,
            set_inventory_count_line,
            post_inventory_count,
            create_inventory_transfer_order,
            list_inventory_transfer_orders,
            receive_inventory_transfer,
            list_stock_balances,
            create_contact,
            create_product,
            receive_stock,
            issue_stock,
            list_journals,
            create_journal_draft,
            create_single_line_journal,
            list_subsidiary_groups,
            list_cost_centers,
            list_projects,
            list_postable_accounts,
            list_product_groups,
            list_parties,
            preview_invoice,
            list_stocktakes,
            create_stocktake,
            get_stocktake,
            set_stocktake_count,
            approve_all_variances,
            post_stocktake,
            preview_bulk_price_change,
            apply_bulk_price_change,
            get_low_stock,
            list_valuation_methods,
            build_installment_plan,
            list_party_routes,
            validate_party_identity,
            update_party_profile,
            list_product_prices,
            set_product_price,
            create_journal,
            post_journal,
            reverse_journal,
            delete_demo_data,
            update_contact,
            delete_contact,
            update_product,
            delete_product,
            transfer_stock,
            adjust_stock,
            list_backups,
            backup_database,
            restore_database,
            create_sales_invoice,
            create_purchase_invoice,
            list_sales_invoices,
            list_purchase_invoices,
            post_sales_invoice,
            post_purchase_invoice,
            settle_invoice,
            list_treasury_accounts,
            create_treasury_account,
            update_treasury_account,
            list_treasury_transactions,
            list_treasury_transactions_filtered,
            get_treasury_statement,
            get_treasury_summary,
            create_treasury_transaction,
            create_treasury_transfer,
            list_treasury_balances,
            get_trial_balance,
            list_checks,
            list_checks_filtered,
            get_check_dashboard,
            create_check,
            update_check_status,
            check_transition_options,
            treasury_docs::preview_treasury_document,
            treasury_docs::create_treasury_document,
            treasury_docs::list_treasury_documents,
            treasury_docs::get_treasury_document,
            treasury_docs::list_payment_methods,
            treasury_accounts::list_treasury_account_details,
            treasury_accounts::save_treasury_account,
            treasury_accounts::deactivate_treasury_account,
            treasury_accounts::list_negative_policies,
            parties_form::list_party_groups,
            parties_form::save_party_group,
            parties_form::get_party,
            parties_form::save_party,
            parties_form::list_party_options,
            parties_form::list_upcoming_occasions,
            parties_form::deactivate_party,
            parties_form::find_duplicate_party,
            chart_of_accounts::get_coding_scheme,
            chart_of_accounts::set_coding_scheme,
            chart_of_accounts::list_account_tree,
            chart_of_accounts::suggest_account_code,
            chart_of_accounts::save_account,
            chart_of_accounts::deactivate_account,
            chart_of_accounts::audit_coding_health,
            returns::list_returnable_lines,
            returns::list_returns,
            returns::get_return,
            returns::post_sales_return_v2,
            returns::post_purchase_return_v2,
            returns::cancel_return,
            quotes::preview_quote,
            quotes::save_quote,
            quotes::list_quotes,
            quotes::get_quote,
            quotes::quote_transitions,
            quotes::set_quote_status,
            quotes::convert_quote,
            production::save_production_formula,
            production::list_production_formulas,
            production::get_production_formula,
            production::expand_production_formula,
            production::delete_production_formula,
            production::preview_production,
            production::post_production,
            production::list_production_orders,
            production::list_cost_allocations,
            production::list_production_expense_accounts,
            settings::list_settings,
            settings::set_setting,
            settings::reset_setting,
            create_sales_return,
            create_purchase_return,
            post_sales_return,
            post_purchase_return,
            get_journal_book,
            get_financial_statement,
            get_party_aging,
            get_account_ledger,
            get_receivables,
            get_payables,
            get_cash_position,
            get_fiscal_period_status,
            close_fiscal_year,
            verify_backup_file,
            get_demo_status,
            list_print_templates,
            save_print_template,
            delete_print_template,
            import_data,
            list_custom_reports,
            save_custom_report,
            delete_custom_report,
            get_dashboard_kpis,
            get_sales_trend,
            get_top_products,
            get_low_stock_report,
            get_recent_invoices,
            get_profit_loss,
            get_stock_card,
            get_inventory_valuation,
            get_sales_report,
            get_purchase_report,
            get_account_ledger_summary,
            list_plugins,
            register_plugin,
            set_plugin_enabled,
            execute_plugin,
            list_api_profiles,
            create_api_profile,
            execute_api_request,
            set_api_profile_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}
