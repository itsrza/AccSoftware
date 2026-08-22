//! تولید داده‌ی نمونه‌ی گسترده و به‌هم‌پیوسته.
//!
//! هدف: محیطی که بتوان نرم‌افزار را واقعاً با آن تست کرد — نه چند رکورد نمایشی.
//!
//! ## قواعد این ماژول
//!
//! ۱. **همه‌چیز به هم متصل است.** هر فاکتور یک مشتری واقعی، کالای واقعی، انبار
//!    واقعی و سند حسابداری متوازن دارد. هر گردش انبار با موجودی می‌خواند.
//! ۲. **قطعی و تکرارپذیر.** هیچ عدد تصادفی استفاده نمی‌شود؛ همه از اندیس ردیف
//!    مشتق می‌شوند تا داده‌ی دمو در هر اجرا یکسان باشد.
//! ۳. **Idempotent.** اجرای دوباره چیزی را دوبار درج نمی‌کند.
//! ۴. **از نظر حسابداری درست.** جمع بدهکار و بستانکار همه‌ی اسناد برابر است.

use rusqlite::{params, Connection, Result};

const COMPANY: &str = "company-demo";
const FISCAL_YEAR: &str = "fy-demo";
const USER: &str = "user-demo";

/// تعداد رکوردهای هر بخش.
const PRODUCT_COUNT: usize = 60;
const CONTACT_COUNT: usize = 50;
const SALES_INVOICE_COUNT: usize = 55;
const PURCHASE_INVOICE_COUNT: usize = 25;
const CHECK_COUNT: usize = 20;

/// انبارهای نمونه.
const WAREHOUSES: [(&str, &str, &str); 5] = [
    ("wh-main", "W01", "انبار مرکزی"),
    ("wh-branch", "W02", "انبار شعبه"),
    ("wh-mashhad", "W03", "انبار مشهد"),
    ("wh-tehran", "W04", "انبار تهران"),
    ("wh-scrap", "W05", "انبار ضایعات"),
];

/// نام کالاها به تفکیک گروه.
const PRODUCT_NAMES: [(&str, &str, &str, i64); 12] = [
    ("pgroup-food", "برنج ایرانی درجه یک", "کیلوگرم", 980_000),
    ("pgroup-food", "روغن آفتابگردان", "لیتر", 720_000),
    ("pgroup-food", "چای سیاه ممتاز", "بسته", 1_450_000),
    ("pgroup-misc", "کارتن بسته‌بندی", "عدد", 85_000),
    ("pgroup-misc", "نوار چسب پهن", "عدد", 62_000),
    ("pgroup-cosmetic", "شامپو ضدشوره", "عدد", 540_000),
    ("pgroup-cosmetic", "کرم مرطوب‌کننده", "عدد", 890_000),
    ("pgroup-fashion", "پیراهن مردانه", "عدد", 3_400_000),
    ("pgroup-fashion", "شلوار جین", "عدد", 4_800_000),
    ("pgroup-raw", "ورق فلزی", "کیلوگرم", 1_250_000),
    ("pgroup-raw", "پارچه نخی", "متر", 640_000),
    ("pgroup-misc", "لامپ ال‌ای‌دی", "عدد", 320_000),
];

/// نام‌های ایرانی برای تولید اشخاص.
const FIRST_NAMES: [&str; 10] = [
    "محمد",
    "علی",
    "رضا",
    "حسین",
    "مهدی",
    "فاطمه",
    "زهرا",
    "مریم",
    "سارا",
    "نگار",
];
const LAST_NAMES: [&str; 10] = [
    "محمدی",
    "احمدی",
    "رضایی",
    "حسینی",
    "کریمی",
    "موسوی",
    "جعفری",
    "قاسمی",
    "نوری",
    "صادقی",
];
const COMPANY_NAMES: [&str; 8] = [
    "بازرگانی پارس",
    "شرکت آریا تجارت",
    "فروشگاه زنجیره‌ای مهر",
    "توزیع کالای البرز",
    "پخش سراسری ایرانیان",
    "صنایع غذایی سپید",
    "گروه تجاری کوروش",
    "بازرگانی نیک‌اندیش",
];
const CITIES: [&str; 6] = ["تهران", "مشهد", "اصفهان", "شیراز", "تبریز", "کرج"];

/// تاریخ شمسی نمونه بر اساس اندیس — **صعودی و یکنواخت**.
///
/// یکنواخت بودن مهم است: سررسید چک باید همیشه بعد از تاریخ صدور بیفتد، و
/// تاریخ اسناد نباید از سال مالی بیرون بزند.
fn demo_date(offset: usize) -> String {
    let year = 1405 + offset / (28 * 12);
    let month = (offset / 28) % 12 + 1;
    let day = offset % 28 + 1;
    format!("{year}/{month:02}/{day:02}")
}

/// درج داده‌ی نمونه‌ی گسترده. اجرای دوباره بی‌اثر است.
pub fn seed_demo_dataset(conn: &Connection) -> Result<()> {
    // اگر قبلاً ساخته شده، دوباره نساز.
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM products WHERE id LIKE 'demo-prod-%'",
        [],
        |row| row.get(0),
    )?;
    if existing >= PRODUCT_COUNT as i64 {
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;

    seed_required_accounts(&tx)?;
    seed_warehouses(&tx)?;
    seed_treasury(&tx)?;
    seed_products(&tx)?;
    seed_contacts(&tx)?;
    seed_inventory(&tx)?;
    seed_sales(&tx)?;
    seed_purchases(&tx)?;
    seed_treasury_documents(&tx)?;
    seed_checks(&tx)?;

    tx.commit()?;
    Ok(())
}

/// حساب‌هایی که داده‌ی نمونه به آن‌ها ارجاع می‌دهد و ممکن است در کدینگ پایه نباشند.
///
/// بدون این تابع، سطرهای سند به حساب ناموجود ارجاع می‌دهند و کلید خارجی می‌شکند.
fn seed_required_accounts(tx: &Connection) -> Result<()> {
    for (id, code, name, level, parent, nature) in [
        (
            "acc-1200",
            "1200",
            "حساب های دریافتنی",
            "general",
            Some("acc-1000"),
            "debit",
        ),
        (
            "acc-1201",
            "1201",
            "حساب مشتریان",
            "detail",
            Some("acc-1200"),
            "debit",
        ),
        (
            "acc-2100",
            "2100",
            "حساب های پرداختنی",
            "general",
            Some("acc-2000"),
            "credit",
        ),
        (
            "acc-2101",
            "2101",
            "تأمین کنندگان",
            "detail",
            Some("acc-2100"),
            "credit",
        ),
        (
            "acc-2401",
            "2401",
            "مالیات بر ارزش افزوده",
            "general",
            Some("acc-2000"),
            "credit",
        ),
        (
            "acc-1100",
            "1100",
            "دارایی های جاری",
            "general",
            Some("acc-1000"),
            "debit",
        ),
        (
            "acc-1101",
            "1101",
            "موجودی نقد و بانک",
            "detail",
            Some("acc-1100"),
            "debit",
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO accounts(id,company_id,code,name,level,parent_id,nature) \
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![id, COMPANY, code, name, level, parent, nature],
        )?;
    }
    Ok(())
}

fn seed_warehouses(tx: &Connection) -> Result<()> {
    for (id, code, name) in WAREHOUSES {
        tx.execute(
            "INSERT OR IGNORE INTO warehouses(id,company_id,name,code) VALUES(?1,?2,?3,?4)",
            params![id, COMPANY, name, code],
        )?;
    }
    Ok(())
}

/// حساب‌های خزانه.
///
/// نکته‌ی مهم: نام‌ها نباید با داده‌ی پایه تداخل کنند. جدول `treasury_accounts`
/// روی `(company_id, name)` یکتاست و در SQLite قاعده‌ی `INSERT OR IGNORE`
/// خطای **کلید خارجی** را نادیده نمی‌گیرد — پس اگر درج به‌خاطر نام تکراری رد
/// شود، هر ارجاع بعدی به آن حساب کل مهاجرت را می‌شکند.
fn seed_treasury(tx: &Connection) -> Result<()> {
    let accounts: [(&str, &str, &str, &str); 5] = [
        ("treasury-cash-1", "صندوق نقدی شماره ۱", "cash", "acc-1101"),
        ("treasury-cash-2", "صندوق فروشگاه شماره ۲", "cash", "acc-1101"),
        (
            "treasury-bank-mellat",
            "بانک ملت — جاری ۱۲۳۴",
            "bank",
            "acc-1101",
        ),
        (
            "treasury-bank-saderat",
            "بانک صادرات — جاری ۵۶۷۸",
            "bank",
            "acc-1101",
        ),
        ("treasury-petty", "تنخواه اداری", "petty_cash", "acc-1101"),
    ];
    for (index, (id, name, kind, linked)) in accounts.iter().enumerate() {
        tx.execute(
            "INSERT OR IGNORE INTO treasury_accounts(id,company_id,name,account_type,linked_account_id) \
             VALUES(?1,?2,?3,?4,?5)",
            params![id, COMPANY, name, kind, linked],
        )?;
        if *kind == "bank" {
            tx.execute(
                "UPDATE treasury_accounts SET account_number=?1, branch_name=?2, \
                 negative_policy='warn', has_pos_terminal=1 WHERE id=?3",
                params![
                    format!("{}00{}", 1000 + index * 137, index),
                    format!("شعبه {}", CITIES[index % CITIES.len()]),
                    id
                ],
            )?;
        }
    }
    tx.execute(
        "INSERT OR IGNORE INTO pos_terminals(id,company_id,treasury_account_id,title,terminal_number) \
         VALUES('pos-1',?1,'treasury-bank-mellat','کارتخوان صندوق ۱','12345678')",
        params![COMPANY],
    )?;
    Ok(())
}

fn seed_products(tx: &Connection) -> Result<()> {
    for index in 0..PRODUCT_COUNT {
        let (group, base_name, unit, base_price) = PRODUCT_NAMES[index % PRODUCT_NAMES.len()];
        let variant = index / PRODUCT_NAMES.len() + 1;
        let id = format!("demo-prod-{index:03}");
        let sku = format!("{}", 10_001 + index);
        let name = if variant > 1 {
            format!("{base_name} مدل {variant}")
        } else {
            base_name.to_string()
        };
        // قیمت با تغییر کنترل‌شده بر اساس اندیس، نه تصادفی
        let sale_price = base_price + (index as i64 % 7) * 25_000;
        let purchase_price = sale_price * 72 / 100;

        tx.execute(
            "INSERT OR IGNORE INTO products(id,company_id,sku,barcode,name,unit,sale_price,\
             purchase_price,min_stock,is_service) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,0)",
            params![
                id,
                COMPANY,
                sku,
                format!("690{:010}", 1_000_000 + index),
                name,
                unit,
                sale_price,
                purchase_price,
                (index % 5 + 3) as f64
            ],
        )?;
        tx.execute(
            "UPDATE products SET group_id=?1, kind='simple', vat_basis_points=900, \
             reorder_point=?2, max_stock=?3 WHERE id=?4",
            params![group, (index % 5 + 3) as f64, 500.0, id],
        )?;
        // سطوح قیمت
        for (level, ratio) in [("retail", 100), ("wholesale", 94), ("partner", 88)] {
            tx.execute(
                "INSERT OR IGNORE INTO product_prices(product_id,level,price) VALUES(?1,?2,?3)",
                params![id, level, sale_price * ratio / 100],
            )?;
        }
        // تخفیف پلکانی برای یک‌سوم کالاها
        if index % 3 == 0 {
            for (min_quantity, discount_bp) in [(10.0, 300), (50.0, 700)] {
                tx.execute(
                    "INSERT OR IGNORE INTO product_discount_tiers(id,product_id,min_quantity,discount_bp) \
                     VALUES(?1,?2,?3,?4)",
                    params![
                        format!("{id}-tier-{}", min_quantity as i64),
                        id,
                        min_quantity,
                        discount_bp
                    ],
                )?;
            }
        }
    }
    Ok(())
}

fn seed_contacts(tx: &Connection) -> Result<()> {
    for index in 0..CONTACT_COUNT {
        let id = format!("demo-contact-{index:03}");
        let is_legal = index % 4 == 0;
        let name = if is_legal {
            format!(
                "{} {}",
                COMPANY_NAMES[index % COMPANY_NAMES.len()],
                index / 8 + 1
            )
        } else {
            format!(
                "{} {}",
                FIRST_NAMES[index % FIRST_NAMES.len()],
                LAST_NAMES[(index / 3) % LAST_NAMES.len()]
            )
        };
        // ترکیب نقش‌ها: بیشتر مشتری، بخشی تأمین‌کننده، بخشی هر دو
        let (is_customer, is_supplier) = match index % 7 {
            5 => (0, 1),
            6 => (1, 1),
            _ => (1, 0),
        };
        tx.execute(
            "INSERT OR IGNORE INTO contacts(id,company_id,kind,name,mobile,phone,address,\
             is_customer,is_supplier,credit_limit) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                id,
                COMPANY,
                if is_legal { "company" } else { "person" },
                name,
                format!("0912{:07}", 1_000_000 + index * 137),
                format!("021{:08}", 22_000_000 + index * 371),
                format!(
                    "{}، خیابان نمونه، پلاک {}",
                    CITIES[index % CITIES.len()],
                    index + 1
                ),
                is_customer,
                is_supplier,
                (index as i64 % 5 + 1) * 200_000_000
            ],
        )?;
        tx.execute(
            "UPDATE contacts SET party_type=?1, party_function='person', company_name=?2, \
             route_id=?3, opening_date='1405/01/01' WHERE id=?4",
            params![
                if is_legal { "private_legal" } else { "natural" },
                if is_legal { Some(name.as_str()) } else { None },
                if index % 2 == 0 {
                    "route-center"
                } else {
                    "route-north"
                },
                id
            ],
        )?;
    }
    Ok(())
}

/// موجودی اولیه: هر کالا با یک رسید انبار وارد انبار مرکزی می‌شود.
fn seed_inventory(tx: &Connection) -> Result<()> {
    for index in 0..PRODUCT_COUNT {
        let product = format!("demo-prod-{index:03}");
        let warehouse = WAREHOUSES[index % 3].0; // پخش بین سه انبار اصلی
        let quantity = (index % 40 + 8) as f64;
        let unit_cost: i64 = tx.query_row(
            "SELECT purchase_price FROM products WHERE id=?1",
            params![product],
            |row| row.get(0),
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
             movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,'receipt',?5,?6,'opening','demo','موجودی اول دوره',?7)",
            params![
                format!("demo-move-{index:03}"),
                COMPANY,
                product,
                warehouse,
                quantity,
                unit_cost,
                USER
            ],
        )?;
        tx.execute(
            "INSERT INTO inventory_balances(product_id,warehouse_id,quantity) VALUES(?1,?2,?3) \
             ON CONFLICT(product_id,warehouse_id) DO UPDATE SET quantity=excluded.quantity",
            params![product, warehouse, quantity],
        )?;
    }
    Ok(())
}

/// ساخت یک سند حسابداری متوازن.
fn insert_journal(
    tx: &Connection,
    id: &str,
    number: i64,
    date: &str,
    description: &str,
    source: &str,
    lines: &[(&str, i64, i64)],
) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,\
         description,status,source_type,created_by) VALUES(?1,?2,?3,?4,?5,?6,'posted',?7,?8)",
        params![
            id,
            COMPANY,
            FISCAL_YEAR,
            number,
            date,
            description,
            source,
            USER
        ],
    )?;
    for (index, (account, debit, credit)) in lines.iter().enumerate() {
        tx.execute(
            "INSERT OR IGNORE INTO journal_lines(id,journal_id,account_id,description,debit,credit) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{id}-l{index}"),
                id,
                account,
                description,
                debit,
                credit
            ],
        )?;
    }
    Ok(())
}

fn seed_sales(tx: &Connection) -> Result<()> {
    for index in 0..SALES_INVOICE_COUNT {
        let invoice_id = format!("demo-sale-{index:03}");
        let contact = format!("demo-contact-{:03}", index % CONTACT_COUNT);
        let warehouse = WAREHOUSES[index % 3].0;
        let date = demo_date(index);
        let line_count = index % 3 + 1;

        let mut subtotal: i64 = 0;
        let mut lines = Vec::new();
        for line_index in 0..line_count {
            let product_index = (index * 3 + line_index) % PRODUCT_COUNT;
            let product = format!("demo-prod-{product_index:03}");
            let (price, _cost): (i64, i64) = tx.query_row(
                "SELECT sale_price,purchase_price FROM products WHERE id=?1",
                params![product],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            let quantity = (line_index + index % 4 + 1) as f64;
            let total = (price as f64 * quantity).round() as i64;
            subtotal += total;
            lines.push((product, quantity, price, total));
        }
        let tax = subtotal * 9 / 100;
        let total = subtotal + tax;
        // یک‌سوم فاکتورها تسویه‌نشده می‌مانند تا گزارش بدهکاران معنا پیدا کند.
        let payment_status = if index % 3 == 0 { "unpaid" } else { "paid" };

        tx.execute(
            "INSERT OR IGNORE INTO sales_invoices(id,company_id,fiscal_year_id,number,invoice_date,\
             contact_id,warehouse_id,status,payment_status,subtotal,discount,tax,total,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,'posted',?8,?9,0,?10,?11,?12)",
            params![
                invoice_id, COMPANY, FISCAL_YEAR, 1000 + index as i64, date, contact,
                warehouse, payment_status, subtotal, tax, total, USER
            ],
        )?;
        for (line_index, (product, quantity, price, line_total)) in lines.iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO sales_invoice_lines(id,invoice_id,product_id,quantity,\
                 unit_price,line_total) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    format!("{invoice_id}-l{line_index}"),
                    invoice_id,
                    product,
                    quantity,
                    price,
                    line_total
                ],
            )?;
            // خروج کالا از انبار
            tx.execute(
                "INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
                 movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
                 VALUES(?1,?2,?3,?4,'issue',?5,0,'sales_invoice',?6,'فروش',?7)",
                params![
                    format!("{invoice_id}-mv{line_index}"),
                    COMPANY,
                    product,
                    warehouse,
                    quantity,
                    invoice_id,
                    USER
                ],
            )?;
        }
        insert_journal(
            tx,
            &format!("demo-jrn-sale-{index:03}"),
            2000 + index as i64,
            &date,
            &format!("فاکتور فروش شماره {}", 1000 + index),
            "sales_invoice",
            &[
                ("acc-1201", total, 0),
                ("acc-4100", 0, subtotal),
                ("acc-2401", 0, tax),
            ],
        )?;
    }
    Ok(())
}

fn seed_purchases(tx: &Connection) -> Result<()> {
    for index in 0..PURCHASE_INVOICE_COUNT {
        let invoice_id = format!("demo-purchase-{index:03}");
        // تأمین‌کننده‌ها اندیس‌هایی هستند که is_supplier دارند
        let contact = format!("demo-contact-{:03}", (index * 7 + 5) % CONTACT_COUNT);
        let warehouse = WAREHOUSES[index % 2].0;
        let date = demo_date(index + 3);

        let product_index = (index * 5) % PRODUCT_COUNT;
        let product = format!("demo-prod-{product_index:03}");
        let cost: i64 = tx.query_row(
            "SELECT purchase_price FROM products WHERE id=?1",
            params![product],
            |row| row.get(0),
        )?;
        let quantity = (index % 20 + 5) as f64;
        let subtotal = (cost as f64 * quantity).round() as i64;
        let tax = subtotal * 9 / 100;
        let total = subtotal + tax;

        tx.execute(
            "INSERT OR IGNORE INTO purchase_invoices(id,company_id,fiscal_year_id,number,\
             invoice_date,contact_id,warehouse_id,status,payment_status,subtotal,discount,tax,\
             total,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,'posted','paid',?8,0,?9,?10,?11)",
            params![
                invoice_id,
                COMPANY,
                FISCAL_YEAR,
                500 + index as i64,
                date,
                contact,
                warehouse,
                subtotal,
                tax,
                total,
                USER
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO purchase_invoice_lines(id,invoice_id,product_id,quantity,\
             unit_price,line_total) VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{invoice_id}-l0"),
                invoice_id,
                product,
                quantity,
                cost,
                subtotal
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
             movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
             VALUES(?1,?2,?3,?4,'receipt',?5,?6,'purchase_invoice',?7,'خرید',?8)",
            params![
                format!("{invoice_id}-mv0"),
                COMPANY,
                product,
                warehouse,
                quantity,
                cost,
                invoice_id,
                USER
            ],
        )?;
        insert_journal(
            tx,
            &format!("demo-jrn-purchase-{index:03}"),
            3000 + index as i64,
            &date,
            &format!("فاکتور خرید شماره {}", 500 + index),
            "purchase_invoice",
            &[
                ("acc-1300", subtotal, 0),
                ("acc-2401", tax, 0),
                ("acc-2101", 0, total),
            ],
        )?;
    }
    Ok(())
}

fn seed_treasury_documents(tx: &Connection) -> Result<()> {
    for index in 0..20 {
        let doc_id = format!("demo-receipt-{index:03}");
        let contact = format!("demo-contact-{:03}", (index * 3) % CONTACT_COUNT);
        let date = demo_date(index + 1);
        let amount = ((index as i64 % 8) + 1) * 12_500_000;
        let treasury = if index % 2 == 0 {
            "treasury-cash-1"
        } else {
            "treasury-bank-mellat"
        };

        tx.execute(
            "INSERT OR IGNORE INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,\
             document_date,party_id,description,total,status,created_by) \
             VALUES(?1,?2,?3,'receipt',?4,?5,?6,'دریافت از مشتری',?7,'posted',?8)",
            params![
                doc_id,
                COMPANY,
                FISCAL_YEAR,
                100 + index as i64,
                date,
                contact,
                amount,
                USER
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO treasury_document_lines(id,document_id,method,amount,\
             treasury_account_id) VALUES(?1,?2,?3,?4,?5)",
            params![
                format!("{doc_id}-l0"),
                doc_id,
                if index % 3 == 0 {
                    "bank_transfer"
                } else {
                    "cash"
                },
                amount,
                treasury
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO treasury_transactions(id,company_id,fiscal_year_id,\
             treasury_account_id,transaction_type,amount,transaction_date,description,\
             reference_type,reference_id,created_by) \
             VALUES(?1,?2,?3,?4,'receipt',?5,?6,'دریافت از مشتری','treasury_document',?7,?8)",
            params![
                format!("demo-tx-{index:03}"),
                COMPANY,
                FISCAL_YEAR,
                treasury,
                amount,
                date,
                doc_id,
                USER
            ],
        )?;
        insert_journal(
            tx,
            &format!("demo-jrn-receipt-{index:03}"),
            4000 + index as i64,
            &date,
            "سند دریافت",
            "treasury_document",
            &[("acc-1101", amount, 0), ("acc-1201", 0, amount)],
        )?;
    }
    Ok(())
}

fn seed_checks(tx: &Connection) -> Result<()> {
    let statuses = [
        "registered",
        "in_progress",
        "cleared",
        "bounced",
        "transferred",
    ];
    for index in 0..CHECK_COUNT {
        let contact = format!("demo-contact-{:03}", (index * 5) % CONTACT_COUNT);
        let issue = demo_date(index);
        // سررسید همیشه پس از تاریخ صدور
        let due = demo_date(index + 45);
        tx.execute(
            "INSERT OR IGNORE INTO checks(id,company_id,fiscal_year_id,check_type,check_number,\
             party_id,treasury_account_id,amount,issue_date,due_date,status,bank_name,description,\
             created_by) VALUES(?1,?2,?3,?4,?5,?6,'treasury-cash-1',?7,?8,?9,?10,?11,'چک نمونه',?12)",
            params![
                format!("demo-check-{index:03}"),
                COMPANY,
                FISCAL_YEAR,
                if index % 4 == 3 { "issued" } else { "received" },
                format!("{}", 700_100 + index),
                contact,
                ((index as i64 % 10) + 1) * 8_500_000,
                issue,
                due,
                statuses[index % statuses.len()],
                format!("بانک {}", CITIES[index % CITIES.len()]),
                USER
            ],
        )?;
    }
    Ok(())
}
