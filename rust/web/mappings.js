import { invoke, listen } from './ipc.js';
import * as controller from './controller.js';
import * as bindPopup from './bind_popup.js';

let config = null;
let svgEl  = null;
let rafQueued = new Map();   // id → frame id, so we don't queue duplicate flashes

export async function init() {
  const root = document.getElementById('pane-mappings');
  await reload();
  svgEl = root.querySelector('svg.controller');
  hookClick();
  hookLiveHighlight();
  listenConfigChanged();
  renderChipList();
}

async function reload() {
  config = await invoke('get_config');
  const bindings = {};
  for (const [id, entry] of Object.entries(config.buttons)) {
    bindings[id] = { kind: kindOf(entry.binding), value: valueOf(entry.binding) };
  }
  controller.render(document.getElementById('controller-host'), bindings);
  svgEl = document.querySelector('#controller-host svg.controller');
}

function kindOf(b) {
  if (b === 'Unbound' || b === null) return 'unbound';
  if (typeof b === 'object' && 'Key'   in b) return 'key';
  if (typeof b === 'object' && 'Macro' in b) return 'macro';
  return 'unbound';
}
function valueOf(b) {
  if (typeof b === 'object' && b !== null && 'Key'   in b) return b.Key;
  if (typeof b === 'object' && b !== null && 'Macro' in b) return b.Macro;
  return undefined;
}

function hookClick() {
  document.getElementById('controller-host').addEventListener('click', e => {
    const hit = e.target.closest('[data-id]');
    if (!hit) return;
    const id = Number(hit.dataset.id);
    openPopup(id);
  });
}

function openPopup(id) {
  const entry = config.buttons[String(id)];
  if (!entry) return;
  controller.selectButton(svgEl, id);
  bindPopup.open({
    id,
    label: entry.label,
    currentEntry: entry,
    macros: Object.keys(config.macros || {}),
    onSaved: () => { /* reload happens in onClosed */ },
    onClosed: async () => {
      controller.clearSelection(svgEl);
      // config-changed event from the bridge will refresh; but explicitly reload
      // now in case the save was a no-op or the event hasn't arrived yet.
      await reload();
      renderChipList();
    },
  });
}

function hookLiveHighlight() {
  listen('button-down', payload => {
    const id = payload.id;
    if (rafQueued.has(id)) return;
    const f = requestAnimationFrame(() => {
      controller.flashPress(svgEl, id);
      rafQueued.delete(id);
    });
    rafQueued.set(id, f);
  });
  listen('button-up', payload => {
    controller.clearPress(svgEl, payload.id);
  });
}

function listenConfigChanged() {
  listen('config-changed', async () => {
    await reload();
    renderChipList();
  });
}

function renderChipList() {
  const host = document.getElementById('chip-list');
  if (!host) return;
  host.innerHTML = '';
  for (let id = 0; id <= 24; id++) {
    const entry = config.buttons[String(id)];
    if (!entry) continue;
    const row = document.createElement('div');
    row.className = 'chip-row';
    row.dataset.id = id;
    const kind = kindOf(entry.binding);
    row.innerHTML = `
      <span class="chip-label">${escape(entry.label)}</span>
      ${renderChip(entry.binding, kind)}
    `;
    row.addEventListener('click', () => openPopup(id));
    host.appendChild(row);
  }
}

function renderChip(b, kind) {
  if (kind === 'key')   return `<code class="chip chip-key">${escape(valueOf(b))}</code>`;
  if (kind === 'macro') return `<code class="chip chip-macro">&#x26A1; ${escape(valueOf(b))}</code>`;
  return `<span class="chip-mute">unbound</span>`;
}
function escape(s) { return String(s).replace(/[<>&]/g, c => ({'<': '&lt;', '>': '&gt;', '&': '&amp;'}[c])); }
