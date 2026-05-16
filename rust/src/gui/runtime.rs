//! Tauri runtime entry: window builder + lifecycle. v0.2.0 default mode.
//!
//! `run` is the top-level entry point for the GUI binary path.
//! It starts the engine, builds the Tauri window, then blocks on the
//! event loop. When the event loop exits (Quit from tray or process kill),
//! `engine.shutdown()` fires, releasing all held keys (Iron rule #3).

use crate::config::Config;
use crate::engine::Engine;
use crate::safety;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tauri::{Manager, WebviewWindowBuilder};

/// Options forwarded from `main.rs` into the GUI runtime.
pub struct RunOptions {
    pub config_path: PathBuf,
    pub dry_run: bool,
}

/// Start the engine and enter the Tauri event loop.
///
/// This function is **blocking** and only returns when the user quits the
/// application (via tray "Quit" action, once the tray is wired up in Task 8).
/// On return — whether clean or via error — `engine.shutdown()` is called so
/// that `KeyboardSink::drop` and `release_all_held` run (Iron rule #3).
pub fn run(cfg: Config, opts: RunOptions) -> Result<()> {
    let engine = Engine::spawn(cfg, opts.dry_run)?;
    // Bind the engine's key state to the global so the panic hook (installed
    // in main::real_main) can drain it on panic. Iron Rule #3, GUI path.
    safety::register_global(engine.handle().key_state());
    let handle = engine.handle();

    // Clone handle before the `move` setup closure so `.manage(handle_for_state)`
    // can still be called on the builder after the closure has captured its copy.
    let handle_for_setup = handle.clone();
    let handle_for_state = handle.clone();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second instance was launched — focus the existing window instead.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(move |app| {
            let _w = WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("DualSense Mapper")
            .inner_size(980.0, 640.0)
            .min_inner_size(880.0, 560.0)
            .visible(true)
            .build()?;

            let tray = crate::gui::tray::build(&app.handle(), handle_for_setup.clone())?;
            // Store the tray so it persists for the app lifetime and can be
            // retrieved in Task 25 via `app.state::<TrayIcon<_>>()`.
            app.manage(tray);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::gui::commands::get_config,
            crate::gui::commands::set_binding,
        ])
        .manage(handle_for_state)
        .manage(opts.config_path.clone())
        .on_window_event(|window, event| {
            // Spec §10: clicking ✕ hides the window; mapper keeps running.
            // Tray "Quit" (Task 8) is the only way to exit the process.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!());

    // Iron rule #3: shut down the engine regardless of how the event loop
    // terminated, so held keys are released before the process dies.
    engine.shutdown();

    result.context("Tauri event loop terminated unexpectedly")?;
    Ok(())
}
