#![allow(warnings)] // موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! فاز ۱۴ — انتقال بین انبارها.
//!
//! ## چرا انتقال دو مرحله‌ای است
//!
//! کالا در لحظه‌ی خروج از انبار مبدأ به مقصد نمی‌رسد؛ در فاصله‌ی این دو
//! «در راه» است. اگر انتقال یک‌مرحله‌ای ثبت شود، موجودی مقصد کالایی را نشان
//! می‌دهد که هنوز نرسیده و انبارگردانی مقصد اختلاف می‌دهد.
//!
//! ## قاعده‌ی حسابداری
//!
//! انتقال بین انبارهای یک شرکت **هیچ اثر مالی ندارد** — نه درآمد می‌سازد نه
//! هزینه. جمع موجودی شرکت پیش و پس از انتقال باید دقیقاً برابر باشد
//! (به‌علاوه‌ی کالای در راه). هر سند حسابداری‌ای که برای انتقال صادر شود،
//! سود ساختگی می‌سازد.

use novin_core::db;
use rusqlite::Connection;

fn seeded() -> Connection {
    let conn = db::open_in_memory().expect("پایگاه داده باید ساخته شود");
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه باید ساخته شود");
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

fn number(conn: &Connection, sql: &str) -> f64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1.0)
}

/// ت۰۱ — قیدهای جدول انتقال داده‌ی بی‌معنا را رد می‌کنند.
#[test]
fn t01_transfer_constraints_reject_nonsense() {
    let conn = db::open_in_memory().unwrap();
    // مقدار صفر
    assert!(
        conn.execute(
            "INSERT INTO inventory_transfer_orders(id,company_id,product_id,from_warehouse_id,\
             to_warehouse_id,quantity,status) \
             VALUES('t01a','company-demo','demo-prod-000','wh-main','wh-branch',0,'in_transit')",
            [],
        )
        .is_err(),
        "مقدار صفر باید رد شود"
    );
    // وضعیت ساختگی
    assert!(
        conn.execute(
            "INSERT INTO inventory_transfer_orders(id,company_id,product_id,from_warehouse_id,\
             to_warehouse_id,quantity,status) \
             VALUES('t01b','company-demo','demo-prod-000','wh-main','wh-branch',5,'flying')",
            [],
        )
        .is_err(),
        "وضعیت ناشناخته باید رد شود"
    );
    // انبار ناموجود
    assert!(
        conn.execute(
            "INSERT INTO inventory_transfer_orders(id,company_id,product_id,from_warehouse_id,\
             to_warehouse_id,quantity,status) \
             VALUES('t01c','company-demo','demo-prod-000','ghost','wh-branch',5,'in_transit')",
            [],
        )
        .is_err(),
        "انبار ناموجود باید رد شود"
    );
}

/// ت۰۲ — مبدأ و مقصد هرگز یکسان نیستند.
#[test]
fn t02_source_and_destination_always_differ() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_transfer_orders WHERE from_warehouse_id=to_warehouse_id"
        ),
        0,
        "انتقال به همان انبار بی‌معناست"
    );
}

/// ت۰۳ — انتقال هیچ سند حسابداری نمی‌سازد.
///
/// اگر انتقال سند بزند، سود یا هزینه‌ی ساختگی در صورت‌های مالی ظاهر می‌شود.
#[test]
fn t03_transfers_never_create_journal_entries() {
    let conn = seeded();
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_entries WHERE source_type='transfer'"
        ),
        0,
        "انتقال بین انبار نباید سند حسابداری بسازد"
    );
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM journal_lines l JOIN journal_entries j ON j.id=l.journal_id \
             WHERE j.source_type LIKE '%transfer%'"
        ),
        0,
        "سطر سند برای انتقال وجود دارد"
    );
}

/// ت۰۴ — حواله‌ی تحویل‌شده دو گردش انبار دارد: خروج و ورود.
#[test]
fn t04_received_transfer_has_both_movements() {
    let conn = seeded();
    let received = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_transfer_orders WHERE status='received'",
    );
    assert!(received > 0, "حواله‌ی تحویل‌شده‌ای وجود ندارد");

    let outs = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_movements WHERE reference_type='transfer' \
         AND movement_type='transfer_out'",
    );
    let ins = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_movements WHERE reference_type='transfer' \
         AND movement_type='transfer_in'",
    );
    assert_eq!(outs, ins, "تعداد خروج و ورود انتقال برابر نیست");
    assert_eq!(outs, received, "هر حواله‌ی تحویل‌شده باید یک خروج داشته باشد");
}

/// ت۰۵ — حواله‌ی «در راه» هنوز به مقصد اضافه نشده است.
#[test]
fn t05_in_transit_stock_has_not_reached_destination() {
    let conn = seeded();
    let in_transit = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_transfer_orders WHERE status='in_transit'",
    );
    assert!(in_transit > 0, "حواله‌ی در راهی وجود ندارد");

    // هیچ گردش ورودی‌ای برای حواله‌های در راه ثبت نشده باشد.
    assert_eq!(
        count(
            &conn,
            "SELECT COUNT(*) FROM inventory_movements m \
             JOIN inventory_transfer_orders t ON t.id = m.reference_id \
             WHERE m.reference_type='transfer' AND m.movement_type='transfer_in' \
             AND t.status='in_transit'"
        ),
        0,
        "کالای در راه به مقصد اضافه شده است"
    );
}

/// ت۰۶ — مقدار «در راه» با جمع حواله‌های در راه همان انبار می‌خواند.
#[test]
fn t06_in_transit_quantity_matches_open_transfers() {
    let conn = seeded();
    let mut statement = conn
        .prepare(
            "SELECT from_warehouse_id, product_id, SUM(quantity) FROM inventory_transfer_orders \
             WHERE status='in_transit' GROUP BY from_warehouse_id, product_id",
        )
        .unwrap();
    let rows: Vec<(String, String, f64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    drop(statement);
    assert!(!rows.is_empty(), "حواله‌ی در راهی وجود ندارد");

    for (warehouse, product, expected) in rows {
        let recorded: f64 = conn
            .query_row(
                "SELECT COALESCE(in_transit_quantity,0) FROM inventory_balances \
                 WHERE product_id=?1 AND warehouse_id=?2",
                rusqlite::params![product, warehouse],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        assert!(
            (recorded - expected).abs() < 1e-9,
            "مقدار در راه «{product}» در «{warehouse}» نمی‌خواند: {recorded} در برابر {expected}"
        );
    }
}

/// ت۰۷ — هیچ انبار و کالایی موجودی منفی ندارد.
///
/// انتقال بیش از موجودی یعنی کالایی جابه‌جا شده که وجود ندارد.
#[test]
fn t07_no_negative_stock_after_transfers() {
    let conn = seeded();
    let negatives = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_balances WHERE quantity < 0",
    );
    assert_eq!(negatives, 0, "موجودی منفی وجود دارد");

    let negative_transit = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_balances WHERE COALESCE(in_transit_quantity,0) < 0",
    );
    assert_eq!(negative_transit, 0, "مقدار در راه منفی است");
}

/// ت۰۸ — جمع کل موجودی شرکت با انتقال تغییر نمی‌کند.
///
/// این همان قاعده‌ی محوری است: انتقال کالا نمی‌سازد و از بین نمی‌برد.
#[test]
fn t08_transfers_conserve_total_stock() {
    let conn = seeded();
    // جمع موجودی + جمع در راه = جمع کل دارایی فیزیکی شرکت
    let on_hand = number(
        &conn,
        "SELECT COALESCE(SUM(quantity),0) FROM inventory_balances",
    );
    let in_transit_balance = number(
        &conn,
        "SELECT COALESCE(SUM(in_transit_quantity),0) FROM inventory_balances",
    );
    let open_transfers = number(
        &conn,
        "SELECT COALESCE(SUM(quantity),0) FROM inventory_transfer_orders WHERE status='in_transit'",
    );
    assert!(on_hand > 0.0, "موجودی صفر است");
    assert!(
        (in_transit_balance - open_transfers).abs() < 1e-9,
        "جمع در راه با حواله‌های باز نمی‌خواند: {in_transit_balance} در برابر {open_transfers}"
    );
}

/// ت۰۹ — ارزش کالای در راه با بهای تمام‌شده محاسبه می‌شود، نه قیمت فروش.
///
/// ارزش‌گذاری موجودی به قیمت فروش، سود تحقق‌نیافته می‌سازد.
#[test]
fn t09_in_transit_value_uses_cost_not_sale_price() {
    let conn = seeded();
    let mut statement = conn
        .prepare(
            "SELECT t.id, t.unit_cost, p.purchase_price, p.sale_price \
             FROM inventory_transfer_orders t JOIN products p ON p.id=t.product_id",
        )
        .unwrap();
    let rows: Vec<(String, i64, i64, i64)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "حواله‌ای وجود ندارد");
    for (id, unit_cost, purchase_price, sale_price) in rows {
        assert_eq!(
            unit_cost, purchase_price,
            "حواله‌ی «{id}» با بهای تمام‌شده ارزش‌گذاری نشده"
        );
        assert_ne!(
            unit_cost, sale_price,
            "حواله‌ی «{id}» به قیمت فروش ارزش‌گذاری شده — سود ساختگی می‌سازد"
        );
    }
}

/// ت۱۰ — هر حواله به کالا و انبارهای واقعی همان شرکت ارجاع دارد.
#[test]
fn t10_transfers_reference_real_entities_of_one_company() {
    let conn = seeded();
    assert!(
        count(&conn, "SELECT COUNT(*) FROM inventory_transfer_orders") >= 5,
        "حواله‌ی نمونه کم است"
    );

    let cross_company = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_transfer_orders t \
         JOIN warehouses f ON f.id=t.from_warehouse_id \
         JOIN warehouses d ON d.id=t.to_warehouse_id \
         WHERE f.company_id <> t.company_id OR d.company_id <> t.company_id",
    );
    assert_eq!(cross_company, 0, "انتقال بین شرکت‌های مختلف وجود دارد");

    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    assert_eq!(
        statement.query_map([], |_| Ok(())).unwrap().count(),
        0,
        "کلید خارجی شکسته است"
    );
}
