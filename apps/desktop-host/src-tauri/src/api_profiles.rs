//! اتصال‌های API خارجی — پروفایل، اجرای درخواست و کلیدهای secret.
//!
//! استخراج‌شده از main.rs در ممیزی دور ۱۳ (C-03) — منطق بدون تغییر.
//! اسرار هرگز در پایگاه داده ذخیره نمی‌شوند؛ keyring سیستم‌عامل میزبان است.

use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::{audit, conn, require_login, require_permission, AppState};

#[derive(Serialize)]
struct ApiProfile {
    id: String,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: Option<String>,
    timeout_ms: i64,
    enabled: bool,
    allowed_domains: String,
}
#[derive(Serialize)]
struct ApiResponse {
    status: u16,
    body: String,
    content_type: Option<String>,
}

#[tauri::command]
pub fn list_api_profiles(state: State<AppState>) -> Result<Vec<ApiProfile>, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "integrations.view")?;
    let mut st=c.prepare("SELECT p.id,p.name,p.base_url,p.auth_type,p.auth_header,p.timeout_ms,p.enabled,p.allowed_domains FROM api_profiles p JOIN company_users cu ON cu.company_id=p.company_id WHERE cu.user_id=?1 AND cu.is_active=1 ORDER BY p.name").map_err(|e|e.to_string())?;
    let rows = st
        .query_map(params![user], |r| {
            Ok(ApiProfile {
                id: r.get(0)?,
                name: r.get(1)?,
                base_url: r.get(2)?,
                auth_type: r.get(3)?,
                auth_header: r.get(4)?,
                timeout_ms: r.get(5)?,
                enabled: r.get::<_, i64>(6)? != 0,
                allowed_domains: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub(crate) fn api_secret_key(profile_id: &str) -> String {
    format!("api-profile:{profile_id}")
}

#[tauri::command]
#[tauri::command]
pub fn create_api_profile(
    state: State<AppState>,
    name: String,
    base_url: String,
    auth_type: String,
    auth_header: Option<String>,
    timeout_ms: i64,
    allowed_domains: String,
    secret: Option<String>,
) -> Result<String, String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "integrations.manage")?;
    if name.trim().is_empty() {
        return Err("API-001: نام اتصال الزامی است".into());
    }
    if !matches!(auth_type.as_str(), "none" | "api_key" | "bearer" | "basic") {
        return Err("API-002: نوع احراز هویت نامعتبر است".into());
    }
    let base =
        reqwest::Url::parse(&base_url).map_err(|_| "API-003: آدرس پایه نامعتبر است".to_string())?;
    if base.scheme() != "https" {
        return Err("API-004: فقط HTTPS برای اتصال خارجی مجاز است".into());
    }
    let host = base
        .host_str()
        .ok_or_else(|| "API-005: دامنه آدرس مشخص نیست".to_string())?;
    let domains = if allowed_domains.trim().is_empty() {
        host.to_string()
    } else {
        allowed_domains
    };
    if !domains.split(',').map(str::trim).any(|d| d == host) {
        return Err("API-006: دامنه Base URL باید در Allowed Domains باشد".into());
    }
    if !(1000..=120000).contains(&timeout_ms) {
        return Err("API-007: Timeout باید بین ۱ تا ۱۲۰ ثانیه باشد".into());
    }
    let company: String = c
        .query_row(
            "SELECT company_id FROM company_users WHERE user_id=?1 AND is_active=1 LIMIT 1",
            params![user],
            |r| r.get(0),
        )
        .map_err(|_| "API-008: شرکت فعال یافت نشد".to_string())?;
    let id = format!(
        "api-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    c.execute("INSERT INTO api_profiles(id,company_id,name,base_url,auth_type,auth_header,timeout_ms,allowed_domains) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,company,name,base_url,auth_type,auth_header,timeout_ms,domains]).map_err(|e|format!("API-009: ثبت اتصال انجام نشد: {e}"))?;
    if let Some(secret) = secret {
        if !secret.is_empty() {
            let entry = keyring::Entry::new("novin-pardaz-accounting", &api_secret_key(&id))
                .map_err(|e| format!("API-010: دسترسی Secret Storage ممکن نیست: {e}"))?;
            entry
                .set_password(&secret)
                .map_err(|e| format!("API-011: ذخیره Secret انجام نشد: {e}"))?;
        }
    }
    Ok(id)
}

#[tauri::command]
#[tauri::command]
pub fn execute_api_request(
    state: State<AppState>,
    profile_id: String,
    method: String,
    path: String,
    headers_json: Option<String>,
    body: Option<String>,
) -> Result<ApiResponse, String> {
    let user = require_login(&state)?;
    let c = conn(&state)?;
    require_permission(&state, &c, "integrations.execute")?;
    let p:(String,String,String,Option<String>,i64,bool)=c.query_row("SELECT base_url,auth_type,allowed_domains,auth_header,timeout_ms,enabled FROM api_profiles p JOIN company_users cu ON cu.company_id=p.company_id WHERE p.id=?1 AND cu.user_id=?2 AND cu.is_active=1",params![profile_id,user],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get::<_, i32>(5)? != 0))).map_err(|_|"API-012: اتصال API پیدا نشد".to_string())?;
    if !p.5 {
        return Err("API-013: اتصال API غیرفعال است".into());
    }
    let base =
        reqwest::Url::parse(&p.0).map_err(|_| "API-014: Base URL نامعتبر است".to_string())?;
    let url = base
        .join(path.trim_start_matches('/'))
        .map_err(|_| "API-015: مسیر درخواست نامعتبر است".to_string())?;
    let host = url
        .host_str()
        .ok_or_else(|| "API-016: دامنه مقصد مشخص نیست".to_string())?;
    if !p.2.split(',').map(str::trim).any(|d| d == host) {
        return Err("API-017: دامنه مقصد در Allowlist نیست".into());
    }
    let m = match method.to_uppercase().as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "PATCH" => reqwest::Method::PATCH,
        "DELETE" => reqwest::Method::DELETE,
        _ => return Err("API-018: HTTP Method پشتیبانی نمی‌شود".into()),
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(p.4 as u64))
        .build()
        .map_err(|e| format!("API-019: ساخت Client انجام نشد: {e}"))?;
    let mut req = client.request(m, url);
    if let Some(h) = headers_json {
        let hv: serde_json::Value = serde_json::from_str(&h)
            .map_err(|_| "API-020: Headers JSON نامعتبر است".to_string())?;
        if let Some(obj) = hv.as_object() {
            for (k, v) in obj {
                if matches!(
                    k.to_lowercase().as_str(),
                    "host" | "authorization" | "cookie"
                ) {
                    return Err(format!("API-021: Header حساس مجاز نیست: {k}"));
                }
                if let Some(val) = v.as_str() {
                    req = req.header(k, val);
                }
            }
        }
    }
    if p.1 != "none" {
        let entry = keyring::Entry::new("novin-pardaz-accounting", &api_secret_key(&profile_id))
            .map_err(|e| format!("API-022: Secret Storage در دسترس نیست: {e}"))?;
        let secret = entry
            .get_password()
            .map_err(|_| "API-023: Secret این اتصال پیدا نشد".to_string())?;
        match p.1.as_str() {
            "api_key" => {
                let h =
                    p.3.ok_or_else(|| "API-024: نام Header برای API Key مشخص نشده".to_string())?;
                req = req.header(h, secret)
            }
            "bearer" => req = req.bearer_auth(secret),
            "basic" => {
                let parts = secret.splitn(2, ':').collect::<Vec<_>>();
                if parts.len() != 2 {
                    return Err("API-025: Secret نوع Basic باید username:password باشد".into());
                }
                req = req.basic_auth(parts[0], Some(parts[1]))
            }
            _ => {}
        }
    }
    if let Some(b) = body {
        req = req.body(b).header("content-type", "application/json");
    }
    let resp = req
        .send()
        .map_err(|e| format!("API-026: درخواست ناموفق بود: {e}"))?;
    let status = resp.status().as_u16();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let text = resp.text().unwrap_or_default();
    let body = text.chars().take(1_000_000).collect();
    Ok(ApiResponse {
        status,
        body,
        content_type: ct,
    })
}

#[tauri::command]
#[tauri::command]
pub fn set_api_profile_enabled(
    state: State<AppState>,
    profile_id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut c = conn(&state)?;
    let user = require_permission(&state, &c, "integrations.manage")?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let n=tx.execute("UPDATE api_profiles SET enabled=?2 WHERE id=?1 AND company_id IN (SELECT company_id FROM company_users WHERE user_id=?3 AND is_active=1)",params![profile_id,enabled as i64,user]).map_err(|e|e.to_string())?;
    if n == 0 {
        return Err("API-027: اتصال API پیدا نشد".into());
    }
    audit(
        &tx,
        &user,
        "api_profile.enable",
        "api_profile",
        &profile_id,
        None,
        Some(&(enabled as i64).to_string()),
    )?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

