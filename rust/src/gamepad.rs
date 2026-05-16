use anyhow::Result;
use gilrs::{Axis, Button, Event, EventType, Gilrs};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GamepadEvent {
    Connected,
    Disconnected,
    ButtonDown(u32),
    ButtonUp(u32),
    /// Sticks: lx=0, ly=1, rx=2, ry=3 — value in [-1.0, 1.0]
    Stick { axis: u32, value: f32 },
    /// Triggers: l2=4, r2=5 — value normalized to [0.0, 1.0]
    Trigger { axis: u32, value: f32 },
}

pub fn normalize_trigger(raw: f32) -> f32 {
    // Platform convention differs:
    //   Linux gilrs: triggers report in [-1.0, 1.0] (idle = -1.0)
    //   Windows / XInput backend: triggers can report in [0.0, 1.0] (idle = 0.0)
    // Detect by sign: negative raw means [-1, 1] convention; otherwise pass through.
    if raw < 0.0 {
        ((raw + 1.0) * 0.5).clamp(0.0, 1.0)
    } else {
        raw.clamp(0.0, 1.0)
    }
}

pub fn button_index(b: Button) -> Option<u32> {
    Some(match b {
        Button::South         => 0,   // Cross
        Button::East          => 1,   // Circle
        Button::West          => 2,   // Square
        Button::North         => 3,   // Triangle
        Button::Select        => 4,   // Share
        Button::Mode          => 5,   // PS
        Button::Start         => 6,   // Options
        Button::LeftThumb     => 7,   // L3
        Button::RightThumb    => 8,   // R3
        Button::LeftTrigger   => 9,   // L1 (shoulder)
        Button::RightTrigger  => 10,  // R1 (shoulder)
        Button::DPadUp        => 11,
        Button::DPadDown      => 12,
        Button::DPadLeft      => 13,
        Button::DPadRight     => 14,
        // Digital trigger events — some platforms report L2/R2 as both a
        // digital button (these) AND an analog axis (LeftZ/RightZ). Map them
        // to the same virtual ids 23/24 so the macro fires regardless of which
        // path gilrs delivers on the current OS/driver.
        Button::LeftTrigger2  => 23,
        Button::RightTrigger2 => 24,
        _ => return None,
    })
}

pub fn stick_axis_index(a: Axis) -> Option<u32> {
    Some(match a {
        Axis::LeftStickX  => 0,
        Axis::LeftStickY  => 1,
        Axis::RightStickX => 2,
        Axis::RightStickY => 3,
        _ => return None,
    })
}

pub fn trigger_axis_index(a: Axis) -> Option<u32> {
    Some(match a {
        Axis::LeftZ  => 4,  // L2
        Axis::RightZ => 5,  // R2
        _ => return None,
    })
}

pub struct GamepadSource {
    gilrs: Gilrs,
}

impl GamepadSource {
    pub fn new() -> Result<Self> {
        Ok(Self { gilrs: Gilrs::new().map_err(|e| anyhow::anyhow!("{e}"))? })
    }

    /// Drain pending events and translate; non-blocking.
    pub fn poll(&mut self, out: &mut Vec<GamepadEvent>) {
        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::Connected => out.push(GamepadEvent::Connected),
                EventType::Disconnected => out.push(GamepadEvent::Disconnected),
                EventType::ButtonPressed(b, _) => {
                    if let Some(i) = button_index(b) { out.push(GamepadEvent::ButtonDown(i)); }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(i) = button_index(b) { out.push(GamepadEvent::ButtonUp(i)); }
                }
                EventType::AxisChanged(axis, value, _) => {
                    if let Some(i) = stick_axis_index(axis) {
                        out.push(GamepadEvent::Stick { axis: i, value });
                    } else if let Some(i) = trigger_axis_index(axis) {
                        out.push(GamepadEvent::Trigger { axis: i, value: normalize_trigger(value) });
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_idle_minus_one_becomes_zero() {
        assert!((normalize_trigger(-1.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn trigger_idle_zero_stays_zero() {
        // Windows / XInput convention: unpressed trigger reads 0.0 → must stay 0.0
        // so it sits below threshold 0.5 (was a bug — previously returned 0.5).
        assert!((normalize_trigger(0.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn trigger_full_press_becomes_one() {
        assert!((normalize_trigger(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn trigger_zero_one_convention_pass_through() {
        // [0, 1] convention: 0.7 stays 0.7, not (0.7+1)/2 = 0.85
        assert!((normalize_trigger(0.7) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn button_index_round_trip() {
        assert_eq!(button_index(Button::South),         Some(0));
        assert_eq!(button_index(Button::RightTrigger),  Some(10));
        assert_eq!(button_index(Button::DPadRight),     Some(14));
        assert_eq!(button_index(Button::LeftTrigger2),  Some(23));
        assert_eq!(button_index(Button::RightTrigger2), Some(24));
        assert_eq!(button_index(Button::Unknown),       None);
    }
}
