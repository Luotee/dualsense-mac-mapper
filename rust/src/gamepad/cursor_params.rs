//! Live-tunable touchpad cursor parameters, shared between the engine
//! (which mutates them from `set_settings`) and the HID worker thread
//! (which reads them on every decoded frame).
//!
//! Wrapped in atomics so the worker thread can read without locking,
//! and stored as an `Arc` so the engine can hand out clones to any
//! consumer that needs the live view (HidSource for now, future
//! debug overlays, etc.).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone)]
pub struct CursorParams {
    sensitivity_bits: Arc<AtomicU32>,
    enabled: Arc<AtomicBool>,
}

impl CursorParams {
    pub fn new(sensitivity: f32, enabled: bool) -> Self {
        Self {
            sensitivity_bits: Arc::new(AtomicU32::new(sensitivity.to_bits())),
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }

    pub fn sensitivity(&self) -> f32 {
        f32::from_bits(self.sensitivity_bits.load(Ordering::Relaxed))
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_sensitivity(&self, v: f32) {
        self.sensitivity_bits.store(v.to_bits(), Ordering::Relaxed);
    }

    pub fn set_enabled(&self, v: bool) {
        self.enabled.store(v, Ordering::Relaxed);
    }
}

impl Default for CursorParams {
    fn default() -> Self {
        Self::new(1.5, true)
    }
}
