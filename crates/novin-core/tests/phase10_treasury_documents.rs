//! فاز ۱۰ — سند دریافت و پرداخت چندروشی.
//!
//! سند خزانه تنها جایی است که شش روش تسویه، چک، طرف حساب و سند حسابداری
//! هم‌زمان درگیر می‌شوند. اشتباه اینجا مستقیماً به ترازنامه سرایت می‌کند، پس
//! هر ده تست زیر یک قاعده‌ی حسابداری مشخص را می‌سنجد.
//!
//! قاعده‌ی محوری: **چک پول نیست.** چک دریافتی تا وصول نشود به صندوق نمی‌رود،
//! بلکه در «اسناد دریافتنی» می‌نشیند. تخفیف و تهاتر هم پول جابه‌جا نمی‌کنند.

use novin_core::money::Money;
use novin_core::treasury::{
    build_journal, calculate_totals, CheckDetails, DocumentKind, DocumentLine, PaymentMethod,
    TreasuryAccounts,
};

fn accounts() -> TreasuryAccounts {
    TreasuryAccounts {
        party_account: "acc-1201".into(),
        notes_receivable: "acc-1103".into(),
        notes_payable: "acc-2103".into(),
        discount_account: "acc-4400".into(),
    }
}

fn cash(amount: i64) -> DocumentLine {
    DocumentLine::new(PaymentMethod::Cash, Money::from_rials(amount)).with_account("treasury-cash-1")
}

fn check(amount: i64) -> DocumentLine {
    let mut line = DocumentLine::new(PaymentMethod::Check, Money::from_rials(amount));
    line.check = Some(CheckDetails {
        serial: "700123".into(),
        due_date: "1405/08/10".into(),
        bank_name: Some("بانک ملت".into()),
        sayad_id: None,
    });
    line
}

fn sum_debit(lines: &[novin_core::accounting::JournalLine]) -> i64 {
    lines.iter().map(|l| l.debit.rials()).sum()
}

fn sum_credit(lines: &[novin_core::accounting::JournalLine]) -> i64 {
    lines.iter().map(|l| l.credit.rials()).sum()
}

/// ت۰۱ — سند چندروشی همیشه متوازن است، هر ترکیبی از روش‌ها که باشد.
#[test]
fn t01_multi_method_document_is_always_balanced() {
    let mut discount = DocumentLine::new(PaymentMethod::Discount, Money::from_rials(500_000));
    discount.description = Some("تخفیف نقدی".into());
    let mut transfer =
        DocumentLine::new(PaymentMethod::BankTransfer, Money::from_rials(30_000_000));
    transfer.treasury_account = Some("treasury-bank-mellat".into());
    let offset = DocumentLine::new(PaymentMethod::Offset, Money::from_rials(2_000_000));

    let lines = vec![cash(10_000_000), check(25_000_000), transfer, discount, offset];
    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();

    assert_eq!(
        sum_debit(&journal),
        sum_credit(&journal),
        "سند دریافت چندروشی باید متوازن باشد"
    );
    assert_eq!(sum_debit(&journal), 67_500_000, "جمع سند اشتباه است");
}

/// ت۰۲ — چک دریافتی به صندوق نمی‌رود؛ به «اسناد دریافتنی» می‌نشیند.
///
/// این مهم‌ترین قاعده‌ی این فاز است: چک تا وصول نشود دارایی نقدی نیست.
#[test]
fn t02_received_check_lands_on_notes_receivable_not_cash() {
    let lines = vec![check(25_000_000)];
    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    let debit_line = journal.iter().find(|l| l.debit.rials() > 0).unwrap();
    assert_eq!(
        debit_line.account_id, "acc-1103",
        "چک دریافتی باید به اسناد دریافتنی بنشیند"
    );
    assert!(
        !journal.iter().any(|l| l.account_id.starts_with("treasury-")),
        "هیچ حساب صندوقی نباید در سند چک ظاهر شود"
    );
}

/// ت۰۳ — چک پرداختی به «اسناد پرداختنی» می‌رود، نه کاهش صندوق.
#[test]
fn t03_issued_check_lands_on_notes_payable() {
    let lines = vec![check(18_000_000)];
    let journal = build_journal(DocumentKind::Payment, &lines, &accounts()).unwrap();
    let credit_line = journal.iter().find(|l| l.credit.rials() > 0).unwrap();
    assert_eq!(
        credit_line.account_id, "acc-2103",
        "چک صادرشده باید به اسناد پرداختنی بنشیند"
    );
}

/// ت۰۴ — سند پرداخت دقیقاً معکوس سند دریافت است.
#[test]
fn t04_payment_mirrors_receipt() {
    let lines = vec![cash(12_000_000)];
    let receipt = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    let payment = build_journal(DocumentKind::Payment, &lines, &accounts()).unwrap();

    let receipt_cash = receipt
        .iter()
        .find(|l| l.account_id == "treasury-cash-1")
        .unwrap();
    let payment_cash = payment
        .iter()
        .find(|l| l.account_id == "treasury-cash-1")
        .unwrap();
    assert_eq!(receipt_cash.debit.rials(), 12_000_000, "دریافت: صندوق بدهکار");
    assert_eq!(
        payment_cash.credit.rials(),
        12_000_000,
        "پرداخت: صندوق بستانکار"
    );
}

/// ت۰۵ — تخفیف و تهاتر پول جابه‌جا نمی‌کنند و نباید در گردش خزانه بیایند.
#[test]
fn t05_discount_and_offset_do_not_move_treasury() {
    let discount = DocumentLine::new(PaymentMethod::Discount, Money::from_rials(3_000_000));
    let offset = DocumentLine::new(PaymentMethod::Offset, Money::from_rials(4_000_000));
    let lines = vec![cash(5_000_000), discount, offset];
    let totals = calculate_totals(&lines).unwrap();

    assert_eq!(totals.total.rials(), 12_000_000, "جمع سند باید کل مبلغ باشد");
    assert_eq!(
        totals.treasury_movement.rials(),
        5_000_000,
        "فقط بخش نقدی باید موجودی خزانه را جابه‌جا کند"
    );
    assert!(!PaymentMethod::Discount.moves_treasury());
    assert!(!PaymentMethod::Offset.moves_treasury());
    assert!(!PaymentMethod::Check.moves_treasury());
}

/// ت۰۶ — تخفیف نقدی اعطایی، کاهش درآمد است نه دریافت.
///
/// در سند دریافت، تخفیف سمت بدهکار می‌نشیند: مشتری کمتر پرداخت کرده و
/// مابه‌التفاوت هزینه‌ی ماست.
#[test]
fn t06_discount_is_recorded_as_expense_side() {
    let discount = DocumentLine::new(PaymentMethod::Discount, Money::from_rials(1_500_000));
    let lines = vec![cash(8_500_000), discount];
    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();

    let discount_line = journal.iter().find(|l| l.account_id == "acc-4400").unwrap();
    assert_eq!(
        discount_line.debit.rials(),
        1_500_000,
        "تخفیف اعطایی در سند دریافت باید بدهکار شود"
    );
    // و طرف حساب به‌اندازه‌ی کل تسویه بستانکار می‌شود، نه فقط نقد دریافتی.
    let party = journal.iter().find(|l| l.account_id == "acc-1201").unwrap();
    assert_eq!(
        party.credit.rials(),
        10_000_000,
        "بدهی مشتری باید به‌اندازه‌ی کل تسویه کم شود"
    );
}

/// ت۰۷ — سطر بدون حساب خزانه برای روش نقدی باید رد شود.
#[test]
fn t07_cash_line_without_treasury_account_is_rejected() {
    let line = DocumentLine::new(PaymentMethod::Cash, Money::from_rials(1_000_000));
    assert!(
        calculate_totals(&[line]).is_err(),
        "روش نقدی بدون صندوق نباید پذیرفته شود"
    );

    let mut terminal = DocumentLine::new(PaymentMethod::CardTerminal, Money::from_rials(1_000_000));
    terminal.treasury_account = Some("treasury-bank-mellat".into());
    assert!(
        calculate_totals(&[terminal]).is_err(),
        "کارتخوان بدون شناسه‌ی پایانه نباید پذیرفته شود"
    );
}

/// ت۰۸ — سطر چک بدون شماره یا سررسید باید رد شود.
#[test]
fn t08_check_line_requires_serial_and_due_date() {
    let bare = DocumentLine::new(PaymentMethod::Check, Money::from_rials(5_000_000));
    assert!(calculate_totals(&[bare]).is_err(), "چک بدون مشخصات");

    let mut missing_due = DocumentLine::new(PaymentMethod::Check, Money::from_rials(5_000_000));
    missing_due.check = Some(CheckDetails {
        serial: "700999".into(),
        due_date: "   ".into(),
        bank_name: None,
        sayad_id: None,
    });
    assert!(
        calculate_totals(&[missing_due]).is_err(),
        "چک بدون سررسید نباید ثبت شود"
    );
}

/// ت۰۹ — سند خالی یا با مبلغ صفر/منفی هرگز ثبت نمی‌شود.
#[test]
fn t09_empty_or_non_positive_document_is_rejected() {
    assert!(calculate_totals(&[]).is_err(), "سند بدون سطر");
    assert!(
        calculate_totals(&[cash(0)]).is_err(),
        "مبلغ صفر نباید پذیرفته شود"
    );
    assert!(
        calculate_totals(&[cash(-1)]).is_err(),
        "مبلغ منفی نباید پذیرفته شود"
    );
}

/// ت۱۰ — تفکیک روش‌ها با جمع کل و با سند حسابداری کاملاً می‌خواند.
///
/// اگر تفکیک و جمع از هم جدا شوند، گزارش «دریافت به تفکیک روش» با ترازنامه
/// اختلاف پیدا می‌کند — خطایی که فقط در حسابرسی لو می‌رود.
#[test]
fn t10_method_breakdown_reconciles_with_journal() {
    let mut transfer =
        DocumentLine::new(PaymentMethod::BankTransfer, Money::from_rials(7_000_000));
    transfer.treasury_account = Some("treasury-bank-mellat".into());
    let mut terminal = DocumentLine::new(PaymentMethod::CardTerminal, Money::from_rials(3_000_000));
    terminal.treasury_account = Some("treasury-bank-saderat".into());
    terminal.terminal_id = Some("POS-1001".into());
    let discount = DocumentLine::new(PaymentMethod::Discount, Money::from_rials(250_000));

    let lines = vec![cash(2_000_000), check(9_000_000), transfer, terminal, discount];
    let totals = calculate_totals(&lines).unwrap();

    let breakdown = totals.cash.rials()
        + totals.check.rials()
        + totals.bank_transfer.rials()
        + totals.card_terminal.rials()
        + totals.discount.rials()
        + totals.offset.rials();
    assert_eq!(breakdown, totals.total.rials(), "تفکیک با جمع کل نمی‌خواند");

    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    let party = journal.iter().find(|l| l.account_id == "acc-1201").unwrap();
    assert_eq!(
        party.credit.rials(),
        totals.total.rials(),
        "سطر طرف حساب باید دقیقاً جمع سند باشد"
    );
    assert_eq!(sum_debit(&journal), sum_credit(&journal));
}
