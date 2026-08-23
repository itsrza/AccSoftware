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

/// برچسب‌گذاری خطای هر مرحله تا در صورت شکست، دقیقاً بدانیم کجا بوده است.
///
/// از `ToSqlConversionFailure` استفاده می‌کنیم چون تنها گونه‌ی `rusqlite::Error`
/// است که بدون فیچر اضافی می‌پذیرد پیام دلخواه را حمل کند
/// (`ModuleError` تنها با فیچر `vtab` در دسترس است).
fn step(name: &str, outcome: Result<()>) -> Result<()> {
    outcome.map_err(|error| {
        let labelled = format!("مرحله‌ی داده‌ی نمونه «{name}»: {error}");
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(labelled)))
    })
}

/// خواندن شناسه‌های واقعی یک جدول پس از درج.
///
/// چرا لازم است: اگر درجی به‌خاطر یکتا بودن نام یا کد نادیده گرفته شود،
/// شناسه‌ی hardcode شده وجود نخواهد داشت و هر ارجاع بعدی کلید خارجی را
/// می‌شکند. با خواندن شناسه‌های واقعی، داده‌ی نمونه در برابر تداخل با داده‌ی
/// پایه مقاوم می‌شود.
/// نخستین شماره‌ی آزاد یک دفتر شماره‌گذاری‌شده.
///
/// چرا لازم است: جدول‌های سند و فاکتور قید `UNIQUE(company_id,fiscal_year_id,number)`
/// دارند. اگر داده‌ی نمونه شماره‌ای بسازد که داده‌ی پایه از قبل مصرف کرده،
/// `INSERT OR IGNORE` سطر والد را **بی‌صدا** رد می‌کند و سطرهای فرزند
/// (اقلام فاکتور، سطور سند) کلید خارجی را می‌شکنند. با شروع از یک شماره‌ی
/// بالاتر از بیشینه‌ی موجود، تداخل شماره از اساس ممکن نیست.
///
/// این همان قاعده‌ی دفترنویسی است: شماره‌ی سند در هر سال مالی یکتا و پیوسته.
fn next_number(tx: &Connection, table: &str) -> Result<i64> {
    let sql = format!(
        "SELECT COALESCE(MAX(number),0)+1 FROM {table} WHERE company_id=?1 AND fiscal_year_id=?2"
    );
    tx.query_row(&sql, params![COMPANY, FISCAL_YEAR], |row| row.get(0))
}

/// اطمینان از اینکه سطر والد واقعاً درج شده است.
///
/// اگر `INSERT OR IGNORE` سطر را رد کرده باشد، به‌جای خطای مبهم کلید خارجی در
/// چند سطر بعد، همین‌جا با نام جدول و شناسه خطا می‌دهیم.
fn require_row(tx: &Connection, table: &str, id: &str) -> Result<()> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE id=?1");
    let exists: i64 = tx.query_row(&sql, params![id], |row| row.get(0))?;
    if exists == 0 {
        let message = format!("سطر والد «{id}» در جدول «{table}» درج نشد");
        return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
            std::io::Error::other(message),
        )));
    }
    Ok(())
}

fn collect_ids(tx: &Connection, sql: &str) -> Result<Vec<String>> {
    let mut statement = tx.prepare(sql)?;
    let rows = statement
        .query_map(params![COMPANY], |row| row.get::<_, String>(0))?
        .filter_map(std::result::Result::ok)
        .collect();
    Ok(rows)
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
        // داده‌ی جریان اصلی قبلاً ساخته شده است.
        //
        // ولی نسخه‌های بعدی ممکن است بخش‌های پشتیبان تازه‌ای اضافه کنند
        // (تولید، انبارگردانی، قالب چاپ…). اگر اینجا زودهنگام برگردیم،
        // کاربری که قبلاً برنامه را باز کرده هرگز آن‌ها را نمی‌بیند. پس
        // فقط بخش پشتیبان — که خودش Idempotent است — دوباره اجرا می‌شود.
        let tx = conn.unchecked_transaction()?;
        let warehouse_ids = collect_ids(
            &tx,
            "SELECT id FROM warehouses WHERE company_id=?1 ORDER BY code",
        )?;
        if !warehouse_ids.is_empty() {
            step(
                "supporting",
                super::demo_extras::seed_supporting_data(&tx, &warehouse_ids),
            )?;
        }
        tx.commit()?;
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;

    step("accounts", seed_required_accounts(&tx))?;
    step("warehouses", seed_warehouses(&tx))?;
    step("treasury", seed_treasury(&tx))?;

    // شناسه‌های واقعی پس از درج خوانده می‌شوند تا ارجاع‌ها همیشه معتبر باشند.
    let warehouse_ids = collect_ids(
        &tx,
        "SELECT id FROM warehouses WHERE company_id=?1 ORDER BY code",
    )?;
    let treasury_ids = collect_ids(
        &tx,
        "SELECT id FROM treasury_accounts WHERE company_id=?1 ORDER BY account_type,id",
    )?;
    if warehouse_ids.is_empty() || treasury_ids.is_empty() {
        return Ok(());
    }

    step("products", seed_products(&tx))?;
    step("contacts", seed_contacts(&tx))?;
    step("inventory", seed_inventory(&tx, &warehouse_ids))?;
    step("sales", seed_sales(&tx, &warehouse_ids))?;
    step("purchases", seed_purchases(&tx, &warehouse_ids))?;
    step(
        "treasury_documents",
        seed_treasury_documents(&tx, &treasury_ids),
    )?;
    step("checks", seed_checks(&tx, &treasury_ids))?;
    step("returns", seed_returns(&tx, &warehouse_ids))?;
    step("transfers", seed_transfers(&tx, &warehouse_ids))?;
    step("quotes", seed_quotes(&tx, &warehouse_ids))?;
    step(
        "supporting",
        super::demo_extras::seed_supporting_data(&tx, &warehouse_ids),
    )?;

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
        (
            "treasury-cash-2",
            "صندوق فروشگاه شماره ۲",
            "cash",
            "acc-1101",
        ),
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
    // پایانه فقط وقتی ساخته می‌شود که حساب بانکی‌اش واقعاً وجود داشته باشد.
    let bank: Option<String> = tx
        .query_row(
            "SELECT id FROM treasury_accounts WHERE company_id=?1 AND account_type='bank' \
             ORDER BY id LIMIT 1",
            params![COMPANY],
            |row| row.get(0),
        )
        .ok();
    if let Some(bank_id) = bank {
        tx.execute(
            "INSERT OR IGNORE INTO pos_terminals(id,company_id,treasury_account_id,title,\
             terminal_number) VALUES('pos-1',?1,?2,'کارتخوان صندوق ۱','12345678')",
            params![COMPANY, bank_id],
        )?;
    }
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
        // گروه‌بندی معنادار: تأمین‌کننده‌ها بستانکار تجاری، بقیه بدهکار تجاری.
        let group = match index % 7 {
            5 => "pgroup-trade-creditor",
            6 => "pgroup-colleagues",
            _ if index % 11 == 0 => "pgroup-vip",
            _ => "pgroup-trade-debtor",
        };
        tx.execute(
            "UPDATE contacts SET code=?1, group_id=?2, is_active=1, city=?3, province=?4, \
             email=?5 WHERE id=?6",
            params![
                format!("{}", 1001 + index),
                group,
                CITIES[index % CITIES.len()],
                CITIES[index % CITIES.len()],
                format!("contact{index}@example.com"),
                id
            ],
        )?;
        // تلفن ثابت به‌عنوان شماره‌ی پیش‌فرض
        tx.execute(
            "INSERT OR IGNORE INTO party_phones(id,contact_id,title,number,is_primary) \
             VALUES(?1,?2,'دفتر',?3,1)",
            params![
                format!("{id}-phone-0"),
                id,
                format!("021{:08}", 22_000_000 + index * 371)
            ],
        )?;
        // یک‌سوم اشخاص حساب بانکی ثبت‌شده دارند
        if index % 3 == 0 {
            tx.execute(
                "INSERT OR IGNORE INTO party_bank_accounts(id,contact_id,bank_name,branch_name,\
                 account_number,holder_name,is_default) VALUES(?1,?2,?3,'شعبه مرکزی',?4,?5,1)",
                params![
                    format!("{id}-bank-0"),
                    id,
                    format!("بانک {}", CITIES[(index + 2) % CITIES.len()]),
                    format!("{}", 4_000_000 + index * 17),
                    name
                ],
            )?;
        }
        // مناسبت تولد برای یک‌چهارم اشخاص
        if index % 4 == 0 {
            tx.execute(
                "INSERT OR IGNORE INTO party_occasions(id,contact_id,title,jalali_month,\
                 jalali_day,remind_days_before) VALUES(?1,?2,'تولد',?3,?4,3)",
                params![
                    format!("{id}-occasion-0"),
                    id,
                    (index % 12 + 1) as i64,
                    (index % 28 + 1) as i64
                ],
            )?;
        }
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
fn seed_inventory(tx: &Connection, warehouses: &[String]) -> Result<()> {
    for index in 0..PRODUCT_COUNT {
        let product = format!("demo-prod-{index:03}");
        let warehouse = &warehouses[index % warehouses.len()];
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
    // اگر حتی یکی از حساب‌ها موجود نباشد، سند اصلاً صادر نمی‌شود؛ سند ناقص
    // بدتر از نبود سند است.
    for (account, _, _) in lines {
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM accounts WHERE id=?1",
            params![account],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(());
        }
    }
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

fn seed_sales(tx: &Connection, warehouses: &[String]) -> Result<()> {
    // شماره‌ی فاکتور و شماره‌ی سند از بیشینه‌ی موجود ادامه می‌یابند تا با
    // داده‌ی پایه تداخل نکنند.
    let first_invoice_number = next_number(tx, "sales_invoices")?;
    let first_journal_number = next_number(tx, "journal_entries")?;
    for index in 0..SALES_INVOICE_COUNT {
        let invoice_id = format!("demo-sale-{index:03}");
        let contact = format!("demo-contact-{:03}", index % CONTACT_COUNT);
        let warehouse = &warehouses[index % warehouses.len()];
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
                invoice_id, COMPANY, FISCAL_YEAR, first_invoice_number + index as i64, date, contact,
                warehouse, payment_status, subtotal, tax, total, USER
            ],
        )?;
        require_row(tx, "sales_invoices", &invoice_id)?;
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
            first_journal_number + index as i64,
            &date,
            &format!("فاکتور فروش شماره {}", first_invoice_number + index as i64),
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

fn seed_purchases(tx: &Connection, warehouses: &[String]) -> Result<()> {
    let first_invoice_number = next_number(tx, "purchase_invoices")?;
    let first_journal_number = next_number(tx, "journal_entries")?;
    for index in 0..PURCHASE_INVOICE_COUNT {
        let invoice_id = format!("demo-purchase-{index:03}");
        // تأمین‌کننده‌ها اندیس‌هایی هستند که is_supplier دارند
        let contact = format!("demo-contact-{:03}", (index * 7 + 5) % CONTACT_COUNT);
        let warehouse = &warehouses[index % warehouses.len()];
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
                first_invoice_number + index as i64,
                date,
                contact,
                warehouse,
                subtotal,
                tax,
                total,
                USER
            ],
        )?;
        require_row(tx, "purchase_invoices", &invoice_id)?;
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
            first_journal_number + index as i64,
            &date,
            &format!("فاکتور خرید شماره {}", first_invoice_number + index as i64),
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

fn seed_treasury_documents(tx: &Connection, treasury: &[String]) -> Result<()> {
    let first_document_number = next_number(tx, "treasury_documents")?;
    let first_journal_number = next_number(tx, "journal_entries")?;
    for index in 0..20 {
        let doc_id = format!("demo-receipt-{index:03}");
        let contact = format!("demo-contact-{:03}", (index * 3) % CONTACT_COUNT);
        let treasury = &treasury[index % treasury.len()];

        // مبلغ دریافت از بدهی واقعی همان مشتری می‌آید، نه از عددی دلخواه.
        //
        // چرا مهم است: دریافت بیش از بدهی، حساب مشتریان را بستانکار می‌کند —
        // یعنی گزارش می‌گوید شرکت به مشتری بدهکار است. این وضعیت فقط با
        // پیش‌دریافت واقعی معنا دارد، نه به‌عنوان داده‌ی نمونه‌ی تصادفی.
        let owed: i64 = tx
            .query_row(
                "SELECT COALESCE(SUM(total),0) FROM sales_invoices \
                 WHERE contact_id=?1 AND status='posted'",
                params![contact],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let already_received: i64 = tx
            .query_row(
                "SELECT COALESCE(SUM(total),0) FROM treasury_documents \
                 WHERE party_id=?1 AND kind='receipt'",
                params![contact],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let outstanding = owed - already_received;
        if outstanding <= 0 {
            // این مشتری بدهی ندارد؛ دریافتی هم ثبت نمی‌شود.
            continue;
        }
        // دریافت جزئی — حالت رایج واقعی — ولی هرگز بیش از بدهی.
        let amount = (outstanding * 60 / 100).max(1).min(outstanding);

        // تاریخ دریافت باید پس از تاریخ فاکتور باشد.
        let date = demo_date(index + 70);

        tx.execute(
            "INSERT OR IGNORE INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,\
             document_date,party_id,description,total,status,created_by) \
             VALUES(?1,?2,?3,'receipt',?4,?5,?6,'دریافت از مشتری',?7,'posted',?8)",
            params![
                doc_id,
                COMPANY,
                FISCAL_YEAR,
                first_document_number + index as i64,
                date,
                contact,
                amount,
                USER
            ],
        )?;
        require_row(tx, "treasury_documents", &doc_id)?;
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
            first_journal_number + index as i64,
            &date,
            "سند دریافت",
            "treasury_document",
            &[("acc-1101", amount, 0), ("acc-1201", 0, amount)],
        )?;
    }
    Ok(())
}

fn seed_checks(tx: &Connection, treasury: &[String]) -> Result<()> {
    // وضعیت‌ها به تفکیک نوع چک انتخاب می‌شوند؛ وضعیت چک دریافتی روی چک
    // پرداختی معنا ندارد و برعکس.
    let received_statuses = ["in_hand", "deposited", "collected", "bounced", "endorsed"];
    let issued_statuses = ["outstanding", "paid", "bounced"];
    for index in 0..CHECK_COUNT {
        let check_type = if index % 4 == 3 { "issued" } else { "received" };
        let status = if check_type == "issued" {
            issued_statuses[index % issued_statuses.len()]
        } else {
            received_statuses[index % received_statuses.len()]
        };
        let contact = format!("demo-contact-{:03}", (index * 5) % CONTACT_COUNT);
        let issue = demo_date(index);
        // سررسید همیشه پس از تاریخ صدور
        let due = demo_date(index + 45);
        tx.execute(
            "INSERT OR IGNORE INTO checks(id,company_id,fiscal_year_id,check_type,check_number,\
             party_id,treasury_account_id,amount,issue_date,due_date,status,bank_name,description,\
             created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'چک نمونه',?13)",
            params![
                format!("demo-check-{index:03}"),
                COMPANY,
                FISCAL_YEAR,
                check_type,
                format!("{}", 700_100 + index),
                contact,
                treasury[index % treasury.len()],
                ((index as i64 % 10) + 1) * 8_500_000,
                issue,
                due,
                status,
                format!("بانک {}", CITIES[index % CITIES.len()]),
                USER
            ],
        )?;
    }
    Ok(())
}

/// برگشت از فروش و خرید نمونه.
///
/// قاعده‌ای که رعایت می‌شود: مقدار برگشتی هرگز از مقدار فاکتور اصلی بیشتر
/// نیست. بخشی از برگشت‌ها پیش‌نویس می‌مانند تا هر دو وضعیت در نرم‌افزار
/// قابل مشاهده باشد. سند حسابداری در همین‌جا صادر نمی‌شود؛ صدور سند کار
/// «ثبت قطعی» است و باید از مسیر واقعی برنامه انجام شود.
fn seed_returns(tx: &Connection, warehouses: &[String]) -> Result<()> {
    let first_number = next_number(tx, "sales_returns")?;
    for index in 0..8usize {
        let invoice_id = format!("demo-sale-{:03}", index * 6);
        // اگر فاکتور مرجع وجود نداشته باشد، از این برگشت صرف‌نظر می‌شود.
        let invoice: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT id,contact_id FROM sales_invoices WHERE id=?1",
                params![invoice_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        let Some((invoice_id, contact)) = invoice else {
            continue;
        };

        // نخستین قلم فاکتور، با نصف مقدار — برگشت جزئی، حالت رایج واقعی.
        let line: Option<(String, f64, i64)> = tx
            .query_row(
                "SELECT product_id,quantity,unit_price FROM sales_invoice_lines \
                 WHERE invoice_id=?1 ORDER BY id LIMIT 1",
                params![invoice_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();
        let Some((product, quantity, price)) = line else {
            continue;
        };
        let returned = (quantity / 2.0).max(1.0).min(quantity);
        let total = (returned * price as f64).round() as i64;

        let return_id = format!("demo-return-{index:03}");
        let status = if index % 3 == 0 { "draft" } else { "posted" };
        tx.execute(
            "INSERT OR IGNORE INTO sales_returns(id,company_id,fiscal_year_id,number,return_date,\
             original_invoice_id,contact_id,warehouse_id,status,total,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                return_id,
                COMPANY,
                FISCAL_YEAR,
                first_number + index as i64,
                demo_date(index + 60),
                invoice_id,
                contact,
                warehouses[index % warehouses.len()],
                status,
                total,
                USER
            ],
        )?;
        require_row(tx, "sales_returns", &return_id)?;
        tx.execute(
            "INSERT OR IGNORE INTO sales_return_lines(id,return_id,product_id,quantity,\
             unit_price,line_total) VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{return_id}-l0"),
                return_id,
                product,
                returned,
                price,
                total
            ],
        )?;
    }
    Ok(())
}

/// حواله‌های انتقال بین انبارها.
///
/// انتقال هیچ اثر مالی ندارد — فقط جای کالا عوض می‌شود. حواله‌های «در راه»
/// موجودی مبدأ را کم کرده‌اند ولی هنوز به مقصد اضافه نشده‌اند؛ همان وضعیتی
/// که در دنیای واقعی بین بارگیری و تحویل وجود دارد.
fn seed_transfers(tx: &Connection, warehouses: &[String]) -> Result<()> {
    if warehouses.len() < 2 {
        return Ok(());
    }
    for index in 0..10usize {
        let product = format!("demo-prod-{:03}", (index * 6) % PRODUCT_COUNT);
        let from = &warehouses[index % warehouses.len()];
        let to = &warehouses[(index + 1) % warehouses.len()];
        if from == to {
            continue;
        }
        let unit_cost: i64 = tx
            .query_row(
                "SELECT purchase_price FROM products WHERE id=?1",
                params![product],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let quantity = (index % 6 + 2) as f64;

        // فقط تا سقف موجودی مبدأ منتقل می‌شود؛ انتقال بیش از موجودی یعنی
        // کالایی که وجود ندارد جابه‌جا شده است.
        let available: f64 = tx
            .query_row(
                "SELECT COALESCE(quantity,0) FROM inventory_balances \
                 WHERE product_id=?1 AND warehouse_id=?2",
                params![product, from],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        if available < quantity {
            continue;
        }

        let status = if index % 3 == 0 {
            "in_transit"
        } else {
            "received"
        };
        tx.execute(
            "INSERT OR IGNORE INTO inventory_transfer_orders(id,company_id,product_id,\
             from_warehouse_id,to_warehouse_id,quantity,unit_cost,status,note,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                format!("demo-transfer-{index:03}"),
                COMPANY,
                product,
                from,
                to,
                quantity,
                unit_cost,
                status,
                "تأمین موجودی شعبه",
                USER
            ],
        )?;

        // موجودی مبدأ در هر دو حالت کم می‌شود؛ مقصد فقط پس از تحویل زیاد.
        tx.execute(
            "UPDATE inventory_balances SET quantity=quantity-?1 \
             WHERE product_id=?2 AND warehouse_id=?3",
            params![quantity, product, from],
        )?;
        if status == "in_transit" {
            tx.execute(
                "UPDATE inventory_balances SET in_transit_quantity=in_transit_quantity+?1 \
                 WHERE product_id=?2 AND warehouse_id=?3",
                params![quantity, product, from],
            )?;
        } else {
            tx.execute(
                "INSERT INTO inventory_balances(product_id,warehouse_id,quantity) \
                 VALUES(?1,?2,?3) ON CONFLICT(product_id,warehouse_id) \
                 DO UPDATE SET quantity=quantity+excluded.quantity",
                params![product, to, quantity],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
                 movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
                 VALUES(?1,?2,?3,?4,'transfer_out',?5,?6,'transfer',?7,'انتقال بین انبار',?8)",
                params![
                    format!("demo-transfer-{index:03}-out"),
                    COMPANY,
                    product,
                    from,
                    quantity,
                    unit_cost,
                    format!("demo-transfer-{index:03}"),
                    USER
                ],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO inventory_movements(id,company_id,product_id,warehouse_id,\
                 movement_type,quantity,unit_cost,reference_type,reference_id,note,created_by) \
                 VALUES(?1,?2,?3,?4,'transfer_in',?5,?6,'transfer',?7,'انتقال بین انبار',?8)",
                params![
                    format!("demo-transfer-{index:03}-in"),
                    COMPANY,
                    product,
                    to,
                    quantity,
                    unit_cost,
                    format!("demo-transfer-{index:03}"),
                    USER
                ],
            )?;
        }
    }
    Ok(())
}

/// پیش‌فاکتور فروش و سفارش خرید نمونه.
///
/// نکته‌ی حسابداری: این اسناد **هیچ سند مالی و هیچ گردش انباری** نمی‌سازند؛
/// فقط تعهد ثبت می‌کنند. به همین دلیل اینجا هیچ درجی در `journal_entries` یا
/// `inventory_movements` انجام نمی‌شود.
fn seed_quotes(tx: &Connection, warehouses: &[String]) -> Result<()> {
    let statuses = ["draft", "sent", "accepted", "rejected", "converted"];
    for index in 0..18usize {
        let sales = index % 3 != 2;
        let kind = if sales {
            "sales_quote"
        } else {
            "purchase_order"
        };
        let quote_id = format!("demo-quote-{index:03}");
        let contact = format!("demo-contact-{:03}", (index * 5) % CONTACT_COUNT);
        let status = statuses[index % statuses.len()];

        // سه قلم واقعی از کاتالوگ
        let mut subtotal = 0i64;
        let mut discount = 0i64;
        let mut items = Vec::with_capacity(3);
        for offset in 0..3usize {
            let product = format!("demo-prod-{:03}", (index * 3 + offset) % PRODUCT_COUNT);
            let price: i64 = tx
                .query_row(
                    "SELECT sale_price FROM products WHERE id=?1",
                    params![product],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let quantity = (offset + 2) as f64;
            let gross = (price as f64 * quantity).round() as i64;
            let line_discount = if offset == 1 { gross / 20 } else { 0 };
            subtotal += gross;
            discount += line_discount;
            items.push((product, quantity, price, line_discount, gross));
        }
        let net = subtotal - discount;
        let tax = net * 900 / 10_000;
        let total = net + tax;

        let number = tx
            .query_row(
                "SELECT COALESCE(MAX(number),0)+1 FROM quotes \
                 WHERE company_id=?1 AND fiscal_year_id=?2 AND kind=?3",
                params![COMPANY, FISCAL_YEAR, kind],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(1);

        tx.execute(
            "INSERT OR IGNORE INTO quotes(id,company_id,fiscal_year_id,kind,number,issue_date,\
             valid_until,contact_id,warehouse_id,description,subtotal,discount,tax,total,status,\
             converted_invoice_id,created_by) \
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                quote_id,
                COMPANY,
                FISCAL_YEAR,
                kind,
                number,
                demo_date(index + 2),
                demo_date(index + 32),
                contact,
                warehouses[index % warehouses.len()],
                if sales {
                    "پیشنهاد قیمت"
                } else {
                    "سفارش تأمین موجودی"
                },
                subtotal,
                discount,
                tax,
                total,
                status,
                // فقط سند تبدیل‌شده شناسه‌ی فاکتور دارد.
                if status == "converted" {
                    Some(format!("demo-sale-{:03}", index % SALES_INVOICE_COUNT))
                } else {
                    None
                },
                USER
            ],
        )?;
        require_row(tx, "quotes", &quote_id)?;

        for (offset, (product, quantity, price, line_discount, gross)) in
            items.into_iter().enumerate()
        {
            let line_net = gross - line_discount;
            let line_tax = line_net * 900 / 10_000;
            tx.execute(
                "INSERT OR IGNORE INTO quote_lines(id,quote_id,product_id,quantity,unit_price,\
                 discount,tax,line_total) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    format!("{quote_id}-l{offset}"),
                    quote_id,
                    product,
                    quantity,
                    price,
                    line_discount,
                    line_tax,
                    line_net + line_tax
                ],
            )?;
        }
    }
    Ok(())
}
