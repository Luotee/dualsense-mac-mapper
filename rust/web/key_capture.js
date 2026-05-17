// Convert a browser KeyboardEvent into the canonical key name used by Rust's
// config::parse_key(). Returns one of:
//   { name: "Up" }        — accepted, here's the name
//   { cancel: true }      — user pressed Escape, abort capture
//   { reject: "msg" }     — unsupported key, show message in popup

export function normaliseKeyEvent(ev) {
  ev.preventDefault();
  ev.stopPropagation();

  if (ev.key === 'Escape') return { cancel: true };

  // Modifier keys — distinguish left/right via ev.code so users can bind
  // LShift vs RShift independently. The plain "Shift" / "Control" / "Alt"
  // names still work as generic fallbacks (no side specified).
  if (ev.key === 'Shift')   return { name: ev.code === 'ShiftRight'   ? 'RShift'   : 'LShift' };
  if (ev.key === 'Control') return { name: ev.code === 'ControlRight' ? 'RControl' : 'LControl' };
  if (ev.key === 'Alt')     return { name: ev.code === 'AltRight'     ? 'RAlt'     : 'LAlt' };
  if (ev.key === 'Meta')    return { name: 'Meta' };

  // Function keys
  if (/^F\d+$/.test(ev.key)) {
    const n = parseInt(ev.key.slice(1), 10);
    if (n >= 1 && n <= 12) return { name: ev.key };
  }

  // Arrows
  if (ev.code === 'ArrowUp')    return { name: 'Up' };
  if (ev.code === 'ArrowDown')  return { name: 'Down' };
  if (ev.code === 'ArrowLeft')  return { name: 'Left' };
  if (ev.code === 'ArrowRight') return { name: 'Right' };

  // Space
  if (ev.key === ' ' || ev.code === 'Space') return { name: 'Space' };

  // Named keys
  const named = ['Enter', 'Return', 'Tab', 'Escape', 'Esc', 'Backspace', 'Delete', 'Del',
                 'Home', 'End', 'PageUp', 'PageDown'];
  if (named.includes(ev.key)) return { name: ev.key };

  // Any single printable ASCII character. Letters and digits go through
  // case-normalisation; punctuation (`-`, `=`, `,`, `.`, `/`, `;`, `'`,
  // `\\`, `[`, `]`, backtick) round-trips as-is. Backend `parse_key`
  // maps each to the appropriate Windows VK code so games see them
  // as real held keys, not Unicode-injected characters.
  if (ev.key.length === 1) {
    const c = ev.key.charCodeAt(0);
    if (c >= 0x20 && c <= 0x7E) {
      return { name: /[A-Z]/.test(ev.key) ? ev.key.toLowerCase() : ev.key };
    }
  }

  return { reject: `Unsupported key "${ev.key}".` };
}

// Non-mutating variant of normaliseKeyEvent — returns the same canonical
// name but does NOT call preventDefault/stopPropagation, so it is safe to
// run on every document keydown (including events targeted at form fields).
//
// Returns the canonical key name (string) or `null` if the event is not a
// supported binding key. The keyboard-mirror feature uses this to look up
// `bindingsByKey` without disrupting normal input.
export function resolveKeyName(ev) {
  if (ev.key === 'Escape') return null;

  if (ev.key === 'Shift')   return ev.code === 'ShiftRight'   ? 'RShift'   : 'LShift';
  if (ev.key === 'Control') return ev.code === 'ControlRight' ? 'RControl' : 'LControl';
  if (ev.key === 'Alt')     return ev.code === 'AltRight'     ? 'RAlt'     : 'LAlt';
  if (ev.key === 'Meta')    return 'Meta';

  if (/^F\d+$/.test(ev.key)) {
    const n = parseInt(ev.key.slice(1), 10);
    if (n >= 1 && n <= 12) return ev.key;
  }

  if (ev.code === 'ArrowUp')    return 'Up';
  if (ev.code === 'ArrowDown')  return 'Down';
  if (ev.code === 'ArrowLeft')  return 'Left';
  if (ev.code === 'ArrowRight') return 'Right';

  if (ev.key === ' ' || ev.code === 'Space') return 'Space';

  const named = ['Enter', 'Return', 'Tab', 'Backspace', 'Delete', 'Del',
                 'Home', 'End', 'PageUp', 'PageDown'];
  if (named.includes(ev.key)) return ev.key;

  if (ev.key.length === 1) {
    const c = ev.key.charCodeAt(0);
    if (c >= 0x20 && c <= 0x7E) {
      return /[A-Z]/.test(ev.key) ? ev.key.toLowerCase() : ev.key;
    }
  }

  return null;
}
