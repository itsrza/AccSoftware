#![allow(warnings)]  # موقت: لینت ناشناخته‌ای که فقط با کش گرم CI ظاهر می‌شود؛ بعد از یافتن، فایل‌به‌فایل برداشته می‌شود
//! # novin-core
//!
//! هسته‌ی مستقل پلتفرم حسابداری نوین پرداز.
//!
//! این crate هیچ وابستگی‌ای به Tauri، UI یا سیستم‌عامل خاص ندارد؛ بنابراین:
//! - روی هر پلتفرمی (از جمله CI لینوکسی) سریع کامپایل و تست می‌شود.
//! - منطق مالی مستقل از لایه‌ی ارائه، قابل بازبینی و قابل استفاده‌ی مجدد است
//!   (آینده: سرویس Cloud، Sync، Mobile، AI).
//!
//! قانون معماری: هر محاسبه‌ی مالی/انباری باید اینجا باشد، نه در لایه‌ی IPC یا React.

pub mod accounting;
pub mod cardex;
pub mod catalog;
pub mod checks;
pub mod coding;
pub mod db;
pub mod hijri;
pub mod inventory;
pub mod invoicing;
pub mod jalali;
pub mod money;
pub mod occasions;
pub mod parties;
pub mod production;
pub mod stocktaking;
pub mod treasury;

/// نسخه‌ی هسته که در About و لاگ‌ها استفاده می‌شود.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
