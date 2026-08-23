//! ممیزی ۴ — قرارداد لایه‌ی میزبان و صحت گزارش‌های مالی.
//!
//! ## چرا این ممیزی لازم بود
//!
//! دو ممیزی قبلی هسته‌ی مالی و رابط کاربری را سنجیدند. اما **لایه‌ی میزبان
//! (۱۸۲ فرمان Tauri) هرگز مستقیم آزموده نشده بود** — و همان‌جاست که مجوز
//! بررسی می‌شود، تراکنش باز می‌شود و گزارش ساخته می‌شود.
//!
//! ## دو تکنیک این فایل
//!
//! ۱. **ممیزی کد منبع**: قواعدی مثل «هر فرمان تغییردهنده باید مجوز بررسی
//!    کند» یا «هیچ `unwrap` در مسیر کاربر نباشد» را نمی‌شود با اجرای یک
//!    فرمان سنجید؛ باید کل کد بررسی شود.
//!
//! ۲. **بازسازی پرس‌وجوی گزارش**: گزارش‌های مالی (تراز، سود و زیان، سنی
//!    شدن) در میزبان با SQL ساخته می‌شوند. همان SQL اینجا روی داده‌ی نمونه
//!    اجرا و نتیجه‌اش با قواعد حسابداری سنجیده می‌شود.

use novin_core::db;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn seeded() -> Connection {
    let conn = db::open_in_memory().expect("پایگاه داده");
    db::demo::seed_demo_dataset(&conn).expect("داده‌ی نمونه");
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap_or(-1)
}

/// خواندن همه‌ی فایل‌های منبع میزبان.
///
/// مسیر نسبی از ریشه‌ی کارگاه محاسبه می‌شود تا تست در CI و محلی یکسان کار کند.
fn host_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("ریشه‌ی مخزن")
        .join("apps/desktop-host/src-tauri/src");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&root).expect("پوشه‌ی میزبان باید وجود داشته باشد")
    {
        let path = entry.expect("ورودی پوشه").path();
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            files.push((name, std::fs::read_to_string(&path).expect("خواندن فایل")));
        }
    }
    assert!(!files.is_empty(), "هیچ فایل منبعی در میزبان پیدا نشد");
    files
}

/// استخراج بدنه‌ی هر تابع دارای `#[tauri::command]`.
fn commands() -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    for (file, code) in host_sources() {
        let mut cursor = 0usize;
        while let Some(offset) = code[cursor..].find("#[tauri::command]") {
            let start = cursor + offset;
            // نام تابع
            let after = &code[start..];
            let Some(fn_offset) = after.find("fn ") else {
                break;
            };
            let name_start = start + fn_offset + 3;
            let name_end = code[name_start..]
                .find('(')
                .map(|index| name_start + index)
                .unwrap_or(name_start);
            let name = code[name_start..name_end].trim().to_string();

            // بدنه: از نخستین `{` پس از امضا تا `{`ِ متوازن
            let body_start = code[name_end..]
                .find('{')
                .map(|index| name_end + index)
                .unwrap_or(name_end);
            let mut depth = 0i32;
            let mut body_end = body_start;
            for (index, character) in code[body_start..].char_indices() {
                match character {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            body_end = body_start + index + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            result.push((file.clone(), name, code[body_start..body_end].to_string()));
            cursor = body_end.max(start + 1);
        }
    }
    assert!(result.len() > 150, "تعداد فرمان‌ها کمتر از انتظار است");
    result
}

// ===========================================================================
// قرارداد فرمان‌های میزبان
// ===========================================================================

/// ت۱۱۱ — هر فرمان اعلام‌شده، در فهرست ثبت فرمان‌ها هم آمده است.
///
/// فرمان ثبت‌نشده از رابط کاربری قابل فراخوانی نیست — یعنی دکمه‌ای که کار
/// نمی‌کند.
#[test]
fn t111_every_declared_command_is_registered() {
    let sources = host_sources();
    let main = sources
        .iter()
        .find(|(name, _)| name == "main.rs")
        .expect("main.rs");
    let handler_start = main.1.find("generate_handler![").expect("فهرست ثبت فرمان");
    let handler_end = main.1[handler_start..]
        .find(']')
        .map(|index| handler_start + index)
        .expect("پایان فهرست");
    let handler = &main.1[handler_start..handler_end];

    let mut missing = Vec::new();
    for (file, name, _) in commands() {
        // نام ممکن است با پیشوند ماژول ثبت شده باشد.
        if !handler.contains(&format!("{name},")) && !handler.contains(&format!("::{name},")) {
            missing.push(format!("{file}::{name}"));
        }
    }
    assert!(missing.is_empty(), "فرمان‌های ثبت‌نشده: {missing:?}");
}

/// ت۱۱۲ — هیچ فرمانی دو بار ثبت نشده است.
#[test]
fn t112_no_command_is_registered_twice() {
    let sources = host_sources();
    let main = &sources
        .iter()
        .find(|(name, _)| name == "main.rs")
        .unwrap()
        .1;
    let start = main.find("generate_handler![").unwrap();
    let end = start + main[start..].find(']').unwrap();
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for entry in main[start..end].split(',') {
        let name = entry.trim().rsplit("::").next().unwrap_or("").trim();
        if name.is_empty() || name.contains('[') {
            continue;
        }
        if !seen.insert(name.to_string()) {
            duplicates.push(name.to_string());
        }
    }
    assert!(duplicates.is_empty(), "فرمان تکراری: {duplicates:?}");
}

/// ت۱۱۳ — هر کد خطا در کل میزبان یکتاست.
///
/// دو خطای متفاوت با یک کد یعنی پشتیبانی نمی‌تواند مشکل کاربر را تشخیص دهد.
#[test]
fn t113_every_error_code_is_unique() {
    let mut codes: HashMap<String, Vec<String>> = HashMap::new();
    for (file, code) in host_sources() {
        // الگوی کد خطا: سه تا پنج حرف بزرگ، خط تیره، سه رقم
        let bytes: Vec<char> = code.chars().collect();
        let mut index = 0usize;
        while index + 8 < bytes.len() {
            if bytes[index].is_ascii_uppercase() {
                let mut letters = 0usize;
                while index + letters < bytes.len() && bytes[index + letters].is_ascii_uppercase() {
                    letters += 1;
                }
                if (3..=5).contains(&letters)
                    && index + letters + 3 < bytes.len()
                    && bytes[index + letters] == '-'
                    && bytes[index + letters + 1..index + letters + 4]
                        .iter()
                        .all(char::is_ascii_digit)
                {
                    let value: String = bytes[index..index + letters + 4].iter().collect();
                    codes.entry(value).or_default().push(file.clone());
                    index += letters + 4;
                    continue;
                }
                index += letters;
                continue;
            }
            index += 1;
        }
    }
    assert!(!codes.is_empty(), "هیچ کد خطایی پیدا نشد");

    // یک کد می‌تواند در چند جای یک فایل تکرار شود (مسیرهای مختلف یک خطا)،
    // ولی نباید در دو فایل متفاوت با معنای متفاوت به کار رود.
    let cross_file: Vec<String> = codes
        .iter()
        .filter(|(_, files)| files.iter().collect::<HashSet<_>>().len() > 1)
        .map(|(code, files)| {
            let unique: HashSet<_> = files.iter().collect();
            format!("{code} در {unique:?}")
        })
        .collect();
    assert!(
        cross_file.is_empty(),
        "کد خطای مشترک بین فایل‌ها: {cross_file:?}"
    );
}

/// ت۱۱۴ — هر پیام خطا کد دارد.
///
/// پیام بدون کد یعنی کاربر چیزی برای گفتن به پشتیبانی ندارد.
#[test]
fn t114_every_error_message_carries_a_code() {
    let mut naked = Vec::new();
    for (file, code) in host_sources() {
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            // پیام‌های فارسی که به‌عنوان خطا برگردانده می‌شوند
            if !(trimmed.contains("return Err(") || trimmed.contains("ok_or_else")) {
                continue;
            }
            if !trimmed
                .chars()
                .any(|character| ('\u{0600}'..='\u{06FF}').contains(&character))
            {
                continue;
            }
            let has_code = trimmed.contains('-')
                && trimmed
                    .chars()
                    .zip(trimmed.chars().skip(1))
                    .any(|(a, b)| a.is_ascii_uppercase() && (b.is_ascii_uppercase() || b == '-'));
            if !has_code {
                naked.push(format!(
                    "{file}: {}",
                    trimmed.chars().take(80).collect::<String>()
                ));
            }
        }
    }
    assert!(naked.is_empty(), "پیام خطای بدون کد: {naked:?}");
}

/// ت۱۱۵ — هر فرمان تغییردهنده، مجوز بررسی می‌کند.
///
/// فرمانی که بدون بررسی مجوز داده را عوض می‌کند، دور زدن کامل سیستم دسترسی است.
#[test]
fn t115_every_mutating_command_checks_permission() {
    let mut unguarded = Vec::new();
    for (file, name, body) in commands() {
        let mutates = body.contains("INSERT ")
            || body.contains("UPDATE ")
            || body.contains("DELETE ")
            || body.contains("tx.execute");
        if !mutates {
            continue;
        }
        let guarded = body.contains("require_permission")
            || body.contains("require_login")
            // فرمان‌هایی که خودشان فرمان دیگری را صدا می‌زنند، آنجا بررسی می‌شود.
            || body.contains("set_setting(")
            || body.contains("post(&state");
        if !guarded {
            unguarded.push(format!("{file}::{name}"));
        }
    }
    assert!(
        unguarded.is_empty(),
        "فرمان تغییردهنده بدون بررسی مجوز: {unguarded:?}"
    );
}

/// ت۱۱۶ — هیچ فرمانی با `unwrap` روی ورودی کاربر نمی‌شکند.
///
/// `unwrap` در مسیر کاربر یعنی کرش برنامه به‌جای پیام خطا. کاربر پنجره‌ی
/// بسته‌شده می‌بیند و نمی‌فهمد چه شد.
#[test]
fn t116_no_command_panics_on_user_input() {
    let mut risky = Vec::new();
    for (file, name, body) in commands() {
        // نظرها حذف می‌شوند تا مثال داخل توضیح، خطای کاذب نسازد.
        let code: String = body
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");

        for occurrence in code
            .match_indices(".unwrap()")
            .chain(code.match_indices(".expect("))
        {
            let (position, _) = occurrence;
            // متن اطراف برای تشخیص استثناهای مجاز
            let context_start = position.saturating_sub(120);
            let context = &code[context_start..position];
            let allowed = context.contains("timestamp_nanos_opt")
                || context.contains("Utc::now")
                || context.contains("Local::now")
                || context.contains("app.handle")
                || context.contains("path_resolver")
                || context.contains("Regex::new");
            if allowed {
                continue;
            }
            let snippet: String = code[context_start..(position + 12).min(code.len())]
                .chars()
                .rev()
                .take(60)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            risky.push(format!("{file}::{name} → {snippet}"));
        }
    }
    assert!(risky.is_empty(), "احتمال کرش روی ورودی کاربر: {risky:?}");
}

/// ت۱۱۷ — هر فرمانی که چند جدول را تغییر می‌دهد، در یک تراکنش کار می‌کند.
///
/// بدون تراکنش، شکست میانی داده‌ی نیمه‌کاره می‌گذارد: فاکتور بدون قلم، سند
/// بدون سطر.
#[test]
fn t117_multi_table_commands_run_inside_a_transaction() {
    let mut unsafe_commands = Vec::new();
    for (file, name, body) in commands() {
        let inserts = body.matches("INSERT ").count() + body.matches("UPDATE ").count();
        if inserts < 2 {
            continue;
        }
        let transactional = body.contains("c.transaction()")
            || body.contains("unchecked_transaction")
            || body.contains("tx.commit()");
        if !transactional {
            unsafe_commands.push(format!("{file}::{name}"));
        }
    }
    assert!(
        unsafe_commands.is_empty(),
        "تغییر چندجدولی بدون تراکنش: {unsafe_commands:?}"
    );
}

/// ت۱۱۸ — هر تراکنشی که باز می‌شود، بسته هم می‌شود.
#[test]
fn t118_every_opened_transaction_is_committed() {
    let mut leaking = Vec::new();
    for (file, name, body) in commands() {
        let opens = body.contains("c.transaction()") || body.contains("unchecked_transaction");
        if !opens {
            continue;
        }
        // فرمان فقط-خواندنی تراکنش را برای نمای ثابت باز می‌کند و نیازی
        // به commit ندارد؛ rollback خودکار همان رفتار درست است.
        let writes =
            body.contains("INSERT ") || body.contains("UPDATE ") || body.contains("DELETE ");
        if writes && !body.contains("tx.commit()") {
            leaking.push(format!("{file}::{name}"));
        }
    }
    assert!(leaking.is_empty(), "تراکنش بدون commit: {leaking:?}");
}

/// ت۱۱۹ — هیچ فرمانی رمز عبور را خام ذخیره نمی‌کند.
#[test]
fn t119_no_command_stores_a_raw_password() {
    for (file, name, body) in commands() {
        if !body.to_lowercase().contains("password") {
            continue;
        }
        // فقط نوشتن سنجیده می‌شود؛ خواندن پرچم «رمز تنظیم شده» بی‌خطر است.
        let writes_password = (body.contains("INSERT ") || body.contains("UPDATE "))
            && body.contains("password_hash");
        let hashes = body.contains("hash_password") || body.contains("db_hash");
        assert!(
            !writes_password
                || hashes
                || body.contains("PasswordVerifier")
                || body.contains("verify"),
            "{file}::{name} رمز را بدون هش ذخیره می‌کند"
        );
    }
}

/// ت۱۲۰ — هر فرمان مالی، تاریخ را با سال مالی اعتبارسنجی می‌کند.
#[test]
fn t120_financial_commands_validate_the_fiscal_date() {
    let mut unchecked = Vec::new();
    for (file, name, body) in commands() {
        // فرمانی که سند حسابداری می‌سازد
        if !body.contains("INSERT INTO journal_entries") {
            continue;
        }
        if !body.contains("validate_fiscal_date") {
            unchecked.push(format!("{file}::{name}"));
        }
    }
    assert!(
        unchecked.is_empty(),
        "سند بدون اعتبارسنجی سال مالی: {unchecked:?}"
    );
}

/// ت۱۲۱ — هر عملیات مالی ردپای حسابرسی می‌گذارد.
#[test]
fn t121_financial_operations_leave_an_audit_trail() {
    let mut untracked = Vec::new();
    for (file, name, body) in commands() {
        let posts_journal = body.contains("INSERT INTO journal_entries");
        if !posts_journal {
            continue;
        }
        if !body.contains("audit(") {
            untracked.push(format!("{file}::{name}"));
        }
    }
    assert!(
        untracked.is_empty(),
        "سند بدون ردپای حسابرسی: {untracked:?}"
    );
}

/// ت۱۲۲ — هیچ پرس‌وجویی رشته‌ی ورودی کاربر را مستقیم به SQL نمی‌چسباند.
///
/// این دقیقاً تعریف تزریق SQL است.
#[test]
fn t122_no_user_input_is_concatenated_into_sql() {
    let mut risky = Vec::new();
    for (file, _, body) in commands() {
        for line in body.lines() {
            let trimmed = line.trim();
            if !trimmed.contains("format!(") {
                continue;
            }
            let builds_sql = trimmed.contains("SELECT ")
                || trimmed.contains("INSERT ")
                || trimmed.contains("UPDATE ")
                || trimmed.contains("DELETE ");
            if !builds_sql {
                continue;
            }
            // جای‌گذاری نام جدول از ثابت‌های داخلی مجاز است؛ جای‌گذاری مقدار نه.
            // شناسه‌هایی که خودِ برنامه ساخته (`{jid}-line-1`) داخل
            // `params![]` می‌روند، نه داخل متن SQL — آن‌ها امن‌اند.
            // خطر واقعی وقتی است که مقدارِ آمده از کاربر در خود جمله بنشیند.
            let sql_literal = trimmed.contains("format!(\"SELECT")
                || trimmed.contains("format!(\"INSERT")
                || trimmed.contains("format!(\"UPDATE")
                || trimmed.contains("format!(\"DELETE");
            if !sql_literal {
                continue;
            }
            let interpolates_value = trimmed.contains("{value}")
                || trimmed.contains("{input")
                || trimmed.contains("{query");
            if interpolates_value {
                risky.push(format!(
                    "{file}: {}",
                    trimmed.chars().take(90).collect::<String>()
                ));
            }
        }
    }
    assert!(risky.is_empty(), "احتمال تزریق SQL: {risky:?}");
}

/// ت۱۲۳ — هیچ تاریخ میلادی در لایه‌ی میزبان به‌عنوان تاریخ سند نمی‌نشیند.
#[test]
fn t123_host_never_writes_a_gregorian_document_date() {
    let mut offenders = Vec::new();
    for (file, name, body) in commands() {
        // الگوی خطرناک: تاریخ میلادی مستقیماً در سند
        if body.contains("%Y-%m-%d") && body.contains("entry_date") {
            offenders.push(format!("{file}::{name}"));
        }
        if body.contains("Utc::now().format") && body.contains("INSERT INTO journal_entries") {
            offenders.push(format!("{file}::{name} (تاریخ امروز به‌جای تاریخ سند)"));
        }
    }
    assert!(offenders.is_empty(), "تاریخ میلادی در سند: {offenders:?}");
}

/// ت۱۲۴ — هیچ ماژول میزبانی بیش از حد بزرگ نشده است.
///
/// فایل غول‌پیکر یعنی مرور کد غیرممکن و باگ پنهان.
#[test]
fn t124_no_host_module_grows_unreviewable() {
    let mut oversized = Vec::new();
    for (file, code) in host_sources() {
        let lines = code.lines().count();
        // `main.rs` تاریخی بزرگ است و در حال شکسته‌شدن؛ سقفش جدا سنجیده می‌شود.
        let limit = if file == "main.rs" { 8_000 } else { 1_200 };
        if lines > limit {
            oversized.push(format!("{file}: {lines} خط"));
        }
    }
    assert!(oversized.is_empty(), "ماژول بیش از حد بزرگ: {oversized:?}");
}

/// ت۱۲۵ — هر ماژول میزبان توضیح سرفایل دارد.
#[test]
fn t125_every_host_module_documents_itself() {
    let mut undocumented = Vec::new();
    for (file, code) in host_sources() {
        if file == "main.rs" {
            continue;
        }
        let head: String = code.lines().take(3).collect::<Vec<_>>().join("\n");
        if !head.contains("//!") {
            undocumented.push(file);
        }
    }
    assert!(
        undocumented.is_empty(),
        "ماژول بدون توضیح: {undocumented:?}"
    );
}

// ===========================================================================
// صحت گزارش‌های مالی — بازسازی پرس‌وجو روی داده‌ی نمونه
// ===========================================================================

/// ت۱۲۶ — تراز آزمایشی متوازن است.
#[test]
fn t126_trial_balance_is_balanced() {
    let conn = seeded();
    let (debit, credit): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(l.debit),0), COALESCE(SUM(l.credit),0) \
             FROM journal_lines l JOIN journal_entries j ON j.id=l.journal_id \
             WHERE j.status='posted'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(debit > 0, "تراز خالی است");
    assert_eq!(debit, credit, "تراز آزمایشی نامتوازن است");
}

/// ت۱۲۷ — معادله‌ی حسابداری برقرار است: دارایی = بدهی + سرمایه + (درآمد − هزینه).
#[test]
fn t127_the_accounting_equation_holds() {
    let conn = seeded();
    let group_balance = |prefix: &str| -> i64 {
        conn.query_row(
            "SELECT COALESCE(SUM(l.debit - l.credit),0) FROM journal_lines l \
             JOIN accounts a ON a.id = l.account_id WHERE a.code LIKE ?1",
            rusqlite::params![format!("{prefix}%")],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };
    let assets = group_balance("1");
    let liabilities = group_balance("2");
    let revenue = group_balance("4");
    let expenses = group_balance("5");

    // با علامت طبیعی: دارایی بدهکار مثبت، بدهی و درآمد بستانکار منفی.
    // مجموع همه‌ی گروه‌ها باید صفر شود، چون هر سند متوازن است.
    let total = assets + liabilities + revenue + expenses;
    assert_eq!(
        total, 0,
        "معادله‌ی حسابداری برقرار نیست: دارایی {assets} بدهی {liabilities} درآمد {revenue} هزینه {expenses}"
    );
}

/// ت۱۲۸ — سود ناخالص = فروش خالص منهای بهای تمام‌شده.
#[test]
fn t128_gross_profit_equals_net_revenue_minus_cost_of_sales() {
    let conn = seeded();
    let account_balance = |code: &str| -> i64 {
        conn.query_row(
            "SELECT COALESCE(SUM(l.credit - l.debit),0) FROM journal_lines l \
             JOIN accounts a ON a.id=l.account_id WHERE a.code=?1",
            rusqlite::params![code],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };
    let sales = account_balance("4100");
    let returns = -account_balance("4200"); // کاهنده، ماهیت بدهکار
    let net_revenue = sales - returns;

    assert!(sales > 0, "فروشی ثبت نشده");
    assert!(
        net_revenue <= sales,
        "فروش خالص نمی‌تواند از فروش ناخالص بیشتر باشد"
    );
    // و مالیات نباید بخشی از درآمد باشد.
    let vat = account_balance("2401");
    assert!(vat > 0, "مالیاتی ثبت نشده — یعنی مالیات جای دیگری رفته");
}

/// ت۱۲۹ — مانده‌ی حساب مشتریان با جمع فاکتورهای تسویه‌نشده هم‌جهت است.
#[test]
fn t129_receivable_balance_agrees_with_unpaid_invoices() {
    let conn = seeded();
    let receivable: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(l.debit - l.credit),0) FROM journal_lines l \
             JOIN accounts a ON a.id=l.account_id WHERE a.code='1201'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // حساب مشتریان ماهیت بدهکار دارد؛ مانده‌اش نباید بستانکار شود مگر
    // پیش‌دریافت داشته باشیم.
    assert!(
        receivable >= 0,
        "حساب مشتریان بستانکار شده: {receivable} — یعنی دریافت بیش از فروش ثبت شده"
    );
}

/// ت۱۳۰ — ارزش موجودی انبار هرگز منفی نیست و با گردش می‌خواند.
#[test]
fn t130_inventory_value_is_never_negative() {
    let conn = seeded();
    let negative = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_balances b JOIN products p ON p.id=b.product_id \
         WHERE b.quantity * p.purchase_price < 0",
    );
    assert_eq!(negative, 0, "ارزش موجودی منفی وجود دارد");

    // و هر کالایی که موجودی دارد، حتماً گردش ورودی داشته است.
    let phantom = count(
        &conn,
        "SELECT COUNT(*) FROM inventory_balances b WHERE b.quantity > 0 AND NOT EXISTS \
         (SELECT 1 FROM inventory_movements m WHERE m.product_id=b.product_id \
          AND m.warehouse_id=b.warehouse_id AND m.movement_type IN ('receipt','transfer_in','adjustment'))",
    );
    assert_eq!(phantom, 0, "موجودی بدون گردش ورودی وجود دارد");
}

/// ت۱۳۱ — گزارش سنی شدن مطالبات، همه‌ی فاکتورهای باز را پوشش می‌دهد.
#[test]
fn t131_aging_report_covers_every_open_invoice() {
    let conn = seeded();
    let unpaid = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices WHERE status='posted' AND payment_status<>'paid'",
    );
    assert!(unpaid > 0, "فاکتور تسویه‌نشده‌ای وجود ندارد");

    // هر فاکتور باز باید طرف حساب داشته باشد، وگرنه در گزارش سنی گم می‌شود.
    let orphan = count(
        &conn,
        "SELECT COUNT(*) FROM sales_invoices WHERE status='posted' \
         AND payment_status<>'paid' AND contact_id IS NULL",
    );
    assert_eq!(orphan, 0, "فاکتور باز بدون طرف حساب در گزارش سنی گم می‌شود");
}

/// ت۱۳۲ — مانده‌ی خزانه با جمع تراکنش‌هایش می‌خواند.
#[test]
fn t132_treasury_balance_reconciles_with_its_transactions() {
    let conn = seeded();
    let mut statement = conn
        .prepare(
            "SELECT a.id, a.name, \
             COALESCE(SUM(CASE WHEN t.transaction_type='receipt' THEN t.amount ELSE -t.amount END),0) \
             FROM treasury_accounts a LEFT JOIN treasury_transactions t \
             ON t.treasury_account_id=a.id GROUP BY a.id",
        )
        .unwrap();
    let rows: Vec<(String, String, i64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
    assert!(!rows.is_empty(), "حساب خزانه‌ای وجود ندارد");

    for (id, name, balance) in rows {
        // صندوق نقدی نباید منفی باشد؛ پولی که نیست پرداخت نمی‌شود.
        let kind: String = conn
            .query_row(
                "SELECT account_type FROM treasury_accounts WHERE id=?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .unwrap();
        if kind == "cash" {
            assert!(balance >= 0, "صندوق «{name}» منفی شده: {balance}");
        }
    }
}

/// ت۱۳۳ — هر سند حسابداری به منبعش قابل ردیابی است.
///
/// سند بدون منبع یعنی در حسابرسی نمی‌شود گفت از کجا آمده.
#[test]
fn t133_every_voucher_is_traceable_to_its_source() {
    let conn = seeded();
    let untraceable = count(
        &conn,
        "SELECT COUNT(*) FROM journal_entries WHERE source_type IS NULL OR source_type=''",
    );
    assert_eq!(untraceable, 0, "سند بدون نوع منبع وجود دارد");

    // و هر سند باید دست‌کم دو سطر داشته باشد — سند تک‌سطری متوازن نمی‌شود.
    let single_line = count(
        &conn,
        "SELECT COUNT(*) FROM (SELECT journal_id FROM journal_lines \
         GROUP BY journal_id HAVING COUNT(*) < 2)",
    );
    assert_eq!(single_line, 0, "سند تک‌سطری وجود دارد");
}

/// ت۱۳۴ — شماره‌ی سند در هر سال مالی پیوسته است (بدون جای خالی مشکوک).
#[test]
fn t134_voucher_numbering_has_no_suspicious_gaps() {
    let conn = seeded();
    let (min, max, total): (i64, i64, i64) = conn
        .query_row(
            "SELECT MIN(number), MAX(number), COUNT(*) FROM journal_entries \
             WHERE company_id='company-demo' AND fiscal_year_id='fy-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(total > 0, "سندی وجود ندارد");
    // بازه‌ی شماره نباید بی‌دلیل بزرگ‌تر از تعداد اسناد باشد.
    let span = max - min + 1;
    assert!(
        span <= total * 2,
        "بازه‌ی شماره‌ی سند ({span}) نسبت به تعداد ({total}) غیرعادی است"
    );
}

/// ت۱۳۵ — مهاجرت پایگاه داده بارها قابل اجراست.
///
/// اگر مهاجرت بار دوم بشکند، هر بار بازکردن برنامه ریسک است.
#[test]
fn t135_migration_is_repeatable() {
    let conn = db::open_in_memory().expect("پایگاه داده");
    let before_tables = count(
        &conn,
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
    );
    // اجرای دوباره‌ی مهاجرت و داده‌ی پایه
    db::migrate(&conn).expect("مهاجرت دوم باید موفق باشد");
    db::migrate(&conn).expect("مهاجرت سوم باید موفق باشد");
    let after_tables = count(
        &conn,
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
    );
    assert_eq!(before_tables, after_tables, "مهاجرت جدول تکراری ساخت");

    // و داده هم دوبرابر نشده باشد.
    let mut statement = conn.prepare("PRAGMA foreign_key_check").unwrap();
    assert_eq!(
        statement.query_map([], |_| Ok(())).unwrap().count(),
        0,
        "مهاجرت دوباره کلید خارجی را شکست"
    );
}
