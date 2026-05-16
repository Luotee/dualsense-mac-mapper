//! Tauri IPC commands exposed to the frontend.
//!
//! Every function in this module is registered via `generate_handler![]` in
//! `runtime.rs`. JS invokes them with `window.__TAURI__.core.invoke(name, args)`.
//!
//! Iron rule #9: commands are the gate between JS and Rust. JS may NOT
//! synthesise keys, may NOT decide mappings. Read-only commands (like
//! `get_config`) are trivially safe. Write commands go through `safety` +
//! engine.
//!
//! Pattern: each command has a `*_impl` sibling that is Tauri-free and can be
//! called directly from integration tests (which cannot construct
//! `tauri::State`). The `#[tauri::command]` wrappers are gated on the `gui`
//! feature; the `*_impl` functions are always compiled.

use crate::config::{Binding, ButtonEntry, Config, MacroDef};
use crate::config_io::{write_atomic, ConfigDoc};
use crate::engine::Handle;
use std::path::Path;
#[cfg(feature = "gui")]
use std::path::PathBuf;

// ─── Tauri command wrappers (gui feature only) ───────────────────────────────

#[cfg(feature = "gui")]
use tauri::State;

/// Return the current live config.
///
/// The config is held in a `RwLock` inside the engine's `Handle`. We clone
/// it here so the lock is not held across the async serialisation boundary.
#[cfg_attr(feature = "gui", tauri::command)]
#[cfg(feature = "gui")]
pub fn get_config(engine: State<'_, Handle>) -> Result<Config, String> {
    Ok(engine.config_read().clone())
}

/// Tauri command: replace one button binding, validate, atomically write to
/// disk, and hot-rebind the live engine — all or nothing.
///
/// On validation failure the disk is untouched and the live config unchanged.
#[cfg_attr(feature = "gui", tauri::command)]
#[cfg(feature = "gui")]
pub fn set_binding(
    engine: State<'_, Handle>,
    config_path: State<'_, PathBuf>,
    id: u32,
    entry: ButtonEntry,
) -> Result<(), String> {
    set_binding_impl(&*engine, &config_path, id, entry).map_err(|e| format!("{e:#}"))
}

// ─── Pure *_impl helpers (always compiled, no Tauri dependency) ──────────────

/// Pure implementation of `get_config` callable without `tauri::State`.
///
/// Exposed for integration tests; the Tauri wrapper above delegates here.
#[allow(dead_code)]
pub fn get_config_impl(engine: &Handle) -> Config {
    engine.config_read().clone()
}

/// Pure implementation of `set_binding` callable without `tauri::State`.
///
/// Loads the on-disk config via `ConfigDoc`, patches the button entry,
/// validates the result, and only then atomically writes to disk and
/// hot-rebinds the live engine. Any failure leaves both disk and engine
/// state unchanged.
pub fn set_binding_impl(
    engine: &Handle,
    config_path: &Path,
    id: u32,
    entry: ButtonEntry,
) -> anyhow::Result<()> {
    let mut doc = ConfigDoc::load(config_path)?;
    doc.replace_button(id, entry.clone());
    doc.validate()?;
    write_atomic(config_path, &doc)?;
    engine.config_write().buttons.insert(id.to_string(), entry);
    Ok(())
}

// ─── set_macro ───────────────────────────────────────────────────────────────

/// Tauri command: insert or replace a named macro, validate, atomically write
/// to disk, and hot-rebind the live engine.
#[cfg_attr(feature = "gui", tauri::command)]
#[cfg(feature = "gui")]
pub fn set_macro(
    engine: State<'_, Handle>,
    config_path: State<'_, PathBuf>,
    name: String,
    def: MacroDef,
) -> Result<(), String> {
    set_macro_impl(&*engine, &config_path, name, def).map_err(|e| format!("{e:#}"))
}

/// Pure implementation of `set_macro` callable without `tauri::State`.
pub fn set_macro_impl(
    engine: &Handle,
    config_path: &Path,
    name: String,
    def: MacroDef,
) -> anyhow::Result<()> {
    let mut doc = ConfigDoc::load(config_path)?;
    let mut macros = doc.typed().macros.clone();
    macros.insert(name, def);
    doc.replace_macros(macros);
    doc.validate()?;
    write_atomic(config_path, &doc)?;
    engine.config_write().macros = doc.typed().macros.clone();
    Ok(())
}

// ─── delete_macro ────────────────────────────────────────────────────────────

/// Tauri command: delete a named macro, checking it is not referenced by any
/// button binding before removal.
#[cfg_attr(feature = "gui", tauri::command)]
#[cfg(feature = "gui")]
pub fn delete_macro(
    engine: State<'_, Handle>,
    config_path: State<'_, PathBuf>,
    name: String,
) -> Result<(), String> {
    delete_macro_impl(&*engine, &config_path, &name).map_err(|e| format!("{e:#}"))
}

/// Pure implementation of `delete_macro` callable without `tauri::State`.
///
/// Returns an error if any button binding references the macro (referential
/// integrity check). The error message names both the macro and the button id.
pub fn delete_macro_impl(
    engine: &Handle,
    config_path: &Path,
    name: &str,
) -> anyhow::Result<()> {
    let mut doc = ConfigDoc::load(config_path)?;
    // Referential check: refuse if any button binding points at this macro.
    let bound_to: Vec<String> = doc
        .typed()
        .buttons
        .iter()
        .filter_map(|(id, entry)| match &entry.binding {
            Binding::Macro(n) if n == name => Some(id.clone()),
            _ => None,
        })
        .collect();
    if !bound_to.is_empty() {
        anyhow::bail!(
            "cannot delete macro '{name}' — bound by button {}",
            bound_to.join(", ")
        );
    }
    let mut macros = doc.typed().macros.clone();
    macros.remove(name);
    doc.replace_macros(macros);
    doc.validate()?;
    write_atomic(config_path, &doc)?;
    engine.config_write().macros = doc.typed().macros.clone();
    Ok(())
}

// ─── rename_macro ────────────────────────────────────────────────────────────

/// Tauri command: rename a macro and update all button bindings that reference
/// the old name.
#[cfg_attr(feature = "gui", tauri::command)]
#[cfg(feature = "gui")]
pub fn rename_macro(
    engine: State<'_, Handle>,
    config_path: State<'_, PathBuf>,
    old: String,
    new: String,
) -> Result<(), String> {
    rename_macro_impl(&*engine, &config_path, &old, &new).map_err(|e| format!("{e:#}"))
}

/// Pure implementation of `rename_macro` callable without `tauri::State`.
///
/// Renames the macro in the macros map and rewrites every button binding that
/// referenced the old name so referential integrity is preserved.
pub fn rename_macro_impl(
    engine: &Handle,
    config_path: &Path,
    old: &str,
    new: &str,
) -> anyhow::Result<()> {
    if old == new {
        return Ok(());
    }
    let mut doc = ConfigDoc::load(config_path)?;
    if !doc.typed().macros.contains_key(old) {
        anyhow::bail!("macro '{old}' does not exist");
    }
    if doc.typed().macros.contains_key(new) {
        anyhow::bail!("macro '{new}' already exists");
    }
    // Rename in macros map.
    let mut macros = doc.typed().macros.clone();
    let def = macros.remove(old).expect("just checked");
    macros.insert(new.to_string(), def);
    doc.replace_macros(macros);
    // Update every button binding that points at the old name.
    let buttons_snapshot = doc.typed().buttons.clone();
    for (id_str, entry) in &buttons_snapshot {
        if let Binding::Macro(ref n) = entry.binding {
            if n == old {
                let id: u32 = id_str.parse().expect("button keys are u32 strings");
                let updated = ButtonEntry {
                    label: entry.label.clone(),
                    binding: Binding::Macro(new.to_string()),
                };
                doc.replace_button(id, updated);
            }
        }
    }
    doc.validate()?;
    write_atomic(config_path, &doc)?;
    *engine.config_write() = doc.typed().clone();
    Ok(())
}
