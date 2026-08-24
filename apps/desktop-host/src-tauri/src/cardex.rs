//! دستور میزبان کاردکس کالا — F4 فروش / F5 خرید / F6 کلی.
//!
//! مرجع: لیست کالاهای نرم‌افزار فعلی (تصویر `8Xmc1p`).
//!
//! ## چرا این ماژول این‌قدر نازک است
//!
//! همه‌ی منطق (جهت حرکت‌ها، تفکیک فروش/خرید، افتتاحیه و ماند سطری) در
//! `novin_core::cardex` زندگی می‌کند تا همان‌جا در CI تست شود و هر گزارش
//! دیگری که فردا به آن نیاز داشت — چاپ، خروجی، داشبورد — به همان
//! تک‌منبعِ محاسبه وصل باشد، نه به یک کپی تازه در میزبان.
//!
//! تاریخ از رابط کاربری به شمسی می‌آید و همین‌جا یک بار به میلادی ISO
//! (قالب ذخیره‌سازی) تبدیل می‌شود؛ خطای تاریخ با کد `CRDX-003` برمی‌گردد.

use novin_core::cardex::{self, CardexFilter, CardexKind, CardexReport};
use rusqlite::params;
use tauri::State;

use crate::{conn, require_login, AppState};

/// گزارش کاردکس یک کالا.
///
/// - `kind`: `sales` (F4) · `purchase` (F5) · `all` (F6)
/// - `from_jalali` / `to_jalali`: بازه‌ی شمسی مثل `1404/01/01`
/// - `warehouse_id`: اختیاری؛ خالی یعنی همه‌ی انبارها
#[tauri::command]
pub fn product_cardex(
    state: State<AppState>,
    product_id: String,
    kind: String,
    from_jalali: String,
    to_jalali: String,
    warehouse_id: Option<String>,
) -> Result<CardexReport, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;

    let kind = CardexKind::parse(&kind).map_err(|error| format!("CRDX-001: {error}"))?;
    let from = novin_core::jalali::JalaliDate::parse(&from_jalali)
        .and_then(|date| date.to_gregorian())
        .map_err(|error| format!("CRDX-003: {error}"))?;
    let to = novin_core::jalali::JalaliDate::parse(&to_jalali)
        .and_then(|date| date.to_gregorian())
        .map_err(|error| format!("CRDX-003: {error}"))?;

    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |row| row.get(0),
        )
        .map_err(|_| "AUTH-403: دسترسی به شرکت وجود ندارد".to_string())?;

    let filter = CardexFilter {
        company_id: company,
        product_id,
        kind,
        from,
        to,
        warehouse_id,
    };
    cardex::cardex(&c, &filter).map_err(|error| format!("CRDX-004: {error}"))
}
