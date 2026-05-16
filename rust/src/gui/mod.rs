//! GUI layer. Tauri builder, IPC commands, system tray, file watcher.
//!
//! Phase 3 of the v0.2.0 rewrite adds the window. This module hosts
//! everything Tauri-related so the engine/mapper/safety modules stay
//! a pure Rust library that the CLI binary (`--cli`) can still use.
//!
//! All Tauri code is gated behind the `gui` cargo feature. To build
//! a binary with the GUI, pass `--features gui` to cargo.

pub mod file_watcher;

#[cfg(feature = "gui")]
pub mod runtime;

#[cfg(feature = "gui")]
pub use runtime::{run, RunOptions};
