//! محاسبات انبار: ارزش‌گذاری موجودی و موجودی قابل فروش.
//!
//! سه روش پشتیبانی‌شده مطابق نیاز بازار ایران:
//! - FIFO (اولین صادره از اولین وارده)
//! - میانگین متحرک (Moving Average)
//! - میانگین موزون (Weighted Average)

use serde::{Deserialize, Serialize};

/// خطاهای دامنه‌ی انبار.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InventoryError {
    #[error("INV-001: روش ارزش‌گذاری نامعتبر است")]
    InvalidMethod,
    #[error("INV-002: تعداد نامعتبر است")]
    InvalidQuantity,
    #[error("INV-003: موجودی قابل برداشت کافی نیست")]
    InsufficientStock,
}

/// روش ارزش‌گذاری موجودی.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationMethod {
    Fifo,
    MovingAverage,
    WeightedAverage,
}

impl ValuationMethod {
    pub fn parse(value: &str) -> Result<Self, InventoryError> {
        match value {
            "fifo" => Ok(ValuationMethod::Fifo),
            "moving_average" => Ok(ValuationMethod::MovingAverage),
            "weighted_average" => Ok(ValuationMethod::WeightedAverage),
            _ => Err(InventoryError::InvalidMethod),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ValuationMethod::Fifo => "fifo",
            ValuationMethod::MovingAverage => "moving_average",
            ValuationMethod::WeightedAverage => "weighted_average",
        }
    }
}

/// نوع گردش انبار.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementKind {
    Receipt,
    Issue,
    TransferIn,
    TransferOut,
    Adjustment,
}

impl MovementKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "receipt" => Some(MovementKind::Receipt),
            "issue" => Some(MovementKind::Issue),
            "transfer_in" => Some(MovementKind::TransferIn),
            "transfer_out" => Some(MovementKind::TransferOut),
            "adjustment" => Some(MovementKind::Adjustment),
            _ => None,
        }
    }

    pub fn is_inbound(&self) -> bool {
        matches!(self, MovementKind::Receipt | MovementKind::TransferIn)
    }

    pub fn is_outbound(&self) -> bool {
        matches!(self, MovementKind::Issue | MovementKind::TransferOut)
    }
}

/// یک گردش انبار (به ترتیب زمانی).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Movement {
    pub kind: MovementKind,
    pub quantity: f64,
    /// بهای واحد بر حسب ریال.
    pub unit_cost: i64,
}

impl Movement {
    pub fn new(kind: MovementKind, quantity: f64, unit_cost: i64) -> Self {
        Movement {
            kind,
            quantity,
            unit_cost,
        }
    }
}

/// نتیجه‌ی ارزش‌گذاری.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Valuation {
    /// موجودی پایانی.
    pub quantity: f64,
    /// بهای واحد موجودی پایانی (ریال).
    pub unit_cost: i64,
    /// ارزش کل موجودی (ریال).
    pub total_value: i64,
}

/// لایه‌ی FIFO باقی‌مانده.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer {
    pub quantity: f64,
    pub unit_cost: i64,
}

/// محاسبه‌ی ارزش موجودی از روی گردش‌ها.
///
/// گردش‌ها باید به ترتیب زمانی مرتب باشند.
pub fn valuate(
    movements: &[Movement],
    method: ValuationMethod,
) -> Result<Valuation, InventoryError> {
    for movement in movements {
        if !movement.quantity.is_finite() || movement.quantity < 0.0 {
            return Err(InventoryError::InvalidQuantity);
        }
    }
    match method {
        ValuationMethod::Fifo => Ok(valuate_fifo(movements)),
        ValuationMethod::MovingAverage | ValuationMethod::WeightedAverage => {
            Ok(valuate_moving_average(movements))
        }
    }
}

/// لایه‌های باقی‌مانده‌ی FIFO — برای گزارش ارزش موجودی و ردیابی بهای تمام‌شده.
pub fn fifo_layers(movements: &[Movement]) -> Vec<Layer> {
    let mut layers: Vec<Layer> = Vec::new();
    for movement in movements {
        if movement.kind.is_inbound() {
            if movement.quantity > 0.0 {
                layers.push(Layer {
                    quantity: movement.quantity,
                    unit_cost: movement.unit_cost,
                });
            }
        } else if movement.kind.is_outbound() {
            let mut remaining = movement.quantity;
            while remaining > 0.0 && !layers.is_empty() {
                if layers[0].quantity <= remaining {
                    remaining -= layers[0].quantity;
                    layers.remove(0);
                } else {
                    layers[0].quantity -= remaining;
                    remaining = 0.0;
                }
            }
        }
    }
    layers
}

/// بهای تمام‌شده‌ی کالای فروش‌رفته (COGS) برای یک خروج، به روش FIFO.
///
/// لایه‌های ورودی مصرف می‌شوند و بهای دقیق هر لایه لحاظ می‌گردد.
pub fn consume_fifo(layers: &mut Vec<Layer>, quantity: f64) -> Result<i64, InventoryError> {
    if !quantity.is_finite() || quantity <= 0.0 {
        return Err(InventoryError::InvalidQuantity);
    }
    let available: f64 = layers.iter().map(|l| l.quantity).sum();
    if available + f64::EPSILON < quantity {
        return Err(InventoryError::InsufficientStock);
    }
    let mut remaining = quantity;
    let mut cost = 0.0f64;
    while remaining > 0.0 && !layers.is_empty() {
        let layer = layers[0];
        if layer.quantity <= remaining {
            cost += layer.quantity * layer.unit_cost as f64;
            remaining -= layer.quantity;
            layers.remove(0);
        } else {
            cost += remaining * layer.unit_cost as f64;
            layers[0].quantity -= remaining;
            remaining = 0.0;
        }
    }
    Ok(cost.round() as i64)
}

fn valuate_fifo(movements: &[Movement]) -> Valuation {
    let layers = fifo_layers(movements);
    let quantity: f64 = layers.iter().map(|l| l.quantity).sum();
    if quantity <= 0.0 {
        return Valuation {
            quantity: 0.0,
            unit_cost: 0,
            total_value: 0,
        };
    }
    let value: f64 = layers.iter().map(|l| l.quantity * l.unit_cost as f64).sum();
    Valuation {
        quantity,
        unit_cost: (value / quantity).round() as i64,
        total_value: value.round() as i64,
    }
}

fn valuate_moving_average(movements: &[Movement]) -> Valuation {
    let mut quantity = 0.0f64;
    let mut average = 0.0f64;
    for movement in movements {
        if movement.kind.is_inbound() {
            let new_quantity = quantity + movement.quantity;
            if movement.quantity > 0.0 && new_quantity > 0.0 {
                average = ((average * quantity) + (movement.unit_cost as f64 * movement.quantity))
                    / new_quantity;
            }
            quantity = new_quantity;
        } else if movement.kind.is_outbound() {
            quantity = (quantity - movement.quantity).max(0.0);
        }
    }
    let unit_cost = average.round() as i64;
    Valuation {
        quantity,
        unit_cost,
        total_value: (quantity * average).round() as i64,
    }
}

/// موجودی قابل فروش = موجودی فیزیکی − رزروشده.
pub fn available_quantity(on_hand: f64, reserved: f64) -> f64 {
    (on_hand - reserved).max(0.0)
}

/// آیا برداشت درخواستی از موجودی قابل فروش امکان‌پذیر است؟
pub fn can_issue(on_hand: f64, reserved: f64, requested: f64) -> Result<(), InventoryError> {
    if !requested.is_finite() || requested <= 0.0 {
        return Err(InventoryError::InvalidQuantity);
    }
    if available_quantity(on_hand, reserved) + f64::EPSILON < requested {
        return Err(InventoryError::InsufficientStock);
    }
    Ok(())
}
