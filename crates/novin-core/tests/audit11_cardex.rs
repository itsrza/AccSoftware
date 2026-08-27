//! ممیزی دور ۱۱ — کاردکس کالا (F4 فروش / F5 خرید / F6 کلی).
//!
//! مرجع: لیست کالاهای نرم‌افزار فعلی (تصویر `8Xmc1p`) و `docs/FEATURE_BASELINE.md`
//! بخش ۳. کاردکس «گزارش حرکات کالا» است: تاریخ، سند، انبار، ورود، خروج،
//! بهای واحد، ارزش و ماند.
//!
//! ## چه چیزی اینجا سنجیده می‌شود
//!
//! ۱. **جهت حرکت‌ها** — حتی تعدیل انبارگردانی که علامتش در یادداشت است.
//! ۲. **تفکیک فروش/خرید** — از join به چهار جدول سند، با پشتیبانی از
//!    نوع‌های قدیمی seed و دمو.
//! ۳. **ماند درست** — افتتاحیه‌ی قبل از بازه + جمع تجمعی؛ چون کاردکس یک
//!    دفتر است، نه عکس فوری.
//!
//! سناریوی این پرونده قبل از نوشتن، در SQLite مستقل بازپخش شده و همه‌ی
//! اعداد ادعاشده همان‌ها هستند.

use chrono::NaiveDate;
use novin_core::cardex::{
    cardex, channel_of, movement_flow, signed_quantity, CardexError, CardexFilter, CardexKind,
    Channel, DocLinks, Flow,
};
use novin_core::db;
use rusqlite::{params, Connection};

fn seeded() -> Connection {
    db::open_in_memory().expect("پایگاه داده")
}

fn company(conn: &Connection) -> String {
    conn.query_row("SELECT id FROM companies ORDER BY id LIMIT 1", [], |row| {
        row.get(0)
    })
    .expect("شرکت پایه")
}

fn fiscal(conn: &Connection) -> String {
    conn.query_row(
        "SELECT id FROM fiscal_years ORDER BY id LIMIT 1",
        [],
        |row| row.get(0),
    )
    .expect("سال مالی پایه")
}

fn date_of(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("تاریخ معتبر")
}

/// سناریوی کامل: افتتاحیه، فاکتور فروش/خرید، برگشت‌ها، تعدیل، انتقال و نوع قدیمی.
fn scenario() -> Connection {
    let conn = seeded();
    let firm = company(&conn);
    let fiscal = fiscal(&conn);

    conn.execute(
        "INSERT INTO warehouses(id, company_id, name, code) VALUES('audit11-wh-b',?1,'شعبه','99')",
        params![firm],
    )
    .expect("انبار دوم");
    conn.execute(
        "INSERT INTO products(id, company_id, kind, sku, name, unit) \
         VALUES('audit11-p1',?1,'simple','T1','کالای کاردکس','عدد')",
        params![firm],
    )
    .expect("کالای سناریو");
    conn.execute(
        "INSERT INTO sales_invoices(id, company_id, fiscal_year_id, number, invoice_date, status) \
         VALUES('audit11-inv-s',?1,?2,90007,'2025-09-01','posted')",
        params![firm, fiscal],
    )
    .expect("فاکتور فروش");
    conn.execute(
        "INSERT INTO purchase_invoices(id, company_id, fiscal_year_id, number, invoice_date, status) \
         VALUES('audit11-inv-p',?1,?2,90012,'2025-09-02','posted')",
        params![firm, fiscal],
    )
    .expect("فاکتور خرید");
    conn.execute(
        "INSERT INTO sales_returns(id, company_id, fiscal_year_id, number, return_date, original_invoice_id, status) \
         VALUES('audit11-ret-s',?1,?2,90002,'2025-09-05','audit11-inv-s','posted')",
        params![firm, fiscal],
    )
    .expect("برگشت از فروش");
    conn.execute(
        "INSERT INTO purchase_returns(id, company_id, fiscal_year_id, number, return_date, original_invoice_id, status) \
         VALUES('audit11-ret-p',?1,?2,90003,'2025-09-06','audit11-inv-p','posted')",
        params![firm, fiscal],
    )
    .expect("برگشت از خرید");

    /// یک حرکت سناریو — ساختار به‌جای تاپل ۸عضوی (type_complexity).
    struct Mov<'a> {
        id: &'a str,
        movement_type: &'a str,
        quantity: f64,
        unit_cost: i64,
        reference_type: &'a str,
        reference_id: &'a str,
        note: &'a str,
        created: &'a str,
    }

    let movements = [
        Mov {
            id: "m1",
            movement_type: "receipt",
            quantity: 10.0,
            unit_cost: 1_000_000,
            reference_type: "opening",
            reference_id: "",
            note: "موجودی اول دوره",
            created: "2025-08-20 09:00",
        },
        Mov {
            id: "m2",
            movement_type: "issue",
            quantity: 3.0,
            unit_cost: 0,
            reference_type: "invoice",
            reference_id: "audit11-inv-s",
            note: "فروش",
            created: "2025-09-01 10:00",
        },
        Mov {
            id: "m3",
            movement_type: "receipt",
            quantity: 5.0,
            unit_cost: 1_000_000,
            reference_type: "invoice",
            reference_id: "audit11-inv-p",
            note: "خرید",
            created: "2025-09-02 10:00",
        },
        Mov {
            id: "m4",
            movement_type: "receipt",
            quantity: 1.0,
            unit_cost: 0,
            reference_type: "invoice_return",
            reference_id: "audit11-ret-s",
            note: "برگشت از فروش",
            created: "2025-09-05 10:00",
        },
        Mov {
            id: "m5",
            movement_type: "issue",
            quantity: 2.0,
            unit_cost: 1_000_000,
            reference_type: "invoice_return",
            reference_id: "audit11-ret-p",
            note: "برگشت از خرید",
            created: "2025-09-06 10:00",
        },
        Mov {
            id: "m6",
            movement_type: "adjustment",
            quantity: 4.0,
            unit_cost: 0,
            reference_type: "inventory_count",
            reference_id: "sess",
            note: "variance:-4",
            created: "2025-09-07 10:00",
        },
        Mov {
            id: "m8",
            movement_type: "transfer_out",
            quantity: 2.0,
            unit_cost: 0,
            reference_type: "transfer",
            reference_id: "tr-1",
            note: "",
            created: "2025-09-09 10:00",
        },
        Mov {
            id: "m9",
            movement_type: "issue",
            quantity: 1.0,
            unit_cost: 0,
            reference_type: "sales_invoice",
            reference_id: "audit11-inv-s",
            note: "نوع قدیمی",
            created: "2025-09-10 10:00",
        },
    ];

    for movement in &movements {
        let (id, movement_type, quantity, unit_cost, reference_type, reference_id, note, created) = (
            movement.id,
            movement.movement_type,
            movement.quantity,
            movement.unit_cost,
            movement.reference_type,
            movement.reference_id,
            movement.note,
            movement.created,
        );
        conn.execute(
            "INSERT INTO inventory_movements(id, company_id, product_id, warehouse_id, \
             movement_type, quantity, unit_cost, reference_type, reference_id, note, created_at) \
             VALUES(?1,'company-demo','audit11-p1','wh-main',?2,?3,?4,?5,?6,?7,?8)",
            params![
                format!("audit11-{id}"),
                movement_type,
                quantity,
                unit_cost,
                reference_type,
                reference_id,
                note,
                created
            ],
        )
        .expect("حرکت سناریو");
    }
    // حرکت انبار دیگر — برای فیلتر انبار
    conn.execute(
        "INSERT INTO inventory_movements(id, company_id, product_id, warehouse_id, \
         movement_type, quantity, unit_cost, reference_type, note, created_at) \
         VALUES('audit11-m7','company-demo','audit11-p1','audit11-wh-b','receipt',6,500000,'opening','','2025-09-08 10:00')",
        [],
    )
    .expect("حرکت انبار شعبه");

    conn
}

fn filter_for(conn: &Connection, kind: CardexKind, from: NaiveDate, to: NaiveDate) -> CardexFilter {
    CardexFilter {
        company_id: company(conn),
        product_id: "audit11-p1".into(),
        kind,
        from,
        to,
        warehouse_id: None,
    }
}

// ---------------------------------------------------------------------------
// قواعد خالص
// ---------------------------------------------------------------------------

/// ک۳۹ — سه کانال مرجع و رد نوع نامعتبر.
#[test]
fn k39_kind_parsing() {
    assert_eq!(CardexKind::parse("sales"), Ok(CardexKind::Sales));
    assert_eq!(CardexKind::parse("purchase"), Ok(CardexKind::Purchase));
    assert_eq!(CardexKind::parse("all"), Ok(CardexKind::All));
    assert_eq!(CardexKind::parse("vip"), Err(CardexError::UnknownKind));
}

/// ک۴۰ — جهت حرکت، از جمله علامت تعدیل انبارگردانی در یادداشت.
#[test]
fn k40_movement_flow_and_sign() {
    assert_eq!(movement_flow("receipt", None), Flow::In);
    assert_eq!(movement_flow("transfer_in", None), Flow::In);
    assert_eq!(movement_flow("issue", None), Flow::Out);
    assert_eq!(movement_flow("transfer_out", None), Flow::Out);
    // تعدیل: علامت فقط در یادداشت است
    assert_eq!(movement_flow("adjustment", Some("variance:-4")), Flow::Out);
    assert_eq!(movement_flow("adjustment", Some("variance:5")), Flow::In);
    assert_eq!(movement_flow("adjustment", None), Flow::In);

    assert_eq!(signed_quantity(3.0, "issue", None), -3.0);
    assert_eq!(signed_quantity(3.0, "receipt", None), 3.0);
    assert_eq!(
        signed_quantity(4.0, "adjustment", Some("variance:-4")),
        -4.0
    );
}

/// ک۴۱ — تفکیک کانال فروش/خرید از جدول مقصد سند.
#[test]
fn k41_channel_classification() {
    let sales = DocLinks {
        sales_invoice: Some(7),
        ..Default::default()
    };
    let purchase = DocLinks {
        purchase_invoice: Some(12),
        ..Default::default()
    };
    let sales_return = DocLinks {
        sales_return: Some(2),
        ..Default::default()
    };
    let purchase_return = DocLinks {
        purchase_return: Some(3),
        ..Default::default()
    };
    assert_eq!(channel_of(Some("invoice"), &sales), Channel::Sales);
    assert_eq!(channel_of(Some("invoice"), &purchase), Channel::Purchase);
    // سند گم‌شده → داخلی، نه این‌که حدس بزند
    assert_eq!(
        channel_of(Some("invoice"), &DocLinks::default()),
        Channel::Internal
    );
    assert_eq!(
        channel_of(Some("invoice_return"), &sales_return),
        Channel::Sales
    );
    assert_eq!(
        channel_of(Some("invoice_return"), &purchase_return),
        Channel::Purchase
    );
    // نوع‌های قدیمی seed و دمو
    assert_eq!(
        channel_of(Some("sales_invoice"), &DocLinks::default()),
        Channel::Sales
    );
    assert_eq!(
        channel_of(Some("purchase_invoice"), &DocLinks::default()),
        Channel::Purchase
    );
    assert_eq!(
        channel_of(Some("opening"), &DocLinks::default()),
        Channel::Internal
    );
    assert_eq!(
        channel_of(Some("transfer"), &DocLinks::default()),
        Channel::Internal
    );
    assert_eq!(channel_of(None, &DocLinks::default()), Channel::Internal);
}

// ---------------------------------------------------------------------------
// گزارش کامل روی سناریو
// ---------------------------------------------------------------------------

/// ک۴۲ — کاردکس کلی: افتتاحیه، ورود/خروج، ماند سطری و بستن — با اعداد بازپخش‌شده.
#[test]
fn k42_all_cardex_report() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::All,
            date_of(2025, 9, 1),
            date_of(2025, 9, 30),
        ),
    )
    .expect("گزارش");

    assert_eq!(report.product_name, "کالای کاردکس");
    assert_eq!(report.product_unit, "عدد");
    assert_eq!(report.kind, "all");
    assert_eq!(report.opening_balance, 10.0, "افتتاحیه = رسید قبل از بازه");
    assert_eq!(report.total_in, 12.0, "۵ خرید + ۱ برگشت فروش + ۶ شعبه");
    assert_eq!(
        report.total_out, 12.0,
        "۳ فروش + ۲ برگشت خرید + ۴ تعدیل + ۲ انتقال + ۱ قدیمی"
    );
    assert_eq!(report.closing_balance, 10.0);
    assert_eq!(report.entries.len(), 8);

    // ماند سطری: 7 → 12 → 13 → 11 → 7 → 13 → 11 → 10
    let balances: Vec<f64> = report.entries.iter().map(|entry| entry.balance).collect();
    assert_eq!(
        balances,
        vec![7.0, 12.0, 13.0, 11.0, 7.0, 13.0, 11.0, 10.0],
        "ماند هر سطر = افتتاحیه + تجمعی تا آن‌جا"
    );
    // اولین سطر: فروش ۳ عددی با سند شماره ۷
    assert_eq!(report.entries[0].flow, "out");
    assert_eq!(report.entries[0].quantity, 3.0);
    assert_eq!(report.entries[0].doc_kind, "sales_invoice");
    assert_eq!(report.entries[0].doc_number, Some(90_007));
    assert_eq!(
        report.entries[0].date_jalali, "1404/06/10",
        "2025-09-01 شمسی"
    );
}

/// ک۴۳ — کاردکس فروش: فقط فاکتور فروش، برگشت از فروش و نوع قدیمی.
#[test]
fn k43_sales_cardex_report() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::Sales,
            date_of(2025, 9, 1),
            date_of(2025, 9, 30),
        ),
    )
    .expect("گزارش");

    let docs: Vec<&str> = report
        .entries
        .iter()
        .map(|entry| entry.doc_kind.as_str())
        .collect();
    assert_eq!(docs, vec!["sales_invoice", "sales_return", "sales_invoice"]);
    let flows: Vec<&str> = report.entries.iter().map(|entry| entry.flow).collect();
    assert_eq!(
        flows,
        vec!["out", "in", "out"],
        "برگشت از فروش به انبار برمی‌گردد"
    );
    assert_eq!(report.total_in, 1.0);
    assert_eq!(report.total_out, 4.0);
    assert_eq!(
        report.closing_balance, -3.0,
        "خالص فروش مثبت = ماند منفی انبار"
    );
}

/// ک۴۴ — کاردکس خرید: فاکتور خرید + برگشت از خرید با شماره سند.
#[test]
fn k44_purchase_cardex_report() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::Purchase,
            date_of(2025, 9, 1),
            date_of(2025, 9, 30),
        ),
    )
    .expect("گزارش");

    assert_eq!(
        report.entries.len(),
        2,
        "کانال خرید: {:?}",
        report
            .entries
            .iter()
            .map(|entry| (
                entry.date_jalali.clone(),
                entry.doc_kind.clone(),
                entry.doc_number
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        report.entries[0].doc_kind,
        "purchase_invoice",
        "سطرها: {:?}",
        report
            .entries
            .iter()
            .map(|entry| (
                entry.date_jalali.clone(),
                entry.doc_kind.clone(),
                entry.doc_number
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(report.entries[0].doc_number, Some(90_012));
    assert_eq!(report.entries[0].value, 5_000_000, "۵ عدد × ۱٬۰۰۰٬۰۰۰ ریال");
    assert_eq!(report.entries[1].doc_kind, "purchase_return");
    assert_eq!(report.entries[1].doc_number, Some(90_003));
    assert_eq!(report.entries[1].flow, "out");
    assert_eq!(report.closing_balance, 3.0, "۵ خرید − ۲ برگشت");
}

/// ک۴۵ — فیلتر انبار: فقط حرکات همان انبار و ماند مستقل از بقیه.
#[test]
fn k45_warehouse_filter() {
    let conn = scenario();
    let mut filter = filter_for(
        &conn,
        CardexKind::All,
        date_of(2025, 9, 1),
        date_of(2025, 9, 30),
    );
    filter.warehouse_id = Some("wh-main".into());
    let report = cardex(&conn, &filter).expect("گزارش");

    assert_eq!(report.entries.len(), 7, "حرکت شعبه حذف می‌شود");
    assert!(
        report
            .entries
            .iter()
            .all(|entry| entry.warehouse_name == "انبار مرکزی"),
        "هیچ ردیف انبار دیگر نباشد"
    );
    assert_eq!(
        report.closing_balance, 4.0,
        "۱۰ + ۵ + ۱ − ۳ − ۲ − ۴ − ۲ − ۱ = ۴"
    );
}

/// ک۴۶ — جابه‌جایی بازه: ماند قبلی تبدیل به افتتاحیه می‌شود و سطری نشان داده نمی‌شود.
#[test]
fn k46_opening_balance_when_range_moves() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::All,
            date_of(2025, 9, 8),
            date_of(2025, 9, 30),
        ),
    )
    .expect("گزارش");

    assert_eq!(
        report.opening_balance, 7.0,
        "جمع علامت‌دار همه‌ی حرکات قبل از ۱۴۰۴/۰۶/۱۷"
    );
    assert_eq!(report.entries.len(), 3, "فروشِ قدیمی، انتقال و شعبه");
    assert_eq!(report.closing_balance, 10.0, "افتتاحیه + ۶ − ۲ − ۱");
}

/// ک۴۷ — مقدار هر سطر = مقدار × بهای واحد؛ فروش بدون بها یعنی ارزش صفر نه خطا.
#[test]
fn k47_value_lines() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::All,
            date_of(2025, 9, 1),
            date_of(2025, 9, 30),
        ),
    )
    .expect("گزارش");

    let purchase = &report.entries[1];
    assert_eq!(purchase.unit_cost, 1_000_000);
    assert_eq!(purchase.value, 5_000_000);
    let sale = &report.entries[0];
    assert_eq!(sale.unit_cost, 0);
    assert_eq!(sale.value, 0, "خروج فروش بهای تمام‌شده صفر دارد");
}

/// ک۴۸ — تعدیل منفی انبارگردانی باید ماند را کم کند، نه زیاد.
#[test]
fn k48_negative_stocktaking_variance() {
    let conn = scenario();
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::All,
            date_of(2025, 9, 7),
            date_of(2025, 9, 7),
        ),
    )
    .expect("گزارش");

    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].flow, "out");
    assert_eq!(report.entries[0].quantity, 4.0);
    // افتتاحیه = جمع حرکات قبل از ۰۹-۰۷: ۱۰−۳+۵+۱−۲ = ۱۱؛ بستن = ۱۱−۴ = ۷
    assert_eq!(report.opening_balance, 11.0);
    assert_eq!(report.closing_balance, 7.0);
}

// ---------------------------------------------------------------------------
// داده‌ی دمو و خطاها
// ---------------------------------------------------------------------------

/// ک۴۹ — روی داده‌ی واقعی دمو: دو حرکت prod-1 با جمع و مانده‌ی سازگار.
#[test]
fn k49_demo_data_consistency() {
    let conn = seeded();
    let firm = company(&conn);
    let today = chrono::Utc::now().date_naive();
    let yesterday = novin_core::jalali::add_days(today, -1).expect("دیروز");
    let tomorrow = novin_core::jalali::add_days(today, 1).expect("فردا");
    let filter = CardexFilter {
        company_id: firm,
        product_id: "prod-1".into(),
        kind: CardexKind::All,
        from: yesterday,
        to: tomorrow,
        warehouse_id: None,
    };
    let report = cardex(&conn, &filter).expect("گزارش prod-1");
    assert!(
        report.entries.len() >= 2,
        "داده‌ی پایه باید رسید ۲۴ و فروش ۲ داشته باشد"
    );
    assert!(
        report.total_in >= 24.0,
        "رسید اولیه‌ی prod-1 باید دیده شود: in={}",
        report.total_in
    );
    assert_eq!(
        report.closing_balance,
        report.opening_balance + report.total_in - report.total_out
    );
}

/// ک۵۰ — کالای ناموجود یا خالی → خطای صریح، نه گزارش خالی‌ی گمراه‌کننده.
#[test]
fn k50_missing_product_errors() {
    let conn = scenario();
    let mut filter = filter_for(
        &conn,
        CardexKind::All,
        date_of(2025, 9, 1),
        date_of(2025, 9, 30),
    );
    filter.product_id = "   ".into();
    assert_eq!(
        cardex(&conn, &filter),
        Err(CardexError::MissingProduct),
        "شناسه‌ی خالی"
    );

    let mut filter = filter_for(
        &conn,
        CardexKind::All,
        date_of(2025, 9, 1),
        date_of(2025, 9, 30),
    );
    filter.product_id = "ghost-404".into();
    assert_eq!(
        cardex(&conn, &filter),
        Err(CardexError::MissingProduct),
        "شناسه‌ی ناموجود"
    );
}

/// ک۵۱ — بازه‌ی وارونه رد می‌شود.
#[test]
fn k51_invalid_range_rejected() {
    let conn = scenario();
    let filter = filter_for(
        &conn,
        CardexKind::All,
        date_of(2025, 9, 30),
        date_of(2025, 9, 1),
    );
    assert_eq!(cardex(&conn, &filter), Err(CardexError::InvalidRange));
}

/// ک۵۲ — ترتیب هم‌تاریخ‌ها پایدار است (rowid) و ماند سطری پیوسته می‌ماند.
#[test]
fn k52_same_date_ordering_is_stable() {
    let conn = scenario();
    // دو حرکت هم‌تاریخ اضافه کن: اول رسید بعد خروج
    for (id, movement_type, quantity) in [("mx", "receipt", 100.0), ("my", "issue", 1.0)] {
        conn.execute(
            "INSERT INTO inventory_movements(id, company_id, product_id, warehouse_id, \
             movement_type, quantity, unit_cost, reference_type, created_at) \
             VALUES(?1,'company-demo','audit11-p1','wh-main',?2,?3,0,'opening','2025-09-15 08:00')",
            params![format!("audit11-{id}"), movement_type, quantity],
        )
        .expect("حرکت هم‌تاریخ");
    }
    let report = cardex(
        &conn,
        &filter_for(
            &conn,
            CardexKind::All,
            date_of(2025, 9, 15),
            date_of(2025, 9, 15),
        ),
    )
    .expect("گزارش");
    // افتتاحیه = جمع علامت‌دار همه‌ی حرکات قبلی = ۱۰
    assert_eq!(report.opening_balance, 10.0);
    assert_eq!(report.entries.len(), 2);
    assert_eq!(
        report.entries[0].flow, "in",
        "ترتیب درج همان ترتیب خواندن است"
    );
    assert_eq!(report.entries[1].flow, "out");
    assert_eq!(report.entries[0].balance, 110.0, "افتتاحیه ۱۰ + رسید ۱۰۰");
    assert_eq!(report.entries[1].balance, 109.0);
}
