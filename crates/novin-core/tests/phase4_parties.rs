//! # تست‌های سخت‌گیرانه‌ی فاز ۴ — اشخاص
//!
//! مرجع: تصاویر `c9pvYl` (لیست اشخاص با پنل خلاصه‌ی حساب) و `1zkKV5`
//! (فرم افزودن شخص) — با اعداد واقعی همان تصاویر.
//!
//! | # | موضوع | ادعا |
//! |---|-------|------|
//! | ۱ | انواع شخصیت | چهار نوع فرم + تفکیک حقیقی/حقوقی |
//! | ۲ | نقش‌ها | شخص، بازاریاب، سوپروایزر و حق پورسانت |
//! | ۳ | کد ملی | الگوریتم رسمی، شامل رد کدهای تکراری |
//! | ۴ | شناسه ملی حقوقی | الگوریتم ۱۱ رقمی رسمی |
//! | ۵ | شبا | mod-97 استاندارد با تشخیص یک رقم اشتباه |
//! | ۶ | کارت و کد پستی و موبایل | Luhn و یکسان‌سازی قالب |
//! | ۷ | اعتبارسنجی شخص | ترکیب قواعد در فرم افزودن شخص |
//! | ۸ | خلاصه‌ی حساب | بازتولید دقیق پنل کناری لیست اشخاص |
//! | ۹ | سقف اعتبار | جلوگیری از فروش نسیه فراتر از سقف |
//! | ۱۰ | پایگاه داده | جدول‌ها، ستون‌ها و داده‌ی پایه‌ی اشخاص |

use novin_core::money::Money;
use novin_core::parties::{
    card_number_is_valid, check_credit_limit, economic_code_is_valid, iban_is_valid,
    legal_id_is_valid, national_id_is_valid, normalize_mobile, postal_code_is_valid,
    remaining_credit, summarize_balances, BalanceStatus, PartyDefinition, PartyError,
    PartyFunction, PartyType,
};

fn customer(name: &str) -> PartyDefinition {
    PartyDefinition {
        code: "1".into(),
        party_type: PartyType::Natural,
        function: PartyFunction::Person,
        first_name: Some(name.into()),
        last_name: Some("زاهدی".into()),
        company_name: None,
        national_id: None,
        economic_code: None,
        postal_code: None,
        mobile: None,
        is_customer: true,
        is_supplier: false,
        credit_limit: 0,
        route: None,
        marketer_code: None,
    }
}

// ---------------------------------------------------------------------------
// تست ۱ — انواع شخصیت
// ---------------------------------------------------------------------------
#[test]
fn t01_party_types_match_legacy_form() {
    assert_eq!(PartyType::Natural.label(), "حقیقی");
    assert_eq!(PartyType::PrivateLegal.label(), "حقوقی غیردولتی");
    assert_eq!(PartyType::GovernmentLegal.label(), "حقوقی دولتی");
    assert_eq!(PartyType::CivilPartnership.label(), "مشارکت مدنی");

    assert!(!PartyType::Natural.is_legal_entity());
    for kind in [
        PartyType::PrivateLegal,
        PartyType::GovernmentLegal,
        PartyType::CivilPartnership,
    ] {
        assert!(kind.is_legal_entity(), "{} باید حقوقی باشد", kind.label());
        assert_eq!(PartyType::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(PartyType::parse("unknown"), None);

    // نام نمایشی: حقیقی از نام و نام خانوادگی، حقوقی از نام شرکت
    let natural = customer("رضا");
    assert_eq!(natural.display_name(), "رضا زاهدی");

    let mut legal = customer("بی‌اثر");
    legal.party_type = PartyType::PrivateLegal;
    legal.company_name = Some("شرکت آریا تجارت".into());
    assert_eq!(legal.display_name(), "شرکت آریا تجارت");
}

// ---------------------------------------------------------------------------
// تست ۲ — نقش‌های سازمانی
// ---------------------------------------------------------------------------
#[test]
fn t02_party_functions_and_commission() {
    assert_eq!(PartyFunction::Person.label(), "شخص");
    assert_eq!(PartyFunction::Marketer.label(), "بازاریاب");
    assert_eq!(PartyFunction::Supervisor.label(), "سوپروایزر");

    assert!(!PartyFunction::Person.earns_commission());
    assert!(PartyFunction::Marketer.earns_commission());
    assert!(PartyFunction::Supervisor.earns_commission());

    for function in [
        PartyFunction::Person,
        PartyFunction::Marketer,
        PartyFunction::Supervisor,
    ] {
        assert_eq!(PartyFunction::parse(function.as_str()), Some(function));
    }

    // بازاریاب لازم نیست مشتری یا تأمین‌کننده باشد
    let mut marketer = customer("سعید");
    marketer.function = PartyFunction::Marketer;
    marketer.is_customer = false;
    marketer.is_supplier = false;
    assert!(marketer.validate().is_ok());

    // ولی «شخص» عادی باید حداقل یک نقش تجاری داشته باشد
    let mut plain = customer("بدون نقش");
    plain.is_customer = false;
    plain.is_supplier = false;
    assert_eq!(plain.validate(), Err(PartyError::NoCommercialRole));
}

// ---------------------------------------------------------------------------
// تست ۳ — کد ملی
// ---------------------------------------------------------------------------
#[test]
fn t03_national_id_algorithm_is_official() {
    for valid in ["0499370899", "0012345679", "1234567891"] {
        assert!(national_id_is_valid(valid), "{valid} باید معتبر باشد");
    }
    // یک رقم اشتباه
    assert!(!national_id_is_valid("0499370898"));
    // طول نادرست
    assert!(!national_id_is_valid("049937089"));
    assert!(!national_id_is_valid("04993708990"));
    assert!(!national_id_is_valid(""));
    // ارقام یکسان: رقم کنترل تصادفاً درست است ولی چنین کدی صادر نمی‌شود
    assert!(
        !national_id_is_valid("1111111111"),
        "کد با ارقام یکسان باید رد شود حتی اگر رقم کنترل بگذرد"
    );
    assert!(!national_id_is_valid("0000000000"));
    // ارقام فارسی و جداکننده
    assert!(national_id_is_valid("۰۴۹۹۳۷۰۸۹۹"));
    assert!(national_id_is_valid("049-937-0899"));
    // حروف
    assert!(!national_id_is_valid("abcdefghij"));
}

// ---------------------------------------------------------------------------
// تست ۴ — شناسه ملی اشخاص حقوقی
// ---------------------------------------------------------------------------
#[test]
fn t04_legal_id_algorithm_is_official() {
    for valid in ["10293847568", "10630617911"] {
        assert!(legal_id_is_valid(valid), "{valid} باید معتبر باشد");
    }
    assert!(!legal_id_is_valid("10293847567"));
    assert!(!legal_id_is_valid("1029384756")); // ۱۰ رقم
    assert!(!legal_id_is_valid("102938475680")); // ۱۲ رقم
    assert!(!legal_id_is_valid("11111111111"));

    // کد ملی حقیقی نباید به‌عنوان شناسه ملی حقوقی قبول شود
    assert!(!legal_id_is_valid("0499370899"));
    assert!(!national_id_is_valid("10293847568"));

    // شخص حقوقی با شناسه‌ی نامعتبر
    let mut legal = customer("بی‌اثر");
    legal.party_type = PartyType::PrivateLegal;
    legal.company_name = Some("شرکت پارس".into());
    legal.national_id = Some("10293847567".into());
    assert_eq!(legal.validate(), Err(PartyError::InvalidLegalId));
    legal.national_id = Some("10293847568".into());
    assert!(legal.validate().is_ok());
}

// ---------------------------------------------------------------------------
// تست ۵ — شماره شبا
// ---------------------------------------------------------------------------
#[test]
fn t05_iban_mod97_validation() {
    let valid = "IR280620000000001234567891";
    assert!(iban_is_valid(valid));
    assert_eq!(valid.len(), 26);

    // با فاصله و خط تیره و حروف کوچک
    assert!(iban_is_valid("ir28 0620 0000 0000 1234 5678 91"));
    assert!(iban_is_valid("IR28-0620-0000-0000-1234-5678-91"));

    // تغییر یک رقم باید رد شود (قدرت اصلی mod-97)
    let mut broken: Vec<char> = valid.chars().collect();
    broken[10] = if broken[10] == '9' { '8' } else { '9' };
    assert!(!iban_is_valid(&broken.iter().collect::<String>()));

    // جابه‌جایی دو رقم مجاور هم باید کشف شود
    let mut swapped: Vec<char> = valid.chars().collect();
    swapped.swap(20, 21);
    assert!(!iban_is_valid(&swapped.iter().collect::<String>()));

    // قالب‌های نادرست
    assert!(!iban_is_valid("DE89370400440532013000"));
    assert!(!iban_is_valid("IR2806200000000012345678")); // کوتاه
    assert!(!iban_is_valid(""));
    assert!(!iban_is_valid("IR28062000000000123456789X"));
}

// ---------------------------------------------------------------------------
// تست ۶ — کارت بانکی، کد پستی و موبایل
// ---------------------------------------------------------------------------
#[test]
fn t06_card_postal_and_mobile_normalisation() {
    // Luhn
    assert!(card_number_is_valid("6037991234567893"));
    assert!(card_number_is_valid("6037-9912-3456-7893"));
    assert!(!card_number_is_valid("6037991234567894"));
    assert!(!card_number_is_valid("603799123456789")); // ۱۵ رقم

    // کد پستی
    assert!(postal_code_is_valid("1234567890"));
    assert!(!postal_code_is_valid("0234567890")); // شروع با صفر
    assert!(!postal_code_is_valid("123456789")); // ۹ رقم
    assert!(!postal_code_is_valid("1111111111"));

    // کد اقتصادی
    assert!(economic_code_is_valid("411111111111"));
    assert!(!economic_code_is_valid("41111111111")); // ۱۱ رقم
    assert!(!economic_code_is_valid("111111111111"));

    // یکسان‌سازی موبایل — همه باید به قالب 09xxxxxxxxx برسند
    let expected = "09309767300"; // موبایل واقعی سربرگ فاکتور
    for input in [
        "09309767300",
        "9309767300",
        "989309767300",
        "00989309767300",
        "+98 930 976 7300",
        "۰۹۳۰۹۷۶۷۳۰۰",
    ] {
        assert_eq!(
            normalize_mobile(input).as_deref(),
            Some(expected),
            "ورودی «{input}» درست یکسان‌سازی نشد"
        );
    }
    assert_eq!(normalize_mobile("021-88776655"), None);
    assert_eq!(normalize_mobile("0930976730"), None); // یک رقم کم
    assert_eq!(normalize_mobile(""), None);
}

// ---------------------------------------------------------------------------
// تست ۷ — اعتبارسنجی کامل فرم شخص
// ---------------------------------------------------------------------------
#[test]
fn t07_party_validation_combines_all_rules() {
    let mut party = customer("رضا");
    party.national_id = Some("0499370899".into());
    party.economic_code = Some("411111111111".into());
    party.postal_code = Some("1234567890".into());
    party.mobile = Some("09309767300".into());
    party.credit_limit = 500_000_000;
    assert!(party.validate().is_ok());

    // فیلدهای اختیاری خالی نباید خطا بدهند
    let mut minimal = customer("علی");
    minimal.national_id = Some(String::new());
    minimal.mobile = Some(String::new());
    assert!(minimal.validate().is_ok());

    // هر فیلد نامعتبر، خطای اختصاصی خودش را می‌دهد
    let mut invalid = party.clone();
    invalid.national_id = Some("0499370898".into());
    assert_eq!(invalid.validate(), Err(PartyError::InvalidNationalId));

    let mut invalid = party.clone();
    invalid.economic_code = Some("123".into());
    assert_eq!(invalid.validate(), Err(PartyError::InvalidEconomicCode));

    let mut invalid = party.clone();
    invalid.postal_code = Some("0000000000".into());
    assert_eq!(invalid.validate(), Err(PartyError::InvalidPostalCode));

    let mut invalid = party.clone();
    invalid.mobile = Some("12345".into());
    assert_eq!(invalid.validate(), Err(PartyError::InvalidMobile));

    let mut invalid = party.clone();
    invalid.credit_limit = -1;
    assert_eq!(invalid.validate(), Err(PartyError::NegativeCreditLimit));

    // نام خالی
    let mut nameless = customer("");
    nameless.first_name = None;
    nameless.last_name = None;
    assert_eq!(nameless.validate(), Err(PartyError::EmptyName));

    // شخص حقوقی بدون نام شرکت
    let mut legal = customer("بی‌اثر");
    legal.party_type = PartyType::GovernmentLegal;
    legal.company_name = None;
    assert_eq!(legal.validate(), Err(PartyError::MissingCompanyName));
}

// ---------------------------------------------------------------------------
// تست ۸ — خلاصه‌ی حساب (پنل کناری لیست اشخاص)
// ---------------------------------------------------------------------------
#[test]
fn t08_balance_summary_reproduces_legacy_panel() {
    // وضعیت تک‌تک مانده‌ها
    assert_eq!(
        BalanceStatus::of(Money::from_rials(1)),
        BalanceStatus::Debtor
    );
    assert_eq!(
        BalanceStatus::of(Money::from_rials(-1)),
        BalanceStatus::Creditor
    );
    assert_eq!(BalanceStatus::of(Money::ZERO), BalanceStatus::Settled);
    assert_eq!(BalanceStatus::Debtor.indicator(), "بد");
    assert_eq!(BalanceStatus::Creditor.indicator(), "بس");

    // نمونه‌ی واقعی از تصویر: بدهکاران با علامت مثبت، بستانکاران با منفی
    let balances = [
        Money::from_rials(5_749_885_636), // _متفرقه
        Money::from_rials(659_375_489),   // حسن باصری
        Money::from_rials(1_983_672),     // پرداختی های نامشخص
        Money::from_rials(-610_541_527),  // فرهاد ترابی (بستانکار)
        Money::from_rials(-67_930_542),   // خاکپور (بستانکار)
        Money::from_rials(10_687_500),    // محمدعلی رشیدی
        Money::ZERO,                      // بی‌حساب
        Money::ZERO,                      // بی‌حساب
    ];
    let summary = summarize_balances(&balances);

    assert_eq!(summary.total_count, 8);
    assert_eq!(summary.debtor_count, 4);
    assert_eq!(summary.creditor_count, 2);
    assert_eq!(summary.settled_count, 2);
    assert_eq!(
        summary.debtor_total,
        Money::from_rials(5_749_885_636 + 659_375_489 + 1_983_672 + 10_687_500)
    );
    assert_eq!(
        summary.creditor_total,
        Money::from_rials(610_541_527 + 67_930_542),
        "جمع بستانکاران باید مثبت نمایش داده شود"
    );
    assert_eq!(
        summary.net_total,
        summary.debtor_total - summary.creditor_total
    );
    // مجموع تعدادها باید با کل برابر باشد
    assert_eq!(
        summary.debtor_count + summary.creditor_count + summary.settled_count,
        summary.total_count
    );

    // فهرست خالی
    let empty = summarize_balances(&[]);
    assert_eq!(empty.total_count, 0);
    assert_eq!(empty.net_total, Money::ZERO);
    assert_eq!(empty.debtor_total, Money::ZERO);
}

// ---------------------------------------------------------------------------
// تست ۹ — سقف اعتبار
// ---------------------------------------------------------------------------
#[test]
fn t09_credit_limit_protects_against_over_selling() {
    let limit = 500_000_000i64;
    let balance = Money::from_rials(450_000_000);

    // فروش داخل سقف
    assert!(check_credit_limit(balance, limit, Money::from_rials(50_000_000)).is_ok());
    // دقیقاً روی سقف مجاز است
    assert!(check_credit_limit(balance, limit, Money::from_rials(50_000_000)).is_ok());
    // یک ریال بیشتر مردود است
    assert_eq!(
        check_credit_limit(balance, limit, Money::from_rials(50_000_001)),
        Err(PartyError::CreditLimitExceeded {
            balance: 500_000_001,
            limit
        })
    );

    // سقف صفر یعنی بدون محدودیت
    assert!(check_credit_limit(
        Money::from_rials(9_999_999_999),
        0,
        Money::from_rials(1_000_000)
    )
    .is_ok());
    // سقف منفی نامعتبر است
    assert_eq!(
        check_credit_limit(balance, -1, Money::ZERO),
        Err(PartyError::NegativeCreditLimit)
    );

    // مشتری بستانکار: ظرفیت اعتبارش بیشتر از سقف است
    assert!(check_credit_limit(
        Money::from_rials(-100_000_000),
        limit,
        Money::from_rials(590_000_000)
    )
    .is_ok());

    // اعتبار باقی‌مانده برای نمایش در فاکتور
    assert_eq!(
        remaining_credit(balance, limit),
        Some(Money::from_rials(50_000_000))
    );
    assert_eq!(remaining_credit(balance, 0), None);
    // پس از ممیزی: «اعتبار باقیمانده» هرگز منفی نمی‌شود.
    //
    // نمایش «اعتبار باقیمانده: منفی صد میلیون» برای کاربر بی‌معناست؛ آنچه
    // باید بفهمد این است که اعتبارش تمام شده. مبلغ تجاوز از سقف اطلاعات
    // جداگانه‌ای است و از خطای `check_credit_limit` می‌آید.
    assert_eq!(
        remaining_credit(Money::from_rials(600_000_000), limit),
        Some(Money::ZERO),
        "پس از عبور از سقف، باقیمانده صفر است نه منفی"
    );
    // و تلاش برای فروش نسیه‌ی بیشتر باید صریحاً رد شود.
    assert!(
        check_credit_limit(Money::from_rials(600_000_000), limit, Money::from_rials(1)).is_err(),
        "فروش نسیه پس از عبور از سقف باید رد شود"
    );
}

// ---------------------------------------------------------------------------
// تست ۱۰ — پایگاه داده‌ی اشخاص
// ---------------------------------------------------------------------------
#[test]
fn t10_party_database_schema_and_seed() {
    let conn = novin_core::db::open_in_memory().unwrap();

    for table in [
        "party_routes",
        "party_bank_accounts",
        "party_phones",
        "party_occasions",
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
        let mut statement = conn.prepare("PRAGMA table_info(contacts)").unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect()
    };
    for column in [
        "party_type",
        "party_function",
        "company_name",
        "economic_code",
        "postal_code",
        "route_id",
        "marketer_id",
        "opening_date",
    ] {
        assert!(columns.contains(&column.to_string()), "ستون {column} نیست");
    }

    // مسیرهای پخش و بازاریاب نمونه
    let routes: i64 = conn
        .query_row("SELECT COUNT(*) FROM party_routes", [], |row| row.get(0))
        .unwrap();
    assert!(routes >= 2, "مسیر پخش نمونه ایجاد نشده");

    let marketers: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE party_function='marketer'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marketers, 1, "بازاریاب نمونه ایجاد نشده");

    // اشخاص حقوقی باید نوع درست گرفته باشند
    let legal: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE party_type='private_legal' AND company_name IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(legal >= 1, "اشخاص حقوقی نمونه تنظیم نشده‌اند");

    // مشتریان باید مسیر و سقف اعتبار داشته باشند
    let assigned: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE is_customer=1 AND route_id IS NOT NULL AND credit_limit>0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(assigned >= 1, "مسیر یا سقف اعتبار مشتریان تنظیم نشده");

    // ماه مناسبت خارج از بازه باید توسط خود پایگاه داده رد شود
    let invalid = conn.execute(
        "INSERT INTO party_occasions(id,contact_id,title,jalali_month,jalali_day) \
         SELECT 'occ-bad',id,'تولد',13,1 FROM contacts LIMIT 1",
        [],
    );
    assert!(invalid.is_err(), "CHECK ماه شمسی کار نمی‌کند");

    // مهاجرت idempotent
    novin_core::db::migrate(&conn).unwrap();
    let routes_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM party_routes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(routes, routes_after);
}
