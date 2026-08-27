#![allow(warnings)] // موقت: بعد از پایدارشدن CI فایل‌به‌فایل برداشته می‌شود
//! # تست‌های سخت‌گیرانه‌ی فاز ۲ — کدینگ حساب‌ها، تفصیلی شناور و سند یک‌سطری
//!
//! مرجع: منوی «کدینگ حساب‌ها»، زبانه‌ی «تنظیمات کدینگ» و فرم «صدور سند یک‌سطری»
//! نرم‌افزار فعلی (تصاویر `dgNqWj`، `k51J4O`، `Rb2xiG`).
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | طرح کدینگ | تشخیص سطح و کد والد با کدهای واقعی نرم‌افزار درست است |
//! | ۲ | طرح کدینگ | تعداد سطوح قابل پیکربندی است و Hard-code نشده |
//! | ۳ | شماره‌گذاری | تولید کد فرزند و یافتن نخستین کد آزاد بی‌نقص است |
//! | ۴ | درخت حساب‌ها | ساخت درخت و کشف خطاهای ساختاری |
//! | ۵ | ماهیت حساب | ماهیت فرزند نمی‌تواند با والد در تضاد باشد |
//! | ۶ | ثبت‌پذیری | سند فقط روی حساب سطح آخر ثبت می‌شود |
//! | ۷ | تفصیلی شناور | الزام تفصیلی و تعلق آن به گروه مجاز کنترل می‌شود |
//! | ۸ | ابعاد مالی | مرکز هزینه و پروژه‌ی الزامی رعایت می‌شوند |
//! | ۹ | سند یک‌سطری | سند دوسطری متعادل با ابعاد کامل تولید می‌کند |
//! | ۱۰ | پایگاه داده | جدول‌های ابعاد مالی و ستون‌های سطر سند واقعاً ساخته شده‌اند |

use novin_core::accounting::{
    single_line_entry, swap_sides, validate_journal, validate_journal_dimensions, AccountingError,
    PostingSide,
};
use novin_core::coding::{
    build_tree, validate_posting, AccountDefinition, AccountNature, CodingError, CodingScheme,
    Dimensions, Subsidiary,
};
use novin_core::money::Money;

fn scheme() -> CodingScheme {
    CodingScheme::default()
}

/// حساب سطح آخر (تفصیلی، ۷ رقم) که قابل ثبت است.
fn leaf(code: &str, title: &str, nature: AccountNature) -> AccountDefinition {
    AccountDefinition::new(code, title, nature)
}

// ---------------------------------------------------------------------------
// تست ۱ — تشخیص سطح با کدهای واقعی نرم‌افزار
// ---------------------------------------------------------------------------
#[test]
fn t01_levels_match_legacy_codes() {
    let scheme = scheme();
    assert_eq!(scheme.depth(), 4);
    assert_eq!(scheme.level_title(0), Some("گروه"));
    assert_eq!(scheme.level_title(3), Some("تفصیلی"));

    // طول تجمعی سطوح: ۱ / ۳ / ۵ / ۷
    assert_eq!(scheme.code_length(0), Some(1));
    assert_eq!(scheme.code_length(1), Some(3));
    assert_eq!(scheme.code_length(2), Some(5));
    assert_eq!(scheme.code_length(3), Some(7));
    assert_eq!(scheme.code_length(4), None);

    // «۱۱۰۳۱۰۱ – اشخاص (حساب های دریافتنی)» از تصویر فاکتور فروش
    assert_eq!(scheme.level_of("1103101").unwrap(), 3);
    assert_eq!(scheme.parent_code("1103101").unwrap(), "11031");
    assert_eq!(scheme.parent_code("11031").unwrap(), "110");
    assert_eq!(scheme.parent_code("110").unwrap(), "1");
    assert_eq!(scheme.parent_code("1"), Err(CodingError::RootHasNoParent));

    // کدهای نامعتبر
    assert_eq!(
        scheme.level_of("11"),
        Err(CodingError::UnknownLevel { code: "11".into() })
    );
    assert_eq!(scheme.level_of("11a"), Err(CodingError::NonNumericCode));
    assert_eq!(scheme.level_of(""), Err(CodingError::NonNumericCode));

    assert!(scheme.is_leaf_level("1103101").unwrap());
    assert!(!scheme.is_leaf_level("11031").unwrap());
}

// ---------------------------------------------------------------------------
// تست ۲ — تعداد سطوح Hard-code نشده است
// ---------------------------------------------------------------------------
#[test]
fn t02_coding_depth_is_configurable() {
    // طرح پنج‌سطحی برای شرکت‌های بزرگ‌تر
    let five = CodingScheme::new(
        vec![1, 2, 2, 2, 3],
        ["گروه", "کل", "معین", "تفصیلی", "تفصیلی ۲"]
            .iter()
            .map(|title| title.to_string())
            .collect(),
    )
    .expect("طرح پنج‌سطحی باید معتبر باشد");
    assert_eq!(five.depth(), 5);
    assert_eq!(five.code_length(4), Some(10));
    assert_eq!(five.level_of("1103101001").unwrap(), 4);
    assert!(five.is_leaf_level("1103101001").unwrap());
    // در طرح پنج‌سطحی، کد هفت‌رقمی دیگر برگ نیست
    assert!(!five.is_leaf_level("1103101").unwrap());

    // طرح دوسطحی ساده
    let two = CodingScheme::new(vec![2, 3], vec!["کل".into(), "معین".into()]).unwrap();
    assert_eq!(two.level_of("12345").unwrap(), 1);
    assert_eq!(two.parent_code("12345").unwrap(), "12");

    // طرح‌های نامعتبر رد می‌شوند
    assert!(CodingScheme::new(vec![], vec![]).is_none());
    assert!(CodingScheme::new(vec![1, 2], vec!["فقط یکی".into()]).is_none());
    assert!(CodingScheme::new(vec![0, 2], vec!["الف".into(), "ب".into()]).is_none());
    assert!(CodingScheme::new(vec![9], vec!["خیلی بلند".into()]).is_none());
}

// ---------------------------------------------------------------------------
// تست ۳ — شماره‌گذاری خودکار کد حساب
// ---------------------------------------------------------------------------
#[test]
fn t03_child_code_generation_is_exact() {
    let scheme = scheme();

    assert_eq!(scheme.child_code("1", 10).unwrap(), "110");
    assert_eq!(scheme.child_code("110", 3).unwrap(), "11003");
    assert_eq!(scheme.child_code("11031", 1).unwrap(), "1103101");
    // صفر پیشوند باید حفظ شود
    assert_eq!(scheme.child_code("1", 1).unwrap(), "101");

    // ظرفیت سطح (۲ رقم = ۹۹ فرزند)
    assert_eq!(scheme.child_code("1", 99).unwrap(), "199");
    assert_eq!(
        scheme.child_code("1", 100),
        Err(CodingError::LevelExhausted)
    );
    assert_eq!(scheme.child_code("1", 0), Err(CodingError::LevelExhausted));
    // سطح آخر فرزند ندارد
    assert!(scheme.child_code("1103101", 1).is_err());

    // نخستین کد آزاد
    let existing = vec!["101".to_string(), "102".to_string(), "104".to_string()];
    assert_eq!(scheme.next_child_code("1", &existing).unwrap(), "103");
    assert_eq!(scheme.next_child_code("1", &[]).unwrap(), "101");
}

// ---------------------------------------------------------------------------
// تست ۴ — ساخت درخت حساب‌ها و کشف خطاهای ساختاری
// ---------------------------------------------------------------------------
#[test]
fn t04_account_tree_is_built_and_validated() {
    let scheme = scheme();
    let accounts = vec![
        AccountDefinition::new("1", "دارایی ها", AccountNature::Debit),
        AccountDefinition::new("110", "دارایی جاری", AccountNature::Debit),
        AccountDefinition::new("11031", "حساب های دریافتنی", AccountNature::Debit),
        AccountDefinition::new("1103101", "اشخاص", AccountNature::Debit),
        AccountDefinition::new("1103102", "پیش پرداخت ها", AccountNature::Debit),
        AccountDefinition::new("2", "بدهی ها", AccountNature::Credit),
        AccountDefinition::new("210", "بدهی جاری", AccountNature::Credit),
        AccountDefinition::new("21001", "حساب های پرداختنی", AccountNature::Credit),
        AccountDefinition::new("2100101", "تأمین کنندگان", AccountNature::Credit),
    ];

    let tree = build_tree(&scheme, &accounts).unwrap();
    assert_eq!(tree.len(), 2, "دو گروه ریشه");
    assert_eq!(tree[0].account.code, "1");
    assert_eq!(tree[0].children[0].children[0].children.len(), 2);
    assert!(tree[0].children[0].children[0].children[0].is_postable(&scheme));
    assert!(!tree[0].is_postable(&scheme), "گروه قابل ثبت نیست");

    // والد گم‌شده
    let orphan = vec![AccountDefinition::new(
        "1103101",
        "یتیم",
        AccountNature::Debit,
    )];
    assert_eq!(
        build_tree(&scheme, &orphan),
        Err(CodingError::MissingParent {
            parent: "11031".into()
        })
    );

    // کد تکراری
    let duplicate = vec![
        AccountDefinition::new("1", "الف", AccountNature::Debit),
        AccountDefinition::new("1", "ب", AccountNature::Debit),
    ];
    assert_eq!(
        build_tree(&scheme, &duplicate),
        Err(CodingError::DuplicateCode { code: "1".into() })
    );

    // کد با طول نامعتبر
    let bad = vec![AccountDefinition::new("12", "بد", AccountNature::Debit)];
    assert!(build_tree(&scheme, &bad).is_err());
}

// ---------------------------------------------------------------------------
// تست ۵ — سازگاری ماهیت حساب با والد
// ---------------------------------------------------------------------------
#[test]
fn t05_account_nature_must_agree_with_parent() {
    let scheme = scheme();

    assert!(AccountNature::Mixed.accepts_child(AccountNature::Debit));
    assert!(AccountNature::Mixed.accepts_child(AccountNature::Credit));
    assert!(AccountNature::Debit.accepts_child(AccountNature::Debit));
    assert!(!AccountNature::Debit.accepts_child(AccountNature::Credit));
    assert_eq!(AccountNature::Credit.label(), "بستانکار");
    assert_eq!(AccountNature::Mixed.as_str(), "mixed");

    // فرزند بستانکار زیر والد بدهکار → خطا
    let conflicting = vec![
        AccountDefinition::new("1", "دارایی ها", AccountNature::Debit),
        AccountDefinition::new("110", "متضاد", AccountNature::Credit),
    ];
    assert_eq!(
        build_tree(&scheme, &conflicting),
        Err(CodingError::NatureConflict { code: "110".into() })
    );

    // والد دوطرفه هر دو ماهیت را می‌پذیرد
    let mixed_parent = vec![
        AccountDefinition::new("3", "حساب های واسط", AccountNature::Mixed),
        AccountDefinition::new("310", "بدهکار", AccountNature::Debit),
        AccountDefinition::new("320", "بستانکار", AccountNature::Credit),
    ];
    assert!(build_tree(&scheme, &mixed_parent).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۶ — ثبت سند فقط روی حساب سطح آخر
// ---------------------------------------------------------------------------
#[test]
fn t06_only_leaf_accounts_are_postable() {
    let scheme = scheme();
    let dimensions = Dimensions::default();

    let leaf_account = leaf("1103102", "پیش پرداخت ها", AccountNature::Debit);
    assert!(validate_posting(&scheme, &leaf_account, &dimensions).is_ok());

    for code in ["1", "110", "11031"] {
        let account = AccountDefinition::new(code, "سطح میانی", AccountNature::Debit);
        assert_eq!(
            validate_posting(&scheme, &account, &dimensions),
            Err(CodingError::NotPostable { code: code.into() }),
            "ثبت روی سطح میانی {code} نباید مجاز باشد"
        );
    }
}

// ---------------------------------------------------------------------------
// تست ۷ — تفصیلی شناور و گروه تفصیلی
// ---------------------------------------------------------------------------
#[test]
fn t07_floating_subsidiary_rules() {
    let scheme = scheme();

    // «۲۰۳۰۰۳ بانک سینا» از گروه تفصیلی «بانک ها» در نرم‌افزار فعلی
    let bank = Subsidiary {
        code: "203003".into(),
        title: "بانک سینا".into(),
        group: "banks".into(),
    };
    let cashbox = Subsidiary {
        code: "202002".into(),
        title: "صندوق 1".into(),
        group: "cashboxes".into(),
    };

    let bank_account =
        leaf("1101101", "موجودی بانک", AccountNature::Debit).with_subsidiary_group("banks");

    // تفصیلی درست از گروه درست
    assert!(validate_posting(
        &scheme,
        &bank_account,
        &Dimensions::with_subsidiary(bank.clone())
    )
    .is_ok());

    // تفصیلی از گروه اشتباه
    assert_eq!(
        validate_posting(
            &scheme,
            &bank_account,
            &Dimensions::with_subsidiary(cashbox)
        ),
        Err(CodingError::SubsidiaryGroupMismatch)
    );

    // نبود تفصیلی روی حسابی که الزام دارد
    assert_eq!(
        validate_posting(&scheme, &bank_account, &Dimensions::default()),
        Err(CodingError::SubsidiaryRequired)
    );

    // حسابی که اصلاً تفصیلی نمی‌پذیرد
    let plain = leaf("5100101", "هزینه متفرقه", AccountNature::Debit);
    assert_eq!(
        validate_posting(&scheme, &plain, &Dimensions::with_subsidiary(bank)),
        Err(CodingError::SubsidiaryNotAllowed)
    );
}

// ---------------------------------------------------------------------------
// تست ۸ — مرکز هزینه و پروژه
// ---------------------------------------------------------------------------
#[test]
fn t08_cost_center_and_project_requirements() {
    let scheme = scheme();
    let mut expense = leaf("5100102", "هزینه تبلیغات", AccountNature::Debit);
    expense.requires_cost_center = true;
    expense.requires_project = true;

    assert_eq!(
        validate_posting(&scheme, &expense, &Dimensions::default()),
        Err(CodingError::CostCenterRequired)
    );

    let with_cost_center = Dimensions {
        cost_center: Some("4001".into()),
        ..Default::default()
    };
    assert_eq!(
        validate_posting(&scheme, &expense, &with_cost_center),
        Err(CodingError::ProjectRequired)
    );

    let complete = Dimensions {
        cost_center: Some("4001".into()),
        project: Some("5001".into()),
        ..Default::default()
    };
    assert!(validate_posting(&scheme, &expense, &complete).is_ok());

    // حسابی که الزامی ندارد، با ابعاد خالی هم مشکلی ندارد
    let free = leaf("5100103", "هزینه آزاد", AccountNature::Debit);
    assert!(validate_posting(&scheme, &free, &Dimensions::default()).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۹ — سند حسابداری یک‌سطری
// ---------------------------------------------------------------------------
#[test]
fn t09_single_line_entry_produces_balanced_journal() {
    let scheme = scheme();
    let customer = Subsidiary {
        code: "1000021".into(),
        title: "رضا زاهدی".into(),
        group: "persons".into(),
    };
    let debit_side = PostingSide::with_dimensions(
        leaf("1103101", "اشخاص - دریافتنی", AccountNature::Debit).with_subsidiary_group("persons"),
        Dimensions::with_subsidiary(customer.clone()),
    );
    let credit_side = PostingSide::new(leaf("4101101", "فروش کالا", AccountNature::Credit));

    let amount = Money::from_rials(30_774_330); // مانده‌ی واقعی تصویر فاکتور
    let lines = single_line_entry(
        &scheme,
        amount,
        Some("فروش نمونه".to_string()),
        &debit_side,
        &credit_side,
    )
    .unwrap();

    assert_eq!(lines.len(), 2);
    let totals = validate_journal(&lines).unwrap();
    assert_eq!(totals.total_debit, amount);
    assert_eq!(totals.total_debit, totals.total_credit);
    assert_eq!(lines[0].account_id, "1103101");
    assert_eq!(lines[0].subsidiary_id.as_deref(), Some("1000021"));
    assert_eq!(lines[0].debit, amount);
    assert_eq!(lines[1].credit, amount);
    assert_eq!(lines[1].subsidiary_id, None);

    // دکمه‌ی جابه‌جایی طرفین
    let swapped = swap_sides(&lines).unwrap();
    assert_eq!(swapped[0].credit, amount);
    assert_eq!(swapped[1].debit, amount);
    validate_journal(&swapped).unwrap();

    // مبلغ نامعتبر
    assert_eq!(
        single_line_entry(&scheme, Money::ZERO, None, &debit_side, &credit_side),
        Err(AccountingError::NonPositiveAmount)
    );
    assert_eq!(
        single_line_entry(
            &scheme,
            Money::from_rials(-1),
            None,
            &debit_side,
            &credit_side
        ),
        Err(AccountingError::NonPositiveAmount)
    );
    // یک حساب در هر دو طرف
    assert_eq!(
        single_line_entry(&scheme, amount, None, &debit_side, &debit_side),
        Err(AccountingError::SameAccountOnBothSides)
    );
    // حساب سطح میانی در یک طرف
    let invalid_side = PostingSide::new(AccountDefinition::new(
        "11031",
        "سطح معین",
        AccountNature::Debit,
    ));
    assert!(single_line_entry(&scheme, amount, None, &invalid_side, &credit_side).is_err());

    // اعتبارسنجی ابعاد روی سند چندسطری
    let resolve = |code: &str| -> Option<(AccountDefinition, Dimensions)> {
        match code {
            "1103101" => Some((
                debit_side.account.clone(),
                Dimensions::with_subsidiary(customer.clone()),
            )),
            "4101101" => Some((credit_side.account.clone(), Dimensions::default())),
            _ => None,
        }
    };
    assert!(validate_journal_dimensions(&scheme, &lines, resolve).is_ok());
}

// ---------------------------------------------------------------------------
// تست ۱۰ — پایگاه داده واقعاً ابعاد مالی را نگه می‌دارد
// ---------------------------------------------------------------------------
#[test]
fn t10_database_supports_financial_dimensions() {
    let conn = novin_core::db::open_in_memory().unwrap();

    for table in [
        "coding_schemes",
        "subsidiary_groups",
        "subsidiaries",
        "cost_centers",
        "projects",
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

    // ستون‌های ابعاد روی سطر سند
    let columns: Vec<String> = {
        let mut statement = conn.prepare("PRAGMA table_info(journal_lines)").unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        rows
    };
    for column in ["subsidiary_id", "cost_center_id", "project_id"] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }

    // گروه‌های تفصیلی سیستمی مطابق نرم‌افزار فعلی
    let groups: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT title FROM subsidiary_groups ORDER BY code")
            .unwrap();
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        rows
    };
    for expected in ["اشخاص", "صندوق ها", "بانک ها", "مراکز هزینه", "پروژه ها"]
    {
        assert!(
            groups.iter().any(|title| title == expected),
            "گروه تفصیلی «{expected}» در داده‌ی پایه نیست"
        );
    }

    // حساب‌های دریافتنی/پرداختنی باید تفصیلی الزامی داشته باشند
    let required: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE requires_subsidiary=1 AND code IN ('1201','2101')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(required, 2, "الزام تفصیلی روی حساب‌های اشخاص اعمال نشده");

    // مرکز هزینه و پروژه‌ی نمونه
    let cost_centers: i64 = conn
        .query_row("SELECT COUNT(*) FROM cost_centers", [], |row| row.get(0))
        .unwrap();
    assert!(cost_centers >= 2);
    let projects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE status='open'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(projects >= 1);

    // مهاجرت باید idempotent بماند
    novin_core::db::migrate(&conn).expect("اجرای دوباره‌ی مهاجرت نباید خطا بدهد");
    let groups_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM subsidiary_groups", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(groups_after, 5, "داده‌ی پایه نباید تکرار شود");
}
