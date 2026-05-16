// Tauri 2.x exposes window.__TAURI__ when `withGlobalTauri: true` is set in
// tauri.conf.json (under the "app" key). Wrap it so the rest of the frontend
// can import named helpers and stay testable.
//
// Do NOT import from '@tauri-apps/api' — we are running without a bundler.
// All Tauri API surface comes through the injected global.

const T = window.__TAURI__;
if (!T) {
  throw new Error(
    'Tauri globals missing — ensure withGlobalTauri: true is set in tauri.conf.json'
  );
}

/**
 * Invoke a Tauri IPC command.
 * @param {string} cmd  - Command name (matches #[tauri::command] fn name).
 * @param {object} args - Optional arguments object.
 * @returns {Promise<any>}
 */
export function invoke(cmd, args) {
  return T.core.invoke(cmd, args);
}

/**
 * Listen for a Tauri event emitted from Rust.
 * @param {string}   name - Event name.
 * @param {function} cb   - Called with the payload each time the event fires.
 * @returns {Promise<function>} - Resolves to an unlisten function.
 */
export function listen(name, cb) {
  return T.event.listen(name, e => cb(e.payload));
}
