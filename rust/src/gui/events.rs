//! Engine event → Tauri emit bridge. Runs on its own thread for the lifetime
//! of the app. Drains the engine's event channel every ~8 ms (matches the
//! engine loop tick) and re-emits as Tauri events for the frontend.

use crate::engine::{EngineEvent, Handle};
use tauri::{AppHandle, Emitter, Runtime};

pub fn spawn<R: Runtime>(app: AppHandle<R>, engine: Handle) {
    std::thread::spawn(move || {
        loop {
            for ev in engine.drain_events() {
                let _ = match ev {
                    EngineEvent::ControllerConnected { name, transport } =>
                        app.emit("controller-status", serde_json::json!({
                            "connected": true, "name": name, "transport": transport
                        })),
                    EngineEvent::ControllerDisconnected =>
                        app.emit("controller-status", serde_json::json!({
                            "connected": false, "name": "", "transport": ""
                        })),
                    EngineEvent::ButtonDown { id } =>
                        app.emit("button-down", serde_json::json!({ "id": id })),
                    EngineEvent::ButtonUp { id } =>
                        app.emit("button-up", serde_json::json!({ "id": id })),
                    EngineEvent::KeyEmit { ts_ms, key, action } =>
                        app.emit("activity", serde_json::json!({
                            "ts_ms": ts_ms, "kind": "emit", "key": key, "action": action
                        })),
                    EngineEvent::MacroStart { ts_ms, name } =>
                        app.emit("activity", serde_json::json!({
                            "ts_ms": ts_ms, "kind": "macro-start", "name": name
                        })),
                    EngineEvent::MacroEnd { ts_ms, name, completed } =>
                        app.emit("activity", serde_json::json!({
                            "ts_ms": ts_ms, "kind": "macro-end", "name": name, "completed": completed
                        })),
                };
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    });
}
