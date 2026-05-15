use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: u32,
    pub deadzone: f32,
    pub trigger_threshold: f32,
    pub min_press_ms: [u32; 2],
    pub tick_jitter_ms: [u32; 2],
    pub log_events: bool,
    pub buttons: BTreeMap<String, ButtonEntry>,
    pub macros: BTreeMap<String, MacroDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ButtonEntry {
    pub label: String,
    #[serde(flatten)]
    pub binding: Binding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum Binding {
    Key(String),
    Macro(String),
    Unbound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroDef {
    #[serde(rename = "loop")]
    pub repeat: bool,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroStep {
    pub key: String,
    pub action: StepAction,
    pub delay_ms: [u32; 2],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StepAction {
    Down,
    Up,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal_config() {
        let json = r#"{
            "version": 1,
            "deadzone": 0.4,
            "trigger_threshold": 0.5,
            "min_press_ms": [8, 25],
            "tick_jitter_ms": [0, 3],
            "log_events": true,
            "buttons": {
                "0": { "label": "Cross (X)", "type": "key", "value": "x" },
                "3": { "label": "Triangle", "type": "unbound" },
                "23": { "label": "L2", "type": "macro", "value": "macro_A" }
            },
            "macros": {
                "macro_A": {
                    "loop": true,
                    "steps": [
                        { "key": "Left", "action": "down", "delay_ms": [50, 80] }
                    ]
                }
            }
        }"#;

        let cfg: Config = serde_json::from_str(json).expect("parse");
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.buttons.len(), 3);
        match &cfg.buttons["0"].binding {
            Binding::Key(k) => assert_eq!(k, "x"),
            _ => panic!("expected key"),
        }
        match &cfg.buttons["3"].binding {
            Binding::Unbound => {}
            _ => panic!("expected unbound"),
        }
    }
}
