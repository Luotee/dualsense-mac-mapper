//! Configuration on-disk reader/writer that preserves unknown JSON fields.
//!
//! Typed Config gives us validation; serde_json::Value gives us byte-for-byte
//! round-trip of `_help`, `_keyboard_keys`, and any future "_*" doc fields.
//! We carry both.

use crate::config::{Binding, ButtonEntry, Config, MacroDef, TOUCHPAD_QUADRANT_IDS};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct ConfigDoc {
    raw: Value,
    typed: Config,
}

impl ConfigDoc {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut raw: Value = serde_json::from_str(&bytes)
            .with_context(|| format!("parsing {} as JSON", path.display()))?;
        let mut typed: Config = serde_json::from_value(raw.clone())
            .with_context(|| format!("typed-parsing {}", path.display()))?;
        migrate_touchpad_ids(&mut typed, &mut raw);
        typed.validate()?;
        Ok(Self { raw, typed })
    }

    /// Build a `ConfigDoc` from an already-typed value tree. Used by tests
    /// that seed a known config without touching the filesystem first.
    pub fn load_from_value(mut raw: Value) -> Result<Self> {
        let mut typed: Config =
            serde_json::from_value(raw.clone()).context("typed-parsing in-memory config")?;
        migrate_touchpad_ids(&mut typed, &mut raw);
        typed.validate()?;
        Ok(Self { raw, typed })
    }

    pub fn typed(&self) -> &Config {
        &self.typed
    }

    pub fn typed_mut(&mut self) -> &mut Config {
        &mut self.typed
    }

    pub fn set_deadzone(&mut self, v: f32) {
        self.typed.deadzone = v;
        self.raw["deadzone"] = Value::from(v);
    }

    pub fn set_trigger_threshold(&mut self, v: f32) {
        self.typed.trigger_threshold = v;
        self.raw["trigger_threshold"] = Value::from(v);
    }

    pub fn set_log_events(&mut self, v: bool) {
        self.typed.log_events = v;
        self.raw["log_events"] = Value::from(v);
    }

    /// Set min_press_ms without running validation.
    /// Callers that need the invariant enforced must call `validate()` afterwards.
    pub fn set_min_press_ms_unchecked(&mut self, v: [u32; 2]) {
        self.typed.min_press_ms = v;
        self.raw["min_press_ms"] = serde_json::json!([v[0], v[1]]);
    }

    /// Set tick_jitter_ms without running validation.
    /// Callers that need the invariant enforced must call `validate()` afterwards.
    pub fn set_tick_jitter_ms_unchecked(&mut self, v: [u32; 2]) {
        self.typed.tick_jitter_ms = v;
        self.raw["tick_jitter_ms"] = serde_json::json!([v[0], v[1]]);
    }

    pub fn replace_button(&mut self, id: u32, entry: ButtonEntry) {
        let k = id.to_string();
        self.raw["buttons"][&k] = serde_json::to_value(&entry).unwrap();
        self.typed.buttons.insert(k, entry);
    }

    pub fn replace_macros(&mut self, macros: BTreeMap<String, MacroDef>) {
        self.raw["macros"] = serde_json::to_value(&macros).unwrap();
        self.typed.macros = macros;
    }

    pub fn validate(&self) -> Result<()> {
        self.typed.validate()
    }

    pub fn pretty(&self) -> String {
        serde_json::to_string_pretty(&self.raw).expect("Value is always serialisable")
    }
}

/// Atomically write `doc` to `target`.
///
/// Calls `doc.validate()` first — returns an error without touching the
/// filesystem if validation fails.  Otherwise writes to a sibling `.tmp`
/// file and renames it into place so readers never see a partial write.
/// Migrate v2.0 → v2.1 by inserting missing touchpad quadrant ids
/// (25..=28) as `Unbound` into both the typed and the raw JSON view.
/// The raw patch keeps the next `write_atomic` round-trip honest.
fn migrate_touchpad_ids(typed: &mut Config, raw: &mut Value) {
    typed.fill_touchpad_defaults();
    let labels = ["Touchpad TL", "Touchpad TR", "Touchpad BL", "Touchpad BR"];
    for (id, label) in TOUCHPAD_QUADRANT_IDS.iter().zip(labels.iter()) {
        let key = id.to_string();
        if raw["buttons"][&key].is_null() {
            raw["buttons"][&key] = serde_json::to_value(&ButtonEntry {
                label: (*label).to_string(),
                binding: Binding::Unbound,
            }).unwrap();
        }
    }
    if raw["touchpad_cursor_enabled"].is_null() {
        raw["touchpad_cursor_enabled"] = Value::from(typed.touchpad_cursor_enabled);
    }
    if raw["touchpad_cursor_sensitivity"].is_null() {
        raw["touchpad_cursor_sensitivity"] = Value::from(typed.touchpad_cursor_sensitivity);
    }
}

pub fn write_atomic(target: &Path, doc: &ConfigDoc) -> Result<()> {
    doc.validate().with_context(|| "validating before write")?;
    let mut tmp: PathBuf = target.to_path_buf();
    let fname = format!("{}.tmp", target.file_name().unwrap().to_string_lossy());
    tmp.set_file_name(fname);
    std::fs::write(&tmp, doc.pretty())
        .with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, target)
        .with_context(|| format!("renaming {} → {}", tmp.display(), target.display()))?;
    Ok(())
}
