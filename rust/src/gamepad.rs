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
    // gilrs reports triggers in [-1.0, 1.0] on Windows DualSense; map to [0.0, 1.0]
    ((raw + 1.0) * 0.5).clamp(0.0, 1.0)
}

pub fn button_index(b: Button) -> Option<u32> {
    Some(match b {
        Button::South        => 0,   // Cross
        Button::East         => 1,   // Circle
        Button::West         => 2,   // Square
        Button::North        => 3,   // Triangle
        Button::Select       => 4,   // Share
        Button::Mode         => 5,   // PS
        Button::Start        => 6,   // Options
        Button::LeftThumb    => 7,   // L3
        Button::RightThumb   => 8,   // R3
        Button::LeftTrigger  => 9,   // L1
        Button::RightTrigger => 10,  // R1
        Button::DPadUp       => 11,
        Button::DPadDown     => 12,
        Button::DPadLeft     => 13,
        Button::DPadRight    => 14,
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
    fn trigger_idle_zero_stays_in_range() {
        // some platforms report 0.0 for an unpressed trigger
        let v = normalize_trigger(0.0);
        assert!(v >= 0.0 && v <= 0.5, "got {v}");
    }

    #[test]
    fn trigger_full_press_becomes_one() {
        assert!((normalize_trigger(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn button_index_round_trip() {
        assert_eq!(button_index(Button::South),       Some(0));
        assert_eq!(button_index(Button::RightTrigger), Some(10));
        assert_eq!(button_index(Button::DPadRight),    Some(14));
        assert_eq!(button_index(Button::Unknown),      None);
    }
}
