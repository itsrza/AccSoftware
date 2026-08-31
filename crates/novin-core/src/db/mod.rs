//! لایه‌ی پایگاه داده: باز کردن اتصال، مهاجرت‌های نسخه‌ای و داده‌ی نمونه.
//!
//! تمام دسترسی به SQLite از این ماژول عبور می‌کند تا PRAGMAها، مهاجرت‌ها و
//! یکپارچگی داده در یک نقطه تضمین شود.

pub mod demo;
pub mod demo_extras;

use rusqlite::{Connection, Result};
use std::path::Path;

/// باز کردن پایگاه داده روی دیسک و اجرای مهاجرت‌ها.
pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&conn)?;
    Ok(conn)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// اجرای مهاجرت‌های نسخه‌ای پایگاه داده.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
    CREATE TABLE IF NOT EXISTS schema_migrations(
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS companies(
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      national_id TEXT,
      phone TEXT,
      address TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS fiscal_years(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      title TEXT NOT NULL,
      start_date TEXT NOT NULL,
      end_date TEXT NOT NULL,
      is_closed INTEGER NOT NULL DEFAULT 0,
      UNIQUE(company_id,title)
    );
    CREATE TABLE IF NOT EXISTS users(
      id TEXT PRIMARY KEY,
      username TEXT NOT NULL UNIQUE,
      display_name TEXT NOT NULL,
      password_hash TEXT NOT NULL,
      is_active INTEGER NOT NULL DEFAULT 1,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS roles(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE);
    CREATE TABLE IF NOT EXISTS permissions(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE);
    CREATE TABLE IF NOT EXISTS user_roles(user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,PRIMARY KEY(user_id,role_id));
    CREATE TABLE IF NOT EXISTS role_permissions(role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,PRIMARY KEY(role_id,permission_id));
    CREATE TABLE IF NOT EXISTS accounts(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      name TEXT NOT NULL,
      level TEXT NOT NULL CHECK(level IN ('group','general','subsidiary','detail')),
      parent_id TEXT REFERENCES accounts(id),
      nature TEXT NOT NULL DEFAULT 'debit' CHECK(nature IN ('debit','credit','mixed')),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id,code)
    );
    CREATE TABLE IF NOT EXISTS journal_entries(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      number INTEGER NOT NULL,
      entry_date TEXT NOT NULL,
      description TEXT NOT NULL,
      status TEXT NOT NULL CHECK(status IN ('draft','validated','posted','reversed')),
      source_type TEXT,
      source_id TEXT,
      created_by TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,fiscal_year_id,number)
    );
    CREATE TABLE IF NOT EXISTS journal_lines(
      id TEXT PRIMARY KEY,
      journal_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
      account_id TEXT NOT NULL REFERENCES accounts(id),
      description TEXT,
      debit INTEGER NOT NULL DEFAULT 0 CHECK(debit >= 0),
      credit INTEGER NOT NULL DEFAULT 0 CHECK(credit >= 0),
      CHECK((debit > 0 AND credit = 0) OR (credit > 0 AND debit = 0))
    );
    CREATE INDEX IF NOT EXISTS idx_journal_lines_account ON journal_lines(account_id);
    CREATE INDEX IF NOT EXISTS idx_journal_entries_date ON journal_entries(entry_date);
    CREATE TABLE IF NOT EXISTS contacts(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      kind TEXT NOT NULL CHECK(kind IN ('person','company')),
      name TEXT NOT NULL, national_id TEXT, phone TEXT, mobile TEXT, address TEXT,
      is_customer INTEGER NOT NULL DEFAULT 0, is_supplier INTEGER NOT NULL DEFAULT 0,
      credit_limit INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_contacts_name ON contacts(company_id,name);
    CREATE TABLE IF NOT EXISTS products(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      sku TEXT NOT NULL, barcode TEXT, name TEXT NOT NULL, unit TEXT NOT NULL,
      sale_price INTEGER NOT NULL DEFAULT 0, purchase_price INTEGER NOT NULL DEFAULT 0,
      min_stock REAL NOT NULL DEFAULT 0, is_service INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(company_id,sku)
    );
    CREATE TABLE IF NOT EXISTS warehouses(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      name TEXT NOT NULL, code TEXT NOT NULL, is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id,code)
    );
    CREATE TABLE IF NOT EXISTS app_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS print_templates(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      name TEXT NOT NULL, template_type TEXT NOT NULL CHECK(template_type IN ('invoice','receipt','journal','report','label')),
      content_html TEXT NOT NULL, is_default INTEGER NOT NULL DEFAULT 0,
      created_by TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,name)
    );
    CREATE INDEX IF NOT EXISTS idx_print_templates_company_type ON print_templates(company_id,template_type);
    CREATE TABLE IF NOT EXISTS import_batches(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      entity_type TEXT NOT NULL, row_count INTEGER NOT NULL, status TEXT NOT NULL,
      error_message TEXT, created_by TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS audit_logs(
      id TEXT PRIMARY KEY,user_id TEXT,action TEXT NOT NULL,entity_type TEXT,entity_id TEXT,
      before_json TEXT,after_json TEXT,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS company_users(
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
      is_active INTEGER NOT NULL DEFAULT 1,
      PRIMARY KEY(company_id,user_id)
    );
    CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_logs(entity_type,entity_id,created_at);
    CREATE INDEX IF NOT EXISTS idx_journal_company_date ON journal_entries(company_id,entry_date);
    CREATE INDEX IF NOT EXISTS idx_accounts_company_parent ON accounts(company_id,parent_id);
    CREATE INDEX IF NOT EXISTS idx_products_company_barcode ON products(company_id,barcode);
    CREATE INDEX IF NOT EXISTS idx_warehouses_company ON warehouses(company_id);
    CREATE TABLE IF NOT EXISTS inventory_balances(
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      warehouse_id TEXT NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
      quantity REAL NOT NULL DEFAULT 0 CHECK(quantity >= 0),
      reserved_quantity REAL NOT NULL DEFAULT 0 CHECK(reserved_quantity >= 0),
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY(product_id,warehouse_id)
    );
    CREATE TABLE IF NOT EXISTS inventory_movements(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      movement_type TEXT NOT NULL CHECK(movement_type IN ('receipt','issue','transfer_in','transfer_out','adjustment')),
      quantity REAL NOT NULL CHECK(quantity > 0), unit_cost INTEGER NOT NULL DEFAULT 0,
      reference_type TEXT, reference_id TEXT, note TEXT, created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_inventory_movements_product_date ON inventory_movements(product_id,created_at);
    CREATE INDEX IF NOT EXISTS idx_inventory_balances_warehouse ON inventory_balances(warehouse_id);
    CREATE TABLE IF NOT EXISTS inventory_reservations(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      quantity REAL NOT NULL CHECK(quantity > 0), status TEXT NOT NULL CHECK(status IN ('reserved','released','consumed')),
      reference_type TEXT, reference_id TEXT, created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      released_at TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_inventory_reservations_stock ON inventory_reservations(product_id,warehouse_id,status);
    CREATE TABLE IF NOT EXISTS inventory_lots(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      lot_number TEXT NOT NULL, lot_type TEXT NOT NULL CHECK(lot_type IN ('batch','serial')),
      serial_number TEXT, manufacture_date TEXT, expiry_date TEXT, quantity REAL NOT NULL CHECK(quantity >= 0),
      unit_cost INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','depleted','blocked')),
      created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,product_id,warehouse_id,lot_number,serial_number)
    );
    CREATE INDEX IF NOT EXISTS idx_inventory_lots_expiry ON inventory_lots(company_id,expiry_date,status);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_inventory_serial_unique ON inventory_lots(company_id,product_id,serial_number) WHERE lot_type='serial' AND serial_number IS NOT NULL;
    CREATE TABLE IF NOT EXISTS inventory_count_sessions(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      warehouse_id TEXT NOT NULL REFERENCES warehouses(id), title TEXT NOT NULL, count_date TEXT NOT NULL,
      status TEXT NOT NULL CHECK(status IN ('draft','counting','review','posted','cancelled')),
      created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, posted_at TEXT
    );
    CREATE TABLE IF NOT EXISTS inventory_count_lines(
      id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES inventory_count_sessions(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), system_quantity REAL NOT NULL, counted_quantity REAL,
      variance REAL, recount_quantity REAL, note TEXT, status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','counted','recounted','approved')),
      UNIQUE(session_id,product_id)
    );
    CREATE INDEX IF NOT EXISTS idx_inventory_count_session ON inventory_count_lines(session_id,status);
    CREATE TABLE IF NOT EXISTS inventory_transfer_orders(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), from_warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      to_warehouse_id TEXT NOT NULL REFERENCES warehouses(id), quantity REAL NOT NULL CHECK(quantity > 0), unit_cost INTEGER NOT NULL DEFAULT 0,
      status TEXT NOT NULL CHECK(status IN ('in_transit','received','cancelled')), note TEXT, created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      received_at TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_inventory_transfer_orders_status ON inventory_transfer_orders(company_id,status);
    INSERT OR IGNORE INTO app_settings(key,value) VALUES('inventory_valuation_method','weighted_average');
    CREATE TABLE IF NOT EXISTS sales_invoices(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id), number INTEGER NOT NULL,
      invoice_date TEXT NOT NULL, contact_id TEXT REFERENCES contacts(id), warehouse_id TEXT REFERENCES warehouses(id),
      status TEXT NOT NULL CHECK(status IN ('draft','posted','cancelled','reversed')),
      payment_status TEXT NOT NULL DEFAULT 'unpaid' CHECK(payment_status IN ('unpaid','partial','paid')),
      subtotal INTEGER NOT NULL DEFAULT 0, discount INTEGER NOT NULL DEFAULT 0, tax INTEGER NOT NULL DEFAULT 0, total INTEGER NOT NULL DEFAULT 0,
      journal_id TEXT REFERENCES journal_entries(id), created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,fiscal_year_id,number)
    );
    CREATE TABLE IF NOT EXISTS sales_invoice_lines(
      id TEXT PRIMARY KEY, invoice_id TEXT NOT NULL REFERENCES sales_invoices(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), quantity REAL NOT NULL CHECK(quantity > 0),
      unit_price INTEGER NOT NULL CHECK(unit_price >= 0), discount INTEGER NOT NULL DEFAULT 0, tax INTEGER NOT NULL DEFAULT 0,
      line_total INTEGER NOT NULL CHECK(line_total >= 0)
    );
    CREATE TABLE IF NOT EXISTS purchase_invoices(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id), number INTEGER NOT NULL,
      invoice_date TEXT NOT NULL, contact_id TEXT REFERENCES contacts(id), warehouse_id TEXT REFERENCES warehouses(id),
      status TEXT NOT NULL CHECK(status IN ('draft','posted','cancelled','reversed')),
      payment_status TEXT NOT NULL DEFAULT 'unpaid' CHECK(payment_status IN ('unpaid','partial','paid')),
      subtotal INTEGER NOT NULL DEFAULT 0, discount INTEGER NOT NULL DEFAULT 0, tax INTEGER NOT NULL DEFAULT 0, total INTEGER NOT NULL DEFAULT 0,
      journal_id TEXT REFERENCES journal_entries(id), created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,fiscal_year_id,number)
    );
    CREATE TABLE IF NOT EXISTS purchase_invoice_lines(
      id TEXT PRIMARY KEY, invoice_id TEXT NOT NULL REFERENCES purchase_invoices(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id), quantity REAL NOT NULL CHECK(quantity > 0),
      unit_price INTEGER NOT NULL CHECK(unit_price >= 0), discount INTEGER NOT NULL DEFAULT 0, tax INTEGER NOT NULL DEFAULT 0,
      line_total INTEGER NOT NULL CHECK(line_total >= 0)
    );
    CREATE INDEX IF NOT EXISTS idx_sales_invoices_company_date ON sales_invoices(company_id,invoice_date);
    CREATE INDEX IF NOT EXISTS idx_purchase_invoices_company_date ON purchase_invoices(company_id,invoice_date);
    CREATE INDEX IF NOT EXISTS idx_sales_lines_invoice ON sales_invoice_lines(invoice_id);
    CREATE INDEX IF NOT EXISTS idx_purchase_lines_invoice ON purchase_invoice_lines(invoice_id);
    CREATE TABLE IF NOT EXISTS invoice_settlements(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id), invoice_id TEXT NOT NULL,
      invoice_type TEXT NOT NULL CHECK(invoice_type IN ('sales','purchase')),
      amount INTEGER NOT NULL CHECK(amount > 0), settlement_date TEXT NOT NULL,
      journal_id TEXT NOT NULL REFERENCES journal_entries(id), created_by TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_invoice_settlements_invoice ON invoice_settlements(invoice_id,invoice_type);
    CREATE INDEX IF NOT EXISTS idx_invoice_settlements_company_type_date ON invoice_settlements(company_id,invoice_type,settlement_date);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_products_company_barcode_unique ON products(company_id,barcode) WHERE barcode IS NOT NULL AND barcode <> '';
    CREATE TABLE IF NOT EXISTS treasury_accounts(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      name TEXT NOT NULL, account_type TEXT NOT NULL CHECK(account_type IN ('cash','bank','petty_cash')),
      account_number TEXT, iban TEXT, linked_account_id TEXT REFERENCES accounts(id), is_active INTEGER NOT NULL DEFAULT 1,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(company_id,name)
    );
    CREATE INDEX IF NOT EXISTS idx_treasury_accounts_company ON treasury_accounts(company_id,is_active);
    CREATE TABLE IF NOT EXISTS treasury_transactions(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id), treasury_account_id TEXT NOT NULL REFERENCES treasury_accounts(id),
      transaction_type TEXT NOT NULL CHECK(transaction_type IN ('receipt','payment','transfer_in','transfer_out')),
      amount INTEGER NOT NULL CHECK(amount > 0), transaction_date TEXT NOT NULL, description TEXT NOT NULL,
      reference_type TEXT, reference_id TEXT, journal_id TEXT REFERENCES journal_entries(id), created_by TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_treasury_tx_company_date ON treasury_transactions(company_id,transaction_date);
    CREATE INDEX IF NOT EXISTS idx_treasury_tx_account_date ON treasury_transactions(treasury_account_id,transaction_date);
    CREATE TABLE IF NOT EXISTS treasury_account_closures(
      id TEXT PRIMARY KEY, treasury_account_id TEXT NOT NULL REFERENCES treasury_accounts(id) ON DELETE CASCADE,
      closed_at TEXT NOT NULL, reason TEXT, created_by TEXT
    );
    CREATE TABLE IF NOT EXISTS checks(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE, fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      check_type TEXT NOT NULL CHECK(check_type IN ('received','issued')), check_number TEXT NOT NULL,
      party_id TEXT REFERENCES contacts(id), treasury_account_id TEXT REFERENCES treasury_accounts(id), amount INTEGER NOT NULL CHECK(amount > 0),
      issue_date TEXT NOT NULL, due_date TEXT NOT NULL, status TEXT NOT NULL CHECK(status IN ('registered','deposited','transferred','cleared','bounced','cancelled')),
      bank_name TEXT, description TEXT, created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_checks_company_due ON checks(company_id,due_date,status);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_checks_company_number_unique ON checks(company_id,check_type,check_number) WHERE status <> 'void';
    CREATE TABLE IF NOT EXISTS plugins(
      id TEXT PRIMARY KEY, company_id TEXT, name TEXT NOT NULL, version TEXT NOT NULL,
      description TEXT, entrypoint TEXT NOT NULL, manifest_json TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 0, installed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS plugin_permissions(
      plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE, permission TEXT NOT NULL,
      PRIMARY KEY(plugin_id,permission)
    );
    CREATE TABLE IF NOT EXISTS api_profiles(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      name TEXT NOT NULL, base_url TEXT NOT NULL, auth_type TEXT NOT NULL CHECK(auth_type IN ('none','api_key','bearer','basic')),
      auth_header TEXT, timeout_ms INTEGER NOT NULL DEFAULT 10000, enabled INTEGER NOT NULL DEFAULT 1,
      allowed_domains TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(company_id,name)
    );
    CREATE INDEX IF NOT EXISTS idx_plugins_company ON plugins(company_id,enabled);
    CREATE INDEX IF NOT EXISTS idx_api_profiles_company ON api_profiles(company_id,enabled);
    INSERT OR IGNORE INTO permissions(id,name) VALUES
      ('plugins.view','plugins.view'),('plugins.manage','plugins.manage'),('plugins.execute','plugins.execute'),
      ('native.execute','native.execute'),('integrations.view','integrations.view'),('integrations.manage','integrations.manage'),('integrations.execute','integrations.execute');
    INSERT OR IGNORE INTO permissions(id,name) VALUES ('reporting.view','reporting.view'),('accounting.period.close','accounting.period.close'),('accounting.settings.edit','accounting.settings.edit');
    CREATE TABLE IF NOT EXISTS sales_returns(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE, fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      number INTEGER NOT NULL, return_date TEXT NOT NULL, original_invoice_id TEXT NOT NULL REFERENCES sales_invoices(id), contact_id TEXT REFERENCES contacts(id), warehouse_id TEXT REFERENCES warehouses(id),
      status TEXT NOT NULL CHECK(status IN ('draft','posted','cancelled')), total INTEGER NOT NULL DEFAULT 0, journal_id TEXT REFERENCES journal_entries(id), created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,fiscal_year_id,number)
    );
    CREATE TABLE IF NOT EXISTS sales_return_lines(
      id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES sales_returns(id) ON DELETE CASCADE, product_id TEXT NOT NULL REFERENCES products(id), quantity REAL NOT NULL CHECK(quantity > 0), unit_price INTEGER NOT NULL CHECK(unit_price >= 0), line_total INTEGER NOT NULL CHECK(line_total >= 0)
    );
    CREATE TABLE IF NOT EXISTS purchase_returns(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE, fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      number INTEGER NOT NULL, return_date TEXT NOT NULL, original_invoice_id TEXT NOT NULL REFERENCES purchase_invoices(id), contact_id TEXT REFERENCES contacts(id), warehouse_id TEXT REFERENCES warehouses(id),
      status TEXT NOT NULL CHECK(status IN ('draft','posted','cancelled')), total INTEGER NOT NULL DEFAULT 0, journal_id TEXT REFERENCES journal_entries(id), created_by TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id,fiscal_year_id,number)
    );
    /* پیش‌فاکتور و سفارش خرید.
     *
     * هر دو «تعهد» هستند، نه «رویداد مالی»: هیچ سند حسابداری نمی‌سازند و
     * موجودی انبار را تغییر نمی‌دهند. فقط وقتی به فاکتور تبدیل شوند، اثر
     * مالی و انبارگردانی پیدا می‌کنند. به همین دلیل جدول جدا دارند و در
     * دفتر کل دیده نمی‌شوند.
     *
     * `valid_until` برای پیش‌فاکتور معنا دارد: قیمت پیشنهادی تاریخ انقضا دارد.
     */
    CREATE TABLE IF NOT EXISTS quotes(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      kind TEXT NOT NULL CHECK(kind IN ('sales_quote','purchase_order')),
      number INTEGER NOT NULL,
      issue_date TEXT NOT NULL,
      valid_until TEXT,
      contact_id TEXT REFERENCES contacts(id),
      warehouse_id TEXT REFERENCES warehouses(id),
      description TEXT,
      subtotal INTEGER NOT NULL DEFAULT 0 CHECK(subtotal >= 0),
      discount INTEGER NOT NULL DEFAULT 0 CHECK(discount >= 0),
      tax INTEGER NOT NULL DEFAULT 0 CHECK(tax >= 0),
      total INTEGER NOT NULL DEFAULT 0 CHECK(total >= 0),
      status TEXT NOT NULL DEFAULT 'draft'
        CHECK(status IN ('draft','sent','accepted','rejected','expired','converted','cancelled')),
      /* فاکتوری که این پیش‌فاکتور به آن تبدیل شده — تبدیل دوباره ممنوع است. */
      converted_invoice_id TEXT,
      created_by TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id, fiscal_year_id, kind, number)
    );
    CREATE TABLE IF NOT EXISTS quote_lines(
      id TEXT PRIMARY KEY,
      quote_id TEXT NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      quantity REAL NOT NULL CHECK(quantity > 0),
      unit_price INTEGER NOT NULL CHECK(unit_price >= 0),
      discount INTEGER NOT NULL DEFAULT 0 CHECK(discount >= 0),
      tax INTEGER NOT NULL DEFAULT 0 CHECK(tax >= 0),
      line_total INTEGER NOT NULL CHECK(line_total >= 0),
      description TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_quotes_company ON quotes(company_id,kind,status);
    CREATE INDEX IF NOT EXISTS idx_quote_lines_quote ON quote_lines(quote_id);
    /* تولید و فرمول تولید.
     *
     * معادله‌ی محوری: مواد مصرفی + هزینه‌ها = بهای تمام‌شده‌ی محصولات.
     * تولید سود نمی‌سازد؛ فقط شکل دارایی از «مواد اولیه» به «کالای ساخته‌شده»
     * تغییر می‌کند. سود در لحظه‌ی فروش محقق می‌شود.
     */
    CREATE TABLE IF NOT EXISTS production_formulas(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      title TEXT NOT NULL,
      output_quantity REAL NOT NULL CHECK(output_quantity > 0),
      is_active INTEGER NOT NULL DEFAULT 1,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id, product_id, title)
    );
    CREATE TABLE IF NOT EXISTS production_formula_components(
      id TEXT PRIMARY KEY,
      formula_id TEXT NOT NULL REFERENCES production_formulas(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      quantity_per_unit REAL NOT NULL CHECK(quantity_per_unit > 0),
      waste_percent REAL NOT NULL DEFAULT 0 CHECK(waste_percent >= 0 AND waste_percent < 100),
      UNIQUE(formula_id, product_id)
    );
    CREATE TABLE IF NOT EXISTS production_orders(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      number INTEGER NOT NULL,
      production_date TEXT NOT NULL,
      warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      formula_id TEXT REFERENCES production_formulas(id),
      cost_allocation TEXT NOT NULL DEFAULT 'by_quantity'
        CHECK(cost_allocation IN ('by_quantity','by_market_value')),
      materials_total INTEGER NOT NULL DEFAULT 0 CHECK(materials_total >= 0),
      expenses_total INTEGER NOT NULL DEFAULT 0 CHECK(expenses_total >= 0),
      total_cost INTEGER NOT NULL DEFAULT 0 CHECK(total_cost >= 0),
      status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted','cancelled')),
      description TEXT,
      journal_id TEXT REFERENCES journal_entries(id),
      created_by TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id, fiscal_year_id, number)
    );
    CREATE TABLE IF NOT EXISTS production_inputs(
      id TEXT PRIMARY KEY,
      order_id TEXT NOT NULL REFERENCES production_orders(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      quantity REAL NOT NULL CHECK(quantity > 0),
      unit_cost INTEGER NOT NULL CHECK(unit_cost >= 0),
      line_total INTEGER NOT NULL CHECK(line_total >= 0)
    );
    CREATE TABLE IF NOT EXISTS production_outputs(
      id TEXT PRIMARY KEY,
      order_id TEXT NOT NULL REFERENCES production_orders(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      quantity REAL NOT NULL CHECK(quantity > 0),
      market_unit_price INTEGER,
      allocated_cost INTEGER NOT NULL DEFAULT 0 CHECK(allocated_cost >= 0),
      unit_cost INTEGER NOT NULL DEFAULT 0 CHECK(unit_cost >= 0)
    );
    CREATE TABLE IF NOT EXISTS production_expenses(
      id TEXT PRIMARY KEY,
      order_id TEXT NOT NULL REFERENCES production_orders(id) ON DELETE CASCADE,
      account_id TEXT NOT NULL REFERENCES accounts(id),
      title TEXT NOT NULL,
      amount INTEGER NOT NULL CHECK(amount >= 0)
    );
    CREATE INDEX IF NOT EXISTS idx_production_orders ON production_orders(company_id,status);
    CREATE INDEX IF NOT EXISTS idx_production_inputs ON production_inputs(order_id);
    CREATE INDEX IF NOT EXISTS idx_production_outputs ON production_outputs(order_id);
    CREATE TABLE IF NOT EXISTS custom_reports(
      id TEXT PRIMARY KEY, company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      name TEXT NOT NULL, source TEXT NOT NULL, config_json TEXT NOT NULL,
      created_by TEXT NOT NULL REFERENCES users(id), created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(company_id,name)
    );
    CREATE INDEX IF NOT EXISTS idx_custom_reports_company ON custom_reports(company_id,name);
    CREATE TABLE IF NOT EXISTS purchase_return_lines(
      id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES purchase_returns(id) ON DELETE CASCADE, product_id TEXT NOT NULL REFERENCES products(id), quantity REAL NOT NULL CHECK(quantity > 0), unit_price INTEGER NOT NULL CHECK(unit_price >= 0), line_total INTEGER NOT NULL CHECK(line_total >= 0)
    );
    CREATE INDEX IF NOT EXISTS idx_sales_returns_invoice ON sales_returns(original_invoice_id);
    CREATE INDEX IF NOT EXISTS idx_purchase_returns_invoice ON purchase_returns(original_invoice_id);
    INSERT OR IGNORE INTO permissions(id,name) VALUES
      ('accounting.journal.create','accounting.journal.create'),
      ('accounting.journal.post','accounting.journal.post'),
      ('accounting.journal.reverse','accounting.journal.reverse'),
      ('security.audit.view','security.audit.view'),
      ('security.role.manage','security.role.manage'),
      ('backup.create','backup.create'),
      ('backup.restore','backup.restore'),
      ('contacts.create','contacts.create'),('contacts.edit','contacts.edit'),('contacts.delete','contacts.delete'),
      ('products.create','products.create'),('products.edit','products.edit'),('products.delete','products.delete'),
      ('inventory.receive','inventory.receive'),('inventory.issue','inventory.issue'),('inventory.transfer','inventory.transfer'),('inventory.adjust','inventory.adjust'),
      ('inventory.reserve','inventory.reserve'),('inventory.count.create','inventory.count.create'),('inventory.count.post','inventory.count.post'),('inventory.lot.manage','inventory.lot.manage'),('inventory.transfer.receive','inventory.transfer.receive'),('inventory.valuation.manage','inventory.valuation.manage'),
      ('sales.invoice.create','sales.invoice.create'),('sales.invoice.post','sales.invoice.post'),('sales.invoice.cancel','sales.invoice.cancel'),
      ('purchase.invoice.create','purchase.invoice.create'),('purchase.invoice.post','purchase.invoice.post'),('purchase.invoice.cancel','purchase.invoice.cancel'),
      ('reports.view','reports.view'),('reports.builder.manage','reports.builder.manage'),('printing.template.view','printing.template.view'),('printing.template.manage','printing.template.manage'),('treasury.receipt.create','treasury.receipt.create'),('treasury.payment.create','treasury.payment.create'),('treasury.account.create','treasury.account.create'),('treasury.account.view','treasury.account.view'),('treasury.account.edit','treasury.account.edit'),('treasury.check.view','treasury.check.view'),('treasury.check.create','treasury.check.create'),('treasury.check.update','treasury.check.update'),('sales.return.create','sales.return.create'),('sales.return.post','sales.return.post'),('purchase.return.create','purchase.return.create'),('purchase.return.post','purchase.return.post');
    "#)?;
    // --- مهاجرت نسخه‌ی ۲: کدینگ چندسطحی، تفصیلی شناور و ابعاد مالی ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS coding_schemes(
      company_id TEXT PRIMARY KEY REFERENCES companies(id) ON DELETE CASCADE,
      level_widths TEXT NOT NULL DEFAULT '1,2,2,2',
      level_titles TEXT NOT NULL DEFAULT 'گروه,کل,معین,تفصیلی'
    );
    CREATE TABLE IF NOT EXISTS subsidiary_groups(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      is_system INTEGER NOT NULL DEFAULT 0,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS subsidiaries(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      group_id TEXT NOT NULL REFERENCES subsidiary_groups(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS cost_centers(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      parent_id TEXT REFERENCES cost_centers(id),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS projects(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      start_date TEXT,
      end_date TEXT,
      status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','closed')),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE INDEX IF NOT EXISTS idx_subsidiaries_group ON subsidiaries(group_id);
    CREATE INDEX IF NOT EXISTS idx_cost_centers_company ON cost_centers(company_id);
    CREATE INDEX IF NOT EXISTS idx_projects_company ON projects(company_id);
    "#,
    )?;
    for (table, column, definition) in [
        (
            "accounts",
            "requires_subsidiary",
            "ALTER TABLE accounts ADD COLUMN requires_subsidiary INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "accounts",
            "subsidiary_group_id",
            "ALTER TABLE accounts ADD COLUMN subsidiary_group_id TEXT REFERENCES subsidiary_groups(id)",
        ),
        (
            "accounts",
            "requires_cost_center",
            "ALTER TABLE accounts ADD COLUMN requires_cost_center INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "accounts",
            "requires_project",
            "ALTER TABLE accounts ADD COLUMN requires_project INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "journal_lines",
            "subsidiary_id",
            "ALTER TABLE journal_lines ADD COLUMN subsidiary_id TEXT REFERENCES subsidiaries(id)",
        ),
        (
            "journal_lines",
            "cost_center_id",
            "ALTER TABLE journal_lines ADD COLUMN cost_center_id TEXT REFERENCES cost_centers(id)",
        ),
        (
            "journal_lines",
            "project_id",
            "ALTER TABLE journal_lines ADD COLUMN project_id TEXT REFERENCES projects(id)",
        ),
    ] {
        if !column_exists(conn, table, column)? {
            conn.execute(definition, [])?;
        }
    }

    // --- مهاجرت نسخه‌ی ۳: کاتالوگ کالا (گروه درختی، سطوح قیمت، چند واحدی، مالیات) ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS product_groups(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      parent_id TEXT REFERENCES product_groups(id),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS product_prices(
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      level TEXT NOT NULL CHECK(level IN
        ('retail','wholesale','partner','partner_tier2','partner_tier3','seasonal','exhibition')),
      price INTEGER NOT NULL CHECK(price >= 0),
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY(product_id, level)
    );
    CREATE TABLE IF NOT EXISTS product_units(
      id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      unit_name TEXT NOT NULL,
      factor REAL NOT NULL CHECK(factor > 0),
      is_default_sale INTEGER NOT NULL DEFAULT 0,
      UNIQUE(product_id, unit_name)
    );
    CREATE TABLE IF NOT EXISTS product_components(
      parent_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      component_id TEXT NOT NULL REFERENCES products(id),
      quantity REAL NOT NULL CHECK(quantity > 0),
      PRIMARY KEY(parent_id, component_id),
      CHECK(parent_id <> component_id)
    );
    CREATE TABLE IF NOT EXISTS product_attributes(
      id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      name TEXT NOT NULL,
      values_csv TEXT NOT NULL,
      UNIQUE(product_id, name)
    );
    CREATE TABLE IF NOT EXISTS product_variants(
      id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      sku TEXT NOT NULL,
      attribute_values TEXT NOT NULL,
      barcode TEXT,
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(product_id, sku)
    );
    CREATE TABLE IF NOT EXISTS product_gold_specs(
      product_id TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
      weight_grams REAL NOT NULL CHECK(weight_grams > 0),
      carat INTEGER NOT NULL DEFAULT 18,
      making_charge_bp INTEGER NOT NULL DEFAULT 0,
      profit_bp INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_product_prices_level ON product_prices(level);
    CREATE INDEX IF NOT EXISTS idx_product_groups_parent ON product_groups(parent_id);
    CREATE INDEX IF NOT EXISTS idx_product_variants_product ON product_variants(product_id);
    "#,
    )?;
    for (table, column, definition) in [
        (
            "products",
            "kind",
            "ALTER TABLE products ADD COLUMN kind TEXT NOT NULL DEFAULT 'simple'",
        ),
        (
            "products",
            "group_id",
            "ALTER TABLE products ADD COLUMN group_id TEXT REFERENCES product_groups(id)",
        ),
        (
            "products",
            "vat_basis_points",
            "ALTER TABLE products ADD COLUMN vat_basis_points INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "products",
            "duty_basis_points",
            "ALTER TABLE products ADD COLUMN duty_basis_points INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "products",
            "tax_code",
            "ALTER TABLE products ADD COLUMN tax_code TEXT",
        ),
        (
            "products",
            "tax_exempt",
            "ALTER TABLE products ADD COLUMN tax_exempt INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "products",
            "display_name",
            "ALTER TABLE products ADD COLUMN display_name TEXT",
        ),
        (
            "products",
            "brand",
            "ALTER TABLE products ADD COLUMN brand TEXT",
        ),
        (
            "products",
            "max_stock",
            "ALTER TABLE products ADD COLUMN max_stock REAL NOT NULL DEFAULT 0",
        ),
        (
            "products",
            "reorder_point",
            "ALTER TABLE products ADD COLUMN reorder_point REAL NOT NULL DEFAULT 0",
        ),
    ] {
        if !column_exists(conn, table, column)? {
            conn.execute(definition, [])?;
        }
    }

    // --- مهاجرت نسخه‌ی ۴: اشخاص (نقش‌ها، مسیر، حساب بانکی، مناسبت) ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS party_routes(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      supervisor_id TEXT REFERENCES contacts(id),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS party_bank_accounts(
      id TEXT PRIMARY KEY,
      contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
      bank_name TEXT NOT NULL,
      branch_name TEXT,
      account_number TEXT,
      iban TEXT,
      card_number TEXT,
      holder_name TEXT,
      is_default INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS party_phones(
      id TEXT PRIMARY KEY,
      contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
      title TEXT,
      number TEXT NOT NULL,
      is_primary INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS party_occasions(
      id TEXT PRIMARY KEY,
      contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
      title TEXT NOT NULL,
      jalali_month INTEGER NOT NULL CHECK(jalali_month BETWEEN 1 AND 12),
      jalali_day INTEGER NOT NULL CHECK(jalali_day BETWEEN 1 AND 31),
      remind_days_before INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS party_groups(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      title TEXT NOT NULL,
      parent_id TEXT REFERENCES party_groups(id),
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS party_images(
      id TEXT PRIMARY KEY,
      contact_id TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
      title TEXT,
      /* مسیر فایل روی دیسک کاربر؛ تصویر داخل پایگاه داده ذخیره نمی‌شود تا
         حجم فایل و زمان پشتیبان‌گیری کنترل‌شده بماند. */
      file_path TEXT NOT NULL,
      is_primary INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_party_images_contact ON party_images(contact_id);
    CREATE INDEX IF NOT EXISTS idx_party_bank_contact ON party_bank_accounts(contact_id);
    CREATE INDEX IF NOT EXISTS idx_party_phones_contact ON party_phones(contact_id);
    CREATE INDEX IF NOT EXISTS idx_party_occasions_date ON party_occasions(jalali_month, jalali_day);
    "#,
    )?;
    for (table, column, definition) in [
        (
            "contacts",
            "party_type",
            "ALTER TABLE contacts ADD COLUMN party_type TEXT NOT NULL DEFAULT 'natural'",
        ),
        (
            "contacts",
            "party_function",
            "ALTER TABLE contacts ADD COLUMN party_function TEXT NOT NULL DEFAULT 'person'",
        ),
        (
            "contacts",
            "first_name",
            "ALTER TABLE contacts ADD COLUMN first_name TEXT",
        ),
        (
            "contacts",
            "last_name",
            "ALTER TABLE contacts ADD COLUMN last_name TEXT",
        ),
        (
            "contacts",
            "company_name",
            "ALTER TABLE contacts ADD COLUMN company_name TEXT",
        ),
        (
            "contacts",
            "economic_code",
            "ALTER TABLE contacts ADD COLUMN economic_code TEXT",
        ),
        (
            "contacts",
            "postal_code",
            "ALTER TABLE contacts ADD COLUMN postal_code TEXT",
        ),
        (
            "contacts",
            "route_id",
            "ALTER TABLE contacts ADD COLUMN route_id TEXT REFERENCES party_routes(id)",
        ),
        (
            "contacts",
            "marketer_id",
            "ALTER TABLE contacts ADD COLUMN marketer_id TEXT REFERENCES contacts(id)",
        ),
        (
            "contacts",
            "opening_date",
            "ALTER TABLE contacts ADD COLUMN opening_date TEXT",
        ),
        (
            "contacts",
            "job_title",
            "ALTER TABLE contacts ADD COLUMN job_title TEXT",
        ),
        (
            "contacts",
            "introduction",
            "ALTER TABLE contacts ADD COLUMN introduction TEXT",
        ),
    ] {
        if !column_exists(conn, table, column)? {
            conn.execute(definition, [])?;
        }
    }

    // --- مهاجرت نسخه‌ی ۵: تخفیف پلکانی، کوپن و اقساط ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS product_discount_tiers(
      id TEXT PRIMARY KEY,
      product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      min_quantity REAL NOT NULL CHECK(min_quantity > 0),
      discount_bp INTEGER NOT NULL CHECK(discount_bp BETWEEN 0 AND 10000),
      UNIQUE(product_id, min_quantity)
    );
    CREATE TABLE IF NOT EXISTS discount_coupons(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      code TEXT NOT NULL,
      kind TEXT NOT NULL CHECK(kind IN ('percent','amount')),
      value INTEGER NOT NULL CHECK(value >= 0),
      minimum_invoice INTEGER NOT NULL DEFAULT 0,
      maximum_discount INTEGER NOT NULL DEFAULT 0,
      valid_from TEXT,
      valid_to TEXT,
      usage_limit INTEGER NOT NULL DEFAULT 0,
      used_count INTEGER NOT NULL DEFAULT 0,
      is_active INTEGER NOT NULL DEFAULT 1,
      UNIQUE(company_id, code)
    );
    CREATE TABLE IF NOT EXISTS invoice_installments(
      id TEXT PRIMARY KEY,
      invoice_id TEXT NOT NULL,
      invoice_kind TEXT NOT NULL CHECK(invoice_kind IN ('sales','purchase')),
      number INTEGER NOT NULL CHECK(number > 0),
      due_date TEXT NOT NULL,
      amount INTEGER NOT NULL CHECK(amount > 0),
      paid_amount INTEGER NOT NULL DEFAULT 0,
      status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','partial','paid','overdue')),
      UNIQUE(invoice_id, number)
    );
    CREATE TABLE IF NOT EXISTS invoice_line_serials(
      id TEXT PRIMARY KEY,
      invoice_id TEXT NOT NULL,
      product_id TEXT NOT NULL REFERENCES products(id),
      serial TEXT NOT NULL,
      UNIQUE(invoice_id, serial)
    );
    CREATE INDEX IF NOT EXISTS idx_installments_due ON invoice_installments(due_date, status);
    "#,
    )?;
    for (table, column, definition) in [
        (
            "sales_invoice_lines",
            "commission_bp",
            "ALTER TABLE sales_invoice_lines ADD COLUMN commission_bp INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "sales_invoice_lines",
            "duty",
            "ALTER TABLE sales_invoice_lines ADD COLUMN duty INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "sales_invoices",
            "freight",
            "ALTER TABLE sales_invoices ADD COLUMN freight INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "sales_invoices",
            "coupon_code",
            "ALTER TABLE sales_invoices ADD COLUMN coupon_code TEXT",
        ),
        (
            "purchase_invoices",
            "freight",
            "ALTER TABLE purchase_invoices ADD COLUMN freight INTEGER NOT NULL DEFAULT 0",
        ),
    ] {
        if !column_exists(conn, table, column)? {
            conn.execute(definition, [])?;
        }
    }

    // --- مهاجرت نسخه‌ی ۶: انبارگردانی اصولی و عملیات جمعی ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS stocktake_sessions(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      warehouse_id TEXT NOT NULL REFERENCES warehouses(id),
      title TEXT NOT NULL,
      count_date TEXT NOT NULL,
      status TEXT NOT NULL DEFAULT 'draft'
        CHECK(status IN ('draft','counting','review','posted','cancelled')),
      frozen_at TEXT,
      posted_at TEXT,
      journal_id TEXT REFERENCES journal_entries(id),
      recount_threshold_percent REAL NOT NULL DEFAULT 5,
      created_by TEXT NOT NULL REFERENCES users(id),
      approved_by TEXT REFERENCES users(id),
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS stocktake_lines(
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL REFERENCES stocktake_sessions(id) ON DELETE CASCADE,
      product_id TEXT NOT NULL REFERENCES products(id),
      frozen_quantity REAL NOT NULL DEFAULT 0,
      counted_quantity REAL CHECK(counted_quantity IS NULL OR counted_quantity >= 0),
      recount_quantity REAL CHECK(recount_quantity IS NULL OR recount_quantity >= 0),
      unit_cost INTEGER NOT NULL DEFAULT 0,
      variance_approved INTEGER NOT NULL DEFAULT 0,
      approved_by TEXT REFERENCES users(id),
      note TEXT,
      UNIQUE(session_id, product_id)
    );
    CREATE TABLE IF NOT EXISTS bulk_operations(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      operation TEXT NOT NULL
        CHECK(operation IN ('price_change','group_change','zero_stock','activate','deactivate','tax_change')),
      payload TEXT NOT NULL,
      affected_count INTEGER NOT NULL DEFAULT 0,
      performed_by TEXT NOT NULL REFERENCES users(id),
      performed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_stocktake_lines_session ON stocktake_lines(session_id);
    CREATE INDEX IF NOT EXISTS idx_stocktake_sessions_status ON stocktake_sessions(status);
    "#,
    )?;

    // --- مهاجرت نسخه‌ی ۷: خزانه (سند چندروشی، بانک، صندوق، دسته‌چک) ---
    conn.execute_batch(
        r#"
    CREATE TABLE IF NOT EXISTS treasury_documents(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
      kind TEXT NOT NULL CHECK(kind IN ('receipt','payment')),
      number INTEGER NOT NULL,
      document_date TEXT NOT NULL,
      party_id TEXT REFERENCES contacts(id),
      description TEXT,
      total INTEGER NOT NULL DEFAULT 0 CHECK(total >= 0),
      status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','posted','cancelled')),
      journal_id TEXT REFERENCES journal_entries(id),
      created_by TEXT NOT NULL REFERENCES users(id),
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(company_id, fiscal_year_id, kind, number)
    );
    CREATE TABLE IF NOT EXISTS treasury_document_lines(
      id TEXT PRIMARY KEY,
      document_id TEXT NOT NULL REFERENCES treasury_documents(id) ON DELETE CASCADE,
      method TEXT NOT NULL CHECK(method IN
        ('cash','check','bank_transfer','card_terminal','discount','offset')),
      amount INTEGER NOT NULL CHECK(amount > 0),
      description TEXT,
      treasury_account_id TEXT REFERENCES treasury_accounts(id),
      terminal_id TEXT,
      check_serial TEXT,
      check_due_date TEXT,
      check_bank_name TEXT,
      sayad_id TEXT,
      check_id TEXT REFERENCES checks(id)
    );
    CREATE TABLE IF NOT EXISTS checkbooks(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      treasury_account_id TEXT NOT NULL REFERENCES treasury_accounts(id),
      title TEXT NOT NULL,
      serial_from INTEGER NOT NULL CHECK(serial_from > 0),
      serial_to INTEGER NOT NULL,
      is_active INTEGER NOT NULL DEFAULT 1,
      CHECK(serial_to >= serial_from)
    );
    CREATE TABLE IF NOT EXISTS checkbook_serials(
      checkbook_id TEXT NOT NULL REFERENCES checkbooks(id) ON DELETE CASCADE,
      serial INTEGER NOT NULL,
      check_id TEXT REFERENCES checks(id),
      used_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY(checkbook_id, serial)
    );
    CREATE TABLE IF NOT EXISTS pos_terminals(
      id TEXT PRIMARY KEY,
      company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
      treasury_account_id TEXT NOT NULL REFERENCES treasury_accounts(id),
      title TEXT NOT NULL,
      terminal_number TEXT,
      merchant_code TEXT,
      is_active INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX IF NOT EXISTS idx_treasury_doc_lines ON treasury_document_lines(document_id);
    CREATE INDEX IF NOT EXISTS idx_treasury_docs_party ON treasury_documents(party_id);
    "#,
    )?;
    for (table, column, definition) in [
        (
            "treasury_accounts",
            "negative_policy",
            "ALTER TABLE treasury_accounts ADD COLUMN negative_policy TEXT NOT NULL DEFAULT 'warn'",
        ),
        (
            "treasury_accounts",
            "card_number",
            "ALTER TABLE treasury_accounts ADD COLUMN card_number TEXT",
        ),
        (
            "treasury_accounts",
            "branch_name",
            "ALTER TABLE treasury_accounts ADD COLUMN branch_name TEXT",
        ),
        (
            "treasury_accounts",
            "branch_code",
            "ALTER TABLE treasury_accounts ADD COLUMN branch_code TEXT",
        ),
        (
            "treasury_accounts",
            "holder_name",
            "ALTER TABLE treasury_accounts ADD COLUMN holder_name TEXT",
        ),
        (
            "treasury_accounts",
            "has_pos_terminal",
            "ALTER TABLE treasury_accounts ADD COLUMN has_pos_terminal INTEGER NOT NULL DEFAULT 0",
        ),
        // --- فرم کامل شخص ---
        (
            "contacts",
            "code",
            "ALTER TABLE contacts ADD COLUMN code TEXT",
        ),
        (
            "contacts",
            "group_id",
            "ALTER TABLE contacts ADD COLUMN group_id TEXT REFERENCES party_groups(id)",
        ),
        (
            "contacts",
            "is_active",
            "ALTER TABLE contacts ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1",
        ),
        (
            "contacts",
            "title_prefix",
            "ALTER TABLE contacts ADD COLUMN title_prefix TEXT",
        ),
        (
            "contacts",
            "email",
            "ALTER TABLE contacts ADD COLUMN email TEXT",
        ),
        (
            "contacts",
            "website",
            "ALTER TABLE contacts ADD COLUMN website TEXT",
        ),
        (
            "contacts",
            "city",
            "ALTER TABLE contacts ADD COLUMN city TEXT",
        ),
        (
            "contacts",
            "province",
            "ALTER TABLE contacts ADD COLUMN province TEXT",
        ),
        // مشخصات کاربری فروشگاه اینترنتی؛ رمز هرگز خام ذخیره نمی‌شود.
        (
            "contacts",
            "portal_username",
            "ALTER TABLE contacts ADD COLUMN portal_username TEXT",
        ),
        (
            "contacts",
            "portal_password_hash",
            "ALTER TABLE contacts ADD COLUMN portal_password_hash TEXT",
        ),
        (
            "contacts",
            "note",
            "ALTER TABLE contacts ADD COLUMN note TEXT",
        ),
    ] {
        if !column_exists(conn, table, column)? {
            conn.execute(definition, [])?;
        }
    }

    // Backward-compatible schema migrations. Check column existence first.
    if !column_exists(conn, "inventory_balances", "in_transit_quantity")? {
        conn.execute(
            "ALTER TABLE inventory_balances ADD COLUMN in_transit_quantity REAL NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !column_exists(conn, "checks", "clearing_journal_id")? {
        conn.execute(
            "ALTER TABLE checks ADD COLUMN clearing_journal_id TEXT REFERENCES journal_entries(id)",
            [],
        )?;
    }
    // گروه‌های دارای حساب کاهنده باید دوطرفه باشند — اصلاح پایگاه داده‌های موجود.
    //
    // «برگشت از فروش» و «تخفیف نقدی» ماهیت بدهکار دارند ولی زیر گروه درآمد
    // می‌نشینند؛ «برگشت از خرید» بستانکار است ولی زیر گروه بهای تمام‌شده.
    // اگر گروه دوطرفه نباشد، قاعده‌ی «ماهیت فرزند با والد بخواند» نقض می‌شود.
    conn.execute(
        "UPDATE accounts SET nature='mixed' WHERE level='group' AND EXISTS \
         (SELECT 1 FROM accounts child WHERE child.parent_id = accounts.id \
          AND child.nature <> accounts.nature)",
        [],
    )?;
    migrate_check_statuses(conn)?;
    seed(conn)?;
    Ok(())
}

/// گسترش قید وضعیت چک به مجموعه‌ی کامل چرخه‌ی عمر چک.
///
/// ## چرا لازم است
///
/// ماشین حالت چک در `novin_core::checks` دوازده وضعیت واقعی دارد
/// (موجود، واگذار شده، وصول شده، نقد شده، خرج شده، برگشتی، عودت شده، باطل،
/// پرداختی، پرداخت شده و دو وضعیت انتظامی). اما قید `CHECK` جدول تنها شش
/// وضعیت قدیمی را می‌پذیرفت؛ در نتیجه هر تغییر وضعیتی که ماشین حالت مجاز
/// می‌دانست، در پایگاه داده رد می‌شد.
///
/// ## نگاشت وضعیت‌های قدیمی
///
/// | قدیمی | چک دریافتی | چک پرداختی |
/// |---|---|---|
/// | `registered` | `in_hand` موجود | `outstanding` پرداختی |
/// | `cleared` | `collected` وصول شده | `paid` پرداخت شده |
/// | `transferred` | `endorsed` خرج شده | `endorsed` |
/// | `cancelled` | `void` باطل | `void` |
///
/// SQLite قید `CHECK` را تغییر نمی‌دهد، پس جدول بازسازی می‌شود.
fn migrate_check_statuses(conn: &Connection) -> Result<()> {
    let definition: String = conn.query_row(
        "SELECT COALESCE(sql,'') FROM sqlite_master WHERE type='table' AND name='checks'",
        [],
        |row| row.get(0),
    )?;
    // اگر قبلاً مهاجرت انجام شده، دوباره انجام نده.
    if definition.contains("'memo_in_hand'") {
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        "CREATE TABLE checks_migrated(
           id TEXT PRIMARY KEY,
           company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
           fiscal_year_id TEXT NOT NULL REFERENCES fiscal_years(id),
           check_type TEXT NOT NULL CHECK(check_type IN ('received','issued')),
           check_number TEXT NOT NULL,
           party_id TEXT REFERENCES contacts(id),
           treasury_account_id TEXT REFERENCES treasury_accounts(id),
           amount INTEGER NOT NULL CHECK(amount > 0),
           issue_date TEXT NOT NULL,
           due_date TEXT NOT NULL,
           status TEXT NOT NULL CHECK(status IN (
             'in_hand','deposited','collected','cashed','endorsed','bounced',
             'returned','void','outstanding','paid','memo_in_hand','memo_returned'
           )),
           bank_name TEXT,
           description TEXT,
           created_by TEXT,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           clearing_journal_id TEXT REFERENCES journal_entries(id)
         );
         INSERT INTO checks_migrated(
           id,company_id,fiscal_year_id,check_type,check_number,party_id,
           treasury_account_id,amount,issue_date,due_date,status,bank_name,
           description,created_by,created_at,clearing_journal_id)
         SELECT id,company_id,fiscal_year_id,check_type,check_number,party_id,
           treasury_account_id,amount,issue_date,due_date,
           CASE status
             WHEN 'registered' THEN CASE check_type WHEN 'issued' THEN 'outstanding' ELSE 'in_hand' END
             WHEN 'cleared' THEN CASE check_type WHEN 'issued' THEN 'paid' ELSE 'collected' END
             WHEN 'transferred' THEN 'endorsed'
             WHEN 'cancelled' THEN 'void'
             ELSE status
           END,
           bank_name,description,created_by,created_at,clearing_journal_id
         FROM checks;
         DROP TABLE checks;
         ALTER TABLE checks_migrated RENAME TO checks;",
    )?;
    tx.commit()?;
    Ok(())
}

/// درج داده‌های پایه و داده‌ی نمونه‌ی آموزشی.
pub fn seed(conn: &Connection) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("INSERT OR IGNORE INTO companies(id,name,national_id,phone) VALUES('company-demo','نوین پرداز','14000000000','021-00000000')",[])?;
    tx.execute("INSERT OR IGNORE INTO fiscal_years(id,company_id,title,start_date,end_date) VALUES('fy-demo','company-demo','1405','1405/01/01','1405/12/29')",[])?;
    tx.execute(
        "INSERT OR IGNORE INTO roles(id,name) VALUES('role-admin','مدیر سیستم')",
        [],
    )?;
    tx.execute("INSERT OR IGNORE INTO role_permissions(role_id,permission_id) SELECT 'role-admin', id FROM permissions",[])?;
    let password_hash = hash_password("demo");
    tx.execute("INSERT OR IGNORE INTO users(id,username,display_name,password_hash) VALUES('user-demo','admin','مدیر سیستم',?1)", [&password_hash])?;
    tx.execute(
        "INSERT OR IGNORE INTO user_roles(user_id,role_id) VALUES('user-demo','role-admin')",
        [],
    )?;
    tx.execute("INSERT OR IGNORE INTO company_users(company_id,user_id) VALUES('company-demo','user-demo')",[])?;
    let accounts = [
        ("1000", "دارایی ها", "group", None::<&str>, "debit"),
        (
            "1100",
            "موجودی نقد و بانک",
            "general",
            Some("1000"),
            "debit",
        ),
        ("1101", "صندوق مرکزی", "detail", Some("1100"), "debit"),
        (
            "1200",
            "حساب های دریافتنی",
            "general",
            Some("1000"),
            "debit",
        ),
        ("1201", "حساب مشتریان", "detail", Some("1200"), "debit"),
        ("2000", "بدهی ها", "group", None, "credit"),
        (
            "2100",
            "حساب های پرداختنی",
            "general",
            Some("2000"),
            "credit",
        ),
        ("2101", "تأمین کنندگان", "detail", Some("2100"), "credit"),
        // مالیات بر ارزش افزوده بدهی به سازمان مالیاتی است، نه درآمد شرکت.
        // در کدینگ پایه می‌آید چون نخستین سند فروش هم به آن نیاز دارد.
        ("2400", "مالیات و عوارض", "general", Some("2000"), "credit"),
        (
            "2401",
            "مالیات بر ارزش افزوده",
            "detail",
            Some("2400"),
            "credit",
        ),
        // گروه دوطرفه: هم فروش (بستانکار) و هم حساب‌های کاهنده‌ی فروش
        // (برگشت از فروش و تخفیف نقدی، بدهکار) زیر آن می‌نشینند.
        ("4000", "درآمد فروش", "group", None, "mixed"),
        ("4100", "فروش کالا", "general", Some("4000"), "credit"),
        // گروه دوطرفه: بهای تمام‌شده (بدهکار) و برگشت از خرید (بستانکار).
        ("5000", "بهای تمام شده", "group", None, "mixed"),
        (
            "5100",
            "بهای تمام شده کالای فروش رفته",
            "general",
            Some("5000"),
            "debit",
        ),
    ];
    for (code, name, level, parent, nature) in accounts {
        tx.execute("INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",rusqlite::params![format!("acc-{code}"),code,name,level,parent.map(|p|format!("acc-{p}")),nature])?;
    }
    tx.execute("INSERT OR IGNORE INTO treasury_accounts(id,company_id,name,account_type,linked_account_id) VALUES('treasury-cash-demo','company-demo','صندوق مرکزی','cash','acc-1101')",[])?;
    tx.execute("INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) VALUES('acc-4200','company-demo','4200','برگشت از فروش','general','acc-4000','debit')",[])?;
    tx.execute("INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) VALUES('acc-5200','company-demo','5200','برگشت از خرید','general','acc-5000','credit')",[])?;
    tx.execute("INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) VALUES('acc-1300','company-demo','1300','موجودی کالا','general','acc-1000','debit')",[])?;
    let contacts = [
        (
            "contact-1",
            "شرکت آریا تجارت",
            "company",
            "09120000001",
            1,
            0,
        ),
        ("contact-2", "فروشگاه پارس", "company", "09120000002", 1, 0),
        ("contact-3", "محمد رضایی", "person", "09120000003", 1, 0),
        (
            "contact-4",
            "تأمین‌کننده آریا",
            "company",
            "09120000004",
            0,
            1,
        ),
    ];
    for (id, name, kind, mobile, cust, supp) in contacts {
        tx.execute("INSERT OR IGNORE INTO contacts(id,company_id,kind,name,mobile,is_customer,is_supplier) VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",rusqlite::params![id,kind,name,mobile,cust,supp])?;
    }
    let products = [
        (
            "prod-1",
            "1001",
            "8901001001001",
            "پرینتر حرارتی X100",
            "دستگاه",
            12500000,
            9500000,
            5.0,
        ),
        (
            "prod-2",
            "1002",
            "8901001001002",
            "بارکدخوان Pro",
            "دستگاه",
            8900000,
            6500000,
            5.0,
        ),
        (
            "prod-3",
            "1003",
            "8901001001003",
            "لیبل حرارتی 50×30",
            "رول",
            185000,
            120000,
            100.0,
        ),
    ];
    for (id, sku, barcode, name, unit, sale, purchase, min) in products {
        tx.execute("INSERT OR IGNORE INTO products(id,company_id,sku,barcode,name,unit,sale_price,purchase_price,min_stock) VALUES(?1,'company-demo',?2,?3,?4,?5,?6,?7,?8)",rusqlite::params![id,sku,barcode,name,unit,sale,purchase,min])?;
    }
    tx.execute("INSERT OR IGNORE INTO warehouses(id,company_id,name,code) VALUES('wh-main','company-demo','انبار مرکزی','01')",[])?;
    tx.execute(
        "INSERT OR IGNORE INTO app_settings(key,value) VALUES('demo_data','true')",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO app_settings(key,value) VALUES('demo_version','1.7.0')",
        [],
    )?;
    tx.execute("INSERT OR IGNORE INTO print_templates(id,company_id,name,template_type,content_html,is_default,created_by) VALUES('tpl-demo-invoice','company-demo','قالب استاندارد فاکتور','invoice','<section dir=\"rtl\"><h1>{{company.name}}</h1><p>فاکتور فروش شماره {{invoice.number}}</p><table>{{#lines}}<tr><td>{{product.name}}</td><td>{{quantity}}</td><td>{{line_total}}</td></tr>{{/lines}}</table><strong>{{invoice.total}}</strong></section>',1,'user-demo')",[])?;
    tx.execute("INSERT OR IGNORE INTO inventory_balances(product_id,warehouse_id,quantity,reserved_quantity) VALUES('prod-1','wh-main',24,3)",[])?;
    tx.execute("INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,note,created_by) VALUES('demo-mov-1','company-demo','prod-1','wh-main','receipt',24,9500000,'demo','موجودی نمونه','user-demo')",[])?;
    tx.execute("INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) VALUES('demo-mov-2','company-demo','prod-1','wh-main','issue',2,9500000,'sales_invoice','demo-sale-1','خروج بابت فروش نمونه','user-demo')",[])?;
    tx.execute("UPDATE inventory_balances SET quantity=22 WHERE product_id='prod-1' AND warehouse_id='wh-main'",[])?;
    tx.execute("INSERT OR IGNORE INTO sales_invoices(id,company_id,fiscal_year_id,number,invoice_date,contact_id,warehouse_id,status,payment_status,subtotal,discount,tax,total,created_by) VALUES('demo-sale-1','company-demo','fy-demo',1001,'1405/05/10','contact-1','wh-main','posted','paid',25000000,1000000,2400000,26400000,'user-demo')",[])?;
    // قاعده‌ی ثابت فاکتور: `line_total` مبلغ ناخالص همان سطر است
    // (مقدار × فی)، نه جمع کل فاکتور. و `subtotal` سربرگ برابر جمع
    // `line_total` سطرهاست. پیش از این، مبلغ سطر اشتباهاً جمع کل فاکتور
    // (۲۶٬۴۰۰٬۰۰۰) گذاشته شده بود و سربرگ با اقلامش نمی‌خواند.
    tx.execute("INSERT OR IGNORE INTO sales_invoice_lines(id,invoice_id,product_id,quantity,unit_price,discount,tax,line_total) VALUES('demo-sale-line-1','demo-sale-1','prod-1',2,12500000,1000000,2400000,25000000)",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES('demo-journal-1','company-demo','fy-demo',1001,'1405/05/10','فروش نمونه','posted','sales','demo-sale-1','user-demo')",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) VALUES('demo-jl-1','demo-journal-1','acc-1201','حساب مشتری',26400000,0)",[])?;
    // درآمد فقط مبلغ خالص فروش است؛ مالیات بر ارزش افزوده بدهی به سازمان
    // مالیاتی است نه درآمد شرکت. پیش از این کل ۲۶٬۴۰۰٬۰۰۰ به‌عنوان درآمد
    // ثبت می‌شد که هم سود را متورم می‌کرد هم مالیات را از دفاتر پنهان.
    tx.execute("INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) VALUES('demo-jl-2','demo-journal-1','acc-4100','فروش کالا',0,24000000)",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) VALUES('demo-jl-2b','demo-journal-1','acc-2401','مالیات بر ارزش افزوده',0,2400000)",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,status,source_type,source_id,created_by) VALUES('demo-journal-2','company-demo','fy-demo',1002,'1405/05/10','بهای تمام شده فروش نمونه','posted','inventory','demo-sale-1','user-demo')",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) VALUES('demo-jl-3','demo-journal-2','acc-5100','بهای تمام شده',19000000,0)",[])?;
    tx.execute("INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) VALUES('demo-jl-4','demo-journal-2','acc-1300','کاهش موجودی',0,19000000)",[])?;
    tx.execute("INSERT OR IGNORE INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,transaction_type,amount,transaction_date,description,reference_type,reference_id,created_by) VALUES('demo-tx-1','company-demo','fy-demo','treasury-cash-demo','receipt',26400000,'1405/05/10','دریافت نمونه فروش','sales','demo-sale-1','user-demo')",[])?;
    tx.execute("INSERT OR IGNORE INTO checks(id,company_id,fiscal_year_id,check_type,check_number,party_id,treasury_account_id,amount,issue_date,due_date,status,bank_name,description,created_by) VALUES('demo-check-1','company-demo','fy-demo','received','CHK-DEMO-001','contact-1','treasury-cash-demo',15000000,'1405/05/01','1405/06/01','in_hand','بانک نمونه','چک نمونه آموزشی','user-demo')",[])?;
    // --- گروه‌های تفصیلی شناور (مطابق نرم‌افزار فعلی) ---
    for (id, code, title) in [
        ("subgroup-persons", "10", "اشخاص"),
        ("subgroup-cashboxes", "2020", "صندوق ها"),
        ("subgroup-banks", "2030", "بانک ها"),
        ("subgroup-cost-centers", "40", "مراکز هزینه"),
        ("subgroup-projects", "50", "پروژه ها"),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO subsidiary_groups(id,company_id,code,title,is_system) VALUES(?1,'company-demo',?2,?3,1)",
            rusqlite::params![id, code, title],
        )?;
    }
    tx.execute(
        "INSERT OR IGNORE INTO coding_schemes(company_id) VALUES('company-demo')",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO cost_centers(id,company_id,code,title) VALUES('cc-sales','company-demo','4001','واحد فروش')",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO cost_centers(id,company_id,code,title) VALUES('cc-admin','company-demo','4002','واحد اداری')",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO projects(id,company_id,code,title,status) VALUES('project-demo','company-demo','5001','پروژه نمونه','open')",
        [],
    )?;
    // حساب‌های دریافتنی/پرداختنی باید تفصیلی شخص داشته باشند.
    tx.execute(
        "UPDATE accounts SET requires_subsidiary=1, subsidiary_group_id='subgroup-persons' \
         WHERE company_id='company-demo' AND code IN ('1201','2101')",
        [],
    )?;
    // --- گروه‌های کالا (درخت نمونه مطابق لیست کالاهای نرم‌افزار فعلی) ---
    for (id, code, title, parent) in [
        ("pgroup-food", "1", "مواد غذایی", None),
        ("pgroup-misc", "2", "کالاهای متفرقه", None),
        ("pgroup-cosmetic", "3", "آرایشی بهداشتی", None),
        ("pgroup-fashion", "4", "مد و پوشاک", None),
        ("pgroup-restaurant", "106", "رستورانی", Some("pgroup-food")),
        ("pgroup-raw", "9", "مواد اولیه", None),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO product_groups(id,company_id,code,title,parent_id) VALUES(?1,'company-demo',?2,?3,?4)",
            rusqlite::params![id, code, title, parent],
        )?;
    }
    // سطوح قیمت کالاهای نمونه بر پایه‌ی قیمت فروش موجود.
    tx.execute(
        "INSERT OR IGNORE INTO product_prices(product_id,level,price) \
         SELECT id,'retail',sale_price FROM products WHERE company_id='company-demo'",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO product_prices(product_id,level,price) \
         SELECT id,'wholesale',CAST(sale_price*0.95 AS INTEGER) FROM products WHERE company_id='company-demo'",
        [],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO product_prices(product_id,level,price) \
         SELECT id,'partner',CAST(sale_price*0.90 AS INTEGER) FROM products WHERE company_id='company-demo'",
        [],
    )?;
    tx.execute(
        "UPDATE products SET group_id='pgroup-misc' WHERE company_id='company-demo' AND group_id IS NULL",
        [],
    )?;
    tx.execute(
        "UPDATE products SET vat_basis_points=900 WHERE company_id='company-demo' AND is_service=0",
        [],
    )?;
    // --- گروه‌بندی اشخاص ---
    //
    // درخت گروه در فرم لیست اشخاص از همین جدول ساخته می‌شود. گروه‌بندی صرفاً
    // برای فیلتر و گزارش است و اثر حسابداری ندارد؛ اثر حسابداری از نقش شخص
    // (مشتری/تأمین‌کننده) می‌آید نه از گروهش.
    for (id, code, title, parent) in [
        ("pgroup-trade-debtor", "G01", "بدهکاران تجاری", None::<&str>),
        ("pgroup-trade-creditor", "G02", "بستانکاران تجاری", None),
        ("pgroup-site", "G03", "سایت", None),
        ("pgroup-colleagues", "G04", "همکاران", None),
        ("pgroup-staff", "G05", "کارکنان", None),
        (
            "pgroup-vip",
            "G06",
            "مشتریان ویژه",
            Some("pgroup-trade-debtor"),
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO party_groups(id,company_id,code,title,parent_id) \
             VALUES(?1,'company-demo',?2,?3,?4)",
            rusqlite::params![id, code, title, parent],
        )?;
    }

    // --- مسیرهای پخش و بازاریاب نمونه ---
    for (id, code, title) in [
        ("route-north", "R01", "مسیر شمال شهر"),
        ("route-center", "R02", "مسیر مرکز"),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO party_routes(id,company_id,code,title) VALUES(?1,'company-demo',?2,?3)",
            rusqlite::params![id, code, title],
        )?;
    }
    tx.execute(
        "INSERT OR IGNORE INTO contacts(id,company_id,kind,name,mobile,is_customer,is_supplier) \
         VALUES('contact-marketer','company-demo','person','سعید بازاریان','09121110022',0,0)",
        [],
    )?;
    tx.execute(
        "UPDATE contacts SET party_function='marketer' WHERE id='contact-marketer'",
        [],
    )?;
    tx.execute(
        "UPDATE contacts SET party_type='private_legal', company_name=name \
         WHERE company_id='company-demo' AND kind='company'",
        [],
    )?;
    tx.execute(
        "UPDATE contacts SET route_id='route-center', marketer_id='contact-marketer' \
         WHERE company_id='company-demo' AND is_customer=1 AND route_id IS NULL",
        [],
    )?;
    tx.execute(
        "UPDATE contacts SET credit_limit=500000000 \
         WHERE company_id='company-demo' AND is_customer=1 AND credit_limit=0",
        [],
    )?;
    // آستانه‌ی نمایش کارت «نزدیک به اتمام موجودی» — قابل تغییر در تنظیمات.
    tx.execute(
        "INSERT OR IGNORE INTO app_settings(key,value) VALUES('inventory.low_stock_threshold','5')",
        [],
    )?;
    // درصد اختلافی که شمارش مجدد را الزامی می‌کند.
    tx.execute(
        "INSERT OR IGNORE INTO app_settings(key,value) VALUES('inventory.recount_threshold_percent','5')",
        [],
    )?;
    // --- نگاشت حساب‌های پیش‌فرض (حذف حساب‌های hardcoded از میزبان) ---
    // دو حساب تخفیف و یک حساب پیگیری چک برگشتی به کدینگ پایه اضافه می‌شود.
    for (id, code, name, level, parent, nature) in [
        (
            "acc-4250",
            "4250",
            "تخفیفات فروش",
            "general",
            Some("acc-4000"),
            "debit",
        ),
        (
            "acc-5250",
            "5250",
            "تخفیفات خرید",
            "general",
            Some("acc-5000"),
            "credit",
        ),
        (
            "acc-1260",
            "1260",
            "چک‌های برگشتی در پیگیری",
            "detail",
            Some("acc-1200"),
            "credit",
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) \
             VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",
            rusqlite::params![id, code, name, level, parent, nature],
        )?;
    }
    tx.execute(
        "CREATE TABLE IF NOT EXISTS account_mappings(
          company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
          mapping_key TEXT NOT NULL,
          account_id TEXT NOT NULL REFERENCES accounts(id),
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          PRIMARY KEY(company_id, mapping_key)
        )",
        [],
    )?;
    // seed فقط برای company-demo — شرکت‌های تازه باید از تنظیمات پر شوند.
    for (key, account) in [
        ("cash_default", "acc-1101"),
        ("ar_default", "acc-1201"),
        ("ap_default", "acc-2101"),
        ("sales_revenue_default", "acc-4100"),
        ("cogs_default", "acc-5100"),
        ("sales_return_default", "acc-4200"),
        ("purchase_return_default", "acc-5200"),
        ("tax_payable_default", "acc-2401"),
        ("tax_receivable_default", "acc-2401"),
        ("sales_discount_default", "acc-4250"),
        ("purchase_discount_default", "acc-5250"),
        ("check_bounce_tracking_default", "acc-1260"),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO account_mappings(company_id,mapping_key,account_id) \
             VALUES('company-demo',?1,?2)",
            rusqlite::params![key, account],
        )?;
    }
    // حساب‌های کسری و اضافی انبار برای سند تعدیل انبارگردانی.
    for (id, code, name, level, parent, nature) in [
        (
            "acc-6300",
            "6300",
            "کسری و ضایعات انبار",
            "general",
            Some("acc-5000"),
            "debit",
        ),
        (
            "acc-4300",
            "4300",
            "اضافات انبار",
            "general",
            Some("acc-4000"),
            "credit",
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) \
             VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",
            rusqlite::params![id, code, name, level, parent, nature],
        )?;
    }
    // حساب‌های تولید.
    //
    // «موجودی مواد اولیه» و «موجودی کالای ساخته‌شده» جدا از هم‌اند، چون
    // ارزش این دو در ترازنامه معنای متفاوتی دارد و گزارش بهای تمام‌شده به
    // تفکیک آن‌ها نیاز دارد.
    for (id, code, name, level, parent, nature) in [
        (
            "acc-1310",
            "1310",
            "موجودی مواد اولیه",
            "general",
            Some("acc-1000"),
            "debit",
        ),
        (
            "acc-1320",
            "1320",
            "موجودی کالای ساخته شده",
            "general",
            Some("acc-1000"),
            "debit",
        ),
        (
            "acc-5300",
            "5300",
            "دستمزد مستقیم تولید",
            "general",
            Some("acc-5000"),
            "debit",
        ),
        (
            "acc-5400",
            "5400",
            "سربار تولید",
            "general",
            Some("acc-5000"),
            "debit",
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) \
             VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",
            rusqlite::params![id, code, name, level, parent, nature],
        )?;
    }

    // حساب‌های خزانه برای سند دریافت و پرداخت چندروشی.
    for (id, code, name, level, parent, nature) in [
        (
            "acc-1103",
            "1103",
            "اسناد دریافتنی",
            "general",
            Some("acc-1000"),
            "debit",
        ),
        (
            "acc-2103",
            "2103",
            "اسناد پرداختنی",
            "general",
            Some("acc-2000"),
            "credit",
        ),
        (
            "acc-4400",
            "4400",
            "تخفیفات نقدی اعطایی",
            "general",
            Some("acc-4000"),
            "debit",
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) \
             VALUES(?1,'company-demo',?2,?3,?4,?5,?6)",
            rusqlite::params![id, code, name, level, parent, nature],
        )?;
    }
    // دسته‌چک نمونه برای صندوق مرکزی.
    tx.execute(
        "INSERT OR IGNORE INTO checkbooks(id,company_id,treasury_account_id,title,serial_from,serial_to) \
         VALUES('checkbook-demo','company-demo','treasury-cash-demo','دسته‌چک بانک ملت',100001,100025)",
        [],
    )?;
    tx.commit()?;

    // داده‌ی نمونه‌ی گسترده برای تست واقعی محیط نرم‌افزار.
    //
    // قاعده: داده‌ی نمونه **اختیاری** است و هرگز نباید راه‌اندازی برنامه یا
    // مهاجرت پایگاه داده را بشکند. اگر ساخت آن به هر دلیلی ناموفق باشد، برنامه
    // با پایگاه داده‌ی سالم بالا می‌آید و خطا فقط گزارش می‌شود.
    if let Err(error) = demo::seed_demo_dataset(conn) {
        eprintln!("[novin-core] ساخت داده‌ی نمونه انجام نشد: {error}");
    }
    Ok(())
}

/// هش کردن رمز عبور با Argon2id و نمک تصادفی امن.
pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::SaltString;
    use argon2::{Argon2, PasswordHasher};
    use rand_core::OsRng;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

/// اتصال درون‌حافظه‌ای با همان اسکیمای واقعی — ویژه‌ی تست‌های خودکار.
///
/// از آنجا که دقیقاً همان `migrate` و `seed` نسخه‌ی تولیدی اجرا می‌شود،
/// تست‌ها روی ساختار واقعی محصول اعتبارسنجی می‌کنند نه یک شبیه‌سازی.
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

// ---------------------------------------------------------------------------
// گارد نام جدول — دفاع‌در‌عمق علیه SQL Injection از مسیر نام جدول داینامیک
// ---------------------------------------------------------------------------

/// نام جدول‌های سند که میزبان به‌صورت داینامیک در SQL می‌نشیند.
///
/// ## چرا این تابع لازم است
///
/// میزبان در چند نقطه (`next_invoice_number`، ثبت/پست فاکتور، برگشت‌ها)
/// نام جدول را با `format!` داخل SQL می‌گذارد. امروز همه‌ی مقادیر از
/// `if sale {...} else {...}` ثابت می‌آیند، اما هیچ گاردی در مسیر نیست؛
/// اولین فراخوانی آینده با ورودی کاربر یعنی تزریق کامل SQL. این تابع
/// فقط ۸ نام مجاز را قبول می‌کند و بقیه را با خطای صریح رد می‌کند.
pub fn validated_table(name: &str) -> Result<&'static str, String> {
    const ALLOWED: [&str; 8] = [
        "sales_invoices",
        "purchase_invoices",
        "sales_invoice_lines",
        "purchase_invoice_lines",
        "sales_returns",
        "purchase_returns",
        "sales_return_lines",
        "purchase_return_lines",
    ];
    ALLOWED
        .iter()
        .find(|candidate| **candidate == name)
        .copied()
        .ok_or_else(|| format!("SQLGUARD-001: نام جدول مجاز نیست: {name}"))
}
