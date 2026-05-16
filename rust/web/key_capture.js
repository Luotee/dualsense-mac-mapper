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
