//! GUI layer. Tauri builder, IPC commands, system tray, file watcher.
//!
//! Phase 3 of the v0.2.0 rewrite adds the window. This module hosts
//! everything Tauri-related so the engine/mapper/safety modules stay
//! a pure Rust library that the CLI binary (`--cli`) can still use.

pub mod file_watcher;
