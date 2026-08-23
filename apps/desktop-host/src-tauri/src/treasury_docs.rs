//! سند دریافت و پرداخت چندروشی.
//!
//! ## چرا ماژول جدا
//!
//! سند خزانه تنها جایی است که شش روش تسویه، چک، حساب طرف مقابل و سند
//! حسابداری هم‌زمان درگیر می‌شوند. نگه داشتن این منطق در `main.rs` همان
//! Monolith است که از آن پرهیز می‌کنیم.
//!
//! ## قواعد حسابداری پیاده‌شده
//!
//! ```text
//! سند دریافت:
//!   بدهکار  صندوق / بانک / کارتخوان        (پول واقعاً جابه‌جا شد)
//!   بدهکار  اسناد دریافتنی                 (چک دریافتی — هنوز پول نیست)
//!   بدهکار  تخفیف نقدی اعطایی              (کاهش درآمد، نه دریافت)
//!   بستانکار طرف حساب                      جمع کل
//!
//! سند پرداخت: دقیقاً معکوس.
//! ```
//!
//! نکته‌ی کلیدی: **چک به صندوق نمی‌رود.** چک دریافتی تا وصول نشود پول نیست،
//! پس به «اسناد دریافتنی» می‌نشیند و در همین سند یک رکورد چک با وضعیت
//! «موجود» ساخته می‌شود تا در چرخه‌ی چک قابل پیگیری باشد.
//!
//! «تهاتر» و «تخفیف» پول جابه‌جا نمی‌کنند، پس در گردش خزانه ثبت نمی‌شوند.

use novin_core::money::Money;
use novin_core::treasury::{
    build_journal, calculate_totals, CheckDetails, DocumentKind, DocumentLine, PaymentMethod,
};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    active_context, audit, conn, next_journal_number, require_permission, validate_fiscal_date,
    AppState,
};

/// یک سطر سند، همان‌طور که از فرم می‌آید.
///
/// عمداً از رشته استفاده می‌کنیم تا لایه‌ی رابط کاربری به enum های Rust وابسته
/// نشود؛ اعتبارسنجی همین‌جا و با واژه‌نامه‌ی هسته انجام می‌شود.
#[derive(Debug, Clone, Deserialize)]
pub struct DocumentLineInput {
    pub method: String,
    pub amount: i64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub treasury_account_id: Option<String>,
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub check_serial: Option<String>,
    #[serde(default)]
    pub check_due_date: Option<String>,
    #[serde(default)]
    pub check_bank_name: Option<String>,
    #[serde(default)]
    pub sayad_id: Option<String>,
}

/// خلاصه‌ی محاسبه‌شده‌ی سند — برای پیش‌نمایش زنده در فرم.
#[derive(Debug, Serialize)]
pub struct DocumentPreview {
    pub cash: i64,
    pub check: i64,
    pub bank_transfer: i64,
    pub card_terminal: i64,
    pub discount: i64,
    pub offset: i64,
    pub total: i64,
    /// بخشی که واقعاً موجودی خزانه را جابه‌جا می‌کند.
    pub treasury_movement: i64,
    /// سطرهای سند حسابداری که در صورت ثبت صادر خواهد شد.
    pub journal_preview: Vec<JournalPreviewLine>,
}

#[derive(Debug, Serialize)]
pub struct JournalPreviewLine {
    pub account_id: String,
    pub account_name: String,
    pub debit: i64,
    pub credit: i64,
}

/// سطر فهرست اسناد خزانه.
#[derive(Debug, Serialize)]
pub struct TreasuryDocumentRow {
    pub id: String,
    pub kind: String,
    pub kind_label: String,
    pub number: i64,
    pub document_date: String,
    pub party_id: Option<String>,
    pub party_name: Option<String>,
    pub description: Option<String>,
    pub total: i64,
    pub status: String,
    pub status_label: String,
    pub journal_id: Option<String>,
    pub line_count: i64,
}

/// جزئیات کامل یک سند برای نمایش.
#[derive(Debug, Serialize)]
pub struct TreasuryDocumentDetail {
    pub header: TreasuryDocumentRow,
    pub lines: Vec<TreasuryDocumentLineRow>,
}

#[derive(Debug, Serialize)]
pub struct TreasuryDocumentLineRow {
    pub id: String,
    pub method: String,
    pub method_label: String,
    pub amount: i64,
    pub description: Option<String>,
    pub treasury_account_id: Option<String>,
    pub treasury_account_name: Option<String>,
    pub terminal_id: Option<String>,
    pub check_serial: Option<String>,
    pub check_due_date: Option<String>,
    pub check_bank_name: Option<String>,
    pub sayad_id: Option<String>,
    pub check_id: Option<String>,
}

fn parse_kind(kind: &str) -> Result<DocumentKind, String> {
    match kind {
        "receipt" => Ok(DocumentKind::Receipt),
        "payment" => Ok(DocumentKind::Payment),
        _ => Err("TDOC-001: نوع سند باید دریافت یا پرداخت باشد".into()),
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "draft" => "پیش‌نویس",
        "posted" => "ثبت‌شده",
        "cancelled" => "باطل‌شده",
        _ => "نامشخص",
    }
}

/// تبدیل ورودی فرم به سطر هسته، با پیام خطای گویا به‌ازای هر مشکل.
fn to_core_line(input: &DocumentLineInput, index: usize) -> Result<DocumentLine, String> {
    let row = index + 1;
    let method = PaymentMethod::parse(&input.method)
        .ok_or_else(|| format!("TDOC-002: روش تسویه‌ی سطر {row} شناخته نمی‌شود"))?;
    if input.amount <= 0 {
        return Err(format!("TDOC-003: مبلغ سطر {row} باید بیشتر از صفر باشد"));
    }
    let mut line = DocumentLine::new(method, Money::from_rials(input.amount));
    line.description = input.description.clone();
    line.treasury_account = input.treasury_account_id.clone();
    line.terminal_id = input.terminal_id.clone();
    if method == PaymentMethod::Check {
        let serial = input
            .check_serial
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("TDOC-004: شماره‌ی چک سطر {row} الزامی است"))?;
        let due = input
            .check_due_date
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("TDOC-005: سررسید چک سطر {row} الزامی است"))?;
        line.check = Some(CheckDetails {
            serial: serial.to_string(),
            due_date: due.to_string(),
            bank_name: input.check_bank_name.clone(),
            sayad_id: input.sayad_id.clone(),
        });
    }
    Ok(line)
}

/// حساب‌های موردنیاز سند، خوانده‌شده از کدینگ همان شرکت.
struct ResolvedAccounts {
    party_account: String,
    notes_receivable: String,
    notes_payable: String,
    discount_account: String,
}

fn account_by_code(
    tx: &rusqlite::Transaction<'_>,
    company: &str,
    code: &str,
    error: &str,
) -> Result<String, String> {
    tx.query_row(
        "SELECT id FROM accounts WHERE company_id=?1 AND code=?2 AND is_active=1",
        params![company, code],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| error.to_string())
}

fn resolve_accounts(
    tx: &rusqlite::Transaction<'_>,
    company: &str,
    kind: DocumentKind,
) -> Result<ResolvedAccounts, String> {
    // طرف حساب: دریافت از مشتری → حساب مشتریان، پرداخت به تأمین‌کننده → تأمین‌کنندگان.
    let party_code = match kind {
        DocumentKind::Receipt => "1201",
        DocumentKind::Payment => "2101",
    };
    Ok(ResolvedAccounts {
        party_account: account_by_code(
            tx,
            company,
            party_code,
            "TDOC-006: حساب طرف مقابل در کدینگ تعریف نشده است",
        )?,
        notes_receivable: account_by_code(
            tx,
            company,
            "1103",
            "TDOC-007: حساب اسناد دریافتنی تعریف نشده است",
        )?,
        notes_payable: account_by_code(
            tx,
            company,
            "2103",
            "TDOC-008: حساب اسناد پرداختنی تعریف نشده است",
        )?,
        discount_account: account_by_code(
            tx,
            company,
            "4400",
            "TDOC-009: حساب تخفیف نقدی تعریف نشده است",
        )?,
    })
}

fn core_accounts(resolved: &ResolvedAccounts) -> novin_core::treasury::TreasuryAccounts {
    novin_core::treasury::TreasuryAccounts {
        party_account: resolved.party_account.clone(),
        notes_receivable: resolved.notes_receivable.clone(),
        notes_payable: resolved.notes_payable.clone(),
        discount_account: resolved.discount_account.clone(),
    }
}

/// پیش‌نمایش سند: جمع‌ها و سند حسابداری، **بدون** هیچ نوشتنی در پایگاه داده.
///
/// همان اعداد و همان سند حسابداری که هنگام ثبت صادر می‌شود؛ پس آنچه کاربر
/// می‌بیند دقیقاً همان چیزی است که ثبت خواهد شد.
#[tauri::command]
pub fn preview_treasury_document(
    state: State<AppState>,
    kind: String,
    lines: Vec<DocumentLineInput>,
) -> Result<DocumentPreview, String> {
    let document_kind = parse_kind(&kind)?;
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.view")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let core_lines = lines
        .iter()
        .enumerate()
        .map(|(index, line)| to_core_line(line, index))
        .collect::<Result<Vec<_>, _>>()?;
    let totals = calculate_totals(&core_lines).map_err(|e| format!("TDOC-010: {e}"))?;

    let resolved = resolve_accounts(&tx, &company, document_kind)?;
    let journal = build_journal(document_kind, &core_lines, &core_accounts(&resolved))
        .map_err(|e| format!("TDOC-011: {e}"))?;

    let mut journal_preview = Vec::with_capacity(journal.len());
    for line in &journal {
        let name: String = tx
            .query_row(
                "SELECT name FROM accounts WHERE id=?1",
                params![line.account_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| line.account_id.clone());
        journal_preview.push(JournalPreviewLine {
            account_id: line.account_id.clone(),
            account_name: name,
            debit: line.debit.rials(),
            credit: line.credit.rials(),
        });
    }

    Ok(DocumentPreview {
        cash: totals.cash.rials(),
        check: totals.check.rials(),
        bank_transfer: totals.bank_transfer.rials(),
        card_terminal: totals.card_terminal.rials(),
        discount: totals.discount.rials(),
        offset: totals.offset.rials(),
        total: totals.total.rials(),
        treasury_movement: totals.treasury_movement.rials(),
        journal_preview,
    })
}

/// ثبت سند دریافت یا پرداخت چندروشی.
///
/// همه‌چیز در یک تراکنش انجام می‌شود: سند خزانه، سطرها، چک‌های تولیدشده،
/// سند حسابداری و گردش خزانه. اگر هر مرحله شکست بخورد، هیچ‌چیز ثبت نمی‌شود.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_treasury_document(
    state: State<AppState>,
    kind: String,
    document_date: String,
    party_id: String,
    description: Option<String>,
    lines: Vec<DocumentLineInput>,
) -> Result<String, String> {
    let document_kind = parse_kind(&kind)?;
    let permission = match document_kind {
        DocumentKind::Receipt => "treasury.receipt.create",
        DocumentKind::Payment => "treasury.payment.create",
    };
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, permission)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;
    validate_fiscal_date(&tx, &fy, &document_date)?;

    let party_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM contacts WHERE id=?1 AND company_id=?2",
            params![party_id, company],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if party_exists == 0 {
        return Err("TDOC-012: طرف حساب سند معتبر نیست".into());
    }

    let core_lines = lines
        .iter()
        .enumerate()
        .map(|(index, line)| to_core_line(line, index))
        .collect::<Result<Vec<_>, _>>()?;
    let totals = calculate_totals(&core_lines).map_err(|e| format!("TDOC-010: {e}"))?;

    // حساب خزانه‌ی هر سطر باید متعلق به همین شرکت و فعال باشد.
    for (index, line) in lines.iter().enumerate() {
        if let Some(account) = line
            .treasury_account_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let ok: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM treasury_accounts WHERE id=?1 AND company_id=?2 AND is_active=1",
                    params![account, company],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if ok == 0 {
                return Err(format!(
                    "TDOC-013: حساب خزانه‌ی سطر {} معتبر نیست",
                    index + 1
                ));
            }
        }
    }

    let resolved = resolve_accounts(&tx, &company, document_kind)?;
    let journal_lines = build_journal(document_kind, &core_lines, &core_accounts(&resolved))
        .map_err(|e| format!("TDOC-011: {e}"))?;

    let stamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let document_id = format!("tdoc-{stamp}");
    let number: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(number),0)+1 FROM treasury_documents \
             WHERE company_id=?1 AND fiscal_year_id=?2 AND kind=?3",
            params![company, fy, document_kind.as_str()],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // سند حسابداری اول صادر می‌شود تا شناسه‌اش روی سند خزانه بنشیند.
    let journal_id = format!("journal-tdoc-{stamp}");
    let journal_number = next_journal_number(&tx, &company, &fy)?;
    let journal_description = description.clone().unwrap_or_else(|| {
        format!(
            "{} شماره {number}",
            match document_kind {
                DocumentKind::Receipt => "سند دریافت",
                DocumentKind::Payment => "سند پرداخت",
            }
        )
    });
    tx.execute(
        "INSERT INTO journal_entries(id,company_id,fiscal_year_id,number,entry_date,description,\
         status,source_type,source_id,created_by) VALUES(?1,?2,?3,?4,?5,?6,'posted',?7,?8,?9)",
        params![
            journal_id,
            company,
            fy,
            journal_number,
            document_date,
            journal_description,
            document_kind.as_str(),
            document_id,
            user
        ],
    )
    .map_err(|e| format!("TDOC-014: {e}"))?;
    for (index, line) in journal_lines.iter().enumerate() {
        tx.execute(
            "INSERT INTO journal_lines(id,journal_id,account_id,description,debit,credit) \
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                format!("{journal_id}-l{index}"),
                journal_id,
                line.account_id,
                journal_description,
                line.debit.rials(),
                line.credit.rials()
            ],
        )
        .map_err(|e| format!("TDOC-015: {e}"))?;
    }

    tx.execute(
        "INSERT INTO treasury_documents(id,company_id,fiscal_year_id,kind,number,document_date,\
         party_id,description,total,status,journal_id,created_by) \
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'posted',?10,?11)",
        params![
            document_id,
            company,
            fy,
            document_kind.as_str(),
            number,
            document_date,
            party_id,
            description,
            totals.total.rials(),
            journal_id,
            user
        ],
    )
    .map_err(|e| format!("TDOC-016: {e}"))?;

    for (index, (input, core)) in lines.iter().zip(core_lines.iter()).enumerate() {
        let line_id = format!("{document_id}-l{index}");
        // چک واقعی ساخته می‌شود تا در چرخه‌ی چک قابل پیگیری باشد.
        let check_id = if core.method == PaymentMethod::Check {
            let details = core
                .check
                .as_ref()
                .ok_or_else(|| format!("TDOC-004: شماره‌ی چک سطر {} الزامی است", index + 1))?;
            let check_type = match document_kind {
                DocumentKind::Receipt => "received",
                DocumentKind::Payment => "issued",
            };
            let duplicate: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM checks WHERE company_id=?1 AND check_type=?2 \
                     AND check_number=?3 AND status<>'void'",
                    params![company, check_type, details.serial],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if duplicate > 0 {
                return Err(format!(
                    "TDOC-017: شماره‌ی چک «{}» قبلاً ثبت شده است",
                    details.serial
                ));
            }
            let kind_enum = match document_kind {
                DocumentKind::Receipt => novin_core::checks::CheckKind::Received,
                DocumentKind::Payment => novin_core::checks::CheckKind::Issued,
            };
            let initial = novin_core::checks::CheckStatus::initial(kind_enum, false);
            let check_id = format!("{line_id}-check");
            tx.execute(
                "INSERT INTO checks(id,company_id,fiscal_year_id,check_type,check_number,party_id,\
                 treasury_account_id,amount,issue_date,due_date,status,bank_name,description,created_by) \
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    check_id,
                    company,
                    fy,
                    check_type,
                    details.serial,
                    party_id,
                    input.treasury_account_id,
                    core.amount.rials(),
                    document_date,
                    details.due_date,
                    initial.as_str(),
                    details.bank_name,
                    journal_description,
                    user
                ],
            )
            .map_err(|e| format!("TDOC-018: {e}"))?;
            Some(check_id)
        } else {
            None
        };

        tx.execute(
            "INSERT INTO treasury_document_lines(id,document_id,method,amount,description,\
             treasury_account_id,terminal_id,check_serial,check_due_date,check_bank_name,\
             sayad_id,check_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                line_id,
                document_id,
                core.method.as_str(),
                core.amount.rials(),
                input.description,
                input.treasury_account_id,
                input.terminal_id,
                input.check_serial,
                input.check_due_date,
                input.check_bank_name,
                input.sayad_id,
                check_id
            ],
        )
        .map_err(|e| format!("TDOC-019: {e}"))?;

        // فقط روش‌هایی که واقعاً پول جابه‌جا می‌کنند در گردش خزانه می‌نشینند.
        if core.method.moves_treasury() {
            let account = input
                .treasury_account_id
                .as_deref()
                .ok_or_else(|| format!("TDOC-020: حساب خزانه‌ی سطر {} خالی است", index + 1))?;
            let transaction_type = match document_kind {
                DocumentKind::Receipt => "receipt",
                DocumentKind::Payment => "payment",
            };
            tx.execute(
                "INSERT INTO treasury_transactions(id,company_id,fiscal_year_id,treasury_account_id,\
                 transaction_type,amount,transaction_date,description,reference_type,reference_id,\
                 journal_id,created_by) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'treasury_document',?9,?10,?11)",
                params![
                    format!("{line_id}-tx"),
                    company,
                    fy,
                    account,
                    transaction_type,
                    core.amount.rials(),
                    document_date,
                    journal_description,
                    document_id,
                    journal_id,
                    user
                ],
            )
            .map_err(|e| format!("TDOC-021: {e}"))?;
        }
    }

    audit(
        &tx,
        &user,
        "treasury.document.create",
        "treasury_document",
        &document_id,
        None,
        Some(&format!(
            "{{\"kind\":\"{}\",\"number\":{number},\"total\":{}}}",
            document_kind.as_str(),
            totals.total.rials()
        )),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(document_id)
}

/// فهرست اسناد خزانه با فیلتر نوع، بازه‌ی تاریخ و طرف حساب.
#[tauri::command]
pub fn list_treasury_documents(
    state: State<AppState>,
    kind: Option<String>,
    party_id: Option<String>,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<Vec<TreasuryDocumentRow>, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.view")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, fy) = active_context(&tx, &user)?;

    let mut sql = String::from(
        "SELECT d.id,d.kind,d.number,d.document_date,d.party_id,c.name,d.description,d.total,\
         d.status,d.journal_id,(SELECT COUNT(*) FROM treasury_document_lines l WHERE l.document_id=d.id) \
         FROM treasury_documents d LEFT JOIN contacts c ON c.id=d.party_id \
         WHERE d.company_id=?1 AND d.fiscal_year_id=?2",
    );
    let mut values: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(company.clone()), Box::new(fy.clone())];
    if let Some(value) = kind.filter(|v| !v.trim().is_empty()) {
        parse_kind(&value)?;
        values.push(Box::new(value));
        sql.push_str(&format!(" AND d.kind=?{}", values.len()));
    }
    if let Some(value) = party_id.filter(|v| !v.trim().is_empty()) {
        values.push(Box::new(value));
        sql.push_str(&format!(" AND d.party_id=?{}", values.len()));
    }
    if let Some(value) = from_date.filter(|v| !v.trim().is_empty()) {
        values.push(Box::new(value));
        sql.push_str(&format!(" AND d.document_date>=?{}", values.len()));
    }
    if let Some(value) = to_date.filter(|v| !v.trim().is_empty()) {
        values.push(Box::new(value));
        sql.push_str(&format!(" AND d.document_date<=?{}", values.len()));
    }
    sql.push_str(" ORDER BY d.document_date DESC, d.number DESC LIMIT 500");

    let mut statement = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let bound: Vec<&dyn rusqlite::ToSql> = values.iter().map(|value| value.as_ref()).collect();
    let rows = statement
        .query_map(bound.as_slice(), |row| {
            let kind: String = row.get(1)?;
            let status: String = row.get(8)?;
            Ok(TreasuryDocumentRow {
                id: row.get(0)?,
                kind_label: if kind == "receipt" {
                    "دریافت".into()
                } else {
                    "پرداخت".into()
                },
                kind,
                number: row.get(2)?,
                document_date: row.get(3)?,
                party_id: row.get(4)?,
                party_name: row.get(5)?,
                description: row.get(6)?,
                total: row.get(7)?,
                status_label: status_label(&status).to_string(),
                status,
                journal_id: row.get(9)?,
                line_count: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

/// جزئیات یک سند خزانه همراه با همه‌ی سطرها.
#[tauri::command]
pub fn get_treasury_document(
    state: State<AppState>,
    id: String,
) -> Result<TreasuryDocumentDetail, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "treasury.check.view")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let (company, _) = active_context(&tx, &user)?;

    let header = tx
        .query_row(
            "SELECT d.id,d.kind,d.number,d.document_date,d.party_id,c.name,d.description,d.total,\
             d.status,d.journal_id,(SELECT COUNT(*) FROM treasury_document_lines l WHERE l.document_id=d.id) \
             FROM treasury_documents d LEFT JOIN contacts c ON c.id=d.party_id \
             WHERE d.id=?1 AND d.company_id=?2",
            params![id, company],
            |row| {
                let kind: String = row.get(1)?;
                let status: String = row.get(8)?;
                Ok(TreasuryDocumentRow {
                    id: row.get(0)?,
                    kind_label: if kind == "receipt" {
                        "دریافت".into()
                    } else {
                        "پرداخت".into()
                    },
                    kind,
                    number: row.get(2)?,
                    document_date: row.get(3)?,
                    party_id: row.get(4)?,
                    party_name: row.get(5)?,
                    description: row.get(6)?,
                    total: row.get(7)?,
                    status_label: status_label(&status).to_string(),
                    status,
                    journal_id: row.get(9)?,
                    line_count: row.get(10)?,
                })
            },
        )
        .map_err(|_| "TDOC-022: سند یافت نشد".to_string())?;

    let mut statement = tx
        .prepare(
            "SELECT l.id,l.method,l.amount,l.description,l.treasury_account_id,t.name,\
             l.terminal_id,l.check_serial,l.check_due_date,l.check_bank_name,l.sayad_id,l.check_id \
             FROM treasury_document_lines l \
             LEFT JOIN treasury_accounts t ON t.id=l.treasury_account_id \
             WHERE l.document_id=?1 ORDER BY l.id",
        )
        .map_err(|e| e.to_string())?;
    let lines = statement
        .query_map(params![id], |row| {
            let method: String = row.get(1)?;
            Ok(TreasuryDocumentLineRow {
                id: row.get(0)?,
                method_label: PaymentMethod::parse(&method)
                    .map(|m| m.label().to_string())
                    .unwrap_or_else(|| method.clone()),
                method,
                amount: row.get(2)?,
                description: row.get(3)?,
                treasury_account_id: row.get(4)?,
                treasury_account_name: row.get(5)?,
                terminal_id: row.get(6)?,
                check_serial: row.get(7)?,
                check_due_date: row.get(8)?,
                check_bank_name: row.get(9)?,
                sayad_id: row.get(10)?,
                check_id: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();

    Ok(TreasuryDocumentDetail { header, lines })
}

/// روش‌های تسویه‌ی در دسترس، با برچسب فارسی و نیازمندی‌های هر روش.
///
/// رابط کاربری این فهرست را از backend می‌گیرد تا هیچ روشی در فرم باشد که
/// موتور نمی‌شناسد، و هیچ فیلد اجباری‌ای جا نیفتد.
#[derive(Debug, Serialize)]
pub struct PaymentMethodInfo {
    pub value: String,
    pub label: String,
    pub requires_treasury_account: bool,
    pub requires_terminal: bool,
    pub requires_check_details: bool,
    pub moves_treasury: bool,
}

#[tauri::command]
pub fn list_payment_methods() -> Vec<PaymentMethodInfo> {
    [
        PaymentMethod::Cash,
        PaymentMethod::Check,
        PaymentMethod::BankTransfer,
        PaymentMethod::CardTerminal,
        PaymentMethod::Discount,
        PaymentMethod::Offset,
    ]
    .into_iter()
    .map(|method| PaymentMethodInfo {
        value: method.as_str().to_string(),
        label: method.label().to_string(),
        requires_treasury_account: method.requires_treasury_account(),
        requires_terminal: method == PaymentMethod::CardTerminal,
        requires_check_details: method == PaymentMethod::Check,
        moves_treasury: method.moves_treasury(),
    })
    .collect()
}
