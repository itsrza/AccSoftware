//! داده‌ی نمونه‌ی بخش‌هایی که در `demo.rs` پوشش داده نشده بودند.
//!
//! ## چرا فایل جدا
//! `demo.rs` رویدادهای «جریان اصلی» را می‌سازد: کالا، شخص، فاکتور، چک،
//! برگشت، انتقال و پیش‌فاکتور. این فایل بخش‌های «پشتیبان» را پر می‌کند تا
//! هیچ صفحه‌ای در نرم‌افزار نصب‌شده خالی باز نشود:
//!
//! - تولید (فرمول ساخت + رسید تولید با بهای تمام‌شده‌ی متوازن)
//! - انبارگردانی (یک دوره‌ی در حال شمارش با اختلاف واقعی)
//! - سری/بچ و سریال کالا
//! - برگشت از خرید
//! - قالب‌های چاپ، گزارش‌های ذخیره‌شده، اتصال API و افزونه
//!
//! ## قواعد
//! همان قواعد `demo.rs`: قطعی، Idempotent، و از نظر حسابداری درست. بهای
//! تمام‌شده‌ی رسید تولید دقیقاً برابر مجموع مواد و هزینه‌هاست — یک ریال هم
//! نه کم، نه زیاد.

use rusqlite::{params, Connection, Result};

const COMPANY: &str = "company-demo";
const FISCAL_YEAR: &str = "fy-demo";
const USER: &str = "user-demo";

/// درج داده‌ی نمونه‌ی بخش‌های پشتیبان. اجرای دوباره بی‌اثر است.
pub fn seed_supporting_data(tx: &Connection, warehouses: &[String]) -> Result<()> {
    let main = warehouses.first().cloned().unwrap_or_default();
    if main.is_empty() {
        return Ok(());
    }
    seed_production(tx, &main)?;
    seed_stocktake(tx, &main)?;
    seed_lots(tx, &main)?;
    seed_purchase_returns(tx, &main)?;
    seed_print_templates(tx)?;
    seed_custom_reports(tx)?;
    seed_integrations(tx)?;
    Ok(())
}

/// آیا کالایی با این شناسه وجود دارد؟ ارجاع به کالای ناموجود کلید خارجی را می‌شکند.
fn product_exists(tx: &Connection, id: &str) -> bool {
    tx.query_row(
        "SELECT 1 FROM products WHERE id=?1",
        params![id],
        |row| row.get::<_, i64>(0),
    )
    .is_ok()
}

fn unit_cost(tx: &Connection, id: &str) -> i64 {
    tx.query_row(
        "SELECT purchase_price FROM products WHERE id=?1",
        params![id],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// تولید
// ---------------------------------------------------------------------------

/// دو فرمول ساخت و دو رسید تولید با بهای تمام‌شده‌ی دقیقاً متوازن.
fn seed_production(tx: &Connection, warehouse: &str) -> Result<()> {
    // محصول تولیدی و مواد اولیه از کاتالوگ واقعی دمو انتخاب می‌شوند.
    let recipes: [(&str, &str, &str, f64, [(&str, f64, f64); 2]); 2] = [
        (
            "demo-formula-shirt",
            "demo-prod-007",
            "دوخت پیراهن مردانه — هر ۱۰ عدد",
            10.0,
            [("demo-prod-010", 1.6, 4.0), ("demo-prod-004", 0.2, 0.0)],
        ),
        (
            "demo-formula-pack",
            "demo-prod-003",
            "بسته‌بندی کارتن — هر ۵۰ عدد",
            50.0,
            [("demo-prod-009", 0.35, 2.5), ("demo-prod-004", 0.1, 0.0)],
        ),
    ];

    for (formula_id, product, title, output_quantity, components) in recipes {
        if !product_exists(tx, product) {
            continue;
        }
        tx.execute(
            "INSERT OR IGNORE INTO production_formulas\
             (id,company_id,product_id,title,output_quantity,is_active) \
             VALUES(?1,?2,?3,?4,?5,1)",
            params![formula_id, COMPANY, product, title, output_quantity],
        )?;
        for (index, (component, per_unit, waste)) in components.iter().enumerate() {
            if !product_exists(tx, component) {
                continue;
            }
            tx.execute(
                "INSERT OR IGNORE INTO production_formula_components\
                 (id,formula_id,product_id,quantity_per_unit,waste_percent) \
                 VALUES(?1,?2,?3,?4,?5)",
                params![
                    format!("{formula_id}-c{index}"),
                    formula_id,
                    component,
                    per_unit,
                    waste
                ],
            )?;
        }
    }

    // --- رسیدهای تولید ---
    for index in 0..2usize {
        let order_id = format!("demo-production-{index:03}");
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM production_orders WHERE id=?1",
            params![order_id],
            |row| row.get(0),
        )?;
        if exists > 0 {
            continue;
        }

        let (formula, output_product, output_quantity) = if index == 0 {
            ("demo-formula-shirt", "demo-prod-007", 10.0f64)
        } else {
            ("demo-formula-pack", "demo-prod-003", 50.0f64)
        };
        if !product_exists(tx, output_product) {
            continue;
        }

        // مواد مصرفی با بهای واقعی کاتالوگ.
        let inputs: [(&str, f64); 2] = if index == 0 {
            [("demo-prod-010", 16.6), ("demo-prod-004", 2.0)]
        } else {
            [("demo-prod-009", 17.9), ("demo-prod-004", 5.0)]
        };
        let mut materials_total = 0i64;
        let mut input_rows = Vec::new();
        for (product, quantity) in inputs {
            if !product_exists(tx, product) {
                continue;
            }
            let cost = unit_cost(tx, product);
            let line_total = (cost as f64 * quantity).round() as i64;
            materials_total += line_total;
            input_rows.push((product, quantity, cost, line_total));
        }
        if input_rows.is_empty() {
            continue;
        }

        // دو هزینه‌ی تولید: دستمزد مستقیم و سربار.
        let expenses: [(&str, &str, i64); 2] = [
            ("acc-5300", "دستمزد مستقیم کارگاه", 18_500_000 + index as i64 * 4_000_000),
            ("acc-5400", "سربار تولید (برق و استهلاک)", 7_200_000 + index as i64 * 1_500_000),
        ];
        let expenses_total: i64 = expenses.iter().map(|(_, _, amount)| *amount).sum();
        let total_cost = materials_total + expenses_total;

        let number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(number),0)+1 FROM production_orders \
             WHERE company_id=?1 AND fiscal_year_id=?2",
            params![COMPANY, FISCAL_YEAR],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO production_orders\
             (id,company_id,fiscal_year_id,number,production_date,warehouse_id,formula_id,\
              cost_allocation,materials_total,expenses_total,total_cost,status,description,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,'by_quantity',?8,?9,?10,'posted',?11,?12)",
            params![
                order_id,
                COMPANY,
                FISCAL_YEAR,
                number,
                format!("1405/0{}/1{}", index + 3, index + 2),
                warehouse,
                formula,
                materials_total,
                expenses_total,
                total_cost,
                format!("رسید تولید نمونه {}", index + 1),
                USER
            ],
        )?;

        for (position, (product, quantity, cost, line_total)) in input_rows.iter().enumerate() {
            tx.execute(
                "INSERT INTO production_inputs(id,order_id,product_id,quantity,unit_cost,line_total) \
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    format!("{order_id}-in{position}"),
                    order_id,
                    product,
                    quantity,
                    cost,
                    line_total
                ],
            )?;
        }

        // تک‌محصولی: کل بهای تمام‌شده به همین محصول تخصیص می‌یابد، پس
        // معادله‌ی «مواد + هزینه = محصول» دقیقاً برقرار است.
        let unit = (total_cost as f64 / output_quantity).round() as i64;
        tx.execute(
            "INSERT INTO production_outputs\
             (id,order_id,product_id,quantity,market_unit_price,allocated_cost,unit_cost) \
             VALUES(?1,?2,?3,?4,NULL,?5,?6)",
            params![
                format!("{order_id}-out0"),
                order_id,
                output_product,
                output_quantity,
                total_cost,
                unit
            ],
        )?;

        for (position, (account, title, amount)) in expenses.iter().enumerate() {
            tx.execute(
                "INSERT INTO production_expenses(id,order_id,account_id,title,amount) \
                 VALUES(?1,?2,?3,?4,?5)",
                params![
                    format!("{order_id}-exp{position}"),
                    order_id,
                    account,
                    title,
                    amount
                ],
            )?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// انبارگردانی
// ---------------------------------------------------------------------------

/// یک دوره‌ی انبارگردانی «در حال شمارش» با اختلاف واقعی.
///
/// عمداً `posted` نیست: کاربر باید بتواند چرخه‌ی شمارش، شمارش مجدد، تأیید
/// اختلاف و ثبت سند تعدیل را خودش تمرین کند.
fn seed_stocktake(tx: &Connection, warehouse: &str) -> Result<()> {
    let session = "demo-count-001";
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM inventory_count_sessions WHERE id=?1",
        params![session],
        |row| row.get(0),
    )?;
    if exists > 0 {
        return Ok(());
    }

    tx.execute(
        "INSERT INTO inventory_count_sessions\
         (id,company_id,warehouse_id,title,count_date,status,created_by) \
         VALUES(?1,?2,?3,'انبارگردانی پایان مرداد ۱۴۰۵','1405/05/29','counting',?4)",
        params![session, COMPANY, warehouse, USER],
    )?;

    for index in 0..12usize {
        let product = format!("demo-prod-{index:03}");
        if !product_exists(tx, &product) {
            continue;
        }
        let system: f64 = tx
            .query_row(
                "SELECT COALESCE(SUM(quantity),0) FROM inventory_balances \
                 WHERE product_id=?1 AND warehouse_id=?2",
                params![product, warehouse],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        // الگوی قطعی: هر سه قلم یکی اختلاف دارد؛ بقیه دقیق شمرده شده‌اند.
        let (counted, status) = match index % 3 {
            0 => (Some(system + 2.0), "counted"),
            1 => (Some(system - 1.0), "counted"),
            _ => (None, "pending"),
        };

        tx.execute(
            "INSERT INTO inventory_count_lines\
             (id,session_id,product_id,system_quantity,counted_quantity,variance,status) \
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                format!("{session}-l{index:02}"),
                session,
                product,
                system,
                counted,
                counted.map(|value| value - system),
                status
            ],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// سری/بچ و سریال
// ---------------------------------------------------------------------------

fn seed_lots(tx: &Connection, warehouse: &str) -> Result<()> {
    for index in 0..6usize {
        let product = format!("demo-prod-{:03}", index * 3);
        if !product_exists(tx, &product) {
            continue;
        }
        let batch = index % 2 == 0;
        let id = format!("demo-lot-{index:03}");
        tx.execute(
            "INSERT OR IGNORE INTO inventory_lots\
             (id,company_id,product_id,warehouse_id,lot_number,lot_type,serial_number,\
              manufacture_date,expiry_date,quantity,unit_cost,status,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'active',?12)",
            params![
                id,
                COMPANY,
                product,
                warehouse,
                format!("LOT-1405-{:04}", 100 + index),
                if batch { "batch" } else { "serial" },
                if batch {
                    None
                } else {
                    Some(format!("SN-{:08}", 20_250_000 + index))
                },
                format!("1405/0{}/05", (index % 5) + 1),
                format!("1406/0{}/05", (index % 5) + 1),
                (index as f64 + 1.0) * 12.0,
                unit_cost(tx, &product),
                USER
            ],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// برگشت از خرید
// ---------------------------------------------------------------------------

/// سه برگشت از خرید، متصل به فاکتورهای خرید واقعی دمو.
fn seed_purchase_returns(tx: &Connection, warehouse: &str) -> Result<()> {
    for index in 0..3usize {
        let return_id = format!("demo-preturn-{index:03}");
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM purchase_returns WHERE id=?1",
            params![return_id],
            |row| row.get(0),
        )?;
        if exists > 0 {
            continue;
        }

        let invoice = format!("demo-purchase-{:03}", index * 4);
        let Ok((contact, _)) = tx.query_row(
            "SELECT COALESCE(contact_id,''), total FROM purchase_invoices WHERE id=?1",
            params![invoice],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        ) else {
            continue;
        };

        let Ok((product, unit_price)) = tx.query_row(
            "SELECT product_id, unit_price FROM purchase_invoice_lines WHERE invoice_id=?1 LIMIT 1",
            params![invoice],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        ) else {
            continue;
        };

        let quantity = (index + 1) as f64;
        let line_total = (unit_price as f64 * quantity).round() as i64;
        let number: i64 = tx.query_row(
            "SELECT COALESCE(MAX(number),0)+1 FROM purchase_returns \
             WHERE company_id=?1 AND fiscal_year_id=?2",
            params![COMPANY, FISCAL_YEAR],
            |row| row.get(0),
        )?;

        tx.execute(
            "INSERT INTO purchase_returns\
             (id,company_id,fiscal_year_id,number,return_date,original_invoice_id,contact_id,\
              warehouse_id,status,total,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                return_id,
                COMPANY,
                FISCAL_YEAR,
                number,
                format!("1405/0{}/2{}", index + 2, index + 1),
                invoice,
                if contact.is_empty() { None } else { Some(contact) },
                warehouse,
                if index == 0 { "draft" } else { "posted" },
                line_total,
                USER
            ],
        )?;
        tx.execute(
            "INSERT INTO purchase_return_lines(id,return_id,product_id,quantity,unit_price,line_total) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{return_id}-l0"),
                return_id,
                product,
                quantity,
                unit_price,
                line_total
            ],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// قالب چاپ، گزارش ذخیره‌شده، اتصال و افزونه
// ---------------------------------------------------------------------------

fn seed_print_templates(tx: &Connection) -> Result<()> {
    let templates: [(&str, &str, &str, i64, &str); 4] = [
        (
            "demo-tpl-invoice-a4",
            "فاکتور فروش A4",
            "invoice",
            1,
            "<section dir=\"rtl\"><h1>{{company.name}}</h1>\
             <h2>فاکتور فروش شماره {{invoice.number}}</h2>\
             <p>تاریخ: {{invoice.date}} — خریدار: {{party.name}}</p>\
             <table>{{#lines}}<tr><td>{{product.name}}</td><td>{{quantity}}</td>\
             <td>{{unit_price}}</td><td>{{line_total}}</td></tr>{{/lines}}</table>\
             <strong>جمع کل: {{invoice.total}} ریال</strong></section>",
        ),
        (
            "demo-tpl-receipt-80",
            "رسید حرارتی ۸۰ میلی‌متر",
            "receipt",
            1,
            "<section dir=\"rtl\" style=\"width:80mm\"><h3>{{company.name}}</h3>\
             <p>رسید {{document.number}} — {{document.date}}</p>\
             {{#lines}}<div>{{method}} — {{amount}}</div>{{/lines}}\
             <strong>{{document.total}} ریال</strong></section>",
        ),
        (
            "demo-tpl-journal",
            "سند حسابداری رسمی",
            "journal",
            1,
            "<section dir=\"rtl\"><h2>سند شماره {{journal.number}}</h2><p>{{journal.date}}</p>\
             <table>{{#lines}}<tr><td>{{account.code}}</td><td>{{account.name}}</td>\
             <td>{{debit}}</td><td>{{credit}}</td></tr>{{/lines}}</table></section>",
        ),
        (
            "demo-tpl-label",
            "برچسب قفسه کالا",
            "label",
            1,
            "<section dir=\"rtl\" style=\"width:50mm\"><b>{{product.name}}</b>\
             <div>{{product.sku}}</div><div>{{product.price}} ریال</div></section>",
        ),
    ];
    for (id, name, kind, is_default, html) in templates {
        tx.execute(
            "INSERT OR IGNORE INTO print_templates\
             (id,company_id,name,template_type,content_html,is_default,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![id, COMPANY, name, kind, html, is_default, USER],
        )?;
    }
    Ok(())
}

fn seed_custom_reports(tx: &Connection) -> Result<()> {
    let reports: [(&str, &str, &str, &str); 3] = [
        (
            "demo-report-sales-by-customer",
            "فروش به تفکیک مشتری",
            "sales",
            "{\"columns\":[\"contact_name\",\"total\",\"tax\"],\"groupBy\":\"contact_name\",\
             \"sort\":\"total\",\"direction\":\"desc\",\"search\":\"\"}",
        ),
        (
            "demo-report-purchase-monthly",
            "خرید ماهانه",
            "purchase",
            "{\"columns\":[\"date\",\"invoice_number\",\"total\"],\"groupBy\":\"date\",\
             \"sort\":\"date\",\"direction\":\"asc\",\"search\":\"\"}",
        ),
        (
            "demo-report-inventory-value",
            "ارزش موجودی انبارها",
            "inventory",
            "{\"columns\":[\"product_name\",\"warehouse_name\",\"quantity\",\"value\"],\
             \"groupBy\":\"warehouse_name\",\"sort\":\"value\",\"direction\":\"desc\",\"search\":\"\"}",
        ),
    ];
    for (id, name, source, config) in reports {
        tx.execute(
            "INSERT OR IGNORE INTO custom_reports(id,company_id,name,source,config_json,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, COMPANY, name, source, config, USER],
        )?;
    }
    Ok(())
}

fn seed_integrations(tx: &Connection) -> Result<()> {
    let profiles: [(&str, &str, &str, &str, Option<&str>, i64, i64, &str); 3] = [
        (
            "demo-api-tax",
            "سامانه مؤدیان — کارپوشه",
            "https://tp.tax.gov.ir",
            "bearer",
            Some("Authorization"),
            15_000,
            1,
            "tax.gov.ir",
        ),
        (
            "demo-api-sms",
            "پیامک یادآوری چک",
            "https://api.sms-provider.ir",
            "api_key",
            Some("X-API-KEY"),
            8_000,
            1,
            "sms-provider.ir",
        ),
        (
            "demo-api-sayad",
            "استعلام صیادی",
            "https://sayad.cbi.ir",
            "basic",
            None,
            12_000,
            0,
            "cbi.ir",
        ),
    ];
    for (id, name, url, auth, header, timeout, enabled, domains) in profiles {
        tx.execute(
            "INSERT OR IGNORE INTO api_profiles\
             (id,company_id,name,base_url,auth_type,auth_header,timeout_ms,enabled,allowed_domains) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, COMPANY, name, url, auth, header, timeout, enabled, domains],
        )?;
    }

    let plugins: [(&str, &str, &str, &str, i64, [&str; 2]); 2] = [
        (
            "demo-plugin-barcode",
            "اسکنر بارکد USB",
            "1.2.0",
            "خواندن بارکد از دستگاه‌های HID و درج خودکار در سطر فاکتور",
            1,
            ["plugins.execute", "integrations.view"],
        ),
        (
            "demo-plugin-pos",
            "درگاه کارتخوان",
            "2.0.1",
            "اتصال به پایانه فروشگاهی و ثبت خودکار سند دریافت",
            0,
            ["plugins.execute", "native.execute"],
        ),
    ];
    for (id, name, version, description, enabled, permissions) in plugins {
        tx.execute(
            "INSERT OR IGNORE INTO plugins\
             (id,company_id,name,version,description,entrypoint,manifest_json,enabled) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                id,
                COMPANY,
                name,
                version,
                description,
                format!("plugins/{id}/main.exe"),
                format!(
                    "{{\"id\":\"{id}\",\"name\":\"{name}\",\"version\":\"{version}\",\"permissions\":[\"{}\",\"{}\"]}}",
                    permissions[0], permissions[1]
                ),
                enabled
            ],
        )?;
        for permission in permissions {
            tx.execute(
                "INSERT OR IGNORE INTO plugin_permissions(plugin_id,permission) VALUES(?1,?2)",
                params![id, permission],
            )?;
        }
    }
    Ok(())
}
