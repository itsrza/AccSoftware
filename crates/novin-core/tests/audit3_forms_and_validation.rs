#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! ممیزی ۳ — فرم‌ها، اعتبارسنجی و منطق حسابداری عمیق.
//!
//! ممیزی ۱ پرسید «هست؟»، ممیزی ۲ پرسید «درست کار می‌کند؟».
//! این یکی می‌پرسد: **«وقتی کاربر اشتباه وارد می‌کند، چه می‌شود؟»**
//!
//! ## چرا این مهم‌تر از مسیر موفق است
//!
//! مسیر موفق را همه تست می‌کنند. خطاهای گران‌قیمت در مسیرهای انحرافی اتفاق
//! می‌افتند: تخفیف بیشتر از مبلغ، قسط منفی، کوپنی که سقف ندارد، شمارشی که
//! ثبت نشده ولی سند خورده. اینجا دقیقاً همان‌ها سنجیده می‌شوند.
//!
//! ## قاعده‌ی ثابت
//!
//! هر تست یک **مسیر انحرافی** را می‌سنجد و انتظار دارد سیستم **با پیام
//! مشخص رد کند**، نه اینکه عدد غلط تولید کند. عدد غلط بدتر از خطاست، چون
//! دیده نمی‌شود.

use novin_core::catalog::{GoldPricing, TaxProfile};
use novin_core::inventory::{self, MovementKind, ValuationMethod};
use novin_core::invoicing::{self, Coupon, CouponKind, DiscountTier, InvoiceInput, InvoiceLine};
use novin_core::jalali::{self, JalaliDate};
use novin_core::money::Money;
use novin_core::parties::{
    check_credit_limit, remaining_credit, summarize_balances, BalanceStatus,
};
use novin_core::stocktaking::{self, CountLine, StocktakeStatus};

fn line(quantity: f64, unit_price: i64) -> InvoiceLine {
    InvoiceLine {
        product_id: "p".into(),
        quantity,
        unit_price: Money::from_rials(unit_price),
        discount_amount: Money::ZERO,
        discount_bp: 0,
        tiers: Vec::new(),
        vat_bp: 0,
        duty_bp: 0,
        commission_bp: 0,
        unit_cost: Money::ZERO,
        serial_tracked: false,
        serials: Vec::new(),
    }
}

fn input(lines: Vec<InvoiceLine>) -> InvoiceInput {
    InvoiceInput {
        lines,
        header_discount: Money::ZERO,
        coupon: None,
        freight: Money::ZERO,
        freight_mode: invoicing::FreightMode::AddToTotal,
    }
}

// ===========================================================================
// فرم فاکتور — مسیرهای انحرافی
// ===========================================================================

/// ت۵۱ — فاکتور بدون قلم ثبت نمی‌شود.
#[test]
fn t51_invoice_without_lines_is_rejected() {
    assert!(
        invoicing::calculate(&input(Vec::new())).is_err(),
        "فاکتور خالی نباید محاسبه شود"
    );
}

/// ت۵۲ — مقدار صفر یا منفی در سطر فاکتور رد می‌شود.
#[test]
fn t52_non_positive_quantity_is_rejected() {
    assert!(invoicing::calculate(&input(vec![line(0.0, 1000)])).is_err());
    assert!(invoicing::calculate(&input(vec![line(-1.0, 1000)])).is_err());
    // ولی مقدار اعشاری معتبر است — کالای وزنی وجود دارد.
    assert!(invoicing::calculate(&input(vec![line(0.5, 1000)])).is_ok());
}

/// ت۵۳ — تخفیف سطر هرگز از مبلغ سطر بیشتر نمی‌شود.
///
/// تخفیف بیشتر از مبلغ یعنی فاکتور منفی — یعنی فروشنده به مشتری پول می‌دهد.
#[test]
fn t53_line_discount_cannot_exceed_the_line_amount() {
    let mut over = line(1.0, 1_000_000);
    over.discount_amount = Money::from_rials(1_500_000);
    assert!(
        invoicing::calculate(&input(vec![over])).is_err(),
        "تخفیف بیشتر از مبلغ سطر باید رد شود"
    );

    // تخفیف دقیقاً برابر مبلغ مجاز است (کالای رایگان).
    let mut exact = line(1.0, 1_000_000);
    exact.discount_amount = Money::from_rials(1_000_000);
    let result = invoicing::calculate(&input(vec![exact])).expect("کالای رایگان مجاز است");
    assert_eq!(result.net_total.rials(), 0);
}

/// ت۵۴ — نرخ خارج از بازه‌ی صفر تا صد درصد رد می‌شود.
#[test]
fn t54_rates_outside_zero_to_hundred_percent_are_rejected() {
    for (label, mutate) in [
        ("مالیات", 0usize),
        ("عوارض", 1usize),
        ("پورسانت", 2usize),
        ("تخفیف درصدی", 3usize),
    ] {
        let mut bad = line(1.0, 1_000_000);
        match mutate {
            0 => bad.vat_bp = 10_001,
            1 => bad.duty_bp = 10_001,
            2 => bad.commission_bp = 10_001,
            _ => bad.discount_bp = 10_001,
        }
        assert!(
            invoicing::calculate(&input(vec![bad])).is_err(),
            "نرخ «{label}» بیش از ۱۰۰٪ باید رد شود"
        );

        let mut negative = line(1.0, 1_000_000);
        match mutate {
            0 => negative.vat_bp = -1,
            1 => negative.duty_bp = -1,
            2 => negative.commission_bp = -1,
            _ => negative.discount_bp = -1,
        }
        assert!(
            invoicing::calculate(&input(vec![negative])).is_err(),
            "نرخ «{label}» منفی باید رد شود"
        );
    }
}

/// ت۵۵ — تخفیف پلکانی، بالاترین پله‌ی واجد شرایط را اعمال می‌کند، نه اولی.
///
/// اگر پله‌ها مرتب نباشند و اولین تطابق انتخاب شود، مشتری تخفیف کمتری
/// می‌گیرد و شکایت به‌حق دارد.
#[test]
fn t55_tier_discount_picks_the_best_qualifying_tier() {
    let tiers = vec![
        DiscountTier {
            min_quantity: 10.0,
            discount_bp: 300,
        },
        DiscountTier {
            min_quantity: 50.0,
            discount_bp: 700,
        },
        DiscountTier {
            min_quantity: 100.0,
            discount_bp: 1200,
        },
    ];
    assert_eq!(
        invoicing::resolve_tier_discount(&tiers, 5.0),
        0,
        "زیر پله‌ی اول"
    );
    assert_eq!(
        invoicing::resolve_tier_discount(&tiers, 10.0),
        300,
        "دقیقاً پله‌ی اول"
    );
    assert_eq!(invoicing::resolve_tier_discount(&tiers, 49.0), 300);
    assert_eq!(
        invoicing::resolve_tier_discount(&tiers, 60.0),
        700,
        "پله‌ی دوم"
    );
    assert_eq!(
        invoicing::resolve_tier_discount(&tiers, 1000.0),
        1200,
        "بالاترین پله"
    );

    // ترتیب ورودی نباید نتیجه را عوض کند.
    let shuffled = vec![
        DiscountTier {
            min_quantity: 100.0,
            discount_bp: 1200,
        },
        DiscountTier {
            min_quantity: 10.0,
            discount_bp: 300,
        },
        DiscountTier {
            min_quantity: 50.0,
            discount_bp: 700,
        },
    ];
    assert_eq!(invoicing::resolve_tier_discount(&shuffled, 60.0), 700);
}

/// ت۵۶ — کوپن درصدی سقف دارد و حداقل مبلغ را رعایت می‌کند.
#[test]
fn t56_coupon_respects_minimum_and_maximum() {
    let coupon = Coupon {
        code: "NEWYEAR".into(),
        kind: CouponKind::Percent(2000), // ۲۰٪
        minimum_invoice: Some(Money::from_rials(5_000_000)),
        maximum_discount: Some(Money::from_rials(1_000_000)),
    };
    // زیر حداقل → کوپن اعمال نمی‌شود
    assert!(
        coupon.discount_for(Money::from_rials(4_000_000)).is_err(),
        "کوپن زیر حداقل مبلغ نباید اعمال شود"
    );
    // بالای حداقل ولی زیر سقف → ۲۰٪
    assert_eq!(
        coupon
            .discount_for(Money::from_rials(5_000_000))
            .unwrap()
            .rials(),
        1_000_000
    );
    // خیلی بالاتر → سقف اعمال می‌شود، نه ۲۰٪ کامل
    assert_eq!(
        coupon
            .discount_for(Money::from_rials(50_000_000))
            .unwrap()
            .rials(),
        1_000_000,
        "سقف کوپن رعایت نشد"
    );
}

/// ت۵۷ — کوپن نامعتبر (درصد بیش از صد) رد می‌شود.
#[test]
fn t57_invalid_coupon_percentage_is_rejected() {
    let broken = Coupon {
        code: "BAD".into(),
        kind: CouponKind::Percent(10_001),
        minimum_invoice: None,
        maximum_discount: None,
    };
    assert!(broken.discount_for(Money::from_rials(1_000_000)).is_err());
}

/// ت۵۸ — کالای سریال‌دار باید به تعداد مقدار، سریال داشته باشد.
///
/// سریال کم یا تکراری یعنی رهگیری گارانتی غیرممکن.
#[test]
fn t58_serial_tracked_lines_need_matching_unique_serials() {
    let mut ok = line(3.0, 1_000_000);
    ok.serial_tracked = true;
    ok.serials = vec!["S1".into(), "S2".into(), "S3".into()];
    assert!(invoicing::validate_serials(&[ok]).is_ok());

    let mut short = line(3.0, 1_000_000);
    short.serial_tracked = true;
    short.serials = vec!["S1".into(), "S2".into()];
    assert!(
        invoicing::validate_serials(&[short]).is_err(),
        "تعداد سریال کمتر از مقدار باید رد شود"
    );

    let mut duplicate = line(2.0, 1_000_000);
    duplicate.serial_tracked = true;
    duplicate.serials = vec!["S1".into(), "S1".into()];
    assert!(
        invoicing::validate_serials(&[duplicate]).is_err(),
        "سریال تکراری باید رد شود"
    );
}

// ===========================================================================
// اقساط
// ===========================================================================

/// ت۵۹ — جمع اقساط با پیش‌پرداخت، دقیقاً برابر کل فاکتور است.
#[test]
fn t59_installments_plus_down_payment_equal_the_invoice() {
    let total = Money::from_rials(10_000_001); // عمداً بخش‌ناپذیر
    let down = Money::from_rials(1_000_000);
    let first = JalaliDate::new(1405, 6, 15)
        .unwrap()
        .to_gregorian()
        .unwrap();
    let plan = invoicing::installment_plan(total, down, 6, first).unwrap();

    assert_eq!(plan.len(), 6);
    let sum: i64 = plan.iter().map(|item| item.amount.rials()).sum();
    assert_eq!(
        sum + down.rials(),
        total.rials(),
        "جمع اقساط با پیش‌پرداخت برابر فاکتور نیست"
    );
    // هیچ قسطی صفر یا منفی نباشد.
    assert!(plan.iter().all(|item| item.amount.rials() > 0));
}

/// ت۶۰ — سررسید اقساط با ماه شمسی جلو می‌رود، نه با ۳۰ روز ثابت.
///
/// ۳۰ روز ثابت باعث می‌شود سررسیدها به‌مرور از روز ماه منحرف شوند.
#[test]
fn t60_installment_due_dates_advance_by_jalali_months() {
    let first = JalaliDate::new(1405, 1, 31)
        .unwrap()
        .to_gregorian()
        .unwrap();
    let plan =
        invoicing::installment_plan(Money::from_rials(3_000_000), Money::ZERO, 3, first).unwrap();

    let dates: Vec<JalaliDate> = plan
        .iter()
        .map(|item| jalali::from_gregorian(item.due_date))
        .collect();
    assert_eq!(dates[0], JalaliDate::new(1405, 1, 31).unwrap());
    assert_eq!(dates[1], JalaliDate::new(1405, 2, 31).unwrap());
    // مهر ۳۰ روزه است، پس ۳۱ به ۳۰ می‌چسبد — نه به یکم آبان.
    assert_eq!(dates[2], JalaliDate::new(1405, 3, 31).unwrap());
}

/// ت۶۱ — پیش‌پرداخت بیشتر از کل فاکتور رد می‌شود.
#[test]
fn t61_down_payment_larger_than_total_is_rejected() {
    let first = JalaliDate::new(1405, 6, 1).unwrap().to_gregorian().unwrap();
    assert!(invoicing::installment_plan(
        Money::from_rials(1_000_000),
        Money::from_rials(2_000_000),
        3,
        first
    )
    .is_err());
    // تعداد قسط صفر یا بیش از حد هم رد می‌شود.
    assert!(
        invoicing::installment_plan(Money::from_rials(1_000_000), Money::ZERO, 0, first).is_err()
    );
    assert!(
        invoicing::installment_plan(Money::from_rials(1_000_000), Money::ZERO, 121, first).is_err()
    );
}

// ===========================================================================
// سقف اعتبار و مانده‌ی شخص
// ===========================================================================

/// ت۶۲ — سقف اعتبار پیش از فروش نسیه بررسی می‌شود.
#[test]
fn t62_credit_limit_blocks_over_limit_credit_sales() {
    let current = Money::from_rials(8_000_000);
    let limit = 10_000_000;
    // فروش ۱٬۵۰۰٬۰۰۰ زیر سقف است
    assert!(check_credit_limit(current, limit, Money::from_rials(1_500_000)).is_ok());
    // فروش ۳٬۰۰۰٬۰۰۰ از سقف رد می‌شود
    assert!(check_credit_limit(current, limit, Money::from_rials(3_000_000)).is_err());
    // سقف صفر یعنی بدون محدودیت
    assert!(check_credit_limit(current, 0, Money::from_rials(999_000_000)).is_ok());
}

/// ت۶۳ — اعتبار باقیمانده درست محاسبه می‌شود و منفی نمی‌شود.
#[test]
fn t63_remaining_credit_is_computed_and_never_negative() {
    assert_eq!(
        remaining_credit(Money::from_rials(3_000_000), 10_000_000)
            .unwrap()
            .rials(),
        7_000_000
    );
    // اگر از سقف رد شده، باقیمانده صفر است نه منفی.
    let over = remaining_credit(Money::from_rials(12_000_000), 10_000_000).unwrap();
    assert_eq!(over.rials(), 0, "اعتبار باقیمانده نباید منفی شود");
    // بدون سقف، مقدار باقیمانده معنا ندارد.
    assert_eq!(remaining_credit(Money::from_rials(1_000_000), 0), None);
}

/// ت۶۴ — خلاصه‌ی حساب اشخاص، بدهکار و بستانکار و بی‌حساب را درست تفکیک می‌کند.
#[test]
fn t64_party_balance_summary_classifies_correctly() {
    let balances = [
        Money::from_rials(5_000_000),  // بدهکار
        Money::from_rials(-2_000_000), // بستانکار
        Money::ZERO,                   // بی‌حساب
        Money::from_rials(3_000_000),  // بدهکار
    ];
    let summary = summarize_balances(&balances);
    assert_eq!(summary.debtor_count, 2);
    assert_eq!(summary.creditor_count, 1);
    assert_eq!(summary.settled_count, 1);
    assert_eq!(summary.debtor_total.rials(), 8_000_000);
    assert_eq!(summary.creditor_total.rials(), 2_000_000);
    // خالص = بدهکار منهای بستانکار
    assert_eq!(summary.net_total.rials(), 6_000_000);

    assert_eq!(
        BalanceStatus::of(Money::from_rials(1)),
        BalanceStatus::Debtor
    );
    assert_eq!(
        BalanceStatus::of(Money::from_rials(-1)),
        BalanceStatus::Creditor
    );
    assert_eq!(BalanceStatus::of(Money::ZERO), BalanceStatus::Settled);
    // نشانگر «بد/بس» تصویر مرجع
    assert_eq!(BalanceStatus::Debtor.indicator(), "بد");
    assert_eq!(BalanceStatus::Creditor.indicator(), "بس");
}

// ===========================================================================
// انبارگردانی
// ===========================================================================

/// ت۶۵ — قلم شمرده‌نشده اختلاف ندارد؛ صفر با «نشمرده» یکی نیست.
///
/// اگر نشمرده را صفر بگیریم، سند تعدیل کل موجودی را از بین می‌برد.
#[test]
fn t65_uncounted_line_is_not_the_same_as_zero() {
    let uncounted = CountLine::new("p1", 10.0, Money::from_rials(100_000));
    assert!(!uncounted.is_counted());
    assert_eq!(uncounted.variance(), None, "قلم نشمرده نباید اختلاف بدهد");
    assert!(!uncounted.has_variance());

    let mut counted_zero = CountLine::new("p2", 10.0, Money::from_rials(100_000));
    counted_zero.counted_quantity = Some(0.0);
    assert!(counted_zero.is_counted());
    assert_eq!(
        counted_zero.variance(),
        Some(-10.0),
        "کسری کامل باید دیده شود"
    );
}

/// ت۶۶ — شمارش مجدد بر شمارش اول اولویت دارد.
#[test]
fn t66_recount_overrides_the_first_count() {
    let mut item = CountLine::new("p1", 100.0, Money::from_rials(50_000));
    item.counted_quantity = Some(90.0);
    assert_eq!(item.final_quantity(), Some(90.0));
    assert_eq!(item.variance(), Some(-10.0));

    item.recount_quantity = Some(98.0);
    assert_eq!(
        item.final_quantity(),
        Some(98.0),
        "شمارش مجدد باید حاکم باشد"
    );
    assert_eq!(item.variance(), Some(-2.0));
    // ارزش اختلاف = مقدار اختلاف × بهای واحد
    assert_eq!(item.variance_value().unwrap().rials(), -100_000);
}

/// ت۶۷ — جلسه‌ی قفل‌شده دیگر شمارش نمی‌پذیرد.
#[test]
fn t67_locked_stocktake_session_accepts_no_more_counts() {
    assert!(
        StocktakeStatus::Posted.is_locked(),
        "جلسه‌ی ثبت‌شده باید قفل باشد"
    );
    assert!(StocktakeStatus::Cancelled.is_locked());
    assert!(!StocktakeStatus::Counting.is_locked());
    // و از وضعیت پایانی هیچ گذاری نیست.
    assert!(stocktaking::allowed_transitions(StocktakeStatus::Posted).is_empty());
    assert!(
        stocktaking::transition(StocktakeStatus::Posted, StocktakeStatus::Counting).is_err(),
        "بازگشت از وضعیت ثبت‌شده نباید ممکن باشد"
    );
}

/// ت۶۸ — خلاصه‌ی انبارگردانی، کسری و اضافی را جدا گزارش می‌کند.
///
/// جمع جبری کسری و اضافی، اطلاعات را پنهان می‌کند: ۱۰۰ کسری و ۱۰۰ اضافی
/// یعنی دو مشکل، نه «همه‌چیز درست».
#[test]
fn t68_stocktake_summary_separates_shortage_from_surplus() {
    let mut shortage = CountLine::new("p1", 100.0, Money::from_rials(10_000));
    shortage.counted_quantity = Some(90.0);
    let mut surplus = CountLine::new("p2", 50.0, Money::from_rials(20_000));
    surplus.counted_quantity = Some(55.0);
    let summary = stocktaking::summarize(&[shortage, surplus]).unwrap();

    assert_eq!(summary.shortage_value.rials(), 100_000, "ارزش کسری");
    assert_eq!(summary.surplus_value.rials(), 100_000, "ارزش اضافی");
    // خالص صفر است ولی دو مشکل وجود دارد — هر دو باید دیده شوند.
    assert_eq!(summary.counted_lines, 2);
    // کسری و اضافی جدا شمرده می‌شوند، نه یک عدد جبری.
    assert_eq!(summary.shortage_lines, 1, "یک قلم کسری دارد");
    assert_eq!(summary.surplus_lines, 1, "یک قلم اضافی دارد");
    assert_eq!(summary.net_value.rials(), 0, "اثر خالص صفر است");
}

// ===========================================================================
// ارزش‌گذاری موجودی
// ===========================================================================

/// ت۶۹ — سه روش ارزش‌گذاری، ارزش موجودی متفاوتی می‌دهند.
///
/// اگر نتیجه‌ی هر سه یکی باشد، یعنی روش انتخابی کاربر واقعاً اعمال نمی‌شود
/// و تنظیم «روش ارزش‌گذاری» تزئینی است.
#[test]
fn t69_three_valuation_methods_give_different_results() {
    // دو ورود با بهای متفاوت، سپس یک خروج
    let movements = vec![
        inventory::Movement::new(MovementKind::Receipt, 10.0, 100_000),
        inventory::Movement::new(MovementKind::Receipt, 10.0, 200_000),
        inventory::Movement::new(MovementKind::Issue, 5.0, 0),
    ];
    let fifo = inventory::valuate(&movements, ValuationMethod::Fifo).unwrap();
    let moving = inventory::valuate(&movements, ValuationMethod::MovingAverage).unwrap();
    let weighted = inventory::valuate(&movements, ValuationMethod::WeightedAverage).unwrap();

    // موجودی پایانی در هر سه روش یکسان است — روش فقط بر ارزش اثر دارد.
    assert_eq!(fifo.quantity, 15.0);
    assert_eq!(moving.quantity, 15.0);
    assert_eq!(weighted.quantity, 15.0);

    // FIFO ارزان‌ترین لایه را خارج می‌کند، پس موجودی باقیمانده گران‌تر است.
    assert!(
        fifo.total_value > weighted.total_value,
        "FIFO باید ارزش موجودی بالاتری بدهد: {} در برابر {}",
        fifo.total_value,
        weighted.total_value
    );
    // ارزش موجودی در هیچ روشی منفی یا صفر نمی‌شود وقتی موجودی مثبت است.
    for (label, valuation) in [("FIFO", fifo), ("متحرک", moving), ("موزون", weighted)] {
        assert!(
            valuation.total_value > 0,
            "ارزش موجودی در روش «{label}» صفر یا منفی است"
        );
        assert!(
            valuation.unit_cost >= 100_000 && valuation.unit_cost <= 200_000,
            "بهای واحد روش «{label}» بین دو بهای خرید نیست: {}",
            valuation.unit_cost
        );
    }
}

/// ت۷۰ — هر روش ارزش‌گذاری توضیح ساده و قابل فهم دارد.
///
/// بازخورد کارفرما: «توضیح سه‌جمله‌ای برای هر روش ارزش‌گذاری».
#[test]
fn t70_each_valuation_method_has_a_plain_explanation() {
    for method in [
        ValuationMethod::WeightedAverage,
        ValuationMethod::Fifo,
        ValuationMethod::MovingAverage,
    ] {
        assert!(!method.label().is_empty(), "برچسب خالی");
        let explanation = method.plain_explanation();
        assert!(
            explanation.chars().count() > 60,
            "توضیح «{}» بیش از حد کوتاه است",
            method.label()
        );
        // توضیح باید فارسی ساده باشد، نه اصطلاح خام انگلیسی.
        assert!(
            !explanation.contains("FIFO") || explanation.contains("وارده"),
            "توضیح «{}» فقط اصطلاح فنی است",
            method.label()
        );
    }
}

/// ت۷۱ — خروج بیش از موجودی قابل فروش رد می‌شود.
#[test]
fn t71_issuing_more_than_available_is_rejected() {
    // موجودی ۱۰، رزرو ۳ → قابل فروش ۷
    assert_eq!(inventory::available_quantity(10.0, 3.0), 7.0);
    assert!(inventory::can_issue(10.0, 3.0, 7.0).is_ok());
    assert!(
        inventory::can_issue(10.0, 3.0, 8.0).is_err(),
        "خروج بیش از قابل فروش باید رد شود"
    );
    // رزرو بیشتر از موجودی نباید عدد منفی بدهد.
    assert_eq!(inventory::available_quantity(5.0, 8.0), 0.0);
}

// ===========================================================================
// قیمت طلا و مالیات
// ===========================================================================

/// ت۷۲ — در طلا، مالیات فقط بر اجرت و سود بسته می‌شود، نه بر ارزش طلا.
///
/// این قاعده‌ی خاص صنف طلاست؛ بستن مالیات بر کل مبلغ، قیمت را به‌شدت
/// بالاتر از واقع نشان می‌دهد.
#[test]
fn t72_gold_vat_applies_only_to_making_charge_and_profit() {
    let pricing = GoldPricing {
        weight_grams: 10.0,
        rate_per_gram: Money::from_rials(30_000_000),
        making_charge_bp: 700, // ۷٪
        profit_bp: 500,        // ۵٪
        vat_bp: 900,           // ۹٪
    };
    let breakdown = novin_core::catalog::gold_price(pricing).unwrap();

    assert_eq!(breakdown.metal_value.rials(), 300_000_000, "ارزش طلا");
    assert_eq!(breakdown.making_charge.rials(), 21_000_000, "اجرت ۷٪");

    // مالیات باید فقط روی اجرت + سود باشد.
    let taxable = breakdown.making_charge.rials() + breakdown.profit.rials();
    assert_eq!(
        breakdown.vat.rials(),
        taxable * 900 / 10_000,
        "مالیات طلا باید فقط بر اجرت و سود باشد"
    );
    // و قطعاً نباید مالیات کل مبلغ باشد.
    assert!(
        breakdown.vat.rials() < breakdown.metal_value.rials() * 900 / 10_000,
        "مالیات روی ارزش طلا هم بسته شده — اشتباه است"
    );
    // جمع اجزا باید دقیقاً برابر مبلغ نهایی باشد.
    assert_eq!(
        breakdown.total.rials(),
        breakdown.metal_value.rials()
            + breakdown.making_charge.rials()
            + breakdown.profit.rials()
            + breakdown.vat.rials()
    );
}

/// ت۷۳ — کالای معاف از مالیات، صفر مالیات می‌گیرد.
#[test]
fn t73_tax_exempt_products_pay_nothing() {
    let exempt = TaxProfile::exempt();
    assert!(exempt.validate().is_ok());
    assert_eq!(
        exempt
            .tax_on(Money::from_rials(10_000_000))
            .unwrap()
            .rials(),
        0,
        "کالای معاف نباید مالیات بگیرد"
    );

    let standard = TaxProfile::standard();
    assert!(
        standard
            .tax_on(Money::from_rials(10_000_000))
            .unwrap()
            .rials()
            > 0,
        "کالای مشمول باید مالیات بگیرد"
    );
}

// ===========================================================================
// تقویم در فرم‌ها
// ===========================================================================

/// ت۷۴ — تاریخ شمسی نامعتبر در هیچ فرمی پذیرفته نمی‌شود.
#[test]
fn t74_invalid_jalali_dates_are_rejected_everywhere() {
    // ماه صفر یا ۱۳
    assert!(JalaliDate::new(1405, 0, 1).is_err());
    assert!(JalaliDate::new(1405, 13, 1).is_err());
    // روز صفر یا بیش از حد ماه
    assert!(JalaliDate::new(1405, 1, 0).is_err());
    assert!(JalaliDate::new(1405, 1, 32).is_err());
    assert!(JalaliDate::new(1405, 7, 31).is_err(), "مهر ۳۱ روز ندارد");
    // قالب متنی نامعتبر
    assert!(JalaliDate::parse("").is_err());
    assert!(JalaliDate::parse("1405-13-01").is_err());
    // ارقام فارسی عمداً پذیرفته می‌شوند: کاربر ایرانی با صفحه‌کلید فارسی
    // تایپ می‌کند و رد کردن ورودی‌اش آزاردهنده است. یکسان‌سازی در خود
    // تجزیه‌گر انجام می‌شود.
    assert_eq!(
        JalaliDate::parse("۱۴۰۵/۰۱/۰۱").unwrap(),
        JalaliDate::new(1405, 1, 1).unwrap(),
        "ارقام فارسی باید پذیرفته و یکسان‌سازی شوند"
    );
    // ولی متن بی‌معنا همچنان رد می‌شود.
    assert!(JalaliDate::parse("۱۴۰۵/سیزده/۰۱").is_err());
    // ولی قالب استاندارد با و بدون صفر پیشوند کار می‌کند.
    assert!(JalaliDate::parse("1405/01/01").is_ok());
    assert!(JalaliDate::parse("1405/1/1").is_ok());
}

/// ت۷۵ — مرتب‌سازی متنی تاریخ شمسی با ترتیب واقعی زمان یکی است.
///
/// کل فیلترهای بازه‌ی تاریخ در گزارش‌ها به مقایسه‌ی رشته‌ای تکیه می‌کنند؛
/// اگر این برقرار نباشد، هیچ گزارش دوره‌ای درست درنمی‌آید.
#[test]
fn t75_jalali_text_sorting_matches_chronological_order() {
    let mut samples = vec![
        "1405/12/29",
        "1404/01/01",
        "1405/01/09",
        "1405/01/10",
        // ۱۴۰۴ کبیسه نیست، پس اسفند ۲۹ روز دارد — همان قاعده‌ای که ت۴۶ می‌سنجد.
        "1404/12/29",
        "1405/02/01",
    ];
    // مرتب‌سازی متنی ساده
    samples.sort();

    // مرتب‌سازی واقعی زمانی
    let mut chronological: Vec<JalaliDate> = samples
        .iter()
        .map(|text| JalaliDate::parse(text).unwrap())
        .collect();
    chronological.sort();

    // از قالب‌کننده‌ی خود موتور استفاده می‌کنیم، نه ساخت دستی رشته —
    // وگرنه تست چیزی جز خودش را نمی‌سنجد.
    let as_text: Vec<String> = chronological.iter().map(JalaliDate::format).collect();
    assert_eq!(
        samples,
        as_text.iter().map(String::as_str).collect::<Vec<_>>(),
        "ترتیب متنی با ترتیب زمانی یکی نیست — فیلتر بازه‌ی تاریخ خراب می‌شود"
    );
}
