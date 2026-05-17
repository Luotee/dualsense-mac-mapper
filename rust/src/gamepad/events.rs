//! Event surface produced by the gamepad source. Kept identical to the
//! v1.2.0 enum so engine.rs / mapper.rs / safety.rs / keyboard.rs
//! compile unchanged across the gilrs → hidapi migration.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadEvent {
    Connected,
    Disconnected,
    ButtonDown(u32),
    ButtonUp(u32),
    /// Sticks: lx=0, ly=1, rx=2, ry=3 — value in [-1.0, 1.0]
    Stick { axis: u32, value: f32 },
    /// Triggers: l2=4, r2=5 — value normalised to [0.0, 1.0]
    Trigger { axis: u32, value: f32 },
}
