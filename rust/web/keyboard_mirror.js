// Mirror keyboard-press visual feedback onto the in-app controller SVG.
//
// When the GUI window has focus and the user presses a key that is bound
// to a button (entry.type === 'key' && entry.value === resolved-name),
// flash the matching button(s). Releasing the key clears the flash.
//
// Suppression rules — all of these skip the flash:
//   - ev.repeat (key auto-repeat) — would re-trigger the flash 30×/s
//   - capture_state.isCaptureActive() — a bind popup or step editor is
//     listening for the same key; flashing would imply the binding is
//     already applied before the user clicked Save
//   - ev.target is INPUT / SELECT / TEXTAREA / contenteditable — user is
//     typing into a form (Settings number fields, macro inline forms)
//
// keyup is intentionally NOT gated on the same suppression rules: if the
// user pressed `x` outside a form and then moved focus into one before
// releasing, we still need to clear the flash we set.

import { invoke, listen } from './ipc.js';
import { resolveKeyName } from './key_capture.js';
import { isCaptureActive } from './capture_state.js';
import * as controller from './controller.js';

let bindingsByKey = new Map();   // key-name (string) → array of numeric button ids
const activeKeys  = new Set();    // canonical names currently lit up

// Always re-query — mappings.js::reload calls controller.render() which
// clears #controller-host and appends a fresh <svg>, so any cached
// reference to the SVG element is stale the moment a binding changes.
function currentSvg() {
  return document.querySelector('#controller-host svg.controller');
}

export async function init() {
  await refresh();
  listen('config-changed', refresh);
  document.addEventListener('keydown', onKeyDown);
  document.addEventListener('keyup',   onKeyUp);
}

async function refresh() {
  try {
    const cfg = await invoke('get_config');
    const m = new Map();
    for (const [id, entry] of Object.entries(cfg.buttons || {})) {
      if (entry?.type !== 'key' || !entry?.value) continue;
      const arr = m.get(entry.value) || [];
      arr.push(Number(id));
      m.set(entry.value, arr);
    }
    bindingsByKey = m;
  } catch (_) {
    bindingsByKey = new Map();
  }
}

function shouldSkip(ev) {
  if (ev.repeat) return true;
  if (isCaptureActive()) return true;
  const t = ev.target;
  if (!t) return false;
  const tag = t.tagName;
  if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return true;
  if (t.isContentEditable) return true;
  return false;
}

function onKeyDown(ev) {
  if (shouldSkip(ev)) return;
  const name = resolveKeyName(ev);
  if (!name) return;
  const ids = bindingsByKey.get(name);
  if (!ids || ids.length === 0) return;
  const svg = currentSvg();
  if (!svg) return;
  activeKeys.add(name);
  for (const id of ids) controller.flashPress(svg, id);
}

function onKeyUp(ev) {
  const name = resolveKeyName(ev);
  if (!name || !activeKeys.has(name)) return;
  activeKeys.delete(name);
  const ids = bindingsByKey.get(name);
  if (!ids) return;
  const svg = currentSvg();
  if (!svg) return;
  for (const id of ids) controller.clearPress(svg, id);
}
