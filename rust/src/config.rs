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

use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;

pub const VALID_BUTTON_IDS: std::ops::RangeInclusive<u32> = 0..=24;

impl Config {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config: {}", path.display()))?;
        let cfg: Config = serde_json::from_str(&text)
            .with_context(|| format!("parsing config: {}", path.display()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load_from_str(s: &str) -> Result<Self> {
        let cfg: Config = serde_json::from_str(s)
            .context("parsing config from string")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            bail!("unsupported config version {}; expected 1", self.version);
        }
        if self.min_press_ms[0] >= self.min_press_ms[1] {
            bail!("min_press_ms range must have min < max, got {:?}", self.min_press_ms);
        }
        if self.tick_jitter_ms[0] > self.tick_jitter_ms[1] {
            bail!("tick_jitter_ms range must have min <= max, got {:?}", self.tick_jitter_ms);
        }
        for id in VALID_BUTTON_IDS {
            let key = id.to_string();
            if !self.buttons.contains_key(&key) {
                bail!("missing button id {id}; config must list all ids 0..=24");
            }
        }
        for k in self.buttons.keys() {
            let id: u32 = k.parse()
                .map_err(|_| anyhow!("unknown button id {k}; ids must be numeric 0..=24"))?;
            if !VALID_BUTTON_IDS.contains(&id) {
                bail!("unknown button id {id}; allowed range is 0..=24");
            }
        }
        for (id, entry) in &self.buttons {
            if let Binding::Macro(name) = &entry.binding {
                if !self.macros.contains_key(name) {
                    bail!("button {id} references unknown macro \"{name}\"");
                }
            }
        }
        for (id, entry) in &self.buttons {
            if let Binding::Key(k) = &entry.binding {
                parse_key(k).with_context(|| format!("button {id}"))?;
            }
        }
        for (name, m) in &self.macros {
            for (i, step) in m.steps.iter().enumerate() {
                parse_key(&step.key)
                    .with_context(|| format!("macro \"{name}\" step {i}"))?;
            }
        }
        for (name, m) in &self.macros {
            for (i, step) in m.steps.iter().enumerate() {
                if step.delay_ms[0] >= step.delay_ms[1] {
                    bail!(
                        "macro \"{name}\" step {i}: delay_ms range must have min < max, got {:?}",
                        step.delay_ms
                    );
                }
            }
        }
        Ok(())
    }
}

use enigo::Key;

pub fn parse_key(name: &str) -> Result<Key> {
    if name.chars().count() == 1 {
        let c = name.chars().next().unwrap();
        // On Windows, route ASCII letters and digits through Key::Other(VK)
        // so they are injected as real virtual-key events (auto-repeat fires,
        // games that read virtual-key state can see them). Key::Unicode goes
        // through KEYEVENTF_UNICODE which inserts a character but does NOT
        // register as a held key — that's the bug we're fixing here.
        //
        // Linux keeps Key::Unicode as the simple fallback; the real product
        // only runs on Windows, this branch exists so cargo test still works
        // on a Linux dev host.
        #[cfg(target_os = "windows")]
        {
            if c.is_ascii_alphabetic() {
                // VK_A..VK_Z == 0x41..0x5A, same as ASCII upper-case codes
                let vk = c.to_ascii_uppercase() as u32;
                return Ok(Key::Other(vk));
            }
            if c.is_ascii_digit() {
                // VK_0..VK_9 == 0x30..0x39, same as ASCII digit codes
                return Ok(Key::Other(c as u32));
            }
            // Other ASCII printable (`-`, `=`, `,`, `.`, etc.): no clean VK
            // mapping that's layout-independent, so fall through to Unicode.
        }
        return Ok(Key::Unicode(c));
    }
    let lower = name.to_ascii_lowercase();
    Ok(match lower.as_str() {
        "shift"   => Key::Shift,
        "control" | "ctrl" => Key::Control,
        "alt"     => Key::Alt,
        "meta" | "win" | "cmd" => Key::Meta,
        "left"    => Key::LeftArrow,
        "right"   => Key::RightArrow,
        "up"      => Key::UpArrow,
        "down"    => Key::DownArrow,
        "space"   => Key::Space,
        "enter" | "return" => Key::Return,
        "tab"     => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "home"    => Key::Home,
        "end"     => Key::End,
        "pageup"  => Key::PageUp,
        "pagedown" => Key::PageDown,
        f if f.starts_with('f') && f[1..].parse::<u32>().is_ok() => {
            let n: u32 = f[1..].parse().unwrap();
            match n {
                1 => Key::F1, 2 => Key::F2, 3 => Key::F3, 4 => Key::F4,
                5 => Key::F5, 6 => Key::F6, 7 => Key::F7, 8 => Key::F8,
                9 => Key::F9, 10 => Key::F10, 11 => Key::F11, 12 => Key::F12,
                _ => bail!("unknown key name \"{name}\" (F-key out of range)"),
            }
        }
        _ => bail!("unknown key name \"{name}\""),
    })
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

    #[test]
    fn rejects_missing_button_id() {
        let mut cfg = sample_full_config();
        cfg.buttons.remove("7");
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("missing button id 7"), "got: {err}");
    }

    #[test]
    fn rejects_extra_button_id() {
        let mut cfg = sample_full_config();
        cfg.buttons.insert("99".into(), ButtonEntry {
            label: "bogus".into(),
            binding: Binding::Unbound,
        });
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("unknown button id 99"), "got: {err}");
    }

    #[test]
    fn rejects_unknown_macro_reference() {
        let mut cfg = sample_full_config();
        cfg.buttons.insert("23".into(), ButtonEntry {
            label: "L2".into(),
            binding: Binding::Macro("does_not_exist".into()),
        });
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("references unknown macro \"does_not_exist\""), "got: {err}");
    }

    #[test]
    fn rejects_zero_range_delay() {
        let mut cfg = sample_full_config();
        let m = MacroDef {
            repeat: false,
            steps: vec![MacroStep {
                key: "Left".into(),
                action: StepAction::Down,
                delay_ms: [10, 10],
            }],
        };
        cfg.macros.insert("zero_range".into(), m);
        cfg.buttons.insert("23".into(), ButtonEntry {
            label: "L2".into(),
            binding: Binding::Macro("zero_range".into()),
        });
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("delay_ms range must have min < max"), "got: {err}");
    }

    #[test]
    fn rejects_inverted_min_press_ms() {
        let mut cfg = sample_full_config();
        cfg.min_press_ms = [30, 8];
        let err = cfg.validate().unwrap_err().to_string();
        assert!(err.contains("min_press_ms"), "got: {err}");
    }

    #[test]
    fn parses_letter_keys() {
        assert!(matches!(parse_key("x"), Ok(_)));
        assert!(matches!(parse_key("A"), Ok(_)));
    }

    #[test]
    fn parses_named_keys() {
        assert!(matches!(parse_key("Shift"), Ok(_)));
        assert!(matches!(parse_key("Left"), Ok(_)));
        assert!(matches!(parse_key("Up"), Ok(_)));
        assert!(matches!(parse_key("F1"), Ok(_)));
    }

    #[test]
    fn rejects_unknown_key() {
        let err = parse_key("PotatoButton").unwrap_err().to_string();
        assert!(err.contains("unknown key name"), "got: {err}");
    }

    #[test]
    fn validate_flags_unknown_key_in_binding() {
        let mut cfg = sample_full_config();
        cfg.buttons.insert("0".into(), ButtonEntry {
            label: "Cross".into(),
            binding: Binding::Key("PotatoButton".into()),
        });
        let err = cfg.validate().unwrap_err();
        let s = format!("{err:#}");
        assert!(s.contains("button 0"), "got: {s}");
        assert!(s.contains("PotatoButton"), "got: {s}");
    }

    fn sample_full_config() -> Config {
        let mut buttons = BTreeMap::new();
        for id in 0u32..=24 {
            buttons.insert(id.to_string(), ButtonEntry {
                label: format!("button{id}"),
                binding: Binding::Unbound,
            });
        }
        Config {
            version: 1,
            deadzone: 0.4,
            trigger_threshold: 0.5,
            min_press_ms: [8, 25],
            tick_jitter_ms: [0, 3],
            log_events: false,
            buttons,
            macros: BTreeMap::new(),
        }
    }
}
