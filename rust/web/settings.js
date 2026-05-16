import { invoke, listen } from './ipc.js';

let current = null;   // the Settings struct as loaded from get_config

const APP_VERSION = '0.2.0';

export async function init() {
  await reload();
  render();
  listen('config-changed', async () => { await reload(); render(); });
}

async function reload() {
  const cfg = await invoke('get_config');
  current = {
    deadzone: cfg.deadzone,
    trigger_threshold: cfg.trigger_threshold,
    min_press_ms: cfg.min_press_ms,
    tick_jitter_ms: cfg.tick_jitter_ms,
    log_events: cfg.log_events,
  };
}

function render() {
  const pane = document.getElementById('pane-settings');
  pane.innerHTML = '';
  const form = document.createElement('form');
  form.className = 'settings-form';
  form.onsubmit = e => e.preventDefault();
  form.innerHTML = `
    <div class="settings-section">
      <h3>Analog input</h3>
      <div class="field">
        <label for="f-deadzone">Stick deadzone <span class="hint">(0.0–1.0)</span></label>
        <input id="f-deadzone" type="number" step="0.05" min="0" max="1"
               value="${current.deadzone}">
      </div>
      <div class="field">
        <label for="f-trigger">Trigger threshold <span class="hint">(0.0–1.0)</span></label>
        <input id="f-trigger" type="number" step="0.05" min="0" max="1"
               value="${current.trigger_threshold}">
      </div>
    </div>

    <div class="settings-section">
      <h3>Anti-cheat timing</h3>
      <div class="hint" style="margin-bottom:8px;">Ranges, not constants. The engine picks a random value inside each range on every press / tick.</div>
      <div class="field">
        <label>Min press duration <span class="hint">(ms, min &lt; max)</span></label>
        <div class="dual-range">
          <input id="f-minpress-lo" type="number" min="0" max="500" value="${current.min_press_ms[0]}">
          <span>to</span>
          <input id="f-minpress-hi" type="number" min="1" max="500" value="${current.min_press_ms[1]}">
        </div>
      </div>
      <div class="field">
        <label>Tick jitter <span class="hint">(ms, min ≤ max)</span></label>
        <div class="dual-range">
          <input id="f-jitter-lo" type="number" min="0" max="50" value="${current.tick_jitter_ms[0]}">
          <span>to</span>
          <input id="f-jitter-hi" type="number" min="0" max="50" value="${current.tick_jitter_ms[1]}">
        </div>
      </div>
    </div>

    <div class="settings-section">
      <h3>Logging</h3>
      <div class="field field-toggle">
        <label for="f-log-events">Log every event to console</label>
        <input id="f-log-events" type="checkbox" ${current.log_events ? 'checked' : ''}>
      </div>
    </div>

    <div class="settings-error" hidden></div>

    <div class="settings-actions">
      <button type="button" class="btn-tiny" id="btn-reset">Reset to defaults</button>
      <button type="button" class="btn-tiny" id="btn-open-cfg">Open config file in editor</button>
      <div class="grow"></div>
      <button type="button" class="primary" id="btn-save">Save</button>
    </div>

    <div class="settings-about">
      <h4>About</h4>
      <p>
        <strong>DualSense Mapper</strong> v${APP_VERSION}<br>
        Built with <a href="https://tauri.app/" target="_blank" rel="noreferrer">Tauri</a>.
        See the <a href="https://github.com/Luotee/dualsense-mac-mapper/releases" target="_blank" rel="noreferrer">GitHub releases</a> for changelog and updates.
      </p>
    </div>
  `;
  pane.appendChild(form);

  pane.querySelector('#btn-save').addEventListener('click', save);
  pane.querySelector('#btn-reset').addEventListener('click', resetDefaults);
  pane.querySelector('#btn-open-cfg').addEventListener('click', openInEditor);
}

async function save() {
  const errorEl = document.querySelector('.settings-error');
  errorEl.hidden = true;
  const payload = {
    deadzone: parseFloat(document.getElementById('f-deadzone').value),
    trigger_threshold: parseFloat(document.getElementById('f-trigger').value),
    min_press_ms: [
      parseInt(document.getElementById('f-minpress-lo').value, 10),
      parseInt(document.getElementById('f-minpress-hi').value, 10),
    ],
    tick_jitter_ms: [
      parseInt(document.getElementById('f-jitter-lo').value, 10),
      parseInt(document.getElementById('f-jitter-hi').value, 10),
    ],
    log_events: document.getElementById('f-log-events').checked,
  };
  try {
    await invoke('set_settings', { settings: payload });
    current = payload;
    flash('Saved.');
  } catch (e) {
    errorEl.textContent = `${e}`;
    errorEl.hidden = false;
  }
}

async function resetDefaults() {
  try {
    await invoke('reset_settings');
    await reload();
    render();
  } catch (e) {
    const errorEl = document.querySelector('.settings-error');
    errorEl.textContent = `${e}`;
    errorEl.hidden = false;
  }
}

async function openInEditor() {
  try {
    await invoke('open_config_in_editor');
  } catch (e) {
    const errorEl = document.querySelector('.settings-error');
    errorEl.textContent = `Open failed: ${e}`;
    errorEl.hidden = false;
  }
}

function flash(msg) {
  const el = document.createElement('div');
  el.className = 'settings-flash';
  el.textContent = msg;
  document.body.appendChild(el);
  setTimeout(() => el.remove(), 1600);
}
