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
    /// Raw px/frame threshold below which the accel curve uses
    /// `accel_gain_slow` for precision. Default 5.
    accel_slow_threshold: Arc<AtomicU32>,
    /// Raw px/frame threshold above which the accel curve uses
    /// `accel_gain_fast` for flick acceleration. Default 20.
    accel_fast_threshold: Arc<AtomicU32>,
    /// Acceleration gain at and below `accel_slow_threshold`,
    /// stored ×100 so the AtomicU32 holds 2-decimal precision.
    /// Default 50 (= 0.50).
    accel_gain_slow_x100: Arc<AtomicU32>,
    /// Acceleration gain at and above `accel_fast_threshold`,
    /// stored ×100. Default 150 (= 1.50).
    accel_gain_fast_x100: Arc<AtomicU32>,
    /// Raw px deadzone radius. Per-frame |Δ| below this is silently
    /// dropped for 3 consecutive frames. Default 2.
    deadzone_radius: Arc<AtomicU32>,
    /// When true, suppress all cursor deltas while the touchpad click
    /// button is held down. Default true. See Issue 8 click-drift.
    click_freeze_enabled: Arc<AtomicBool>,
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
            accel_slow_threshold: Arc::new(AtomicU32::new(5)),
            accel_fast_threshold: Arc::new(AtomicU32::new(20)),
            accel_gain_slow_x100: Arc::new(AtomicU32::new(50)),    // 0.50
            accel_gain_fast_x100: Arc::new(AtomicU32::new(150)),   // 1.50
            deadzone_radius: Arc::new(AtomicU32::new(2)),
            click_freeze_enabled: Arc::new(AtomicBool::new(true)),
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

    pub fn accel_slow_threshold(&self) -> u32 {
        self.accel_slow_threshold.load(Ordering::Relaxed)
    }
    pub fn set_accel_slow_threshold(&self, v: u32) {
        self.accel_slow_threshold.store(v, Ordering::Relaxed);
    }
    pub fn accel_fast_threshold(&self) -> u32 {
        self.accel_fast_threshold.load(Ordering::Relaxed)
    }
    pub fn set_accel_fast_threshold(&self, v: u32) {
        self.accel_fast_threshold.store(v, Ordering::Relaxed);
    }
    pub fn accel_gain_slow(&self) -> f32 {
        self.accel_gain_slow_x100.load(Ordering::Relaxed) as f32 / 100.0
    }
    pub fn set_accel_gain_slow(&self, v: f32) {
        self.accel_gain_slow_x100.store((v * 100.0) as u32, Ordering::Relaxed);
    }
    pub fn accel_gain_fast(&self) -> f32 {
        self.accel_gain_fast_x100.load(Ordering::Relaxed) as f32 / 100.0
    }
    pub fn set_accel_gain_fast(&self, v: f32) {
        self.accel_gain_fast_x100.store((v * 100.0) as u32, Ordering::Relaxed);
    }
    pub fn deadzone_radius(&self) -> u32 {
        self.deadzone_radius.load(Ordering::Relaxed)
    }
    pub fn set_deadzone_radius(&self, v: u32) {
        self.deadzone_radius.store(v, Ordering::Relaxed);
    }
    pub fn click_freeze_enabled(&self) -> bool {
        self.click_freeze_enabled.load(Ordering::Relaxed)
    }
    pub fn set_click_freeze_enabled(&self, v: bool) {
        self.click_freeze_enabled.store(v, Ordering::Relaxed);
    }
}

impl Default for CursorParams {
    fn default() -> Self {
        Self::with_midpoints(1.5, true, 960, 540)
    }
}
