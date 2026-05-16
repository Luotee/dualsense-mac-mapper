//! Tauri IPC commands exposed to the frontend.
//!
//! Every function in this module is registered via `generate_handler![]` in
//! `runtime.rs`. JS invokes them with `window.__TAURI__.core.invoke(name, args)`.
//!
//! Iron rule #9: commands are the gate between JS and Rust. JS may NOT
//! synthesise keys, may NOT decide mappings. Read-only commands (like
//! `get_config`) are trivially safe. Write commands go through `safety` +
//! engine.

use crate::config::Config;
use crate::engine::Handle;
use tauri::State;

/// Return the current live config.
///
/// The config is held in a `RwLock` inside the engine's `Handle`. We clone
/// it here so the lock is not held across the async serialisation boundary.
#[tauri::command]
pub fn get_config(engine: State<'_, Handle>) -> Result<Config, String> {
    Ok(engine.config_read().clone())
}
