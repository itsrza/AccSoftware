//! # تست‌های سخت‌گیرانه‌ی فاز ۵ — فاکتور کامل
//!
//! مرجع: تصاویر `sFpxWK` (فاکتور فروش)، `PI5uot` (فاکتور خرید)، `FRPBDr`
//! (برگشت از فروش) — با مبالغ واقعی همان فرم‌ها.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | معادله‌ی بنیادی | جمع اجزای سطرها دقیقاً برابر جمع فاکتور |
//! | ۲ | تخفیف پلکانی | پله‌ی درست انتخاب و اعمال می‌شود |
//! | ۳ | تخفیف چندلایه | سطری + سرجمع + کوپن بدون گم شدن ریال |
//! | ۴ | کوپن | سقف، حداقل فاکتور و درصد/مبلغ |
//! | ۵ | مالیات و عوارض | ترتیب درست: عوارض پیش از ارزش افزوده |
//! | ۶ | کرایه حمل | دو حالت افزودن به جمع و سرشکن روی سطرها |
//! | ۷ | سود فاکتور | سود ناخالص و حاشیه پس از پورسانت |
//! | ۸ | سریال | تعداد برابر مقدار و بدون تکرار |
//! | ۹ | اقساط | سررسید ماه شمسی و جمع دقیق |
//! | ۱۰ | مانده و تسویه | نمایش زنده‌ی مانده و تفکیک روش‌های پرداخت |

use chrono::NaiveDate;
use novin_core::invoicing::{
    balance_view, calculate, installment_plan, resolve_tier_discount, validate_serials, Coupon,
    CouponKind, DiscountTier, FreightMode, InvoiceError, InvoiceInput, InvoiceLine,
    SettlementBreakdown,
};
use novin_core::jalali::JalaliDate;
use novin_core::money::Money;

fn simple_invoice(lines: Vec<InvoiceLine>) -> InvoiceInput {
    InvoiceInput {
        lines,
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    }
}

// ---------------------------------------------------------------------------
// تست ۱ — معادله‌ی بنیادی فاکتور
// ---------------------------------------------------------------------------
#[test]
fn t01_invoice_identity_always_holds() {
    // قیمت‌های واقعی از لیست کالاها: پرینتر ۱۲٬۵۰۰٬۰۰۰ و بارکدخوان ۸٬۹۰۰٬۰۰۰
    let mut printer = InvoiceLine::new("prod-1", 2.0, Money::from_rials(12_500_000));
    printer.vat_bp = 900;
    printer.unit_cost = Money::from_rials(9_500_000);
    let mut scanner = InvoiceLine::new("prod-2", 3.0, Money::from_rials(8_900_000));
    scanner.vat_bp = 900;
    scanner.discount_bp = 500;
    scanner.unit_cost = Money::from_rials(6_500_000);

    let input = InvoiceInput {
        lines: vec![printer, scanner],
        header_discount: Money::from_rials(1_234_567),
        coupon: None,
        freight: Money::from_rials(350_000),
        freight_mode: FreightMode::AddToTotal,
    };
    let result = calculate(&input).unwrap();

    // جمع ستون‌ها = جمع فاکتور
    let sum = |extract: fn(&novin_core::invoicing::ComputedLine) -> Money| -> Money {
        result.lines.iter().map(extract).sum()
    };
    assert_eq!(sum(|line| line.gross), result.subtotal);
    assert_eq!(sum(|line| line.total_discount), result.discount_total);
    assert_eq!(sum(|line| line.net), result.net_total);
    assert_eq!(sum(|line| line.vat), result.vat_total);
    assert_eq!(sum(|line| line.duty), result.duty_total);
    assert_eq!(
        sum(|line| line.total) + result.freight,
        result.total,
        "کرایه در حالت افزودن به جمع باید فقط یک بار اضافه شود"
    );

    // معادله‌ی بنیادی
    assert_eq!(
        result.total,
        result.subtotal - result.discount_total
            + result.duty_total
            + result.vat_total
            + result.freight
    );
    // تخفیف سرجمع کامل توزیع شده است
    let header_shares: Money = sum(|line| line.header_discount_share);
    assert_eq!(header_shares, Money::from_rials(1_234_567));
}

// ---------------------------------------------------------------------------
// تست ۲ — تخفیف پلکانی
// ---------------------------------------------------------------------------
#[test]
fn t02_tiered_discount_picks_the_right_step() {
    let tiers = vec![
        DiscountTier {
            min_quantity: 10.0,
            discount_bp: 500, // ۵٪
        },
        DiscountTier {
            min_quantity: 50.0,
            discount_bp: 1_000, // ۱۰٪
        },
        DiscountTier {
            min_quantity: 100.0,
            discount_bp: 1_500, // ۱۵٪
        },
    ];

    assert_eq!(resolve_tier_discount(&tiers, 5.0), 0);
    assert_eq!(resolve_tier_discount(&tiers, 10.0), 500, "مرز پله شامل است");
    assert_eq!(resolve_tier_discount(&tiers, 49.9), 500);
    assert_eq!(resolve_tier_discount(&tiers, 50.0), 1_000);
    assert_eq!(resolve_tier_discount(&tiers, 99.0), 1_000);
    assert_eq!(resolve_tier_discount(&tiers, 1_000.0), 1_500);
    assert_eq!(resolve_tier_discount(&[], 100.0), 0);

    // ترتیب ورودی نباید مهم باشد
    let shuffled = vec![tiers[2], tiers[0], tiers[1]];
    assert_eq!(resolve_tier_discount(&shuffled, 60.0), 1_000);

    // اعمال روی فاکتور: ۶۰ عدد × ۱٬۰۰۰٬۰۰۰ با پله‌ی ۱۰٪
    let mut line = InvoiceLine::new("prod-1", 60.0, Money::from_rials(1_000_000));
    line.tiers = tiers;
    let result = calculate(&simple_invoice(vec![line])).unwrap();
    assert_eq!(result.subtotal, Money::from_rials(60_000_000));
    assert_eq!(result.lines[0].tier_discount, Money::from_rials(6_000_000));
    assert_eq!(result.net_total, Money::from_rials(54_000_000));
}

// ---------------------------------------------------------------------------
// تست ۳ — تخفیف چندلایه بدون گم شدن ریال
// ---------------------------------------------------------------------------
#[test]
fn t03_layered_discounts_lose_no_rial() {
    let mut first = InvoiceLine::new("p1", 3.0, Money::from_rials(333_333));
    first.discount_amount = Money::from_rials(11_111);
    first.discount_bp = 250;
    first.tiers = vec![DiscountTier {
        min_quantity: 2.0,
        discount_bp: 300,
    }];
    let second = InvoiceLine::new("p2", 7.0, Money::from_rials(777_777));
    let input = InvoiceInput {
        lines: vec![first, second],
        header_discount: Money::from_rials(97),
        coupon: Some(Coupon {
            code: "NOWRUZ".into(),
            kind: CouponKind::Percent(150),
            minimum_invoice: None,
            maximum_discount: None,
        }),
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    };
    let result = calculate(&input).unwrap();

    let discount_sum: Money = result.lines.iter().map(|line| line.total_discount).sum();
    assert_eq!(discount_sum, result.discount_total);
    let net_sum: Money = result.lines.iter().map(|line| line.net).sum();
    assert_eq!(net_sum, result.net_total);
    assert_eq!(result.net_total, result.subtotal - result.discount_total);

    // تخفیف سرجمع دقیقاً ۹۷ ریال توزیع شده
    let header: Money = result
        .lines
        .iter()
        .map(|line| line.header_discount_share)
        .sum();
    assert_eq!(header, Money::from_rials(97));

    // تخفیف بیش از مبلغ سطر مردود است
    let mut invalid = InvoiceLine::new("p", 1.0, Money::from_rials(1_000));
    invalid.discount_amount = Money::from_rials(2_000);
    assert_eq!(
        calculate(&simple_invoice(vec![invalid])),
        Err(InvoiceError::DiscountTooLarge)
    );

    // تخفیف سرجمع بیش از جمع سطرها
    let too_much = InvoiceInput {
        lines: vec![InvoiceLine::new("p", 1.0, Money::from_rials(1_000))],
        header_discount: Money::from_rials(5_000),
        coupon: None,
        freight: Money::ZERO,
        freight_mode: FreightMode::AddToTotal,
    };
    assert_eq!(calculate(&too_much), Err(InvoiceError::DiscountTooLarge));
}

// ---------------------------------------------------------------------------
// تست ۴ — کوپن تخفیف
// ---------------------------------------------------------------------------
#[test]
fn t04_coupon_rules() {
    let base = Money::from_rials(10_000_000);

    // درصدی ساده
    let percent = Coupon {
        code: "OFF10".into(),
        kind: CouponKind::Percent(1_000),
        minimum_invoice: None,
        maximum_discount: None,
    };
    assert_eq!(
        percent.discount_for(base).unwrap(),
        Money::from_rials(1_000_000)
    );

    // سقف تخفیف
    let capped = Coupon {
        code: "CAP".into(),
        kind: CouponKind::Percent(5_000),
        minimum_invoice: None,
        maximum_discount: Some(Money::from_rials(2_000_000)),
    };
    assert_eq!(
        capped.discount_for(base).unwrap(),
        Money::from_rials(2_000_000)
    );

    // حداقل مبلغ فاکتور
    let minimum = Coupon {
        code: "MIN".into(),
        kind: CouponKind::Amount(Money::from_rials(500_000)),
        minimum_invoice: Some(Money::from_rials(20_000_000)),
        maximum_discount: None,
    };
    assert_eq!(
        minimum.discount_for(base),
        Err(InvoiceError::CouponNotApplicable)
    );
    assert_eq!(
        minimum.discount_for(Money::from_rials(20_000_000)).unwrap(),
        Money::from_rials(500_000)
    );

    // کوپن نمی‌تواند بیشتر از خود مبلغ باشد
    let huge = Coupon {
        code: "HUGE".into(),
        kind: CouponKind::Amount(Money::from_rials(99_000_000)),
        minimum_invoice: None,
        maximum_discount: None,
    };
    assert_eq!(huge.discount_for(base).unwrap(), base);

    // نرخ نامعتبر
    let invalid = Coupon {
        code: "BAD".into(),
        kind: CouponKind::Percent(20_000),
        minimum_invoice: None,
        maximum_discount: None,
    };
    assert_eq!(invalid.discount_for(base), Err(InvoiceError::InvalidRate));
}

// ---------------------------------------------------------------------------
// تست ۵ — عوارض و ارزش افزوده
// ---------------------------------------------------------------------------
#[test]
fn t05_duty_is_applied_before_vat() {
    let mut line = InvoiceLine::new("p", 1.0, Money::from_rials(1_000_000));
    line.duty_bp = 100; // ۱٪ عوارض
    line.vat_bp = 900; // ۹٪ ارزش افزوده
    let result = calculate(&simple_invoice(vec![line])).unwrap();

    assert_eq!(result.duty_total, Money::from_rials(10_000));
    // ارزش افزوده روی (خالص + عوارض) محاسبه می‌شود: ۹٪ × ۱٬۰۱۰٬۰۰۰
    assert_eq!(result.vat_total, Money::from_rials(90_900));
    assert_eq!(result.total, Money::from_rials(1_100_900));

    // کالای معاف
    let exempt = InvoiceLine::new("p", 1.0, Money::from_rials(1_000_000));
    let result = calculate(&simple_invoice(vec![exempt])).unwrap();
    assert_eq!(result.vat_total, Money::ZERO);
    assert_eq!(result.total, Money::from_rials(1_000_000));

    // نرخ نامعتبر
    let mut bad = InvoiceLine::new("p", 1.0, Money::from_rials(1_000));
    bad.vat_bp = 10_001;
    assert_eq!(
        calculate(&simple_invoice(vec![bad])),
        Err(InvoiceError::InvalidRate)
    );
}

// ---------------------------------------------------------------------------
// تست ۶ — کرایه حمل
// ---------------------------------------------------------------------------
#[test]
fn t06_freight_modes_behave_differently() {
    let make = |mode| InvoiceInput {
        lines: vec![
            InvoiceLine::new("p1", 1.0, Money::from_rials(1_000_000)),
            InvoiceLine::new("p2", 1.0, Money::from_rials(3_000_000)),
        ],
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::from_rials(400_000),
        freight_mode: mode,
    };

    // حالت افزودن به جمع: سطرها دست‌نخورده
    let added = calculate(&make(FreightMode::AddToTotal)).unwrap();
    assert_eq!(added.lines[0].freight_share, Money::ZERO);
    assert_eq!(added.total, Money::from_rials(4_400_000));

    // حالت سرشکن: به نسبت مبلغ سطر (۱ به ۳)
    let allocated = calculate(&make(FreightMode::AllocateToLines)).unwrap();
    assert_eq!(allocated.lines[0].freight_share, Money::from_rials(100_000));
    assert_eq!(allocated.lines[1].freight_share, Money::from_rials(300_000));
    let shares: Money = allocated.lines.iter().map(|line| line.freight_share).sum();
    assert_eq!(shares, Money::from_rials(400_000), "کرایه کامل توزیع شود");
    assert_eq!(allocated.total, Money::from_rials(4_400_000));

    // در هر دو حالت جمع نهایی یکی است
    assert_eq!(added.total, allocated.total);

    // کرایه منفی
    let mut negative = make(FreightMode::AddToTotal);
    negative.freight = Money::from_rials(-1);
    assert_eq!(calculate(&negative), Err(InvoiceError::NegativeAmount));
}

// ---------------------------------------------------------------------------
// تست ۷ — سود فاکتور
// ---------------------------------------------------------------------------
#[test]
fn t07_invoice_profit_and_commission() {
    // فروش ۱۰ عدد به ۱٬۰۰۰٬۰۰۰ با بهای ۶۰۰٬۰۰۰ و پورسانت ۵٪
    let mut line = InvoiceLine::new("p", 10.0, Money::from_rials(1_000_000));
    line.unit_cost = Money::from_rials(600_000);
    line.commission_bp = 500;
    line.vat_bp = 900;
    let result = calculate(&simple_invoice(vec![line])).unwrap();

    assert_eq!(result.net_total, Money::from_rials(10_000_000));
    assert_eq!(result.cost_total, Money::from_rials(6_000_000));
    assert_eq!(result.commission_total, Money::from_rials(500_000));
    // سود = فروش خالص − بهای تمام‌شده − پورسانت
    assert_eq!(result.profit, Money::from_rials(3_500_000));
    assert_eq!(result.profit_margin_bp, 3_500, "حاشیه‌ی سود ۳۵٪");
    // ارزش افزوده نباید در سود دیده شود
    assert!(result.vat_total > Money::ZERO);
    assert_eq!(
        result.profit,
        result.net_total - result.cost_total - result.commission_total
    );

    // فروش زیر قیمت تمام‌شده → سود منفی
    let mut loss = InvoiceLine::new("p", 1.0, Money::from_rials(500_000));
    loss.unit_cost = Money::from_rials(800_000);
    let result = calculate(&simple_invoice(vec![loss])).unwrap();
    assert_eq!(result.profit, Money::from_rials(-300_000));
    assert!(result.profit_margin_bp < 0);

    // بدون بهای تمام‌شده، سود برابر فروش خالص است
    let plain = InvoiceLine::new("p", 1.0, Money::from_rials(100_000));
    let result = calculate(&simple_invoice(vec![plain])).unwrap();
    assert_eq!(result.profit, Money::from_rials(100_000));
    assert_eq!(result.profit_margin_bp, 10_000);
}

// ---------------------------------------------------------------------------
// تست ۸ — سریال کالا
// ---------------------------------------------------------------------------
#[test]
fn t08_serial_numbers_are_validated() {
    let mut line = InvoiceLine::new("p", 3.0, Money::from_rials(1_000_000));
    line.serial_tracked = true;
    line.serials = vec!["SN-1".into(), "SN-2".into(), "SN-3".into()];
    assert!(validate_serials(&[line.clone()]).is_ok());
    assert!(calculate(&simple_invoice(vec![line.clone()])).is_ok());

    // تعداد کمتر از مقدار
    let mut short = line.clone();
    short.serials = vec!["SN-1".into()];
    assert_eq!(
        validate_serials(&[short]),
        Err(InvoiceError::SerialCountMismatch {
            expected: 3,
            actual: 1
        })
    );

    // تکرار سریال در دو سطر مختلف فاکتور
    let mut other = InvoiceLine::new("p2", 1.0, Money::from_rials(500_000));
    other.serial_tracked = true;
    other.serials = vec!["SN-2".into()];
    assert_eq!(
        validate_serials(&[line.clone(), other]),
        Err(InvoiceError::DuplicateSerial {
            serial: "SN-2".into()
        })
    );

    // کالای بدون سریال نیازی به سریال ندارد
    let plain = InvoiceLine::new("p3", 5.0, Money::from_rials(100));
    assert!(validate_serials(&[plain]).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۹ — اقساط
// ---------------------------------------------------------------------------
#[test]
fn t09_installment_plan_uses_jalali_months() {
    // مبلغی که بر تعداد اقساط بخش‌پذیر نیست
    let total = Money::from_rials(10_000_000);
    let down = Money::from_rials(1_000_000);
    let first_due = JalaliDate::new(1404, 6, 31)
        .unwrap()
        .to_gregorian()
        .unwrap();
    let plan = installment_plan(total, down, 7, first_due).unwrap();

    assert_eq!(plan.len(), 7);
    let sum: Money = plan.iter().map(|item| item.amount).sum();
    assert_eq!(
        sum,
        Money::from_rials(9_000_000),
        "جمع اقساط باید دقیق باشد"
    );

    // سررسیدها ماه شمسی جلو می‌روند و روز در ماه کوتاه‌تر اصلاح می‌شود
    assert_eq!(plan[0].due_date_jalali, "1404/06/31");
    assert_eq!(plan[1].due_date_jalali, "1404/07/30", "مهر ۳۰ روز دارد");
    assert_eq!(
        plan[6].due_date_jalali, "1404/12/29",
        "اسفند سال عادی ۲۹ روز"
    );
    assert_eq!(plan[0].number, 1);
    assert_eq!(plan[6].number, 7);

    // عبور از سال
    let year_end = JalaliDate::new(1404, 11, 30)
        .unwrap()
        .to_gregorian()
        .unwrap();
    let crossing = installment_plan(Money::from_rials(300), Money::ZERO, 3, year_end).unwrap();
    assert_eq!(crossing[0].due_date_jalali, "1404/11/30");
    assert_eq!(crossing[1].due_date_jalali, "1404/12/29");
    assert_eq!(crossing[2].due_date_jalali, "1405/01/30");

    // بدون پیش‌پرداخت
    let full = installment_plan(Money::from_rials(1_000), Money::ZERO, 3, first_due).unwrap();
    let sum: Money = full.iter().map(|item| item.amount).sum();
    assert_eq!(sum, Money::from_rials(1_000));
    assert_eq!(full[0].amount, Money::from_rials(334));

    // ورودی‌های نامعتبر
    assert_eq!(
        installment_plan(total, down, 0, first_due),
        Err(InvoiceError::InvalidInstallmentCount)
    );
    assert_eq!(
        installment_plan(total, down, 121, first_due),
        Err(InvoiceError::InvalidInstallmentCount)
    );
    assert_eq!(
        installment_plan(total, Money::from_rials(20_000_000), 3, first_due),
        Err(InvoiceError::DownPaymentTooLarge)
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — مانده‌ی زنده و تفکیک تسویه
// ---------------------------------------------------------------------------
#[test]
fn t10_live_balance_and_settlement_breakdown() {
    // مانده‌ی واقعی سربرگ فاکتور: ۳۰٬۷۷۴٬۳۳۰ بدهکار
    let before = Money::from_rials(30_774_330);
    let invoice = Money::from_rials(12_500_000);
    let received = Money::from_rials(5_000_000);

    let view = balance_view(before, invoice, received);
    assert_eq!(view.before, before);
    assert_eq!(view.invoice_effect, invoice);
    assert_eq!(view.after, Money::from_rials(38_274_330));
    assert_eq!(view.invoice_remainder, Money::from_rials(7_500_000));

    // تسویه‌ی کامل
    let settled = balance_view(before, invoice, invoice);
    assert_eq!(settled.after, before);
    assert_eq!(settled.invoice_remainder, Money::ZERO);

    // مشتری بستانکار
    let creditor = balance_view(Money::from_rials(-1_000_000), invoice, Money::ZERO);
    assert_eq!(creditor.after, Money::from_rials(11_500_000));

    // تفکیک روش‌های تسویه
    let breakdown = SettlementBreakdown {
        cash: Money::from_rials(2_000_000),
        check: Money::from_rials(5_000_000),
        transfer: Money::from_rials(3_000_000),
        card: Money::from_rials(2_500_000),
    };
    assert_eq!(breakdown.total(), Money::from_rials(12_500_000));
    assert!(breakdown.validate(invoice).is_ok());

    // پرداخت بیش از مبلغ فاکتور
    let over = SettlementBreakdown {
        cash: Money::from_rials(99_000_000),
        ..Default::default()
    };
    assert_eq!(over.validate(invoice), Err(InvoiceError::DiscountTooLarge));

    // مبلغ منفی
    let negative = SettlementBreakdown {
        cash: Money::from_rials(-1),
        ..Default::default()
    };
    assert_eq!(
        negative.validate(invoice),
        Err(InvoiceError::NegativeAmount)
    );

    // فاکتور خالی
    assert_eq!(
        calculate(&InvoiceInput {
            lines: vec![],
            header_discount: Money::ZERO,
            coupon: None,
            freight: Money::ZERO,
            freight_mode: FreightMode::AddToTotal,
        }),
        Err(InvoiceError::EmptyInvoice)
    );
    // مقدار نامعتبر
    assert_eq!(
        calculate(&simple_invoice(vec![InvoiceLine::new(
            "p",
            0.0,
            Money::from_rials(1)
        )])),
        Err(InvoiceError::InvalidQuantity)
    );
}
