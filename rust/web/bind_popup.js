import { invoke } from './ipc.js';
import { normaliseKeyEvent } from './key_capture.js';

// Open a modal popup to edit one button binding. `options`:
//   id        : number
//   label     : string (button display label)
//   currentEntry : { binding: 'Unbound' | { Key: string } | { Macro: string } }
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

  // State
  const current = options.currentEntry.binding;
  let segment = (current === 'Unbound' || current === null) ? 'unbound'
              : (typeof current === 'object' && 'Key'   in current) ? 'key'
              : (typeof current === 'object' && 'Macro' in current) ? 'macro'
              : 'unbound';
  let capturedKey = (segment === 'key' && current.Key) ? current.Key : null;
  let chosenMacro = (segment === 'macro' && current.Macro) ? current.Macro : null;

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
      if (capturedKey) {
        box.innerHTML = `Current: <code>${escape(capturedKey)}</code><br><span class="hint">Click here and press another key to change.</span>`;
      } else {
        box.innerHTML = `<strong>Press the key to bind…</strong>`;
      }
      editorEl.appendChild(box);

      // Wire up capture
      box.addEventListener('focus', () => {
        invoke('set_capture_active', { active: true }).catch(() => {});
        const onKey = ev => {
          const r = normaliseKeyEvent(ev);
          if (r.cancel) { box.blur(); return; }
          if (r.reject) { show(r.reject); return; }
          capturedKey = r.name;
          renderEditor();
          box.blur();
        };
        box.addEventListener('keydown', onKey, { once: true });
        box.addEventListener('blur', () => {
          invoke('set_capture_active', { active: false }).catch(() => {});
        }, { once: true });
      });
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
    let binding;
    if (segment === 'key') {
      if (!capturedKey) { show('Press a key to bind, or pick Unbound.'); return; }
      binding = { Key: capturedKey };
    } else if (segment === 'macro') {
      if (!chosenMacro) { show('Pick a macro, or define one in the Macros tab.'); return; }
      binding = { Macro: chosenMacro };
    } else {
      binding = 'Unbound';
    }

    try {
      await invoke('set_binding', {
        id: options.id,
        entry: { label: options.label, binding },
      });
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
