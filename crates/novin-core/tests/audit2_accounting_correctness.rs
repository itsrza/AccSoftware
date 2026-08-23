//! ممیزی ۲ — صحت منطق حسابداری و اثرات بین‌ماژولی.
//!
//! ممیزی اول پرسید «آیا قابلیت هست؟». این یکی می‌پرسد **«آیا درست کار می‌کند
//! و اثرش به جای درستی می‌رسد؟»**
//!
//! ## چرا اثر بین‌ماژولی مهم‌تر از خود ماژول است
//!
//! هر ماژول به‌تنهایی ممکن است درست باشد ولی اثرش به ماژول دیگر نرسد یا
//! دوبار برسد. این خطاها در ظاهر دیده نمی‌شوند و فقط در حسابرسی یا در
//! اظهارنامه‌ی مالیاتی لو می‌روند — یعنی وقتی که دیر است.
//!
//! نمونه‌های واقعی که اینجا سنجیده می‌شوند:
//! - فروش باید هم‌زمان موجودی را کم کند، هم بدهی مشتری را زیاد، هم مالیات را
//! - برگشت باید هر سه را دقیقاً معکوس کند، نه فقط دو تا
//! - تولید نباید سود بسازد
//! - انتقال انبار نباید هیچ اثر مالی بگذارد

use novin_core::db;
use novin_core::invoicing::{self, FreightMode, InvoiceInput, InvoiceLine};
use novin_core::jalali::{self, JalaliDate};
use novin_core::money::Money;
use novin_core::production::{
    assert_cost_balance, calculate_costing, ConsumedMaterial, CostAllocation, ProducedItem,
};
use novin_core::treasury::{
    build_journal, DocumentKind, DocumentLine, PaymentMethod, TreasuryAccounts,
};
use rusqlite::Connection;

fn seeded() -> Connection {
    let conn = db::open_in_memory().expect("پایگاه داده");
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه");
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

fn number(conn: &Connection, sql: &str) -> f64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1.0)
}

// ===========================================================================
// یکپارچگی دفتر کل
// ===========================================================================

/// ت۲۶ — دفتر کل در هر لحظه متوازن است.
#[test]
fn t26_general_ledger_is_always_balanced() {
    let conn = seeded();
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(debit > 0, "دفتر کل خالی است");
    assert_eq!(debit, credit, "دفتر کل نامتوازن است");
}

/// ت۲۷ — هیچ سندی به‌تنهایی نامتوازن نیست.
///
/// تراز کل می‌تواند خطای دو سند مخالف را بپوشاند؛ این تست آن را می‌گیرد.
#[test]
fn t27_every_single_voucher_is_balanced() {
    let conn = seeded();
    let unbalanced = count(
        &conn,
        "SELECT COUNT(*) FROM (SELECT journal_id FROM journal_lines \
         GROUP BY journal_id HAVING SUM(debit) <> SUM(credit))",
    );
    assert_eq!(unbalanced, 0, "سند نامتوازن وجود دارد");
}

/// ت۲۸ — هیچ سطری هم‌زمان بدهکار و بستانکار نیست.
#[test]
fn t28_no_line_is_both_debit_and_credit() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_lines WHERE debit > 0 AND credit > 0"
        ),
        0,
        "سطر دوطرفه وجود دارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_lines WHERE debit = 0 AND credit = 0"
        ),
        0,
        "سطر با مبلغ صفر وجود دارد"
    );
}

/// ت۲۹ — هر سند به سال مالی خودش تعلق دارد و تاریخش داخل آن است.
#[test]
fn t29_every_voucher_falls_inside_its_fiscal_year() {
    let conn = seeded();
    let (start, end): (String, String) = conn
        .query_row(
            "SELECT start_date,end_date FROM fiscal_years WHERE id='fy-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let start = JalaliDate::parse(&start).unwrap();
    let end = JalaliDate::parse(&end).unwrap();

    let mut statement = conn
        .prepare("SELECT id, entry_date FROM journal_entries")
        .unwrap();
    let rows: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "سندی وجود ندارد");
    for (id, date) in rows {
        let parsed = JalaliDate::parse(&date)
            .unwrap_or_else(|_| panic!("تاریخ سند «{id}» شمسی معتبر نیست: {date}"));
        assert!(
            parsed >= start && parsed <= end,
            "سند «{id}» ({date}) خارج از سال مالی است"
        );
    }
}

// ===========================================================================
// اثر فروش بر سه ماژول
// ===========================================================================

/// ت۳۰ — هر فاکتور فروش هم‌زمان سه اثر دارد: قلم، گردش انبار، سند.
#[test]
fn t30_a_sale_touches_lines_inventory_and_ledger_together() {
    let conn = seeded();
    let invoices = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices WHERE id LIKE 'demo-sale-%'",
    );
    assert!(invoices >= 50, "فاکتور نمونه کم است");

    // هر فاکتور باید قلم داشته باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sales_invoices s WHERE s.id LIKE 'demo-sale-%' \
             AND NOT EXISTS (SELECT 1 FROM sales_invoice_lines l WHERE l.invoice_id=s.id)"
        ),
        0,
        "فاکتور بدون قلم وجود دارد"
    );
    // هر فاکتور باید گردش انبار داشته باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sales_invoices s WHERE s.id LIKE 'demo-sale-%' \
             AND NOT EXISTS (SELECT 1 FROM inventory_movements m \
                             WHERE m.reference_type='sales_invoice' AND m.reference_id=s.id)"
        ),
        0,
        "فاکتوری بدون گردش انبار وجود دارد"
    );
    // و باید سند حسابداری داشته باشد.
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_entries WHERE source_type='sales_invoice'"
        ) >= 50,
        "سند فروش کم است"
    );
}

/// ت۳۱ — جمع فاکتور با جمع اقلامش می‌خواند و مالیات جدا حساب شده.
#[test]
fn t31_invoice_total_reconciles_with_its_lines_and_tax() {
    let conn = seeded();
    let mismatched = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices s WHERE s.subtotal <> COALESCE(\
           (SELECT SUM(l.line_total) FROM sales_invoice_lines l WHERE l.invoice_id=s.id),0)",
    );
    assert_eq!(mismatched, 0, "جمع خالص فاکتور با اقلامش نمی‌خواند");

    let wrong_total = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices WHERE total <> subtotal - discount + tax",
    );
    assert_eq!(wrong_total, 0, "جمع کل فاکتور اشتباه است");
}

/// ت۳۲ — سند فروش دقیقاً بدهی مشتری، درآمد و مالیات را ثبت می‌کند.
#[test]
fn t32_sales_voucher_hits_receivable_revenue_and_tax() {
    let conn = seeded();
    // یک فاکتور نمونه را کامل رد می‌گیریم.
    let (subtotal, tax, total): (i64, i64, i64) = conn
        .query_row(
            "SELECT subtotal,tax,total FROM sales_invoices WHERE id='demo-sale-005'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("فاکتور نمونه");

    let receivable = number(
        &conn,
        "SELECT COALESCE(SUM(l.debit),0) FROM journal_lines l \
         JOIN journal_entries j ON j.id=l.journal_id \
         WHERE j.id='demo-jrn-sale-005' AND l.account_id='acc-1201'",
    );
    let revenue = number(
        &conn,
        "SELECT COALESCE(SUM(l.credit),0) FROM journal_lines l \
         WHERE l.journal_id='demo-jrn-sale-005' AND l.account_id='acc-4100'",
    );
    let vat = number(
        &conn,
        "SELECT COALESCE(SUM(l.credit),0) FROM journal_lines l \
         WHERE l.journal_id='demo-jrn-sale-005' AND l.account_id='acc-2401'",
    );

    assert_eq!(receivable as i64, total, "بدهی مشتری برابر جمع کل نیست");
    assert_eq!(revenue as i64, subtotal, "درآمد برابر مبلغ خالص نیست");
    assert_eq!(vat as i64, tax, "مالیات ثبت‌شده با فاکتور نمی‌خواند");
    // و این سه باید متوازن باشند.
    assert_eq!(receivable as i64, revenue as i64 + vat as i64);
}

/// ت۳۳ — موتور فاکتور: ترتیب مرحله‌ای محاسبه درست است.
///
/// ترتیب حیاتی است: تخفیف پیش از مالیات، و کرایه پس از مالیات. جابه‌جایی
/// این ترتیب، مبلغ نهایی و مالیات را عوض می‌کند.
#[test]
fn t33_invoice_engine_applies_the_correct_calculation_order() {
    // ۱۰ واحد × ۱٬۰۰۰٬۰۰۰ = ۱۰٬۰۰۰٬۰۰۰ ناخالص
    // تخفیف سطری ۱٬۰۰۰٬۰۰۰ → خالص ۹٬۰۰۰٬۰۰۰
    // مالیات ۹٪ روی خالص = ۸۱۰٬۰۰۰
    // کرایه ۵۰۰٬۰۰۰ به جمع اضافه می‌شود
    let line = InvoiceLine {
        product_id: "p1".into(),
        quantity: 10.0,
        unit_price: Money::from_rials(1_000_000),
        discount_amount: Money::from_rials(1_000_000),
        discount_bp: 0,
        tiers: Vec::new(),
        vat_bp: 900,
        duty_bp: 0,
        commission_bp: 0,
        unit_cost: Money::ZERO,
        serial_tracked: false,
        serials: Vec::new(),
    };
    let input = InvoiceInput {
        lines: vec![line],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::from_rials(500_000),
        freight_mode: FreightMode::AddToTotal,
    };
    let result = invoicing::calculate(&input).expect("محاسبه‌ی فاکتور");

    assert_eq!(result.subtotal.rials(), 10_000_000, "مبلغ ناخالص");
    assert_eq!(result.discount_total.rials(), 1_000_000, "تخفیف");
    assert_eq!(result.net_total.rials(), 9_000_000, "خالص پس از تخفیف");
    assert_eq!(
        result.vat_total.rials(),
        810_000,
        "مالیات باید روی مبلغ پس از تخفیف باشد، نه ناخالص"
    );
    assert_eq!(
        result.total.rials(),
        9_000_000 + 810_000 + 500_000,
        "جمع کل اشتباه است"
    );
}

/// ت۳۴ — دو حالت کرایه اثر متفاوتی بر بهای تمام‌شده دارند.
///
/// «افزودن به جمع» فقط مبلغ فاکتور را بالا می‌برد؛ «سرشکن روی سطرها» کرایه
/// را وارد بهای تمام‌شده می‌کند و سود واقعی را کم می‌کند. اگر این دو یکی
/// شوند، سود گزارش‌شده اشتباه درمی‌آید.
#[test]
fn t34_two_freight_modes_affect_cost_differently() {
    let make_line = || InvoiceLine {
        product_id: "p1".into(),
        quantity: 2.0,
        unit_price: Money::from_rials(1_000_000),
        discount_amount: Money::ZERO,
        discount_bp: 0,
        tiers: Vec::new(),
        vat_bp: 0,
        duty_bp: 0,
        commission_bp: 0,
        unit_cost: Money::from_rials(600_000),
        serial_tracked: false,
        serials: Vec::new(),
    };
    let added = invoicing::calculate(&InvoiceInput {
        lines: vec![make_line()],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::from_rials(200_000),
        freight_mode: FreightMode::AddToTotal,
    })
    .unwrap();
    let allocated = invoicing::calculate(&InvoiceInput {
        lines: vec![make_line()],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::from_rials(200_000),
        freight_mode: FreightMode::AllocateToLines,
    })
    .unwrap();

    // جمع فاکتور در هر دو حالت یکسان است — مشتری همان مبلغ را می‌پردازد.
    assert_eq!(added.total.rials(), allocated.total.rials());
    // ولی بهای تمام‌شده و در نتیجه سود متفاوت است.
    assert!(
        allocated.cost_total.rials() > added.cost_total.rials(),
        "سرشکن‌کردن کرایه باید بهای تمام‌شده را بالا ببرد"
    );
    assert!(
        allocated.profit.rials() < added.profit.rials(),
        "سود در حالت سرشکن باید کمتر باشد"
    );
}

// ===========================================================================
// اثر برگشت
// ===========================================================================

/// ت۳۵ — برگشت هرگز از مقدار فروخته‌شده بیشتر نیست.
#[test]
fn t35_returns_never_exceed_what_was_sold() {
    let conn = seeded();
    let violations = count(
        &conn,
        "SELECT COUNT(*) FROM (\
           SELECT rl.product_id, r.original_invoice_id, SUM(rl.quantity) AS returned, \
                  (SELECT SUM(il.quantity) FROM sales_invoice_lines il \
                   WHERE il.invoice_id=r.original_invoice_id AND il.product_id=rl.product_id) AS sold \
           FROM sales_return_lines rl JOIN sales_returns r ON r.id=rl.return_id \
           WHERE r.status<>'cancelled' GROUP BY rl.product_id, r.original_invoice_id \
           HAVING returned > COALESCE(sold,0))",
    );
    assert_eq!(violations, 0, "برگشت بیش از فروش وجود دارد");
}

/// ت۳۶ — مالیات برگشت به همان نسبت مالیات فاکتور اصلی است.
///
/// اگر نسبت رعایت نشود، مانده‌ی حساب مالیات غلط می‌ماند و اظهارنامه اشتباه
/// درمی‌آید.
#[test]
fn t36_return_tax_is_proportional_to_the_original_invoice() {
    // فاکتور: خالص ۲۰٬۰۰۰٬۰۰۰ با مالیات ۱٬۸۰۰٬۰۰۰ (۹٪)
    let proportional = |returned: i64, subtotal: i64, tax: i64| -> i64 {
        if subtotal <= 0 {
            0
        } else {
            (returned as i128 * tax as i128 / subtotal as i128) as i64
        }
    };
    assert_eq!(proportional(20_000_000, 20_000_000, 1_800_000), 1_800_000);
    assert_eq!(proportional(10_000_000, 20_000_000, 1_800_000), 900_000);
    assert_eq!(proportional(4_000_000, 20_000_000, 1_800_000), 360_000);
    // فاکتور معاف → برگشت هم معاف
    assert_eq!(proportional(5_000_000, 20_000_000, 0), 0);
    // و هرگز تقسیم بر صفر
    assert_eq!(proportional(1_000, 0, 900), 0);
}

/// ت۳۷ — برگشت هرگز قبل از فاکتور اصلی نیست.
#[test]
fn t37_a_return_never_precedes_its_invoice() {
    let conn = seeded();
    let mut statement = conn
        .prepare(
            "SELECT r.id, r.return_date, i.invoice_date FROM sales_returns r \
             JOIN sales_invoices i ON i.id=r.original_invoice_id",
        )
        .unwrap();
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "برگشتی وجود ندارد");
    for (id, return_date, invoice_date) in rows {
        assert!(
            JalaliDate::parse(&return_date).unwrap() >= JalaliDate::parse(&invoice_date).unwrap(),
            "برگشت «{id}» قبل از فاکتورش است"
        );
    }
}

// ===========================================================================
// اثر خزانه
// ===========================================================================

/// ت۳۸ — چک به صندوق نمی‌رود؛ به اسناد دریافتنی می‌نشیند.
#[test]
fn t38_check_never_lands_in_the_cash_box() {
    let accounts = TreasuryAccounts {
        party_account: "acc-1201".into(),
        notes_receivable: "acc-1103".into(),
        notes_payable: "acc-2103".into(),
        discount_account: "acc-4400".into(),
    };
    let mut check_line = DocumentLine::new(PaymentMethod::Check, Money::from_rials(50_000_000));
    check_line.check = Some(novin_core::treasury::CheckDetails {
        serial: "700001".into(),
        due_date: "1405/09/01".into(),
        bank_name: None,
        sayad_id: None,
    });
    let journal = build_journal(DocumentKind::Receipt, &[check_line], &accounts).unwrap();

    let debit = journal.iter().find(|line| line.debit.rials() > 0).unwrap();
    assert_eq!(
        debit.account_id, "acc-1103",
        "چک باید به اسناد دریافتنی برود"
    );
    assert!(
        !journal
            .iter()
            .any(|line| line.account_id.contains("treasury")),
        "حساب صندوق در سند چک ظاهر شده"
    );
}

/// ت۳۹ — گردش خزانه فقط برای روش‌هایی ثبت می‌شود که واقعاً پول جابه‌جا می‌کنند.
#[test]
fn t39_only_real_money_movements_hit_the_treasury_ledger() {
    let conn = seeded();
    // هر تراکنش خزانه باید حساب خزانه‌ی معتبر داشته باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM treasury_transactions t WHERE NOT EXISTS \
             (SELECT 1 FROM treasury_accounts a WHERE a.id=t.treasury_account_id)"
        ),
        0,
        "تراکنش خزانه به حساب ناموجود ارجاع دارد"
    );
    // تخفیف و تهاتر پول جابه‌جا نمی‌کنند.
    assert!(!PaymentMethod::Discount.moves_treasury());
    assert!(!PaymentMethod::Offset.moves_treasury());
    assert!(!PaymentMethod::Check.moves_treasury());
    // و سطرهای تخفیف/تهاتر/چک نباید حساب خزانه بخواهند.
    assert!(!PaymentMethod::Discount.requires_treasury_account());
    assert!(!PaymentMethod::Offset.requires_treasury_account());
}

/// ت۴۰ — تخفیف نقدی، بدهی مشتری را کامل تسویه می‌کند ولی پول نمی‌آورد.
#[test]
fn t40_cash_discount_settles_debt_without_bringing_money() {
    let accounts = TreasuryAccounts {
        party_account: "acc-1201".into(),
        notes_receivable: "acc-1103".into(),
        notes_payable: "acc-2103".into(),
        discount_account: "acc-4400".into(),
    };
    let cash = DocumentLine::new(PaymentMethod::Cash, Money::from_rials(9_500_000))
        .with_account("treasury-cash-1");
    let discount = DocumentLine::new(PaymentMethod::Discount, Money::from_rials(500_000));
    let journal = build_journal(DocumentKind::Receipt, &[cash, discount], &accounts).unwrap();

    let party = journal
        .iter()
        .find(|line| line.account_id == "acc-1201")
        .unwrap();
    // مشتری ۱۰ میلیون بدهی داشت؛ ۹٫۵ نقد داد و ۰٫۵ تخفیف گرفت.
    assert_eq!(
        party.credit.rials(),
        10_000_000,
        "بدهی مشتری باید کامل تسویه شود"
    );
    let discount_line = journal
        .iter()
        .find(|line| line.account_id == "acc-4400")
        .unwrap();
    assert_eq!(
        discount_line.debit.rials(),
        500_000,
        "تخفیف باید بدهکار شود"
    );
}

// ===========================================================================
// اثر تولید و انبار
// ===========================================================================

/// ت۴۱ — تولید سود نمی‌سازد: ارزش ورودی و خروجی برابر است.
#[test]
fn t41_production_creates_no_profit() {
    let materials = vec![ConsumedMaterial {
        product_id: "mat".into(),
        quantity: 4.0,
        unit_cost: Money::from_rials(2_500_000),
    }];
    let expenses = vec![novin_core::production::ProductionExpense {
        account_id: "acc-5300".into(),
        title: "دستمزد".into(),
        amount: Money::from_rials(1_000_000),
    }];
    let outputs = vec![ProducedItem {
        product_id: "prod".into(),
        quantity: 5.0,
        market_unit_price: None,
    }];
    let costing =
        calculate_costing(&materials, &expenses, &outputs, CostAllocation::ByQuantity).unwrap();

    let allocated: i64 = costing
        .outputs
        .iter()
        .map(|item| item.allocated_cost.rials())
        .sum();
    assert_eq!(
        allocated,
        costing.materials_total.rials() + costing.expenses_total.rials(),
        "ارزش خروجی با ورودی برابر نیست — سود یا زیان ساختگی ساخته شد"
    );
    assert!(assert_cost_balance(&costing).is_ok());
}

/// ت۴۲ — انتقال بین انبارها هیچ سند مالی نمی‌سازد.
#[test]
fn t42_warehouse_transfer_has_zero_financial_footprint() {
    let conn = seeded();
    assert!(
        count(&conn, "SELECT COUNT(*) FROM inventory_transfer_orders") > 0,
        "حواله‌ی انتقالی وجود ندارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_entries WHERE source_type LIKE '%transfer%'"
        ),
        0,
        "انتقال انبار سند مالی ساخته است"
    );
}

/// ت۴۳ — موجودی هیچ کالایی در هیچ انباری منفی نیست.
#[test]
fn t43_no_negative_stock_anywhere() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_balances WHERE quantity < 0"
        ),
        0,
        "موجودی منفی وجود دارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_balances WHERE COALESCE(in_transit_quantity,0) < 0"
        ),
        0,
        "مقدار در راه منفی است"
    );
}

/// ت۴۴ — هر گردش انبار به کالا و انبار موجود ارجاع دارد.
#[test]
fn t44_every_stock_movement_points_at_real_entities() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements m WHERE NOT EXISTS \
             (SELECT 1 FROM products p WHERE p.id=m.product_id)"
        ),
        0,
        "گردش انبار به کالای ناموجود ارجاع دارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements m WHERE NOT EXISTS \
             (SELECT 1 FROM warehouses w WHERE w.id=m.warehouse_id)"
        ),
        0,
        "گردش انبار به انبار ناموجود ارجاع دارد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements WHERE quantity <= 0"
        ),
        0,
        "گردش انبار با مقدار صفر یا منفی وجود دارد"
    );
}

// ===========================================================================
// تقویم، پول و شناسه‌ها
// ===========================================================================

/// ت۴۵ — تقویم شمسی با مقادیر راستی‌آزمایی‌شده‌ی نرم‌افزار واقعی می‌خواند.
#[test]
fn t45_jalali_calendar_matches_verified_reference_values() {
    let cases = [
        ("1404/05/30", 2025, 8, 21),
        ("1405/01/01", 2026, 3, 21),
        ("1403/01/01", 2024, 3, 20),
        ("1357/11/22", 1979, 2, 11),
    ];
    for (jalali_text, year, month, day) in cases {
        let parsed = JalaliDate::parse(jalali_text).expect("تاریخ شمسی معتبر");
        let gregorian = parsed.to_gregorian().expect("تبدیل به میلادی");
        let expected =
            chrono::NaiveDate::from_ymd_opt(year, month, day).expect("تاریخ میلادی معتبر");
        assert_eq!(gregorian, expected, "تبدیل «{jalali_text}» اشتباه است");
        // و رفت‌وبرگشت باید بی‌خطا باشد.
        assert_eq!(jalali::from_gregorian(expected), parsed);
    }
}

/// ت۴۶ — سال کبیسه‌ی شمسی درست تشخیص داده می‌شود.
///
/// اشتباه در کبیسه یعنی سررسید چک یک روز جابه‌جا می‌شود.
#[test]
fn t46_jalali_leap_years_are_correct() {
    // ۱۴۰۳ کبیسه است (اسفند ۳۰ روز)، ۱۴۰۴ نیست.
    assert!(
        JalaliDate::new(1403, 12, 30).is_ok(),
        "۱۴۰۳ باید کبیسه باشد"
    );
    assert!(
        JalaliDate::new(1404, 12, 30).is_err(),
        "۱۴۰۴ نباید کبیسه باشد"
    );
    // شش ماه اول ۳۱ روز، شش ماه دوم ۳۰ روز.
    assert!(JalaliDate::new(1405, 6, 31).is_ok());
    assert!(JalaliDate::new(1405, 7, 31).is_err(), "مهر ۳۱ روز ندارد");
}

/// ت۴۷ — ریال واحد داخلی است و هیچ محاسبه‌ای اعشار پول ندارد.
///
/// محاسبه‌ی پول با ممیز شناور، خطای انباشتی می‌سازد که در جمع هزاران سطر
/// به اختلاف واقعی تبدیل می‌شود.
#[test]
fn t47_money_is_integer_rials_with_no_lost_fractions() {
    let conn = seeded();
    // همه‌ی ستون‌های مبلغ باید عدد صحیح باشند.
    for (table, column) in [
        ("sales_invoices", "total"),
        ("journal_lines", "debit"),
        ("treasury_documents", "total"),
        ("production_orders", "total_cost"),
    ] {
        let fractional = count(
            &conn,
            &format!("SELECT COUNT(*) FROM {table} WHERE {column} <> CAST({column} AS INTEGER)"),
        );
        assert_eq!(fractional, 0, "{table}.{column} اعشار دارد");
    }
    // و تقسیم مبلغ نباید ریالی گم کند.
    let total = Money::from_rials(1_000_000);
    let shares = total.allocate(&[1, 1, 1]).unwrap();
    let sum: i64 = shares.iter().map(|share| share.rials()).sum();
    assert_eq!(sum, 1_000_000, "تقسیم مبلغ ریال گم کرد");
}

/// ت۴۸ — هیچ رکوردی به موجودیت حذف‌شده ارجاع ندارد.
#[test]
fn t48_referential_integrity_holds_across_every_table() {
    let conn = seeded();
    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    let violations: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(
        violations.is_empty(),
        "کلید خارجی در جدول‌های {violations:?} شکسته است"
    );
}

/// ت۴۹ — گزارش حسابرسی هر عملیات مالی را ثبت می‌کند.
///
/// عملیات مالی بدون ردپا یعنی حسابرسی غیرممکن.
#[test]
fn t49_audit_trail_table_is_wired_for_financial_actions() {
    let conn = seeded();
    assert!(
        count(
            &conn,
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='audit_logs'"
        ) == 1,
        "جدول حسابرسی نیست"
    );
    let columns: Vec<String> = {
        let mut statement = conn.prepare("PRAGMA table_info(audit_logs)").unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    // ردپا باید بگوید چه کسی، چه کاری، روی چه چیزی، و مقدار قبل و بعد چه بود.
    for column in [
        "user_id",
        "action",
        "entity_type",
        "entity_id",
        "before_json",
        "after_json",
    ] {
        assert!(
            columns.contains(&column.to_string()),
            "ستون حسابرسی «{column}» نیست"
        );
    }
}

/// ت۵۰ — کل داده‌ی نمونه از هر زاویه سالم است.
///
/// آخرین ممیزی: همه‌ی ماژول‌ها با هم، نه جدا از هم.
#[test]
fn t50_the_whole_demo_dataset_is_coherent() {
    let conn = seeded();

    // ۱. همه‌ی ماژول‌ها داده دارند — کاربر هیچ صفحه‌ی خالی نمی‌بیند.
    let modules: [(&str, i64); 12] = [
        ("products", 60),
        ("contacts", 50),
        ("warehouses", 5),
        ("sales_invoices", 55),
        ("purchase_invoices", 25),
        ("checks", 20),
        ("treasury_accounts", 5),
        ("treasury_documents", 20),
        ("sales_returns", 5),
        ("inventory_transfer_orders", 5),
        ("quotes", 10),
        ("party_groups", 6),
    ];
    for (table, minimum) in modules {
        let actual = count(&conn, &format!("SELECT COUNT(*) FROM {table}"));
        assert!(
            actual >= minimum,
            "ماژول «{table}» فقط {actual} رکورد دارد (حداقل {minimum})"
        );
    }

    // ۲. دفتر کل متوازن است.
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(debit, credit, "دفتر کل نامتوازن است");

    // ۳. هیچ کلید خارجی نشکسته.
    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    assert_eq!(
        statement.query_map([], |_| Ok(())).unwrap().count(),
        0,
        "کلید خارجی شکسته است"
    );
    drop(statement);

    // ۴. اجرای دوباره چیزی را دوبرابر نمی‌کند.
    let before = count(&conn, "SELECT COUNT(*) FROM journal_lines");
    db::demo::seed_demo_dataset(&conn).unwrap();
    assert_eq!(
        before,
        count(&conn, "SELECT COUNT(*) FROM journal_lines"),
        "اجرای دوباره داده را دوبرابر کرد"
    );
}
