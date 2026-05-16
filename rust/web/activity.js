// Activity log drawer. Toggled by the 📊 button in the toolbar. When open,
// shows a live stream of activity events (button-down/up, key emit, macro
// start/end). Throttled to ≤1 DOM update per rAF (matches spec §11.1).
// Drawer-open state persisted to ui-prefs JSON beside the main config.

import { invoke, listen } from './ipc.js';

let prefs = { drawer_open: false };
const queue = [];
let rafScheduled = false;
const MAX_ROWS = 200;

export async function init() {
  prefs = await invoke('get_ui_prefs').catch(() => ({ drawer_open: false }));
  setOpen(prefs.drawer_open);

  document.getElementById('btn-activity').addEventListener('click', () => {
    setOpen(!isOpen());
  });

  document.getElementById('btn-activity-close')?.addEventListener('click', () => setOpen(false));

  listen('activity', ev => {
    queue.push(ev);
    if (queue.length > 1000) queue.splice(0, queue.length - 1000); // cap memory
    schedule();
  });
  listen('button-down', ev => {
    queue.push({ ts_ms: Date.now(), kind: 'button-down', id: ev.id });
    schedule();
  });
  listen('button-up', ev => {
    queue.push({ ts_ms: Date.now(), kind: 'button-up', id: ev.id });
    schedule();
  });
  listen('config-changed', () => {
    queue.push({ ts_ms: Date.now(), kind: 'config-changed' });
    schedule();
  });
}

function schedule() {
  if (!isOpen() || rafScheduled) return;
  rafScheduled = true;
  requestAnimationFrame(flush);
}

function flush() {
  rafScheduled = false;
  const list = document.getElementById('activity-list');
  if (!list) { queue.length = 0; return; }
  const frag = document.createDocumentFragment();
  const batch = queue.splice(0, queue.length);
  for (const ev of batch) frag.appendChild(renderRow(ev));
  list.appendChild(frag);
  while (list.childElementCount > MAX_ROWS) list.removeChild(list.firstElementChild);
  // Auto-scroll
  list.scrollTop = list.scrollHeight;
}

function renderRow(ev) {
  const row = document.createElement('div');
  row.className = 'activity-row';
  const ts = new Date(ev.ts_ms || Date.now());
  const t = `${pad(ts.getHours())}:${pad(ts.getMinutes())}:${pad(ts.getSeconds())}.${pad3(ts.getMilliseconds())}`;
  row.innerHTML = `<span class="a-ts">${t}</span><span class="a-msg">${escape(label(ev))}</span>`;
  if (ev.kind === 'button-down') row.classList.add('a-press');
  if (ev.kind === 'button-up')   row.classList.add('a-release');
  if (ev.kind === 'emit')        row.classList.add('a-emit');
  if (ev.kind === 'macro-start' || ev.kind === 'macro-end') row.classList.add('a-macro');
  return row;
}

function label(ev) {
  switch (ev.kind) {
    case 'button-down':    return `▼ button ${ev.id}`;
    case 'button-up':      return `▲ button ${ev.id}`;
    case 'emit':           return `→ key ${ev.key} ${ev.action}`;
    case 'macro-start':    return `⚡ macro ${ev.name} started`;
    case 'macro-end':      return `⏹ macro ${ev.name} ${ev.completed ? 'completed' : 'cancelled'}`;
    case 'config-changed': return `↻ config reloaded`;
    default:               return ev.kind || '?';
  }
}

function setOpen(open) {
  const drawer = document.getElementById('activity-drawer');
  const toggle = document.getElementById('btn-activity');
  if (!drawer || !toggle) return;
  drawer.classList.toggle('open', open);
  toggle.classList.toggle('on', open);
  prefs.drawer_open = open;
  invoke('set_ui_prefs', { prefs }).catch(() => {});
}

function isOpen() {
  return document.getElementById('activity-drawer')?.classList.contains('open');
}

function pad(n)  { return String(n).padStart(2, '0'); }
function pad3(n) { return String(n).padStart(3, '0'); }
function escape(s) {
  return String(s).replace(/[<>&]/g, c => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;' }[c]));
}
