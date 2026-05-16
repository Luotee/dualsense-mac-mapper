import { invoke } from './ipc.js';
import { normaliseKeyEvent } from './key_capture.js';

// Open a modal popup to edit one button binding. `options`:
//   id        : number
//   label     : string (button display label)
//   currentEntry : ButtonEntry as returned by `get_config` — flattened JSON:
//                  { label, type: 'key'|'macro'|'unbound', value? }
//   macros    : array of macro names from current config
//   onSaved() : called after a successful save
//   onClosed(): called after the popup is dismissed (save or cancel)
//
// While the popup is in Key-capture mode it calls invoke('set_capture_active',
// { active: true }) so the engine pauses synth and self-keystrokes can't be
// captured by accident. The flag is cleared on Save or Cancel.

export function open(options) {
  const root = document.createElement('div');
  root.className = 'bind-popup-overlay';
  root.innerHTML = `
    <div class="bind-popup">
      <div class="bp-title">${escape(options.label)}</div>
      <div class="bp-sub">id ${options.id}</div>

      <div class="seg" role="tablist">
        <button class="seg-item" data-seg="key">Key</button>
        <button class="seg-item" data-seg="macro">Macro</button>
        <button class="seg-item" data-seg="unbound">Unbound</button>
      </div>

      <div class="bp-editor"></div>

      <div class="bp-error" hidden></div>

      <div class="bp-actions">
        <button class="bp-cancel">Cancel</button>
        <button class="bp-save primary">Save</button>
      </div>
    </div>
  `;
  document.body.appendChild(root);

  // State — currentEntry is the flattened JSON: { label, type, value? }
  const entry = options.currentEntry || {};
  let segment = entry.type || 'unbound';
  let capturedKey = segment === 'key'   ? (entry.value ?? null) : null;
  let chosenMacro = segment === 'macro' ? (entry.value ?? null) : null;

  const errorEl  = root.querySelector('.bp-error');
  const editorEl = root.querySelector('.bp-editor');

  function show(msg) {
    errorEl.textContent = msg;
    errorEl.hidden = false;
  }
  function hideError() {
    errorEl.hidden = true;
  }

  function renderSeg() {
    root.querySelectorAll('.seg-item').forEach(b => {
      b.classList.toggle('on', b.dataset.seg === segment);
    });
    renderEditor();
  }

  function renderEditor() {
    hideError();
    editorEl.innerHTML = '';

    if (segment === 'key') {
      const box = document.createElement('div');
      box.className = 'capture-box';
      box.tabIndex = 0;

      const updateDisplay = () => {
        if (capturedKey) {
          box.innerHTML = `Current: <code>${escape(capturedKey)}</code><br><span class="hint">Press another key to overwrite. Esc cancels.</span>`;
        } else {
          box.innerHTML = `<strong>Press a key to bind…</strong><br><span class="hint">Esc cancels.</span>`;
        }
      };
      updateDisplay();
      editorEl.appendChild(box);

      // Persistent capture: listener stays attached so the user can keep
      // pressing keys to overwrite the binding without re-clicking the box.
      // Each accepted keypress updates the display in place — we no longer
      // rebuild the editor or blur, so focus is preserved.
      const onKey = ev => {
        const r = normaliseKeyEvent(ev);
        if (r.cancel) {
          // Let Escape bubble up to the popup-root close handler.
          return;
        }
        ev.preventDefault();
        ev.stopPropagation();
        if (r.reject) { show(r.reject); return; }
        hideError();
        capturedKey = r.name;
        updateDisplay();
      };
      box.addEventListener('keydown', onKey);
      box.addEventListener('focus', () => {
        invoke('set_capture_active', { active: true }).catch(() => {});
      });
      box.addEventListener('blur', () => {
        invoke('set_capture_active', { active: false }).catch(() => {});
      });
      // Auto-focus so the user doesn't have to click the box first.
      setTimeout(() => box.focus(), 0);
    }

    if (segment === 'macro') {
      if (!options.macros.length) {
        const m = document.createElement('div');
        m.className = 'hint';
        m.textContent = 'No macros defined. Add one in the Macros tab first.';
        editorEl.appendChild(m);
        return;
      }
      const sel = document.createElement('select');
      sel.className = 'macro-select';
      for (const name of options.macros) {
        const opt = document.createElement('option');
        opt.value = name;
        opt.textContent = name;
        if (name === chosenMacro) opt.selected = true;
        sel.appendChild(opt);
      }
      if (!chosenMacro) chosenMacro = options.macros[0];
      sel.addEventListener('change', () => { chosenMacro = sel.value; });
      editorEl.appendChild(sel);
    }

    if (segment === 'unbound') {
      const m = document.createElement('div');
      m.className = 'hint';
      m.textContent = 'This button will do nothing.';
      editorEl.appendChild(m);
    }
  }

  root.querySelectorAll('.seg-item').forEach(b => {
    b.addEventListener('click', () => {
      segment = b.dataset.seg;
      renderSeg();
    });
  });

  root.querySelector('.bp-cancel').addEventListener('click', () => {
    invoke('set_capture_active', { active: false }).catch(() => {});
    close();
  });

  root.querySelector('.bp-save').addEventListener('click', async () => {
    hideError();
    // ButtonEntry serialises flattened — see comment on `entry` above.
    let entryOut;
    if (segment === 'key') {
      if (!capturedKey) { show('Press a key to bind, or pick Unbound.'); return; }
      entryOut = { label: options.label, type: 'key', value: capturedKey };
    } else if (segment === 'macro') {
      if (!chosenMacro) { show('Pick a macro, or define one in the Macros tab.'); return; }
      entryOut = { label: options.label, type: 'macro', value: chosenMacro };
    } else {
      entryOut = { label: options.label, type: 'unbound' };
    }

    try {
      await invoke('set_binding', { id: options.id, entry: entryOut });
    } catch (e) {
      show(`Save failed: ${e}`);
      return;
    }

    invoke('set_capture_active', { active: false }).catch(() => {});
    options.onSaved?.();
    close();
  });

  function close() {
    root.remove();
    options.onClosed?.();
  }

  // Esc dismisses the popup as a whole
  root.addEventListener('keydown', ev => {
    if (ev.key === 'Escape') {
      invoke('set_capture_active', { active: false }).catch(() => {});
      close();
    }
  });

  renderSeg();
}

function escape(s) { return String(s).replace(/[<>&]/g, c => ({'<': '&lt;', '>': '&gt;', '&': '&amp;'}[c])); }
