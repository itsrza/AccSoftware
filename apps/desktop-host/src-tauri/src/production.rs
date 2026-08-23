//! رسید تولید و فرمول تولید.
//!
//! مرجع: تصویر `3qTCnS` (رسید تولید با سه زبانه).
//!
//! ## معادله‌ی محوری
//!
//! ```text
//! جمع مواد مصرفی + جمع هزینه‌های تولید = جمع بهای تمام‌شده‌ی محصولات
//! ```
//!
//! ## سند حسابداری تولید
//!
//! ```text
//! بدهکار  موجودی کالای ساخته‌شده     بهای تمام‌شده‌ی محصولات
//! بستانکار موجودی مواد اولیه          ارزش مواد مصرفی
//! بستانکار حساب‌های هزینه‌ی تولید      دستمزد و سربار
//! ```
//!
//! **تولید سود نمی‌سازد.** فقط شکل دارایی از «مواد اولیه» به «کالای
//! ساخته‌شده» تغییر می‌کند. سود در لحظه‌ی فروش محقق می‌شود، نه در لحظه‌ی
//! تولید. هر سندی که در تولید درآمد ثبت کند، سود ساختگی می‌سازد.
//!
//! ## چرا بهای مواد از کاردکس خوانده می‌شود
//!
//! بهای تمام‌شده‌ی ماده‌ی مصرفی باید بهای واقعی همان ماده در انبار باشد، نه
//! قیمت خرید فعلی بازار. اگر قیمت روز استفاده شود، تفاوت آن به سود یا زیان
//! ساختگی تبدیل می‌شود.

use novin_core::money::Money;
use novin_core::production::{
    assert_cost_balance, calculate_costing, producible_quantity, ConsumedMaterial, CostAllocation,
    FormulaComponent, ProducedItem, ProductionExpense, ProductionFormula,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{active_context, audit, conn, next_journal_number, require_permission, AppState};

// ---------------------------------------------------------------------------
// فرمول تولید
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct FormulaComponentInput {
    pub product_id: String,
    pub quantity_per_unit: f64,
    #[serde(default)]
    pub waste_percent: f64,
}

#[derive(Debug, Deserialize)]
pub struct FormulaInput {
    #[serde(default)]
    pub id: Option<String>,
    pub product_id: String,
    pub title: String,
    pub output_quantity: f64,
    pub components: Vec<FormulaComponentInput>,
}

#[derive(Debug, Serialize)]
pub struct FormulaRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub title: String,
    pub output_quantity: f64,
    pub is_active: bool,
    pub component_count: i64,
    /// بهای تمام‌شده‌ی برآوردی یک واحد بر اساس بهای فعلی مواد.
    pub estimated_unit_cost: i64,
    /// بیشترین مقدار قابل تولید با موجودی فعلی همه‌ی انبارها.
    pub producible_now: f64,
}

#[derive(Debug, Serialize)]
pub struct FormulaComponentRow {
    pub id: String,
    pub product_id: String,
    pub product_name: String,
    pub unit: String,
    pub quantity_per_unit: f64,
    pub waste_percent: f64,
    pub unit_cost: i64,
    /// مصرف واقعی یک واحد محصول، با احتساب ضایعات.
    pub effective_quantity: f64,
    pub available_stock: f64,
}

#[derive(Debug, Serialize)]
pub struct FormulaDetail {
    pub header: FormulaRow,
    pub components: Vec<FormulaComponentRow>,
}

fn load_formula(
    tx: &rusqlite::Transaction<'_>,
    formula_id: &str,
) -> Result<ProductionFormula, String> {
    let (product_id, title, output_quantity): (String, String, f64) = tx
        .query_row(
            "SELECT product_id,title,output_quantity FROM production_formulas WHERE id=?1",
            params![formula_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "PRD-001: فرمول تولید یافت نشد".to_string())?;

    let mut statement = tx
        .prepare(
            "SELECT product_id,quantity_per_unit,waste_percent \
             FROM production_formula_components WHERE formula_id=?1 ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let components = statement
        .query_map(params![formula_id], |row| {
            Ok(FormulaComponent {
                product_id: row.get(0)?,
                quantity_per_unit: row.get(1)?,
                waste_percent: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    Ok(ProductionFormula {
        product_id,
        title,
        output_quantity,
        components,
    })
}

/// موجودی کل یک کالا در همه‌ی انبارهای شرکت.
fn total_stock(tx: &rusqlite::Transaction<'_>, product_id: &str) -> f64 {
    tx.query_row(
        "SELECT COALESCE(SUM(quantity),0) FROM inventory_balances WHERE product_id=?1",
        params![product_id],
        |row| row.get(0),
    )
    .unwrap_or(0.0)
}

/// بهای تمام‌شده‌ی واحد یک کالا — از قیمت خرید ثبت‌شده در کاتالوگ.
fn unit_cost_of(tx: &rusqlite::Transaction<'_>, product_id: &str) -> i64 {
    tx.query_row(
        "SELECT purchase_price FROM products WHERE id=?1",
        params![product_id],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// ذخیره‌ی فرمول تولید با اعتبارسنجی کامل هسته.
#[tauri::command]
pub fn save_production_formula(
    state: State<AppState>,
    input: FormulaInput,
) -> Result<String, String> {
    let formula = ProductionFormula {
        product_id: input.product_id.clone(),
        title: input.title.trim().to_string(),
        output_quantity: input.output_quantity,
        components: input
            .components
            .iter()
            .map(|component| FormulaComponent {
                product_id: component.product_id.clone(),
                quantity_per_unit: component.quantity_per_unit,
                waste_percent: component.waste_percent,
            })
            .collect(),
    };
    formula
        .validate()
        .map_err(|error| format!("PRD-002: {error}"))?;
    if formula.title.is_empty() {
        return Err("PRD-003: عنوان فرمول الزامی است".into());
    }

    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    // همه‌ی کالاهای فرمول باید متعلق به همین شرکت باشند.
    let mut all_products = vec![formula.product_id.clone()];
    all_products.extend(formula.components.iter().map(|c| c.product_id.clone()));
    for product in &all_products {
        let ok: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id=?1 AND company_id=?2",
                params![product, company],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if ok == 0 {
            return Err(format!("PRD-004: کالای «{product}» معتبر نیست"));
        }
    }

    let formula_id = match &input.id {
        Some(existing) => {
            tx.execute(
                "UPDATE production_formulas SET product_id=?1,title=?2,output_quantity=?3 \
                 WHERE id=?4 AND company_id=?5",
                params![
                    formula.product_id,
                    formula.title,
                    formula.output_quantity,
                    existing,
                    company
                ],
            )
            .map_err(|e| format!("PRD-005: {e}"))?;
            tx.execute(
                "DELETE FROM production_formula_components WHERE formula_id=?1",
                params![existing],
            )
            .map_err(|e| e.to_string())?;
            existing.clone()
        }
        None => {
            let new_id = format!(
                "formula-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
            );
            tx.execute(
                "INSERT INTO production_formulas(id,company_id,product_id,title,output_quantity) \
                 VALUES(?1,?2,?3,?4,?5)",
                params![
                    new_id,
                    company,
                    formula.product_id,
                    formula.title,
                    formula.output_quantity
                ],
            )
            .map_err(|e| format!("PRD-006: {e}"))?;
            new_id
        }
    };

    for (index, component) in formula.components.iter().enumerate() {
        tx.execute(
            "INSERT INTO production_formula_components(id,formula_id,product_id,\
             quantity_per_unit,waste_percent) VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{formula_id}-c{index}"),
                formula_id,
                component.product_id,
                component.quantity_per_unit,
                component.waste_percent
            ],
        )
        .map_err(|e| format!("PRD-007: {e}"))?;
    }

    audit(
        &tx,
        &user,
        "production.formula.save",
        "production_formula",
        &formula_id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(formula_id)
}

/// فهرست فرمول‌ها با بهای برآوردی و ظرفیت تولید فعلی.
#[tauri::command]
pub fn list_production_formulas(state: State<AppState>) -> Result<Vec<FormulaRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let mut statement = tx
        .prepare(
            "SELECT f.id,f.product_id,p.name,f.title,f.output_quantity,f.is_active,\
             (SELECT COUNT(*) FROM production_formula_components k WHERE k.formula_id=f.id) \
             FROM production_formulas f JOIN products p ON p.id=f.product_id \
             WHERE f.company_id=?1 ORDER BY p.name, f.title",
        )
        .map_err(|e| e.to_string())?;
    let raw: Vec<(String, String, String, String, f64, i64, i64)> = statement
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

    let mut rows = Vec::with_capacity(raw.len());
    for (id, product_id, product_name, title, output_quantity, is_active, component_count) in raw {
        let formula = load_formula(&tx, &id)?;
        // بهای برآوردی یک واحد = جمع (مصرف با ضایعات × بهای ماده).
        let estimated = formula
            .expand(formula.output_quantity)
            .map(|components| {
                components
                    .iter()
                    .map(|(product, quantity)| {
                        (quantity * unit_cost_of(&tx, product) as f64).round() as i64
                    })
                    .sum::<i64>()
            })
            .unwrap_or(0);
        let stock: Vec<(String, f64)> = formula
            .components
            .iter()
            .map(|component| {
                (
                    component.product_id.clone(),
                    total_stock(&tx, &component.product_id),
                )
            })
            .collect();
        rows.push(FormulaRow {
            id,
            product_id,
            product_name,
            title,
            output_quantity,
            is_active: is_active == 1,
            component_count,
            estimated_unit_cost: if formula.output_quantity > 0.0 {
                (estimated as f64 / formula.output_quantity).round() as i64
            } else {
                0
            },
            producible_now: producible_quantity(&formula, &stock).unwrap_or(0.0),
        });
    }
    Ok(rows)
}

/// جزئیات یک فرمول با موجودی و بهای هر جزء.
#[tauri::command]
pub fn get_production_formula(
    state: State<AppState>,
    id: String,
) -> Result<FormulaDetail, String> {
    let all = list_production_formulas(state.clone())?;
    let header = all
        .into_iter()
        .find(|row| row.id == id)
        .ok_or_else(|| "PRD-001: فرمول تولید یافت نشد".to_string())?;

    let mut c = conn(&state)?;
    require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;

    let mut statement = tx
        .prepare(
            "SELECT k.id,k.product_id,p.name,p.unit,k.quantity_per_unit,k.waste_percent \
             FROM production_formula_components k JOIN products p ON p.id=k.product_id \
             WHERE k.formula_id=?1 ORDER BY p.name",
        )
        .map_err(|e| e.to_string())?;
    let raw: Vec<(String, String, String, String, f64, f64)> = statement
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

    let components = raw
        .into_iter()
        .map(
            |(component_id, product_id, product_name, unit, quantity, waste)| {
                FormulaComponentRow {
                    effective_quantity: quantity * (1.0 + waste / 100.0),
                    unit_cost: unit_cost_of(&tx, &product_id),
                    available_stock: total_stock(&tx, &product_id),
                    id: component_id,
                    product_id,
                    product_name,
                    unit,
                    quantity_per_unit: quantity,
                    waste_percent: waste,
                }
            },
        )
        .collect();

    Ok(FormulaDetail { header, components })
}

// ---------------------------------------------------------------------------
// رسید تولید
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct InputLine {
    pub product_id: String,
    pub quantity: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputLine {
    pub product_id: String,
    pub quantity: f64,
    #[serde(default)]
    pub market_unit_price: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpenseLine {
    pub account_id: String,
    pub title: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProductionInput {
    pub production_date: String,
    pub warehouse_id: String,
    #[serde(default)]
    pub formula_id: Option<String>,
    pub cost_allocation: String,
    #[serde(default)]
    pub description: Option<String>,
    pub inputs: Vec<InputLine>,
    pub outputs: Vec<OutputLine>,
    #[serde(default)]
    pub expenses: Vec<ExpenseLine>,
}

#[derive(Debug, Serialize)]
pub struct CostingPreview {
    pub materials_total: i64,
    pub expenses_total: i64,
    pub total_cost: i64,
    pub outputs: Vec<OutputCostRow>,
    /// هشدارهایی که مانع ثبت نیستند ولی باید دیده شوند.
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OutputCostRow {
    pub product_id: String,
    pub product_name: String,
    pub quantity: f64,
    pub allocated_cost: i64,
    pub unit_cost: i64,
    /// بهای تمام‌شده‌ی قبلی این کالا، برای مقایسه.
    pub previous_unit_cost: i64,
}

fn build_costing(
    tx: &rusqlite::Transaction<'_>,
    input: &ProductionInput,
) -> Result<(CostingPreview, Vec<ConsumedMaterial>), String> {
    let allocation = CostAllocation::parse(&input.cost_allocation)
        .ok_or_else(|| "PRD-008: روش تخصیص بهای تمام‌شده نامعتبر است".to_string())?;

    let materials: Vec<ConsumedMaterial> = input
        .inputs
        .iter()
        .map(|line| ConsumedMaterial {
            unit_cost: Money::from_rials(unit_cost_of(tx, &line.product_id)),
            product_id: line.product_id.clone(),
            quantity: line.quantity,
        })
        .collect();
    let outputs: Vec<ProducedItem> = input
        .outputs
        .iter()
        .map(|line| ProducedItem {
            product_id: line.product_id.clone(),
            quantity: line.quantity,
            market_unit_price: line.market_unit_price.map(Money::from_rials),
        })
        .collect();
    let expenses: Vec<ProductionExpense> = input
        .expenses
        .iter()
        .map(|line| ProductionExpense {
            account_id: line.account_id.clone(),
            title: line.title.clone(),
            amount: Money::from_rials(line.amount),
        })
        .collect();

    let costing = calculate_costing(&materials, &expenses, &outputs, allocation)
        .map_err(|error| format!("PRD-009: {error}"))?;
    assert_cost_balance(&costing).map_err(|error| format!("PRD-010: {error}"))?;

    let mut warnings = Vec::new();
    // هشدار موجودی: مصرف بیش از موجودی، تولید را غیرممکن می‌کند.
    for material in &materials {
        let available: f64 = tx
            .query_row(
                "SELECT COALESCE(quantity,0) FROM inventory_balances \
                 WHERE product_id=?1 AND warehouse_id=?2",
                params![material.product_id, input.warehouse_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        if available < material.quantity {
            let name: String = tx
                .query_row(
                    "SELECT name FROM products WHERE id=?1",
                    params![material.product_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| material.product_id.clone());
            warnings.push(format!(
                "موجودی «{name}» در این انبار {available} است ولی {} واحد مصرف شده",
                material.quantity
            ));
        }
    }

    let output_rows = costing
        .outputs
        .iter()
        .map(|output| {
            let name: String = tx
                .query_row(
                    "SELECT name FROM products WHERE id=?1",
                    params![output.product_id],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| output.product_id.clone());
            OutputCostRow {
                previous_unit_cost: unit_cost_of(tx, &output.product_id),
                product_id: output.product_id.clone(),
                product_name: name,
                quantity: output.quantity,
                allocated_cost: output.allocated_cost.rials(),
                unit_cost: output.unit_cost.rials(),
            }
        })
        .collect();

    Ok((
        CostingPreview {
            materials_total: costing.materials_total.rials(),
            expenses_total: costing.expenses_total.rials(),
            total_cost: costing.total_cost.rials(),
            outputs: output_rows,
            warnings,
        },
        materials,
    ))
}

/// پیش‌نمایش بهای تمام‌شده بدون نوشتن در پایگاه داده.
#[tauri::command]
pub fn preview_production(
    state: State<AppState>,
    input: ProductionInput,
) -> Result<CostingPreview, String> {
    let mut c = conn(&state)?;
    require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (preview, _) = build_costing(&tx, &input)?;
    Ok(preview)
}

/// گسترش فرمول: چه مقدار از هر ماده برای تولید مقدار مشخص لازم است.
#[derive(Debug, Serialize)]
pub struct ExpandedComponent {
    pub product_id: String,
    pub product_name: String,
    pub unit: String,
    pub required_quantity: f64,
    pub available_stock: f64,
    pub unit_cost: i64,
}

#[tauri::command]
pub fn expand_production_formula(
    state: State<AppState>,
    formula_id: String,
    output_quantity: f64,
) -> Result<Vec<ExpandedComponent>, String> {
    let mut c = conn(&state)?;
    require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let formula = load_formula(&tx, &formula_id)?;
    let expanded = formula
        .expand(output_quantity)
        .map_err(|error| format!("PRD-011: {error}"))?;

    Ok(expanded
        .into_iter()
        .map(|(product_id, quantity)| {
            let (name, unit): (String, String) = tx
                .query_row(
                    "SELECT name,unit FROM products WHERE id=?1",
                    params![product_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap_or_else(|_| (product_id.clone(), String::new()));
            ExpandedComponent {
                available_stock: total_stock(&tx, &product_id),
                unit_cost: unit_cost_of(&tx, &product_id),
                product_id,
                product_name: name,
                unit,
                required_quantity: quantity,
            }
        })
        .collect())
}

/// ثبت رسید تولید: خروج مواد، ورود محصول، و صدور سند حسابداری.
#[tauri::command]
pub fn post_production(state: State<AppState>, input: ProductionInput) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    crate::validate_fiscal_date(&tx, &fy, &input.production_date)?;

    let warehouse_ok: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM warehouses WHERE id=?1 AND company_id=?2 AND is_active=1",
            params![input.warehouse_id, company],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if warehouse_ok == 0 {
        return Err("PRD-012: انبار تولید معتبر نیست".into());
    }

    let (preview, materials) = build_costing(&tx, &input)?;
    // برخلاف پیش‌نمایش، اینجا کمبود موجودی خطاست: نمی‌شود ماده‌ای را مصرف
    // کرد که در انبار نیست.
    if !preview.warnings.is_empty() {
        return Err(format!("PRD-013: {}", preview.warnings.join(" — ")));
    }

    let number: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(number),0)+1 FROM production_orders \
             WHERE company_id=?1 AND fiscal_year_id=?2",
            params![company, fy],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let order_id = format!(
        "production-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let description = input
        .description
        .clone()
        .unwrap_or_else(|| format!("رسید تولید شماره {number}"));

    // ---- سند حسابداری ----
    let journal_id = format!("journal-production-{order_id}");
    let journal_number = next_journal_number(&tx, &company, &fy)?;
    tx.execute(
        "INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,\
         status,source_type,source_id,created_by) VALUES(?1,?2,?3,?4,?5,?6,'posted','production',?7,?8)",
        params![
            journal_id,
            company,
            fy,
            journal_number,
            input.production_date,
            description,
            order_id,
            user
        ],
    )
    .map_err(|e| format!("PRD-014: {e}"))?;

    let mut journal_lines: Vec<(String, i64, i64)> = Vec::new();
    // بدهکار: موجودی کالای ساخته‌شده به اندازه‌ی کل بهای تمام‌شده
    journal_lines.push(("acc-1320".to_string(), preview.total_cost, 0));
    // بستانکار: موجودی مواد اولیه
    if preview.materials_total > 0 {
        journal_lines.push(("acc-1310".to_string(), 0, preview.materials_total));
    }
    // بستانکار: هر حساب هزینه به‌طور جداگانه، تا گزارش بهای تمام‌شده تفکیک داشته باشد
    for expense in &input.expenses {
        if expense.amount > 0 {
            journal_lines.push((expense.account_id.clone(), 0, expense.amount));
        }
    }

    let debit: i64 = journal_lines.iter().map(|(_, d, _)| *d).sum();
    let credit: i64 = journal_lines.iter().map(|(_, _, c)| *c).sum();
    if debit != credit {
        return Err(format!(
            "PRD-015: سند تولید متوازن نیست (بدهکار {debit} در برابر بستانکار {credit})"
        ));
    }

    for (index, (account, line_debit, line_credit)) in journal_lines.iter().enumerate() {
        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id=?1 AND company_id=?2",
                params![account, company],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if exists == 0 {
            return Err(format!("PRD-016: حساب «{account}» در کدینگ تعریف نشده است"));
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
        .map_err(|e| format!("PRD-017: {e}"))?;
    }

    // ---- سربرگ رسید ----
    tx.execute(
        "INSERT INTO production_orders(id,company_id,fiscal_year_id,number,production_date,\
         warehouse_id,formula_id,cost_allocation,materials_total,expenses_total,total_cost,\
         status,description,journal_id,created_by) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'posted',?12,?13,?14)",
        params![
            order_id,
            company,
            fy,
            number,
            input.production_date,
            input.warehouse_id,
            input.formula_id,
            input.cost_allocation,
            preview.materials_total,
            preview.expenses_total,
            preview.total_cost,
            input.description,
            journal_id,
            user
        ],
    )
    .map_err(|e| format!("PRD-018: {e}"))?;

    // ---- خروج مواد از انبار ----
    for (index, material) in materials.iter().enumerate() {
        let line_total = (material.quantity * material.unit_cost.rials() as f64).round() as i64;
        tx.execute(
            "INSERT INTO production_inputs(id,order_id,product_id,quantity,unit_cost,line_total) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{order_id}-in{index}"),
                order_id,
                material.product_id,
                material.quantity,
                material.unit_cost.rials(),
                line_total
            ],
        )
        .map_err(|e| format!("PRD-019: {e}"))?;

        tx.execute(
            "UPDATE inventory_balances SET quantity=quantity-?1,updated_at=CURRENT_TIMESTAMP \
             WHERE product_id=?2 AND warehouse_id=?3",
            params![material.quantity, material.product_id, input.warehouse_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,\
             quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,'issue',?5,?6,'production',?7,'مصرف در تولید',?8)",
            params![
                format!("{order_id}-mv-in{index}"),
                company,
                material.product_id,
                input.warehouse_id,
                material.quantity,
                material.unit_cost.rials(),
                order_id,
                user
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    // ---- ورود محصولات به انبار ----
    for (index, output) in preview.outputs.iter().enumerate() {
        tx.execute(
            "INSERT INTO production_outputs(id,order_id,product_id,quantity,market_unit_price,\
             allocated_cost,unit_cost) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                format!("{order_id}-out{index}"),
                order_id,
                output.product_id,
                output.quantity,
                input
                    .outputs
                    .iter()
                    .find(|line| line.product_id == output.product_id)
                    .and_then(|line| line.market_unit_price),
                output.allocated_cost,
                output.unit_cost
            ],
        )
        .map_err(|e| format!("PRD-020: {e}"))?;

        tx.execute(
            "INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?1,?2,?3) \
             ON CONFLICT(product_id,warehouse_id) DO UPDATE SET \
             quantity=quantity+excluded.quantity,updated_at=CURRENT_TIMESTAMP",
            params![output.product_id, input.warehouse_id, output.quantity],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,\
             quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,'receipt',?5,?6,'production',?7,'تولید محصول',?8)",
            params![
                format!("{order_id}-mv-out{index}"),
                company,
                output.product_id,
                input.warehouse_id,
                output.quantity,
                output.unit_cost,
                order_id,
                user
            ],
        )
        .map_err(|e| e.to_string())?;

        // بهای تمام‌شده‌ی محصول به‌روز می‌شود تا فروش بعدی سود واقعی بدهد.
        tx.execute(
            "UPDATE products SET purchase_price=?1 WHERE id=?2",
            params![output.unit_cost, output.product_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // ---- هزینه‌ها ----
    for (index, expense) in input.expenses.iter().enumerate() {
        tx.execute(
            "INSERT INTO production_expenses(id,order_id,account_id,title,amount) \
             VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{order_id}-exp{index}"),
                order_id,
                expense.account_id,
                expense.title,
                expense.amount
            ],
        )
        .map_err(|e| format!("PRD-021: {e}"))?;
    }

    audit(
        &tx,
        &user,
        "production.post",
        "production_order",
        &order_id,
        None,
        Some(&format!("{{\"total_cost\":{}}}", preview.total_cost)),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(order_id)
}

/// فهرست رسیدهای تولید.
#[derive(Debug, Serialize)]
pub struct ProductionOrderRow {
    pub id: String,
    pub number: i64,
    pub production_date: String,
    pub warehouse_name: String,
    pub materials_total: i64,
    pub expenses_total: i64,
    pub total_cost: i64,
    pub status: String,
    pub description: Option<String>,
    pub journal_id: Option<String>,
    pub input_count: i64,
    pub output_count: i64,
}

#[tauri::command]
pub fn list_production_orders(
    state: State<AppState>,
) -> Result<Vec<ProductionOrderRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;

    let mut statement = tx
        .prepare(
            "SELECT o.id,o.number,o.production_date,w.name,o.materials_total,o.expenses_total,\
             o.total_cost,o.status,o.description,o.journal_id,\
             (SELECT COUNT(*) FROM production_inputs i WHERE i.order_id=o.id),\
             (SELECT COUNT(*) FROM production_outputs p WHERE p.order_id=o.id) \
             FROM production_orders o JOIN warehouses w ON w.id=o.warehouse_id \
             WHERE o.company_id=?1 AND o.fiscal_year_id=?2 ORDER BY o.number DESC LIMIT 500",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![company, fy], |row| {
            Ok(ProductionOrderRow {
                id: row.get(0)?,
                number: row.get(1)?,
                production_date: row.get(2)?,
                warehouse_name: row.get(3)?,
                materials_total: row.get(4)?,
                expenses_total: row.get(5)?,
                total_cost: row.get(6)?,
                status: row.get(7)?,
                description: row.get(8)?,
                journal_id: row.get(9)?,
                input_count: row.get(10)?,
                output_count: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// روش‌های تخصیص بهای تمام‌شده با توضیح — رابط کاربری فهرست را از اینجا می‌گیرد.
#[derive(Debug, Serialize)]
pub struct AllocationInfo {
    pub value: String,
    pub label: String,
    pub explanation: String,
}

#[tauri::command]
pub fn list_cost_allocations() -> Vec<AllocationInfo> {
    [CostAllocation::ByQuantity, CostAllocation::ByMarketValue]
        .into_iter()
        .map(|value| AllocationInfo {
            value: value.as_str().to_string(),
            label: value.label().to_string(),
            explanation: value.explanation().to_string(),
        })
        .collect()
}

/// حساب‌های هزینه‌ی قابل استفاده در رسید تولید.
#[derive(Debug, Serialize)]
pub struct ExpenseAccountRow {
    pub id: String,
    pub code: String,
    pub name: String,
}

#[tauri::command]
pub fn list_production_expense_accounts(
    state: State<AppState>,
) -> Result<Vec<ExpenseAccountRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    // فقط حساب‌های زیرمجموعه‌ی بهای تمام‌شده و هزینه‌ها معنا دارند.
    let mut statement = tx
        .prepare(
            "SELECT id,code,name FROM accounts \
             WHERE company_id=?1 AND is_active=1 AND nature='debit' \
             AND (code LIKE '5%' OR code LIKE '6%') AND level <> 'group' ORDER BY code",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![company], |row| {
            Ok(ExpenseAccountRow {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// حذف فرمول تولید — فقط اگر در هیچ رسیدی استفاده نشده باشد.
#[tauri::command]
pub fn delete_production_formula(state: State<AppState>, id: String) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "inventory.transfer")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let used: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM production_orders WHERE formula_id=?1",
            params![id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if used > 0 {
        // غیرفعال می‌کنیم تا رسیدهای قبلی مرجعشان را از دست ندهند.
        tx.execute(
            "UPDATE production_formulas SET is_active=0 WHERE id=?1 AND company_id=?2",
            params![id, company],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let exists: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM production_formulas WHERE id=?1 AND company_id=?2",
                params![id, company],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err("PRD-001: فرمول تولید یافت نشد".into());
        }
        tx.execute("DELETE FROM production_formulas WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
    }
    audit(
        &tx,
        &user,
        "production.formula.delete",
        "production_formula",
        &id,
        None,
        None,
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
