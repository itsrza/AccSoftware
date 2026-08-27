#![allow(warnings)] // موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! ممیزی ۵ — انطباق با **منطق حسابداری ایران**.
//!
//! چهار ممیزی قبلی پرسیدند «آیا قابلیت هست؟»، «آیا محاسبه درست است؟»،
//! «آیا فرم اعتبارسنجی دارد؟» و «آیا قرارداد میزبان کامل است؟».
//!
//! این ممیزی یک پرسش تازه دارد: **«آیا رفتار سیستم با رویه‌ی حسابداری و
//! مقررات مالیاتی ایران می‌خواند؟»** — چیزی که فقط با دانستن قواعد محلی
//! سنجیده می‌شود، نه با درستی ریاضی.
//!
//! هفت خانواده‌ی قاعده که اینجا آزموده می‌شوند:
//!
//! ۱. **مأخذ مالیات بر ارزش افزوده** — ماده ۵ قانون دائمی مالیات بر ارزش
//!    افزوده: مأخذ، بهای کالا **پس از کسر تخفیف** است و عوارض پیش از
//!    ارزش افزوده محاسبه می‌شود.
//! ۲. **واحد پول** — ریال واحد رسمی؛ هیچ محاسبه‌ای نباید کسر ریال بسازد.
//! ۳. **تقویم هجری شمسی** — سال مالی، سررسید چک و اقساط با ماه شمسی
//!    حرکت می‌کنند نه ۳۰ روزه.
//! ۴. **قاعده‌ی تراز دوطرفه** — هر رویداد مالی سند متوازن می‌سازد.
//! ۵. **چرخه‌ی چک ایرانی** — واگذاری، وصول، برگشت، خرج‌کردن (ظهرنویسی)
//!    و چک انتظامی؛ اثر خزانه‌ای هر گذار.
//! ۶. **کدینگ چهارسطحی و تفصیلی شناور** — رویه‌ی استاندارد حسابداری
//!    ایران: گروه/کل/معین/تفصیلی و ثبت فقط روی سطح آخر.
//! ۷. **شناسه‌های هویتی ایران** — کد ملی، شناسه ملی، شبا، کارت بانکی و
//!    کد پستی با الگوریتم رسمی خودشان.

use novin_core::accounting::{
    self, build_reversal, calculate_invoice, purchase_invoice_journal, sales_invoice_journal,
    single_line_entry, validate_journal, InvoiceLineInput, JournalLine, PostingSide,
};
use novin_core::catalog::{PriceLevel, PriceList, ProductKind, TaxProfile, UnitSet};
use novin_core::checks::{
    allowed_transitions, maturity_date, transition, treasury_effect, weighted_maturity, CheckItem,
    CheckKind, CheckStatus, TreasuryEffect,
};
use novin_core::coding::{AccountDefinition, AccountNature, CodingScheme, Dimensions, Subsidiary};
use novin_core::inventory::{consume_fifo, valuate, Movement, MovementKind, ValuationMethod};
use novin_core::invoicing::{
    self, DiscountTier, FreightMode, InvoiceInput, InvoiceLine, SettlementBreakdown,
};
use novin_core::jalali::{self, days_in_jalali_month, is_jalali_leap, JalaliDate};
use novin_core::money::Money;
use novin_core::parties::{
    card_number_is_valid, check_credit_limit, economic_code_is_valid, iban_is_valid,
    legal_id_is_valid, national_id_is_valid, normalize_mobile, postal_code_is_valid,
    remaining_credit, summarize_balances, PartyError,
};
use novin_core::stocktaking::{build_adjustment_journal, summarize, CountLine, VarianceAccounts};
use novin_core::treasury::{
    build_journal, calculate_totals, check_withdrawal, BalanceCheck, DocumentKind, DocumentLine,
    NegativeBalancePolicy, PaymentMethod, TreasuryAccounts,
};

use chrono::NaiveDate;
use novin_core::db;
use rusqlite::Connection;

// ---------------------------------------------------------------------------
// ابزار
// ---------------------------------------------------------------------------

fn rials(value: i64) -> Money {
    Money::from_rials(value)
}

fn seeded() -> Connection {
    let conn = db::open_in_memory().expect("پایگاه داده");
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه");
    conn
}

fn gregorian(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("تاریخ میلادی معتبر")
}

/// یک سطر ساده‌ی فاکتور با نرخ ارزش افزوده‌ی دلخواه.
fn line(quantity: f64, unit_price: i64, vat_bp: i64) -> InvoiceLine {
    let mut item = InvoiceLine::new("p-1", quantity, rials(unit_price));
    item.vat_bp = vat_bp;
    item
}

fn scheme() -> CodingScheme {
    CodingScheme::default()
}

/// یک حساب قابل ثبت (سطح آخر) در طرح پیش‌فرض `[1,2,2,2]`.
fn leaf(code: &str, title: &str, nature: AccountNature) -> AccountDefinition {
    AccountDefinition::new(code, title, nature)
}

// ===========================================================================
// خانواده ۱ — مأخذ مالیات بر ارزش افزوده (۱۰ تست)
// ===========================================================================

/// ح۱ — مالیات روی مبلغ **پس از** تخفیف بسته می‌شود، نه روی ناخالص.
///
/// ماده ۵ قانون دائمی م.ا.ا: مأخذ محاسبه «بهای کالا پس از کسر تخفیفات» است.
/// اگر روی ناخالص محاسبه شود، مؤدی بیش از تکلیف قانونی مالیات می‌پردازد.
#[test]
fn h1_vat_base_is_net_of_discount() {
    let mut item = line(10.0, 1_000_000, 900);
    item.discount_amount = rials(1_000_000);
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![item],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");

    assert_eq!(result.subtotal, rials(10_000_000), "ناخالص");
    assert_eq!(result.net_total, rials(9_000_000), "خالص پس از تخفیف");
    // ۹٪ روی ۹ میلیون = ۸۱۰ هزار، نه ۹۰۰ هزار.
    assert_eq!(
        result.vat_total,
        rials(810_000),
        "مأخذ مالیات باید خالص باشد"
    );
}

/// ح۲ — تخفیف سرجمع هم از مأخذ مالیات کم می‌شود.
#[test]
fn h2_header_discount_also_reduces_vat_base() {
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![line(1.0, 10_000_000, 900)],
        header_discount: rials(2_000_000),
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");
    assert_eq!(result.net_total, rials(8_000_000));
    assert_eq!(result.vat_total, rials(720_000));
}

/// ح۳ — عوارض **پیش از** ارزش افزوده و ارزش افزوده روی (خالص + عوارض).
///
/// در صورتحساب رسمی ایران، عوارض جزو مأخذ ارزش افزوده است.
#[test]
fn h3_duty_is_inside_the_vat_base() {
    let mut item = line(1.0, 100_000_000, 900);
    item.duty_bp = 100; // ۱٪ عوارض
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![item],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");
    assert_eq!(result.duty_total, rials(1_000_000), "عوارض ۱٪");
    // ۹٪ روی ۱۰۱٬۰۰۰٬۰۰۰
    assert_eq!(
        result.vat_total,
        rials(9_090_000),
        "ارزش افزوده روی خالص + عوارض"
    );
    assert_eq!(result.total, rials(110_090_000));
}

/// ح۴ — کالای معاف هیچ مالیاتی نمی‌گیرد، حتی اگر نرخ عمومی تعریف شده باشد.
#[test]
fn h4_exempt_goods_carry_no_tax() {
    let exempt = TaxProfile::exempt();
    assert_eq!(
        exempt.tax_on(rials(50_000_000)).expect("مالیات"),
        Money::ZERO
    );

    let standard = TaxProfile::standard();
    assert_eq!(
        standard.tax_on(rials(50_000_000)).expect("مالیات"),
        rials(4_500_000),
        "نرخ استاندارد ایران ۹٪ است"
    );
}

/// ح۵ — نرخ استاندارد ارزش افزوده در هسته ۹٪ است (۹۰۰ پایه‌نقطه).
#[test]
fn h5_standard_vat_rate_is_nine_percent() {
    assert_eq!(TaxProfile::standard().vat_basis_points, 900);
}

/// ح۶ — کرایه حمل «افزوده به جمع» وارد مأخذ مالیات نمی‌شود.
///
/// وقتی کرایه را شرکت حمل مستقیم از خریدار می‌گیرد، فروشنده واسط است و
/// نباید روی آن ارزش افزوده ببندد.
#[test]
fn h6_freight_added_to_total_is_outside_vat_base() {
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![line(1.0, 10_000_000, 900)],
        header_discount: Money::ZERO,
        coupon: None,
        freight: rials(500_000),
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");
    assert_eq!(result.vat_total, rials(900_000), "کرایه نباید مشمول شود");
    assert_eq!(result.total, rials(11_400_000), "کرایه به جمع اضافه می‌شود");
}

/// ح۷ — کرایه‌ی «سرشکن روی سطرها» وارد مأخذ مالیات می‌شود.
#[test]
fn h7_freight_allocated_to_lines_is_inside_vat_base() {
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![line(1.0, 10_000_000, 900)],
        header_discount: Money::ZERO,
        coupon: None,
        freight: rials(500_000),
        freight_mode: FreightMode::AllocateToLines,
    })
    .expect("محاسبه");
    // ۹٪ روی ۱۰٬۵۰۰٬۰۰۰
    assert_eq!(result.vat_total, rials(945_000));
    assert_eq!(result.total, rials(11_445_000));
    assert_eq!(
        result.freight,
        rials(500_000),
        "کرایه در نتیجه گزارش می‌شود حتی وقتی سرشکن شده"
    );
}

/// ح۸ — انتخاب اشتباه حالت کرایه، مبلغ اظهارنامه را عوض می‌کند.
///
/// این تست اختلاف را عددی می‌کند تا اهمیت انتخاب درست روشن بماند.
#[test]
fn h8_freight_mode_changes_declared_vat() {
    let make = |mode| InvoiceInput {
        lines: vec![line(1.0, 100_000_000, 900)],
        header_discount: Money::ZERO,
        coupon: None,
        freight: rials(10_000_000),
        freight_mode: mode,
    };
    let outside = invoicing::calculate(&make(FreightMode::AddToTotal)).expect("محاسبه");
    let inside = invoicing::calculate(&make(FreightMode::AllocateToLines)).expect("محاسبه");
    assert_eq!(
        inside
            .vat_total
            .checked_sub(outside.vat_total)
            .expect("تفاضل"),
        rials(900_000),
        "۹٪ از کرایه"
    );
}

/// ح۹ — مالیات به حساب درآمد نمی‌رود؛ حساب «مالیات پرداختنی» بستانکار می‌شود.
///
/// مالیات وصول‌شده بدهی مؤدی به سازمان امور مالیاتی است، نه درآمد او.
#[test]
fn h9_collected_vat_is_a_liability_not_revenue() {
    let totals = calculate_invoice(
        &[InvoiceLineInput {
            product_id: "p-1".into(),
            quantity: 2.0,
            unit_price: rials(50_000_000),
            line_discount: Money::ZERO,
            tax_basis_points: 900,
        }],
        Money::ZERO,
    )
    .expect("محاسبه");

    let journal =
        sales_invoice_journal("1301001", "4100001", "2400001", &totals).expect("سند فروش");
    let revenue: i64 = journal
        .iter()
        .filter(|l| l.account_id == "4100001")
        .map(|l| l.credit.rials())
        .sum();
    let tax: i64 = journal
        .iter()
        .filter(|l| l.account_id == "2400001")
        .map(|l| l.credit.rials())
        .sum();
    assert_eq!(revenue, 100_000_000, "درآمد فقط خالص فروش است");
    assert_eq!(tax, 9_000_000, "مالیات جدا و در حساب بدهی");
    validate_journal(&journal).expect("سند فروش متوازن است");
}

/// ح۱۰ — در خرید، مالیات **دارایی** (مالیات دریافتنی) است نه بهای کالا.
///
/// اگر مالیات خرید به بهای موجودی اضافه شود، هم بهای تمام‌شده اشتباه
/// می‌شود و هم اعتبار مالیاتی مؤدی از بین می‌رود.
#[test]
fn h10_input_vat_is_an_asset_not_inventory_cost() {
    let totals = calculate_invoice(
        &[InvoiceLineInput {
            product_id: "p-1".into(),
            quantity: 1.0,
            unit_price: rials(20_000_000),
            line_discount: Money::ZERO,
            tax_basis_points: 900,
        }],
        Money::ZERO,
    )
    .expect("محاسبه");
    let journal =
        purchase_invoice_journal("1105001", "1302001", "2101001", &totals).expect("سند خرید");
    let inventory: i64 = journal
        .iter()
        .filter(|l| l.account_id == "1105001")
        .map(|l| l.debit.rials())
        .sum();
    let vat_asset: i64 = journal
        .iter()
        .filter(|l| l.account_id == "1302001")
        .map(|l| l.debit.rials())
        .sum();
    assert_eq!(inventory, 20_000_000, "بهای موجودی بدون مالیات");
    assert_eq!(vat_asset, 1_800_000, "مالیات خرید دارایی است");
    validate_journal(&journal).expect("سند خرید متوازن است");
}

// ===========================================================================
// خانواده ۲ — ریال، گرد کردن و نبود «ریال گمشده» (۷ تست)
// ===========================================================================

/// ح۱۱ — تخفیف سرجمع بدون گم شدن حتی یک ریال پخش می‌شود.
#[test]
fn h11_header_discount_allocation_loses_no_rial() {
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![
            line(1.0, 3_333_333, 0),
            line(1.0, 3_333_333, 0),
            line(1.0, 3_333_334, 0),
        ],
        header_discount: rials(1_000_000),
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");
    let shares: i64 = result
        .lines
        .iter()
        .map(|l| l.header_discount_share.rials())
        .sum();
    assert_eq!(shares, 1_000_000, "جمع سهم‌ها باید دقیقاً برابر تخفیف باشد");
    assert_eq!(result.discount_total, rials(1_000_000));
}

/// ح۱۲ — اقساط بدون گم شدن ریال تقسیم می‌شوند.
#[test]
fn h12_installments_sum_exactly_to_the_remainder() {
    let plan = invoicing::installment_plan(
        rials(10_000_000),
        rials(1_000_000),
        7,
        gregorian(2025, 8, 21),
    )
    .expect("جدول اقساط");
    assert_eq!(plan.len(), 7);
    let total: i64 = plan.iter().map(|item| item.amount.rials()).sum();
    assert_eq!(
        total, 9_000_000,
        "جمع اقساط باید دقیقاً برابر باقیمانده باشد"
    );
}

/// ح۱۳ — واحد داخلی ریال است و تبدیل تومان دقیقاً ×۱۰.
#[test]
fn h13_toman_is_ten_rials() {
    assert_eq!(novin_core::money::RIALS_PER_TOMAN, 10);
    let amount = Money::from_tomans(1_250_000).expect("تبدیل");
    assert_eq!(amount.rials(), 12_500_000);
    assert_eq!(amount.tomans(), 1_250_000);
}

/// ح۱۴ — جداکننده‌ی هزارگان برای نمایش، بدون تغییر مقدار.
#[test]
fn h14_grouped_format_is_display_only() {
    assert_eq!(rials(12_500_000).format_grouped(), "12,500,000");
    assert_eq!(rials(0).format_grouped(), "0");
    assert_eq!(rials(-4_500).format_grouped(), "-4,500");
}

/// ح۱۵ — ورودی با ارقام فارسی و جداکننده پذیرفته می‌شود.
///
/// کاربر ایرانی مبلغ را «۱۲٬۵۰۰٬۰۰۰» تایپ می‌کند؛ رد کردن آن یعنی
/// وادار کردن او به تغییر صفحه‌کلید در وسط ثبت سند.
#[test]
fn h15_persian_digits_and_separators_are_accepted() {
    assert_eq!(
        Money::parse_rials("۱۲,۵۰۰,۰۰۰").expect("تجزیه"),
        rials(12_500_000)
    );
    assert_eq!(Money::parse_rials("۱۲۵۰").expect("تجزیه"), rials(1_250));
}

/// ح۱۶ — درصد با پایه‌نقطه محاسبه می‌شود تا ۰٫۵٪ هم دقیق باشد.
#[test]
fn h16_basis_points_allow_fractional_percent() {
    assert_eq!(
        rials(100_000_000).percent_bp(50).expect("درصد"),
        rials(500_000)
    );
    assert_eq!(
        rials(100_000_000).percent_bp(900).expect("درصد"),
        rials(9_000_000)
    );
}

/// ح۱۷ — تسویه‌ی چندروشی نمی‌تواند از مبلغ فاکتور بیشتر شود.
#[test]
fn h17_settlement_cannot_exceed_invoice_total() {
    let breakdown = SettlementBreakdown {
        cash: rials(6_000_000),
        check: rials(5_000_000),
        transfer: Money::ZERO,
        card: Money::ZERO,
    };
    assert_eq!(breakdown.total(), rials(11_000_000));
    assert!(
        breakdown.validate(rials(10_000_000)).is_err(),
        "بیش‌تسویه باید رد شود"
    );
    breakdown
        .validate(rials(11_000_000))
        .expect("تسویه‌ی کامل مجاز است");
}

// ===========================================================================
// خانواده ۳ — تقویم هجری شمسی (۷ تست)
// ===========================================================================

/// ح۱۸ — اقساط با **ماه شمسی** جلو می‌روند، نه ۳۰ روزه.
///
/// در ایران قسط «هر ماه» یعنی همان روز از ماه شمسی بعد. شش ماه اول سال
/// ۳۱ روزه است؛ اگر ۳۰ روزه حساب شود سررسیدها به‌مرور جابه‌جا می‌شوند.
#[test]
fn h18_installments_advance_by_jalali_months() {
    let first = JalaliDate::new(1405, 1, 15)
        .expect("تاریخ")
        .to_gregorian()
        .expect("میلادی");
    let plan =
        invoicing::installment_plan(rials(3_000_000), Money::ZERO, 3, first).expect("جدول اقساط");
    assert_eq!(plan[0].due_date_jalali, "1405/01/15");
    assert_eq!(plan[1].due_date_jalali, "1405/02/15");
    assert_eq!(plan[2].due_date_jalali, "1405/03/15");
    // فروردین ۳۱ روزه است، پس فاصله‌ی قسط اول تا دوم ۳۱ روز است نه ۳۰.
    assert_eq!((plan[1].due_date - plan[0].due_date).num_days(), 31);
}

/// ح۱۹ — سررسید ۳۱ اُم در ماه ۳۰ روزه به آخرین روز ماه می‌چسبد.
#[test]
fn h19_month_end_clamps_when_target_month_is_shorter() {
    let start = JalaliDate::new(1405, 6, 31)
        .expect("۳۱ شهریور")
        .to_gregorian()
        .expect("میلادی");
    let next = jalali::add_jalali_months(start, 1).expect("ماه بعد");
    assert_eq!(jalali::jalali_string(next), "1405/07/30", "مهر ۳۰ روزه است");
}

/// ح۲۰ — کبیسه‌ی شمسی درست تشخیص داده می‌شود.
#[test]
fn h20_jalali_leap_years_are_correct() {
    assert!(is_jalali_leap(1403), "۱۴۰۳ کبیسه است");
    assert!(!is_jalali_leap(1404), "۱۴۰۴ کبیسه نیست");
    assert_eq!(days_in_jalali_month(1403, 12), 30);
    assert_eq!(days_in_jalali_month(1404, 12), 29);
    assert!(
        JalaliDate::new(1404, 12, 30).is_err(),
        "۳۰ اسفند ۱۴۰۴ وجود ندارد"
    );
}

/// ح۲۱ — تبدیل شمسی به میلادی با نقاط مرجع تأییدشده می‌خواند.
#[test]
fn h21_calendar_conversion_matches_known_anchors() {
    let cases = [
        ((1404, 5, 30), (2025, 8, 21)),
        ((1405, 1, 1), (2026, 3, 21)),
        ((1403, 1, 1), (2024, 3, 20)),
        ((1357, 11, 22), (1979, 2, 11)),
    ];
    for ((jy, jm, jd), (gy, gm, gd)) in cases {
        let jalali_date = JalaliDate::new(jy, jm, jd).expect("تاریخ شمسی");
        assert_eq!(
            jalali_date.to_gregorian().expect("میلادی"),
            gregorian(gy, gm, gd),
            "تبدیل {jy}/{jm}/{jd}"
        );
        assert_eq!(
            jalali::from_gregorian(gregorian(gy, gm, gd)),
            jalali_date,
            "تبدیل معکوس {gy}-{gm}-{gd}"
        );
    }
}

/// ح۲۲ — سال مالی داده‌ی نمونه از ۱ فروردین تا ۲۹ یا ۳۰ اسفند است.
#[test]
fn h22_fiscal_year_spans_a_full_jalali_year() {
    let conn = seeded();
    let (start, end): (String, String) = conn
        .query_row(
            "SELECT start_date,end_date FROM fiscal_years WHERE id='fy-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("سال مالی");
    let start = JalaliDate::parse(&start).expect("شروع");
    let end = JalaliDate::parse(&end).expect("پایان");
    assert_eq!(
        (start.month, start.day),
        (1, 1),
        "سال مالی از ۱ فروردین شروع می‌شود"
    );
    assert_eq!(end.month, 12, "سال مالی در اسفند تمام می‌شود");
    assert_eq!(
        end.day,
        days_in_jalali_month(end.year, 12),
        "پایان سال مالی باید آخرین روز اسفند باشد"
    );
    assert_eq!(start.year, end.year);
}

/// ح۲۳ — هر تاریخ ذخیره‌شده در پایگاه داده شمسی معتبر است.
#[test]
fn h23_every_stored_date_is_a_valid_jalali_date() {
    let conn = seeded();
    for (table, column) in [
        ("journal_entries", "entry_date"),
        ("sales_invoices", "invoice_date"),
        ("checks", "due_date"),
        ("checks", "issue_date"),
    ] {
        let mut statement = conn
            .prepare(&format!("SELECT {column} FROM {table}"))
            .expect("پرس‌وجو");
        let values: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .expect("اجرا")
            .filter_map(Result::ok)
            .collect();
        assert!(!values.is_empty(), "جدول {table} خالی است");
        for value in values {
            JalaliDate::parse(&value)
                .unwrap_or_else(|_| panic!("{table}.{column} شمسی معتبر نیست: {value}"));
        }
    }
}

/// ح۲۴ — مرتب‌سازی متنی تاریخ شمسی با ترتیب زمانی یکی است.
///
/// چون قالب `YYYY/MM/DD` با صفر پیشرو است، گزارش‌ها می‌توانند بدون تبدیل
/// به میلادی مرتب شوند.
#[test]
fn h24_textual_date_sort_equals_chronological_sort() {
    let mut texts = ["1405/01/09", "1404/12/29", "1405/01/10", "1404/02/05"];
    texts.sort_unstable();
    let as_dates: Vec<NaiveDate> = texts
        .iter()
        .map(|text| {
            JalaliDate::parse(text)
                .expect("تاریخ")
                .to_gregorian()
                .expect("میلادی")
        })
        .collect();
    let mut chronological = as_dates.clone();
    chronological.sort();
    assert_eq!(
        as_dates, chronological,
        "ترتیب متنی تاریخ شمسی باید همان ترتیب زمانی باشد"
    );
}

// ===========================================================================
// خانواده ۴ — قاعده‌ی تراز دوطرفه (۷ تست)
// ===========================================================================

/// ح۲۵ — سند یک‌سطری دقیقاً دو سطر متوازن می‌سازد.
#[test]
fn h25_single_line_entry_produces_a_balanced_pair() {
    let debit = PostingSide::new(leaf("1103101", "بانک ملت", AccountNature::Debit));
    let credit = PostingSide::new(leaf("1301101", "حساب‌های دریافتنی", AccountNature::Debit));
    let lines = single_line_entry(
        &scheme(),
        rials(25_000_000),
        Some("دریافت از مشتری".into()),
        &debit,
        &credit,
    )
    .expect("سند");
    assert_eq!(lines.len(), 2);
    let totals = validate_journal(&lines).expect("تراز");
    assert_eq!(totals.total_debit, totals.total_credit);
    assert_eq!(totals.total_debit, rials(25_000_000));
}

/// ح۲۶ — یک حساب نمی‌تواند هم‌زمان هر دو طرف سند باشد.
#[test]
fn h26_same_account_on_both_sides_is_rejected() {
    let side = PostingSide::new(leaf("1103101", "بانک ملت", AccountNature::Debit));
    let error = single_line_entry(&scheme(), rials(1_000_000), None, &side, &side)
        .expect_err("باید رد شود");
    assert_eq!(error, accounting::AccountingError::SameAccountOnBothSides);
}

/// ح۲۷ — سند نامتوازن با اعلام اختلاف رد می‌شود.
#[test]
fn h27_unbalanced_voucher_reports_the_difference() {
    let lines = vec![
        JournalLine::debit("1103101", rials(1_000_000)),
        JournalLine::credit("1301101", rials(900_000)),
    ];
    let error = validate_journal(&lines).expect_err("باید رد شود");
    assert_eq!(
        error,
        accounting::AccountingError::Unbalanced {
            difference: 100_000
        }
    );
}

/// ح۲۸ — سند برگشتی دقیقاً معکوس سند اصلی است.
///
/// در رویه‌ی ایران سند اشتباه «حذف» نمی‌شود؛ سند معکوس می‌خورد تا رد
/// حسابرسی باقی بماند.
#[test]
fn h28_reversal_mirrors_the_original_exactly() {
    let original = vec![
        JournalLine::debit("1103101", rials(7_500_000)),
        JournalLine::credit("4100101", rials(7_500_000)),
    ];
    let reversal = build_reversal(&original).expect("سند معکوس");
    assert_eq!(reversal[0].debit, original[0].credit);
    assert_eq!(reversal[0].credit, original[0].debit);
    validate_journal(&reversal).expect("سند معکوس متوازن است");

    let net_debit: i64 = original
        .iter()
        .chain(reversal.iter())
        .map(|l| l.debit.rials() - l.credit.rials())
        .sum();
    assert_eq!(net_debit, 0, "اثر خالص سند و معکوسش باید صفر باشد");
}

/// ح۲۹ — سند دریافت چندروشی متوازن است و طرف حساب یک سطر تجمیعی دارد.
#[test]
fn h29_multi_method_receipt_voucher_is_balanced() {
    let lines = vec![
        DocumentLine::new(PaymentMethod::Cash, rials(4_000_000)).with_account("1101101"),
        DocumentLine::new(PaymentMethod::BankTransfer, rials(6_000_000)).with_account("1103101"),
        DocumentLine::new(PaymentMethod::Discount, rials(500_000)),
    ];
    let totals = calculate_totals(&lines).expect("جمع‌ها");
    assert_eq!(totals.total, rials(10_500_000), "تخفیف هم جزو تسویه است");
    assert_eq!(
        totals.treasury_movement,
        rials(10_000_000),
        "تخفیف پول جابه‌جا نمی‌کند"
    );

    let journal = build_journal(
        DocumentKind::Receipt,
        &lines,
        &TreasuryAccounts {
            party_account: "1301101".into(),
            notes_receivable: "1304101".into(),
            notes_payable: "2103101".into(),
            discount_account: "7101101".into(),
        },
    )
    .expect("سند خزانه");
    let totals = validate_journal(&journal).expect("تراز");
    assert_eq!(totals.total_debit, rials(10_500_000));
    let party_credit: i64 = journal
        .iter()
        .filter(|l| l.account_id == "1301101")
        .map(|l| l.credit.rials())
        .sum();
    assert_eq!(party_credit, 10_500_000, "بدهی مشتری کل مبلغ کم می‌شود");
}

/// ح۳۰ — سند تعدیل انبارگردانی هم برای کسری و هم اضافی متوازن است.
#[test]
fn h30_stocktake_adjustment_voucher_is_balanced() {
    let mut surplus = CountLine::new("p-1", 10.0, rials(1_000_000));
    surplus.counted_quantity = Some(12.0);
    surplus.variance_approved = true;
    let mut shortage = CountLine::new("p-2", 20.0, rials(500_000));
    shortage.counted_quantity = Some(17.0);
    shortage.variance_approved = true;

    let lines = vec![surplus, shortage];
    let summary = summarize(&lines).expect("خلاصه");
    assert_eq!(summary.surplus_value, rials(2_000_000));
    assert_eq!(summary.shortage_value, rials(1_500_000));
    assert_eq!(summary.net_value, rials(500_000));

    let journal = build_adjustment_journal(
        &lines,
        &VarianceAccounts {
            inventory: "1105101".into(),
            shortage_expense: "6105101".into(),
            surplus_income: "4105101".into(),
        },
    )
    .expect("سند تعدیل");
    let totals = validate_journal(&journal).expect("تراز");
    assert_eq!(totals.total_debit, totals.total_credit);
}

/// ح۳۱ — کل دفتر و تک‌تک اسناد داده‌ی نمونه متوازن‌اند.
#[test]
fn h31_seed_ledger_is_balanced_at_every_level() {
    let conn = seeded();
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(debit),0), COALESCE(SUM(credit),0) FROM journal_lines",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("جمع");
    assert!(debit > 0);
    assert_eq!(debit, credit, "دفتر کل نامتوازن است");

    let unbalanced: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT journal_id FROM journal_lines \
             GROUP BY journal_id HAVING SUM(debit) <> SUM(credit))",
            [],
            |row| row.get(0),
        )
        .expect("شمارش");
    assert_eq!(unbalanced, 0, "سند نامتوازن وجود دارد");
}

// ===========================================================================
// خانواده ۵ — چرخه‌ی چک ایرانی (۸ تست)
// ===========================================================================

/// ح۳۲ — وضعیت آغازین چک دریافتی «موجود» و پرداختی «در جریان» است.
#[test]
fn h32_initial_check_status_depends_on_kind() {
    assert_eq!(
        CheckStatus::initial(CheckKind::Received, false),
        CheckStatus::InHand
    );
    assert_eq!(
        CheckStatus::initial(CheckKind::Issued, false),
        CheckStatus::Outstanding
    );
    assert_eq!(
        CheckStatus::initial(CheckKind::Received, true),
        CheckStatus::MemoInHand
    );
}

/// ح۳۳ — چرخه‌ی متعارف چک دریافتی: موجود ← واگذار ← وصول.
#[test]
fn h33_received_check_happy_path() {
    let next = transition(
        CheckKind::Received,
        CheckStatus::InHand,
        CheckStatus::Deposited,
    )
    .expect("واگذاری");
    let final_status = transition(CheckKind::Received, next, CheckStatus::Collected).expect("وصول");
    assert_eq!(final_status, CheckStatus::Collected);
    assert!(final_status.is_terminal());
    assert!(!final_status.is_open(), "چک وصول‌شده دیگر دارایی جاری نیست");
}

/// ح۳۴ — چک وصول‌شده هم می‌تواند برگشت بخورد؛ اثرش معکوس است.
///
/// در ایران بانک می‌تواند پس از وصول، مبلغ را از حساب کسر کند.
#[test]
fn h34_a_collected_check_can_still_bounce_with_reverse_effect() {
    let bounced = transition(
        CheckKind::Received,
        CheckStatus::Collected,
        CheckStatus::Bounced,
    )
    .expect("برگشت پس از وصول");
    assert_eq!(bounced, CheckStatus::Bounced);
    assert_eq!(
        treasury_effect(
            CheckKind::Received,
            CheckStatus::Collected,
            CheckStatus::Bounced
        ),
        TreasuryEffect::Decrease,
        "برگشت باید موجودی را کم کند"
    );
}

/// ح۳۵ — وصول چک دریافتی موجودی خزانه را زیاد می‌کند.
#[test]
fn h35_collecting_a_received_check_increases_treasury() {
    assert_eq!(
        treasury_effect(
            CheckKind::Received,
            CheckStatus::Deposited,
            CheckStatus::Collected
        ),
        TreasuryEffect::Increase
    );
    assert_eq!(
        treasury_effect(
            CheckKind::Issued,
            CheckStatus::Outstanding,
            CheckStatus::Paid
        ),
        TreasuryEffect::Decrease,
        "پرداخت چک صادره از موجودی کم می‌کند"
    );
}

/// ح۳۶ — چک انتظامی هیچ اثر مالی ندارد.
///
/// چک ضمانت در دفاتر ثبت نمی‌شود؛ فقط در حساب‌های انتظامی می‌آید.
#[test]
fn h36_memo_checks_have_no_financial_effect() {
    for status in [CheckStatus::MemoInHand, CheckStatus::MemoReturned] {
        assert!(status.is_memo());
        assert!(!status.is_open(), "چک انتظامی در مانده‌ی خزانه دیده نمی‌شود");
    }
    assert_eq!(
        treasury_effect(
            CheckKind::Received,
            CheckStatus::MemoInHand,
            CheckStatus::MemoReturned
        ),
        TreasuryEffect::None
    );
}

/// ح۳۷ — گذار غیرمجاز با کد خطای مشخص رد می‌شود.
#[test]
fn h37_illegal_transitions_are_refused() {
    let error = transition(
        CheckKind::Received,
        CheckStatus::InHand,
        CheckStatus::Collected,
    )
    .expect_err("چک باید اول واگذار شود");
    assert!(format!("{error}").contains("CHK-101"));

    // وضعیت چک پرداختی برای چک دریافتی معنا ندارد.
    let error = transition(CheckKind::Received, CheckStatus::InHand, CheckStatus::Paid)
        .expect_err("وضعیت نامربوط");
    assert!(format!("{error}").contains("CHK-102"));
}

/// ح۳۸ — «خرج کردن» (ظهرنویسی) چک دریافتی مجاز است و فقط برگشت می‌پذیرد.
#[test]
fn h38_endorsement_is_supported() {
    assert!(
        allowed_transitions(CheckKind::Received, CheckStatus::InHand)
            .contains(&CheckStatus::Endorsed)
    );
    assert_eq!(
        allowed_transitions(CheckKind::Received, CheckStatus::Endorsed),
        &[CheckStatus::Bounced]
    );
    // ظهرنویسی پول نقد جابه‌جا نمی‌کند: بدهی ما به تأمین‌کننده کم می‌شود و
    // اسناد دریافتنی هم کم می‌شود. اثر «خزانه» (صندوق/بانک) صفر است.
    assert_eq!(
        treasury_effect(
            CheckKind::Received,
            CheckStatus::InHand,
            CheckStatus::Endorsed
        ),
        TreasuryEffect::None,
        "خرج کردن چک وجه نقد جابه‌جا نمی‌کند"
    );
}

/// ح۳۹ — راس‌گیری سبد چک با فرمول Σ(مبلغ×روز)/Σ(مبلغ).
///
/// «راس» ابزار روزمره‌ی بازار ایران است: تاریخی که پرداخت یک‌جا معادل
/// مالی سبد چک باشد.
#[test]
fn h39_weighted_maturity_matches_the_market_formula() {
    let base = gregorian(2025, 8, 21);
    let items = vec![
        CheckItem::new(rials(10_000_000), base + chrono::Duration::days(30)),
        CheckItem::new(rials(30_000_000), base + chrono::Duration::days(90)),
    ];
    let average = weighted_maturity(base, &items).expect("راس");
    // (10×30 + 30×90) / 40 = 75
    assert_eq!(average.days, 75);
    assert_eq!(average.total_amount, rials(40_000_000));
    assert_eq!(average.count, 2);
    assert_eq!(
        maturity_date(base, &items).expect("تاریخ راس"),
        base + chrono::Duration::days(75)
    );
}

// ===========================================================================
// خانواده ۶ — کدینگ، تفصیلی شناور و ارزش‌گذاری انبار (۶ تست)
// ===========================================================================

/// ح۴۰ — کدینگ پیش‌فرض ایرانی: گروه/کل/معین/تفصیلی با طول ۱،۳،۵،۷.
#[test]
fn h40_default_coding_is_the_iranian_four_level_scheme() {
    let scheme = scheme();
    assert_eq!(scheme.depth(), 4);
    assert_eq!(scheme.code_length(0), Some(1));
    assert_eq!(scheme.code_length(1), Some(3));
    assert_eq!(scheme.code_length(2), Some(5));
    assert_eq!(scheme.code_length(3), Some(7));
    assert_eq!(scheme.level_title(2), Some("معین"));
    assert_eq!(scheme.parent_code("1103101").expect("والد"), "11031");
    assert!(scheme.is_leaf_level("1103101").expect("سطح آخر"));
    assert!(!scheme.is_leaf_level("11031").expect("سطح معین"));
}

/// ح۴۱ — ثبت سند فقط روی حساب سطح آخر مجاز است.
#[test]
fn h41_posting_is_only_allowed_on_leaf_accounts() {
    let debit = PostingSide::new(leaf("11031", "بانک‌ها", AccountNature::Debit));
    let credit = PostingSide::new(leaf("1301101", "دریافتنی", AccountNature::Debit));
    let error = single_line_entry(&scheme(), rials(1_000_000), None, &debit, &credit)
        .expect_err("حساب معین قابل ثبت نیست");
    assert!(format!("{error}").contains("COD-008"));
}

/// ح۴۲ — تفصیلی شناور: حسابی که تفصیلی الزامی دارد بدون آن ثبت نمی‌شود.
#[test]
fn h42_floating_subsidiary_is_enforced() {
    let mut account = leaf("1301101", "حساب‌های دریافتنی", AccountNature::Debit);
    account.requires_subsidiary = true;
    account.subsidiary_group = Some("اشخاص".into());

    let bank = PostingSide::new(leaf("1103101", "بانک ملت", AccountNature::Debit));

    let without = PostingSide::new(account.clone());
    let error = single_line_entry(&scheme(), rials(1_000_000), None, &bank, &without)
        .expect_err("تفصیلی الزامی است");
    assert!(format!("{error}").contains("COD-009"));

    // تفصیلی از گروه اشتباه هم رد می‌شود.
    let wrong_group = PostingSide::with_dimensions(
        account.clone(),
        Dimensions::with_subsidiary(Subsidiary {
            code: "9001".into(),
            title: "بانک ملت".into(),
            group: "بانک‌ها".into(),
        }),
    );
    let error = single_line_entry(&scheme(), rials(1_000_000), None, &bank, &wrong_group)
        .expect_err("گروه تفصیلی نادرست");
    assert!(format!("{error}").contains("COD-010"));

    // تفصیلی درست پذیرفته می‌شود و کد تفصیلی روی سطر می‌نشیند.
    let correct = PostingSide::with_dimensions(
        account,
        Dimensions::with_subsidiary(Subsidiary {
            code: "1001".into(),
            title: "شرکت آریا".into(),
            group: "اشخاص".into(),
        }),
    );
    let lines = single_line_entry(&scheme(), rials(1_000_000), None, &bank, &correct).expect("سند");
    assert_eq!(lines[1].subsidiary_id.as_deref(), Some("1001"));
}

/// ح۴۳ — روش‌های ارزش‌گذاری موجودی: FIFO و میانگین موزون.
///
/// روش LIFO در ایران پذیرفته نیست و عمداً پیاده نشده است.
#[test]
fn h43_valuation_methods_match_iranian_practice() {
    assert!(
        ValuationMethod::parse("lifo").is_err(),
        "LIFO نباید پشتیبانی شود"
    );
    assert_eq!(
        ValuationMethod::parse("fifo").expect("روش"),
        ValuationMethod::Fifo
    );
    assert_eq!(
        ValuationMethod::parse("weighted_average").expect("روش"),
        ValuationMethod::WeightedAverage
    );

    let movements = vec![
        Movement::new(MovementKind::Receipt, 10.0, 1_000_000),
        Movement::new(MovementKind::Receipt, 10.0, 2_000_000),
        Movement::new(MovementKind::Issue, 5.0, 0),
    ];
    let fifo = valuate(&movements, ValuationMethod::Fifo).expect("FIFO");
    assert_eq!(fifo.quantity, 15.0);
    // ۵ عدد از لایه‌ی اول مصرف شد: ۵×۱٬۰۰۰٬۰۰۰ + ۱۰×۲٬۰۰۰٬۰۰۰
    assert_eq!(fifo.total_value, 25_000_000);

    let average = valuate(&movements, ValuationMethod::WeightedAverage).expect("میانگین");
    assert_eq!(average.quantity, 15.0);
    assert_eq!(average.unit_cost, 1_500_000);
}

/// ح۴۴ — بهای تمام‌شده‌ی کالای فروش‌رفته به FIFO از لایه‌ها برداشت می‌شود.
#[test]
fn h44_cogs_consumes_fifo_layers_in_order() {
    let mut layers = novin_core::inventory::fifo_layers(&[
        Movement::new(MovementKind::Receipt, 4.0, 1_000_000),
        Movement::new(MovementKind::Receipt, 6.0, 1_500_000),
    ]);
    let cost = consume_fifo(&mut layers, 5.0).expect("بهای تمام‌شده");
    // ۴×۱٬۰۰۰٬۰۰۰ + ۱×۱٬۵۰۰٬۰۰۰
    assert_eq!(cost, 5_500_000);
    assert_eq!(layers.len(), 1);
    assert!((layers[0].quantity - 5.0).abs() < 1e-9);
    assert!(
        consume_fifo(&mut layers, 99.0).is_err(),
        "موجودی ناکافی باید رد شود"
    );
}

/// ح۴۵ — خدمت موجودی انبار ندارد؛ کالای ساده و مرکب دارند.
#[test]
fn h45_services_do_not_carry_inventory() {
    assert!(!ProductKind::Service.tracks_inventory());
    assert!(ProductKind::Simple.tracks_inventory());
    assert!(ProductKind::GoldJewelry.tracks_inventory());
}

// ===========================================================================
// خانواده ۷ — شناسه‌های هویتی و اعتبار مشتری (۵ تست)
// ===========================================================================

/// ح۴۶ — کد ملی با الگوریتم رقم کنترل رسمی سنجیده می‌شود.
#[test]
fn h46_national_id_checksum_is_enforced() {
    assert!(national_id_is_valid("0499370899"), "کد ملی معتبر");
    assert!(!national_id_is_valid("0499370898"), "رقم کنترل غلط");
    assert!(
        !national_id_is_valid("1111111111"),
        "ارقام یکسان معتبر نیست"
    );
    assert!(!national_id_is_valid("049937089"), "طول کمتر از ۱۰");
}

/// ح۴۷ — شناسه ملی شخص حقوقی و کد اقتصادی و کد پستی.
#[test]
fn h47_legal_identifiers_follow_their_official_rules() {
    assert!(
        legal_id_is_valid("10101234565"),
        "شناسه ملی با رقم کنترل درست"
    );
    assert!(!legal_id_is_valid("10101234561"), "رقم کنترل غلط");
    assert!(!legal_id_is_valid("1010123456"), "شناسه ملی ۱۱ رقمی است");
    assert!(!legal_id_is_valid("11111111111"), "ارقام یکسان معتبر نیست");
    assert!(economic_code_is_valid("411111111111"), "کد اقتصادی ۱۲ رقمی");
    assert!(!economic_code_is_valid("41111111111"), "طول نادرست");
    assert!(postal_code_is_valid("1968833113"), "کد پستی ۱۰ رقمی");
    assert!(!postal_code_is_valid("19688331"), "طول نادرست");
}

/// ح۴۸ — شبا و شماره کارت با الگوریتم رسمی (mod-97 و Luhn).
#[test]
fn h48_iban_and_card_number_use_official_algorithms() {
    assert!(iban_is_valid("IR062960000000100324200001"), "شبای معتبر");
    assert!(
        !iban_is_valid("IR062960000000100324200002"),
        "رقم کنترل غلط"
    );
    assert!(
        card_number_is_valid("6037997599999993"),
        "کارت با Luhn درست"
    );
    assert!(
        !card_number_is_valid("6037997599999999"),
        "رقم کنترل Luhn غلط"
    );
    assert!(!card_number_is_valid("603799759999999"), "کارت ۱۶ رقمی است");
}

/// ح۴۹ — موبایل ایرانی به قالب یکسان `09xxxxxxxxx` نرمال می‌شود.
#[test]
fn h49_mobile_numbers_are_normalised_to_a_single_format() {
    for input in [
        "09121234567",
        "+989121234567",
        "00989121234567",
        "۰۹۱۲۱۲۳۴۵۶۷",
    ] {
        assert_eq!(
            normalize_mobile(input).as_deref(),
            Some("09121234567"),
            "نرمال‌سازی «{input}»"
        );
    }
    assert!(
        normalize_mobile("02188776655").is_none(),
        "تلفن ثابت موبایل نیست"
    );
}

/// ح۵۰ — سقف اعتبار، اعتبار باقیمانده و علامت مانده‌ی بدهکار/بستانکار.
///
/// قرارداد علامت در کل سیستم: مثبت = بدهکار (طلب ما از مشتری).
#[test]
fn h50_credit_limit_and_balance_sign_convention() {
    // سقف صفر یعنی بدون محدودیت.
    check_credit_limit(rials(900_000_000), 0, rials(100_000_000)).expect("بدون سقف");

    // عبور از سقف رد می‌شود و مبلغ‌ها در پیام خطا می‌آیند.
    let error = check_credit_limit(rials(80_000_000), 100_000_000, rials(30_000_000))
        .expect_err("عبور از سقف");
    assert_eq!(
        error,
        PartyError::CreditLimitExceeded {
            balance: 110_000_000,
            limit: 100_000_000
        }
    );

    // اعتبار باقیمانده هرگز منفی نمایش داده نمی‌شود.
    assert_eq!(
        remaining_credit(rials(120_000_000), 100_000_000),
        Some(Money::ZERO)
    );
    assert_eq!(
        remaining_credit(rials(40_000_000), 100_000_000),
        Some(rials(60_000_000))
    );
    assert_eq!(
        remaining_credit(rials(40_000_000), 0),
        None,
        "بدون سقف یعنی بدون نمایش"
    );

    // خلاصه‌ی مانده‌ها: بدهکار و بستانکار جدا شمرده می‌شوند.
    let summary = summarize_balances(&[
        rials(50_000_000),
        rials(-20_000_000),
        Money::ZERO,
        rials(10_000_000),
    ]);
    assert_eq!(summary.debtor_count, 2);
    assert_eq!(summary.debtor_total, rials(60_000_000));
    assert_eq!(summary.creditor_count, 1);
    assert_eq!(
        summary.creditor_total,
        rials(20_000_000),
        "بستانکار قدرمطلق"
    );
    assert_eq!(summary.settled_count, 1);
    assert_eq!(summary.net_total, rials(40_000_000));
}

// ===========================================================================
// تکمیلی — قواعد فروشگاهی ایران
// ===========================================================================

/// ح۵۱ — سطوح قیمت با زنجیره‌ی جایگزینی کار می‌کنند.
///
/// در بازار ایران «همکار درجه ۲» اگر قیمت اختصاصی نداشته باشد باید به
/// قیمت همکار، بعد عمده، بعد خرده‌فروشی برگردد — نه اینکه خطا بدهد.
#[test]
fn h51_price_level_fallback_chain() {
    let mut list = PriceList::new();
    list.set(PriceLevel::Retail, rials(1_000_000))
        .expect("قیمت");
    list.set(PriceLevel::Wholesale, rials(900_000))
        .expect("قیمت");

    assert_eq!(
        list.effective(PriceLevel::PartnerTier2).expect("قیمت"),
        rials(900_000)
    );
    assert!(list.exact(PriceLevel::PartnerTier2).is_none());
    assert_eq!(
        list.effective(PriceLevel::Retail).expect("قیمت"),
        rials(1_000_000)
    );
}

/// ح۵۲ — تخفیف پلکانی بر اساس مقدار، همان «تخفیف عمده» بازار است.
#[test]
fn h52_quantity_tiers_apply_the_highest_matching_step() {
    let mut item = InvoiceLine::new("p-1", 120.0, rials(100_000));
    item.tiers = vec![
        DiscountTier {
            min_quantity: 50.0,
            discount_bp: 500,
        },
        DiscountTier {
            min_quantity: 100.0,
            discount_bp: 1_000,
        },
    ];
    let result = invoicing::calculate(&InvoiceInput {
        lines: vec![item],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    })
    .expect("محاسبه");
    assert_eq!(result.subtotal, rials(12_000_000));
    assert_eq!(
        result.discount_total,
        rials(1_200_000),
        "پله‌ی ۱۰٪ باید اعمال شود"
    );
}

/// ح۵۳ — تبدیل واحد (کارتن ← عدد) و قیمت واحد فرعی.
#[test]
fn h53_unit_conversion_keeps_price_consistent() {
    let units = UnitSet::new("عدد")
        .with_unit("کارتن", 12.0)
        .expect("واحد")
        .with_unit("بسته", 4.0)
        .expect("واحد");
    assert_eq!(units.to_base(3.0, "کارتن").expect("تبدیل"), 36.0);
    assert_eq!(units.convert(3.0, "کارتن", "بسته").expect("تبدیل"), 9.0);
    assert_eq!(
        units.unit_price(rials(50_000), "کارتن").expect("قیمت"),
        rials(600_000)
    );
}

/// ح۵۴ — سیاست منفی شدن موجودی صندوق: خطا، هشدار یا نادیده.
#[test]
fn h54_negative_treasury_balance_policy() {
    let error = check_withdrawal(
        "صندوق فروشگاه",
        rials(1_000_000),
        rials(3_000_000),
        NegativeBalancePolicy::Error,
    )
    .expect_err("باید رد شود");
    assert!(format!("{error}").contains("TRS-006"));

    let warning = check_withdrawal(
        "صندوق فروشگاه",
        rials(1_000_000),
        rials(3_000_000),
        NegativeBalancePolicy::Warn,
    )
    .expect("هشدار");
    match warning {
        BalanceCheck::Warning(message) => assert!(message.contains("2,000,000")),
        BalanceCheck::Allowed => panic!("باید هشدار می‌داد"),
    }

    assert_eq!(
        check_withdrawal(
            "صندوق فروشگاه",
            rials(5_000_000),
            rials(3_000_000),
            NegativeBalancePolicy::Error
        )
        .expect("مجاز"),
        BalanceCheck::Allowed
    );
}

/// ح۵۵ — مانده‌ی زنده‌ی طرف حساب هنگام ثبت فاکتور.
#[test]
fn h55_live_party_balance_during_invoicing() {
    let view = invoicing::balance_view(rials(5_000_000), rials(12_000_000), rials(4_000_000));
    assert_eq!(view.after, rials(13_000_000), "مانده پس از فاکتور");
    assert_eq!(
        view.invoice_remainder,
        rials(8_000_000),
        "مانده‌ی خود فاکتور"
    );
}
