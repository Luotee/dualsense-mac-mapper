import { invoke, listen } from './ipc.js';
import * as controller from './controller.js';

const tabs = ['mappings', 'macros', 'settings'];

function activate(tab) {
  tabs.forEach(t => {
    document.querySelector(`[data-tab="${t}"]`).classList.toggle('active', t === tab);
    document.getElementById(`pane-${t}`).classList.toggle('active', t === tab);
  });
}
tabs.forEach(t => {
  document.querySelector(`[data-tab="${t}"]`).addEventListener('click', () => activate(t));
});

// Settings cog opens the Settings tab.
document.getElementById('btn-settings').addEventListener('click', () => activate('settings'));

// Connection status — listen for engine bridge emits (Task 15).
const statusEl = document.querySelector('.status');
const statusText = document.getElementById('status-text');
await listen('controller-status', s => {
  if (s.connected) {
    statusEl.classList.add('connected');
    statusText.textContent = `Connected · ${s.name}${s.transport ? ' · ' + s.transport : ''}`;
  } else {
    statusEl.classList.remove('connected');
    statusText.textContent = 'Waiting for controller…';
  }
});

// Load config once; use it for both the IPC sanity check and the controller render.
let cfg = null;
try {
  cfg = await invoke('get_config');
} catch (e) {
  console.error('get_config failed', e);
}

// Task 19 — render the controller diagram with current bindings.
if (cfg) {
  const bindings = buildBindings(cfg);
  controller.render(document.getElementById('controller-host'), bindings);
}

// ─── Config helpers ───────────────────────────────────────────────────────────

/**
 * Build the flat { "0": { kind, value? }, ... } map the controller renderer
 * expects from the full `get_config` response.
 *
 * serde_json serialises Rust enums in "external" form:
 *   Binding::Key("x")    → { "Key": "x" }
 *   Binding::Macro("m")  → { "Macro": "m" }
 *   Binding::Unbound     → "Unbound"
 */
function buildBindings(cfg) {
  const bindings = {};
  const buttons = (cfg && cfg.buttons) ? cfg.buttons : {};
  for (const [id, entry] of Object.entries(buttons)) {
    const b = entry && entry.binding !== undefined ? entry.binding : entry;
    bindings[id] = { kind: kindOf(b), value: valueOf(b) };
  }
  return bindings;
}

function kindOf(b) {
  if (!b || b === 'Unbound') return 'unbound';
  if (typeof b === 'object' && 'Key'   in b) return 'key';
  if (typeof b === 'object' && 'Macro' in b) return 'macro';
  return 'unbound';
}

function valueOf(b) {
  if (typeof b === 'object' && b !== null && 'Key'   in b) return b.Key;
  if (typeof b === 'object' && b !== null && 'Macro' in b) return b.Macro;
  return undefined;
}
