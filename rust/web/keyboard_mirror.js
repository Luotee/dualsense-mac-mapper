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

let bindingsByKey   = new Map();   // key-name (string) → array of numeric button ids
let keyByButtonId   = new Map();   // numeric id → key-name
const activeKeys    = new Set();   // canonical names currently lit up
// Keys recently synthesised by our own engine (in response to a
// physical gamepad press). The OS will deliver these as keydown events
// to the focused window; without suppression keyboard_mirror would
// double-flash every button bound to the same key (e.g. pressing
// D-pad ← would also flash L-stick ← if both bind to "Left").
const synthSuppressed = new Map();  // key → setTimeout handle

// Always re-query — mappings.js::reload calls controller.render() which
// clears #controller-host and appends a fresh <svg>, so any cached
// reference to the SVG element is stale the moment a binding changes.
function currentSvg() {
  return document.querySelector('#controller-host svg.controller');
}

export async function init() {
  await refresh();
  listen('config-changed', refresh);
  listen('button-down', ev => onPhysicalDown(ev.id));
  document.addEventListener('keydown', onKeyDown);
  document.addEventListener('keyup',   onKeyUp);
}

function onPhysicalDown(id) {
  const numId = Number(id);
  const key = keyByButtonId.get(numId);
  if (!key) return;
  // The synthesised keydown typically arrives at the JS event loop a
  // few ms BEFORE the engine's Tauri 'button-down' event, so a pure
  // forward-suppression set was racy — mirror would already have lit
  // every OTHER button bound to the same key. Two complementary fixes
  // here:
  //
  //   1. Retroactively clear any mirror-flashed presses on OTHER ids
  //      that share this key. mappings.js's own 'button-down' handler
  //      has already (or will shortly) flashed the *correct* physical
  //      button, so removing the wrong ones from the DOM mid-animation
  //      kills the spurious flash before the user can perceive it.
  //   2. Arm a forward suppression window so a synth keydown that
  //      arrives AFTER this event (the opposite race) is also skipped.
  const ids = bindingsByKey.get(key) || [];
  const svg = currentSvg();
  if (svg) {
    for (const otherId of ids) {
      if (otherId !== numId) controller.clearPress(svg, otherId);
    }
  }
  activeKeys.delete(key);
  // Forward suppression: any keydown for this key in the next 250 ms
  // is treated as the engine's synth.
  const prev = synthSuppressed.get(key);
  if (prev) clearTimeout(prev);
  const h = setTimeout(() => synthSuppressed.delete(key), 250);
  synthSuppressed.set(key, h);
}

async function refresh() {
  try {
    const cfg = await invoke('get_config');
    const m = new Map();
    const k = new Map();
    for (const [id, entry] of Object.entries(cfg.buttons || {})) {
      if (entry?.type !== 'key' || !entry?.value) continue;
      const arr = m.get(entry.value) || [];
      arr.push(Number(id));
      m.set(entry.value, arr);
      k.set(Number(id), entry.value);
    }
    bindingsByKey = m;
    keyByButtonId = k;
  } catch (_) {
    bindingsByKey = new Map();
    keyByButtonId = new Map();
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
  // Synth from our own engine — the physical button's flashPress
  // already lit the correct hit zone via mappings.js's 'button-down'
  // listener. Mirroring would double-light every OTHER button bound
  // to the same key.
  if (synthSuppressed.has(name)) return;
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
