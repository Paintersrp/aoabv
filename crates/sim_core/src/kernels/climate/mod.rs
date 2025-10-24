mod albedo;
mod classification;

pub use albedo::albedo_reconcile;
pub use classification::update;

pub const STAGE: &str = "kernel:climate";
pub const ALBEDO_RECONCILE_STAGE: &str = "kernel:climate/albedo_reconcile";
pub const CORE_STAGE: &str = "kernel:climate/core";
