//! فاز ۱۶ — تولید، فرمول تولید و بهای تمام‌شده.
//!
//! مرجع: تصویر `3qTCnS` (رسید تولید).
//!
//! ## معادله‌ی محوری
//!
//! ```text
//! جمع مواد مصرفی + جمع هزینه‌های تولید = جمع بهای تمام‌شده‌ی محصولات
//! ```
//!
//! اگر این معادله برقرار نباشد، یا ارزشی از هوا ساخته شده یا ارزشی ناپدید
//! شده — و هر دو در صورت‌های مالی به سود یا زیان ساختگی تبدیل می‌شوند.
//!
//! **تولید سود نمی‌سازد.** فقط شکل دارایی از «مواد اولیه» به «کالای
//! ساخته‌شده» تغییر می‌کند. سود در لحظه‌ی فروش محقق می‌شود.

use novin_core::money::Money;
use novin_core::production::{
    assert_cost_balance, calculate_costing, producible_quantity, ConsumedMaterial, CostAllocation,
    FormulaComponent, ProducedItem, ProductionError, ProductionExpense, ProductionFormula,
};

fn material(product: &str, quantity: f64, unit_cost: i64) -> ConsumedMaterial {
    ConsumedMaterial {
        product_id: product.into(),
        quantity,
        unit_cost: Money::from_rials(unit_cost),
    }
}

fn output(product: &str, quantity: f64) -> ProducedItem {
    ProducedItem {
        product_id: product.into(),
        quantity,
        market_unit_price: None,
    }
}

fn expense(amount: i64) -> ProductionExpense {
    ProductionExpense {
        account_id: "acc-5300".into(),
        title: "دستمزد".into(),
        amount: Money::from_rials(amount),
    }
}

/// ت۰۱ — معادله‌ی بهای تمام‌شده دقیقاً برقرار است.
#[test]
fn t01_cost_equation_holds_exactly() {
    let materials = vec![
        material("mat-a", 10.0, 500_000),  // ۵٬۰۰۰٬۰۰۰
        material("mat-b", 3.0, 1_200_000), // ۳٬۶۰۰٬۰۰۰
    ];
    let expenses = vec![expense(1_400_000)];
    let outputs = vec![output("product-x", 20.0)];

    let costing =
        calculate_costing(&materials, &expenses, &outputs, CostAllocation::ByQuantity).unwrap();

    assert_eq!(costing.materials_total.rials(), 8_600_000);
    assert_eq!(costing.expenses_total.rials(), 1_400_000);
    assert_eq!(costing.total_cost.rials(), 10_000_000);
    assert_eq!(costing.outputs[0].allocated_cost.rials(), 10_000_000);
    assert_eq!(costing.outputs[0].unit_cost.rials(), 500_000);
    assert!(assert_cost_balance(&costing).is_ok());
}

/// ت۰۲ — حتی یک ریال هم در تخصیص بین محصولات گم نمی‌شود.
///
/// مبلغی که بر تعداد محصولات بخش‌پذیر نیست، بحرانی‌ترین حالت است.
#[test]
fn t02_allocation_loses_not_a_single_rial() {
    let materials = vec![material("mat-a", 1.0, 1_000_001)];
    let outputs = vec![
        output("p1", 1.0),
        output("p2", 1.0),
        output("p3", 1.0),
    ];
    let costing = calculate_costing(&materials, &[], &outputs, CostAllocation::ByQuantity).unwrap();

    let sum: i64 = costing
        .outputs
        .iter()
        .map(|item| item.allocated_cost.rials())
        .sum();
    assert_eq!(sum, 1_000_001, "جمع سهم‌ها با کل برابر نیست");
    assert!(assert_cost_balance(&costing).is_ok());
}

/// ت۰۳ — تخصیص بر اساس ارزش بازار، حاشیه‌ی سود را برابر می‌کند.
///
/// محصول اصلی و فرعی نباید بهای یکسان بگیرند؛ محصولی که گران‌تر می‌فروشد
/// باید سهم بیشتری از بهای تمام‌شده بردارد.
#[test]
fn t03_market_value_allocation_matches_selling_value() {
    let materials = vec![material("mat", 1.0, 12_000_000)];
    let outputs = vec![
        ProducedItem {
            product_id: "main".into(),
            quantity: 1.0,
            market_unit_price: Some(Money::from_rials(9_000_000)),
        },
        ProducedItem {
            product_id: "byproduct".into(),
            quantity: 1.0,
            market_unit_price: Some(Money::from_rials(3_000_000)),
        },
    ];
    let costing =
        calculate_costing(&materials, &[], &outputs, CostAllocation::ByMarketValue).unwrap();

    // نسبت ارزش ۳ به ۱ → سهم بها هم باید ۳ به ۱ باشد.
    assert_eq!(costing.outputs[0].allocated_cost.rials(), 9_000_000);
    assert_eq!(costing.outputs[1].allocated_cost.rials(), 3_000_000);

    // با تخصیص بر اساس مقدار، هر دو ۶ میلیون می‌گرفتند — یعنی محصول فرعی
    // با زیان و محصول اصلی با سود کاذب ثبت می‌شد.
    let by_quantity =
        calculate_costing(&materials, &[], &outputs, CostAllocation::ByQuantity).unwrap();
    assert_eq!(by_quantity.outputs[0].allocated_cost.rials(), 6_000_000);
    assert_ne!(
        by_quantity.outputs[0].allocated_cost,
        costing.outputs[0].allocated_cost
    );
}

/// ت۰۴ — محصول نمی‌تواند ماده‌ی مصرفی خودش باشد.
///
/// وگرنه بهای تمام‌شده‌اش به خودش وابسته می‌شود و محاسبه بی‌معنا است.
#[test]
fn t04_circular_production_is_rejected() {
    let materials = vec![material("same-product", 2.0, 100_000)];
    let outputs = vec![output("same-product", 1.0)];
    assert_eq!(
        calculate_costing(&materials, &[], &outputs, CostAllocation::ByQuantity),
        Err(ProductionError::CircularFormula {
            product: "same-product".into()
        })
    );

    let formula = ProductionFormula {
        product_id: "p".into(),
        title: "حلقه".into(),
        output_quantity: 1.0,
        components: vec![FormulaComponent {
            product_id: "p".into(),
            quantity_per_unit: 1.0,
            waste_percent: 0.0,
        }],
    };
    assert!(formula.validate().is_err(), "فرمول حلقه‌ای باید رد شود");
}

/// ت۰۵ — رسید بدون ماده یا بدون محصول ثبت نمی‌شود.
#[test]
fn t05_empty_inputs_or_outputs_are_rejected() {
    assert_eq!(
        calculate_costing(&[], &[], &[output("p", 1.0)], CostAllocation::ByQuantity),
        Err(ProductionError::NoInput)
    );
    assert_eq!(
        calculate_costing(
            &[material("m", 1.0, 100)],
            &[],
            &[],
            CostAllocation::ByQuantity
        ),
        Err(ProductionError::NoOutput)
    );
}

/// ت۰۶ — مقدار و مبلغ نامعتبر رد می‌شوند.
#[test]
fn t06_invalid_quantities_and_amounts_are_rejected() {
    // مقدار صفر در ماده
    assert!(calculate_costing(
        &[material("m", 0.0, 1000)],
        &[],
        &[output("p", 1.0)],
        CostAllocation::ByQuantity
    )
    .is_err());
    // مقدار منفی در محصول
    assert!(calculate_costing(
        &[material("m", 1.0, 1000)],
        &[],
        &[output("p", -1.0)],
        CostAllocation::ByQuantity
    )
    .is_err());
    // بهای منفی
    assert!(calculate_costing(
        &[material("m", 1.0, -1)],
        &[],
        &[output("p", 1.0)],
        CostAllocation::ByQuantity
    )
    .is_err());
    // هزینه‌ی منفی
    assert_eq!(
        calculate_costing(
            &[material("m", 1.0, 1000)],
            &[expense(-1)],
            &[output("p", 1.0)],
            CostAllocation::ByQuantity
        ),
        Err(ProductionError::NegativeExpense)
    );
}

/// ت۰۷ — ماده‌ی تکراری در یک رسید یا فرمول رد می‌شود.
///
/// دو سطر برای یک ماده یعنی احتمال زیاد اشتباه کاربر؛ و در فرمول، نسبت مصرف
/// را مبهم می‌کند.
#[test]
fn t07_duplicate_components_are_rejected() {
    let materials = vec![material("mat-a", 1.0, 100), material("mat-a", 2.0, 100)];
    assert_eq!(
        calculate_costing(
            &materials,
            &[],
            &[output("p", 1.0)],
            CostAllocation::ByQuantity
        ),
        Err(ProductionError::DuplicateComponent {
            product: "mat-a".into()
        })
    );
}

/// ت۰۸ — ضایعات مصرف واقعی را افزایش می‌دهد.
///
/// ماده‌ای که ضایع می‌شود هم پول شرکت را مصرف کرده؛ نادیده‌گرفتنش بهای
/// تمام‌شده را کمتر از واقع نشان می‌دهد.
#[test]
fn t08_waste_increases_effective_consumption() {
    let component = FormulaComponent {
        product_id: "cloth".into(),
        quantity_per_unit: 2.0,
        waste_percent: 10.0,
    };
    // برای ۵ واحد محصول: ۱۰ متر پایه + ۱۰٪ ضایعات = ۱۱ متر
    let required = component.required_for(5.0).unwrap();
    assert!((required - 11.0).abs() < 1e-9, "مصرف با ضایعات: {required}");

    let no_waste = FormulaComponent {
        product_id: "cloth".into(),
        quantity_per_unit: 2.0,
        waste_percent: 0.0,
    };
    assert!((no_waste.required_for(5.0).unwrap() - 10.0).abs() < 1e-9);

    // ضایعات ۱۰۰٪ یا بیشتر بی‌معناست.
    let impossible = ProductionFormula {
        product_id: "p".into(),
        title: "t".into(),
        output_quantity: 1.0,
        components: vec![FormulaComponent {
            product_id: "m".into(),
            quantity_per_unit: 1.0,
            waste_percent: 100.0,
        }],
    };
    assert!(impossible.validate().is_err(), "ضایعات ۱۰۰٪ باید رد شود");
}

/// ت۰۹ — گسترش فرمول برای تولید چند واحد درست کار می‌کند.
#[test]
fn t09_formula_expands_correctly_for_batches() {
    // فرمولی که با ۳ واحد محصول تعریف شده
    let formula = ProductionFormula {
        product_id: "cake".into(),
        title: "کیک".into(),
        output_quantity: 3.0,
        components: vec![
            FormulaComponent {
                product_id: "flour".into(),
                quantity_per_unit: 1.5,
                waste_percent: 0.0,
            },
            FormulaComponent {
                product_id: "sugar".into(),
                quantity_per_unit: 0.5,
                waste_percent: 20.0,
            },
        ],
    };
    // تولید ۶ واحد = دو برابر فرمول
    let expanded = formula.expand(6.0).unwrap();
    assert_eq!(expanded.len(), 2);
    assert!((expanded[0].1 - 9.0).abs() < 1e-9, "آرد: {}", expanded[0].1);
    // ۰٫۵ × ۶ = ۳ به‌علاوه ۲۰٪ ضایعات = ۳٫۶
    assert!((expanded[1].1 - 3.6).abs() < 1e-9, "شکر: {}", expanded[1].1);

    assert!(formula.expand(0.0).is_err(), "مقدار صفر باید رد شود");
}

/// ت۱۰ — ظرفیت تولید را کمیاب‌ترین ماده تعیین می‌کند.
///
/// گلوگاه تولید، ماده‌ای است که کمترین تعداد محصول را ممکن می‌کند — نه
/// میانگین موجودی‌ها.
#[test]
fn t10_producible_quantity_is_limited_by_the_scarcest_material() {
    let formula = ProductionFormula {
        product_id: "shirt".into(),
        title: "پیراهن".into(),
        output_quantity: 1.0,
        components: vec![
            FormulaComponent {
                product_id: "cloth".into(),
                quantity_per_unit: 2.0,
                waste_percent: 0.0,
            },
            FormulaComponent {
                product_id: "button".into(),
                quantity_per_unit: 6.0,
                waste_percent: 0.0,
            },
        ],
    };

    // پارچه برای ۵۰ پیراهن، دکمه فقط برای ۱۰ تا → گلوگاه دکمه است.
    let stock = vec![("cloth".to_string(), 100.0), ("button".to_string(), 60.0)];
    let producible = producible_quantity(&formula, &stock).unwrap();
    assert!((producible - 10.0).abs() < 1e-9, "ظرفیت: {producible}");

    // ماده‌ای که اصلاً موجود نیست، ظرفیت را صفر می‌کند.
    let missing = vec![("cloth".to_string(), 100.0)];
    assert_eq!(producible_quantity(&formula, &missing).unwrap(), 0.0);

    // ضایعات ظرفیت را کم می‌کند.
    let with_waste = ProductionFormula {
        components: vec![FormulaComponent {
            product_id: "cloth".into(),
            quantity_per_unit: 2.0,
            waste_percent: 25.0,
        }],
        ..formula.clone()
    };
    let limited = producible_quantity(&with_waste, &[("cloth".to_string(), 100.0)]).unwrap();
    assert!((limited - 40.0).abs() < 1e-9, "با ضایعات: {limited}");
}
