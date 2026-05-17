//! Live-tunable touchpad cursor parameters, shared between the engine
//! (which mutates them from `set_settings`) and the HID worker thread
//! (which reads them on every decoded frame).
//!
//! Wrapped in atomics so the worker thread can read without locking,
//! and stored as an `Arc` so the engine can hand out clones to any
//! consumer that needs the live view (HidSource for now, future
//! debug overlays, etc.).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

#[derive(Clone)]
pub struct CursorParams {
    sensitivity_bits: Arc<AtomicU32>,
    enabled: Arc<AtomicBool>,
    /// X coordinate that splits TL/BL vs TR/BR. Tunable so a pad with a
    /// non-standard touchpad range can be calibrated without touching
    /// code. Defaults to 960 (half of the documented 1920-wide pad).
    midpoint_x: Arc<AtomicU16>,
    /// Y coordinate that splits TL/TR vs BL/BR. Defaults to 540.
    midpoint_y: Arc<AtomicU16>,
}

impl CursorParams {
    pub fn new(sensitivity: f32, enabled: bool) -> Self {
        Self::with_midpoints(sensitivity, enabled, 960, 540)
    }

    pub fn with_midpoints(sensitivity: f32, enabled: bool, mid_x: u16, mid_y: u16) -> Self {
        Self {
            sensitivity_bits: Arc::new(AtomicU32::new(sensitivity.to_bits())),
            enabled: Arc::new(AtomicBool::new(enabled)),
            midpoint_x: Arc::new(AtomicU16::new(mid_x)),
            midpoint_y: Arc::new(AtomicU16::new(mid_y)),
        }
    }

    pub fn sensitivity(&self) -> f32 {
        f32::from_bits(self.sensitivity_bits.load(Ordering::Relaxed))
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn midpoint_x(&self) -> u16 { self.midpoint_x.load(Ordering::Relaxed) }
    pub fn midpoint_y(&self) -> u16 { self.midpoint_y.load(Ordering::Relaxed) }

    pub fn set_sensitivity(&self, v: f32) {
        self.sensitivity_bits.store(v.to_bits(), Ordering::Relaxed);
    }

    pub fn set_enabled(&self, v: bool) {
        self.enabled.store(v, Ordering::Relaxed);
    }

    pub fn set_midpoint_x(&self, v: u16) { self.midpoint_x.store(v, Ordering::Relaxed); }
    pub fn set_midpoint_y(&self, v: u16) { self.midpoint_y.store(v, Ordering::Relaxed); }
}

impl Default for CursorParams {
    fn default() -> Self {
        Self::with_midpoints(1.5, true, 960, 540)
    }
}
