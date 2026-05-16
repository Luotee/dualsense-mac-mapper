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

use crate::config::{ButtonEntry, Config};
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
