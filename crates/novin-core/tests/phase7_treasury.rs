//! # تست‌های سخت‌گیرانه‌ی فاز ۷ — خزانه
//!
//! مرجع: تصاویر `MZlUiD` (سند دریافت چندروشی)، `p6hT01` (بانک‌ها)،
//! `WLumbs` (صندوق‌ها) و «دسته چک» در منوی اطلاعات پایه.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | روش‌های تسویه | شش روش با رفتار درست نسبت به خزانه |
//! | ۲ | اعتبارسنجی سطر | هر روش الزامات خودش را دارد |
//! | ۳ | جمع سند | تفکیک به‌ازای روش و جمع کل دقیق |
//! | ۴ | سند دریافت | سند حسابداری چندسطری و متعادل |
//! | ۵ | سند پرداخت | جهت بدهکار/بستانکار معکوس دریافت |
//! | ۶ | چک در سند | به حساب اسناد دریافتنی/پرداختنی می‌رود |
//! | ۷ | تخفیف و تهاتر | پول جابه‌جا نمی‌کنند ولی مانده را می‌بندند |
//! | ۸ | سیاست منفی شدن موجودی | خطا / هشدار / بی‌تأثیر |
//! | ۹ | دسته‌چک | شماره‌ی تکراری، خارج از محدوده و اتمام دسته |
//! | ۱۰ | پایگاه داده | جدول‌ها، CHECKها و حساب‌های خزانه |

use novin_core::accounting::validate_journal;
use novin_core::money::Money;
use novin_core::treasury::{
    build_journal, calculate_totals, check_withdrawal, validate_line, BalanceCheck, CheckDetails,
    Checkbook, DocumentKind, DocumentLine, NegativeBalancePolicy, PaymentMethod, TreasuryAccounts,
    TreasuryError,
};

fn accounts() -> TreasuryAccounts {
    TreasuryAccounts {
        party_account: "acc-1201".into(),
        notes_receivable: "acc-1103".into(),
        notes_payable: "acc-2103".into(),
        discount_account: "acc-4400".into(),
    }
}

fn check_line(amount: i64) -> DocumentLine {
    let mut line = DocumentLine::new(PaymentMethod::Check, Money::from_rials(amount));
    line.check = Some(CheckDetails {
        serial: "CHK-100001".into(),
        due_date: "1405/07/15".into(),
        bank_name: Some("بانک ملت".into()),
        sayad_id: None,
    });
    line
}

// ---------------------------------------------------------------------------
// تست ۱ — روش‌های تسویه
// ---------------------------------------------------------------------------
#[test]
fn t01_payment_methods_match_legacy_form() {
    assert_eq!(PaymentMethod::Cash.label(), "نقد");
    assert_eq!(PaymentMethod::Check.label(), "چک");
    assert_eq!(PaymentMethod::BankTransfer.label(), "حواله");
    assert_eq!(PaymentMethod::CardTerminal.label(), "کارتخوان");
    assert_eq!(PaymentMethod::Discount.label(), "تخفیف");
    assert_eq!(PaymentMethod::Offset.label(), "تهاتر");

    // فقط این سه روش موجودی خزانه را جابه‌جا می‌کنند
    assert!(PaymentMethod::Cash.moves_treasury());
    assert!(PaymentMethod::BankTransfer.moves_treasury());
    assert!(PaymentMethod::CardTerminal.moves_treasury());
    assert!(!PaymentMethod::Check.moves_treasury(), "چک هنوز وصول نشده");
    assert!(!PaymentMethod::Discount.moves_treasury());
    assert!(!PaymentMethod::Offset.moves_treasury());

    for method in [
        PaymentMethod::Cash,
        PaymentMethod::Check,
        PaymentMethod::BankTransfer,
        PaymentMethod::CardTerminal,
        PaymentMethod::Discount,
        PaymentMethod::Offset,
    ] {
        assert_eq!(PaymentMethod::parse(method.as_str()), Some(method));
    }
    assert_eq!(PaymentMethod::parse("bitcoin"), None);
    assert_eq!(DocumentKind::Receipt.label(), "سند دریافت");
    assert_eq!(DocumentKind::Payment.label(), "سند پرداخت");
}

// ---------------------------------------------------------------------------
// تست ۲ — اعتبارسنجی سطر
// ---------------------------------------------------------------------------
#[test]
fn t02_each_method_has_its_own_requirements() {
    // نقد بدون حساب خزانه مردود است
    let cash = DocumentLine::new(PaymentMethod::Cash, Money::from_rials(1_000_000));
    assert_eq!(
        validate_line(&cash),
        Err(TreasuryError::MissingTreasuryAccount)
    );
    assert!(validate_line(&cash.clone().with_account("treasury-cash")).is_ok());

    // کارتخوان علاوه بر حساب، پایانه هم می‌خواهد
    let mut card = DocumentLine::new(PaymentMethod::CardTerminal, Money::from_rials(500_000))
        .with_account("treasury-bank");
    assert_eq!(validate_line(&card), Err(TreasuryError::MissingTerminal));
    card.terminal_id = Some("pos-1".into());
    assert!(validate_line(&card).is_ok());

    // چک بدون شماره یا سررسید مردود است
    let mut broken = DocumentLine::new(PaymentMethod::Check, Money::from_rials(1_000));
    assert_eq!(
        validate_line(&broken),
        Err(TreasuryError::MissingCheckDetails)
    );
    broken.check = Some(CheckDetails {
        serial: "  ".into(),
        due_date: "1405/07/15".into(),
        bank_name: None,
        sayad_id: None,
    });
    assert_eq!(
        validate_line(&broken),
        Err(TreasuryError::MissingCheckDetails)
    );
    assert!(validate_line(&check_line(1_000)).is_ok());

    // تخفیف و تهاتر حساب خزانه نمی‌خواهند
    assert!(validate_line(&DocumentLine::new(
        PaymentMethod::Discount,
        Money::from_rials(50_000)
    ))
    .is_ok());

    // مبلغ صفر یا منفی
    assert_eq!(
        validate_line(&DocumentLine::new(PaymentMethod::Discount, Money::ZERO)),
        Err(TreasuryError::NonPositiveAmount)
    );
    assert_eq!(
        validate_line(&DocumentLine::new(
            PaymentMethod::Discount,
            Money::from_rials(-1)
        )),
        Err(TreasuryError::NonPositiveAmount)
    );
}

// ---------------------------------------------------------------------------
// تست ۳ — جمع سند چندروشی
// ---------------------------------------------------------------------------
#[test]
fn t03_multi_method_totals() {
    let mut card = DocumentLine::new(PaymentMethod::CardTerminal, Money::from_rials(2_500_000))
        .with_account("treasury-bank");
    card.terminal_id = Some("pos-1".into());

    let lines = vec![
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(2_000_000))
            .with_account("treasury-cash"),
        check_line(5_000_000),
        DocumentLine::new(PaymentMethod::BankTransfer, Money::from_rials(3_000_000))
            .with_account("treasury-bank"),
        card,
        DocumentLine::new(PaymentMethod::Discount, Money::from_rials(100_000)),
    ];

    let totals = calculate_totals(&lines).unwrap();
    assert_eq!(totals.cash, Money::from_rials(2_000_000));
    assert_eq!(totals.check, Money::from_rials(5_000_000));
    assert_eq!(totals.bank_transfer, Money::from_rials(3_000_000));
    assert_eq!(totals.card_terminal, Money::from_rials(2_500_000));
    assert_eq!(totals.discount, Money::from_rials(100_000));
    assert_eq!(totals.total, Money::from_rials(12_600_000));
    // فقط نقد + حواله + کارتخوان پول واقعی جابه‌جا می‌کنند
    assert_eq!(totals.treasury_movement, Money::from_rials(7_500_000));

    // سند خالی
    assert_eq!(calculate_totals(&[]), Err(TreasuryError::EmptyDocument));
    // یک سطر نامعتبر کل سند را رد می‌کند
    let bad = vec![
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(1)).with_account("t"),
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(1)),
    ];
    assert!(calculate_totals(&bad).is_err());
}

// ---------------------------------------------------------------------------
// تست ۴ — سند حسابداری دریافت
// ---------------------------------------------------------------------------
#[test]
fn t04_receipt_journal_is_balanced_and_multi_line() {
    let lines = vec![
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(2_000_000))
            .with_account("treasury-cash"),
        DocumentLine::new(PaymentMethod::BankTransfer, Money::from_rials(3_000_000))
            .with_account("treasury-bank"),
    ];
    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();

    assert_eq!(journal.len(), 3, "دو سطر خزانه + یک سطر طرف حساب");
    let totals = validate_journal(&journal).unwrap();
    assert_eq!(totals.total_debit, Money::from_rials(5_000_000));
    assert_eq!(totals.total_debit, totals.total_credit);

    // حساب‌های خزانه بدهکار می‌شوند
    assert_eq!(journal[0].account_id, "treasury-cash");
    assert_eq!(journal[0].debit, Money::from_rials(2_000_000));
    assert_eq!(journal[1].account_id, "treasury-bank");
    // طرف حساب بستانکار می‌شود (بدهی‌اش کم می‌شود)
    let party = journal.last().unwrap();
    assert_eq!(party.account_id, "acc-1201");
    assert_eq!(party.credit, Money::from_rials(5_000_000));

    // بدون طرف حساب
    let mut missing = accounts();
    missing.party_account = String::new();
    assert_eq!(
        build_journal(DocumentKind::Receipt, &lines, &missing),
        Err(TreasuryError::MissingParty)
    );
}

// ---------------------------------------------------------------------------
// تست ۵ — سند پرداخت معکوس دریافت است
// ---------------------------------------------------------------------------
#[test]
fn t05_payment_journal_mirrors_receipt() {
    let lines = vec![
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(4_000_000))
            .with_account("treasury-cash"),
    ];

    let receipt = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    let payment = build_journal(DocumentKind::Payment, &lines, &accounts()).unwrap();

    validate_journal(&receipt).unwrap();
    validate_journal(&payment).unwrap();

    // جهت‌ها دقیقاً معکوس‌اند
    assert_eq!(receipt[0].debit, payment[0].credit);
    assert_eq!(receipt[0].credit, payment[0].debit);
    assert_eq!(
        receipt.last().unwrap().credit,
        payment.last().unwrap().debit
    );

    // در پرداخت، طرف حساب بدهکار می‌شود
    assert_eq!(payment.last().unwrap().account_id, "acc-1201");
    assert_eq!(payment.last().unwrap().debit, Money::from_rials(4_000_000));
    assert_eq!(payment[0].credit, Money::from_rials(4_000_000));
}

// ---------------------------------------------------------------------------
// تست ۶ — چک به حساب اسناد می‌رود، نه صندوق
// ---------------------------------------------------------------------------
#[test]
fn t06_check_goes_to_notes_account() {
    let lines = vec![check_line(7_000_000)];

    let receipt = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    assert_eq!(
        receipt[0].account_id, "acc-1103",
        "چک دریافتی به اسناد دریافتنی می‌رود نه صندوق"
    );
    assert_eq!(receipt[0].debit, Money::from_rials(7_000_000));

    let payment = build_journal(DocumentKind::Payment, &lines, &accounts()).unwrap();
    assert_eq!(
        payment[0].account_id, "acc-2103",
        "چک صادرشده به اسناد پرداختنی می‌رود"
    );
    assert_eq!(payment[0].credit, Money::from_rials(7_000_000));

    // چک در جمع «جابه‌جایی خزانه» دیده نمی‌شود چون هنوز وصول نشده
    let totals = calculate_totals(&lines).unwrap();
    assert_eq!(totals.treasury_movement, Money::ZERO);
    assert_eq!(totals.total, Money::from_rials(7_000_000));
}

// ---------------------------------------------------------------------------
// تست ۷ — تخفیف و تهاتر
// ---------------------------------------------------------------------------
#[test]
fn t07_discount_and_offset_close_balance_without_cash() {
    // مشتری ۱۰ میلیون بدهکار است: ۹.۵ نقد + ۰.۵ تخفیف
    let lines = vec![
        DocumentLine::new(PaymentMethod::Cash, Money::from_rials(9_500_000))
            .with_account("treasury-cash"),
        DocumentLine::new(PaymentMethod::Discount, Money::from_rials(500_000)),
    ];
    let journal = build_journal(DocumentKind::Receipt, &lines, &accounts()).unwrap();
    let totals = validate_journal(&journal).unwrap();

    // مانده‌ی طرف حساب کامل بسته می‌شود
    assert_eq!(totals.total_credit, Money::from_rials(10_000_000));
    // ولی فقط ۹.۵ میلیون پول وارد صندوق شده
    let treasury = calculate_totals(&lines).unwrap();
    assert_eq!(treasury.treasury_movement, Money::from_rials(9_500_000));
    // تخفیف به حساب تخفیفات اعطایی می‌رود
    assert!(journal
        .iter()
        .any(|line| line.account_id == "acc-4400" && line.debit == Money::from_rials(500_000)));

    // تهاتر روی خود حساب طرف حساب می‌نشیند
    let offset = vec![DocumentLine::new(
        PaymentMethod::Offset,
        Money::from_rials(1_000_000),
    )];
    let journal = build_journal(DocumentKind::Receipt, &offset, &accounts()).unwrap();
    validate_journal(&journal).unwrap();
    assert_eq!(journal.len(), 2);
    assert_eq!(journal[0].account_id, journal[1].account_id);
}

// ---------------------------------------------------------------------------
// تست ۸ — سیاست منفی شدن موجودی
// ---------------------------------------------------------------------------
#[test]
fn t08_negative_balance_policy() {
    let balance = Money::from_rials(1_000_000);
    let amount = Money::from_rials(1_500_000);

    // برداشت داخل موجودی همیشه مجاز است
    assert_eq!(
        check_withdrawal(
            "صندوق ۱",
            balance,
            Money::from_rials(1_000_000),
            NegativeBalancePolicy::Error
        )
        .unwrap(),
        BalanceCheck::Allowed
    );

    // خطا: عملیات انجام نمی‌شود
    assert_eq!(
        check_withdrawal("صندوق ۱", balance, amount, NegativeBalancePolicy::Error),
        Err(TreasuryError::NegativeBalance {
            account: "صندوق ۱".into(),
            balance: 1_000_000,
            amount: 1_500_000
        })
    );

    // هشدار: انجام می‌شود ولی پیام می‌دهد
    match check_withdrawal("بانک سینا", balance, amount, NegativeBalancePolicy::Warn).unwrap()
    {
        BalanceCheck::Warning(message) => {
            assert!(message.contains("بانک سینا"));
            assert!(message.contains("500,000"), "پیام هشدار: {message}");
        }
        other => panic!("انتظار هشدار داشتیم: {other:?}"),
    }

    // بی‌تأثیر
    assert_eq!(
        check_withdrawal("تنخواه", balance, amount, NegativeBalancePolicy::Ignore).unwrap(),
        BalanceCheck::Allowed
    );

    assert_eq!(
        NegativeBalancePolicy::parse("error"),
        NegativeBalancePolicy::Error
    );
    assert_eq!(
        NegativeBalancePolicy::parse("ignore"),
        NegativeBalancePolicy::Ignore
    );
    assert_eq!(
        NegativeBalancePolicy::parse("چیز دیگر"),
        NegativeBalancePolicy::Warn
    );
    assert_eq!(NegativeBalancePolicy::Error.label(), "خطا");
}

// ---------------------------------------------------------------------------
// تست ۹ — دسته‌چک
// ---------------------------------------------------------------------------
#[test]
fn t09_checkbook_serial_control() {
    let mut book = Checkbook {
        id: "cb-1".into(),
        bank_account_id: "treasury-bank".into(),
        serial_from: 100_001,
        serial_to: 100_003,
        used_serials: vec![],
    };
    assert_eq!(book.capacity(), 3);
    assert_eq!(book.remaining(), 3);
    assert_eq!(book.next_serial().unwrap(), 100_001);

    book.use_serial(100_001).unwrap();
    assert_eq!(book.remaining(), 2);
    assert_eq!(book.next_serial().unwrap(), 100_002);

    // شماره‌ی تکراری
    assert_eq!(
        book.use_serial(100_001),
        Err(TreasuryError::SerialAlreadyUsed { serial: 100_001 })
    );
    // خارج از محدوده
    assert_eq!(
        book.use_serial(999_999),
        Err(TreasuryError::SerialOutOfRange)
    );
    assert_eq!(
        book.use_serial(100_000),
        Err(TreasuryError::SerialOutOfRange)
    );

    // استفاده‌ی غیرترتیبی: شماره‌ی آزاد بعدی باید درست پیدا شود
    book.use_serial(100_003).unwrap();
    assert_eq!(book.next_serial().unwrap(), 100_002);

    // اتمام دسته
    book.use_serial(100_002).unwrap();
    assert_eq!(book.remaining(), 0);
    assert_eq!(book.next_serial(), Err(TreasuryError::CheckbookExhausted));

    // محدوده‌ی نامعتبر
    let invalid = Checkbook {
        id: "cb-2".into(),
        bank_account_id: "b".into(),
        serial_from: 500,
        serial_to: 100,
        used_serials: vec![],
    };
    assert_eq!(
        invalid.validate(),
        Err(TreasuryError::InvalidCheckbookRange)
    );
    assert_eq!(
        invalid.next_serial(),
        Err(TreasuryError::InvalidCheckbookRange)
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — پایگاه داده‌ی خزانه
// ---------------------------------------------------------------------------
#[test]
fn t10_treasury_schema_and_seed() {
    let conn = novin_core::db::open_in_memory().unwrap();

    for table in [
        "treasury_documents",
        "treasury_document_lines",
        "checkbooks",
        "checkbook_serials",
        "pos_terminals",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "جدول {table} ساخته نشده است");
    }

    let columns: Vec<String> = {
        let mut statement = conn
            .prepare("PRAGMA table_info(treasury_accounts)")
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    for column in [
        "negative_policy",
        "card_number",
        "branch_name",
        "branch_code",
        "holder_name",
        "has_pos_terminal",
    ] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }

    // حساب‌های اسناد دریافتنی/پرداختنی و تخفیف
    let treasury_accounts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE code IN ('1103','2103','4400')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(treasury_accounts, 3, "حساب‌های خزانه تعریف نشده‌اند");

    // دسته‌چک نمونه
    let (from, to): (i64, i64) = conn
        .query_row(
            "SELECT serial_from,serial_to FROM checkbooks WHERE id='checkbook-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(to - from + 1, 25, "دسته‌چک نمونه باید ۲۵ برگ باشد");

    // روش پرداخت نامعتبر باید توسط پایگاه داده رد شود
    conn.execute(
        "INSERT INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,document_date,created_by) \
         VALUES('td-1','company-demo','fy-demo','receipt',900001,'1405/05/01','user-demo')",
        [],
    )
    .unwrap();
    let invalid = conn.execute(
        "INSERT INTO treasury_document_lines(id,document_id,method,amount) \
         VALUES('tdl-bad','td-1','crypto',1000)",
        [],
    );
    assert!(invalid.is_err(), "CHECK روش پرداخت کار نمی‌کند");

    // مبلغ صفر هم مردود است
    let zero = conn.execute(
        "INSERT INTO treasury_document_lines(id,document_id,method,amount) \
         VALUES('tdl-zero','td-1','cash',0)",
        [],
    );
    assert!(zero.is_err(), "CHECK مبلغ سطر کار نمی‌کند");

    // محدوده‌ی نامعتبر دسته‌چک
    let bad_range = conn.execute(
        "INSERT INTO checkbooks(id,company_id,treasury_account_id,title,serial_from,serial_to) \
         VALUES('cb-bad','company-demo','treasury-cash-demo','خراب',500,100)",
        [],
    );
    assert!(bad_range.is_err(), "CHECK محدوده‌ی دسته‌چک کار نمی‌کند");
}
