import { invoke, listen } from './ipc.js';
import * as mappings from './mappings.js';

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

document.getElementById('btn-settings').addEventListener('click', () => activate('settings'));

const statusEl   = document.querySelector('.status');
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

await mappings.init();
