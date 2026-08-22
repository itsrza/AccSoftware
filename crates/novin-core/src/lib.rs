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
pub mod catalog;
pub mod checks;
pub mod coding;
pub mod db;
pub mod inventory;
pub mod jalali;
pub mod money;

/// نسخه‌ی هسته که در About و لاگ‌ها استفاده می‌شود.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
