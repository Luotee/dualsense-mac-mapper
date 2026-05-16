// Convert a browser KeyboardEvent into the canonical key name used by Rust's
// config::parse_key(). Returns one of:
//   { name: "Up" }        — accepted, here's the name
//   { cancel: true }      — user pressed Escape, abort capture
//   { reject: "msg" }     — unsupported key, show message in popup

export function normaliseKeyEvent(ev) {
  ev.preventDefault();
  ev.stopPropagation();

  if (ev.key === 'Escape') return { cancel: true };

  // Modifier-only press
  if (['Shift', 'Control', 'Alt', 'Meta'].includes(ev.key)) return { name: ev.key };

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

  // Single printable ASCII
  if (ev.key.length === 1) {
    if (/[a-zA-Z0-9]/.test(ev.key)) return { name: ev.key.toLowerCase() };
  }

  return { reject: `Unsupported key "${ev.key}". Try a letter, digit, arrow, F-key, or named key.` };
}
